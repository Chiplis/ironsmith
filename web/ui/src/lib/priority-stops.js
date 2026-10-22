import { normalizePhaseStep, normalizeStepKey, currentPriorityPhaseLabel } from './constants.js';

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

export function nextSelectedPriorityStop(state, stops = {}) {
  const phase = normalizePhaseStep(state?.phase, state?.step);
  const step = priorityStepKey(state);
  const current = TURN_STOPS.findIndex(([candidatePhase, candidateStep]) =>
    candidatePhase === phase && (!candidateStep || candidateStep === step));
  return TURN_STOPS.slice(current + 1).find(([candidatePhase, candidateStep]) =>
    (candidatePhase !== phase && stops[`phase:${candidatePhase}`])
    || (candidateStep && stops[`step:${candidateStep}`])) || null;
}

export function selectedPriorityAdvanceLabel(state, stops, passing, t) {
  if (passing) return t('action.skippingPhase', {
    phase: currentPriorityPhaseLabel(state?.phase, priorityStepKey(state), t),
  });
  const target = nextSelectedPriorityStop(state, stops);
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

// Out of the box a pass stops where most turns need a decision (like Arena):
// both main phases and the declare-attackers step. "Every time" stops are
// remembered per browser; "once" stops only live for the current game.
export const DEFAULT_PRIORITY_STOPS = Object.freeze({
  'phase:Main': 'always',
  'step:DeclareAttackers': 'always',
  'phase:Main2': 'always',
});
export const PRIORITY_STOPS_STORAGE_KEY = 'ironsmith.priorityStops';

export function readStoredPriorityStops(storage = globalThis.localStorage) {
  try {
    const raw = storage?.getItem(PRIORITY_STOPS_STORAGE_KEY);
    if (raw == null) return { ...DEFAULT_PRIORITY_STOPS };
    const saved = JSON.parse(raw);
    return Object.fromEntries((Array.isArray(saved) ? saved : [])
      .filter(key => typeof key === 'string' && /^(phase|step):\w+$/.test(key))
      .map(key => [key, 'always']));
  } catch {
    return { ...DEFAULT_PRIORITY_STOPS };
  }
}

function storePriorityStops(storage, stops) {
  try {
    storage?.setItem(PRIORITY_STOPS_STORAGE_KEY,
      JSON.stringify(Object.keys(stops).filter(key => stops[key] === 'always')));
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
    const timedStop = phaseStopsEnabled && !state.game_over && isLocalPriority(state)
      && Number(state.active_player) !== Number(state.perspective)
      && Number(state.stack_size || 0) === 0 && selectedStop(state);
    const timedResolve = phaseStopsEnabled && ['safe', 'all'].includes(resolveMode)
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
  const selectedStop = state => candidates(state).find(([key, visit]) => stops[key] && !consumed.has(`${key}|${visit}`)
    && !(key === 'step:DeclareAttackers' && attackersDeclaredIn === phaseSerial));
  const stopReason = state => {
    observe(state);
    if (!isLocalTurnDecision(state)) return null;
    if (heldKey) return 'priority held';
    if (window?.kind === 'stack-resolve') return 'timed stack resolve';
    const stop = selectedStop(state);
    return stop ? `stop at ${stop[0].split(':')[1]}` : null;
  };
  const releaseStops = state => {
    observe(state);
    if (state?.decision?.kind === 'attackers') attackersDeclaredIn = phaseSerial;
    let changed = false;
    for (const [key, visit] of candidates(state)) {
      if (!stops[key]) continue;
      consumed.add(`${key}|${visit}`);
      if (stops[key] === 'once') { delete stops[key]; changed = true; }
    }
    if (changed) publish();
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
    cycleStop(key, state) {
      observe(state);
      stops = { ...stops };
      if (!stops[key]) stops[key] = 'once';
      else if (stops[key] === 'once') stops[key] = 'always';
      else delete stops[key];
      if (storage) storePriorityStops(storage, stops);
      // Setting a stop on the current step should take effect immediately.
      for (const [candidate, visit] of candidates(state)) {
        if (candidate === key) consumed.delete(`${candidate}|${visit}`);
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
      if (window.kind === 'stack-resolve') {
        if (state.viewed_cards && !state.viewed_cards.inspector_only && !state.viewed_cards.inspectorOnly) return false;
        if (resolveMode === 'safe') {
          if (state.decision.analysis_complete === false) return false;
          const playable = (state.decision.actions || []).some(action =>
            !['pass_priority', 'activate_mana_ability', 'untap_land'].includes(action.kind)
            && action.payment_proven !== false);
          if (playable) {
            heldKey = window.key;
            window = null;
            publish();
            return false;
          }
        }
      }
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
