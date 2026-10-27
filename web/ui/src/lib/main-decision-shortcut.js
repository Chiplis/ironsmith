function visible(element) {
  const style = element.ownerDocument.defaultView.getComputedStyle(element);
  return element.getClientRects().length > 0
    && style.visibility !== 'hidden' && style.visibility !== 'collapse';
}

export function installMainDecisionShortcut(document) {
  const onKeyDown = event => {
    const cancel = event.key === 'Escape';
    // Space advances the visible decision even after clicking a card or option.
    // Enter retains native activation of the focused control.
    if ((!cancel && event.key !== 'Enter' && event.key !== ' ') || event.defaultPrevented || event.isComposing
        || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return;
    const target = event.target;
    // Let editing, focused controls, and modal dialogs keep native Enter/Space behavior.
    if (!cancel && target?.closest?.('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="combobox"]')) return;
    if (event.key === 'Enter' && target?.closest?.('button, a')) return;
    if ([...document.querySelectorAll('[role="dialog"], [role="alertdialog"], dialog[open]')].some(visible)) return;
    // Use the same cancellation path as the visible control, including payment
    // cleanup. Escape also works while a casting choice or its input has focus.
    const selector = cancel ? 'button.decision-cancel-button' : 'button.decision-main-button';
    const buttons = [...document.querySelectorAll(selector)].filter(button =>
      !button.disabled && button.getAttribute('aria-disabled') !== 'true'
      && !button.closest('[inert], [aria-hidden="true"]') && visible(button));
    // Do not guess if a transition briefly exposes more than one primary action.
    if (buttons.length !== 1) return;
    event.preventDefault();
    if (!event.repeat) buttons[0].click();
  };
  const onSpace = event => {
    if (event.key !== ' ') return;
    const prevented = event.defaultPrevented;
    onKeyDown(event);
    if (!prevented && event.defaultPrevented) event.stopPropagation();
  };
  const onOtherKey = event => { if (event.key !== ' ') onKeyDown(event); };
  document.addEventListener('keydown', onSpace, true);
  document.addEventListener('keydown', onOtherKey);
  return () => {
    document.removeEventListener('keydown', onSpace, true);
    document.removeEventListener('keydown', onOtherKey);
  };
}
