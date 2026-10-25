import { useCallback, useEffect, useState } from "react";
import { X } from "lucide-react";
import { useGame } from "@/context/GameContext";
import useUiText from "@/i18n/useUiText";
import useModalFocus from "@/hooks/useModalFocus";
import { Checkbox } from "@/components/ui/checkbox";
import { OPEN_SHORTCUTS_EVENT, useShowShortcutsOnStart } from "@/lib/shortcuts-overlay";
import "./keyboard-shortcuts.css";

const SHORTCUTS = [
  { keys: ["Space", "Enter"], action: "Pass priority, or confirm the highlighted main action" },
  { keys: ["←", "→"], action: "Move between cards in your hand. With the hand open, → starts at the first card and ← at the last" },
  { keys: ["Space", "Enter"], scope: "on a hand card", action: "Play or cast that card" },
  { keys: ["."], action: "Open or close your hand" },
  { keys: ["Esc"], action: "Cancel placing a card, or close a menu" },
  { keys: ["Tab"], action: "Move focus between controls" },
  { keys: ["Double-click"], scope: "on a hand card", action: "Play or cast that card, keeping any required choices manual" },
  { keys: ["?"], action: "Show this list" },
];

const TIPS = [
  "Click a phase on the turn track to add a stop: once, every time, or off. Each player has a separate track. By default a pass stops at both main phases on every player’s turn.",
  "In local games the table follows the active player, so you can play both sides. Turn this off in Settings.",
];

function isTypingTarget(target) {
  return Boolean(target?.closest?.('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="combobox"]'));
}

// Pointer-and-keyboard devices only: a touch tablet has no use for the list.
function hasKeyboardPointer() {
  try { return window.matchMedia("(hover: hover) and (pointer: fine)").matches; }
  catch { return true; }
}

export default function KeyboardShortcutsOverlay() {
  const { state } = useGame();
  const ui = useUiText();
  const [showOnStart, setShowOnStart] = useShowShortcutsOnStart();
  const [open, setOpen] = useState(false);
  // Lobby links skip the automatic offer, even if joining later changes the URL.
  const [startChecked, setStartChecked] = useState(() => (
    typeof window !== "undefined"
    && new URLSearchParams(window.location.search).has("lobby")
  ));
  const close = useCallback(() => setOpen(false), []);
  const rootRef = useModalFocus(close, open);
  const gameReady = Boolean(state?.players?.length);

  // Offer the list once per page load, when the first table is ready.
  if (gameReady && !startChecked) {
    setStartChecked(true);
    if (showOnStart && hasKeyboardPointer()) setOpen(true);
  }

  useEffect(() => {
    const openOverlay = () => setOpen(true);
    const onKeyDown = (event) => {
      if (event.key !== "?" || event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey
          || isTypingTarget(event.target)) return;
      event.preventDefault();
      setOpen(current => !current);
    };
    window.addEventListener(OPEN_SHORTCUTS_EVENT, openOverlay);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener(OPEN_SHORTCUTS_EVENT, openOverlay);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  if (!open) return null;
  return (
    <div className="keyboard-shortcuts-backdrop" onPointerDown={(event) => {
      if (event.target === event.currentTarget) close();
    }}>
      <div ref={rootRef} className="keyboard-shortcuts-panel" role="dialog" aria-modal="true"
        aria-labelledby="keyboard-shortcuts-title" tabIndex={-1}>
        <header className="keyboard-shortcuts-header">
          <h2 id="keyboard-shortcuts-title">{ui("Keyboard shortcuts")}</h2>
          <button type="button" className="keyboard-shortcuts-close" aria-label={ui("Close keyboard shortcuts")}
            onClick={close}>
            <X size={16} aria-hidden="true" />
          </button>
        </header>
        <dl className="keyboard-shortcuts-list">
          {SHORTCUTS.map(({ keys, scope, action }) => (
            <div key={`${keys.join("+")}-${action}`} className="keyboard-shortcuts-row">
              <dt>
                {keys.map((key, index) => (
                  <span key={key}>
                    {index > 0 && <span className="keyboard-shortcuts-or">{ui("or")}</span>}
                    <kbd>{ui(key)}</kbd>
                  </span>
                ))}
                {scope && <span className="keyboard-shortcuts-scope">{ui(scope)}</span>}
              </dt>
              <dd>{ui(action)}</dd>
            </div>
          ))}
        </dl>
        <ul className="keyboard-shortcuts-tips">
          {TIPS.map(tip => <li key={tip}>{ui(tip)}</li>)}
        </ul>
        <footer className="keyboard-shortcuts-footer">
          <label>
            <Checkbox checked={!showOnStart} onCheckedChange={(value) => setShowOnStart(!value)} />
            {ui("Don't show this when a game starts")}
          </label>
          <span className="keyboard-shortcuts-hint">{ui("You can change this in Settings, or press ? any time.")}</span>
        </footer>
      </div>
    </div>
  );
}
