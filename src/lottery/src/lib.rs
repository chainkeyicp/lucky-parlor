use candid::{CandidType, Nat, Principal};
use ic_cdk::{caller, query, update};
use ic_cdk_timers::{set_timer_interval, TimerId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;

const ONE_LUCKY: u128 = 100_000_000;
const DRAWS_PER_STEP: u64 = 14;
const STEP_E8S: u64 = 10_000_000; // 0.1 ICP per step
const MIN_TICKET_E8S: u64 = 10_000_000; // 0.1 ICP
const MAX_TICKET_E8S: u64 = 100_000_000; // 1.0 ICP
const DEFAULT_DRAW_INTERVAL_SECS: u64 = 600;
const DAY_NANOS: u64 = 86_400 * 1_000_000_000;

const BASE_REWARD_LUCKY: u128 = 50 * ONE_LUCKY;
const PER_PLAYER_REWARD_LUCKY: u128 = 10 * ONE_LUCKY;
const MAX_REWARD_LUCKY: u128 = 500 * ONE_LUCKY;
const DAILY_EMISSION_CAP_LUCKY: u128 = 500_000 * ONE_LUCKY;
const REWARD_POOL_LUCKY: u128 = 70_000_000 * ONE_LUCKY;

const BURN_TAX_BPS: u128 = 2_000; // 20% of reward → burn pool
const BURN_EPOCH_NANOS: u64 = 7 * DAY_NANOS; // weekly burn

const STREAK_BOOST_3: u64 = 500; // +5% at 3+ consecutive days
const STREAK_BOOST_7: u64 = 1_500; // +15% at 7+ consecutive days
const STREAK_BOOST_14: u64 = 2_500; // +25% at 14+ consecutive days
const STREAK_BOOST_30: u64 = 3_000; // +30% at 30+ consecutive days

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Vec<u8>>,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ConfigureArgs {
    pub token_canister: Option<Principal>,
    pub treasury_canister: Option<Principal>,
    pub ignis_canister: Option<Principal>,
    pub draw_interval_secs: Option<u64>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct IgnisDrawEvent {
    pub round: u64,
    pub winner: Principal,
    pub reward_lucky_e8s: Nat,
    pub active_tickets: u64,
    pub active_players: u64,
    pub winning_weight_bps: u64,
    pub timestamp: u64,
    pub ignis_won: bool,
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

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct DrawResult {
    pub round: u64,
    pub winner: Principal,
    pub reward_lucky_e8s: Nat,
    pub active_tickets: u64,
    pub active_players: u64,
    pub winning_weight_bps: u64,
    pub timestamp: u64,
    pub ticket_principals: Vec<Principal>,
    pub ticket_ids: Vec<u64>,
    pub winning_ticket_id: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct LegacyDrawResult {
    round: u64,
    winner: Principal,
    reward_lucky_e8s: Nat,
    active_tickets: u64,
    active_players: u64,
    winning_weight_bps: u64,
    timestamp: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TicketInfo {
    pub ticket_id: u64,
    pub purchased_at: u64,
    pub remaining_draws: u64,
    pub total_draws: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TicketStatus {
    pub active_tickets: u64,
    pub min_remaining_draws: Option<u64>,
    pub boost_bps: u64,
    pub streak_days: u64,
    pub streak_boost_bps: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StreakInfo {
    pub consecutive_days: u64,
    pub streak_boost_bps: u64,
    pub last_purchase_day: u64,
    pub current_day: u64,
    pub streak_alive: bool,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, Default)]
struct StreakData {
    last_purchase_day: u64,
    consecutive_days: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct LotteryStats {
    pub round: u64,
    pub active_tickets: u64,
    pub active_players: u64,
    pub emitted_today_lucky_e8s: Nat,
    pub daily_cap_lucky_e8s: Nat,
    pub total_rewards_minted_lucky_e8s: Nat,
    pub halving_level: u64,
    pub burn_pool_lucky_e8s: Nat,
    pub total_burned_lucky_e8s: Nat,
    pub last_burn_epoch: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct Ticket {
    id: u64,
    owner: Principal,
    purchased_at: u64,
    remaining_draws: u64,
    total_draws: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct LegacyTicket {
    id: u64,
    owner: Principal,
    purchased_at: u64,
    expires_at: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct State {
    owner: Principal,
    token_canister: Option<Principal>,
    treasury_canister: Option<Principal>,
    ignis_canister: Option<Principal>,
    tickets: Vec<Ticket>,
    next_ticket_id: u64,
    round: u64,
    draw_history: VecDeque<DrawResult>,
    emitted_day: u64,
    emitted_today_lucky_e8s: u128,
    total_rewards_minted_lucky_e8s: u128,
    streaks: HashMap<Principal, StreakData>,
    burn_pool_lucky_e8s: u128,
    last_burn_epoch: u64,
    total_burned_lucky_e8s: u128,
    #[serde(default = "default_draw_interval")]
    draw_interval_secs: u64,
}

fn default_draw_interval() -> u64 {
    DEFAULT_DRAW_INTERVAL_SECS
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct PreBurnState {
    owner: Principal,
    token_canister: Option<Principal>,
    treasury_canister: Option<Principal>,
    ignis_canister: Option<Principal>,
    tickets: Vec<Ticket>,
    next_ticket_id: u64,
    round: u64,
    draw_history: VecDeque<DrawResult>,
    emitted_day: u64,
    emitted_today_lucky_e8s: u128,
    total_rewards_minted_lucky_e8s: u128,
    streaks: HashMap<Principal, StreakData>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct PreDrawsState {
    owner: Principal,
    token_canister: Option<Principal>,
    treasury_canister: Option<Principal>,
    ignis_canister: Option<Principal>,
    tickets: Vec<LegacyTicket>,
    next_ticket_id: u64,
    round: u64,
    draw_history: VecDeque<DrawResult>,
    emitted_day: u64,
    emitted_today_lucky_e8s: u128,
    total_rewards_minted_lucky_e8s: u128,
    streaks: HashMap<Principal, StreakData>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct PreStreakState {
    owner: Principal,
    token_canister: Option<Principal>,
    treasury_canister: Option<Principal>,
    ignis_canister: Option<Principal>,
    tickets: Vec<LegacyTicket>,
    next_ticket_id: u64,
    round: u64,
    draw_history: VecDeque<DrawResult>,
    emitted_day: u64,
    emitted_today_lucky_e8s: u128,
    total_rewards_minted_lucky_e8s: u128,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct PreRevealState {
    owner: Principal,
    token_canister: Option<Principal>,
    treasury_canister: Option<Principal>,
    ignis_canister: Option<Principal>,
    tickets: Vec<LegacyTicket>,
    next_ticket_id: u64,
    round: u64,
    draw_history: VecDeque<LegacyDrawResult>,
    emitted_day: u64,
    emitted_today_lucky_e8s: u128,
    total_rewards_minted_lucky_e8s: u128,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct PreIgnisState {
    owner: Principal,
    token_canister: Option<Principal>,
    treasury_canister: Option<Principal>,
    tickets: Vec<LegacyTicket>,
    next_ticket_id: u64,
    round: u64,
    draw_history: VecDeque<LegacyDrawResult>,
    emitted_day: u64,
    emitted_today_lucky_e8s: u128,
    total_rewards_minted_lucky_e8s: u128,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct OldState {
    owner: Principal,
    token_canister: Option<Principal>,
    treasury_canister: Option<Principal>,
    tickets: Vec<LegacyTicket>,
    next_ticket_id: u64,
    round: u64,
    draw_history: Vec<LegacyDrawResult>,
    emitted_day: u64,
    emitted_today_lucky_e8s: u128,
}

impl Default for State {
    fn default() -> Self {
        Self {
            owner: Principal::anonymous(),
            token_canister: None,
            treasury_canister: None,
            ignis_canister: None,
            tickets: Vec::new(),
            next_ticket_id: 0,
            round: 0,
            draw_history: VecDeque::new(),
            emitted_day: 0,
            emitted_today_lucky_e8s: 0,
            total_rewards_minted_lucky_e8s: 0,
            streaks: HashMap::new(),
            burn_pool_lucky_e8s: 0,
            last_burn_epoch: 0,
            total_burned_lucky_e8s: 0,
            draw_interval_secs: DEFAULT_DRAW_INTERVAL_SECS,
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    static DRAW_TIMER_ID: RefCell<Option<TimerId>> = const { RefCell::new(None) };
    static DRAW_IN_PROGRESS: RefCell<bool> = const { RefCell::new(false) };
}

#[ic_cdk::init]
fn init() {
    STATE.with(|s| s.borrow_mut().owner = caller());
    schedule_draw_timer();
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    let state = STATE.with(|s| s.borrow().clone());
    ic_cdk::storage::stable_save((state,)).expect("failed to save lottery state");
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let (state,): (State,) = ic_cdk::storage::stable_restore()
        .or_else(|_| {
            ic_cdk::storage::stable_restore::<(PreBurnState,)>()
                .map(|(old,)| (migrate_pre_burn_state(old),))
        })
        .or_else(|_| {
            ic_cdk::storage::stable_restore::<(PreDrawsState,)>()
                .map(|(old,)| (migrate_pre_draws_state(old),))
        })
        .or_else(|_| {
            ic_cdk::storage::stable_restore::<(PreStreakState,)>()
                .map(|(old,)| (migrate_pre_streak_state(old),))
        })
        .or_else(|_| {
            ic_cdk::storage::stable_restore::<(PreRevealState,)>()
                .map(|(old,)| (migrate_pre_reveal_state(old),))
        })
        .or_else(|_| {
            ic_cdk::storage::stable_restore::<(PreIgnisState,)>()
                .map(|(old,)| (migrate_pre_ignis_state(old),))
        })
        .or_else(|_| {
            ic_cdk::storage::stable_restore::<(OldState,)>().map(|(old,)| (migrate_old_state(old),))
        })
        .unwrap_or_else(|e| {
            ic_cdk::trap(&format!(
                "CRITICAL: state restore failed, all formats exhausted: {}",
                e
            ))
        });
    STATE.with(|s| *s.borrow_mut() = state);
    schedule_draw_timer();
}

#[update]
fn configure(args: ConfigureArgs) {
    with_owner(|| {
        let mut reschedule = false;
        STATE.with(|s| {
            let mut state = s.borrow_mut();
            if let Some(id) = args.token_canister {
                state.token_canister = Some(id);
            }
            if let Some(id) = args.treasury_canister {
                state.treasury_canister = Some(id);
            }
            if let Some(id) = args.ignis_canister {
                state.ignis_canister = Some(id);
            }
            if let Some(interval) = args.draw_interval_secs {
                if interval > 0 && interval != state.draw_interval_secs {
                    state.draw_interval_secs = interval;
                    reschedule = true;
                }
            }
        });
        if reschedule {
            schedule_draw_timer();
        }
    });
}

#[query]
fn get_deposit_subaccount(user: Principal) -> Vec<u8> {
    deposit_subaccount(user)
}

#[query]
fn get_deposit_account(user: Principal) -> Result<Account, String> {
    let treasury = STATE
        .with(|s| s.borrow().treasury_canister)
        .ok_or("Treasury canister not configured")?;
    Ok(Account {
        owner: treasury,
        subaccount: Some(deposit_subaccount(user)),
    })
}

#[update]
async fn buy_ticket(amount_e8s: u64) -> Result<ClaimResult, String> {
    if !(MIN_TICKET_E8S..=MAX_TICKET_E8S).contains(&amount_e8s) {
        return Err("Amount must be between 0.1 and 1.0 ICP".to_string());
    }
    let draws = (amount_e8s / STEP_E8S) * DRAWS_PER_STEP;
    if draws == 0 {
        return Err("Amount too small for any draws".to_string());
    }

    let buyer = caller();
    let treasury = STATE
        .with(|s| s.borrow().treasury_canister)
        .ok_or("Treasury canister not configured")?;
    let deposit_subaccount = deposit_subaccount(buyer);

    let (claim_result,): (Result<ClaimResult, String>,) = ic_cdk::call(
        treasury,
        "claim_ticket_payment",
        (buyer, deposit_subaccount, amount_e8s),
    )
    .await
    .map_err(|e| format!("Treasury claim failed: {:?}", e))?;

    let claim_result = claim_result?;
    let now = ic_cdk::api::time();

    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let ticket_id = state.next_ticket_id;
        state.next_ticket_id += 1;
        state.tickets.push(Ticket {
            id: ticket_id,
            owner: buyer,
            purchased_at: now,
            remaining_draws: draws,
            total_draws: draws,
        });
        prune_exhausted(&mut state);

        // Streak tracking
        let today = now / DAY_NANOS;
        let entry = state.streaks.entry(buyer).or_default();
        if entry.last_purchase_day == today {
            // Already purchased today — no change
        } else if entry.last_purchase_day + 1 == today {
            // Consecutive day — extend streak
            entry.consecutive_days += 1;
            entry.last_purchase_day = today;
        } else {
            // First purchase ever or streak broken — start fresh
            entry.consecutive_days = 1;
            entry.last_purchase_day = today;
        }
    });

    // Notify IGNIS about the burn (fire-and-forget)
    let burn_e8s = claim_result.burn_e8s;
    if burn_e8s > 0 && claim_result.distribution_complete {
        if let Some(ignis) = STATE.with(|s| s.borrow().ignis_canister) {
            ic_cdk::spawn(async move {
                let _ =
                    ic_cdk::call::<_, (Result<(), String>,)>(ignis, "on_burn", (burn_e8s,)).await;
            });
        }
    }

    Ok(claim_result)
}

#[update]
async fn admin_run_draw() -> Result<Option<DrawResult>, String> {
    assert_owner();
    do_draw().await
}

#[update]
fn admin_reset_daily_emission() {
    assert_owner();
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.emitted_today_lucky_e8s = 0;
        state.emitted_day = 0;
    });
}

#[query]
fn get_active_ticket_count() -> u64 {
    STATE.with(|s| active_tickets(&s.borrow()).len() as u64)
}

#[query]
fn get_active_player_count() -> u64 {
    STATE.with(|s| active_player_count(&s.borrow()))
}

#[query]
fn get_draw_history() -> Vec<DrawResult> {
    STATE.with(|s| s.borrow().draw_history.iter().cloned().collect())
}

#[query]
fn get_round() -> u64 {
    STATE.with(|s| s.borrow().round)
}

#[update]
async fn get_ticket_status(user: Principal) -> TicketStatus {
    let (token, active_count, min_remaining) = STATE.with(|s| {
        let state = s.borrow();
        let active: Vec<&Ticket> = state
            .tickets
            .iter()
            .filter(|ticket| ticket.owner == user && ticket.remaining_draws > 0)
            .collect();
        (
            state.token_canister,
            active.len() as u64,
            active.iter().map(|ticket| ticket.remaining_draws).min(),
        )
    });

    let boost_bps = match token {
        Some(token) => match ic_cdk::call::<_, (u64,)>(token, "get_boost_bps", (user,)).await {
            Ok((boost,)) => boost,
            Err(_) => 10_000,
        },
        None => 10_000,
    };

    let (streak_days, s_boost_bps) = STATE.with(|s| {
        let state = s.borrow();
        let today = ic_cdk::api::time() / DAY_NANOS;
        let days = effective_streak(&state.streaks, &user, today);
        (days, streak_boost_bps(days))
    });

    TicketStatus {
        active_tickets: active_count,
        min_remaining_draws: min_remaining,
        boost_bps,
        streak_days,
        streak_boost_bps: s_boost_bps,
    }
}

#[query]
fn get_my_tickets(user: Principal) -> Vec<TicketInfo> {
    STATE.with(|s| {
        s.borrow()
            .tickets
            .iter()
            .filter(|t| t.owner == user && t.remaining_draws > 0)
            .map(|t| TicketInfo {
                ticket_id: t.id,
                purchased_at: t.purchased_at,
                remaining_draws: t.remaining_draws,
                total_draws: t.total_draws,
            })
            .collect()
    })
}

#[query]
fn get_stats() -> LotteryStats {
    STATE.with(|s| {
        let state = s.borrow();
        LotteryStats {
            round: state.round,
            active_tickets: active_tickets(&state).len() as u64,
            active_players: active_player_count(&state),
            emitted_today_lucky_e8s: Nat::from(state.emitted_today_lucky_e8s),
            daily_cap_lucky_e8s: Nat::from(current_daily_cap(state.total_rewards_minted_lucky_e8s)),
            total_rewards_minted_lucky_e8s: Nat::from(state.total_rewards_minted_lucky_e8s),
            halving_level: halving_level(state.total_rewards_minted_lucky_e8s) as u64,
            burn_pool_lucky_e8s: Nat::from(state.burn_pool_lucky_e8s),
            total_burned_lucky_e8s: Nat::from(state.total_burned_lucky_e8s),
            last_burn_epoch: state.last_burn_epoch,
        }
    })
}

async fn do_draw() -> Result<Option<DrawResult>, String> {
    // Prevent concurrent draws (timer vs admin_run_draw overlap)
    let was_in_progress = DRAW_IN_PROGRESS.with(|d| {
        let prev = *d.borrow();
        *d.borrow_mut() = true;
        prev
    });
    if was_in_progress {
        return Err("Draw already in progress".to_string());
    }

    let result = do_draw_inner().await;

    DRAW_IN_PROGRESS.with(|d| *d.borrow_mut() = false);
    result
}

async fn do_draw_inner() -> Result<Option<DrawResult>, String> {
    let now = ic_cdk::api::time();
    let (token_canister, ignis_canister, active) = STATE.with(|s| {
        let mut state = s.borrow_mut();
        prune_exhausted(&mut state);
        (
            state.token_canister,
            state.ignis_canister,
            active_tickets(&state),
        )
    });

    if active.is_empty() {
        return Ok(None);
    }

    let token = token_canister.ok_or("Token canister not configured")?;

    // Uniform random: every ticket has equal chance. Boosts multiply rewards only.
    let randomness = raw_rand_u128().await?;
    let winner_idx = (randomness % active.len() as u128) as usize;
    let winner = active[winner_idx].owner;

    let stake_boost_bps = match ic_cdk::call::<_, (u64,)>(token, "get_boost_bps", (winner,)).await {
        Ok((boost,)) => boost.clamp(10_000, 30_000),
        Err(_) => 10_000,
    };
    let active_players = unique_player_count(&active);
    let ticket_principals: Vec<Principal> = active.iter().map(|ticket| ticket.owner).collect();
    let ticket_ids: Vec<u64> = active.iter().map(|ticket| ticket.id).collect();
    let winning_ticket_id = active[winner_idx].id;
    if !has_reward_capacity(now) {
        return Ok(None);
    }

    let ritual_boost_bps = consume_ritual_reward_boost(ignis_canister, winner).await;
    let streak_bps = STATE.with(|s| {
        let state = s.borrow();
        let today = now / DAY_NANOS;
        streak_boost_bps(effective_streak(&state.streaks, &winner, today))
    });
    let reward_multiplier_bps = stake_boost_bps
        .saturating_add(ritual_boost_bps)
        .saturating_add(streak_bps)
        .min(33_500);
    let reward = reserve_reward(now, active_players, reward_multiplier_bps)?;
    if reward == 0 {
        return Ok(None);
    }

    let burn_tax = reward * BURN_TAX_BPS / 10_000;
    let winner_reward = reward - burn_tax;

    // Mint winner's portion — rollback reserved reward on ANY failure
    let mint_outcome =
        ic_cdk::call::<_, (Result<(), String>,)>(token, "mint", (winner, Nat::from(winner_reward)))
            .await;
    match mint_outcome {
        Err(reject) => {
            rollback_reward(now, reward);
            return Err(format!("Reward mint rejected: {:?}", reject));
        }
        Ok((Err(err),)) => {
            rollback_reward(now, reward);
            return Err(err);
        }
        Ok((Ok(()),)) => {}
    }

    // Mint burn tax to lottery canister's own balance (burn pool) — best effort
    if burn_tax > 0 {
        let self_id = ic_cdk::id();
        let pool_outcome =
            ic_cdk::call::<_, (Result<(), String>,)>(token, "mint", (self_id, Nat::from(burn_tax)))
                .await;
        if matches!(pool_outcome, Ok((Ok(()),))) {
            STATE.with(|s| s.borrow_mut().burn_pool_lucky_e8s += burn_tax);
        } else {
            ic_cdk::println!("burn pool mint failed (best-effort): {:?}", pool_outcome);
        }
    }

    let result = STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.round += 1;
        let result = DrawResult {
            round: state.round,
            winner,
            reward_lucky_e8s: Nat::from(reward),
            active_tickets: active.len() as u64,
            active_players,
            winning_weight_bps: reward_multiplier_bps,
            timestamp: now,
            ticket_principals,
            ticket_ids,
            winning_ticket_id,
        };
        state.draw_history.push_back(result.clone());
        if state.draw_history.len() > 100 {
            state.draw_history.pop_front();
        }
        // Decrement remaining_draws on all active tickets
        for ticket in state.tickets.iter_mut() {
            if ticket.remaining_draws > 0 {
                ticket.remaining_draws -= 1;
            }
        }
        state.tickets.retain(|t| t.remaining_draws > 0);
        result
    });

    // Notify IGNIS (fire-and-forget, never blocks the draw)
    if let Some(ignis) = ignis_canister {
        let ignis_event = IgnisDrawEvent {
            round: result.round,
            winner: result.winner,
            reward_lucky_e8s: result.reward_lucky_e8s.clone(),
            active_tickets: result.active_tickets,
            active_players: result.active_players,
            winning_weight_bps: result.winning_weight_bps,
            timestamp: result.timestamp,
            ignis_won: false,
        };
        ic_cdk::spawn(async move {
            let _ =
                ic_cdk::call::<_, (Result<(), String>,)>(ignis, "on_draw", (ignis_event,)).await;
        });
    }

    // Check if epoch burn is due
    maybe_execute_burn().await;

    Ok(Some(result))
}

async fn maybe_execute_burn() {
    let now = ic_cdk::api::time();
    let (pool, last_epoch, token) = STATE.with(|s| {
        let st = s.borrow();
        (
            st.burn_pool_lucky_e8s,
            st.last_burn_epoch,
            st.token_canister,
        )
    });

    if pool == 0 || now.saturating_sub(last_epoch) < BURN_EPOCH_NANOS {
        return;
    }
    let Some(token) = token else { return };

    let result: Result<(Result<Nat, String>,), _> =
        ic_cdk::call(token, "burn", (Nat::from(pool),)).await;

    if matches!(result, Ok((Ok(_),))) {
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.total_burned_lucky_e8s += pool;
            st.burn_pool_lucky_e8s = 0;
            st.last_burn_epoch = now;
        });
    }
}

async fn consume_ritual_reward_boost(ignis: Option<Principal>, winner: Principal) -> u64 {
    let Some(ignis) = ignis else {
        return 0;
    };

    match ic_cdk::call::<_, (u64,)>(ignis, "consume_ritual_boost", (winner,)).await {
        Ok((boost,)) => boost.min(500),
        Err(_) => 0,
    }
}

async fn raw_rand_u128() -> Result<u128, String> {
    let bytes = raw_rand_bytes().await?;
    let mut seed = [0u8; 16];
    for (idx, byte) in bytes.iter().take(16).enumerate() {
        seed[idx] = *byte;
    }
    Ok(u128::from_le_bytes(seed))
}

async fn raw_rand_bytes() -> Result<Vec<u8>, String> {
    let (bytes,): (Vec<u8>,) = ic_cdk::api::management_canister::main::raw_rand()
        .await
        .map_err(|e| format!("raw_rand failed: {:?}", e))?;
    Ok(bytes)
}

fn reserve_reward(now: u64, active_players: u64, boost_bps: u64) -> Result<u128, String> {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let day = now / DAY_NANOS;
        if state.emitted_day != day {
            state.emitted_day = day;
            state.emitted_today_lucky_e8s = 0;
        }

        let base = current_reward_target(state.total_rewards_minted_lucky_e8s, active_players);
        let boosted = base * boost_bps as u128 / 10_000;
        let remaining_daily = current_daily_cap(state.total_rewards_minted_lucky_e8s)
            .saturating_sub(state.emitted_today_lucky_e8s);
        let remaining_pool = REWARD_POOL_LUCKY.saturating_sub(state.total_rewards_minted_lucky_e8s);
        let reward = boosted.min(remaining_daily).min(remaining_pool);
        state.emitted_today_lucky_e8s += reward;
        state.total_rewards_minted_lucky_e8s += reward;
        Ok(reward)
    })
}

fn has_reward_capacity(now: u64) -> bool {
    STATE.with(|s| {
        let state = s.borrow();
        let emitted_today = if state.emitted_day == now / DAY_NANOS {
            state.emitted_today_lucky_e8s
        } else {
            0
        };
        current_daily_cap(state.total_rewards_minted_lucky_e8s) > emitted_today
            && REWARD_POOL_LUCKY > state.total_rewards_minted_lucky_e8s
    })
}

fn rollback_reward(now: u64, reward: u128) {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let day = now / DAY_NANOS;
        if state.emitted_day == day {
            state.emitted_today_lucky_e8s = state.emitted_today_lucky_e8s.saturating_sub(reward);
        }
        state.total_rewards_minted_lucky_e8s =
            state.total_rewards_minted_lucky_e8s.saturating_sub(reward);
    });
}

fn current_reward_target(total_rewards_minted: u128, active_players: u64) -> u128 {
    let divisor = halving_divisor(total_rewards_minted);
    let target = (BASE_REWARD_LUCKY + active_players as u128 * PER_PLAYER_REWARD_LUCKY)
        .min(MAX_REWARD_LUCKY);
    (target / divisor).max(ONE_LUCKY)
}

fn current_daily_cap(total_rewards_minted: u128) -> u128 {
    (DAILY_EMISSION_CAP_LUCKY / halving_divisor(total_rewards_minted)).max(ONE_LUCKY)
}

fn halving_divisor(total_rewards_minted: u128) -> u128 {
    1u128 << halving_level(total_rewards_minted).min(32)
}

fn halving_level(total_rewards_minted: u128) -> u32 {
    let mut level = 0u32;
    let mut tranche = REWARD_POOL_LUCKY / 2;
    let mut threshold = tranche;

    while tranche > ONE_LUCKY && total_rewards_minted >= threshold {
        level += 1;
        tranche /= 2;
        threshold = threshold.saturating_add(tranche);
    }

    level
}

fn active_tickets(state: &State) -> Vec<Ticket> {
    state
        .tickets
        .iter()
        .filter(|ticket| ticket.remaining_draws > 0)
        .cloned()
        .collect()
}

fn active_player_count(state: &State) -> u64 {
    unique_player_count(&active_tickets(state))
}

fn unique_player_count(tickets: &[Ticket]) -> u64 {
    tickets
        .iter()
        .map(|ticket| ticket.owner)
        .collect::<HashSet<_>>()
        .len() as u64
}

fn prune_exhausted(state: &mut State) {
    state.tickets.retain(|ticket| ticket.remaining_draws > 0);
}

fn deposit_subaccount(user: Principal) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(b"lucky-ticket-deposit");
    hasher.update(user.as_slice());
    hasher.finalize().to_vec()
}

#[query]
fn get_streak_info(user: Principal) -> StreakInfo {
    let now = ic_cdk::api::time();
    let today = now / DAY_NANOS;
    STATE.with(|s| {
        let state = s.borrow();
        let consecutive = effective_streak(&state.streaks, &user, today);
        let last_day = state
            .streaks
            .get(&user)
            .map(|d| d.last_purchase_day)
            .unwrap_or(0);
        let alive = last_day == today || last_day + 1 == today;
        StreakInfo {
            consecutive_days: consecutive,
            streak_boost_bps: streak_boost_bps(consecutive),
            last_purchase_day: last_day,
            current_day: today,
            streak_alive: alive,
        }
    })
}

fn streak_boost_bps(consecutive_days: u64) -> u64 {
    match consecutive_days {
        0..=2 => 0,
        3..=6 => STREAK_BOOST_3,
        7..=13 => STREAK_BOOST_7,
        14..=29 => STREAK_BOOST_14,
        _ => STREAK_BOOST_30,
    }
}

fn effective_streak(
    streaks: &HashMap<Principal, StreakData>,
    user: &Principal,
    current_day: u64,
) -> u64 {
    match streaks.get(user) {
        Some(data)
            if data.last_purchase_day == current_day
                || data.last_purchase_day + 1 == current_day =>
        {
            data.consecutive_days
        }
        Some(_) => 0,
        None => 0,
    }
}

fn schedule_draw_timer() {
    DRAW_TIMER_ID.with(|t| {
        if let Some(old_id) = t.borrow_mut().take() {
            ic_cdk_timers::clear_timer(old_id);
        }
    });
    let interval = STATE.with(|s| s.borrow().draw_interval_secs);
    let id = set_timer_interval(Duration::from_secs(interval), || {
        ic_cdk::spawn(async {
            let balance = ic_cdk::api::canister_balance128();
            if balance < 1_000_000_000_000 {
                ic_cdk::println!("WARNING: cycles balance low: {}", balance);
            }
            if let Err(err) = do_draw().await {
                ic_cdk::println!("draw skipped: {}", err);
            }
        });
    });
    DRAW_TIMER_ID.with(|t| *t.borrow_mut() = Some(id));
}

#[query]
fn get_draw_interval() -> u64 {
    STATE.with(|s| s.borrow().draw_interval_secs)
}

#[query]
fn get_cycles_balance() -> u128 {
    ic_cdk::api::canister_balance128()
}

fn with_owner<F: FnOnce()>(f: F) {
    assert_owner();
    f();
}

fn assert_owner() {
    let owner = STATE.with(|s| s.borrow().owner);
    assert_eq!(caller(), owner, "Only owner");
}

fn migrate_draw_result(old: LegacyDrawResult) -> DrawResult {
    DrawResult {
        round: old.round,
        winner: old.winner,
        reward_lucky_e8s: old.reward_lucky_e8s,
        active_tickets: old.active_tickets,
        active_players: old.active_players,
        winning_weight_bps: old.winning_weight_bps,
        timestamp: old.timestamp,
        ticket_principals: Vec::new(),
        ticket_ids: Vec::new(),
        winning_ticket_id: 0,
    }
}

fn migrate_legacy_tickets(tickets: Vec<LegacyTicket>) -> Vec<Ticket> {
    let now = ic_cdk::api::time();
    tickets
        .into_iter()
        .filter(|t| t.expires_at > now)
        .map(|t| {
            let remaining_nanos = t.expires_at.saturating_sub(now);
            let remaining_draws = (remaining_nanos / (60 * 1_000_000_000)).clamp(1, 144);
            Ticket {
                id: t.id,
                owner: t.owner,
                purchased_at: t.purchased_at,
                remaining_draws,
                total_draws: remaining_draws,
            }
        })
        .collect()
}

fn migrate_pre_burn_state(old: PreBurnState) -> State {
    State {
        owner: old.owner,
        token_canister: old.token_canister,
        treasury_canister: old.treasury_canister,
        ignis_canister: old.ignis_canister,
        tickets: old.tickets,
        next_ticket_id: old.next_ticket_id,
        round: old.round,
        draw_history: old.draw_history,
        emitted_day: old.emitted_day,
        emitted_today_lucky_e8s: old.emitted_today_lucky_e8s,
        total_rewards_minted_lucky_e8s: old.total_rewards_minted_lucky_e8s,
        streaks: old.streaks,
        burn_pool_lucky_e8s: 0,
        last_burn_epoch: 0,
        total_burned_lucky_e8s: 0,
        draw_interval_secs: DEFAULT_DRAW_INTERVAL_SECS,
    }
}

fn migrate_pre_draws_state(old: PreDrawsState) -> State {
    State {
        owner: old.owner,
        token_canister: old.token_canister,
        treasury_canister: old.treasury_canister,
        ignis_canister: old.ignis_canister,
        tickets: migrate_legacy_tickets(old.tickets),
        next_ticket_id: old.next_ticket_id,
        round: old.round,
        draw_history: old.draw_history,
        emitted_day: old.emitted_day,
        emitted_today_lucky_e8s: old.emitted_today_lucky_e8s,
        total_rewards_minted_lucky_e8s: old.total_rewards_minted_lucky_e8s,
        streaks: old.streaks,
        burn_pool_lucky_e8s: 0,
        last_burn_epoch: 0,
        total_burned_lucky_e8s: 0,
        draw_interval_secs: DEFAULT_DRAW_INTERVAL_SECS,
    }
}

fn migrate_pre_streak_state(old: PreStreakState) -> State {
    State {
        owner: old.owner,
        token_canister: old.token_canister,
        treasury_canister: old.treasury_canister,
        ignis_canister: old.ignis_canister,
        tickets: migrate_legacy_tickets(old.tickets),
        next_ticket_id: old.next_ticket_id,
        round: old.round,
        draw_history: old.draw_history,
        emitted_day: old.emitted_day,
        emitted_today_lucky_e8s: old.emitted_today_lucky_e8s,
        total_rewards_minted_lucky_e8s: old.total_rewards_minted_lucky_e8s,
        streaks: HashMap::new(),
        burn_pool_lucky_e8s: 0,
        last_burn_epoch: 0,
        total_burned_lucky_e8s: 0,
        draw_interval_secs: DEFAULT_DRAW_INTERVAL_SECS,
    }
}

fn migrate_pre_reveal_state(old: PreRevealState) -> State {
    State {
        owner: old.owner,
        token_canister: old.token_canister,
        treasury_canister: old.treasury_canister,
        ignis_canister: old.ignis_canister,
        tickets: migrate_legacy_tickets(old.tickets),
        next_ticket_id: old.next_ticket_id,
        round: old.round,
        draw_history: old
            .draw_history
            .into_iter()
            .map(migrate_draw_result)
            .collect(),
        emitted_day: old.emitted_day,
        emitted_today_lucky_e8s: old.emitted_today_lucky_e8s,
        total_rewards_minted_lucky_e8s: old.total_rewards_minted_lucky_e8s,
        streaks: HashMap::new(),
        burn_pool_lucky_e8s: 0,
        last_burn_epoch: 0,
        total_burned_lucky_e8s: 0,
        draw_interval_secs: DEFAULT_DRAW_INTERVAL_SECS,
    }
}

fn migrate_pre_ignis_state(old: PreIgnisState) -> State {
    State {
        owner: old.owner,
        token_canister: old.token_canister,
        treasury_canister: old.treasury_canister,
        ignis_canister: None,
        tickets: migrate_legacy_tickets(old.tickets),
        next_ticket_id: old.next_ticket_id,
        round: old.round,
        draw_history: old
            .draw_history
            .into_iter()
            .map(migrate_draw_result)
            .collect(),
        emitted_day: old.emitted_day,
        emitted_today_lucky_e8s: old.emitted_today_lucky_e8s,
        total_rewards_minted_lucky_e8s: old.total_rewards_minted_lucky_e8s,
        streaks: HashMap::new(),
        burn_pool_lucky_e8s: 0,
        last_burn_epoch: 0,
        total_burned_lucky_e8s: 0,
        draw_interval_secs: DEFAULT_DRAW_INTERVAL_SECS,
    }
}

fn migrate_old_state(old: OldState) -> State {
    State {
        owner: old.owner,
        token_canister: old.token_canister,
        treasury_canister: old.treasury_canister,
        ignis_canister: None,
        tickets: migrate_legacy_tickets(old.tickets),
        next_ticket_id: old.next_ticket_id,
        round: old.round,
        draw_history: old
            .draw_history
            .into_iter()
            .map(migrate_draw_result)
            .collect(),
        emitted_day: old.emitted_day,
        emitted_today_lucky_e8s: old.emitted_today_lucky_e8s,
        total_rewards_minted_lucky_e8s: 0,
        streaks: HashMap::new(),
        burn_pool_lucky_e8s: 0,
        last_burn_epoch: 0,
        total_burned_lucky_e8s: 0,
        draw_interval_secs: DEFAULT_DRAW_INTERVAL_SECS,
    }
}
