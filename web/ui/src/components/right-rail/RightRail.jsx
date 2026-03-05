import { useEffect, useMemo } from "react";
import HoverArtOverlay from "./HoverArtOverlay";
import InspectorStackTimeline from "./InspectorStackTimeline";
import { useHoveredObjectId } from "@/context/HoverContext";
import { useGame } from "@/context/GameContext";
import { cn } from "@/lib/utils";

const INSPECTOR_WIDTH = "clamp(240px, 24vw, 360px)";
const INSPECTOR_BOTTOM_OFFSET = 208;

function objectExistsInState(state, objectId) {
  if (objectId == null) return false;
  const needle = String(objectId);
  const players = state?.players || [];

  for (const player of players) {
    const zones = [
      player?.battlefield || [],
      player?.hand_cards || [],
      player?.graveyard_cards || [],
      player?.exile_cards || [],
    ];
    for (const cards of zones) {
      for (const card of cards) {
        if (String(card?.id) === needle) return true;
        if (Array.isArray(card?.member_ids) && card.member_ids.some((id) => String(id) === needle)) {
          return true;
        }
      }
    }
  }

  for (const entry of state?.stack_objects || []) {
    if (String(entry?.id) === needle) return true;
  }

  return false;
}

function isFocusedDecision(decision) {
  return (
    !!decision
    && decision.kind !== "priority"
    && decision.kind !== "attackers"
    && decision.kind !== "blockers"
  );
}

export default function RightRail({
  pinnedObjectId,
  onVisibilityChange = null,
  onInspectObject = null,
}) {
  const { state } = useGame();
  const hoveredObjectId = useHoveredObjectId();
  const decision = state?.decision || null;
  const canAct = !!decision && decision.player === state?.perspective;
  const stackObjects = state?.stack_objects || [];
  const stackPreview = state?.stack_preview || [];
  const hasStackEntries = stackObjects.length > 0 || stackPreview.length > 0;
  const topStackObject = stackObjects[0];
  const topStackObjectId = topStackObject ? String(topStackObject.id) : null;
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
    ? (pinnedInspectorObjectId ?? resolvingCastObjectId ?? hoveredObjectId ?? topStackObjectId)
    : null;

  const selectedObjectId = focusedDecision
    ? decisionLockedObjectId
    : (pinnedInspectorObjectId ?? hoveredObjectId ?? resolvingCastObjectId ?? topStackObjectId);
  const validSelectedObjectId = objectExistsInState(state, selectedObjectId)
    ? selectedObjectId
    : null;
  const validPinnedObjectId = objectExistsInState(state, pinnedInspectorObjectId)
    ? pinnedInspectorObjectId
    : null;
  const stackFlowActive = hasStackEntries || (focusedDecision && canAct);
  const suppressDirectResolvingCastInspector =
    !stackFlowActive
    &&
    !focusedDecision
    && pinnedInspectorObjectId == null
    && hoveredObjectId == null
    &&
    validSelectedObjectId != null
    && resolvingCastObjectId != null
    && String(validSelectedObjectId) === String(resolvingCastObjectId);
  const shouldShowInspector = validSelectedObjectId != null && !suppressDirectResolvingCastInspector;
  const showStackTimeline = shouldShowInspector || stackFlowActive;
  const shouldShowRail = shouldShowInspector || showStackTimeline;
  // Reserve layout space only for persistent inspector contexts:
  // explicit pin (click), stack flow, or focused multi-step decision flow.
  const shouldReserveSpace = shouldShowRail && (validPinnedObjectId != null || focusedDecision || hasStackEntries);
  const inspectorSuppressStableId = focusedDecision ? null : resolvingCastStableId;
  const containerStyle = useMemo(
    () => ({ width: INSPECTOR_WIDTH, top: 8, bottom: INSPECTOR_BOTTOM_OFFSET }),
    []
  );

  useEffect(() => {
    if (typeof onVisibilityChange !== "function") return;
    onVisibilityChange(shouldReserveSpace);
  }, [onVisibilityChange, shouldReserveSpace]);

  return (
    <aside
      className={cn(
        "pointer-events-none absolute right-2 z-40 transition-all duration-250 ease-out",
        shouldShowRail
          ? "translate-x-0 opacity-100"
          : "translate-x-[110%] opacity-0"
      )}
      style={containerStyle}
      aria-hidden={!shouldShowRail}
    >
      <div
        className={cn(
          "rail-gradient h-full overflow-hidden rounded border border-[#2a3647] bg-[rgba(7,15,23,0.92)] shadow-[0_18px_42px_rgba(0,0,0,0.5)]",
          shouldShowRail ? "pointer-events-auto" : "pointer-events-none"
        )}
      >
        {shouldShowRail && (
          <div className="relative h-full min-h-0 overflow-hidden">
            {shouldShowInspector ? (
              <HoverArtOverlay
                objectId={validSelectedObjectId}
                suppressStableId={inspectorSuppressStableId}
              />
            ) : (
              <div className="absolute inset-0 bg-[linear-gradient(180deg,rgba(6,14,23,0.92),rgba(5,10,16,0.95))]" />
            )}
            <InspectorStackTimeline
              decision={decision}
              canAct={canAct}
              stackObjects={stackObjects}
              stackPreview={stackPreview}
              selectedObjectId={validSelectedObjectId}
              forceVisible={showStackTimeline}
              onInspectObject={onInspectObject}
            />
          </div>
        )}
      </div>
    </aside>
  );
}
