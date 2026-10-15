import { normalizePhaseStep, normalizeStepKey } from './constants.js';

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
  postActionEnabled = false,
  storage = null,
  initialStops = storage ? readStoredPriorityStops(storage) : {},
} = {}) {
  const listeners = new Set();
  let stops = { ...initialStops };
  let window = null;
  let pending = null;
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
    if (pending && state.game_over) pending = null;
    if (pending && state.decision?.kind === 'priority' && priorityKey(state) !== pending.key) {
      const ownNewStackEntry = (state.stack_objects || []).some(entry =>
        Number(entry.controller) === Number(pending.player) && !pending.ids.has(String(entry.id)));
      if (ownNewStackEntry && isLocalPriority(state)) {
        window = { id: ++sequence, key: priorityKey(state), startedAt: null, duration: PRIORITY_HOLD_WINDOW_MS };
        changed = true;
      }
      pending = null;
    }
    if (changed) publish();
  };
  const candidates = state => [
    [`phase:${normalizePhaseStep(state?.phase, state?.step)}`, `phase:${phaseSerial}`],
    [`step:${priorityStepKey(state)}`, `step:${stepSerial}`],
  ];
  const stopReason = state => {
    observe(state);
    if (!isLocalTurnDecision(state)) return null;
    if (window || pending) return 'post-action priority window';
    if (heldKey) return 'priority held after action';
    const stop = candidates(state).find(([key, visit]) => stops[key] && !consumed.has(`${key}|${visit}`)
      && !(key === 'step:DeclareAttackers' && attackersDeclaredIn === phaseSerial));
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
    setPostActionEnabled(enabled) {
      postActionEnabled = Boolean(enabled);
      if (!postActionEnabled) {
        pending = null;
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
      publish();
    },
    beforeCommand(command, state) {
      observe(state);
      const checkpoint = { stops: { ...stops }, consumed: new Set(consumed), attackersDeclaredIn };
      const action = command?.type === 'priority_action'
        ? (state?.decision?.actions || []).find(action => command.action_ref
          ? JSON.stringify(action.action_ref) === JSON.stringify(command.action_ref)
          : action.index === command.action_index)
        : null;
      const kind = action?.kind || command?.action_ref?.kind;
      if (isLocalTurnDecision(state)) {
        releaseStops(state);
        heldKey = null;
        if (window) { window = null; publish(); }
      }
      if (postActionEnabled && (kind === 'cast_spell' || kind === 'activate_ability')) {
        pending = { key: priorityKey(state), player: state.perspective, ids: new Set((state.stack_objects || []).map(entry => String(entry.id))) };
      }
      if (command?.type === 'cancel_decision') pending = null;
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
    resume(state) {
      observe(state);
      heldKey = null;
      if (window) { window = null; publish(); }
    },
    armWindow(id) {
      if (!window || window.id !== id || window.startedAt !== null) return;
      window = { ...window, startedAt: now() };
      publish();
    },
    expire(id, state) {
      observe(state);
      if (!window || window.id !== id || !isLocalPriority(state)
        || window.startedAt === null || now() < window.startedAt + window.duration) return false;
      const newStop = candidates(state).some(([key, visit]) => stops[key] && !consumed.has(`${key}|${visit}`));
      heldKey = newStop ? null : window.key;
      window = null;
      publish();
      return !newStop;
    },
    rollbackAction(checkpoint, state) {
      if (checkpoint) { stops = checkpoint.stops; consumed = checkpoint.consumed; attackersDeclaredIn = checkpoint.attackersDeclaredIn; }
      pending = null; window = null;
      heldKey = isLocalPriority(state) ? priorityKey(state) : null;
      publish();
    },
    cancelAction() { pending = null; if (window) { window = null; publish(); } },
    reset() {
      pending = null; window = null; heldKey = null; phaseVisit = ''; stepVisit = ''; consumed = new Set();
      attackersDeclaredIn = null; publish();
    },
  };
}
