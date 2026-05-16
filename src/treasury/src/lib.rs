use candid::{CandidType, Nat, Principal};
use ic_cdk::{caller, query, update};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

const DEV_BPS: u64 = 500;
const LIQUIDITY_BPS: u64 = 1_500;
const BURN_BPS: u64 = 8_000;
const DEFAULT_ICP_FEE_E8S: u64 = 10_000;

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
    pub liquidity_account: Option<Account>,
    pub burn_account: Option<Account>,
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
            stats: Stats::default(),
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
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
            ic_cdk::storage::stable_restore::<(OldState,)>()
                .map(|(old,)| (migrate_old_state(old),))
        })
        .unwrap_or_else(|_| (State::default(),));
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
            if let Some(account) = args.liquidity_account {
                state.liquidity_account = Some(account);
            }
            if let Some(account) = args.burn_account {
                state.burn_account = Some(account);
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

    let (lottery, ledger, dev_account, liquidity_account, burn_account, existing_pending) = STATE.with(|s| {
        let state = s.borrow();
        (
            state.lottery_canister,
            state.icp_ledger,
            state.dev_account.clone(),
            state.liquidity_account.clone(),
            state.burn_account.clone(),
            state.pending_splits.get(&deposit_subaccount).cloned(),
        )
    });

    if Some(caller()) != lottery {
        return Err("Only lottery canister can claim ticket payments".to_string());
    }

    let ledger = ledger.ok_or("ICP ledger not configured")?;
    let dev_account = dev_account.ok_or("Development account not configured")?;
    let liquidity_account = liquidity_account.ok_or("Liquidity account not configured")?;
    let burn_account = burn_account.ok_or("Burn account not configured")?;
    let ledger_fee = get_ledger_fee(ledger).await.unwrap_or(DEFAULT_ICP_FEE_E8S);

    if let Some(pending) = existing_pending {
        let complete = distribute_pending_split(
            ledger,
            deposit_subaccount.clone(),
            dev_account,
            liquidity_account,
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
    let result = split_amount(amount_e8s, ledger_fee);
    let required_balance = amount_e8s.saturating_add(result.ledger_fees_e8s);

    if balance < required_balance {
        return Err(format!(
            "Payment not found for {}. Required {} e8s including split fees, balance {} e8s",
            buyer, required_balance, balance
        ));
    }

    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.claimed_by_subaccount.insert(deposit_subaccount.clone(), required_balance);
        state.pending_splits.insert(
            deposit_subaccount.clone(),
            PendingSplit {
                buyer,
                result: result.clone(),
                dev_done: false,
                liquidity_done: false,
                burn_done: false,
            },
        );
        state.stats.total_received += result.amount_e8s;
        state.stats.claim_count += 1;
        state.stats.pending_distributions += 1;
    });

    let complete = distribute_pending_split(
        ledger,
        deposit_subaccount,
        dev_account,
        liquidity_account,
        burn_account,
        ledger_fee,
    )
    .await;

    let mut result = result;
    result.distribution_complete = complete;
    Ok(result)
}

#[update]
async fn retry_pending_distribution(deposit_subaccount: Vec<u8>) -> Result<bool, String> {
    with_owner(|| {});

    let (ledger, dev_account, liquidity_account, burn_account) = STATE.with(|s| {
        let state = s.borrow();
        (
            state.icp_ledger,
            state.dev_account.clone(),
            state.liquidity_account.clone(),
            state.burn_account.clone(),
        )
    });

    let ledger = ledger.ok_or("ICP ledger not configured")?;
    let ledger_fee = get_ledger_fee(ledger).await.unwrap_or(DEFAULT_ICP_FEE_E8S);
    Ok(distribute_pending_split(
        ledger,
        deposit_subaccount,
        dev_account.ok_or("Development account not configured")?,
        liquidity_account.ok_or("Liquidity account not configured")?,
        burn_account.ok_or("Burn account not configured")?,
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
fn get_split() -> (u64, u64, u64) {
    (DEV_BPS, LIQUIDITY_BPS, BURN_BPS)
}

async fn distribute_pending_split(
    ledger: Principal,
    deposit_subaccount: Vec<u8>,
    dev_account: Account,
    liquidity_account: Account,
    burn_account: Account,
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

    if !pending.liquidity_done
        && transfer_from_deposit(
            ledger,
            deposit_subaccount.clone(),
            liquidity_account,
            pending.result.liquidity_e8s,
            ledger_fee,
        )
        .await
        .is_ok()
    {
        mark_split_done(
            &deposit_subaccount,
            SplitLeg::Liquidity,
            pending.result.liquidity_e8s,
        );
    }

    let pending = STATE.with(|s| s.borrow().pending_splits.get(&deposit_subaccount).cloned());
    let Some(pending) = pending else {
        return true;
    };

    if !pending.burn_done
        && transfer_from_deposit(
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
    Liquidity,
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
                SplitLeg::Liquidity if !pending.liquidity_done => {
                    pending.liquidity_done = true;
                    state.stats.total_liquidity += amount_e8s;
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

    let (result,): (Result<Nat, TransferError>,) =
        ic_cdk::call(ledger, "icrc1_transfer", (arg,))
            .await
            .map_err(|e| format!("Ledger transfer failed: {:?}", e))?;

    result.map_err(|e| format!("Ledger transfer rejected: {:?}", e))
}

fn split_amount(amount_e8s: u64, ledger_fee_e8s: u64) -> ClaimResult {
    let dev_e8s = amount_e8s * DEV_BPS / 10_000;
    let liquidity_e8s = amount_e8s * LIQUIDITY_BPS / 10_000;
    let burn_e8s = amount_e8s.saturating_sub(dev_e8s + liquidity_e8s);

    ClaimResult {
        amount_e8s,
        dev_e8s,
        liquidity_e8s,
        burn_e8s,
        ledger_fees_e8s: ledger_fee_e8s * 3,
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
        stats: old.stats,
    }
}
