#!/bin/bash

set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "Usage: $0 <mainnet|devnet> <PROGRAM_ID>"
  exit 1
fi

NETWORK="$1"
PROGRAM_ID="$2"

if ! [[ "$PROGRAM_ID" =~ ^[1-9A-HJ-NP-Za-km-z]{32,44}$ ]]; then
  echo "Invalid Solana program id: $PROGRAM_ID"
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
LIB_RS="$ROOT_DIR/programs/arbitrage-aggregator/src/lib.rs"
ANCHOR_TOML="$ROOT_DIR/Anchor.toml"
MAINNET_TOML="$ROOT_DIR/config/mainnet.toml"
MAINNET_SIM_TOML="$ROOT_DIR/config/mainnet-simulate.toml"
MAINNET_DEPLOY_TOML="$ROOT_DIR/config/mainnet-deploy.toml"
DEVNET_TOML="$ROOT_DIR/config/devnet.toml"

case "$NETWORK" in
  mainnet)
    perl -0pi -e 's/declare_id!\(".*?"\);/declare_id!("'"$PROGRAM_ID"'");/' "$LIB_RS"
    perl -0pi -e 's/(\[programs\.mainnet\]\s+arbitrage_aggregator = ")[^"]+(")/${1}'"$PROGRAM_ID"'$2/s' "$ANCHOR_TOML"
    perl -0pi -e 's/(\[aggregator\]\s+program_id = ")[^"]+(")/${1}'"$PROGRAM_ID"'$2/s' "$MAINNET_TOML"
    perl -0pi -e 's/(\[aggregator\]\s+program_id = ")[^"]+(")/${1}'"$PROGRAM_ID"'$2/s' "$MAINNET_SIM_TOML"
    if [ -f "$MAINNET_DEPLOY_TOML" ]; then
      perl -0pi -e 's/(\[aggregator\]\s+program_id = ")[^"]+(")/${1}'"$PROGRAM_ID"'$2/s' "$MAINNET_DEPLOY_TOML"
    fi
    ;;
  devnet)
    perl -0pi -e 's/declare_id!\(".*?"\);/declare_id!("'"$PROGRAM_ID"'");/' "$LIB_RS"
    perl -0pi -e 's/(\[programs\.devnet\]\s+arbitrage_aggregator = ")[^"]+(")/${1}'"$PROGRAM_ID"'$2/s' "$ANCHOR_TOML"
    perl -0pi -e 's/(\[aggregator\]\s+program_id = ")[^"]+(")/${1}'"$PROGRAM_ID"'$2/s' "$DEVNET_TOML"
    ;;
  *)
    echo "Unsupported network: $NETWORK"
    echo "Use mainnet or devnet"
    exit 1
    ;;
esac

echo "Synchronized aggregator program id for $NETWORK:"
echo "  $PROGRAM_ID"
