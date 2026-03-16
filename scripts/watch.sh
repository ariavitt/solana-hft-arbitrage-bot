#!/bin/bash
# Watch arbitrage opportunities in real-time with color output
# Usage: ./scripts/watch.sh

clear
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║          🚀 SOLANA ARBITRAGE BOT - LIVE MONITOR 🚀            ║"
echo "╠══════════════════════════════════════════════════════════════╣"
echo "║  Ctrl+C to stop                                              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

cd "$(dirname "$0")/.."

# Kill any existing bot
pkill -f "arb-bot" 2>/dev/null || true
sleep 1

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# Start bot
./target/release/arb-bot --dry-run --config config/mainnet-simulate.toml 2>&1 | while IFS= read -r line; do
    
    # Pool prices (cyan)
    if [[ $line == *"📊 Pool"* ]]; then
        # Extract pool name and price
        name=$(echo "$line" | grep -oP 'Pool \K[^(]+' | tr -d ':')
        price=$(echo "$line" | grep -oP 'price=\K[0-9.]+')
        tick=$(echo "$line" | grep -oP 'tick=\K-?[0-9]+')
        printf "${CYAN}📊 %-25s ${GREEN}\$%-10s${NC} tick=${tick}\n" "$name" "$price"
    
    # Path evaluation
    elif [[ $line == *"Path evaluation"* ]]; then
        # Extract profit
        profit=$(echo "$line" | grep -oP 'Profit: \K-?[0-9]+')
        path=$(echo "$line" | grep -oP '\d+\.\d+ SOL → [0-9.]+ USDC → [0-9.]+ SOL')
        
        if [[ $profit -gt 0 ]]; then
            printf "${GREEN}${BOLD}🎯 PROFITABLE! ${path} | +${profit} bps${NC}\n"
        elif [[ $profit -gt -20 ]]; then
            printf "${YELLOW}⚡ Near breakeven: ${path} | ${profit} bps${NC}\n"
        fi
        # Don't show very negative ones to reduce noise
    
    # Found opportunity (green bold)
    elif [[ $line == *"🎯"* ]]; then
        printf "${GREEN}${BOLD}$line${NC}\n"
    
    # Stats
    elif [[ $line == *"Stats"* ]]; then
        printf "${PURPLE}$line${NC}\n"
    
    # Errors (red)
    elif [[ $line == *"❌"* ]] || [[ $line == *"Error"* ]]; then
        printf "${RED}$line${NC}\n"
    
    # Important info
    elif [[ $line == *"✅"* ]]; then
        printf "${GREEN}$line${NC}\n"
    fi
    
done



