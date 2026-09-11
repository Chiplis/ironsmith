import { Check } from "lucide-react";
import { cn } from "@/lib/utils";
import { requestObjectSelection } from "@/lib/object-selection";
import { usePointerClickGuard } from "@/lib/usePointerClickGuard";

/**
 * The check on a chosen card is the only surface that can unchoose it, so it
 * swallows the pointer sequence the card underneath would otherwise read as
 * another choice.
 */
export default function SelectionCheckBadge({
  objectId,
  className = "",
  label = "Deselect card",
}) {
  const { registerPointerDown, shouldHandleClick } = usePointerClickGuard();

  const deselect = (event) => {
    event.preventDefault();
    event.stopPropagation();
    requestObjectSelection(objectId, "remove");
  };

  return (
    <span
      role="button"
      tabIndex={-1}
      aria-label={label}
      title={label}
      className={cn("card-selection-check", className)}
      onPointerDown={(event) => {
        event.stopPropagation();
        if (!registerPointerDown(event)) return;
        deselect(event);
      }}
      onMouseDown={(event) => event.stopPropagation()}
      onClick={(event) => {
        event.stopPropagation();
        if (!shouldHandleClick(event)) {
          event.preventDefault();
          return;
        }
        deselect(event);
      }}
    >
      <Check className="card-selection-check-icon" aria-hidden="true" />
    </span>
  );
}
