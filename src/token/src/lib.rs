use candid::{CandidType, Int, Nat, Principal};
use ic_cdk::{caller, query, update};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

const DECIMALS: u8 = 8;
const ONE_LUCKY: u128 = 100_000_000;
const MAX_SUPPLY: u128 = 100_000_000 * ONE_LUCKY;
const DEV_RESERVE: u128 = 10_000_000 * ONE_LUCKY;
const REWARD_POOL: u128 = 70_000_000 * ONE_LUCKY;
const TRANSFER_FEE: u128 = 10_000;
const TOKEN_NAME: &str = "Lucky";
const TOKEN_SYMBOL: &str = "LUCKY";
const STAKE_LOCK_NANOS: u64 = 86_400 * 1_000_000_000; // 24 hours

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Vec<u8>>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct TransferArgs {
    pub from_subaccount: Option<Vec<u8>>,
    pub to: Account,
    pub amount: Nat,
    pub fee: Option<Nat>,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub enum TransferError {
    BadFee { expected_fee: Nat },
    InsufficientFunds { balance: Nat },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: Nat },
    TemporarilyUnavailable,
    GenericError { error_code: Nat, message: String },
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum MetadataValue {
    Nat(Nat),
    Int(Int),
    Text(String),
    Blob(Vec<u8>),
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct SupportedStandard {
    pub name: String,
    pub url: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StakeBatch {
    pub amount: u128,
    pub staked_at: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StakeBatchInfo {
    pub amount: Nat,
    pub staked_at: u64,
    pub unlocks_at: u64,
    pub unlocked: bool,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StakeInfo {
    pub liquid: Nat,
    pub staked: Nat,
    pub unlockable: Nat,
    pub locked: Nat,
    pub total: Nat,
    pub boost_bps: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct State {
    owner: Principal,
    minting_account: Option<Principal>,
    balances: HashMap<Account, u128>,
    staked: HashMap<Principal, Vec<StakeBatch>>,
    total_minted: u128,
    rewards_minted: u128,
    next_block_index: u128,
}

// Previous format: flat u128 staked amounts
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct OldState {
    owner: Principal,
    minting_account: Option<Principal>,
    balances: HashMap<Account, u128>,
    staked: HashMap<Principal, u128>,
    total_minted: u128,
    rewards_minted: u128,
    next_block_index: u128,
}

// Oldest format: Principal-based balances
#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct LegacyState {
    owner: Principal,
    minting_account: Option<Principal>,
    balances: HashMap<Principal, u128>,
    staked: HashMap<Principal, u128>,
    total_minted: u128,
    rewards_minted: u128,
}

impl Default for State {
    fn default() -> Self {
        Self {
            owner: Principal::anonymous(),
            minting_account: None,
            balances: HashMap::new(),
            staked: HashMap::new(),
            total_minted: 0,
            rewards_minted: 0,
            next_block_index: 0,
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

#[ic_cdk::init]
fn init(dev_reserve: Principal) {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.owner = caller();
        state
            .balances
            .insert(default_account(dev_reserve), DEV_RESERVE);
        state.total_minted = DEV_RESERVE;
    });
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    let state = STATE.with(|s| s.borrow().clone());
    ic_cdk::storage::stable_save((state,)).expect("failed to save token state");
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let (state,): (State,) = ic_cdk::storage::stable_restore()
        .or_else(|_| {
            ic_cdk::storage::stable_restore::<(OldState,)>().map(|(old,)| (migrate_old_state(old),))
        })
        .or_else(|_| {
            ic_cdk::storage::stable_restore::<(LegacyState,)>()
                .map(|(old,)| (migrate_legacy_state(old),))
        })
        .unwrap_or_else(|e| ic_cdk::trap(&format!("CRITICAL: state restore failed: {}", e)));
    STATE.with(|s| *s.borrow_mut() = state);
}

#[update]
fn set_minting_account(lottery_canister: Principal) {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        assert_eq!(caller(), state.owner, "Only owner can set minting account");
        state.minting_account = Some(lottery_canister);
    });
}

// ── ICRC-1 queries ──

#[query]
fn icrc1_name() -> String {
    TOKEN_NAME.to_string()
}

#[query]
fn icrc1_symbol() -> String {
    TOKEN_SYMBOL.to_string()
}

#[query]
fn icrc1_decimals() -> u8 {
    DECIMALS
}

#[query]
fn icrc1_fee() -> Nat {
    Nat::from(TRANSFER_FEE)
}

#[query]
fn icrc1_metadata() -> Vec<(String, MetadataValue)> {
    vec![
        (
            "icrc1:name".to_string(),
            MetadataValue::Text(TOKEN_NAME.to_string()),
        ),
        (
            "icrc1:symbol".to_string(),
            MetadataValue::Text(TOKEN_SYMBOL.to_string()),
        ),
        (
            "icrc1:decimals".to_string(),
            MetadataValue::Nat(Nat::from(DECIMALS)),
        ),
        (
            "icrc1:fee".to_string(),
            MetadataValue::Nat(Nat::from(TRANSFER_FEE)),
        ),
    ]
}

#[query]
fn icrc1_supported_standards() -> Vec<SupportedStandard> {
    vec![SupportedStandard {
        name: "ICRC-1".to_string(),
        url: "https://github.com/dfinity/ICRC-1/tree/main/standards/ICRC-1".to_string(),
    }]
}

#[query]
fn icrc1_total_supply() -> Nat {
    STATE.with(|s| Nat::from(s.borrow().total_minted))
}

#[query]
fn max_supply() -> Nat {
    Nat::from(MAX_SUPPLY)
}

#[query]
fn reward_pool() -> Nat {
    Nat::from(REWARD_POOL)
}

#[query]
fn rewards_minted() -> Nat {
    STATE.with(|s| Nat::from(s.borrow().rewards_minted))
}

#[query]
fn icrc1_minting_account() -> Option<Account> {
    STATE.with(|s| s.borrow().minting_account.map(default_account))
}

#[query]
fn icrc1_balance_of(account: Account) -> Nat {
    STATE.with(|s| Nat::from(*s.borrow().balances.get(&account).unwrap_or(&0)))
}

#[query]
fn staked_balance_of(account: Principal) -> Nat {
    STATE.with(|s| {
        Nat::from(
            s.borrow()
                .staked
                .get(&account)
                .map(|b| total_staked(b))
                .unwrap_or(0),
        )
    })
}

#[query]
fn total_balance_of(account: Principal) -> Nat {
    STATE.with(|s| {
        let state = s.borrow();
        let liquid = *state.balances.get(&default_account(account)).unwrap_or(&0);
        let staked = state
            .staked
            .get(&account)
            .map(|b| total_staked(b))
            .unwrap_or(0);
        Nat::from(liquid + staked)
    })
}

#[query]
fn get_stake_info(account: Principal) -> StakeInfo {
    let now = ic_cdk::api::time();
    STATE.with(|s| stake_info(&s.borrow(), account, now))
}

#[query]
fn get_stake_batches(account: Principal) -> Vec<StakeBatchInfo> {
    let now = ic_cdk::api::time();
    STATE.with(|s| {
        s.borrow()
            .staked
            .get(&account)
            .map(|batches| {
                batches
                    .iter()
                    .map(|b| {
                        let unlocks_at = b.staked_at.saturating_add(STAKE_LOCK_NANOS);
                        StakeBatchInfo {
                            amount: Nat::from(b.amount),
                            staked_at: b.staked_at,
                            unlocks_at,
                            unlocked: now >= unlocks_at,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

#[query]
fn get_boost_bps(account: Principal) -> u64 {
    STATE.with(|s| {
        let staked = s
            .borrow()
            .staked
            .get(&account)
            .map(|b| total_staked(b))
            .unwrap_or(0);
        boost_bps(staked)
    })
}

#[query]
fn get_boost_bps_batch(accounts: Vec<Principal>) -> Vec<u64> {
    STATE.with(|s| {
        let state = s.borrow();
        accounts
            .iter()
            .map(|account| {
                let staked = state
                    .staked
                    .get(account)
                    .map(|b| total_staked(b))
                    .unwrap_or(0);
                boost_bps(staked)
            })
            .collect()
    })
}

#[query]
fn get_cycles_balance() -> u128 {
    ic_cdk::api::canister_balance128()
}

// ── ICRC-1 transfer ──

#[update]
fn icrc1_transfer(args: TransferArgs) -> Result<Nat, TransferError> {
    let from = Account {
        owner: caller(),
        subaccount: args.from_subaccount.clone(),
    };
    let amount = nat_to_u128(&args.amount).ok_or_else(|| generic_error("Amount is too large"))?;
    let fee = args
        .fee
        .as_ref()
        .and_then(nat_to_u128)
        .unwrap_or(TRANSFER_FEE);

    if fee != TRANSFER_FEE {
        return Err(TransferError::BadFee {
            expected_fee: Nat::from(TRANSFER_FEE),
        });
    }

    let total = amount.saturating_add(fee);

    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let balance = *state.balances.get(&from).unwrap_or(&0);
        if balance < total {
            return Err(TransferError::InsufficientFunds {
                balance: Nat::from(balance),
            });
        }

        *state.balances.entry(from).or_insert(0) -= total;
        *state.balances.entry(args.to).or_insert(0) += amount;
        state.total_minted = state.total_minted.saturating_sub(fee);

        let block_index = state.next_block_index;
        state.next_block_index += 1;
        Ok(Nat::from(block_index))
    })
}

// ── Staking ──

#[update]
fn stake(amount: Nat) -> Result<StakeInfo, String> {
    let user = caller();
    let now = ic_cdk::api::time();
    let amount = nat_to_u128(&amount).ok_or("Amount is too large")?;
    if amount == 0 {
        return Err("Amount must be greater than zero".to_string());
    }

    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let account = default_account(user);
        let liquid = *state.balances.get(&account).unwrap_or(&0);
        if liquid < amount {
            return Err("Insufficient liquid LUCKY".to_string());
        }

        *state.balances.entry(account).or_insert(0) -= amount;
        state.staked.entry(user).or_default().push(StakeBatch {
            amount,
            staked_at: now,
        });
        Ok(stake_info(&state, user, now))
    })
}

#[update]
fn unstake(amount: Nat) -> Result<StakeInfo, String> {
    let user = caller();
    let now = ic_cdk::api::time();
    let mut remaining = nat_to_u128(&amount).ok_or("Amount is too large")?;
    if remaining == 0 {
        return Err("Amount must be greater than zero".to_string());
    }

    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let batches = state.staked.entry(user).or_default();

        let unlockable: u128 = batches
            .iter()
            .filter(|b| now >= b.staked_at.saturating_add(STAKE_LOCK_NANOS))
            .map(|b| b.amount)
            .sum();

        if unlockable < remaining {
            return Err(format!(
                "Insufficient unlocked LUCKY. {} unlockable, {} requested",
                unlockable / ONE_LUCKY,
                remaining / ONE_LUCKY
            ));
        }

        // Drain from oldest unlocked batches first (FIFO)
        let mut i = 0;
        while remaining > 0 && i < batches.len() {
            if now < batches[i].staked_at.saturating_add(STAKE_LOCK_NANOS) {
                i += 1;
                continue;
            }
            if batches[i].amount <= remaining {
                remaining -= batches[i].amount;
                batches.remove(i);
            } else {
                batches[i].amount -= remaining;
                remaining = 0;
            }
        }

        let unstaked = nat_to_u128(&amount).ok_or("Amount overflow".to_string())?;
        *state.balances.entry(default_account(user)).or_insert(0) += unstaked;
        Ok(stake_info(&state, user, now))
    })
}

// ── Minting (lottery only) ──

#[update]
fn mint(to: Principal, amount: Nat) -> Result<(), String> {
    let amount = nat_to_u128(&amount).ok_or("Amount is too large")?;
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let minting_account = state.minting_account.ok_or("Minting account not set")?;

        if caller() != minting_account {
            return Err("Only lottery canister can mint rewards".to_string());
        }

        if state.rewards_minted.saturating_add(amount) > REWARD_POOL {
            return Err("Reward pool exhausted".to_string());
        }
        if state.total_minted.saturating_add(amount) > MAX_SUPPLY {
            return Err("Max supply exceeded".to_string());
        }

        *state.balances.entry(default_account(to)).or_insert(0) += amount;
        state.rewards_minted += amount;
        state.total_minted += amount;
        Ok(())
    })
}

// ── Burning ──

#[update]
fn burn(amount: Nat) -> Result<Nat, TransferError> {
    let amount = nat_to_u128(&amount).ok_or_else(|| generic_error("Amount too large"))?;
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let account = default_account(caller());
        let balance = *state.balances.get(&account).unwrap_or(&0);
        if balance < amount {
            return Err(TransferError::InsufficientFunds {
                balance: Nat::from(balance),
            });
        }
        *state.balances.entry(account).or_insert(0) -= amount;
        state.total_minted = state.total_minted.saturating_sub(amount);
        let block_index = state.next_block_index;
        state.next_block_index += 1;
        Ok(Nat::from(block_index))
    })
}

// ── Helpers ──

fn stake_info(state: &State, account: Principal, now: u64) -> StakeInfo {
    let liquid = *state.balances.get(&default_account(account)).unwrap_or(&0);
    let batches = state.staked.get(&account);
    let staked = batches.map(|b| total_staked(b)).unwrap_or(0);
    let unlockable = batches.map(|b| unlockable_amount(b, now)).unwrap_or(0);
    let locked = staked.saturating_sub(unlockable);

    StakeInfo {
        liquid: Nat::from(liquid),
        staked: Nat::from(staked),
        unlockable: Nat::from(unlockable),
        locked: Nat::from(locked),
        total: Nat::from(liquid + staked),
        boost_bps: boost_bps(staked),
    }
}

fn total_staked(batches: &[StakeBatch]) -> u128 {
    batches.iter().map(|b| b.amount).sum()
}

fn unlockable_amount(batches: &[StakeBatch], now: u64) -> u128 {
    batches
        .iter()
        .filter(|b| now >= b.staked_at.saturating_add(STAKE_LOCK_NANOS))
        .map(|b| b.amount)
        .sum()
}

fn boost_bps(staked_e8s: u128) -> u64 {
    let lucky = staked_e8s / ONE_LUCKY;
    match lucky {
        0..=999 => 10_000,
        1_000..=4_999 => 11_000,
        5_000..=9_999 => 12_500,
        10_000..=24_999 => 15_000,
        25_000..=49_999 => 20_000,
        _ => 30_000,
    }
}

fn default_account(owner: Principal) -> Account {
    Account {
        owner,
        subaccount: None,
    }
}

fn migrate_old_state(old: OldState) -> State {
    State {
        owner: old.owner,
        minting_account: old.minting_account,
        balances: old.balances,
        staked: old
            .staked
            .into_iter()
            .filter(|(_, amount)| *amount > 0)
            .map(|(principal, amount)| {
                (
                    principal,
                    vec![StakeBatch {
                        amount,
                        staked_at: 0,
                    }],
                )
            })
            .collect(),
        total_minted: old.total_minted,
        rewards_minted: old.rewards_minted,
        next_block_index: old.next_block_index,
    }
}

fn migrate_legacy_state(old: LegacyState) -> State {
    State {
        owner: old.owner,
        minting_account: old.minting_account,
        balances: old
            .balances
            .into_iter()
            .map(|(owner, balance)| (default_account(owner), balance))
            .collect(),
        staked: old
            .staked
            .into_iter()
            .filter(|(_, amount)| *amount > 0)
            .map(|(principal, amount)| {
                (
                    principal,
                    vec![StakeBatch {
                        amount,
                        staked_at: 0,
                    }],
                )
            })
            .collect(),
        total_minted: old.total_minted,
        rewards_minted: old.rewards_minted,
        next_block_index: 0,
    }
}

fn nat_to_u128(value: &Nat) -> Option<u128> {
    value.0.clone().try_into().ok()
}

fn generic_error(message: &str) -> TransferError {
    TransferError::GenericError {
        error_code: Nat::from(1u8),
        message: message.to_string(),
    }
}
