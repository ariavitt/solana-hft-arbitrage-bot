//! Route graph for finding swap paths

use bot_core::{ParsedPool, PoolType, Route, SwapDirection, SwapLeg};
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};

/// Edge in the route graph representing a pool
#[derive(Clone, Debug)]
pub struct PoolEdge {
    pub pool: Pubkey,
    pub pool_type: PoolType,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub fee_bps: u16,
}

/// Graph structure for route finding
pub struct RouteGraph {
    /// Adjacency list: token -> [(target_token, edge)]
    adjacency: HashMap<Pubkey, Vec<(Pubkey, PoolEdge)>>,
}

impl RouteGraph {
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
        }
    }

    /// Add a pool to the graph (creates edges in both directions)
    pub fn add_pool(&mut self, pool: &ParsedPool) {
        // A -> B edge
        let edge_ab = PoolEdge {
            pool: pool.address,
            pool_type: pool.pool_type,
            token_in: pool.token_a.mint,
            token_out: pool.token_b.mint,
            fee_bps: pool.fee_rate_bps,
        };

        self.adjacency
            .entry(pool.token_a.mint)
            .or_default()
            .push((pool.token_b.mint, edge_ab));

        // B -> A edge
        let edge_ba = PoolEdge {
            pool: pool.address,
            pool_type: pool.pool_type,
            token_in: pool.token_b.mint,
            token_out: pool.token_a.mint,
            fee_bps: pool.fee_rate_bps,
        };

        self.adjacency
            .entry(pool.token_b.mint)
            .or_default()
            .push((pool.token_a.mint, edge_ba));
    }

    /// Remove a pool from the graph
    pub fn remove_pool(&mut self, pool_address: &Pubkey) {
        for edges in self.adjacency.values_mut() {
            edges.retain(|(_, edge)| &edge.pool != pool_address);
        }
    }

    /// Find all routes between two tokens with max hops
    pub fn find_routes(
        &self,
        token_in: &Pubkey,
        token_out: &Pubkey,
        max_hops: usize,
    ) -> Vec<Route> {
        let mut routes = Vec::new();
        let mut path = Vec::new();
        let mut visited = HashSet::new();

        self.dfs(
            token_in,
            token_out,
            max_hops,
            &mut path,
            &mut visited,
            &mut routes,
        );

        routes
    }

    fn dfs(
        &self,
        current: &Pubkey,
        target: &Pubkey,
        remaining_hops: usize,
        path: &mut Vec<PoolEdge>,
        visited: &mut HashSet<Pubkey>,
        routes: &mut Vec<Route>,
    ) {
        if remaining_hops == 0 {
            return;
        }

        if current == target && !path.is_empty() {
            // Found a route
            let legs: Vec<SwapLeg> = path
                .iter()
                .map(|edge| SwapLeg {
                    pool: edge.pool,
                    pool_type: edge.pool_type,
                    token_in: edge.token_in,
                    token_out: edge.token_out,
                    amount_in: 0,
                    expected_out: 0,
                    min_amount_out: 0, // Will be calculated with slippage
                    direction: SwapDirection::AtoB, // Will be calculated
                    a_to_b: true, // Will be set based on direction
                })
                .collect();

            routes.push(Route {
                legs,
                total_amount_in: 0,
                expected_amount_out: 0,
                price_impact_bps: 0,
            });
            return;
        }

        visited.insert(*current);

        if let Some(edges) = self.adjacency.get(current) {
            for (next_token, edge) in edges {
                // Skip if we've visited this token (except for the target in circular routes)
                if visited.contains(next_token) && next_token != target {
                    continue;
                }

                // Skip if we've used this pool already
                if path.iter().any(|e| e.pool == edge.pool) {
                    continue;
                }

                path.push(edge.clone());
                self.dfs(next_token, target, remaining_hops - 1, path, visited, routes);
                path.pop();
            }
        }

        visited.remove(current);
    }

    /// Get all unique tokens in the graph
    pub fn get_tokens(&self) -> Vec<Pubkey> {
        self.adjacency.keys().cloned().collect()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.adjacency.values().map(|v| v.len()).sum()
    }
}

impl Default for RouteGraph {
    fn default() -> Self {
        Self::new()
    }
}

