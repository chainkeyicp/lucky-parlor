#!/usr/bin/env bash
set -euo pipefail

NETWORK="ic"

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

export PATH="$PATH:$HOME/.local/share/dfx/bin"

echo "========================================="
echo "  Cycles Wallet Setup for ICP Mainnet"
echo "========================================="
echo ""

# ── Step 1: Check identity ──

echo "== Step 1: Identity =="
IDENTITY=$(dfx identity whoami)
PRINCIPAL=$(dfx identity get-principal)
echo "Identity:  $IDENTITY"
echo "Principal: $PRINCIPAL"
echo ""

# ── Step 2: Check ICP balance ──

echo "== Step 2: ICP Balance =="
echo "Your account identifier (for ICP transfers):"
dfx ledger account-id --network $NETWORK
echo ""

BALANCE=$(dfx ledger balance --network $NETWORK 2>/dev/null || echo "0.00000000 ICP")
echo "Current balance: $BALANCE"

# Extract numeric value
BALANCE_NUM=$(echo "$BALANCE" | grep -oP '[\d.]+' | head -1)
if (( $(echo "$BALANCE_NUM < 0.3" | bc -l 2>/dev/null || echo 1) )); then
  echo ""
  echo "WARNING: You need at least 0.3 ICP to create a cycles wallet."
  echo "Transfer ICP to the account identifier shown above."
  echo ""
  read -rp "Press Enter once you've transferred ICP, or Ctrl+C to abort..."
  BALANCE=$(dfx ledger balance --network $NETWORK)
  echo "Updated balance: $BALANCE"
fi

# ── Step 3: Check/Create cycles wallet ──

echo ""
echo "== Step 3: Cycles Wallet =="

WALLET=$(dfx identity get-wallet --network $NETWORK 2>/dev/null || echo "NONE")
if [ "$WALLET" != "NONE" ]; then
  echo "Wallet already exists: $WALLET"
  WALLET_BALANCE=$(dfx wallet balance --network $NETWORK 2>/dev/null || echo "unknown")
  echo "Wallet balance: $WALLET_BALANCE"
  echo ""
  echo "Setup complete! Your wallet is ready."
  exit 0
fi

echo "No cycles wallet found. Creating one..."
echo "This will use ~0.1 ICP to create a canister + convert to cycles."
echo ""
read -rp "Continue? (yes/no): " CONFIRM
[ "$CONFIRM" = "yes" ] || { echo "Aborted."; exit 0; }

# Create canister with 0.2 ICP (gives ~2.6T cycles)
echo "Creating canister with 0.2 ICP..."
CANISTER_ID=$(dfx ledger create-canister "$PRINCIPAL" --amount 0.2 --network $NETWORK 2>&1 | grep -oP 'Canister created with id: "\K[^"]+' || echo "")

if [ -z "$CANISTER_ID" ]; then
  echo "Trying alternative parsing..."
  CREATE_OUTPUT=$(dfx ledger create-canister "$PRINCIPAL" --amount 0.2 --network $NETWORK 2>&1)
  echo "$CREATE_OUTPUT"
  CANISTER_ID=$(echo "$CREATE_OUTPUT" | grep -oP '[a-z0-9]{5}-[a-z0-9]{5}-[a-z0-9]{5}-[a-z0-9]{5}-[a-z0-9]{3}' | head -1 || echo "")
fi

if [ -z "$CANISTER_ID" ]; then
  echo "ERROR: Could not parse canister ID from output."
  echo "Run manually:"
  echo "  dfx ledger create-canister \"$PRINCIPAL\" --amount 0.2 --network ic"
  echo "  dfx identity deploy-wallet <canister-id> --network ic"
  exit 1
fi

echo "Created canister: $CANISTER_ID"
echo "Deploying wallet code..."
dfx identity deploy-wallet "$CANISTER_ID" --network $NETWORK

echo ""
echo "== Verification =="
dfx identity get-wallet --network $NETWORK
dfx wallet balance --network $NETWORK

echo ""
echo "========================================="
echo "  Cycles wallet ready!"
echo "========================================="
echo ""
echo "Next step: run scripts/deploy_mainnet.sh"
