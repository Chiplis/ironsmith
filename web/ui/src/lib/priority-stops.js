import { normalizePhaseStep, normalizeStepKey, currentPriorityPhaseLabel, PHASE_TRACK } from './constants.js';

export const PRIORITY_HOLD_WINDOW_MS = 2000;
export const COMBAT_STEPS = ['BeginCombat', 'DeclareAttackers', 'DeclareBlockers', 'FirstStrikeDamage', 'CombatDamage', 'EndCombat'];
export const priorityStepKey = state => state?.combat_damage_step === 'first_strike'
  && normalizeStepKey(state?.step) === 'CombatDamage' ? 'FirstStrikeDamage' : normalizeStepKey(state?.step);
const isLocalPriority = state => state?.decision?.kind === 'priority'
  && Number(state.decision.player) === Number(state.perspective);
const isLocalTurnDecision = state => ['priority', 'attackers', 'blockers'].includes(state?.decision?.kind)
  && Number(state.decision.player) === Number(state.perspective);
const stackKey = state => (state?.stack_objects || []).map(entry => String(entry.id)).join(',');
const priorityKey = state => [state?.turn_number, state?.active_player, state?.phase, state?.step, state?.combat_damage_step,
  state?.decision?.player, stackKey(state)].join('|');

const TURN_STOPS = [
  ['Upkeep'], ['Draw'], ['Main'],
  ...COMBAT_STEPS.map(step => ['Combat', step]),
  ['Main2'], ['End'], ['Cleanup'],
];

// Stops default to "I can act": they pause only when the player has something
// to do besides pass or make mana. "always" pauses unconditionally, and is the
// default for the two main phases.
export const CAN_ACT_STOP = 'can_act';
const DEFAULT_STOP_MODES = Object.freeze({ 'phase:Main': 'always', 'phase:Main2': 'always' });
const defaultStopMode = key => DEFAULT_STOP_MODES[key.replace(/^player:\d+:/, '')] ?? CAN_ACT_STOP;
export const PRIORITY_STOP_MODES = Object.freeze([CAN_ACT_STOP, 'always', 'off']);
export const PRIORITY_STOP_KEYS = Object.freeze([
  ...PHASE_TRACK.filter(phase => phase !== 'Untap').map(phase => `phase:${phase}`),
  ...COMBAT_STEPS.map(step => `step:${step}`),
]);
export const CHECKING_PLAYABLE_ACTIONS = 'checking playable actions';

// Unscoped preferences apply to every turn; a seat override can explicitly be off.
export const playerPriorityStopKey = (key, player) => `player:${player}:${key}`;
export function priorityStopMode(stops, key, player) {
  const mode = stops[playerPriorityStopKey(key, player)] ?? stops[key] ?? defaultStopMode(key);
  return mode === 'off' ? undefined : mode;
}
export function priorityStopsForPlayer(stops, player) {
  return Object.fromEntries(PRIORITY_STOP_KEYS.flatMap(key => {
    const mode = priorityStopMode(stops, key, player);
    return mode ? [[key, mode]] : [];
  }));
}

// An untapped creature without summoning sickness means the advance will stop
// at your own attack declaration. Like the engine's decision, this is a hint
// for the label only: an "always" stop or the declaration itself ends the advance.
export function hasPotentialAttackers(state) {
  if (state?.active_player == null || Number(state.active_player) !== Number(state.perspective)) return false;
  const player = (state.players || []).find(entry => Number(entry?.id) === Number(state.active_player));
  // Battlefield groups split by tapped state; card types are lowercase.
  return (player?.battlefield || []).some(card => (card?.card_types || [])
    .some(type => String(type).toLowerCase() === 'creature') && !card.summoning_sick && !card.tapped);
}

// "I can act" stops depend on the board when they arrive, so only "always"
// stops name the destination of an advance.
export function nextSelectedPriorityStop(state, stops = {}) {
  stops = Object.fromEntries(Object.entries(priorityStopsForPlayer(stops, state?.active_player))
    .filter(([, mode]) => mode === 'always'));
  if (hasPotentialAttackers(state)) stops['step:DeclareAttackers'] = 'always';
  const phase = normalizePhaseStep(state?.phase, state?.step);
  const step = priorityStepKey(state);
  const current = TURN_STOPS.findIndex(([candidatePhase, candidateStep]) =>
    candidatePhase === phase && (!candidateStep || candidateStep === step));
  return TURN_STOPS.slice(current + 1).find(([candidatePhase, candidateStep]) =>
    (candidatePhase !== phase && stops[`phase:${candidatePhase}`])
    || (candidateStep && stops[`step:${candidateStep}`])) || null;
}

export function selectedPriorityAdvanceLabel(state, stops, passing, t) {
  stops = Object.fromEntries(Object.entries(priorityStopsForPlayer(stops || {}, state?.active_player))
    .filter(([, mode]) => mode === 'always'));
  if (passing) return t('action.skippingPhase', {
    phase: currentPriorityPhaseLabel(state?.phase, priorityStepKey(state), t),
  });
  const target = nextSelectedPriorityStop(state, stops);
  if (hasPotentialAttackers(state)) stops['step:DeclareAttackers'] = 'always';
  if (target && !stops?.[`phase:${target[0]}`]
    && ['DeclareAttackers', 'DeclareBlockers'].includes(target[1])) {
    return t(`game.step.${target[1]}`);
  }
  const destination = target
    ? target[1] && !stops?.[`phase:${target[0]}`]
      ? t(`game.step.${target[1]}`)
      : t(`game.track.${target[0]}`)
    : t('game.advance.nextTurn');
  return t('action.goToPhase', { phase: destination });
}

// Saved choices override the default modes above. They are remembered per browser.
export const DEFAULT_PRIORITY_STOPS = Object.freeze({});
// Version 2 had no "I can act" mode; its main-phase "always" stops were defaults.
const LEGACY_DEFAULT_STOPS = new Set(['phase:Main', 'phase:Main2']);
export const PRIORITY_STOPS_STORAGE_KEY = 'ironsmith.priorityStops';

export function readStoredPriorityStops(storage = globalThis.localStorage) {
  try {
    const raw = storage?.getItem(PRIORITY_STOPS_STORAGE_KEY);
    if (raw == null) return { ...DEFAULT_PRIORITY_STOPS };
    const saved = JSON.parse(raw);
    const validKey = key => /^(player:\d+:)?(phase|step):\w+$/.test(key);
    if (saved?.version === 3 && saved.stops && typeof saved.stops === 'object') {
      return Object.fromEntries(Object.entries(saved.stops).filter(([key, mode]) =>
        validKey(key) && PRIORITY_STOP_MODES.includes(mode)));
    }
    // Older saves only listed "always" stops; everything else becomes "I can act".
    const legacy = saved?.version === 2 && saved.stops && typeof saved.stops === 'object'
      ? Object.entries(saved.stops).filter(([, mode]) => mode === 'always').map(([key]) => key)
      : (Array.isArray(saved) ? saved : []);
    return Object.fromEntries(legacy
      .filter(key => typeof key === 'string' && validKey(key) && !LEGACY_DEFAULT_STOPS.has(key))
      .map(key => [key, 'always']));
  } catch {
    return { ...DEFAULT_PRIORITY_STOPS };
  }
}

function storePriorityStops(storage, stops) {
  try {
    storage?.setItem(PRIORITY_STOPS_STORAGE_KEY,
      JSON.stringify({ version: 3, stops: Object.fromEntries(Object.entries(stops)
        .filter(([, mode]) => PRIORITY_STOP_MODES.includes(mode))) }));
  } catch { /* Stops still work for this session without storage. */ }
}

// Local preferences gate normal passes; they never change engine priority or
// submit another player's commands. All automation paths share this controller.
export function createPriorityStops({
  now = () => performance.now(),
  phaseStopsEnabled = false,
  resolveMode = "regular",
  storage = null,
  initialStops = storage ? readStoredPriorityStops(storage) : {},
} = {}) {
  const listeners = new Set();
  let stops = { ...initialStops };
  let window = null;
  let heldKey = null;
  let phaseVisit = '', stepVisit = '', phaseSerial = 0, stepSerial = 0;
  let consumed = new Set();
  // Declaring attackers is the decision a declare-attackers stop exists for;
  // the priority that follows in the same combat must not stop again.
  let attackersDeclaredIn = null;
  let sequence = 0;
  let snapshot = { stops, window };
  const publish = () => {
    snapshot = { stops: { ...stops }, window };
    listeners.forEach(listener => listener());
  };
  const observe = state => {
    if (!state) return;
    const phase = [state.turn_number, state.active_player, normalizePhaseStep(state.phase, state.step)].join('|');
    const step = [phase, priorityStepKey(state)].join('|');
    const combatRestart = priorityStepKey(state) === 'BeginCombat' && phase === phaseVisit
      && stepVisit && !stepVisit.endsWith('|BeginCombat');
    if (phase !== phaseVisit || combatRestart) { phaseVisit = phase; phaseSerial++; consumed = new Set(); }
    if (step !== stepVisit) { stepVisit = step; stepSerial++; }
    let changed = false;
    if (window && (!isLocalPriority(state) || window.key !== priorityKey(state) || state.game_over)) {
      window = null; changed = true;
    }
    if (heldKey && (!isLocalPriority(state) || heldKey !== priorityKey(state) || state.game_over)) heldKey = null;
    const timedStop = phaseStopsEnabled && resolveMode !== "all" && !state.game_over && isLocalPriority(state)
      && Number(state.active_player) !== Number(state.perspective)
      && Number(state.stack_size || 0) === 0 && selectedStop(state, state.decision?.can_act ?? false);
    // White keeps spells manual even when abilities are above them.
    const hasSpell = (state.stack_objects || []).some(entry => !entry.ability_kind);
    const timedResolve = (resolveMode === 'off' || (resolveMode === 'safe' && !hasSpell))
      && !state.game_over && isLocalPriority(state) && !state.mana_payment
      && Number(state.stack_size || 0) > 0;
    if (window?.kind === 'stack-resolve' && !timedResolve) {
      window = null; changed = true;
    }
    if (timedResolve && !window && !heldKey) {
      window = { id: ++sequence, kind: 'stack-resolve', key: priorityKey(state), startedAt: null, duration: PRIORITY_HOLD_WINDOW_MS };
      changed = true;
    }
    if (window?.kind === 'phase-stop' && !timedStop) {
      window = null; changed = true;
    }
    if (timedStop && !window && !heldKey) {
      window = { id: ++sequence, kind: 'phase-stop', key: priorityKey(state), startedAt: null, duration: PRIORITY_HOLD_WINDOW_MS };
      changed = true;
    }
    if (changed) publish();
  };
  const candidates = state => [
    [`phase:${normalizePhaseStep(state?.phase, state?.step)}`, `phase:${phaseSerial}`],
    [`step:${priorityStepKey(state)}`, `step:${stepSerial}`],
  ];
  const stopKey = (key, state) => Object.hasOwn(stops, playerPriorityStopKey(key, state?.active_player))
    ? playerPriorityStopKey(key, state.active_player) : key;
  const modeOf = key => stops[key] ?? defaultStopMode(key);
  // canAct: true/false once known, null while the payment analysis is running.
  // An unknown answer holds rather than risk skipping a response window. Other
  // decisions (attackers, blockers) are required anyway, so only "always" holds them.
  const selectedStop = (state, canAct) => candidates(state).map(([key, visit]) => [stopKey(key, state), visit, key])
    .find(([key, visit, base]) => modeOf(key) !== 'off' && !consumed.has(`${key}|${visit}`)
      && !(base === 'step:DeclareAttackers' && attackersDeclaredIn === phaseSerial)
      && (modeOf(key) === 'always' || (state?.decision?.kind === 'priority' && canAct !== false)));
  const stopReason = (state, canAct = state?.decision?.can_act ?? null) => {
    observe(state);
    if (!isLocalTurnDecision(state)) return null;
    if (heldKey) return 'priority held';
    if (window?.kind === 'stack-resolve') return 'timed stack resolve';
    const stop = selectedStop(state, canAct);
    if (!stop) return null;
    return modeOf(stop[0]) !== 'always' && state?.decision?.kind === 'priority' && canAct == null
      ? CHECKING_PLAYABLE_ACTIONS : `stop at ${stop[0].split(':').at(-1)}`;
  };
  // Only an "I can act" stop needs the answer; "always" and no stop do not.
  const awaitsCanAct = (state, canAct = state?.decision?.can_act ?? null) =>
    canAct == null && stopReason(state, null) === CHECKING_PLAYABLE_ACTIONS;
  const releaseStops = state => {
    observe(state);
    if (state?.decision?.kind === 'attackers') attackersDeclaredIn = phaseSerial;
    for (const [base, visit] of candidates(state)) {
      const key = stopKey(base, state);
      if (modeOf(key) === 'off') continue;
      consumed.add(`${key}|${visit}`);
    }
  };
  return {
    setResolveMode(mode) {
      resolveMode = mode;
      if (window?.kind === "stack-resolve") { window = null; publish(); }
    },
    setPhaseStopsEnabled(enabled) {
      phaseStopsEnabled = Boolean(enabled);
      if (!phaseStopsEnabled) {
        if (window) { window = null; publish(); }
      }
    },
    subscribe(listener) { listeners.add(listener); return () => listeners.delete(listener); },
    getSnapshot: () => snapshot,
    observe,
    windowFor: state => Boolean(window && isLocalPriority(state) && window.key === priorityKey(state)),
    stopReason,
    awaitsCanAct,
    cycleStop(key, state, player) {
      observe(state);
      stops = { ...stops };
      const mode = player == null ? modeOf(key) : (priorityStopMode(stops, key, player) ?? 'off');
      const next = { [CAN_ACT_STOP]: 'always', always: 'off', off: CAN_ACT_STOP }[mode] ?? CAN_ACT_STOP;
      const inherited = player == null ? defaultStopMode(key) : modeOf(key);
      const base = key;
      if (player != null) key = playerPriorityStopKey(key, player);
      if (next === inherited) delete stops[key];
      else stops[key] = next;
      if (storage) storePriorityStops(storage, stops);
      // Setting a stop on the current step should take effect immediately.
      for (const [candidate, visit] of candidates(state)) {
        if (candidate === base) consumed.delete(`${key}|${visit}`);
      }
      observe(state);
      publish();
    },
    beforeCommand(command, state) {
      observe(state);
      const checkpoint = { stops: { ...stops }, consumed: new Set(consumed), attackersDeclaredIn };
      if (isLocalTurnDecision(state)) {
        releaseStops(state);
        heldKey = null;
        if (window) { window = null; publish(); }
      }
      return checkpoint;
    },
    hold(state) {
      observe(state);
      if (!window || !isLocalPriority(state)) return false;
      heldKey = window.key;
      window = null;
      publish();
      return true;
    },
    holdCurrentPriority(state) {
      observe(state);
      window = null;
      heldKey = isLocalPriority(state) ? priorityKey(state) : null;
      publish();
    },
    resume(state) {
      observe(state);
      heldKey = null;
      if (window) { window = null; publish(); }
    },
    deferWindow(id) {
      if (!window || window.id !== id || window.startedAt === null) return;
      window = { ...window, startedAt: null };
      publish();
    },
    armWindow(id) {
      if (!window || window.id !== id || window.startedAt !== null || window.pausedAt != null) return;
      window = { ...window, startedAt: now() };
      publish();
    },
    pauseWindow(id) {
      if (!window || window.id !== id || window.pausedAt != null) return;
      window = { ...window, pausedAt: now() };
      publish();
    },
    resumeWindow(id) {
      if (!window || window.id !== id || window.pausedAt == null) return;
      window = { ...window,
        startedAt: window.startedAt === null ? null : window.startedAt + now() - window.pausedAt,
        pausedAt: null,
      };
      publish();
    },
    expire(id, state) {
      observe(state);
      if (!window || window.id !== id || !isLocalPriority(state)
        || window.pausedAt != null || window.startedAt === null || now() < window.startedAt + window.duration) return false;
      if (window.kind === 'stack-resolve' && state.viewed_cards
        && !state.viewed_cards.inspector_only && !state.viewed_cards.inspectorOnly) return false;
      heldKey = window.key;
      window = null;
      publish();
      return true;
    },
    rollbackAction(checkpoint, state) {
      if (checkpoint) { stops = checkpoint.stops; consumed = checkpoint.consumed; attackersDeclaredIn = checkpoint.attackersDeclaredIn; }
      window = null;
      heldKey = isLocalPriority(state) ? priorityKey(state) : null;
      publish();
    },
    cancelAction() { if (window) { window = null; publish(); } },
    reset() {
      window = null; heldKey = null; phaseVisit = ''; stepVisit = ''; consumed = new Set();
      attackersDeclaredIn = null; publish();
    },
  };
}
