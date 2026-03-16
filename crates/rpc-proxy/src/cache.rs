//! Redis cache layer for RPC responses

use anyhow::Result;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::time::Duration;
use tracing::{debug, warn, info};

/// Serializable wrapper for Account
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedAccount {
    lamports: u64,
    data: Vec<u8>,
    owner: [u8; 32],
    executable: bool,
    rent_epoch: u64,
}

impl From<&Account> for CachedAccount {
    fn from(account: &Account) -> Self {
        Self {
            lamports: account.lamports,
            data: account.data.clone(),
            owner: account.owner.to_bytes(),
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        }
    }
}

impl From<CachedAccount> for Account {
    fn from(cached: CachedAccount) -> Self {
        Self {
            lamports: cached.lamports,
            data: cached.data,
            owner: Pubkey::new_from_array(cached.owner),
            executable: cached.executable,
            rent_epoch: cached.rent_epoch,
        }
    }
}

/// Optional cache layer - works without Redis if unavailable
pub struct CacheLayer {
    pool: Option<Pool>,
    ttl: Duration,
}

impl CacheLayer {
    /// Create new cache layer, returns Ok even if Redis is unavailable
    pub async fn new(redis_url: &str, ttl_ms: u64) -> Result<Self> {
        let ttl = Duration::from_millis(ttl_ms);
        
        // Try to connect to Redis, but don't fail if unavailable
        match Self::try_connect(redis_url).await {
            Ok(pool) => {
                info!("✅ Redis cache connected");
                Ok(Self { pool: Some(pool), ttl })
            }
            Err(e) => {
                warn!("⚠️ Redis unavailable, running without cache: {}", e);
                Ok(Self { pool: None, ttl })
            }
        }
    }
    
    async fn try_connect(redis_url: &str) -> Result<Pool> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

        // Test connection
        let mut conn = pool.get().await?;
        let _: String = redis::cmd("PING").query_async(&mut *conn).await?;

        Ok(pool)
    }
    
    /// Check if cache is available
    pub fn is_available(&self) -> bool {
        self.pool.is_some()
    }

    pub async fn get_account(&self, pubkey: &Pubkey) -> Option<Account> {
        let pool = self.pool.as_ref()?;
        
        let mut conn = match pool.get().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Redis connection error: {}", e);
                return None;
            }
        };

        let key = format!("account:{}", pubkey);
        let data: Option<String> = conn.get(&key).await.ok()?;

        data.and_then(|d| {
            serde_json::from_str::<CachedAccount>(&d)
                .ok()
                .map(Account::from)
        })
    }

    pub async fn set_account(&self, pubkey: &Pubkey, account: &Account) {
        let pool = match &self.pool {
            Some(p) => p,
            None => return,
        };
        
        let mut conn = match pool.get().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Redis connection error: {}", e);
                return;
            }
        };

        let key = format!("account:{}", pubkey);
        let cached = CachedAccount::from(account);
        if let Ok(data) = serde_json::to_string(&cached) {
            let _: Result<(), _> = conn
                .set_ex(&key, data, self.ttl.as_secs() as u64)
                .await;
            debug!("Cached account: {}", pubkey);
        }
    }

    pub async fn get_multiple(&self, pubkeys: &[Pubkey]) -> Vec<Option<Account>> {
        let mut results = vec![None; pubkeys.len()];
        
        if self.pool.is_none() {
            return results;
        }

        for (i, pubkey) in pubkeys.iter().enumerate() {
            results[i] = self.get_account(pubkey).await;
        }

        results
    }
}
