import { isPregameDecision } from './opening-hand-actions.js';

export const HAND_ACTIVATE_SELECTED_EVENT = 'ironsmith:activate-selected-hand-card';
const playKinds = new Set(['cast_spell', 'play_land', 'special_action']);
const activateKinds = new Set(['activate_ability', 'activate_mana_ability']);

export function handCardPriorityActions(decision, objectId) {
  if (decision?.kind !== 'priority' || objectId == null) return [];
  return (decision.actions || []).filter(action => String(action.object_id) === String(objectId)
    && (playKinds.has(action.kind) || activateKinds.has(action.kind))
    && (decision.analysis_complete !== true || action.payment_proven !== false));
}

export function handPriorityAction(state, open, selectedId) {
  const decision = state?.decision;
  const active = open && decision?.kind === 'priority' && !isPregameDecision(decision)
    && Number(decision.player) === Number(state.perspective);
  const player = state?.players?.find(player => Number(player.id) === Number(state.perspective));
  const card = player?.hand_cards?.find(card => String(card.id) === String(selectedId));
  const actions = active && card ? handCardPriorityActions(decision, card.id) : [];
  const play = actions.some(action => playKinds.has(action.kind));
  const activate = actions.some(action => activateKinds.has(action.kind));
  return { active, card, actions, label: play && activate ? 'Play/Activate' : play ? 'Play' : activate ? 'Activate'
    : !card ? 'Select a hand card' : decision?.analysis_complete === false ? 'Checking actions…' : 'No available action' };
}

export function activateSelectedHandCard(objectId) {
  window.dispatchEvent(new CustomEvent(HAND_ACTIVATE_SELECTED_EVENT, { detail: { objectId } }));
}
