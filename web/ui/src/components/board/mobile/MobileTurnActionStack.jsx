import { forwardRef } from "react";
import { cn } from "@/lib/utils";

// `MobileTurnActionStack` is a portal target. `DecisionPopupLayer` renders the actual
// primary / secondary turn-flow buttons (Pass Priority, Confirm Attackers, End Turn,
// Submit, Actions(n)) into this slot via createPortal when given the matching
// `mobileBattleActionStackPortalTarget` prop.
const MobileTurnActionStack = forwardRef(function MobileTurnActionStack(
  { className },
  ref,
) {
  return (
    <div
      ref={ref}
      className={cn("mobile-mtga-turn-action-stack", className)}
      data-mobile-mtga-action-stack
      role="group"
      aria-label="Turn actions"
    />
  );
});

export default MobileTurnActionStack;
