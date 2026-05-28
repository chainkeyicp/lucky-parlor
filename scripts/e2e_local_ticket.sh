#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

export PATH="$PATH:$HOME/.local/share/dfx/bin"

DEV="$(dfx identity get-principal)"
TREASURY_ID="$(dfx canister id treasury)"

SUBACCOUNT="$(dfx canister call lottery get_deposit_subaccount "(principal \"$DEV\")" \
  | tr -d '\n' \
  | sed -E 's/^[[:space:]]*\([[:space:]]*//; s/,[[:space:]]*\)[[:space:]]*$//')"

echo "== Funding local test account =="
dfx canister call icp_ledger_mock faucet "(record { owner = principal \"$DEV\"; subaccount = null }, 100_000_000)"

echo "== Paying ticket deposit =="
dfx canister call icp_ledger_mock icrc1_transfer "(record {
  from_subaccount = null;
  to = record { owner = principal \"$TREASURY_ID\"; subaccount = opt $SUBACCOUNT };
  amount = 10_030_000;
  fee = opt 10_000;
  memo = null;
  created_at_time = null;
})"

echo "== Buying ticket =="
dfx canister call lottery buy_ticket "(10_000_000 : nat64)"

echo "== Treasury stats =="
dfx canister call treasury get_stats

echo "== Active ticket count =="
dfx canister call lottery get_active_ticket_count
