import { readShortcutPreset } from "./keyboard-preferences.js";
import { useLayoutEffect, useSyncExternalStore } from "react";

export const TOGGLE_HAND_EVENT = "ironsmith:toggle-hand";

// Whether the player has raised their hand with the keyboard. Hover and focus
// still lift single cards; this lifts the whole hand until toggled again.
let open = false;
let selectedHandCardId = null;
const listeners = new Set();

function subscribe(listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function setHandOpen(value) {
  const next = Boolean(value);
  if (next === open) return;
  open = next;
  if (!open) selectedHandCardId = null;
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

export function isHandOpen() { return open; }

export function useHandOpen() {
  return useSyncExternalStore(subscribe, () => open, () => false);
}

export function setSelectedHandCardId(value) {
  const next = value == null ? null : String(value);
  if (next === selectedHandCardId) return;
  selectedHandCardId = next;
  listeners.forEach(listener => listener());
}

export function useSelectedHandCardId() {
  return useSyncExternalStore(subscribe, () => selectedHandCardId, () => null);
}

export function useHandNumbersVisible() {
  return useHandOpen();
}

function visible(element) {
  const style = element.ownerDocument.defaultView.getComputedStyle(element);
  return element.getClientRects().length > 0 && style.visibility !== "hidden";
}

// Both shortcuts toggle the same numbered hand view.
export function installHandToggleShortcut(document) {
  const onKeyDown = event => {
    if ((event.key.toLowerCase() !== "h" && event.key !== ".") || event.defaultPrevented || event.isComposing || event.repeat
        || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return;
    if (event.target?.closest?.('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="combobox"]')) return;
    if ([...document.querySelectorAll('[role="dialog"], [role="alertdialog"], dialog[open]')].some(visible)) return;
    event.preventDefault();
    toggleHandOpen();
  };
  document.addEventListener("keydown", onKeyDown);
  return () => document.removeEventListener("keydown", onKeyDown);
}

const RAISED_MARGIN_PX = 12;

// Show each valid prefix immediately, then allow another digit to extend it.
// No delayed focus callback: typing elsewhere must never select a card later.
export function installHandNumberShortcut(document, getCards, selectCard) {
  let digits = "";
  let lastTime = -Infinity;
  const reset = () => { digits = ""; lastTime = -Infinity; };
  const onKeyDown = event => {
    if (event.defaultPrevented || event.isComposing || event.repeat
      || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey
      || !/^[0-9]$/.test(event.key)
      || event.target?.closest?.('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="combobox"]')
      || [...document.querySelectorAll('[role="dialog"], [role="alertdialog"], dialog[open]')].some(visible)) {
      reset();
      return;
    }
    if (readShortcutPreset() === "mtgo" && !open) { reset(); return; }
    const cards = getCards();
    const now = event.timeStamp;
    let next = cards.length > 9 && now - lastTime <= 700 ? digits + event.key : event.key;
    if (Number(next) > cards.length) next = event.key;
    const card = cards[Number(next) - 1];
    if (!card) { reset(); return; }
    digits = next;
    lastTime = now;
    event.preventDefault();
    selectCard(card);
  };
  document.addEventListener("keydown", onKeyDown);
  document.addEventListener("pointerdown", reset);
  document.defaultView.addEventListener("blur", reset);
  return () => {
    document.removeEventListener("keydown", onKeyDown);
    document.removeEventListener("pointerdown", reset);
    document.defaultView.removeEventListener("blur", reset);
  };
}

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

// While the hand is open it owns card navigation, including when a field
// card still has focus. Text inputs, menus and modal dialogs keep their keys.
export function handNavigationKeyAllowed(event, doc) {
  if (!["ArrowRight", "ArrowLeft", "ArrowUp", "ArrowDown"].includes(event.key) || event.defaultPrevented
      || event.isComposing || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey) return false;
  if (event.target?.closest?.('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="combobox"], [role="listbox"], [role="menu"], [role="slider"]')) return false;
  return ![...doc.querySelectorAll('[role="dialog"], [role="alertdialog"], dialog[open]')].some(visible);
}

export function handKeyboardEntryCard(event, handList) {
  const doc = handList.ownerDocument;
  if (!handNavigationKeyAllowed(event, doc)) return null;
  const cards = [...handList.querySelectorAll('.game-card.hand-card[role="button"]')]
    .filter(card => visible(card) && !card.hasAttribute("aria-hidden"));
  if (!cards.length) return null;
  const index = cards.indexOf(doc.activeElement?.closest?.('.game-card.hand-card'));
  const forward = event.key === "ArrowRight" || event.key === "ArrowDown";
  const next = index < 0 ? (forward ? 0 : cards.length - 1)
    : (index + (forward ? 1 : -1) + cards.length) % cards.length;
  return cards[next];
}
