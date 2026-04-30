import { useCallback, useState } from "react";
import HandZone from "@/components/board/HandZone";
import { cn } from "@/lib/utils";

// Half-hidden hand at the bottom of the battlefield. Tap whitespace (anywhere not on
// a `.hand-card`) to fan in place from ~peek to ~fanned height. Drag from a card upward
// uses HandZone's existing pointer-down → drag flow, which Workspace's drop handler
// recognizes via the existing `[data-mobile-hand-drop-target]` attribute on the
// MobileBattlefieldBand self-side wrapper.
export default function MobileHandFan({
  me,
  selectedObjectId,
  onInspect,
  className,
}) {
  const [fanned, setFanned] = useState(false);

  const handleClick = useCallback((event) => {
    if (event.target instanceof Element && event.target.closest(".game-card.hand-card")) {
      return;
    }
    setFanned((current) => !current);
  }, []);

  return (
    <div
      className={cn(
        "mobile-mtga-hand-fan",
        fanned && "mobile-mtga-hand-fan--fanned",
        className,
      )}
      data-fanned={fanned ? "true" : "false"}
      onClick={handleClick}
    >
      <div className="mobile-mtga-hand-fan-viewport">
        <HandZone
          player={me}
          selectedObjectId={selectedObjectId}
          onInspect={onInspect}
          isExpanded
          layout="mobile-fan"
        />
      </div>
    </div>
  );
}
