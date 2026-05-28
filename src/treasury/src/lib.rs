use candid::{CandidType, Nat, Principal};
use ic_cdk::{caller, query, update};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha224};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

const DEV_BPS: u64 = 500;
const CYCLES_BPS: u64 = 9_500; // 95% — merged liquidity (15%) + burn (80%) → all to cycles top-up
const DEFAULT_ICP_FEE_E8S: u64 = 10_000;
const CMC_PRINCIPAL: &str = "rkp4c-7iaaa-aaaaa-aaaca-cai";
const TOPUP_MEMO: u64 = 0x50555054; // "TPUP" — CMC convention

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Vec<u8>>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ConfigureArgs {
    pub lottery_canister: Option<Principal>,
    pub icp_ledger: Option<Principal>,
    pub dev_account: Option<Account>,
    pub burn_account: Option<Account>,
    pub topup_canisters: Option<Vec<TopUpTarget>>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct ClaimResult {
    pub amount_e8s: u64,
    pub dev_e8s: u64,
    pub liquidity_e8s: u64,
    pub burn_e8s: u64,
    pub ledger_fees_e8s: u64,
    pub distribution_complete: bool,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
struct TransferArg {
    from_subaccount: Option<Vec<u8>>,
    to: Account,
    amount: Nat,
    fee: Option<Nat>,
    memo: Option<Vec<u8>>,
    created_at_time: Option<u64>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
enum TransferError {
    BadFee { expected_fee: Nat },
    BadBurn { min_burn_amount: Nat },
    InsufficientFunds { balance: Nat },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: Nat },
    TemporarilyUnavailable,
    GenericError { error_code: Nat, message: String },
}

// ── Legacy ICP ledger transfer types (required by CMC) ──

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct Tokens {
    e8s: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct LegacyTimestamp {
    timestamp_nanos: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct LegacyTransferArgs {
    to: Vec<u8>,
    amount: Tokens,
    fee: Tokens,
    memo: u64,
    from_subaccount: Option<Vec<u8>>,
    created_at_time: Option<LegacyTimestamp>,
}

#[derive(CandidType, Deserialize, Debug)]
enum LegacyTransferError {
    BadFee { expected_fee: Tokens },
    InsufficientFunds { balance: Tokens },
    TxTooOld { allowed_window_nanos: u64 },
    TxCreatedInFuture,
    TxDuplicate { duplicate_of: u64 },
}

#[derive(CandidType, Deserialize, Debug)]
enum LegacyTransferResult {
    Ok(u64),
    Err(LegacyTransferError),
}

// ── CMC notify_top_up types ──

#[derive(CandidType, Serialize, Clone, Debug)]
struct NotifyTopUpArg {
    block_index: u64,
    canister_id: Principal,
}

#[derive(CandidType, Deserialize, Debug)]
enum NotifyError {
    Refunded {
        block_index: Option<u64>,
        reason: String,
    },
    InvalidTransaction(String),
    Other {
        error_message: String,
        error_code: u64,
    },
    Processing,
    TransactionTooOld(u64),
}

#[derive(CandidType, Deserialize, Debug)]
enum NotifyTopUpResult {
    Ok(Nat),
    Err(NotifyError),
}

#[derive(Debug)]
enum TopUpTransferError {
    RefreshTimestamp(String),
    Other(String),
}

impl TopUpTransferError {
    fn message(self) -> String {
        match self {
            TopUpTransferError::RefreshTimestamp(message) | TopUpTransferError::Other(message) => {
                message
            }
        }
    }
}

// ── Top-up target configuration ──

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TopUpTarget {
    pub canister_id: Principal,
    pub name: String,
    pub share_bps: u16,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TopUpStatus {
    pub canister_id: Principal,
    pub name: String,
    pub amount_e8s: u64,
    pub block_index: Option<u64>,
    pub cycles_minted: Option<Nat>,
    pub done: bool,
    pub last_error: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct PendingTopUp {
    canister_id: Principal,
    name: String,
    amount_e8s: u64,
    created_at_time: u64,
    block_index: Option<u64>,
    cycles_minted: Option<Nat>,
    done: bool,
    last_error: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Default)]
pub struct Stats {
    pub total_received: u64,
    pub total_dev: u64,
    pub total_liquidity: u64,
    pub total_burn: u64,
    pub claim_count: u64,
    pub pending_distributions: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct PendingSplit {
    buyer: Principal,
    result: ClaimResult,
    dev_done: bool,
    liquidity_done: bool,
    burn_done: bool,
    #[serde(default)]
    topups: Vec<PendingTopUp>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct State {
    owner: Principal,
    lottery_canister: Option<Principal>,
    icp_ledger: Option<Principal>,
    dev_account: Option<Account>,
    liquidity_account: Option<Account>,
    burn_account: Option<Account>,
    claimed_by_subaccount: HashMap<Vec<u8>, u64>,
    pending_splits: HashMap<Vec<u8>, PendingSplit>,
    #[serde(default)]
    processing_subaccounts: HashSet<Vec<u8>>,
    #[serde(default)]
    topup_canisters: Vec<TopUpTarget>,
    stats: Stats,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct OldState {
    owner: Principal,
    lottery_canister: Option<Principal>,
    icp_ledger: Option<Principal>,
    dev_account: Option<Account>,
    liquidity_account: Option<Account>,
    burn_account: Option<Account>,
    claimed_by_subaccount: HashMap<Vec<u8>, u64>,
    stats: Stats,
}

impl Default for State {
    fn default() -> Self {
        Self {
            owner: Principal::anonymous(),
            lottery_canister: None,
            icp_ledger: None,
            dev_account: None,
            liquidity_account: None,
            burn_account: None,
            claimed_by_subaccount: HashMap::new(),
            pending_splits: HashMap::new(),
            processing_subaccounts: HashSet::new(),
            topup_canisters: Vec::new(),
            stats: Stats::default(),
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// RAII guard that removes a subaccount from `processing_subaccounts` on drop.
struct ProcessingGuard(Vec<u8>);
impl Drop for ProcessingGuard {
    fn drop(&mut self) {
        STATE.with(|s| {
            s.borrow_mut().processing_subaccounts.remove(&self.0);
        });
    }
}

#[ic_cdk::init]
fn init() {
    STATE.with(|s| s.borrow_mut().owner = caller());
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    let state = STATE.with(|s| s.borrow().clone());
    ic_cdk::storage::stable_save((state,)).expect("failed to save treasury state");
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let (mut state,): (State,) = ic_cdk::storage::stable_restore()
        .or_else(|_| {
            ic_cdk::storage::stable_restore::<(OldState,)>().map(|(old,)| (migrate_old_state(old),))
        })
        .unwrap_or_else(|e| ic_cdk::trap(&format!("CRITICAL: state restore failed: {}", e)));
    if state.owner == Principal::anonymous() && caller() != Principal::anonymous() {
        state.owner = caller();
    }
    STATE.with(|s| *s.borrow_mut() = state);
}

#[update]
fn configure(args: ConfigureArgs) {
    with_owner(|| {
        STATE.with(|s| {
            let mut state = s.borrow_mut();
            if let Some(id) = args.lottery_canister {
                state.lottery_canister = Some(id);
            }
            if let Some(id) = args.icp_ledger {
                state.icp_ledger = Some(id);
            }
            if let Some(account) = args.dev_account {
                state.dev_account = Some(account);
            }
            if let Some(account) = args.burn_account {
                state.burn_account = Some(account);
            }
            if let Some(targets) = args.topup_canisters {
                if let Err(message) = validate_topup_targets(&targets) {
                    ic_cdk::trap(&format!("Invalid top-up target configuration: {}", message));
                }
                state.topup_canisters = targets;
            }
        });
    });
}

#[update]
async fn claim_ticket_payment(
    buyer: Principal,
    deposit_subaccount: Vec<u8>,
    amount_e8s: u64,
) -> Result<ClaimResult, String> {
    if deposit_subaccount.len() != 32 {
        return Err("Deposit subaccount must be 32 bytes".to_string());
    }

    let (
        lottery,
        ledger,
        dev_account,
        burn_account,
        topup_enabled,
        existing_pending,
        already_processing,
    ) = STATE.with(|s| {
        let state = s.borrow();
        (
            state.lottery_canister,
            state.icp_ledger,
            state.dev_account.clone(),
            state.burn_account.clone(),
            !state.topup_canisters.is_empty(),
            state.pending_splits.get(&deposit_subaccount).cloned(),
            state.processing_subaccounts.contains(&deposit_subaccount),
        )
    });

    if Some(caller()) != lottery {
        return Err("Only lottery canister can claim ticket payments".to_string());
    }

    // Race condition guard: prevent concurrent claims for the same subaccount
    if already_processing {
        return Err("Payment claim already in progress for this subaccount".to_string());
    }
    STATE.with(|s| {
        s.borrow_mut()
            .processing_subaccounts
            .insert(deposit_subaccount.clone());
    });
    let _guard = ProcessingGuard(deposit_subaccount.clone());

    let ledger = ledger.ok_or("ICP ledger not configured")?;
    let dev_account = dev_account.ok_or("Development account not configured")?;
    if !topup_enabled && burn_account.is_none() {
        return Err("Burn/cycles account not configured".to_string());
    }
    let ledger_fee = get_ledger_fee(ledger).await.unwrap_or(DEFAULT_ICP_FEE_E8S);

    if let Some(pending) = existing_pending {
        let complete = distribute_pending_split(
            ledger,
            deposit_subaccount.clone(),
            dev_account,
            burn_account,
            ledger_fee,
        )
        .await;
        return Err(if complete {
            format!(
                "Payment for {} was already claimed; send a new deposit before buying another ticket",
                pending.buyer
            )
        } else {
            format!(
                "Payment for {} was already claimed and distribution is still pending",
                pending.buyer
            )
        });
    }

    let account = Account {
        owner: ic_cdk::id(),
        subaccount: Some(deposit_subaccount.clone()),
    };

    let (balance,): (Nat,) = ic_cdk::call(ledger, "icrc1_balance_of", (account,))
        .await
        .map_err(|e| format!("Ledger balance query failed: {:?}", e))?;
    let balance = nat_to_u64(&balance).ok_or("Ledger balance too large")?;
    let result = split_amount(amount_e8s, ledger_fee, topup_enabled);
    let required_balance = amount_e8s.saturating_add(result.ledger_fees_e8s);

    if balance < required_balance {
        return Err(format!(
            "Payment not found for {}. Required {} e8s including split fees, balance {} e8s",
            buyer, required_balance, balance
        ));
    }

    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state
            .claimed_by_subaccount
            .insert(deposit_subaccount.clone(), required_balance);
        state.pending_splits.insert(
            deposit_subaccount.clone(),
            PendingSplit {
                buyer,
                result: result.clone(),
                dev_done: false,
                liquidity_done: true, // no liquidity leg — merged into cycles
                burn_done: false,
                topups: Vec::new(),
            },
        );
        state.stats.total_received += result.amount_e8s;
        state.stats.claim_count += 1;
        state.stats.pending_distributions += 1;
    });

    // Phase 1: Do the dev transfer synchronously (fast — single ICRC1 transfer)
    if transfer_from_deposit(
        ledger,
        deposit_subaccount.clone(),
        dev_account.clone(),
        result.dev_e8s,
        ledger_fee,
    )
    .await
    .is_ok()
    {
        mark_split_done(&deposit_subaccount, SplitLeg::Dev, result.dev_e8s);
    }

    // Phase 2: Spawn cycles top-ups asynchronously (slow — multiple CMC transfers)
    let sub = deposit_subaccount.clone();
    ic_cdk::spawn(async move {
        distribute_pending_split(ledger, sub, dev_account, burn_account, ledger_fee).await;
    });

    // Return immediately — top-ups continue in background
    let mut result = result;
    result.distribution_complete = false;
    Ok(result)
}

#[update]
async fn retry_pending_distribution(deposit_subaccount: Vec<u8>) -> Result<bool, String> {
    with_owner(|| {});

    if deposit_subaccount.len() != 32 {
        return Err("Deposit subaccount must be 32 bytes".to_string());
    }
    let already_processing = STATE.with(|s| {
        s.borrow()
            .processing_subaccounts
            .contains(&deposit_subaccount)
    });
    if already_processing {
        return Err("Payment claim already in progress for this subaccount".to_string());
    }
    STATE.with(|s| {
        s.borrow_mut()
            .processing_subaccounts
            .insert(deposit_subaccount.clone());
    });
    let _guard = ProcessingGuard(deposit_subaccount.clone());

    let (ledger, dev_account, burn_account) = STATE.with(|s| {
        let state = s.borrow();
        (
            state.icp_ledger,
            state.dev_account.clone(),
            state.burn_account.clone(),
        )
    });

    let ledger = ledger.ok_or("ICP ledger not configured")?;
    let ledger_fee = get_ledger_fee(ledger).await.unwrap_or(DEFAULT_ICP_FEE_E8S);
    Ok(distribute_pending_split(
        ledger,
        deposit_subaccount,
        dev_account.ok_or("Development account not configured")?,
        burn_account,
        ledger_fee,
    )
    .await)
}

#[query]
fn get_deposit_account(subaccount: Vec<u8>) -> Account {
    Account {
        owner: ic_cdk::id(),
        subaccount: Some(subaccount),
    }
}

#[query]
fn get_stats() -> Stats {
    STATE.with(|s| s.borrow().stats.clone())
}

#[query]
fn get_topup_targets() -> Vec<TopUpTarget> {
    STATE.with(|s| s.borrow().topup_canisters.clone())
}

#[query]
fn get_pending_topups(deposit_subaccount: Vec<u8>) -> Vec<TopUpStatus> {
    STATE.with(|s| {
        s.borrow()
            .pending_splits
            .get(&deposit_subaccount)
            .map(|pending| {
                pending
                    .topups
                    .iter()
                    .map(|topup| TopUpStatus {
                        canister_id: topup.canister_id,
                        name: topup.name.clone(),
                        amount_e8s: topup.amount_e8s,
                        block_index: topup.block_index,
                        cycles_minted: topup.cycles_minted.clone(),
                        done: topup.done,
                        last_error: topup.last_error.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

#[query]
fn get_split() -> (u64, u64, u64) {
    (DEV_BPS, 0, CYCLES_BPS)
}

#[query]
fn get_cycles_balance() -> u128 {
    ic_cdk::api::canister_balance128()
}

async fn distribute_pending_split(
    ledger: Principal,
    deposit_subaccount: Vec<u8>,
    dev_account: Account,
    burn_account: Option<Account>,
    ledger_fee: u64,
) -> bool {
    let pending = STATE.with(|s| s.borrow().pending_splits.get(&deposit_subaccount).cloned());
    let Some(pending) = pending else {
        return true;
    };

    if !pending.dev_done
        && transfer_from_deposit(
            ledger,
            deposit_subaccount.clone(),
            dev_account,
            pending.result.dev_e8s,
            ledger_fee,
        )
        .await
        .is_ok()
    {
        mark_split_done(&deposit_subaccount, SplitLeg::Dev, pending.result.dev_e8s);
    }

    let pending = STATE.with(|s| s.borrow().pending_splits.get(&deposit_subaccount).cloned());
    let Some(pending) = pending else {
        return true;
    };

    if !pending.burn_done {
        let topup_targets = STATE.with(|s| s.borrow().topup_canisters.clone());
        if !pending.topups.is_empty() || !topup_targets.is_empty() {
            if pending.topups.is_empty() {
                if let Err(e) = initialize_pending_topups(
                    &deposit_subaccount,
                    pending.result.burn_e8s,
                    &topup_targets,
                ) {
                    ic_cdk::println!("cycles top-up initialization failed: {}", e);
                    return false;
                }
            }

            if distribute_pending_topups(ledger, deposit_subaccount.clone()).await {
                mark_split_done(&deposit_subaccount, SplitLeg::Burn, pending.result.burn_e8s);
            }
        } else if let Some(burn_account) = burn_account {
            if transfer_from_deposit(
                ledger,
                deposit_subaccount.clone(),
                burn_account,
                pending.result.burn_e8s,
                ledger_fee,
            )
            .await
            .is_ok()
            {
                mark_split_done(&deposit_subaccount, SplitLeg::Burn, pending.result.burn_e8s);
            }
        } else {
            ic_cdk::println!("burn leg pending: no burn account and no top-up targets configured");
        }
    }

    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let Some(pending) = state.pending_splits.get(&deposit_subaccount) else {
            return true;
        };
        if pending.dev_done && pending.liquidity_done && pending.burn_done {
            state.pending_splits.remove(&deposit_subaccount);
            state.claimed_by_subaccount.remove(&deposit_subaccount);
            state.stats.pending_distributions = state.stats.pending_distributions.saturating_sub(1);
            true
        } else {
            false
        }
    })
}

enum SplitLeg {
    Dev,
    Burn,
}

fn mark_split_done(deposit_subaccount: &[u8], leg: SplitLeg, amount_e8s: u64) {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        if let Some(pending) = state.pending_splits.get_mut(deposit_subaccount) {
            match leg {
                SplitLeg::Dev if !pending.dev_done => {
                    pending.dev_done = true;
                    state.stats.total_dev += amount_e8s;
                }
                SplitLeg::Burn if !pending.burn_done => {
                    pending.burn_done = true;
                    state.stats.total_burn += amount_e8s;
                }
                _ => {}
            }
        }
    });
}

async fn get_ledger_fee(ledger: Principal) -> Option<u64> {
    let (fee,): (Nat,) = ic_cdk::call(ledger, "icrc1_fee", ()).await.ok()?;
    nat_to_u64(&fee)
}

async fn transfer_from_deposit(
    ledger: Principal,
    deposit_subaccount: Vec<u8>,
    to: Account,
    amount_e8s: u64,
    ledger_fee_e8s: u64,
) -> Result<Nat, String> {
    if amount_e8s == 0 {
        return Ok(Nat::from(0u8));
    }

    let arg = TransferArg {
        from_subaccount: Some(deposit_subaccount),
        to,
        amount: Nat::from(amount_e8s),
        fee: Some(Nat::from(ledger_fee_e8s)),
        memo: None,
        created_at_time: Some(ic_cdk::api::time()),
    };

    let (result,): (Result<Nat, TransferError>,) = ic_cdk::call(ledger, "icrc1_transfer", (arg,))
        .await
        .map_err(|e| format!("Ledger transfer failed: {:?}", e))?;

    result.map_err(|e| format!("Ledger transfer rejected: {:?}", e))
}

// ── Cycles top-up via CMC ──

fn validate_topup_targets(targets: &[TopUpTarget]) -> Result<(), String> {
    if targets.is_empty() {
        return Ok(());
    }

    let mut total_bps = 0u64;
    let mut seen = HashSet::new();
    for target in targets {
        if target.name.trim().is_empty() {
            return Err("Top-up target name cannot be empty".to_string());
        }
        if target.share_bps == 0 {
            return Err(format!("Top-up target {} has zero share", target.name));
        }
        if !seen.insert(target.canister_id) {
            return Err(format!("Duplicate top-up target {}", target.canister_id));
        }
        total_bps += target.share_bps as u64;
    }

    if total_bps != 10_000 {
        return Err(format!(
            "Top-up shares must sum to 10000 bps, got {}",
            total_bps
        ));
    }

    Ok(())
}

fn initialize_pending_topups(
    deposit_subaccount: &[u8],
    burn_e8s: u64,
    targets: &[TopUpTarget],
) -> Result<(), String> {
    let topups = build_pending_topups(burn_e8s, targets)?;
    STATE.with(|s| -> Result<(), String> {
        let mut state = s.borrow_mut();
        let pending = state
            .pending_splits
            .get_mut(deposit_subaccount)
            .ok_or("Pending split disappeared before top-up initialization")?;
        if pending.topups.is_empty() {
            pending.topups = topups;
        }
        Ok(())
    })
}

fn build_pending_topups(
    burn_e8s: u64,
    targets: &[TopUpTarget],
) -> Result<Vec<PendingTopUp>, String> {
    validate_topup_targets(targets)?;
    if targets.is_empty() {
        return Err("No top-up targets configured".to_string());
    }

    let created_at_time = ic_cdk::api::time();
    let mut allocated = 0u64;
    let last_index = targets.len() - 1;
    let mut topups = Vec::with_capacity(targets.len());

    for (index, target) in targets.iter().enumerate() {
        let amount_e8s = if index == last_index {
            burn_e8s.saturating_sub(allocated)
        } else {
            let share = burn_e8s.saturating_mul(target.share_bps as u64) / 10_000;
            allocated = allocated.saturating_add(share);
            share
        };

        if amount_e8s <= DEFAULT_ICP_FEE_E8S {
            return Err(format!(
                "Top-up share for {} is {} e8s, below the {} e8s ledger fee",
                target.name, amount_e8s, DEFAULT_ICP_FEE_E8S
            ));
        }

        topups.push(PendingTopUp {
            canister_id: target.canister_id,
            name: target.name.clone(),
            amount_e8s,
            created_at_time,
            block_index: None,
            cycles_minted: None,
            done: false,
            last_error: None,
        });
    }

    Ok(topups)
}

fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB88320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn principal_to_subaccount(p: &Principal) -> [u8; 32] {
    let bytes = p.as_slice();
    let mut sub = [0u8; 32];
    sub[0] = bytes.len() as u8;
    sub[1..1 + bytes.len()].copy_from_slice(bytes);
    sub
}

fn compute_account_identifier(owner: &Principal, subaccount: &[u8; 32]) -> Vec<u8> {
    let mut hasher = Sha224::new();
    hasher.update(b"\x0Aaccount-id");
    hasher.update(owner.as_slice());
    hasher.update(subaccount);
    let hash = hasher.finalize();
    let crc = crc32_ieee(&hash);
    let mut result = Vec::with_capacity(32);
    result.extend_from_slice(&crc.to_be_bytes());
    result.extend_from_slice(&hash);
    result
}

fn cmc_account_for_canister(canister_id: &Principal) -> Vec<u8> {
    let cmc = Principal::from_text(CMC_PRINCIPAL).unwrap();
    let subaccount = principal_to_subaccount(canister_id);
    compute_account_identifier(&cmc, &subaccount)
}

async fn distribute_pending_topups(ledger: Principal, deposit_subaccount: Vec<u8>) -> bool {
    loop {
        let next = STATE.with(|s| {
            let state = s.borrow();
            state
                .pending_splits
                .get(&deposit_subaccount)
                .and_then(|pending| {
                    pending
                        .topups
                        .iter()
                        .enumerate()
                        .find(|(_, topup)| !topup.done)
                        .map(|(index, topup)| (index, topup.clone()))
                })
        });

        let Some((index, topup)) = next else {
            return true;
        };

        let block_index = match topup.block_index {
            Some(block_index) => block_index,
            None => match transfer_topup_to_cmc(ledger, deposit_subaccount.clone(), &topup).await {
                Ok(block_index) => {
                    mark_topup_block_index(&deposit_subaccount, index, block_index);
                    block_index
                }
                Err(e) => {
                    let refresh_timestamp = matches!(e, TopUpTransferError::RefreshTimestamp(_));
                    let message = e.message();
                    if refresh_timestamp {
                        refresh_topup_created_at_time(&deposit_subaccount, index);
                    }
                    mark_topup_failed(&deposit_subaccount, index, message);
                    return false;
                }
            },
        };

        match notify_topup_via_cmc(block_index, topup.canister_id).await {
            Ok(cycles) => {
                mark_topup_done(&deposit_subaccount, index, cycles);
            }
            Err(e) => {
                mark_topup_failed(&deposit_subaccount, index, e);
                return false;
            }
        }
    }
}

fn mark_topup_block_index(deposit_subaccount: &[u8], index: usize, block_index: u64) {
    STATE.with(|s| {
        if let Some(topup) = s
            .borrow_mut()
            .pending_splits
            .get_mut(deposit_subaccount)
            .and_then(|pending| pending.topups.get_mut(index))
        {
            topup.block_index = Some(block_index);
            topup.last_error = None;
        }
    });
}

fn mark_topup_done(deposit_subaccount: &[u8], index: usize, cycles: Nat) {
    STATE.with(|s| {
        if let Some(topup) = s
            .borrow_mut()
            .pending_splits
            .get_mut(deposit_subaccount)
            .and_then(|pending| pending.topups.get_mut(index))
        {
            topup.done = true;
            topup.cycles_minted = Some(cycles);
            topup.last_error = None;
        }
    });
}

fn mark_topup_failed(deposit_subaccount: &[u8], index: usize, error: String) {
    STATE.with(|s| {
        if let Some(topup) = s
            .borrow_mut()
            .pending_splits
            .get_mut(deposit_subaccount)
            .and_then(|pending| pending.topups.get_mut(index))
        {
            topup.last_error = Some(error);
        }
    });
}

fn refresh_topup_created_at_time(deposit_subaccount: &[u8], index: usize) {
    STATE.with(|s| {
        if let Some(topup) = s
            .borrow_mut()
            .pending_splits
            .get_mut(deposit_subaccount)
            .and_then(|pending| pending.topups.get_mut(index))
        {
            topup.created_at_time = ic_cdk::api::time();
        }
    });
}

async fn transfer_topup_to_cmc(
    ledger: Principal,
    from_subaccount: Vec<u8>,
    topup: &PendingTopUp,
) -> Result<u64, TopUpTransferError> {
    let net_amount = topup.amount_e8s.saturating_sub(DEFAULT_ICP_FEE_E8S);
    if net_amount == 0 {
        return Err(TopUpTransferError::Other(
            "Amount too small after fee".to_string(),
        ));
    }

    let cmc_account = cmc_account_for_canister(&topup.canister_id);

    let transfer_args = LegacyTransferArgs {
        to: cmc_account,
        amount: Tokens { e8s: net_amount },
        fee: Tokens {
            e8s: DEFAULT_ICP_FEE_E8S,
        },
        memo: TOPUP_MEMO,
        from_subaccount: Some(from_subaccount),
        created_at_time: Some(LegacyTimestamp {
            timestamp_nanos: topup.created_at_time,
        }),
    };

    let (result,): (LegacyTransferResult,) = ic_cdk::call(ledger, "transfer", (transfer_args,))
        .await
        .map_err(|e| TopUpTransferError::Other(format!("Legacy transfer call failed: {:?}", e)))?;

    match result {
        LegacyTransferResult::Ok(idx) => Ok(idx),
        LegacyTransferResult::Err(LegacyTransferError::TxDuplicate { duplicate_of }) => {
            Ok(duplicate_of)
        }
        LegacyTransferResult::Err(LegacyTransferError::TxTooOld { .. }) => {
            Err(TopUpTransferError::RefreshTimestamp(
                "Legacy transfer timestamp too old; refreshed timestamp for next retry".to_string(),
            ))
        }
        LegacyTransferResult::Err(e) => Err(TopUpTransferError::Other(format!(
            "Legacy transfer error: {:?}",
            e
        ))),
    }
}

async fn notify_topup_via_cmc(block_index: u64, target_canister: Principal) -> Result<Nat, String> {
    let cmc =
        Principal::from_text(CMC_PRINCIPAL).map_err(|e| format!("Bad CMC principal: {}", e))?;
    let (notify_result,): (NotifyTopUpResult,) = ic_cdk::call(
        cmc,
        "notify_top_up",
        (NotifyTopUpArg {
            block_index,
            canister_id: target_canister,
        },),
    )
    .await
    .map_err(|e| format!("CMC notify call failed: {:?}", e))?;

    match notify_result {
        NotifyTopUpResult::Ok(cycles) => {
            ic_cdk::println!(
                "Cycles top-up OK: {} cycles for {}",
                cycles,
                target_canister
            );
            Ok(cycles)
        }
        NotifyTopUpResult::Err(e) => Err(format!("CMC notify error: {:?}", e)),
    }
}

fn split_amount(amount_e8s: u64, ledger_fee_e8s: u64, topup_enabled: bool) -> ClaimResult {
    let dev_e8s = amount_e8s * DEV_BPS / 10_000;
    let burn_e8s = amount_e8s.saturating_sub(dev_e8s);
    // When cycles top-up is enabled: 1 ICRC1 fee (dev) — legacy transfer fee comes from burn_e8s
    // When using burn account fallback: 2 ICRC1 fees (dev + burn transfer)
    let transfer_fee_count = if topup_enabled { 1 } else { 2 };

    ClaimResult {
        amount_e8s,
        dev_e8s,
        liquidity_e8s: 0,
        burn_e8s,
        ledger_fees_e8s: ledger_fee_e8s * transfer_fee_count,
        distribution_complete: false,
    }
}

fn with_owner<F: FnOnce()>(f: F) {
    let owner = STATE.with(|s| s.borrow().owner);
    assert_eq!(caller(), owner, "Only owner");
    f();
}

fn nat_to_u64(value: &Nat) -> Option<u64> {
    value.0.clone().try_into().ok()
}

fn migrate_old_state(old: OldState) -> State {
    State {
        owner: old.owner,
        lottery_canister: old.lottery_canister,
        icp_ledger: old.icp_ledger,
        dev_account: old.dev_account,
        liquidity_account: old.liquidity_account,
        burn_account: old.burn_account,
        claimed_by_subaccount: old.claimed_by_subaccount,
        pending_splits: HashMap::new(),
        processing_subaccounts: HashSet::new(),
        topup_canisters: Vec::new(),
        stats: old.stats,
    }
}
