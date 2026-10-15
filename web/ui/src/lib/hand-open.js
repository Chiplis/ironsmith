import { useLayoutEffect, useSyncExternalStore } from "react";

export const TOGGLE_HAND_EVENT = "ironsmith:toggle-hand";

// Whether the player has raised their hand with the keyboard. Hover and focus
// still lift single cards; this lifts the whole hand until toggled again.
let open = false;
const listeners = new Set();

function subscribe(listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function setHandOpen(value) {
  const next = Boolean(value);
  if (next === open) return;
  open = next;
  listeners.forEach(listener => listener());
}

export function toggleHandOpen() {
  setHandOpen(!open);
  window.dispatchEvent(new CustomEvent(TOGGLE_HAND_EVENT, { detail: { open } }));
}

// Called with the new value whenever the hand opens or closes.
export function onHandOpenChange(listener) {
  return subscribe(() => listener(open));
}

export function useHandOpen() {
  return useSyncExternalStore(subscribe, () => open, () => false);
}

function visible(element) {
  const style = element.ownerDocument.defaultView.getComputedStyle(element);
  return element.getClientRects().length > 0 && style.visibility !== "hidden";
}

// "." opens or closes the hand, unless the player is typing or a dialog is up.
export function installHandToggleShortcut(document) {
  const onKeyDown = event => {
    if (event.key !== "." || event.defaultPrevented || event.isComposing || event.repeat
        || event.ctrlKey || event.altKey || event.metaKey) return;
    if (event.target?.closest?.('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="combobox"]')) return;
    if ([...document.querySelectorAll('[role="dialog"], [role="alertdialog"], dialog[open]')].some(visible)) return;
    event.preventDefault();
    toggleHandOpen();
  };
  document.addEventListener("keydown", onKeyDown);
  return () => document.removeEventListener("keydown", onKeyDown);
}

const RAISED_MARGIN_PX = 12;
const DEFAULT_REVEAL_Y_PX = -8;

// Lifts a docked hand shell so its tallest resting card clears the bottom of
// the viewport while the hand is open. The shell sits at the bottom of its
// lane, so the lift is measured from the lane rather than the moving shell.
export function useHandShellLift(shellRef, open) {
  useLayoutEffect(() => {
    const shell = shellRef.current;
    if (!shell) return undefined;
    if (!open) {
      shell.style.removeProperty("--hand-reveal-y");
      return undefined;
    }
    const measure = () => {
      const lane = shell.parentElement;
      if (!lane) return;
      const shellBottom = shell.getBoundingClientRect().bottom;
      const resting = [...shell.querySelectorAll(".game-card.hand-card")]
        .filter(card => !card.matches(".hovered, .inspected, .keyboard-selected"));
      if (resting.length === 0) return;
      const overhang = Math.max(...resting.map(card => card.getBoundingClientRect().bottom)) - shellBottom;
      const viewportHeight = shell.ownerDocument.defaultView.innerHeight;
      const y = viewportHeight - RAISED_MARGIN_PX - lane.getBoundingClientRect().bottom - overhang;
      shell.style.setProperty("--hand-reveal-y", `${Math.round(Math.min(DEFAULT_REVEAL_Y_PX, y))}px`);
    };
    measure();
    const view = shell.ownerDocument.defaultView;
    const observer = typeof ResizeObserver === "function" ? new ResizeObserver(measure) : null;
    observer?.observe(shell);
    view.addEventListener("resize", measure);
    return () => {
      observer?.disconnect();
      view.removeEventListener("resize", measure);
    };
  }, [shellRef, open]);
}

// The hand card an arrow key should move to when focus is not already on a
// card in this hand, or null when the key belongs to something else.
export function handKeyboardEntryCard(event, handList) {
  if ((event.key !== "ArrowRight" && event.key !== "ArrowLeft") || event.defaultPrevented
      || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return null;
  const doc = handList.ownerDocument;
  const active = doc.activeElement;
  if (active && active !== doc.body && handList.contains(active)) return null;
  // Other focused controls (inputs, menus, a focused battlefield card) keep
  // their own arrow-key behaviour.
  if (active && active !== doc.body && active.closest?.('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="combobox"], [role="listbox"], [role="menu"], [role="slider"], .game-card[role="button"]')) return null;
  if ([...doc.querySelectorAll('[role="dialog"], [role="alertdialog"], dialog[open]')].some(visible)) return null;
  const cards = [...handList.querySelectorAll('.game-card.hand-card[role="button"]')]
    .filter(card => card.getClientRects().length > 0 && !card.hasAttribute("aria-hidden"));
  if (cards.length === 0) return null;
  return event.key === "ArrowRight" ? cards[0] : cards[cards.length - 1];
}
