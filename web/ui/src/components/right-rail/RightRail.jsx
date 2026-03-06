import { useEffect, useMemo, useState } from "react";
import HoverArtOverlay from "./HoverArtOverlay";
import { useHoveredObjectId } from "@/context/HoverContext";
import { useGame } from "@/context/GameContext";
import { cn } from "@/lib/utils";

const INSPECTOR_OVERLAY_WIDTH = "clamp(240px, 24vw, 360px)";
const INSPECTOR_INLINE_MIN_WIDTH = 220;
const INSPECTOR_INLINE_FALLBACK_WIDTH = 300;
const INSPECTOR_INLINE_MAX_WIDTH = "min(32vw, 420px)";
const DEFAULT_INSPECTOR_BOTTOM_OFFSET = 8;

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
  inspectorBottomOffset = DEFAULT_INSPECTOR_BOTTOM_OFFSET,
  inline = false,
}) {
  const { state } = useGame();
  const [preferredInlineWidth, setPreferredInlineWidth] = useState(null);
  const hoveredObjectId = useHoveredObjectId();
  const decision = state?.decision || null;
  const stackObjects = state?.stack_objects || [];
  const hasStackEntries = stackObjects.length > 0 || (state?.stack_preview || []).length > 0;
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
  const suppressDirectResolvingCastInspector =
    !hasStackEntries
    &&
    !focusedDecision
    && pinnedInspectorObjectId == null
    && hoveredObjectId == null
    &&
    validSelectedObjectId != null
    && resolvingCastObjectId != null
    && String(validSelectedObjectId) === String(resolvingCastObjectId);
  const shouldShowInspector = validSelectedObjectId != null && !suppressDirectResolvingCastInspector;
  const shouldShowRail = shouldShowInspector;
  const inspectorSuppressStableId = focusedDecision ? null : resolvingCastStableId;
  const inlineWidth = useMemo(() => {
    const preferred = Number.isFinite(preferredInlineWidth)
      ? Math.round(preferredInlineWidth)
      : INSPECTOR_INLINE_FALLBACK_WIDTH;
    return `clamp(${INSPECTOR_INLINE_MIN_WIDTH}px, ${preferred}px, ${INSPECTOR_INLINE_MAX_WIDTH})`;
  }, [preferredInlineWidth]);

  useEffect(() => {
    if (!shouldShowRail) {
      setPreferredInlineWidth(null);
    }
  }, [shouldShowRail]);

  const containerStyle = useMemo(
    () => (inline
      ? {
        width: shouldShowRail ? inlineWidth : "0px",
      }
      : {
        width: INSPECTOR_OVERLAY_WIDTH,
        top: 8,
        bottom: inspectorBottomOffset,
      }),
    [inline, inlineWidth, inspectorBottomOffset, shouldShowRail]
  );

  return (
    <aside
      className={cn(
        inline
          ? "pointer-events-none relative h-full shrink-0 overflow-hidden transition-[width,transform,opacity] duration-220 ease-out"
          : "pointer-events-none absolute right-2 z-40 transition-[transform,opacity] duration-140 ease-out",
        shouldShowRail
          ? "translate-x-0 opacity-100"
          : "translate-x-[110%] opacity-0"
      )}
      style={containerStyle}
      aria-hidden={!shouldShowRail}
    >
      <div
        className={cn(
          "h-full overflow-hidden border border-[#2a3647]/70 bg-transparent shadow-[0_18px_42px_rgba(0,0,0,0.24)]",
          inline ? "rounded-r rounded-l-sm" : "rounded",
          shouldShowRail ? "pointer-events-auto" : "pointer-events-none"
        )}
      >
        {shouldShowRail && (
          <div className="relative h-full min-h-0 overflow-hidden">
            <HoverArtOverlay
              objectId={validSelectedObjectId}
              suppressStableId={inspectorSuppressStableId}
              compact={inline}
              onPreferredWidthChange={inline ? setPreferredInlineWidth : null}
            />
          </div>
        )}
      </div>
    </aside>
  );
}
