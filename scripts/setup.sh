#!/usr/bin/env bash
# Prepare local dependencies and build the Rust workspace.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

for dependency in cargo rustc solana docker; do
    if ! command -v "$dependency" >/dev/null 2>&1; then
        echo "Missing prerequisite: $dependency. See README.md." >&2
        exit 1
    fi
done

if ! docker info >/dev/null 2>&1; then
    echo "Docker is not running. Start Docker and retry." >&2
    exit 1
fi

container_name="solana-arb-redis"
if docker container inspect "$container_name" >/dev/null 2>&1; then
    docker start "$container_name" >/dev/null
else
    docker run --name "$container_name" -d -p 127.0.0.1:6379:6379 redis:7-alpine >/dev/null
fi

cargo build --workspace --locked

cat <<'NEXT_STEPS'
Build complete. Redis is running on localhost:6379.

Next steps:
1. Create a development keypair and review config/devnet.toml (see README.md).
2. Run tests: cargo test --workspace --locked
3. Start simulation: cargo run --locked --bin arb-bot -- --config config/devnet.toml --dry-run
NEXT_STEPS
