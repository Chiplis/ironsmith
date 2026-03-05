import { useCallback, useEffect, useMemo, useState } from "react";
import HoverArtOverlay from "./HoverArtOverlay";
import InspectorStackTimeline from "./InspectorStackTimeline";
import { useHoveredObjectId } from "@/context/HoverContext";
import { useGame } from "@/context/GameContext";
import DecisionRouter from "@/components/decisions/DecisionRouter";
import { Button } from "@/components/ui/button";
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

function decisionReferencesObject(decision, objectId) {
  if (!decision || objectId == null) return false;
  const needle = String(objectId);

  if (decision.kind === "select_objects") {
    return (decision.candidates || []).some((candidate) => String(candidate?.id) === needle);
  }

  if (decision.kind === "targets") {
    return (decision.requirements || []).some((req) =>
      (req.legal_targets || []).some(
        (target) => target?.kind === "object" && String(target?.object) === needle
      )
    );
  }

  if (decision.kind === "select_options") {
    return (decision.options || []).some((opt) => String(opt?.object_id) === needle);
  }

  return false;
}

export default function RightRail({
  pinnedObjectId,
  onInspectObject = null,
}) {
  const { state, cancelDecision } = useGame();
  const [inspectorSubmitAction, setInspectorSubmitAction] = useState(null);
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
  const relevantPinnedObjectId = focusedDecision && pinnedInspectorObjectId != null
    ? (decisionReferencesObject(decision, pinnedInspectorObjectId) ? pinnedInspectorObjectId : null)
    : pinnedInspectorObjectId;

  // During non-priority decision steps (targeting, choose number/options, etc),
  // keep inspector focus on the spell being cast/resolved instead of hover.
  const decisionLockedObjectId = focusedDecision
    ? (relevantPinnedObjectId ?? resolvingCastObjectId ?? hoveredObjectId ?? topStackObjectId)
    : null;

  const selectedObjectId = focusedDecision
    ? decisionLockedObjectId
    : (relevantPinnedObjectId ?? hoveredObjectId ?? resolvingCastObjectId ?? topStackObjectId);
  const validSelectedObjectId = objectExistsInState(state, selectedObjectId)
    ? selectedObjectId
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
  const showFallbackDecisionPanel = focusedDecision && canAct && !shouldShowInspector;
  const showStackTimeline = shouldShowInspector || stackFlowActive;
  const shouldShowRail = shouldShowInspector || showStackTimeline;
  const inspectorSuppressStableId = focusedDecision ? null : resolvingCastStableId;
  const containerStyle = useMemo(
    () => ({ width: INSPECTOR_WIDTH, top: 8, bottom: INSPECTOR_BOTTOM_OFFSET }),
    []
  );
  const handleInspectorSubmitChange = useCallback((nextAction) => {
    setInspectorSubmitAction(nextAction || null);
  }, []);

  useEffect(() => {
    if (!focusedDecision || !canAct) {
      setInspectorSubmitAction(null);
    }
  }, [focusedDecision, canAct, decision]);

  return (
    <aside
      className={cn(
        "pointer-events-none absolute right-2 z-40 transition-[transform,opacity] duration-140 ease-out",
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
                submitAction={inspectorSubmitAction}
                onInspectorSubmitChange={handleInspectorSubmitChange}
              />
            ) : (
              <div
                className={cn(
                  "absolute inset-0 bg-[linear-gradient(180deg,rgba(6,14,23,0.92),rgba(5,10,16,0.95))] transition-opacity",
                  showFallbackDecisionPanel ? "opacity-0" : "opacity-100"
                )}
              />
            )}
            {showFallbackDecisionPanel && (
              <div
                className={cn(
                  "absolute inset-x-2 top-2 z-[35] min-h-0 overflow-hidden rounded border border-[#5d7ea0] bg-[linear-gradient(180deg,rgba(6,14,22,0.76),rgba(6,14,22,0.9))] shadow-[0_16px_34px_rgba(0,0,0,0.55)] pointer-events-auto backdrop-blur-[2.2px] flex flex-col",
                  hasStackEntries ? "bottom-[176px]" : "bottom-[8px]"
                )}
              >
                <div className="border-b border-[#3c5876] bg-[rgba(8,19,31,0.9)] px-2.5 py-1.5">
                  <div className="text-[11px] font-bold uppercase tracking-[0.14em] text-[#8cc4ff]">
                    Current Decision
                  </div>
                </div>
                <div className="min-h-0 flex-1 overflow-y-auto px-1.5 py-1">
                  <DecisionRouter
                    decision={decision}
                    canAct={canAct}
                    inspectorOracleTextHeight={0}
                    inlineSubmit={false}
                    onSubmitActionChange={handleInspectorSubmitChange}
                  />
                </div>
                <div className="shrink-0 border-t border-[#3c5876] bg-[rgba(8,18,30,0.88)] px-2 py-1.5">
                  <div className="grid grid-cols-2 gap-1.5">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="h-7 rounded-sm border border-[#3d6ea5] bg-[rgba(40,84,136,0.78)] px-2 text-[12px] font-bold tracking-wide text-[#d9ecff] transition-colors hover:bg-[rgba(58,114,182,0.9)]"
                      disabled={!canAct || !inspectorSubmitAction || inspectorSubmitAction.disabled}
                      onClick={() => {
                        if (!canAct || !inspectorSubmitAction || inspectorSubmitAction.disabled) return;
                        inspectorSubmitAction.onSubmit?.();
                      }}
                    >
                      {inspectorSubmitAction?.label || "Submit"}
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      className="h-7 rounded-sm border border-[#8b3f4a] bg-[rgba(120,35,46,0.76)] px-2 text-[12px] font-bold uppercase tracking-wide text-[#ffd8df] transition-colors hover:bg-[rgba(163,50,64,0.9)]"
                      disabled={!canAct}
                      onClick={() => {
                        if (!canAct) return;
                        cancelDecision();
                      }}
                    >
                      Cancel
                    </Button>
                  </div>
                </div>
              </div>
            )}
            <InspectorStackTimeline
              decision={decision}
              canAct={canAct}
              stackObjects={stackObjects}
              stackPreview={stackPreview}
              selectedObjectId={validSelectedObjectId}
              onInspectObject={onInspectObject}
            />
          </div>
        )}
      </div>
    </aside>
  );
}
