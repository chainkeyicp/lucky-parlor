#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

DEV="$(dfx identity get-principal)"
LEDGER_ID="$(dfx canister id icp_ledger_mock)"
TOKEN_ID="$(dfx canister id token)"
TREASURY_ID="$(dfx canister id treasury)"
LOTTERY_ID="$(dfx canister id lottery)"

dfx build token
dfx canister install token --mode reinstall --yes --argument "(principal \"$DEV\")"

dfx canister call token set_minting_account "(principal \"$LOTTERY_ID\")"
dfx canister call lottery configure "(record {
  token_canister = opt principal \"$TOKEN_ID\";
  treasury_canister = opt principal \"$TREASURY_ID\";
})"
dfx canister call treasury configure "(record {
  lottery_canister = opt principal \"$LOTTERY_ID\";
  icp_ledger = opt principal \"$LEDGER_ID\";
  dev_account = opt record { owner = principal \"$DEV\"; subaccount = null };
  liquidity_account = opt record { owner = principal \"$TREASURY_ID\"; subaccount = null };
  burn_account = opt record { owner = principal \"aaaaa-aa\"; subaccount = null };
})"
