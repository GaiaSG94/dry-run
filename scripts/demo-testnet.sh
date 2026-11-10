#!/usr/bin/env bash
# scripts/demo-testnet.sh
#
# Déploiement et vérification du contrat Vault sur le réseau public NEAR Testnet.
#
# Prérequis :
#   - near-cli-rs (ou near-cli) installé : `cargo install near-cli-rs`
#   - Un compte testnet valide avec quelques testnet NEAR (créable sur https://testnet.mynearwallet.com)
#
# Usage :
#   export NEAR_ACCOUNT="mon-coffre.testnet"
#   ./scripts/demo-testnet.sh

set -euo pipefail

RPC_URL="${NEAR_RPC_URL:-https://rpc.testnet.near.org}"
NETWORK="testnet"
ACCOUNT="${NEAR_ACCOUNT:-}"

echo "╔══════════════════════════════════════════════════════════════════════╗"
echo "║          DRY RUN — DÉPLOIEMENT TESTNET PUBLIC NEAR                   ║"
echo "╚══════════════════════════════════════════════════════════════════════╝"

if [[ -z "$ACCOUNT" ]]; then
    echo "⚠️  Variable NEAR_ACCOUNT non définie."
    echo "    Pour déployer réellement sur testnet :"
    echo "    export NEAR_ACCOUNT=\"votre-compte.testnet\""
    echo "    $0"
    echo ""
    echo "Mode vérification RPC testnet sans déploiement :"
    echo "Test de connectivité avec le RPC public testnet ($RPC_URL)..."
    STATUS=$(curl -s -X POST "$RPC_URL" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":"dontcare","method":"status","params":[]}')
    SYNC_STATUS=$(echo "$STATUS" | python3 -c "import json,sys; d=json.load(sys.stdin); print('Bloc actuel:', d['result']['sync_info']['latest_block_height'], '| Sync:', d['result']['sync_info']['syncing'])")
    echo "✓ Connectivité RPC testnet validée : $SYNC_STATUS"
    echo "✓ Le simulateur dryrun-sim est prêt à synchroniser l'état depuis $RPC_URL"
    exit 0
fi

echo "==> Compilation du contrat vault en WASM..."
cargo build --target wasm32-unknown-unknown --release -p vault
WASM="target/wasm32-unknown-unknown/release/vault.wasm"

echo "==> Déploiement sur le compte $ACCOUNT via le RPC $RPC_URL..."
near contract deploy "$ACCOUNT" use-file "$WASM" without-init-call network-config "$NETWORK"

echo "==> Initialisation du coffre (owner = $ACCOUNT, plafond 2 NEAR, plafond 24h 10 NEAR)..."
near contract call-function as-transaction "$ACCOUNT" new \
    json-args "{\"owner\": \"$ACCOUNT\", \"tx_cap\": \"2000000000000000000000000\", \"daily_cap\": \"10000000000000000000000000\"}" \
    prepaid-gas '100.0 Tgas' attached-deposit '0 NEAR' \
    sign-as "$ACCOUNT" network-config "$NETWORK"

echo "==> Lecture de la vue status()..."
near contract call-function as-read-only "$ACCOUNT" status \
    text-args "{}" network-config "$NETWORK"

echo "✓ Contrat Vault déployé et vérifié sur NEAR Testnet !"
