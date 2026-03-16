#!/bin/bash
# Real-time monitoring of arbitrage opportunities
# Usage: ./scripts/monitor.sh

echo "🚀 Solana Arbitrage Bot Monitor"
echo "================================"
echo ""
echo "Starting bot in background..."

cd "$(dirname "$0")/.."

# Kill any existing bot
lsof -ti:9090 | xargs kill -9 2>/dev/null || true

# Start bot and filter output for opportunities
./target/release/arb-bot --dry-run --verbose --config config/mainnet-simulate.toml 2>&1 | while read line; do
    # Show pool prices
    if echo "$line" | grep -q "📊 Pool"; then
        echo -e "\033[36m$line\033[0m"
    # Show path evaluation
    elif echo "$line" | grep -q "Path evaluation"; then
        profit=$(echo "$line" | grep -oP 'Profit: -?\d+')
        if [[ $profit == *"-"* ]]; then
            echo -e "\033[31m$line\033[0m"  # Red for negative
        else
            echo -e "\033[32m$line\033[0m"  # Green for positive
        fi
    # Show found opportunity
    elif echo "$line" | grep -q "🎯"; then
        echo -e "\033[33;1m$line\033[0m"  # Yellow bold
    # Show stats
    elif echo "$line" | grep -q "Stats"; then
        echo -e "\033[35m$line\033[0m"  # Magenta
    # Show errors
    elif echo "$line" | grep -q "❌\|Error\|error"; then
        echo -e "\033[31;1m$line\033[0m"  # Red bold
    fi
done



