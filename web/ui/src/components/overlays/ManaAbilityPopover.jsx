import { useEffect } from "react";
import { useSuppressCardPreview } from "@/context/HoverContext";
import ActionPopover from "./ActionPopover";

// Use the same speech-bubble menu as drag-to-cast alternative casting methods.
// Mana choices deliberately do not drive the card inspector's hover state.
export default function ManaAbilityPopover({ anchor, actions, disabled, focusOnOpen = false, onAction, onClose, onEnter, onLeave }) {
  useSuppressCardPreview();
  useEffect(() => {
    window.addEventListener("resize", onClose);
    window.addEventListener("scroll", onClose, true);
    return () => {
      window.removeEventListener("resize", onClose);
      window.removeEventListener("scroll", onClose, true);
    };
  }, [onClose]);
  return <ActionPopover anchorRect={anchor.getBoundingClientRect()} anchorElement={anchor}
    actions={actions} onAction={onAction} onClose={onClose} variant="game"
    collapseEquivalentActions={false} previewCards={false} disabled={disabled}
    focusOnOpen={focusOnOpen} ariaLabel="Activate mana ability"
    onMouseEnter={onEnter} onMouseLeave={onLeave} />;
}
