#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

export PATH="$PATH:$HOME/.local/share/dfx/bin"

echo "== Checking local replica =="
if dfx ping local >/dev/null 2>&1; then
  echo "Local replica is already running."
else
  echo "Starting clean local replica."
  dfx stop >/dev/null 2>&1 || true
  dfx start --background --clean
fi

DEV_PRINCIPAL="$(dfx identity get-principal)"
echo "Developer principal: $DEV_PRINCIPAL"

echo "== Deploying canisters =="
dfx deploy icp_ledger_mock --yes
dfx deploy token --yes --argument "(principal \"$DEV_PRINCIPAL\")"
dfx deploy treasury --yes
dfx deploy lottery --yes
dfx deploy ignis --yes

LEDGER_ID="$(dfx canister id icp_ledger_mock)"
TOKEN_ID="$(dfx canister id token)"
TREASURY_ID="$(dfx canister id treasury)"
LOTTERY_ID="$(dfx canister id lottery)"
IGNIS_ID="$(dfx canister id ignis)"

echo "Ledger:   $LEDGER_ID"
echo "Token:    $TOKEN_ID"
echo "Treasury: $TREASURY_ID"
echo "Lottery:  $LOTTERY_ID"
echo "Ignis:    $IGNIS_ID"

echo "== Linking canisters =="
dfx canister call token set_minting_account "(principal \"$LOTTERY_ID\")"
dfx canister call lottery configure "(record {
  token_canister = opt principal \"$TOKEN_ID\";
  treasury_canister = opt principal \"$TREASURY_ID\";
  ignis_canister = opt principal \"$IGNIS_ID\";
  draw_interval_secs = opt (60 : nat64);
})"
dfx canister call treasury configure "(record {
  lottery_canister = opt principal \"$LOTTERY_ID\";
  icp_ledger = opt principal \"$LEDGER_ID\";
  dev_account = opt record { owner = principal \"$DEV_PRINCIPAL\"; subaccount = null };
  liquidity_account = opt record { owner = principal \"$TREASURY_ID\"; subaccount = null };
  burn_account = opt record { owner = principal \"aaaaa-aa\"; subaccount = null };
  topup_canisters = null;
})"
dfx canister call ignis configure "(record {
  lottery_canister = opt principal \"$LOTTERY_ID\";
  treasury_canister = opt principal \"$TREASURY_ID\";
})"

echo "== Writing frontend canister ids =="
cat > src/frontend/canister-ids.js <<EOF_IDS
export const CANISTER_IDS = {
  local: {
    ledger: "$LEDGER_ID",
    token: "$TOKEN_ID",
    treasury: "$TREASURY_ID",
    lottery: "$LOTTERY_ID",
    ignis: "$IGNIS_ID"
  },
  ic: {
    ledger: "ryjl3-tyaaa-aaaaa-aaaba-cai",
    token: "",
    treasury: "",
    lottery: "",
    ignis: ""
  }
};
EOF_IDS

echo "== Deploying frontend =="
dfx deploy frontend --yes

FRONTEND_ID="$(dfx canister id frontend)"
WEB_PORT="$(dfx info webserver-port 2>/dev/null || echo 8080)"
echo ""
echo "LUCKY local frontend:"
echo "http://$FRONTEND_ID.localhost:$WEB_PORT/"
echo "http://127.0.0.1:$WEB_PORT/?canisterId=$FRONTEND_ID"
