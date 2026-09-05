import useModalFocus from "@/hooks/useModalFocus";
import { X } from "lucide-react";
import HandZone from "@/components/board/HandZone";
import { cn } from "@/lib/utils";

// Dedicated full-screen hand-only view, only entered via the explicit `MobileViewToggle`
// (top-right of the scene). Tapping the close button — or the toggle again — returns to
// the battlefield view.
export default function MobileHandFullscreen({
  me,
  selectedObjectId,
  onInspect,
  onClose,
  className,
}) {
  const dialogRef = useModalFocus(onClose);
  const handCount = Number(me?.hand_size ?? 0);
  return (
    <section
      ref={dialogRef}
      tabIndex={-1}
      className={cn("mobile-mtga-hand-fullscreen", className)}
      role="dialog"
      aria-modal="true"
      aria-label="Hand"
    >
      <header className="mobile-mtga-hand-fullscreen-header">
        <span className="mobile-mtga-hand-fullscreen-title">Hand</span>
        <span className="mobile-mtga-hand-fullscreen-count">
          {handCount} card{handCount === 1 ? "" : "s"}
        </span>
        <button
          type="button"
          className="mobile-mtga-hand-fullscreen-close"
          aria-label="Close hand"
          onClick={onClose}
        >
          <X className="size-4" aria-hidden="true" />
        </button>
      </header>
      <div className="mobile-mtga-hand-fullscreen-body">
        <HandZone
          player={me}
          selectedObjectId={selectedObjectId}
          onInspect={onInspect}
          isExpanded
          layout="mobile-fullscreen"
        />
      </div>
    </section>
  );
}
