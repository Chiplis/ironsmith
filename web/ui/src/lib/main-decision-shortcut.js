function visible(element) {
  const style = element.ownerDocument.defaultView.getComputedStyle(element);
  return element.getClientRects().length > 0
    && style.visibility !== 'hidden' && style.visibility !== 'collapse';
}

export function installMainDecisionShortcut(document) {
  const onKeyDown = event => {
    if (event.key !== 'Enter' || event.defaultPrevented || event.isComposing
        || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return;
    const target = event.target;
    // Let editing, focused controls, and modal dialogs keep native Enter behavior.
    if (target?.closest?.('input, textarea, select, button, a, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="combobox"]')) return;
    if ([...document.querySelectorAll('[role="dialog"], [role="alertdialog"], dialog[open]')].some(visible)) return;
    const buttons = [...document.querySelectorAll('button.decision-main-button')].filter(button =>
      !button.disabled && button.getAttribute('aria-disabled') !== 'true'
      && !button.closest('[inert], [aria-hidden="true"]') && visible(button));
    // Do not guess if a transition briefly exposes more than one primary action.
    if (buttons.length !== 1) return;
    event.preventDefault();
    if (!event.repeat) buttons[0].click();
  };
  document.addEventListener('keydown', onKeyDown);
  return () => document.removeEventListener('keydown', onKeyDown);
}
