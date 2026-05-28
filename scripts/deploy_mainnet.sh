#!/usr/bin/env bash
set -euo pipefail

NETWORK="ic"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

export PATH="$PATH:$HOME/.local/share/dfx/bin"

# ── Pre-flight ──

echo "== Pre-flight =="
dfx identity whoami
DEV_PRINCIPAL="$(dfx identity get-principal)"
echo "Deploying as: $DEV_PRINCIPAL"
echo "Network: $NETWORK"

WALLET=$(dfx identity get-wallet --network $NETWORK 2>/dev/null || echo "NONE")
echo "Wallet: $WALLET"
if [ "$WALLET" = "NONE" ]; then
  echo "ERROR: No wallet configured for network $NETWORK."
  echo "Run: dfx identity set-wallet <wallet-id> --network $NETWORK"
  exit 1
fi

read -rp "Continue with mainnet deployment? (yes/no): " CONFIRM
[ "$CONFIRM" = "yes" ] || { echo "Aborted."; exit 0; }

# ── Create canisters ──

echo "== Creating canisters =="
for CANISTER in token treasury lottery ignis frontend; do
  dfx canister create "$CANISTER" --network $NETWORK 2>/dev/null && \
    echo "Created $CANISTER" || echo "$CANISTER already exists"
done

# ── Deploy backend canisters ──

echo "== Deploying canisters =="
dfx deploy token --network $NETWORK --argument "(principal \"$DEV_PRINCIPAL\")"
dfx deploy treasury --network $NETWORK
dfx deploy lottery --network $NETWORK
dfx deploy ignis --network $NETWORK

# ── Capture IDs ──

REAL_LEDGER="ryjl3-tyaaa-aaaaa-aaaba-cai"
TOKEN_ID="$(dfx canister id token --network $NETWORK)"
TREASURY_ID="$(dfx canister id treasury --network $NETWORK)"
LOTTERY_ID="$(dfx canister id lottery --network $NETWORK)"
IGNIS_ID="$(dfx canister id ignis --network $NETWORK)"
FRONTEND_ID="$(dfx canister id frontend --network $NETWORK)"

echo ""
echo "Ledger (real): $REAL_LEDGER"
echo "Token:         $TOKEN_ID"
echo "Treasury:      $TREASURY_ID"
echo "Lottery:       $LOTTERY_ID"
echo "Ignis:         $IGNIS_ID"
echo "Frontend:      $FRONTEND_ID"

# ── Configure inter-canister links ──

echo "== Configuring canisters =="

# TODO: Update these principals before production launch
DEV_REVENUE_PRINCIPAL="$DEV_PRINCIPAL"

dfx canister call token set_minting_account \
  "(principal \"$LOTTERY_ID\")" --network $NETWORK

dfx canister call lottery configure "(record {
  token_canister = opt principal \"$TOKEN_ID\";
  treasury_canister = opt principal \"$TREASURY_ID\";
  ignis_canister = opt principal \"$IGNIS_ID\";
  draw_interval_secs = opt (600 : nat64);
})" --network $NETWORK

dfx canister call treasury configure "(record {
  lottery_canister = opt principal \"$LOTTERY_ID\";
  icp_ledger = opt principal \"$REAL_LEDGER\";
  dev_account = opt record { owner = principal \"$DEV_REVENUE_PRINCIPAL\"; subaccount = null };
  burn_account = null;
  topup_canisters = opt vec {
    record { canister_id = principal \"$LOTTERY_ID\"; name = \"Lottery\"; share_bps = (4500 : nat16) };
    record { canister_id = principal \"$FRONTEND_ID\"; name = \"Frontend\"; share_bps = (2000 : nat16) };
    record { canister_id = principal \"$TREASURY_ID\"; name = \"Treasury\"; share_bps = (3500 : nat16) };
  };
})" --network $NETWORK

dfx canister call ignis configure "(record {
  lottery_canister = opt principal \"$LOTTERY_ID\";
  treasury_canister = opt principal \"$TREASURY_ID\";
  llm_canister = opt principal \"w36hm-eqaaa-aaaal-qr76a-cai\";
})" --network $NETWORK

# ── Write frontend canister IDs ──

echo "== Writing frontend canister ids =="
cat > src/frontend/canister-ids.js <<EOF_IDS
export const CANISTER_IDS = {
  local: {
    ledger: "bkyz2-fmaaa-aaaaa-qaaaq-cai",
    token: "be2us-64aaa-aaaaa-qaabq-cai",
    treasury: "br5f7-7uaaa-aaaaa-qaaca-cai",
    lottery: "bw4dl-smaaa-aaaaa-qaacq-cai",
    ignis: "b77ix-eeaaa-aaaaa-qaada-cai"
  },
  ic: {
    ledger: "$REAL_LEDGER",
    token: "$TOKEN_ID",
    treasury: "$TREASURY_ID",
    lottery: "$LOTTERY_ID",
    ignis: "$IGNIS_ID"
  }
};
EOF_IDS

# ── Deploy frontend ──

echo "== Deploying frontend =="
dfx deploy frontend --network $NETWORK

echo ""
echo "========================================="
echo "  LUCKY Parlor deployed to mainnet!"
echo "========================================="
echo ""
echo "Frontend: https://$FRONTEND_ID.ic0.app"
echo ""
echo "== Post-deploy verification =="
echo "  dfx canister call lottery get_stats '()' --network ic"
echo "  dfx canister call lottery get_draw_interval '()' --network ic"
echo "  dfx canister call treasury get_stats '()' --network ic"
echo "  dfx canister call token icrc1_minting_account '()' --network ic"
