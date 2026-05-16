#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

DEV="$(dfx identity get-principal)"

echo "== Token standards =="
dfx canister call token icrc1_supported_standards

echo "== Dev LUCKY balance =="
dfx canister call token icrc1_balance_of "(record { owner = principal \"$DEV\"; subaccount = null })"

echo "== Lottery stats =="
dfx canister call lottery get_stats

echo "== Treasury stats =="
dfx canister call treasury get_stats

