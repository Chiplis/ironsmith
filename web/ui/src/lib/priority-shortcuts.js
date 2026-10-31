import { resolvableTriggerGroup } from './resolve-group.js';
import { booleanChoice } from './priority-policy.js';
import { isHandOpen } from './hand-open.js';
import { readShortcutPreset } from './keyboard-preferences.js';
export { readShortcutPreset, SHORTCUT_PRESET_KEY } from './keyboard-preferences.js';
export function installPriorityShortcuts(document, actions) {
  const typing = target => target?.closest?.('input:not([type="checkbox"]):not([type="radio"]), textarea, select, [contenteditable="true"], [role="textbox"], [role="combobox"]');
  const onKey = event => {
    if (event.key === 'Control') { if (!typing(event.target)) actions.maintain(event.type === 'keydown'); return; }
    if (event.type !== 'keydown' || event.repeat || event.defaultPrevented || event.isComposing
      || event.ctrlKey || event.metaKey || typing(event.target)) return;
    const mtgo = readShortcutPreset() === 'mtgo';
    // The raised hand owns plain digits, including positions without a card.
    if (mtgo && isHandOpen() && !event.altKey) return;
    // Classic retains hand numbers; Alt+number offers the same commands.
    if (mtgo ? event.altKey : !event.altKey) return;
    if (event.shiftKey || !'12345678'.includes(event.key) || !actions.state()?.decision) return;
    if (event.key !== '5' && [...document.querySelectorAll('[role="dialog"], [role="alertdialog"], dialog[open]')]
      .some(node => node.getClientRects().length)) return;
    let handled = true;
    switch (event.key) {
      case '1': {
        const buttons = [...document.querySelectorAll('button.decision-main-button:not([data-secondary-priority-action])')]
          .filter(button => !button.disabled && button.getClientRects().length && !button.closest('[inert], [aria-hidden="true"]'));
        if (buttons.length === 1) buttons[0].click(); else handled = false;
        break;
      }
      case '2': actions.start('responses'); break;
      case '3': case '4': {
        const state = actions.state();
        const command = Number(state?.decision?.player) === Number(state?.perspective)
          ? booleanChoice(state?.decision, event.key === '3') : null;
        if (command) actions.answer(command); else handled = false;
        break;
      }
      case '5': actions.clear(); break;
      case '6': actions.start('turn'); break;
      case '7': handled = actions.group() !== false; break;
      case '8': actions.start('no_play'); break;
    }
    if (handled) { event.preventDefault(); event.stopPropagation(); }
  };
  const blur = () => actions.maintain(false);
  document.addEventListener('keydown', onKey, true);
  document.addEventListener('keyup', onKey, true);
  document.defaultView.addEventListener('blur', blur);
  return () => {
    document.removeEventListener('keydown', onKey, true);
    document.removeEventListener('keyup', onKey, true);
    document.defaultView.removeEventListener('blur', blur);
  };
}

// Key 7 has one action for each stage: ordering first, resolution afterward.
export function handleTriggerShortcut(policy, state) {
  if (!state?.decision || Number(state.decision.player) !== Number(state.perspective)) return false;
  if (state.decision.choice_kind === 'trigger_order') {
    policy.setGroupTriggers(!policy.status(state).groupTriggers, state);
    return true;
  }
  if (resolvableTriggerGroup(state)) {
    policy.startGroup(state);
    return true;
  }
  return false;
}
