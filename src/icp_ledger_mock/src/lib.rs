use candid::{CandidType, Nat, Principal};
use ic_cdk::{caller, query, update};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

const ICP_FEE_E8S: u128 = 10_000;

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Vec<u8>>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct TransferArg {
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
    BadBurn { min_burn_amount: Nat },
    InsufficientFunds { balance: Nat },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: Nat },
    TemporarilyUnavailable,
    GenericError { error_code: Nat, message: String },
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Transfer {
    pub index: Nat,
    pub from: Account,
    pub to: Account,
    pub amount: Nat,
    pub fee: Nat,
    pub timestamp: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct State {
    owner: Principal,
    balances: HashMap<Account, u128>,
    transfers: Vec<Transfer>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            owner: Principal::anonymous(),
            balances: HashMap::new(),
            transfers: Vec::new(),
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
    ic_cdk::storage::stable_save((state,)).expect("failed to save ledger state");
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let (state,): (State,) =
        ic_cdk::storage::stable_restore().unwrap_or_else(|_| (State::default(),));
    STATE.with(|s| *s.borrow_mut() = state);
}

#[update]
fn faucet(to: Account, amount: Nat) -> Result<Nat, String> {
    let amount = nat_to_u128(&amount).ok_or("Amount is too large")?;
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        *state.balances.entry(to.clone()).or_insert(0) += amount;
        let index = Nat::from(state.transfers.len() as u64);
        state.transfers.push(Transfer {
            index: index.clone(),
            from: Account {
                owner: Principal::anonymous(),
                subaccount: None,
            },
            to,
            amount: Nat::from(amount),
            fee: Nat::from(0u8),
            timestamp: ic_cdk::api::time(),
        });
        Ok(index)
    })
}

#[query]
fn icrc1_balance_of(account: Account) -> Nat {
    STATE.with(|s| Nat::from(*s.borrow().balances.get(&account).unwrap_or(&0)))
}

#[query]
fn icrc1_fee() -> Nat {
    Nat::from(ICP_FEE_E8S)
}

#[update]
fn icrc1_transfer(args: TransferArg) -> Result<Nat, TransferError> {
    let amount = nat_to_u128(&args.amount).ok_or_else(|| TransferError::GenericError {
        error_code: Nat::from(1u8),
        message: "Amount is too large".to_string(),
    })?;

    let fee = args
        .fee
        .as_ref()
        .and_then(nat_to_u128)
        .unwrap_or(ICP_FEE_E8S);

    if fee != ICP_FEE_E8S {
        return Err(TransferError::BadFee {
            expected_fee: Nat::from(ICP_FEE_E8S),
        });
    }

    let from = Account {
        owner: caller(),
        subaccount: args.from_subaccount.clone(),
    };
    let total = amount.saturating_add(fee);

    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let balance = *state.balances.get(&from).unwrap_or(&0);
        if balance < total {
            return Err(TransferError::InsufficientFunds {
                balance: Nat::from(balance),
            });
        }

        *state.balances.entry(from.clone()).or_insert(0) -= total;
        *state.balances.entry(args.to.clone()).or_insert(0) += amount;

        let index = Nat::from(state.transfers.len() as u64);
        state.transfers.push(Transfer {
            index: index.clone(),
            from,
            to: args.to,
            amount: Nat::from(amount),
            fee: Nat::from(fee),
            timestamp: ic_cdk::api::time(),
        });

        Ok(index)
    })
}

#[query]
fn get_transactions() -> Vec<Transfer> {
    STATE.with(|s| s.borrow().transfers.clone())
}

fn nat_to_u128(value: &Nat) -> Option<u128> {
    value.0.clone().try_into().ok()
}

