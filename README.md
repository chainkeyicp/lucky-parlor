# LUCKY

LUCKY is an Internet Computer lottery and ICP burn protocol.

Users buy paid ICP tickets. Each paid ticket is active for 24 hours and enters recurring on-chain draws. Every active ticket has equal draw odds. The winner receives a controlled LUCKY token reward, and holders can stake LUCKY to multiply the reward size if their ticket wins.

## Core Model

- Ticket price: `0.001 ICP`
- Ticket duration: `24 hours`
- Draw interval: `1 minute` locally for testing; raise back to `10 minutes` before production.
- Base reward formula: `50 LUCKY + 10 LUCKY per active player`
- Base reward cap per draw: `500 LUCKY`
- Winner reward: `base reward * reward multiplier`
- Daily emission cap: `500,000 LUCKY`
- Revenue accounting:
  - `80%` ICP burn allocation
  - `15%` liquidity allocation
  - `5%` development allocation

## LUCKY Utility

LUCKY can be staked to multiply the winner reward. It does not change draw odds.

| Staked LUCKY | Reward multiplier |
| ---: | ---: |
| 0 | 1.0x |
| 1,000 | 1.1x |
| 5,000 | 1.25x |
| 10,000 | 1.5x |
| 25,000 | 2.0x |
| 50,000 | 3.0x max |

Simple pitch:

```text
Burn ICP. Win LUCKY. Stake LUCKY to multiply your reward.
```

## Canisters

- `icp_ledger_mock`: local ICRC-style ICP ledger for testing.
- `token`: LUCKY token, staking, rewards minting.
- `treasury`: verifies ICP ticket deposits and accounts for burn/liquidity/dev split.
- `lottery`: ticket lifecycle, timer draws, uniform randomness, reward emissions.
- `ignis`: living burn companion, rituals, draw commentary, and burn-stage state.
- `frontend`: static wallet UI using DFINITY agents and local self-authenticating identities for testing.

## Local Development

Run from WSL:

```bash
./scripts/deploy_local.sh
```

The script starts a clean local replica, deploys canisters, links them, writes frontend canister ids, and deploys the frontend assets.

Local ticket purchases use `0.001 ICP`, plus one user transfer fee and three treasury split transfer fees. This makes the money flow realistic: the treasury can move the paid ICP out of the deposit subaccount into the dev, liquidity, and burn allocations.

The reward model is intentionally LUCKY-denominated and controlled by emissions. The earlier "20 ICP worth of LUCKY" idea is not active in this MVP because it depends on reliable market pricing and can become unsustainable before there is deep liquidity.

For local testing, the frontend uses the mock ICP ledger faucet. On mainnet, replace the mock ledger principal with the real ICP ledger canister:

```text
ryjl3-tyaaa-aaaaa-aaaba-cai
```

The local login creates an Ed25519 identity in browser storage, so each browser profile/incognito window can act as a different participant principal. Use `New Principal` to rotate the local test identity in the same browser.

## Mainnet Status

This project is production-shaped but still needs mainnet-specific integrations before real funds:

- Real ICP ledger configuration and audit.
- Real cycles minting / ICP burn flow through the Cycles Minting Canister.
- Real ICPSwap liquidity integration.
- Internet Identity login for production users.
- Full ICRC-1/ICRC-2 LUCKY token compliance if external wallet/exchange support is required.
- Security review for payment claiming, replay protection, upgrade persistence, and draw fairness.
