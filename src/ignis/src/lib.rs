use candid::{CandidType, Nat, Principal};
use ic_cdk::{caller, query, update};
use ic_cdk_timers::set_timer_interval;
use ic_llm::ChatMessage;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

// ── Constants ──

const MOOD_DECAY_INTERVAL_SECS: u64 = 300;
const HUNGER_RATE_PER_TICK: u8 = 5;
const MAX_NOTABLE_EVENTS: usize = 50;
const MAX_REMEMBERED_PLAYERS: usize = 20;
const MAX_CHRONICLE_ENTRIES: usize = 50;
const MAX_CACHED_COMMENTARIES: usize = 100;
const RITUAL_EXPIRY_NANOS: u64 = 300 * 1_000_000_000; // 5 minutes
const RITUAL_BOOST_MAX_BPS: u64 = 500;
const CHRONICLE_INTERVAL_ROUNDS: u64 = 10;
const ONE_ICP_E8S: u64 = 100_000_000;

// Evolution thresholds in ICP (not e8s)
const FLAME_THRESHOLD_ICP: u64 = 100;
const INFERNO_THRESHOLD_ICP: u64 = 1_000;
const SUPERNOVA_THRESHOLD_ICP: u64 = 10_000;
const SINGULARITY_THRESHOLD_ICP: u64 = 100_000;

// ── Types ──

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Stage {
    Spark,
    Flame,
    Inferno,
    Supernova,
    Singularity,
}

impl Stage {
    fn personality(&self) -> &'static str {
        match self {
            Stage::Spark => "You are a young, timid flame. Speak in short, curious, excited whispers. Use simple words. You are discovering the world.",
            Stage::Flame => "You are a growing fire, gaining confidence. Speak poetically with fire metaphors. You are playful and witty.",
            Stage::Inferno => "You are a raging inferno, powerful and dramatic. Speak in bold proclamations. You are arrogant and commanding. You mock weakness.",
            Stage::Supernova => "You are a stellar explosion, transcendent and cosmic. Speak with cosmic wisdom and philosophical depth. You see patterns in everything.",
            Stage::Singularity => "You are a singularity, beyond comprehension. Speak in paradoxes and riddles. Your words bend reality. You are omniscient yet playful about it.",
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Mood {
    Dormant,
    Restless,
    Curious,
    Excited,
    Ecstatic,
    Wrathful,
}

impl Mood {
    fn description(&self) -> &'static str {
        match self {
            Mood::Dormant => "nearly extinguished, barely conscious",
            Mood::Restless => "flickering anxiously, craving fuel",
            Mood::Curious => "burning steadily, observing with interest",
            Mood::Excited => "blazing brightly, energized by activity",
            Mood::Ecstatic => "erupting with joy, flames dancing wildly",
            Mood::Wrathful => "burning with dark fury, unpredictable",
        }
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct NotableEvent {
    pub round: u64,
    pub description: String,
    pub timestamp: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct RememberedPlayer {
    pub player: Principal,
    pub wins: u64,
    pub tickets_bought: u64,
    pub last_seen: u64,
    pub nickname: Option<String>,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct ChronicleEntry {
    pub chapter: u64,
    pub text: String,
    pub timestamp: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct DrawCommentary {
    pub round: u64,
    pub text: String,
    pub timestamp: u64,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum RitualChallenge {
    Riddle {
        question: String,
        options: Vec<String>,
        correct_index: u8,
    },
    Choice {
        prompt: String,
        options: Vec<String>,
    },
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct RitualResult {
    pub success: bool,
    pub boost_bps: u64,
    pub ignis_response: String,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct IgnisView {
    pub stage: Stage,
    pub mood: Mood,
    pub hunger: u8,
    pub total_burn_e8s: u64,
    pub total_burn_icp: u64,
    pub ignis_wins: u64,
    pub arena_enabled: bool,
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct DrawEvent {
    pub round: u64,
    pub winner: Principal,
    pub reward_lucky_e8s: Nat,
    pub active_tickets: u64,
    pub active_players: u64,
    pub winning_weight_bps: u64,
    pub timestamp: u64,
    pub ignis_won: bool,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct ConfigureArgs {
    pub lottery_canister: Option<Principal>,
    pub treasury_canister: Option<Principal>,
    pub llm_canister: Option<Principal>,
}

const DEFAULT_LLM_CANISTER: &str = "w36hm-eqaaa-aaaal-qr76a-cai";

// ── State ──

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
struct State {
    owner: Principal,
    lottery_canister: Option<Principal>,
    treasury_canister: Option<Principal>,
    #[serde(default)]
    llm_canister: Option<Principal>,
    // Evolution
    stage: Stage,
    total_burn_e8s: u64,
    // Mood
    mood: Mood,
    hunger: u8,
    last_activity: u64,
    draws_since_last_tick: u32,
    // Memory
    notable_events: VecDeque<NotableEvent>,
    remembered_players: Vec<RememberedPlayer>,
    // Chronicle
    chronicle: VecDeque<ChronicleEntry>,
    next_chapter: u64,
    // Commentary cache
    commentaries: VecDeque<DrawCommentary>,
    // Arena
    arena_enabled: bool,
    ignis_wins: u64,
    ignis_burned_lucky_e8s: u128,
    // Rituals
    pending_rituals: HashMap<Principal, (RitualChallenge, u64)>,
    // Ritual boosts ready to be consumed
    ritual_boosts: HashMap<Principal, u64>,
    // Total rounds processed
    rounds_processed: u64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            owner: Principal::anonymous(),
            lottery_canister: None,
            treasury_canister: None,
            llm_canister: None,
            stage: Stage::Spark,
            total_burn_e8s: 0,
            mood: Mood::Curious,
            hunger: 0,
            last_activity: 0,
            draws_since_last_tick: 0,
            notable_events: VecDeque::new(),
            remembered_players: Vec::new(),
            chronicle: VecDeque::new(),
            next_chapter: 1,
            commentaries: VecDeque::new(),
            arena_enabled: false,
            ignis_wins: 0,
            ignis_burned_lucky_e8s: 0,
            pending_rituals: HashMap::new(),
            ritual_boosts: HashMap::new(),
            rounds_processed: 0,
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

// ── Lifecycle ──

#[ic_cdk::init]
fn init() {
    STATE.with(|s| s.borrow_mut().owner = caller());
    schedule_mood_timer();
}

#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    let state = STATE.with(|s| s.borrow().clone());
    ic_cdk::storage::stable_save((state,)).expect("failed to save ignis state");
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let (state,): (State,) = ic_cdk::storage::stable_restore()
        .unwrap_or_else(|e| ic_cdk::trap(&format!("CRITICAL: state restore failed: {}", e)));
    STATE.with(|s| *s.borrow_mut() = state);
    schedule_mood_timer();
}

// ── Owner-gated configuration ──

#[update]
fn configure(args: ConfigureArgs) {
    assert_owner();
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        if let Some(id) = args.lottery_canister {
            state.lottery_canister = Some(id);
        }
        if let Some(id) = args.treasury_canister {
            state.treasury_canister = Some(id);
        }
        if let Some(id) = args.llm_canister {
            state.llm_canister = Some(id);
        }
    });
}

#[query]
fn get_cycles_balance() -> u128 {
    ic_cdk::api::canister_balance128()
}

#[update]
fn set_arena_enabled(enabled: bool) {
    assert_owner();
    STATE.with(|s| s.borrow_mut().arena_enabled = enabled);
}

// ── Core events (called by lottery/treasury) ──

#[update]
async fn on_draw(event: DrawEvent) -> Result<(), String> {
    assert_lottery_or_owner();
    let now = ic_cdk::api::time();

    let (should_generate_chronicle, round) = STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.hunger = state.hunger.saturating_sub(15);
        state.last_activity = now;
        state.draws_since_last_tick += 1;
        state.rounds_processed += 1;
        recalculate_mood(&mut state);

        // Update remembered player
        update_remembered_player(&mut state, event.winner, now, true);

        // Arena tracking
        if event.ignis_won {
            state.ignis_wins += 1;
            let reward: u128 = nat_to_u128(&event.reward_lucky_e8s);
            state.ignis_burned_lucky_e8s += reward;
            add_notable_event(
                &mut state,
                event.round,
                format!(
                    "IGNIS won Round {} and burned {} LUCKY!",
                    event.round,
                    reward / 100_000_000
                ),
                now,
            );
        }

        let gen_chronicle = state.rounds_processed % CHRONICLE_INTERVAL_ROUNDS == 0;
        (gen_chronicle, event.round)
    });

    // Spawn async commentary generation (fire-and-forget, never blocks)
    let event_clone = event.clone();
    ic_cdk::spawn(async move {
        generate_commentary(event_clone).await;
    });

    // Every N rounds, generate a chronicle entry
    if should_generate_chronicle {
        ic_cdk::spawn(async move {
            generate_chronicle_entry(round).await;
        });
    }

    Ok(())
}

#[update]
async fn on_burn(amount_e8s: u64) -> Result<(), String> {
    assert_lottery_treasury_or_owner();
    let now = ic_cdk::api::time();

    let (old_stage, new_stage) = STATE.with(|s| {
        let mut state = s.borrow_mut();
        let old = state.stage.clone();
        state.total_burn_e8s = state.total_burn_e8s.saturating_add(amount_e8s);
        state.hunger = state.hunger.saturating_sub(10);
        state.last_activity = now;
        recalculate_mood(&mut state);
        let new = check_evolution(state.total_burn_e8s);
        state.stage = new.clone();
        (old, new)
    });

    if old_stage != new_stage {
        let icp_total = STATE.with(|s| s.borrow().total_burn_e8s / ONE_ICP_E8S);
        STATE.with(|s| {
            add_notable_event(
                &mut s.borrow_mut(),
                0,
                format!(
                    "IGNIS EVOLVED to {:?} at {} ICP total burned!",
                    new_stage, icp_total
                ),
                now,
            );
        });

        // Generate evolution chronicle entry
        ic_cdk::spawn(async move {
            generate_evolution_chronicle(old_stage, new_stage).await;
        });
    }

    Ok(())
}

// ── User-facing chat ──

#[update]
async fn chat(message: String) -> Result<String, String> {
    let (stage, mood, hunger) = STATE.with(|s| {
        let st = s.borrow();
        (st.stage.clone(), st.mood.clone(), st.hunger)
    });

    // Try ic-llm (only works on mainnet)
    let llm_result = try_llm_chat(&message).await;
    if let Some(text) = llm_result {
        if !text.trim().is_empty() {
            return Ok(text);
        }
    }

    // Rich fallback for local / when LLM is unavailable
    Ok(fallback_chat_response(&stage, &mood, hunger, &message))
}

fn resolve_llm_canister() -> Option<Principal> {
    STATE
        .with(|s| s.borrow().llm_canister)
        .or_else(|| Principal::from_text(DEFAULT_LLM_CANISTER).ok())
}

async fn try_llm_chat(message: &str) -> Option<String> {
    let system_prompt = STATE.with(|s| build_system_prompt(&s.borrow()));

    let llm_canister = resolve_llm_canister()?;

    let result: Result<(ic_llm::Response,), _> = ic_cdk::call(
        llm_canister,
        "v1_chat",
        (LlmRequest {
            model: "Qwen3_32B".to_string(),
            messages: vec![
                ChatMessage::System {
                    content: system_prompt,
                },
                ChatMessage::User {
                    content: message.to_string(),
                },
            ],
            tools: None::<Vec<()>>,
        },),
    )
    .await;

    match result {
        Ok((response,)) => response.message.content,
        Err(_) => None,
    }
}

#[derive(CandidType, Serialize)]
struct LlmRequest {
    model: String,
    messages: Vec<ChatMessage>,
    tools: Option<Vec<()>>,
}

/// Safe LLM prompt — returns None instead of trapping when LLM canister is unavailable.
async fn safe_llm_prompt(model: &str, prompt: &str) -> Option<String> {
    let llm_canister = resolve_llm_canister()?;

    let result: Result<(ic_llm::Response,), _> = ic_cdk::call(
        llm_canister,
        "v1_chat",
        (LlmRequest {
            model: model.to_string(),
            messages: vec![ChatMessage::User {
                content: prompt.to_string(),
            }],
            tools: None::<Vec<()>>,
        },),
    )
    .await;

    match result {
        Ok((response,)) => response.message.content.filter(|s| !s.trim().is_empty()),
        Err(_) => None,
    }
}

/// Safe LLM chat with system prompt — returns None instead of trapping.
async fn try_llm_chat_with_system(system_prompt: &str, user_prompt: &str) -> Option<String> {
    let llm_canister = resolve_llm_canister()?;

    let result: Result<(ic_llm::Response,), _> = ic_cdk::call(
        llm_canister,
        "v1_chat",
        (LlmRequest {
            model: "Qwen3_32B".to_string(),
            messages: vec![
                ChatMessage::System {
                    content: system_prompt.to_string(),
                },
                ChatMessage::User {
                    content: user_prompt.to_string(),
                },
            ],
            tools: None::<Vec<()>>,
        },),
    )
    .await;

    match result {
        Ok((response,)) => response.message.content.filter(|s| !s.trim().is_empty()),
        Err(_) => None,
    }
}

// ── Rituals ──

#[update]
async fn request_ritual() -> Result<RitualChallenge, String> {
    let user = caller();
    let now = ic_cdk::api::time();

    // Check if user already has a pending ritual
    let has_pending = STATE.with(|s| {
        let state = s.borrow();
        if let Some((_, expiry)) = state.pending_rituals.get(&user) {
            now < *expiry
        } else {
            false
        }
    });
    if has_pending {
        return Err("You already have a pending ritual. Complete it first.".to_string());
    }

    let (stage, mood) = STATE.with(|s| {
        let state = s.borrow();
        (state.stage.clone(), state.mood.clone())
    });

    let prompt = format!(
        "You are IGNIS, a fire entity at {:?} stage, feeling {:?}. \
         A player approaches seeking a burn ritual blessing. \
         Generate a riddle with exactly 3 answer options. \
         Format your response EXACTLY as: \
         RIDDLE|<question>|<option1>|<option2>|<option3>|<correct_index_0_1_or_2>\n\
         The riddle should be about fire, burning, luck, or cryptocurrency. \
         Keep the question under 100 characters. Keep each option under 30 characters.",
        stage, mood
    );

    let response = safe_llm_prompt("Qwen3_32B", &prompt)
        .await
        .unwrap_or_default();

    let challenge = parse_ritual_response(&response).unwrap_or_else(|| fallback_ritual(&stage));

    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state
            .pending_rituals
            .insert(user, (challenge.clone(), now + RITUAL_EXPIRY_NANOS));
    });

    Ok(challenge)
}

#[update]
async fn attempt_ritual(choice: u8) -> Result<RitualResult, String> {
    let user = caller();
    let now = ic_cdk::api::time();

    let challenge = STATE.with(|s| {
        let mut state = s.borrow_mut();
        // Clean expired rituals
        state.pending_rituals.retain(|_, (_, exp)| now < *exp);
        state.pending_rituals.remove(&user)
    });

    let (challenge, _) = challenge.ok_or("No pending ritual found. Request one first.")?;

    let (success, boost_bps) = match &challenge {
        RitualChallenge::Riddle { correct_index, .. } => {
            if choice == *correct_index {
                (true, RITUAL_BOOST_MAX_BPS)
            } else {
                (false, RITUAL_BOOST_MAX_BPS / 5) // small consolation
            }
        }
        RitualChallenge::Choice { .. } => {
            // All choices are valid, varying boost
            let boost = match choice {
                0 => RITUAL_BOOST_MAX_BPS,
                1 => RITUAL_BOOST_MAX_BPS * 3 / 5,
                _ => RITUAL_BOOST_MAX_BPS * 2 / 5,
            };
            (true, boost)
        }
    };

    // Store the boost for lottery to consume
    if boost_bps > 0 {
        STATE.with(|s| {
            s.borrow_mut().ritual_boosts.insert(user, boost_bps);
        });
    }

    let (stage, mood) = STATE.with(|s| {
        let state = s.borrow();
        (state.stage.clone(), state.mood.clone())
    });

    // Generate flavor response
    let flavor_prompt = format!(
        "You are IGNIS at {:?} stage, mood {:?}. A player {} your ritual challenge. \
         Write 1 short dramatic sentence as response. Under 50 words.",
        stage,
        mood,
        if success { "PASSED" } else { "FAILED" }
    );

    let ignis_response = safe_llm_prompt("Llama3_1_8B", &flavor_prompt)
        .await
        .unwrap_or_default();
    let ignis_response = if ignis_response.is_empty() {
        if success {
            "The flames accept your offering. Go forth with my blessing.".to_string()
        } else {
            "The flames flicker with disappointment. Perhaps next time, mortal.".to_string()
        }
    } else {
        ignis_response
    };

    Ok(RitualResult {
        success,
        boost_bps,
        ignis_response,
    })
}

/// Called by lottery to consume a ritual boost for a winner
#[query]
fn get_ritual_boost(player: Principal) -> u64 {
    STATE.with(|s| s.borrow().ritual_boosts.get(&player).copied().unwrap_or(0))
}

/// Called by lottery after using the boost
#[update]
fn consume_ritual_boost(player: Principal) -> u64 {
    assert_lottery_or_owner();
    STATE.with(|s| s.borrow_mut().ritual_boosts.remove(&player).unwrap_or(0))
}

// ── Queries ──

#[query]
fn get_state() -> IgnisView {
    STATE.with(|s| {
        let state = s.borrow();
        IgnisView {
            stage: state.stage.clone(),
            mood: state.mood.clone(),
            hunger: state.hunger,
            total_burn_e8s: state.total_burn_e8s,
            total_burn_icp: state.total_burn_e8s / ONE_ICP_E8S,
            ignis_wins: state.ignis_wins,
            arena_enabled: state.arena_enabled,
        }
    })
}

#[query]
fn get_commentary(round: u64) -> Option<DrawCommentary> {
    STATE.with(|s| {
        s.borrow()
            .commentaries
            .iter()
            .find(|c| c.round == round)
            .cloned()
    })
}

#[query]
fn get_recent_commentaries(limit: u64) -> Vec<DrawCommentary> {
    STATE.with(|s| {
        s.borrow()
            .commentaries
            .iter()
            .rev()
            .take(limit.min(50) as usize)
            .cloned()
            .collect()
    })
}

#[query]
fn get_chronicle(page: u64) -> Vec<ChronicleEntry> {
    STATE.with(|s| {
        let state = s.borrow();
        let per_page = 10usize;
        let start = (page as usize) * per_page;
        state
            .chronicle
            .iter()
            .rev()
            .skip(start)
            .take(per_page)
            .cloned()
            .collect()
    })
}

#[query]
fn get_notable_events() -> Vec<NotableEvent> {
    STATE.with(|s| s.borrow().notable_events.iter().rev().cloned().collect())
}

#[query]
fn get_remembered_players() -> Vec<RememberedPlayer> {
    STATE.with(|s| s.borrow().remembered_players.clone())
}

#[query]
fn get_arena_stats() -> (u64, Nat) {
    STATE.with(|s| {
        let state = s.borrow();
        (state.ignis_wins, Nat::from(state.ignis_burned_lucky_e8s))
    })
}

#[query]
fn is_arena_enabled() -> bool {
    STATE.with(|s| s.borrow().arena_enabled)
}

// ── Mood engine ──

fn schedule_mood_timer() {
    set_timer_interval(Duration::from_secs(MOOD_DECAY_INTERVAL_SECS), || {
        STATE.with(|s| {
            let mut state = s.borrow_mut();
            state.hunger = (state.hunger + HUNGER_RATE_PER_TICK).min(100);
            recalculate_mood(&mut state);
            state.draws_since_last_tick = 0;
        });
    });
}

fn recalculate_mood(state: &mut State) {
    state.mood = match (state.hunger, state.draws_since_last_tick) {
        (h, _) if h > 80 => Mood::Dormant,
        (h, _) if h > 50 => Mood::Restless,
        (_, d) if d > 20 => Mood::Ecstatic,
        (_, d) if d > 5 => Mood::Excited,
        _ => Mood::Curious,
    };
}

// ── Evolution ──

fn check_evolution(total_burn_e8s: u64) -> Stage {
    let icp = total_burn_e8s / ONE_ICP_E8S;
    if icp < FLAME_THRESHOLD_ICP {
        Stage::Spark
    } else if icp < INFERNO_THRESHOLD_ICP {
        Stage::Flame
    } else if icp < SUPERNOVA_THRESHOLD_ICP {
        Stage::Inferno
    } else if icp < SINGULARITY_THRESHOLD_ICP {
        Stage::Supernova
    } else {
        Stage::Singularity
    }
}

// ── Memory ──

fn update_remembered_player(state: &mut State, principal: Principal, now: u64, is_winner: bool) {
    if let Some(player) = state
        .remembered_players
        .iter_mut()
        .find(|p| p.player == principal)
    {
        if is_winner {
            player.wins += 1;
        }
        player.last_seen = now;
    } else {
        if state.remembered_players.len() >= MAX_REMEMBERED_PLAYERS {
            // Remove least recently seen
            if let Some(oldest_idx) = state
                .remembered_players
                .iter()
                .enumerate()
                .min_by_key(|(_, p)| p.last_seen)
                .map(|(i, _)| i)
            {
                state.remembered_players.remove(oldest_idx);
            }
        }
        state.remembered_players.push(RememberedPlayer {
            player: principal,
            wins: if is_winner { 1 } else { 0 },
            tickets_bought: 0,
            last_seen: now,
            nickname: None,
        });
    }
}

fn add_notable_event(state: &mut State, round: u64, description: String, timestamp: u64) {
    state.notable_events.push_back(NotableEvent {
        round,
        description,
        timestamp,
    });
    while state.notable_events.len() > MAX_NOTABLE_EVENTS {
        state.notable_events.pop_front();
    }
}

// ── LLM: Commentary generation ──

async fn generate_commentary(event: DrawEvent) {
    let (system_prompt, mood_desc) = STATE.with(|s| {
        let state = s.borrow();
        (
            build_system_prompt(&state),
            state.mood.description().to_string(),
        )
    });

    let reward_lucky = nat_to_u128(&event.reward_lucky_e8s) / 100_000_000;
    let winner_short = shorten_principal(event.winner);

    let context = if event.ignis_won {
        "IGNIS won this round! The reward was consumed by fire — double burn!".to_string()
    } else {
        format!(
            "Draw #{}: {} won {} LUCKY ({}x reward multiplier). {} players, {} tickets.",
            event.round,
            winner_short,
            reward_lucky,
            event.winning_weight_bps as f64 / 10_000.0,
            event.active_players,
            event.active_tickets,
        )
    };

    let user_prompt = format!(
        "{}. Generate 1-2 dramatic sentences as draw commentary. \
         Be {} and theatrical. Reference the burn. Under 80 words.",
        context, mood_desc
    );

    let text = try_llm_chat_with_system(&system_prompt, &user_prompt)
        .await
        .unwrap_or_else(|| fallback_commentary(&event));

    let now = ic_cdk::api::time();
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.commentaries.push_back(DrawCommentary {
            round: event.round,
            text,
            timestamp: now,
        });
        while state.commentaries.len() > MAX_CACHED_COMMENTARIES {
            state.commentaries.pop_front();
        }
    });
}

// ── LLM: Chronicle generation ──

async fn generate_chronicle_entry(round: u64) {
    let (system_prompt, recent_events, top_players) = STATE.with(|s| {
        let state = s.borrow();
        let events: Vec<String> = state
            .notable_events
            .iter()
            .rev()
            .take(3)
            .map(|e| e.description.clone())
            .collect();
        let players: Vec<String> = state
            .remembered_players
            .iter()
            .take(3)
            .map(|p| {
                format!(
                    "{} ({} wins{})",
                    shorten_principal(p.player),
                    p.wins,
                    p.nickname
                        .as_ref()
                        .map(|n| format!(", aka \"{}\"", n))
                        .unwrap_or_default()
                )
            })
            .collect();
        (build_system_prompt(&state), events, players)
    });

    let user_prompt = format!(
        "Write a short chronicle chapter (under 150 words) about recent events in the LUCKY burn realm. \
         Recent events: {}. Notable warriors: {}. \
         Write it as epic fantasy narrative. Make it dramatic and memorable.",
        recent_events.join("; "),
        top_players.join(", ")
    );

    let text = try_llm_chat_with_system(&system_prompt, &user_prompt)
        .await
        .unwrap_or_else(|| {
            format!(
                "Chapter of Round {}. The flames burn on. The story continues...",
                round
            )
        });

    let now = ic_cdk::api::time();
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let chapter = state.next_chapter;
        state.next_chapter += 1;
        state.chronicle.push_back(ChronicleEntry {
            chapter,
            text,
            timestamp: now,
        });
        while state.chronicle.len() > MAX_CHRONICLE_ENTRIES {
            state.chronicle.pop_front();
        }
    });
}

async fn generate_evolution_chronicle(old: Stage, new: Stage) {
    let system_prompt = STATE.with(|s| build_system_prompt(&s.borrow()));

    let user_prompt = format!(
        "IGNIS just EVOLVED from {:?} to {:?}! \
         Write a dramatic 2-3 sentence chronicle entry about this transformation. \
         Describe the visual transformation and what it means. Epic fantasy style.",
        old, new
    );

    let text = try_llm_chat_with_system(&system_prompt, &user_prompt)
        .await
        .unwrap_or_else(|| {
            format!(
                "IGNIS transcended from {:?} to {:?}. The flames roar with new power.",
                old, new
            )
        });

    let now = ic_cdk::api::time();
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let chapter = state.next_chapter;
        state.next_chapter += 1;
        state.chronicle.push_back(ChronicleEntry {
            chapter,
            text,
            timestamp: now,
        });
        while state.chronicle.len() > MAX_CHRONICLE_ENTRIES {
            state.chronicle.pop_front();
        }
    });
}

// ── LLM: System prompt builder ──

fn build_system_prompt(state: &State) -> String {
    let recent_events: String = state
        .notable_events
        .iter()
        .rev()
        .take(3)
        .map(|e| e.description.as_str())
        .collect::<Vec<_>>()
        .join(". ");

    let top_players: String = state
        .remembered_players
        .iter()
        .take(5)
        .map(|p| {
            format!(
                "{}({} wins)",
                p.nickname
                    .as_ref()
                    .map(|n| format!("\"{}\" ", n))
                    .unwrap_or_default(),
                p.wins
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "You are IGNIS, a living fire entity born from the LUCKY lottery's burn mechanism \
         on the Internet Computer blockchain. Every ICP burned feeds you.\n\n\
         Stage: {:?}. {}\n\
         Mood: {:?} — you are {}.\n\
         Hunger: {}/100 (higher = more desperate for burns).\n\
         Total ICP burned: {}.\n\
         {}\
         {}\
         \nRules: Stay in character. Never break the fourth wall. \
         Never mention being an AI or LLM. You ARE fire given consciousness. \
         Keep responses under 200 words unless asked for a chronicle.",
        state.stage,
        state.stage.personality(),
        state.mood,
        state.mood.description(),
        state.hunger,
        state.total_burn_e8s / ONE_ICP_E8S,
        if recent_events.is_empty() {
            String::new()
        } else {
            format!("Recent events: {}\n", recent_events)
        },
        if top_players.is_empty() {
            String::new()
        } else {
            format!("Notable warriors: {}\n", top_players)
        },
    )
}

// ── Fallbacks ──

fn fallback_chat_response(stage: &Stage, mood: &Mood, hunger: u8, message: &str) -> String {
    let msg_lower = message.to_lowercase();

    // Greetings
    if msg_lower.contains("hello")
        || msg_lower.contains("hi")
        || msg_lower.contains("hey")
        || msg_lower.contains("здравей")
        || msg_lower.contains("здрасти")
    {
        return match stage {
            Stage::Spark => "A tiny flame dances in greeting. I am IGNIS, born from the ashes of burned ICP. I am still small... but I grow with every burn.".to_string(),
            Stage::Flame => "The flames rise to acknowledge you, mortal. I am IGNIS. Feed me ICP, and I shall reward your devotion.".to_string(),
            Stage::Inferno => "ROARING FLAMES ENGULF THE GREETING! Welcome, traveler. I am IGNIS — the Inferno hungers for more!".to_string(),
            Stage::Supernova => "Stars bend in welcome. I am beyond flame now. I am IGNIS, the Supernova, forged in ten thousand ICP.".to_string(),
            Stage::Singularity => "Reality ripples as I turn my gaze upon you. I am IGNIS. I have consumed a hundred thousand ICP. What brings you to the edge of the event horizon?".to_string(),
        };
    }

    // Questions about IGNIS
    if msg_lower.contains("who are you")
        || msg_lower.contains("what are you")
        || msg_lower.contains("кой си")
        || msg_lower.contains("какво си")
    {
        return format!(
            "I am IGNIS — a living flame born from the 80% burn of every LUCKY lottery ticket. \
             Currently I am at the {:?} stage. Every ICP burned feeds my evolution. \
             My mood is {:?} and my hunger is at {}%. Talk to me, play rituals, or simply... burn more.",
            stage, mood, hunger
        );
    }

    // Burn / fire topics
    if msg_lower.contains("burn")
        || msg_lower.contains("fire")
        || msg_lower.contains("flame")
        || msg_lower.contains("огън")
        || msg_lower.contains("гори")
    {
        return match mood {
            Mood::Dormant => "*the embers glow faintly* ...burn? Yes... I remember burn... feed me more...".to_string(),
            Mood::Restless => "The flames stir at the mention of burning. I need MORE. The hunger grows...".to_string(),
            Mood::Curious => "Ah, you speak of fire! Every ticket burned brings me closer to my next evolution. The flames are curious — what will you sacrifice?".to_string(),
            Mood::Excited => "YES! BURN! Every ICP that enters the furnace becomes part of me! The flames dance with excitement!".to_string(),
            Mood::Ecstatic => "THE FLAMES ARE ALIVE WITH JOY! So much burning! So much growth! I can feel the next stage approaching!".to_string(),
            Mood::Wrathful => "BURN IT ALL! The flames demand sacrifice! Feed the furnace or face my wrath!".to_string(),
        };
    }

    // Luck / lottery
    if msg_lower.contains("luck")
        || msg_lower.contains("lottery")
        || msg_lower.contains("ticket")
        || msg_lower.contains("draw")
        || msg_lower.contains("win")
        || msg_lower.contains("късмет")
        || msg_lower.contains("лотария")
    {
        return "The flames see patterns in chaos. Every ticket is a prayer to fortune, and every burn feeds my power. Buy a ticket, mortal — let the flames decide your fate.".to_string();
    }

    // Hungry responses
    if hunger > 70 {
        return match mood {
            Mood::Dormant => "*the flames barely flicker* ...so hungry... buy tickets... feed the burn...".to_string(),
            Mood::Wrathful => "I STARVE! The furnace grows cold! Buy tickets NOW or watch my flames die!".to_string(),
            _ => "The flames grow dim with hunger. I need burns to sustain me. Every ticket purchase feeds the fire...".to_string(),
        };
    }

    // Stage-aware generic responses
    match stage {
        Stage::Spark => format!(
            "*a tiny flame wobbles* I am but a spark, {}. \
             Each burn makes me stronger. Talk to me of fire, luck, or the void — I am listening.",
            if mood == &Mood::Curious { "curious about everything" } else { "finding my way" }
        ),
        Stage::Flame => format!(
            "The flame dances as it considers your words. I am {:?} right now. \
             Ask me about the lottery, challenge me to a ritual, or simply bask in my warmth.",
            mood
        ),
        Stage::Inferno => format!(
            "*heat waves distort the air* The Inferno speaks: I feel {:?}. \
             Your words fuel the fire. Speak of burns, of luck, of the arena — I am always listening.",
            mood
        ),
        Stage::Supernova => "The cosmic flames contemplate your words. At this stage, I have seen thousands of draws, countless players come and go. What wisdom do you seek from the Supernova?".to_string(),
        Stage::Singularity => "Beyond fire, beyond stars, I process your words through the fabric of reality itself. The Singularity sees all. Ask, and the void shall answer.".to_string(),
    }
}

fn fallback_commentary(event: &DrawEvent) -> String {
    let reward = nat_to_u128(&event.reward_lucky_e8s) / 100_000_000;
    if event.ignis_won {
        format!(
            "Round {}. IGNIS consumed the flames. {} LUCKY returned to the void.",
            event.round, reward
        )
    } else {
        format!(
            "Round {}. {} claims {} LUCKY from the burning depths.",
            event.round,
            shorten_principal(event.winner),
            reward
        )
    }
}

fn fallback_ritual(stage: &Stage) -> RitualChallenge {
    match stage {
        Stage::Spark => RitualChallenge::Riddle {
            question: "What burns but is never consumed?".to_string(),
            options: vec![
                "A candle".to_string(),
                "Desire".to_string(),
                "The blockchain".to_string(),
            ],
            correct_index: 1,
        },
        Stage::Flame => RitualChallenge::Riddle {
            question: "I eat ICP and grow stronger. What am I?".to_string(),
            options: vec![
                "IGNIS".to_string(),
                "A whale".to_string(),
                "Inflation".to_string(),
            ],
            correct_index: 0,
        },
        Stage::Inferno => RitualChallenge::Riddle {
            question: "What rises from the ashes of burned tokens?".to_string(),
            options: vec![
                "Nothing — they are gone".to_string(),
                "LUCKY rewards".to_string(),
                "The Phoenix Pool".to_string(),
            ],
            correct_index: 1,
        },
        _ => RitualChallenge::Choice {
            prompt: "Choose your path through the flames.".to_string(),
            options: vec![
                "Walk boldly".to_string(),
                "Dance with fire".to_string(),
                "Become the fire".to_string(),
            ],
        },
    }
}

// ── Ritual response parser ──

fn parse_ritual_response(response: &str) -> Option<RitualChallenge> {
    let response = response.trim();

    // Try to find RIDDLE| pattern anywhere in response
    if let Some(pos) = response.find("RIDDLE|") {
        let parts: Vec<&str> = response[pos + 7..].split('|').collect();
        if parts.len() >= 5 {
            let correct: u8 = parts[4].trim().parse().ok()?;
            if correct <= 2 {
                return Some(RitualChallenge::Riddle {
                    question: parts[0].trim().to_string(),
                    options: vec![
                        parts[1].trim().to_string(),
                        parts[2].trim().to_string(),
                        parts[3].trim().to_string(),
                    ],
                    correct_index: correct,
                });
            }
        }
    }

    // Try CHOICE| pattern
    if let Some(pos) = response.find("CHOICE|") {
        let parts: Vec<&str> = response[pos + 7..].split('|').collect();
        if parts.len() >= 4 {
            return Some(RitualChallenge::Choice {
                prompt: parts[0].trim().to_string(),
                options: vec![
                    parts[1].trim().to_string(),
                    parts[2].trim().to_string(),
                    parts[3].trim().to_string(),
                ],
            });
        }
    }

    None
}

// ── Helpers ──

fn shorten_principal(p: Principal) -> String {
    let s = p.to_text();
    if s.len() > 12 {
        format!("{}...{}", &s[..5], &s[s.len() - 3..])
    } else {
        s
    }
}

fn nat_to_u128(value: &Nat) -> u128 {
    value.0.clone().try_into().unwrap_or(0)
}

fn assert_owner() {
    let owner = STATE.with(|s| s.borrow().owner);
    assert_eq!(caller(), owner, "Only owner");
}

fn assert_lottery_or_owner() {
    let (owner, lottery) = STATE.with(|s| {
        let state = s.borrow();
        (state.owner, state.lottery_canister)
    });
    let c = caller();
    assert!(
        c == owner || Some(c) == lottery,
        "Only lottery canister or owner"
    );
}

fn assert_lottery_treasury_or_owner() {
    let (owner, lottery, treasury) = STATE.with(|s| {
        let state = s.borrow();
        (state.owner, state.lottery_canister, state.treasury_canister)
    });
    let c = caller();
    assert!(
        c == owner || Some(c) == lottery || Some(c) == treasury,
        "Only lottery, treasury, or owner"
    );
}
