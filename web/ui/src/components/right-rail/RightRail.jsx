import DecisionPanel from "@/components/left-rail/DecisionPanel";
import HoverArtOverlay from "./HoverArtOverlay";
import { useHoveredObjectId } from "@/context/HoverContext";
import { useGame } from "@/context/GameContext";

function isFocusedDecision(decision) {
  return (
    !!decision
    && decision.kind !== "priority"
    && decision.kind !== "attackers"
    && decision.kind !== "blockers"
  );
}

export default function RightRail({ pinnedObjectId }) {
  const { state } = useGame();
  const hoveredObjectId = useHoveredObjectId();
  const decision = state?.decision || null;
  const topStackObject = (state?.stack_objects || [])[0];
  const resolvingCastObjectId = state?.stack_size > 0 && topStackObject && !topStackObject.ability_kind
    ? String(topStackObject.id)
    : null;
  const resolvingCastStableId = resolvingCastObjectId && topStackObject?.stable_id != null
    ? Number(topStackObject.stable_id)
    : null;
  const pinnedInspectorObjectId = pinnedObjectId != null ? String(pinnedObjectId) : null;
  const focusedDecision = isFocusedDecision(decision);

  // During non-priority decision steps (targeting, choose number/options, etc),
  // keep inspector focus on the spell being cast/resolved instead of hover.
  const decisionLockedObjectId = focusedDecision
    ? (resolvingCastObjectId ?? pinnedInspectorObjectId)
    : null;

  const selectedObjectId = focusedDecision
    ? decisionLockedObjectId
    : (hoveredObjectId ?? resolvingCastObjectId ?? pinnedInspectorObjectId);
  const suppressDirectResolvingCastInspector =
    !focusedDecision
    &&
    selectedObjectId != null
    && resolvingCastObjectId != null
    && String(selectedObjectId) === String(resolvingCastObjectId);
  const shouldShowInspector = selectedObjectId != null && !suppressDirectResolvingCastInspector;
  const inspectorSuppressStableId = focusedDecision ? null : resolvingCastStableId;

  return (
    <aside className="rail-gradient rounded flex flex-col min-h-0 overflow-visible">
      {shouldShowInspector && (
        <div className="flex-1 min-h-0 overflow-hidden relative z-10">
          <HoverArtOverlay
            objectId={selectedObjectId}
            suppressStableId={inspectorSuppressStableId}
          />
        </div>
      )}

      <div
        className={
          shouldShowInspector
            ? "relative z-30 shrink-0 basis-[28%] min-h-[120px] max-h-[38%] overflow-visible bg-[#070f17]"
            : "relative z-30 flex-1 min-h-0 overflow-visible bg-[#070f17]"
        }
      >
        <DecisionPanel />
      </div>
    </aside>
  );
}
