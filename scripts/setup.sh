#!/bin/bash
# Setup script for Solana HFT Bot

set -e

echo "=== Solana HFT Bot Setup ==="
echo ""

# Check Rust
if ! command -v rustc &> /dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "✓ Rust is installed: $(rustc --version)"
fi

# Check Solana CLI
if ! command -v solana &> /dev/null; then
    echo "Installing Solana CLI..."
    sh -c "$(curl -sSfL https://release.solana.com/v1.18.4/install)"
    export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
else
    echo "✓ Solana CLI is installed: $(solana --version)"
fi

# Check Docker
if ! command -v docker &> /dev/null; then
    echo "⚠ Docker is not installed. Please install Docker manually."
else
    echo "✓ Docker is installed: $(docker --version)"
fi

# Check Redis
if docker ps | grep -q redis; then
    echo "✓ Redis is running"
else
    echo "Starting Redis..."
    docker run -d --name redis -p 6379:6379 redis:7-alpine 2>/dev/null || \
    docker start redis 2>/dev/null || \
    echo "⚠ Could not start Redis. Please start it manually."
fi

# Build project
echo ""
echo "Building project..."
cargo build

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "1. Configure RPC endpoints in config/devnet.toml or config/mainnet.toml"
echo "2. Run tests: cargo test"
echo "3. Start the bot: cargo run --bin hft-bot -- --config config/devnet.toml"

