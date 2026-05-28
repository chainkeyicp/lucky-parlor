import { Actor, HttpAgent } from "https://esm.sh/@dfinity/agent@2.1.3";
import { Ed25519KeyIdentity } from "https://esm.sh/@dfinity/identity@2.1.3/lib/esm/identity/ed25519.js";
import { CANISTER_IDS } from "./canister-ids.js";

const E8S = 100_000_000n;
const ICP_LEDGER_FEE = 10_000n;
const TICKET_SPLIT_FEE_COUNT = isLocal ? 3n : 2n;
const DRAWS_PER_ICP = 144;
const BASE_REWARD_LUCKY = 50;
const PER_PLAYER_REWARD_LUCKY = 10;
const MAX_REWARD_LUCKY = 500;
const IDENTITY_STORAGE_KEY = "lucky.localIdentity.v1";
const pageHost = window.location.hostname;
const isLocal = pageHost === "localhost" || pageHost === "127.0.0.1" || pageHost.endsWith(".localhost");
const network = isLocal ? "local" : "ic";
const localPort = window.location.port || "8080";
const host = isLocal ? `${window.location.protocol}//127.0.0.1:${localPort}` : "https://ic0.app";
const ids = CANISTER_IDS[network];

const els = {
  // Top bar
  loginBtn: document.getElementById("loginBtn"),
  newIdentityBtn: document.getElementById("newIdentityBtn"),
  principalText: document.getElementById("principalText"),
  // Parlor — live strip
  liveTimer: document.getElementById("liveTimer"),
  liveRound: document.getElementById("liveRound"),
  liveStats: document.getElementById("liveStats"),
  liveTicker: document.getElementById("liveTicker"),
  // Parlor — buy card
  buyRound: document.getElementById("buyRound"),
  buyStatus: document.getElementById("buyStatus"),
  rewardText: document.getElementById("rewardText"),
  ticketSlider: document.getElementById("ticketSlider"),
  sliderIcp: document.getElementById("sliderIcp"),
  sliderDraws: document.getElementById("sliderDraws"),
  buyBtn: document.getElementById("buyBtn"),
  notice: document.getElementById("notice"),
  faucetBtn: document.getElementById("faucetBtn"),
  // Parlor — timer ring
  timerSeconds: document.getElementById("timerSeconds"),
  timerSuffix: document.getElementById("timerSuffix"),
  timerArc: document.getElementById("timerArc"),
  timerSpinner: document.getElementById("timerSpinner"),
  // Parlor — status card
  statusPrincipal: document.getElementById("statusPrincipal"),
  statusTickets: document.getElementById("statusTickets"),
  ticketScroller: document.getElementById("ticketScroller"),
  statusStreak: document.getElementById("statusStreak"),
  statusWallet: document.getElementById("statusWallet"),
  // Parlor — feed
  winnersList: document.getElementById("winnersList"),
  drawBtn: document.getElementById("drawBtn"),
  // Parlor — burn
  burnTotalNum: document.getElementById("burnTotalNum"),
  burnPoolNum: document.getElementById("burnPoolNum"),
  nextBurnText: document.getElementById("nextBurnText"),
  // Parlor — ignis mini
  parlorIgnisStage: document.getElementById("parlorIgnisStage"),
  parlorIgnisBubble: document.getElementById("parlorIgnisBubble"),
  parlorIgnisStats: document.getElementById("parlorIgnisStats"),
  parlorIgnisPortrait: document.getElementById("parlorIgnisPortrait"),
  // Parlor — ritual
  parlorRitualRequestBtn: document.getElementById("parlorRitualRequestBtn"),
  parlorRitualContent: document.getElementById("parlorRitualContent"),
  parlorRitualChallenge: document.getElementById("parlorRitualChallenge"),
  parlorRitualQuestion: document.getElementById("parlorRitualQuestion"),
  parlorRitualOptions: document.getElementById("parlorRitualOptions"),
  parlorRitualResult: document.getElementById("parlorRitualResult"),
  parlorRitualResultText: document.getElementById("parlorRitualResultText"),
  parlorRitualAgainBtn: document.getElementById("parlorRitualAgainBtn"),
  // Staking
  totalStaked: document.getElementById("totalStaked"),
  unlockableBalance: document.getElementById("unlockableBalance"),
  lockedBalance: document.getElementById("lockedBalance"),
  boostText: document.getElementById("boostText"),
  stakeLiquid: document.getElementById("stakeLiquid"),
  stakeAmount: document.getElementById("stakeAmount"),
  stakeBtn: document.getElementById("stakeBtn"),
  unstakeAllBtn: document.getElementById("unstakeAllBtn"),
  stakeBatchList: document.getElementById("stakeBatchList"),
  tierTrack: document.getElementById("tierTrack"),
  // Streak
  streakBoxes: document.getElementById("streakBoxes"),
  streakTitle: document.getElementById("streakTitle"),
  streakBoostSticker: document.getElementById("streakBoostSticker"),
  // Wallet
  walletIcp: document.getElementById("walletIcp"),
  walletLucky: document.getElementById("walletLucky"),
  walletStaked: document.getElementById("walletStaked"),
  walletTotal: document.getElementById("walletTotal"),
  walletPrincipal: document.getElementById("walletPrincipal"),
  copyPrincipalBtn: document.getElementById("copyPrincipalBtn"),
  walletAction: document.getElementById("walletAction"),
  sendTo: document.getElementById("sendTo"),
  sendAmount: document.getElementById("sendAmount"),
  sendAmountLabel: document.getElementById("sendAmountLabel"),
  sendBtn: document.getElementById("sendBtn"),
  walletNotice: document.getElementById("walletNotice"),
  activityFeed: document.getElementById("activityFeed"),
  // IGNIS page
  ignisStageBadge: document.getElementById("ignisStageBadge"),
  ignisPortrait: document.getElementById("ignisPortrait"),
  ignisMoodText: document.getElementById("ignisMoodText"),
  ignisHungerFill: document.getElementById("ignisHungerFill"),
  ignisHungerText: document.getElementById("ignisHungerText"),
  ignisBurnTotal: document.getElementById("ignisBurnTotal"),
  ignisArenaWins: document.getElementById("ignisArenaWins"),
  ignisChatMessages: document.getElementById("ignisChatMessages"),
  ignisChatInput: document.getElementById("ignisChatInput"),
  ignisChatSendBtn: document.getElementById("ignisChatSendBtn"),
  ignisRitualRequestBtn: document.getElementById("ignisRitualRequestBtn"),
  ignisRitualContent: document.getElementById("ignisRitualContent"),
  ignisRitualChallenge: document.getElementById("ignisRitualChallenge"),
  ignisRitualQuestion: document.getElementById("ignisRitualQuestion"),
  ignisRitualOptions: document.getElementById("ignisRitualOptions"),
  ignisRitualResult: document.getElementById("ignisRitualResult"),
  ignisRitualResultText: document.getElementById("ignisRitualResultText"),
  ignisRitualBoost: document.getElementById("ignisRitualBoost"),
  ignisRitualAgainBtn: document.getElementById("ignisRitualAgainBtn"),
  ignisCommentaryRefresh: document.getElementById("ignisCommentaryRefresh"),
  ignisCommentaryList: document.getElementById("ignisCommentaryList"),
  ignisChronicleList: document.getElementById("ignisChronicleList"),
  chroniclePrev: document.getElementById("chroniclePrev"),
  chronicleNext: document.getElementById("chronicleNext"),
  chroniclePage: document.getElementById("chroniclePage"),
};

// ── IDL definitions ──

const accountType = (IDL) => IDL.Record({
  owner: IDL.Principal,
  subaccount: IDL.Opt(IDL.Vec(IDL.Nat8)),
});

const ledgerTransferError = (IDL) => IDL.Variant({
  BadFee: IDL.Record({ expected_fee: IDL.Nat }),
  BadBurn: IDL.Record({ min_burn_amount: IDL.Nat }),
  InsufficientFunds: IDL.Record({ balance: IDL.Nat }),
  TooOld: IDL.Null,
  CreatedInFuture: IDL.Record({ ledger_time: IDL.Nat64 }),
  Duplicate: IDL.Record({ duplicate_of: IDL.Nat }),
  TemporarilyUnavailable: IDL.Null,
  GenericError: IDL.Record({ error_code: IDL.Nat, message: IDL.Text }),
});

const ledgerTransferArgs = (IDL, Account) => IDL.Record({
  from_subaccount: IDL.Opt(IDL.Vec(IDL.Nat8)),
  to: Account,
  amount: IDL.Nat,
  fee: IDL.Opt(IDL.Nat),
  memo: IDL.Opt(IDL.Vec(IDL.Nat8)),
  created_at_time: IDL.Opt(IDL.Nat64),
});

// Real ICP ledger IDL (ICRC-1 standard — mainnet compatible)
const realLedgerIDL = ({ IDL }) => {
  const Account = accountType(IDL);
  const TransferError = ledgerTransferError(IDL);
  return IDL.Service({
    icrc1_balance_of: IDL.Func([Account], [IDL.Nat], ["query"]),
    icrc1_fee: IDL.Func([], [IDL.Nat], ["query"]),
    icrc1_transfer: IDL.Func([ledgerTransferArgs(IDL, Account)], [IDL.Variant({ Ok: IDL.Nat, Err: TransferError })], []),
  });
};

// Mock ledger IDL (local dev — includes faucet and get_transactions)
const mockLedgerIDL = ({ IDL }) => {
  const Account = accountType(IDL);
  const TransferError = ledgerTransferError(IDL);
  return IDL.Service({
    faucet: IDL.Func([Account, IDL.Nat], [IDL.Variant({ Ok: IDL.Nat, Err: IDL.Text })], []),
    icrc1_balance_of: IDL.Func([Account], [IDL.Nat], ["query"]),
    icrc1_fee: IDL.Func([], [IDL.Nat], ["query"]),
    icrc1_transfer: IDL.Func([ledgerTransferArgs(IDL, Account)], [IDL.Variant({ Ok: IDL.Nat, Err: TransferError })], []),
    get_transactions: IDL.Func([], [IDL.Vec(IDL.Record({
      index: IDL.Nat, from: Account, to: Account, amount: IDL.Nat, fee: IDL.Nat, timestamp: IDL.Nat64,
    }))], ["query"]),
  });
};

const ledgerIDL = isLocal ? mockLedgerIDL : realLedgerIDL;

const tokenIDL = ({ IDL }) => {
  const Account = accountType(IDL);
  const StakeInfo = IDL.Record({
    liquid: IDL.Nat, staked: IDL.Nat, unlockable: IDL.Nat,
    locked: IDL.Nat, total: IDL.Nat, boost_bps: IDL.Nat64,
  });
  const StakeBatchInfo = IDL.Record({
    amount: IDL.Nat, staked_at: IDL.Nat64, unlocks_at: IDL.Nat64, unlocked: IDL.Bool,
  });
  const TransferError = IDL.Variant({
    BadFee: IDL.Record({ expected_fee: IDL.Nat }),
    InsufficientFunds: IDL.Record({ balance: IDL.Nat }),
    TooOld: IDL.Null,
    CreatedInFuture: IDL.Record({ ledger_time: IDL.Nat64 }),
    Duplicate: IDL.Record({ duplicate_of: IDL.Nat }),
    TemporarilyUnavailable: IDL.Null,
    GenericError: IDL.Record({ error_code: IDL.Nat, message: IDL.Text }),
  });
  return IDL.Service({
    icrc1_balance_of: IDL.Func([Account], [IDL.Nat], ["query"]),
    icrc1_transfer: IDL.Func([
      IDL.Record({
        from_subaccount: IDL.Opt(IDL.Vec(IDL.Nat8)),
        to: Account, amount: IDL.Nat, fee: IDL.Opt(IDL.Nat),
        memo: IDL.Opt(IDL.Vec(IDL.Nat8)),
        created_at_time: IDL.Opt(IDL.Nat64),
      })
    ], [IDL.Variant({ Ok: IDL.Nat, Err: TransferError })], []),
    staked_balance_of: IDL.Func([IDL.Principal], [IDL.Nat], ["query"]),
    get_stake_info: IDL.Func([IDL.Principal], [StakeInfo], ["query"]),
    get_stake_batches: IDL.Func([IDL.Principal], [IDL.Vec(StakeBatchInfo)], ["query"]),
    get_boost_bps: IDL.Func([IDL.Principal], [IDL.Nat64], ["query"]),
    stake: IDL.Func([IDL.Nat], [IDL.Variant({ Ok: StakeInfo, Err: IDL.Text })], []),
    unstake: IDL.Func([IDL.Nat], [IDL.Variant({ Ok: StakeInfo, Err: IDL.Text })], []),
  });
};

const treasuryIDL = ({ IDL }) => {
  const Stats = IDL.Record({
    total_received: IDL.Nat64, total_dev: IDL.Nat64, total_liquidity: IDL.Nat64,
    total_burn: IDL.Nat64, claim_count: IDL.Nat64, pending_distributions: IDL.Nat64,
  });
  return IDL.Service({ get_stats: IDL.Func([], [Stats], ["query"]) });
};

const lotteryIDL = ({ IDL }) => {
  const Account = accountType(IDL);
  const ClaimResult = IDL.Record({
    amount_e8s: IDL.Nat64, dev_e8s: IDL.Nat64, liquidity_e8s: IDL.Nat64,
    burn_e8s: IDL.Nat64, ledger_fees_e8s: IDL.Nat64, distribution_complete: IDL.Bool,
  });
  const DrawResult = IDL.Record({
    round: IDL.Nat64, winner: IDL.Principal, reward_lucky_e8s: IDL.Nat,
    active_tickets: IDL.Nat64, active_players: IDL.Nat64,
    winning_weight_bps: IDL.Nat64, timestamp: IDL.Nat64,
    ticket_principals: IDL.Vec(IDL.Principal), ticket_ids: IDL.Vec(IDL.Nat64),
    winning_ticket_id: IDL.Nat64,
  });
  const LotteryStats = IDL.Record({
    round: IDL.Nat64, active_tickets: IDL.Nat64, active_players: IDL.Nat64,
    emitted_today_lucky_e8s: IDL.Nat, daily_cap_lucky_e8s: IDL.Nat,
    burn_pool_lucky_e8s: IDL.Nat, total_burned_lucky_e8s: IDL.Nat,
    last_burn_epoch: IDL.Nat64, total_rewards_minted_lucky_e8s: IDL.Nat,
    halving_level: IDL.Nat64,
  });
  const TicketInfo = IDL.Record({
    ticket_id: IDL.Nat64, purchased_at: IDL.Nat64,
    remaining_draws: IDL.Nat64, total_draws: IDL.Nat64,
  });
  const TicketStatus = IDL.Record({
    active_tickets: IDL.Nat64, min_remaining_draws: IDL.Opt(IDL.Nat64),
    boost_bps: IDL.Nat64, streak_days: IDL.Nat64, streak_boost_bps: IDL.Nat64,
  });
  const StreakInfo = IDL.Record({
    consecutive_days: IDL.Nat64, streak_boost_bps: IDL.Nat64,
    last_purchase_day: IDL.Nat64, current_day: IDL.Nat64, streak_alive: IDL.Bool,
  });
  return IDL.Service({
    get_deposit_account: IDL.Func([IDL.Principal], [IDL.Variant({ Ok: Account, Err: IDL.Text })], ["query"]),
    buy_ticket: IDL.Func([IDL.Nat64], [IDL.Variant({ Ok: ClaimResult, Err: IDL.Text })], []),
    get_draw_history: IDL.Func([], [IDL.Vec(DrawResult)], ["query"]),
    get_stats: IDL.Func([], [LotteryStats], ["query"]),
    get_ticket_status: IDL.Func([IDL.Principal], [TicketStatus], []),
    get_my_tickets: IDL.Func([IDL.Principal], [IDL.Vec(TicketInfo)], ["query"]),
    get_streak_info: IDL.Func([IDL.Principal], [StreakInfo], ["query"]),
    get_draw_interval: IDL.Func([], [IDL.Nat64], ["query"]),
  });
};

const ignisIDL = ({ IDL }) => {
  const Stage = IDL.Variant({ Spark: IDL.Null, Flame: IDL.Null, Inferno: IDL.Null, Supernova: IDL.Null, Singularity: IDL.Null });
  const Mood = IDL.Variant({ Dormant: IDL.Null, Restless: IDL.Null, Curious: IDL.Null, Excited: IDL.Null, Ecstatic: IDL.Null, Wrathful: IDL.Null });
  const IgnisView = IDL.Record({ stage: Stage, mood: Mood, hunger: IDL.Nat8, total_burn_e8s: IDL.Nat64, total_burn_icp: IDL.Nat64, ignis_wins: IDL.Nat64, arena_enabled: IDL.Bool });
  const DrawCommentary = IDL.Record({ round: IDL.Nat64, text: IDL.Text, timestamp: IDL.Nat64 });
  const ChronicleEntry = IDL.Record({ chapter: IDL.Nat64, text: IDL.Text, timestamp: IDL.Nat64 });
  const NotableEvent = IDL.Record({ round: IDL.Nat64, description: IDL.Text, timestamp: IDL.Nat64 });
  const RitualChallenge = IDL.Variant({
    Riddle: IDL.Record({ question: IDL.Text, options: IDL.Vec(IDL.Text), correct_index: IDL.Nat8 }),
    Choice: IDL.Record({ prompt: IDL.Text, options: IDL.Vec(IDL.Text) }),
  });
  const RitualResult = IDL.Record({ success: IDL.Bool, boost_bps: IDL.Nat64, ignis_response: IDL.Text });
  return IDL.Service({
    get_state: IDL.Func([], [IgnisView], ["query"]),
    chat: IDL.Func([IDL.Text], [IDL.Variant({ Ok: IDL.Text, Err: IDL.Text })], []),
    request_ritual: IDL.Func([], [IDL.Variant({ Ok: RitualChallenge, Err: IDL.Text })], []),
    attempt_ritual: IDL.Func([IDL.Nat8], [IDL.Variant({ Ok: RitualResult, Err: IDL.Text })], []),
    get_recent_commentaries: IDL.Func([IDL.Nat64], [IDL.Vec(DrawCommentary)], ["query"]),
    get_chronicle: IDL.Func([IDL.Nat64], [IDL.Vec(ChronicleEntry)], ["query"]),
    get_notable_events: IDL.Func([], [IDL.Vec(NotableEvent)], ["query"]),
    get_arena_stats: IDL.Func([], [IDL.Nat64, IDL.Nat], ["query"]),
    is_arena_enabled: IDL.Func([], [IDL.Bool], ["query"]),
  });
};

// ── State ──

let DRAW_INTERVAL = isLocal ? 60 : 600;
const TIMER_CIRCUMFERENCE = 2 * Math.PI * 32; // r=32

let principal;
let identity;
let actors;
let lastDrawTimestamp = 0;
let lastKnownRound = 0;
let lastRenderedRound = 0;
let bootTimestamp = Date.now();
let pendingHistory = null;
let pendingDraw = null;
let latestLotteryStats = null;
let latestStakeInfo = null;
let latestStreakInfo = null;
let timerReady = false;
let timerInterval = null;
let cachedCommentaries = [];

async function makeActors(currentIdentity) {
  const agent = await HttpAgent.create({ host, identity: currentIdentity ?? undefined });
  if (isLocal) await agent.fetchRootKey();
  return {
    ledger: Actor.createActor(ledgerIDL, { agent, canisterId: ids.ledger }),
    token: Actor.createActor(tokenIDL, { agent, canisterId: ids.token }),
    treasury: Actor.createActor(treasuryIDL, { agent, canisterId: ids.treasury }),
    lottery: Actor.createActor(lotteryIDL, { agent, canisterId: ids.lottery }),
    ignis: ids.ignis ? Actor.createActor(ignisIDL, { agent, canisterId: ids.ignis }) : null,
  };
}

// ── Boot ──

async function boot() {
  actors = await makeActors(null);
  bindEvents();
  if (!isLocal) {
    els.faucetBtn.style.display = "none";
    els.newIdentityBtn.style.display = "none";
    // Restore previous Internet Identity session
    try {
      const client = await getAuthClient();
      if (await client.isAuthenticated()) {
        await activateIdentity(client.getIdentity(), "Session restored.");
      }
    } catch (_) {}
  }
  try {
    const interval = await actors.lottery.get_draw_interval();
    if (interval && Number(interval) > 0) DRAW_INTERVAL = Number(interval);
  } catch (_) {}
  await refreshAll();
  startDrawTimer();
  setInterval(refreshAll, 20_000);
}

function startDrawTimer() {
  if (timerInterval) clearInterval(timerInterval);
  timerInterval = setInterval(tickTimer, 1000);
}

function showTimer() {
  if (timerReady) return;
  timerReady = true;
  els.timerSpinner.hidden = true;
  els.timerSeconds.hidden = false;
  els.timerSuffix.hidden = false;
  tickTimer();
}

let drawPollActive = false;
let pollStartTime = 0;
const TIMER_LAG_SECS = 7;

function tickTimer() {
  if (!timerReady) return;
  const nowMs = Date.now();
  const lastMs = lastDrawTimestamp > 0
    ? Number(BigInt(lastDrawTimestamp) / 1_000_000n)
    : bootTimestamp;

  const frontendStartMs = lastMs + TIMER_LAG_SECS * 1000;
  const elapsed = Math.max(0, (nowMs - frontendStartMs) / 1000);
  const displayed = Math.max(0, Math.ceil(DRAW_INTERVAL - elapsed));

  els.timerSeconds.textContent = displayed;
  const fraction = displayed / DRAW_INTERVAL;
  els.timerArc.setAttribute("stroke-dashoffset",
    (TIMER_CIRCUMFERENCE * (1 - fraction)).toFixed(2));

  const urgent = displayed <= 10;
  els.timerArc.classList.toggle("urgent", urgent);
  els.timerSeconds.classList.toggle("urgent", urgent);

  // Live strip timer
  const mins = String(Math.floor(displayed / 60)).padStart(2, "0");
  const secs = String(displayed % 60).padStart(2, "0");
  if (els.liveTimer) els.liveTimer.textContent = `${mins}:${secs}`;

  if (els.buyStatus) {
    els.buyStatus.textContent = urgent ? "drawing soon" : "open";
  }

  // Safety: force-reset stuck poll after 25s
  if (drawPollActive && pollStartTime && (nowMs - pollStartTime > 25000)) {
    drawPollActive = false;
    pollStartTime = 0;
  }

  if (displayed <= TIMER_LAG_SECS && !drawPollActive && !pendingDraw) {
    drawPollActive = true;
    pollStartTime = nowMs;
    pollForNewDraw();
  }

  if (displayed <= 0 && pendingDraw) {
    const draw = pendingDraw;
    pendingDraw = null;
    lastDrawTimestamp = draw.timestamp;
    showVictoryCelebration(draw);
    setTimeout(() => {
      if (pendingHistory) {
        renderWinnerHistory(pendingHistory);
        lastRenderedRound = lastKnownRound;
        pendingHistory = null;
      }
    }, 4500);
  }
}

async function pollForNewDraw() {
  const prevRound = lastKnownRound;
  for (let i = 0; i < 10; i++) {
    try {
      await refreshAll();
    } catch (e) { console.warn("poll refresh failed", e); }
    if (lastKnownRound > prevRound) { drawPollActive = false; pollStartTime = 0; return; }
    await new Promise(r => setTimeout(r, 2000));
  }
  drawPollActive = false;
  pollStartTime = 0;
}

// ── Events ──

function bindEvents() {
  els.loginBtn.addEventListener("click", login);
  els.newIdentityBtn.addEventListener("click", createNewIdentity);
  els.faucetBtn.addEventListener("click", faucet);
  els.buyBtn.addEventListener("click", buyTicket);
  els.ticketSlider.addEventListener("input", updateSliderDisplay);
  els.stakeBtn.addEventListener("click", () => doStake(true));
  els.unstakeAllBtn.addEventListener("click", unstakeAll);
  els.drawBtn.addEventListener("click", refreshDraws);
  els.copyPrincipalBtn.addEventListener("click", copyPrincipal);
  els.sendBtn.addEventListener("click", doSend);
  initCustomSelect();

  // Quick stake buttons
  document.querySelectorAll("[data-stake-amount]").forEach((btn) => {
    btn.addEventListener("click", () => {
      els.stakeAmount.value = btn.dataset.stakeAmount;
    });
  });

  // IGNIS events
  els.ignisChatSendBtn.addEventListener("click", ignisChat);
  els.ignisChatInput.addEventListener("keydown", (e) => { if (e.key === "Enter") ignisChat(); });
  els.ignisRitualRequestBtn.addEventListener("click", ignisRequestRitual);
  els.ignisRitualAgainBtn.addEventListener("click", ignisResetRitual);
  els.ignisCommentaryRefresh.addEventListener("click", refreshIgnisCommentaries);
  els.chroniclePrev.addEventListener("click", () => ignisChronicleNav(-1));
  els.chronicleNext.addEventListener("click", () => ignisChronicleNav(1));

  // Parlor ritual
  if (els.parlorRitualRequestBtn) els.parlorRitualRequestBtn.addEventListener("click", parlorRequestRitual);
  if (els.parlorRitualAgainBtn) els.parlorRitualAgainBtn.addEventListener("click", parlorResetRitual);

  // Navigation — all buttons with data-page
  document.querySelectorAll("[data-page]").forEach((link) => {
    link.addEventListener("click", (e) => {
      e.preventDefault();
      const page = link.dataset.page;
      document.querySelectorAll(".nav-link").forEach((l) => l.classList.remove("active"));
      document.querySelectorAll(`.nav-link[data-page="${page}"]`).forEach((b) => b.classList.add("active"));
      document.querySelectorAll(".page").forEach((p) => p.classList.remove("active"));
      document.getElementById(`page-${page}`).classList.add("active");
    });
  });
}

// ── Identity ──

let authClient = null;

async function getAuthClient() {
  if (authClient) return authClient;
  const deps = "@dfinity/agent@2.1.3,@dfinity/identity@2.1.3,@dfinity/candid@2.1.3,@dfinity/principal@2.1.3";
  const { AuthClient: AC } = await import(`https://esm.sh/@dfinity/auth-client@2.1.3?deps=${deps}`);
  authClient = await AC.create();
  return authClient;
}

async function login() {
  if (isLocal) {
    await activateIdentity(loadOrCreateIdentity(), "Local identity active.");
  } else {
    const client = await getAuthClient();
    await new Promise((resolve, reject) => {
      client.login({
        identityProvider: "https://id.ai",
        maxTimeToLive: BigInt(7 * 24 * 60 * 60 * 1_000_000_000),
        onSuccess: resolve,
        onError: reject,
      });
    });
    await activateIdentity(client.getIdentity(), "Internet Identity active.");
  }
}

async function createNewIdentity() {
  const nextIdentity = Ed25519KeyIdentity.generate();
  saveIdentity(nextIdentity);
  await activateIdentity(nextIdentity, "New local principal active.");
}

function loadOrCreateIdentity() {
  const stored = localStorage.getItem(IDENTITY_STORAGE_KEY);
  if (stored) {
    try { return Ed25519KeyIdentity.fromJSON(stored); }
    catch (err) { console.warn("stored identity could not be loaded", err); localStorage.removeItem(IDENTITY_STORAGE_KEY); }
  }
  const nextIdentity = Ed25519KeyIdentity.generate();
  saveIdentity(nextIdentity);
  return nextIdentity;
}

function saveIdentity(nextIdentity) {
  localStorage.setItem(IDENTITY_STORAGE_KEY, JSON.stringify(nextIdentity.toJSON()));
}

async function activateIdentity(nextIdentity, message) {
  identity = nextIdentity;
  principal = identity.getPrincipal();
  latestStakeInfo = null;
  actors = await makeActors(identity);
  const pText = principal.toText();
  els.principalText.textContent = shortPrincipal(principal);
  els.walletPrincipal.textContent = pText;
  els.statusPrincipal.textContent = shortPrincipal(principal);
  // Update avatar letter
  const avatar = document.querySelector(".status-card .avatar");
  if (avatar) avatar.textContent = pText.charAt(0).toUpperCase();
  // Enable buttons
  els.buyBtn.disabled = false;
  els.ticketSlider.disabled = false;
  els.stakeBtn.disabled = false;
  els.unstakeAllBtn.disabled = false;
  els.sendBtn.disabled = false;
  els.ignisChatSendBtn.disabled = false;
  els.ignisRitualRequestBtn.disabled = false;
  if (els.parlorRitualRequestBtn) els.parlorRitualRequestBtn.disabled = false;
  setNotice(message, "good");
  await refreshAll();
}

// ── Actions ──

async function faucet() {
  if (!isLocal) return;
  const p = ensurePrincipal();
  if (!p) return;
  setNotice("Minting local test ICP...", "");
  const result = await actors.ledger.faucet(defaultAccount(p), 5n * E8S);
  if ("Err" in result) { setNotice(result.Err, "bad"); }
  else { setNotice("Added 5 local test ICP.", "good"); }
  await refreshBalances();
}

const DRAW_STEPS = [14, 28, 42, 56, 70, 84, 98, 112, 126, 144];

function getSliderStep() {
  return Math.round((parseFloat(els.ticketSlider.value) - 0.1) / 0.1);
}

function updateSliderDisplay() {
  const idx = getSliderStep();
  const draws = DRAW_STEPS[idx];
  const icp = (idx + 1) * 0.1;
  els.sliderIcp.textContent = `${icp.toFixed(1)} ICP`;
  els.sliderDraws.textContent = `${draws} draws`;
  els.buyBtn.querySelector("span").textContent = `Buy`;
}

async function buyTicket() {
  const p = ensurePrincipal();
  if (!p) return;
  const idx = getSliderStep();
  const draws = DRAW_STEPS[idx];
  const icp = (idx + 1) * 0.1;
  const amountE8s = BigInt((idx + 1) * 10_000_000);

  els.buyBtn.disabled = true;
  setNotice(`Sending ${icp.toFixed(1)} ICP...`, "");
  try {
    const deposit = await actors.lottery.get_deposit_account(p);
    if ("Err" in deposit) throw new Error(deposit.Err);
    const depositAmount = amountE8s + TICKET_SPLIT_FEE_COUNT * ICP_LEDGER_FEE;
    const transfer = await actors.ledger.icrc1_transfer({
      from_subaccount: [], to: deposit.Ok, amount: depositAmount,
      fee: [], memo: [], created_at_time: [],
    });
    if ("Err" in transfer) throw new Error("ICP transfer failed");
    setNotice("Payment sent. Claiming ticket...", "");
    const ticket = await actors.lottery.buy_ticket(amountE8s);
    if ("Err" in ticket) throw new Error(ticket.Err);
    setNotice(`Ticket purchased — ${draws} draws.`, "good");
  } catch (err) {
    setNotice(err.message ?? String(err), "bad");
  } finally {
    els.buyBtn.disabled = false;
    await refreshAll();
  }
}

async function doStake(isStake) {
  const p = ensurePrincipal();
  if (!p) return;
  const amount = luckyToE8s(els.stakeAmount.value);
  if (amount <= 0n) { setNotice("Enter a positive LUCKY amount.", "bad"); return; }
  const result = isStake ? await actors.token.stake(amount) : await actors.token.unstake(amount);
  if ("Err" in result) { setNotice(result.Err, "bad"); }
  else { setNotice(isStake ? "LUCKY staked." : "LUCKY unstaked.", "good"); }
  await Promise.all([refreshBalances(), refreshStaking()]);
}

async function unstakeAll() {
  const p = ensurePrincipal();
  if (!p) return;
  if (!latestStakeInfo || latestStakeInfo.unlockable <= 0n) { setNotice("No unlockable LUCKY.", "bad"); return; }
  const result = await actors.token.unstake(latestStakeInfo.unlockable);
  if ("Err" in result) { setNotice(result.Err, "bad"); }
  else { setNotice("All unlocked LUCKY unstaked.", "good"); }
  await Promise.all([refreshBalances(), refreshStaking()]);
}

async function unstakeBatch(amount) {
  const p = ensurePrincipal();
  if (!p) return;
  const result = await actors.token.unstake(amount);
  if ("Err" in result) { setNotice(result.Err, "bad"); }
  else { setNotice("LUCKY unstaked.", "good"); }
  await Promise.all([refreshBalances(), refreshStaking()]);
}

function setWalletNotice(text, kind) {
  els.walletNotice.textContent = text;
  els.walletNotice.className = `notice ${kind}`;
}

async function copyPrincipal() {
  if (!principal) return;
  try { await navigator.clipboard.writeText(principal.toText()); setWalletNotice("Copied.", "good"); }
  catch { setWalletNotice("Copy failed.", "bad"); }
}

function initCustomSelect() {
  const wrap = document.getElementById("walletActionWrap");
  const trigger = document.getElementById("walletActionTrigger");
  const menu = document.getElementById("walletActionMenu");
  if (!wrap || !trigger || !menu) return;

  trigger.addEventListener("click", () => wrap.classList.toggle("open"));

  menu.querySelectorAll(".custom-select-option:not(.disabled)").forEach(opt => {
    opt.addEventListener("click", () => {
      els.walletAction.value = opt.dataset.value;
      trigger.querySelector("span").textContent = opt.textContent;
      menu.querySelectorAll(".custom-select-option").forEach(o => o.classList.remove("active"));
      opt.classList.add("active");
      wrap.classList.remove("open");
      updateSendForm();
    });
  });

  document.addEventListener("click", (e) => {
    if (!wrap.contains(e.target)) wrap.classList.remove("open");
  });
}

function updateSendForm() {
  const action = els.walletAction.value;
  const isIcp = action === "send-icp";
  els.sendAmountLabel.textContent = isIcp ? "Amount (ICP)" : "Amount (LUCKY)";
  els.sendAmount.step = isIcp ? "0.001" : "100";
  els.sendAmount.value = isIcp ? "0.1" : "100";
  els.sendBtn.textContent = isIcp ? "Send ICP" : "Send LUCKY";
}

async function doSend() {
  const p = ensurePrincipal();
  if (!p) return;
  const action = els.walletAction.value;
  const toText = els.sendTo.value.trim();
  if (!toText) { setWalletNotice("Enter a recipient principal.", "bad"); return; }
  const isIcp = action === "send-icp";
  const amount = isIcp ? parseUnits(els.sendAmount.value, 8) : luckyToE8s(els.sendAmount.value);
  if (amount <= 0n) { setWalletNotice("Enter a positive amount.", "bad"); return; }
  els.sendBtn.disabled = true;
  setWalletNotice(isIcp ? "Sending ICP..." : "Sending LUCKY...", "");
  try {
    const { Principal } = await import("https://esm.sh/@dfinity/principal@2.1.3");
    const toPrincipal = Principal.fromText(toText);
    const actor = isIcp ? actors.ledger : actors.token;
    const result = await actor.icrc1_transfer({
      from_subaccount: [], to: { owner: toPrincipal, subaccount: [] },
      amount, fee: [], memo: [], created_at_time: [],
    });
    if ("Err" in result) throw new Error(JSON.stringify(result.Err));
    const formatted = isIcp ? `${formatIcp(amount)} ICP` : `${formatLucky(amount)} LUCKY`;
    setWalletNotice(`Sent ${formatted}.`, "good");
  } catch (err) {
    setWalletNotice(err.message ?? String(err), "bad");
  } finally {
    els.sendBtn.disabled = false;
    await refreshBalances();
  }
}

async function refreshDraws() {
  setNotice("Refreshing...", "");
  await refreshAll();
  setNotice("Updated.", "good");
}

// ── Refresh ──

async function refreshAll() {
  await Promise.all([refreshStats(), refreshBalances(), refreshStaking(), refreshWinners(), refreshIgnis(), refreshStreak(), refreshTicketStatus(), refreshActivity()]);
}

async function refreshTicketStatus() {
  if (!principal) return;
  try {
    const tickets = await actors.lottery.get_my_tickets(principal);
    const count = tickets.length;
    if (els.statusTickets) els.statusTickets.textContent = `${count} active`;
    if (els.ticketScroller) {
      if (!count) {
        els.ticketScroller.innerHTML = "";
      } else {
        els.ticketScroller.classList.toggle("single", count === 1);
        els.ticketScroller.innerHTML = tickets.map((t) => {
          const rem = Number(t.remaining_draws);
          const tot = Number(t.total_draws);
          const pct = tot > 0 ? (rem / tot * 100).toFixed(0) : 0;
          const low = pct < 20;
          const hash = ticketIdToHash(Number(t.ticket_id));
          return `<div class="ticket-item">
            <div class="ticket-hash">${hash}</div>
            <div class="ticket-bottom">
              <div class="ticket-bar"><div class="ticket-bar-fill${low ? " low" : ""}" style="width:${pct}%"></div></div>
              <span class="ticket-draws">${rem}/${tot} draws</span>
            </div>
          </div>`;
        }).join("");
      }
    }
  } catch (err) {
    console.warn("ticket status refresh failed", err);
  }
}

function ticketIdToHash(id) {
  let h = id * 2654435761 >>> 0;
  const chars = "abcdefghijklmnopqrstuvwxyz0123456789";
  const parts = [];
  for (let p = 0; p < 5; p++) {
    let seg = "";
    for (let i = 0; i < 5; i++) {
      h = (h * 31 + 17) >>> 0;
      seg += chars[h % chars.length];
    }
    parts.push(seg);
  }
  return parts.join("-");
}

async function refreshStats() {
  try {
    const [lotteryStats, treasuryStats] = await Promise.all([
      actors.lottery.get_stats(),
      actors.treasury.get_stats(),
    ]);

    // Live strip
    const players = lotteryStats.active_players.toString();
    const tickets = lotteryStats.active_tickets.toString();
    if (els.liveRound) els.liveRound.textContent = `#${lotteryStats.round}`;
    if (els.liveStats) els.liveStats.textContent = `${players} playing · ${tickets} tickets`;
    if (els.buyRound) els.buyRound.textContent = `Round #${lotteryStats.round}`;

    // Burn stats
    const burnPool = Number(lotteryStats.burn_pool_lucky_e8s) / 1e8;
    const totalBurned = Number(lotteryStats.total_burned_lucky_e8s) / 1e8;
    const lastBurnEpoch = Number(lotteryStats.last_burn_epoch);

    if (els.burnPoolNum) els.burnPoolNum.textContent = burnPool.toLocaleString();
    if (els.burnTotalNum) els.burnTotalNum.textContent = totalBurned.toLocaleString();

    if (els.liveTicker) {
      els.liveTicker.innerHTML = `<span>${totalBurned.toLocaleString()} LUCKY burned \u00B7 pool ${burnPool.toLocaleString()} LUCKY \u00B7 ${players} players \u00B7 ${tickets} tickets</span>`;
    }

    if (lastBurnEpoch > 0) {
      const nextBurnMs = (lastBurnEpoch / 1e6) + 7 * 86400 * 1000;
      const remainMs = nextBurnMs - Date.now();
      if (remainMs > 0) {
        const days = Math.floor(remainMs / 86400000);
        const hrs = Math.floor((remainMs % 86400000) / 3600000);
        if (els.nextBurnText) els.nextBurnText.textContent = `${days}d ${hrs}h`;
      } else {
        if (els.nextBurnText) els.nextBurnText.textContent = "Pending";
      }
    }

    latestLotteryStats = lotteryStats;
    updateRewardPreview();
  } catch (err) {
    console.warn("stats refresh failed", err);
  }
}

async function refreshBalances() {
  if (!principal) return;
  try {
    const [icp, stakeInfo] = await Promise.all([
      actors.ledger.icrc1_balance_of(defaultAccount(principal)),
      actors.token.get_stake_info(principal),
    ]);

    const icpFormatted = `${formatIcp(icp)} ICP`;
    const luckyFormatted = formatLucky(stakeInfo.liquid);

    // Staking page
    els.totalStaked.textContent = `${formatLucky(stakeInfo.staked)} LUCKY`;
    els.unlockableBalance.textContent = `${formatLucky(stakeInfo.unlockable)} LUCKY`;
    els.lockedBalance.textContent = `${formatLucky(stakeInfo.locked)} LUCKY`;
    els.boostText.textContent = `${(Number(stakeInfo.boost_bps) / 10000).toFixed(2)}x`;

    // Staking page liquid
    if (els.stakeLiquid) els.stakeLiquid.textContent = `${luckyFormatted} LUCKY`;

    // Wallet page
    els.walletIcp.textContent = `${formatIcp(icp)}`;
    els.walletLucky.textContent = luckyFormatted;
    els.walletStaked.textContent = formatLucky(stakeInfo.staked);
    els.walletTotal.textContent = formatLucky(stakeInfo.total);

    // Status card on parlor
    if (els.statusWallet) els.statusWallet.textContent = `${luckyFormatted} LUCKY`;

    // Tier track update
    updateTierTrack(Number(stakeInfo.staked) / 1e8);

    latestStakeInfo = stakeInfo;
    updateRewardPreview();
  } catch (err) {
    console.warn("balance refresh failed", err);
  }
}

function updateTierTrack(stakedAmount) {
  if (!els.tierTrack) return;
  const tiers = els.tierTrack.querySelectorAll(".tier");
  const thresholds = [0, 1000, 5000, 10000, 25000, 50000];
  let lastActive = 0;
  thresholds.forEach((t, i) => { if (stakedAmount >= t) lastActive = i; });
  tiers.forEach((tier, i) => {
    tier.classList.toggle("active", i === lastActive);
  });
}

function updateRewardPreview() {
  if (!latestLotteryStats) return;
  const activePlayers = Number(latestLotteryStats.active_players);
  const halvingDivisor = 2 ** Number(latestLotteryStats.halving_level ?? 0n);
  const baseReward = Math.max(1, Math.min(BASE_REWARD_LUCKY + activePlayers * PER_PLAYER_REWARD_LUCKY, MAX_REWARD_LUCKY) / halvingDivisor);
  const stakeBps = latestStakeInfo ? Number(latestStakeInfo.boost_bps) : 10_000;
  const streakBps = latestStreakInfo ? Number(latestStreakInfo.streak_boost_bps) : 0;
  const totalBps = stakeBps + streakBps;
  const reward = baseReward * totalBps / 10_000;
  const span = els.rewardText.querySelector("span");
  if (span) span.textContent = formatDisplayNumber(reward);
  else els.rewardText.textContent = formatDisplayNumber(reward);
}

async function refreshStaking() {
  if (!principal) return;
  try {
    const batches = await actors.token.get_stake_batches(principal);
    const hasUnlockable = batches.some((b) => b.unlocked);
    els.unstakeAllBtn.disabled = !hasUnlockable;
    if (!batches.length) {
      els.stakeBatchList.innerHTML = '<div class="empty">No active stakes.</div>';
      return;
    }

    els.stakeBatchList.innerHTML = batches.map((b) => {
      const amount = formatLucky(b.amount);
      const stakedAt = formatTimestamp(b.staked_at);
      const status = b.unlocked ? "Ready" : timeRemaining(b.unlocks_at);
      const statusColor = b.unlocked ? "var(--cherry)" : "var(--ink-soft)";
      const unstakeBtn = b.unlocked
        ? `<button class="btn sm" data-unstake-amount="${b.amount}">Unstake</button>`
        : `<button class="btn sm" disabled>Locked</button>`;
      return `
        <div class="stake-row">
          <div class="cell"><div class="lbl">Amount</div><div class="v">${amount}</div></div>
          <div class="cell"><div class="lbl">Staked</div><div class="v" style="font-size:0.88rem">${stakedAt}</div></div>
          <div class="cell"><div class="lbl">Status</div><div class="v" style="color:${statusColor}">${status}</div></div>
          ${unstakeBtn}
        </div>
      `;
    }).join("");

    els.stakeBatchList.querySelectorAll("[data-unstake-amount]").forEach((btn) => {
      btn.addEventListener("click", () => unstakeBatch(BigInt(btn.dataset.unstakeAmount)));
    });
  } catch (err) {
    console.warn("staking refresh failed", err);
  }
}

async function refreshStreak() {
  if (!principal) return;
  try {
    const info = await actors.lottery.get_streak_info(principal);
    latestStreakInfo = info;
    const days = Number(info.consecutive_days);
    const boostPct = (Number(info.streak_boost_bps) / 100).toFixed(1);
    const alive = info.streak_alive;

    // Status card
    if (els.statusStreak) {
      els.statusStreak.textContent = days > 0 ? `${days}d · +${boostPct}%` : "0d";
    }

    // Streak boxes
    if (els.streakBoxes) {
      const boxes = els.streakBoxes.querySelectorAll(".streak-box");
      boxes.forEach((box, i) => {
        const dayNum = i + 1;
        box.classList.toggle("done", dayNum <= days);
        box.classList.toggle("today", dayNum === days);
        box.textContent = dayNum <= days ? "\u2713" : dayNum;
      });
    }

    // Title & sticker
    if (els.streakTitle) els.streakTitle.textContent = days > 0 ? `${days} day${days > 1 ? "s" : ""} in a row` : "Build your streak";
    if (els.streakBoostSticker) {
      if (Number(info.streak_boost_bps) > 0) {
        els.streakBoostSticker.textContent = `+${boostPct}%`;
        els.streakBoostSticker.hidden = false;
      } else {
        els.streakBoostSticker.hidden = true;
      }
    }

    updateRewardPreview();
  } catch (err) {
    console.warn("streak refresh failed", err);
  }
}

async function refreshWinners() {
  try {
    const history = await actors.lottery.get_draw_history();
    if (!history.length) {
      els.winnersList.innerHTML = '<div class="empty">No draws yet.</div>';
      showTimer();
      return;
    }

    const latest = history[history.length - 1];
    const newRound = Number(latest.round);
    const isNewDraw = lastKnownRound > 0 && newRound > lastKnownRound;

    if (isNewDraw) {
      lastKnownRound = newRound;
      showTimer();
      pendingDraw = latest;
      pendingHistory = history;
      return;
    }

    lastDrawTimestamp = latest.timestamp;
    showTimer();
    if (newRound !== lastRenderedRound) {
      renderWinnerHistory(history);
      lastRenderedRound = newRound;
    }
    lastKnownRound = newRound;
  } catch (err) {
    console.warn("winner refresh failed", err);
  }
}

function renderWinnerHistory(history) {
  const myPrincipal = principal ? principal.toText() : "";
  // Build round→commentary map from cached commentaries
  const commentaryMap = {};
  for (const c of cachedCommentaries) {
    const r = Number(c.round);
    if (!commentaryMap[r]) commentaryMap[r] = c.text;
  }

  els.winnersList.innerHTML = [...history].reverse().slice(0, 20).map((item) => {
    const winner = item.winner.toText();
    const isMe = myPrincipal && winner === myPrincipal;
    const short = `${winner.slice(0, 10)}...${winner.slice(-6)}`;
    const reward = formatLucky(item.reward_lucky_e8s);
    const boost = (Number(item.winning_weight_bps) / 10000).toFixed(2);
    const round = Number(item.round);
    const commentary = commentaryMap[round];
    const ignisLine = commentary
      ? `<div class="feed-ignis">\uD83D\uDD25 <em>${escapeHtml(commentary)}</em></div>`
      : "";
    return `
      <div class="feed-item ${isMe ? "is-you" : ""}">
        <div class="feed-medal">#${String(round).slice(-3)}</div>
        <div class="feed-content">
          <div class="feed-who">${escapeHtml(isMe ? "you" : short)}</div>
          <div class="feed-meta">
            <span>${boost}x reward</span>
          </div>
          ${ignisLine}
        </div>
        <div class="feed-prize">+${reward} <span style="font-size:0.74em; opacity:0.7">LUCKY</span></div>
      </div>
    `;
  }).join("");
}

// ── Activity Feed ──

async function refreshActivity() {
  if (!principal || !els.activityFeed) return;
  try {
    const myText = principal.toText();
    const [txs, history, batches] = await Promise.all([
      actors.ledger.get_transactions ? actors.ledger.get_transactions() : Promise.resolve([]),
      actors.lottery.get_draw_history(),
      actors.token.get_stake_batches(principal),
    ]);

    const events = [];

    // ICP transfers involving this principal
    for (const tx of txs) {
      const from = tx.from?.owner ? principalToText(tx.from.owner) : "";
      const to = tx.to?.owner ? principalToText(tx.to.owner) : "";
      if (from !== myText && to !== myText) continue;
      const isSend = from === myText;
      events.push({
        ts: Number(tx.timestamp),
        icon: isSend ? "\u2191" : "\u2193",
        label: isSend ? "Sent ICP" : "Received ICP",
        detail: `${formatIcp(tx.amount)} ICP`,
        cls: isSend ? "send" : "receive",
      });
    }

    // Wins
    for (const d of history) {
      if (principalToText(d.winner) !== myText) continue;
      events.push({
        ts: Number(d.timestamp),
        icon: "\u2605",
        label: `Won round #${Number(d.round)}`,
        detail: `+${formatLucky(d.reward_lucky_e8s)} LUCKY`,
        cls: "win",
      });
    }

    // Stakes
    for (const b of batches) {
      events.push({
        ts: Number(b.staked_at),
        icon: "\u26A1",
        label: "Staked",
        detail: `${formatLucky(b.amount)} LUCKY`,
        cls: "stake",
      });
    }

    events.sort((a, b) => b.ts - a.ts);
    const top = events.slice(0, 15);

    if (!top.length) {
      els.activityFeed.innerHTML = '<div class="empty">No activity yet.</div>';
      return;
    }

    els.activityFeed.innerHTML = top.map(e => `
      <div class="activity-item ${e.cls}">
        <span class="activity-icon">${e.icon}</span>
        <div class="activity-body">
          <span class="activity-label">${escapeHtml(e.label)}</span>
          <span class="activity-detail">${escapeHtml(e.detail)}</span>
        </div>
        <span class="activity-time">${formatTimestamp(BigInt(e.ts))}</span>
      </div>
    `).join("");
  } catch (err) {
    console.warn("activity refresh failed", err);
  }
}

// ── IGNIS ──

const STAGE_NAMES = { Spark: "SPARK", Flame: "FLAME", Inferno: "INFERNO", Supernova: "SUPERNOVA", Singularity: "SINGULARITY" };
const MOOD_NAMES = { Dormant: "Dormant", Restless: "Restless", Curious: "Curious", Excited: "Excited", Ecstatic: "Ecstatic", Wrathful: "Wrathful" };

// Day-of-week mood: Mon=Sleepy, Tue/Wed=Happy, Thu=Curious, Fri/Sat=Hungry, Sun=Excited(burn day)
const DAY_MOODS = [
  { name: "Excited",  emoji: "\uD83D\uDD25" },  // Sun
  { name: "Sleepy",   emoji: "\uD83D\uDE34" },  // Mon
  { name: "Happy",    emoji: "\uD83D\uDE04" },  // Tue
  { name: "Happy",    emoji: "\uD83D\uDE04" },  // Wed
  { name: "Curious",  emoji: "\uD83E\uDDD0" },  // Thu
  { name: "Hungry",   emoji: "\uD83E\uDD24" },  // Fri
  { name: "Hungry",   emoji: "\uD83E\uDD24" },  // Sat
];

function getTodayMood() {
  return DAY_MOODS[new Date().getDay()];
}

// Full Ignis mascot SVG — ported from Claude Design lucky-art.jsx
function ignisSvg(stage = "flame", mood = "happy") {
  const palettes = {
    spark:       { hot: "#FFC857", warm: "#FF9961", cool: "#FF7A3D" },
    flame:       { hot: "#FFB347", warm: "#FF7A3D", cool: "#E54B4B" },
    inferno:     { hot: "#FF5A4D", warm: "#E54B4B", cool: "#9B2B2B" },
    supernova:   { hot: "#FFF6D6", warm: "#7B9EE6", cool: "#3B5BB0" },
    singularity: { hot: "#C4A6FF", warm: "#8E5BD9", cool: "#3B1B5C" },
  };
  const p = palettes[stage] || palettes.flame;

  function starPoints(cx, cy, r) {
    const pts = [];
    for (let i = 0; i < 10; i++) {
      const a = (Math.PI * 2 * i) / 10 - Math.PI / 2;
      const rr = i % 2 === 0 ? r : r * 0.42;
      pts.push(`${cx + Math.cos(a) * rr},${cy + Math.sin(a) * rr}`);
    }
    return pts.join(" ");
  }

  const eyes = {
    sleepy: `<g stroke="#2A1B0E" stroke-width="3.5" stroke-linecap="round" fill="none">
      <path d="M70 110 q 12 -6 24 0"/><path d="M126 110 q 12 -6 24 0"/></g>`,
    excited: `<g fill="#2A1B0E">
      <polygon points="${starPoints(82, 108, 11)}"/>
      <polygon points="${starPoints(138, 108, 11)}"/></g>`,
    hungry: `<g fill="#2A1B0E">
      <ellipse cx="82" cy="108" rx="6" ry="9"/><ellipse cx="138" cy="108" rx="6" ry="9"/>
      <path d="M118 138 q 0 8 -3 14 q 4 -2 7 0 z" fill="#7B9EE6"/></g>`,
    curious: `<g fill="#2A1B0E">
      <ellipse cx="82" cy="108" rx="7" ry="9"/><ellipse cx="138" cy="108" rx="7" ry="9"/>
      <circle cx="79" cy="105" r="2.2" fill="#FFF"/><circle cx="135" cy="105" r="2.2" fill="#FFF"/></g>`,
    happy: `<g stroke="#2A1B0E" stroke-width="4" stroke-linecap="round" fill="none">
      <path d="M72 112 q 10 -10 20 0"/><path d="M128 112 q 10 -10 20 0"/></g>`,
  };

  const mouths = {
    hungry: `<path d="M90 135 q 20 22 40 0 q -20 -4 -40 0 z" fill="#2A1B0E"/>`,
    excited: `<g><path d="M86 132 q 24 26 48 0 q -24 -6 -48 0 z" fill="#2A1B0E"/>
      <path d="M99 142 q 11 8 22 0 q -11 4 -22 0 z" fill="#E54B4B"/></g>`,
    sleepy: `<path d="M100 138 q 10 6 20 0" stroke="#2A1B0E" stroke-width="3.5" stroke-linecap="round" fill="none"/>`,
    happy: `<path d="M92 134 q 18 16 36 0" stroke="#2A1B0E" stroke-width="4" stroke-linecap="round" fill="none"/>`,
    curious: `<path d="M92 134 q 18 16 36 0" stroke="#2A1B0E" stroke-width="4" stroke-linecap="round" fill="none"/>`,
  };

  const crown = stage === "singularity" ? `<g transform="translate(85, 0)">
    <path d="M0 18 L 12 6 L 24 18 L 36 6 L 48 18 L 48 26 L 0 26 Z" fill="${p.hot}" stroke="#2A1B0E" stroke-width="3" stroke-linejoin="round"/>
    <circle cx="6" cy="6" r="3" fill="#E54B4B" stroke="#2A1B0E" stroke-width="1.5"/>
    <circle cx="42" cy="6" r="3" fill="#E54B4B" stroke="#2A1B0E" stroke-width="1.5"/>
    <circle cx="24" cy="2" r="3" fill="#FFC857" stroke="#2A1B0E" stroke-width="1.5"/></g>` : "";

  const uid = `ig${Math.random().toString(36).slice(2, 8)}`;

  return `<svg viewBox="0 0 220 240" style="overflow:visible">
    <defs>
      <radialGradient id="${uid}" cx="50%" cy="60%" r="55%">
        <stop offset="0%" stop-color="${p.hot}"/>
        <stop offset="60%" stop-color="${p.warm}"/>
        <stop offset="100%" stop-color="${p.cool}"/>
      </radialGradient>
    </defs>
    <ellipse cx="110" cy="218" rx="58" ry="8" fill="#2A1B0E" opacity="0.18"/>
    <g opacity="0.85">
      <circle cx="32" cy="80" r="4" fill="${p.warm}" stroke="#2A1B0E" stroke-width="1.5"/>
      <circle cx="190" cy="60" r="6" fill="${p.hot}" stroke="#2A1B0E" stroke-width="1.5"/>
      <circle cx="200" cy="170" r="3" fill="${p.warm}" stroke="#2A1B0E" stroke-width="1.2"/>
      <circle cx="20" cy="170" r="5" fill="${p.hot}" stroke="#2A1B0E" stroke-width="1.5"/>
      <path d="M180 110 l 8 0 M184 106 l 0 8" stroke="${p.cool}" stroke-width="2.5" stroke-linecap="round"/>
      <path d="M30 130 l 6 0 M33 127 l 0 6" stroke="${p.cool}" stroke-width="2.5" stroke-linecap="round"/>
    </g>
    <path d="M110 28 C 150 65, 178 105, 178 150 C 178 196, 148 220, 110 220 C 72 220, 42 196, 42 150 C 42 105, 70 65, 110 28 Z"
      fill="url(#${uid})" stroke="#2A1B0E" stroke-width="4"/>
    <path d="M110 28 C 150 65, 178 105, 178 150 C 178 196, 148 220, 110 220 C 72 220, 42 196, 42 150 C 42 105, 70 65, 110 28 Z"
      fill="none" stroke="#2A1B0E" stroke-width="4" transform="translate(0,4)" opacity="0.15"/>
    <path d="M70 90 C 78 70, 95 56, 110 56" stroke="${p.hot}" stroke-width="6" stroke-linecap="round" fill="none" opacity="0.7"/>
    <circle cx="60" cy="135" r="9" fill="${p.cool}" opacity="0.55"/>
    <circle cx="160" cy="135" r="9" fill="${p.cool}" opacity="0.55"/>
    ${eyes[mood] || eyes.happy}
    ${mouths[mood] || mouths.happy}
    ${crown}
  </svg>`;
}

let ignisChroniclePageNum = 0;

function getVariantKey(v) {
  return Object.keys(v).find(k => v[k] !== undefined) || Object.keys(v)[0];
}

async function refreshIgnis() {
  if (!actors?.ignis) return;
  try {
    const [state, commentaries] = await Promise.all([
      actors.ignis.get_state(),
      actors.ignis.get_recent_commentaries(10n),
    ]);
    const stageName = getVariantKey(state.stage);
    const moodName = getVariantKey(state.mood);

    const dayMood = getTodayMood();
    const stageKey = stageName.toLowerCase();
    const moodKey = dayMood.name.toLowerCase();

    // IGNIS page
    if (els.ignisStageBadge) {
      els.ignisStageBadge.innerHTML = `<svg width="14" height="14" viewBox="0 0 22 22"><path d="M11 2 C 13 6, 16 8, 16 12 C 16 16, 13.5 19, 11 19 C 8.5 19, 6 16, 6 12 C 6 9, 8 7, 8.5 5 C 9 7, 10 7, 10 5 C 10.3 4, 10.5 3, 11 2 Z" fill="#FFC857" stroke="none"/></svg> Stage &middot; ${STAGE_NAMES[stageName] || stageName}`;
    }
    if (els.ignisMoodText) els.ignisMoodText.textContent = `${dayMood.emoji} ${dayMood.name}`;

    // Full mascot portrait on IGNIS page
    if (els.ignisPortrait) {
      els.ignisPortrait.innerHTML = ignisSvg(stageKey, moodKey);
    }

    // Mood buttons
    const moodBtns = document.getElementById("ignisMoodButtons");
    if (moodBtns && !moodBtns.dataset.rendered) {
      moodBtns.dataset.rendered = "1";
      const allMoods = [
        { emoji: "\uD83D\uDE34", name: "Sleepy", day: "Mon" },
        { emoji: "\uD83D\uDE04", name: "Happy", day: "Tue-Wed" },
        { emoji: "\uD83E\uDDD0", name: "Curious", day: "Thu" },
        { emoji: "\uD83E\uDD24", name: "Hungry", day: "Fri-Sat" },
        { emoji: "\uD83D\uDD25", name: "Excited", day: "Sun" },
      ];
      moodBtns.innerHTML = allMoods.map(m => {
        const active = m.name === dayMood.name;
        return `<span class="mood-chip${active ? " active" : ""}" title="${m.day}">${m.emoji} ${m.name}</span>`;
      }).join("");
    }
    if (els.ignisHungerFill) els.ignisHungerFill.style.width = `${state.hunger}%`;
    if (els.ignisHungerText) els.ignisHungerText.textContent = `${state.hunger}%`;
    if (els.ignisBurnTotal) els.ignisBurnTotal.textContent = `${(Number(state.total_burn_e8s) / 1e8).toLocaleString()} LUCKY`;
    if (els.ignisArenaWins) els.ignisArenaWins.textContent = state.ignis_wins.toString();

    // Hunger bar color
    const hunger = Number(state.hunger);
    if (els.ignisHungerFill) {
      els.ignisHungerFill.className = `bar-fill ${hunger > 80 ? "cherry" : hunger > 50 ? "" : "fern"}`;
    }

    // Parlor mini ignis
    if (els.parlorIgnisStage) {
      els.parlorIgnisStage.innerHTML = `<svg width="12" height="12" viewBox="0 0 22 22"><path d="M11 2 C 13 6, 16 8, 16 12 C 16 16, 13.5 19, 11 19 C 8.5 19, 6 16, 6 12 C 6 9, 8 7, 8.5 5 C 9 7, 10 7, 10 5 C 10.3 4, 10.5 3, 11 2 Z" fill="#FFC857" stroke="none"/></svg> Stage &middot; ${STAGE_NAMES[stageName] || stageName}`;
    }
    if (els.parlorIgnisPortrait) {
      els.parlorIgnisPortrait.innerHTML = ignisSvg(stageKey, moodKey);
    }
    if (els.parlorIgnisStats) {
      els.parlorIgnisStats.textContent = `${dayMood.emoji} ${dayMood.name} · Hunger ${state.hunger}%`;
    }

    renderCommentaries(commentaries);
  } catch (err) {
    console.warn("ignis refresh failed", err);
  }
}

async function refreshIgnisCommentaries() {
  if (!actors?.ignis) return;
  try {
    const commentaries = await actors.ignis.get_recent_commentaries(20n);
    renderCommentaries(commentaries);
  } catch (err) {
    console.warn("commentary refresh failed", err);
  }
}

function renderCommentaries(commentaries) {
  cachedCommentaries = commentaries;
  if (!commentaries.length) {
    els.ignisCommentaryList.innerHTML = '<div class="empty">No commentaries yet. IGNIS will speak after the next draw.</div>';
    return;
  }
  els.ignisCommentaryList.innerHTML = commentaries.map(c => `
    <div class="commentary-card">
      <div class="commentary-head">
        <span>${formatTimestamp(c.timestamp)}</span>
      </div>
      <p class="commentary-text">"${escapeHtml(c.text)}"</p>
    </div>
  `).join("");

  // Update parlor bubble with latest commentary (array is newest-first)
  if (els.parlorIgnisBubble && commentaries.length > 0) {
    els.parlorIgnisBubble.textContent = commentaries[0].text;
  }
}

async function ignisChat() {
  if (!actors?.ignis || !principal) return;
  const message = els.ignisChatInput.value.trim();
  if (!message) return;
  els.ignisChatInput.value = "";
  els.ignisChatSendBtn.disabled = true;
  appendChatMsg("user", message);
  appendChatMsg("ignis", "...", "ignis-thinking");
  try {
    const result = await actors.ignis.chat(message);
    removeChatThinking();
    if ("Ok" in result) appendChatMsg("ignis", result.Ok);
    else appendChatMsg("ignis", result.Err, "ignis-error");
  } catch (err) {
    removeChatThinking();
    appendChatMsg("ignis", "*the flames flicker in confusion*", "ignis-error");
  } finally {
    els.ignisChatSendBtn.disabled = false;
    els.ignisChatInput.focus();
  }
}

function appendChatMsg(sender, text, extraClass = "") {
  const div = document.createElement("div");
  div.className = `bubble ${sender} ${extraClass}`;
  if (sender === "ignis") {
    div.innerHTML = `<span style="font-family:'Bricolage Grotesque',sans-serif; font-weight:800; font-size:0.72rem; color:var(--cherry-deep); text-transform:uppercase; letter-spacing:0.1em; display:block; margin-bottom:0.2rem">Ignis</span>${escapeHtml(text)}`;
  } else {
    div.textContent = text;
  }
  els.ignisChatMessages.appendChild(div);
  els.ignisChatMessages.scrollTop = els.ignisChatMessages.scrollHeight;
}

function removeChatThinking() {
  const thinking = els.ignisChatMessages.querySelector(".ignis-thinking");
  if (thinking) thinking.remove();
}

async function ignisRequestRitual() {
  if (!actors?.ignis || !principal) return;
  els.ignisRitualRequestBtn.disabled = true;
  try {
    const result = await actors.ignis.request_ritual();
    if ("Err" in result) { setNotice(result.Err, "bad"); els.ignisRitualRequestBtn.disabled = false; return; }
    const challenge = result.Ok;
    els.ignisRitualContent.hidden = true;
    els.ignisRitualResult.hidden = true;
    els.ignisRitualChallenge.hidden = false;
    if ("Riddle" in challenge) {
      els.ignisRitualQuestion.textContent = challenge.Riddle.question;
      els.ignisRitualOptions.innerHTML = challenge.Riddle.options.map((opt, i) => `
        <button class="ritual-option" data-choice="${i}">${escapeHtml(opt)}</button>
      `).join("");
    } else {
      els.ignisRitualQuestion.textContent = challenge.Choice.prompt;
      els.ignisRitualOptions.innerHTML = challenge.Choice.options.map((opt, i) => `
        <button class="ritual-option" data-choice="${i}">${escapeHtml(opt)}</button>
      `).join("");
    }
    els.ignisRitualOptions.querySelectorAll(".ritual-option").forEach(btn => {
      btn.addEventListener("click", () => ignisAttemptRitual(Number(btn.dataset.choice)));
    });
  } catch (err) {
    setNotice("Ritual request failed.", "bad");
    els.ignisRitualRequestBtn.disabled = false;
  }
}

async function ignisAttemptRitual(choice) {
  if (!actors?.ignis) return;
  els.ignisRitualOptions.querySelectorAll("button").forEach(b => b.disabled = true);
  try {
    const result = await actors.ignis.attempt_ritual(choice);
    if ("Err" in result) { setNotice(result.Err, "bad"); return; }
    const r = result.Ok;
    els.ignisRitualChallenge.hidden = true;
    els.ignisRitualResult.hidden = false;
    els.ignisRitualResultText.textContent = r.ignis_response;
    const boostPct = (Number(r.boost_bps) / 100).toFixed(1);
    if (els.ignisRitualBoost) {
      els.ignisRitualBoost.textContent = r.success
        ? `Ritual passed! +${boostPct}% reward bonus.`
        : `Ritual failed. +${boostPct}% consolation.`;
      els.ignisRitualBoost.style.color = r.success ? "var(--cherry)" : "var(--plum)";
    }
  } catch (err) {
    setNotice("Ritual failed.", "bad");
  }
}

function ignisResetRitual() {
  els.ignisRitualContent.hidden = false;
  els.ignisRitualChallenge.hidden = true;
  els.ignisRitualResult.hidden = true;
  els.ignisRitualRequestBtn.disabled = false;
}

// Parlor ritual (duplicated for the mini card)
async function parlorRequestRitual() {
  if (!actors?.ignis || !principal) return;
  els.parlorRitualRequestBtn.disabled = true;
  try {
    const result = await actors.ignis.request_ritual();
    if ("Err" in result) { setNotice(result.Err, "bad"); els.parlorRitualRequestBtn.disabled = false; return; }
    const challenge = result.Ok;
    els.parlorRitualContent.hidden = true;
    els.parlorRitualResult.hidden = true;
    els.parlorRitualChallenge.hidden = false;
    const q = "Riddle" in challenge ? challenge.Riddle : challenge.Choice;
    els.parlorRitualQuestion.textContent = q.question || q.prompt;
    const opts = q.options;
    els.parlorRitualOptions.innerHTML = opts.map((opt, i) => `
      <button class="ritual-option" data-choice="${i}">${escapeHtml(opt)}</button>
    `).join("");
    els.parlorRitualOptions.querySelectorAll(".ritual-option").forEach(btn => {
      btn.addEventListener("click", () => parlorAttemptRitual(Number(btn.dataset.choice)));
    });
  } catch (err) {
    setNotice("Ritual request failed.", "bad");
    els.parlorRitualRequestBtn.disabled = false;
  }
}

async function parlorAttemptRitual(choice) {
  if (!actors?.ignis) return;
  els.parlorRitualOptions.querySelectorAll("button").forEach(b => b.disabled = true);
  try {
    const result = await actors.ignis.attempt_ritual(choice);
    if ("Err" in result) { setNotice(result.Err, "bad"); return; }
    const r = result.Ok;
    els.parlorRitualChallenge.hidden = true;
    els.parlorRitualResult.hidden = false;
    const boostPct = (Number(r.boost_bps) / 100).toFixed(1);
    els.parlorRitualResultText.textContent = r.success
      ? `Ignis is pleased. +${boostPct}% on your next win.`
      : `Ignis tilts its head. Try tomorrow.`;
    els.parlorRitualResultText.style.color = r.success ? "var(--cherry)" : "var(--plum)";
  } catch (err) {
    setNotice("Ritual failed.", "bad");
  }
}

function parlorResetRitual() {
  els.parlorRitualContent.hidden = false;
  els.parlorRitualChallenge.hidden = true;
  els.parlorRitualResult.hidden = true;
  els.parlorRitualRequestBtn.disabled = false;
}

async function ignisChronicleNav(delta) {
  ignisChroniclePageNum = Math.max(0, ignisChroniclePageNum + delta);
  await refreshIgnisChronicle();
}

async function refreshIgnisChronicle() {
  if (!actors?.ignis) return;
  try {
    const entries = await actors.ignis.get_chronicle(BigInt(ignisChroniclePageNum));
    els.chroniclePage.textContent = `Page ${ignisChroniclePageNum + 1}`;
    els.chroniclePrev.disabled = ignisChroniclePageNum === 0;
    els.chronicleNext.disabled = entries.length < 10;
    if (!entries.length) {
      els.ignisChronicleList.innerHTML = '<div class="empty">The chronicle has not yet begun...</div>';
      return;
    }
    els.ignisChronicleList.innerHTML = entries.map(e => `
      <div class="chronicle-entry">
        <div class="chronicle-header">
          <span class="chronicle-chapter">Chapter ${e.chapter}</span>
          <span class="chronicle-time">${formatTimestamp(e.timestamp)}</span>
        </div>
        <p class="chronicle-text">${escapeHtml(e.text)}</p>
      </div>
    `).join("");
  } catch (err) {
    console.warn("chronicle refresh failed", err);
  }
}

// ── Victory Celebration ──

function showPhoenixWinnerReveal(draw) {
  const winnerFull = principalToText(draw.winner);
  const ticketIds = draw.ticket_ids ? Array.from(draw.ticket_ids, Number) : [];
  const winningTicketId = Number(draw.winning_ticket_id || 0);
  const ticketCount = ticketIds.length || Number(draw.active_tickets);
  const reward = formatLucky(draw.reward_lucky_e8s);
  const boost = (Number(draw.winning_weight_bps) / 10000).toFixed(2);
  const isMe = principal && winnerFull === principal.toText();

  const overlay = document.createElement("div");
  overlay.className = "reveal-overlay";
  overlay.innerHTML = `
    <div class="reveal-card">
      <div class="reveal-top">
        <div class="reveal-serial">SERIAL &middot; 0x${Number(draw.round).toString(16).toUpperCase()}A7F</div>
        <div class="reveal-stamp">DRAW</div>
      </div>

      <div class="reveal-banner">
        <div class="eyebrow">Round #${Number(draw.round)}</div>
        <h2>Drawing <em>now...</em></h2>
      </div>

      <div class="reel-window">
        <div class="reel-mark"></div>
        <div class="reel-mark r"></div>
        <div class="reel-track"></div>
      </div>

      <div class="reveal-result">
        <div class="reveal-prize">
          <span class="amount">${escapeHtml(reward)}</span>
          <span class="unit">LUCKY</span>
        </div>
        <div class="reveal-winner">${escapeHtml(winnerFull)}</div>
        <div style="margin-top:0.3rem; font-size:0.82rem; color:var(--ink-soft)">
          ${escapeHtml(boost)}x reward &middot; ${Number(draw.active_players)} players &middot; ${ticketCount} tickets
        </div>
        <div class="reveal-actions">
          <button class="btn primary reveal-dismiss">Continue</button>
        </div>
      </div>
    </div>
  `;

  document.body.appendChild(overlay);

  const reel = overlay.querySelector(".reel-track");
  const banner = overlay.querySelector(".reveal-banner h2");
  let rollTimers = [];
  let dismissed = false;

  const schedule = (fn, delay) => { const id = setTimeout(fn, delay); rollTimers.push(id); return id; };
  const clearRollTimers = () => { rollTimers.forEach(clearTimeout); rollTimers = []; };
  const dismiss = () => {
    if (dismissed) return;
    dismissed = true;
    activeRevealDismiss = null;
    clearRollTimers();
    clearTimeout(autoDismiss);
    overlay.classList.remove("show");
    setTimeout(() => overlay.remove(), 400);
  };
  activeRevealDismiss = dismiss;

  // Build reel
  const sequence = buildReelSequence(ticketIds, winningTicketId);
  sequence.forEach((item, index) => {
    const row = document.createElement("div");
    row.className = "reel-row" + (index === sequence.length - 1 ? " winner" : "");
    row.textContent = ticketIdToHash(Number(item));
    reel.appendChild(row);
  });

  const ROW = 64;
  const finalY = -((sequence.length - 1) * ROW);
  const nearY = -Math.floor(sequence.length * 0.72) * ROW;
  const settleY = -Math.floor(sequence.length * 0.94) * ROW;

  requestAnimationFrame(() => overlay.classList.add("show"));

  schedule(() => {
    reel.style.transition = "transform 1.35s linear";
    reel.style.transform = `translateY(${nearY}px)`;
  }, 120);
  schedule(() => {
    reel.style.transition = "transform 1s cubic-bezier(0.22, 0.6, 0.36, 1)";
    reel.style.transform = `translateY(${settleY}px)`;
  }, 1500);
  schedule(() => {
    reel.style.transition = "transform 0.75s cubic-bezier(0.2, 0.9, 0.22, 1.15)";
    reel.style.transform = `translateY(${finalY}px)`;
  }, 2600);
  schedule(() => {
    overlay.classList.add("settled");
    if (isMe) {
      overlay.classList.add("you-won");
      spawnConfetti(overlay);
    }
    banner.innerHTML = isMe ? `You <em>won</em> \u2605` : `Winner <em>drawn</em>`;
  }, 3400);

  overlay.querySelector(".reveal-dismiss").addEventListener("click", dismiss);
  const autoDismiss = setTimeout(dismiss, 10000);
}

function buildReelSequence(ticketIds, winningTicketId) {
  const source = ticketIds.length ? ticketIds : [winningTicketId];
  const rollLength = Math.min(80, Math.max(34, source.length * 2));
  const sequence = [];
  for (let i = 0; i < rollLength; i++) {
    sequence.push(source[Math.floor(Math.random() * source.length)]);
  }
  sequence.push(winningTicketId);
  return sequence;
}

let activeRevealDismiss = null;

function showVictoryCelebration(draw) {
  if (activeRevealDismiss) { activeRevealDismiss(); activeRevealDismiss = null; }
  showPhoenixWinnerReveal(draw);
}

// ── Confetti ──

function spawnConfetti(container) {
  const colors = ["#2A6B47", "#E5A92A", "#E54B4B", "#7B9EE6", "#F2D88B", "#FF8C42"];
  const count = 60;
  for (let i = 0; i < count; i++) {
    const el = document.createElement("div");
    el.className = "confetti-piece";
    el.style.setProperty("--x", `${(Math.random() - 0.5) * 600}px`);
    el.style.setProperty("--r", `${Math.random() * 720 - 360}deg`);
    el.style.left = `${40 + Math.random() * 20}%`;
    el.style.background = colors[Math.floor(Math.random() * colors.length)];
    el.style.animationDelay = `${Math.random() * 0.3}s`;
    el.style.animationDuration = `${1.2 + Math.random() * 0.8}s`;
    container.appendChild(el);
    el.addEventListener("animationend", () => el.remove(), { once: true });
  }
}

// ── Helpers ──

function ensurePrincipal() {
  if (!principal) { setNotice("Login first.", "bad"); return null; }
  return principal;
}

function defaultAccount(owner) { return { owner, subaccount: [] }; }

function setNotice(text, kind) {
  if (!text) return;
  const toast = document.createElement("div");
  toast.className = `toast ${kind}`;
  toast.textContent = text;
  let container = document.getElementById("toastContainer");
  if (!container) {
    container = document.createElement("div");
    container.id = "toastContainer";
    container.className = "toast-container";
    document.body.appendChild(container);
  }
  container.appendChild(toast);
  requestAnimationFrame(() => toast.classList.add("show"));
  const duration = kind === "bad" ? 5000 : 3000;
  setTimeout(() => {
    toast.classList.remove("show");
    toast.addEventListener("transitionend", () => toast.remove(), { once: true });
    setTimeout(() => toast.remove(), 400);
  }, duration);
}

function formatIcp(value) { return formatUnits(BigInt(value), 8, 4); }
function formatLucky(value) { return formatUnits(BigInt(value), 8, 2); }

function formatDisplayNumber(value) {
  return value.toLocaleString(undefined, {
    minimumFractionDigits: Number.isInteger(value) ? 0 : 2,
    maximumFractionDigits: 2,
  });
}

function formatTimestamp(nanos) {
  if (!nanos) return "-";
  const ms = Number(BigInt(nanos) / 1_000_000n);
  return new Date(ms).toLocaleString(undefined, { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

function timeRemaining(unlockNanos) {
  const diffMs = Number(BigInt(unlockNanos) / 1_000_000n) - Date.now();
  if (diffMs <= 0) return "Ready";
  const hours = Math.floor(diffMs / 3_600_000);
  const minutes = Math.floor((diffMs % 3_600_000) / 60_000);
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

function luckyToE8s(value) { return parseUnits(value, 8); }

function parseUnits(value, decimals) {
  const raw = String(value ?? "0").trim();
  if (!/^\d*(\.\d*)?$/.test(raw)) return 0n;
  const [wholeRaw, fractionRaw = ""] = raw.split(".");
  const whole = BigInt(wholeRaw || "0");
  const fraction = (fractionRaw + "0".repeat(decimals)).slice(0, decimals);
  return whole * 10n ** BigInt(decimals) + BigInt(fraction || "0");
}

function formatUnits(value, decimals, maxFraction) {
  const base = 10n ** BigInt(decimals);
  const whole = value / base;
  const fraction = value % base;
  const padded = fraction.toString().padStart(decimals, "0").slice(0, maxFraction);
  const trimmed = padded.replace(/0+$/, "");
  return trimmed ? `${whole.toLocaleString()}.${trimmed}` : whole.toLocaleString();
}

function principalToText(value) {
  if (!value) return "";
  return typeof value.toText === "function" ? value.toText() : String(value);
}

function shortPrincipal(value) {
  const text = principalToText(value);
  if (text.length <= 20) return text;
  return `${text.slice(0, 7)}...${text.slice(-5)}`;
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (char) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#039;",
  }[char]));
}

boot().catch((err) => {
  console.error(err);
  setNotice(err.message ?? String(err), "bad");
});
