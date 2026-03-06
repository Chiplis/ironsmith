import { useLayoutEffect, useMemo, useRef, useState } from "react";
import HoverArtOverlay from "./HoverArtOverlay";
import { useHoveredObjectId } from "@/context/HoverContext";
import { useGame } from "@/context/GameContext";
import { animate, cancelMotion, uiSpring } from "@/lib/motion/anime";
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
  inlineExpanded = false,
  forceInlineExpanded = false,
}) {
  const { state } = useGame();
  const [preferredInlineWidth, setPreferredInlineWidth] = useState(null);
  const railRef = useRef(null);
  const compactInspectorRef = useRef(null);
  const expandedInspectorRef = useRef(null);
  const railMotionRef = useRef(null);
  const compactMotionRef = useRef(null);
  const expandedMotionRef = useRef(null);
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
  const shouldRenderExpandedInlineInspector =
    inline
    && shouldShowRail
    && (inlineExpanded || hoveredObjectId != null);
  const useExpandedInlineInspector =
    shouldRenderExpandedInlineInspector
    && (
      forceInlineExpanded
      || hoveredObjectId != null
      || (
        hoveredObjectId != null
        && pinnedInspectorObjectId != null
        && validSelectedObjectId != null
        && String(hoveredObjectId) === String(pinnedInspectorObjectId)
        && String(validSelectedObjectId) === String(pinnedInspectorObjectId)
      )
    );
  const inspectorSuppressStableId = focusedDecision ? null : resolvingCastStableId;
  const inlineWidth = useMemo(() => {
    const preferred = Number.isFinite(preferredInlineWidth)
      ? Math.round(preferredInlineWidth)
      : INSPECTOR_INLINE_FALLBACK_WIDTH;
    return `clamp(${INSPECTOR_INLINE_MIN_WIDTH}px, ${preferred}px, ${INSPECTOR_INLINE_MAX_WIDTH})`;
  }, [preferredInlineWidth]);

  useLayoutEffect(() => {
    const railEl = railRef.current;
    if (!railEl) return undefined;

    cancelMotion(railMotionRef.current);
    railMotionRef.current = animate(railEl, {
      x: shouldShowRail ? 0 : 88,
      opacity: shouldShowRail ? 1 : 0,
      duration: shouldShowRail ? 360 : 280,
      ease: uiSpring({ duration: shouldShowRail ? 360 : 280, bounce: 0.14 }),
    });

    return () => {
      cancelMotion(railMotionRef.current);
      railMotionRef.current = null;
    };
  }, [inline, shouldShowRail]);

  useLayoutEffect(() => {
    const compactEl = compactInspectorRef.current;
    if (!compactEl) return undefined;

    cancelMotion(compactMotionRef.current);
    compactMotionRef.current = animate(compactEl, {
      opacity: useExpandedInlineInspector ? 0 : 1,
      scale: useExpandedInlineInspector ? 0.986 : 1,
      duration: 220,
      ease: "out(3)",
    });

    return () => {
      cancelMotion(compactMotionRef.current);
      compactMotionRef.current = null;
    };
  }, [useExpandedInlineInspector]);

  useLayoutEffect(() => {
    const expandedEl = expandedInspectorRef.current;
    if (!expandedEl) return undefined;

    cancelMotion(expandedMotionRef.current);
    expandedMotionRef.current = animate(expandedEl, {
      opacity: useExpandedInlineInspector ? 1 : 0,
      y: useExpandedInlineInspector ? 0 : 22,
      scale: useExpandedInlineInspector ? 1 : 0.94,
      rotateX: useExpandedInlineInspector ? 0 : 72,
      duration: 420,
      ease: uiSpring({ duration: 420, bounce: 0.12 }),
    });

    return () => {
      cancelMotion(expandedMotionRef.current);
      expandedMotionRef.current = null;
    };
  }, [useExpandedInlineInspector]);

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
      ref={railRef}
      className={cn(
        inline
          ? "pointer-events-none relative h-full self-end shrink-0 overflow-visible transition-[width] duration-320 ease-[cubic-bezier(0.22,1,0.36,1)]"
          : "pointer-events-none absolute right-2 z-40"
      )}
      style={containerStyle}
      aria-hidden={!shouldShowRail}
    >
      <div className={cn("relative h-full min-h-0", inline ? "overflow-visible" : "overflow-hidden")}>
        <div
          ref={compactInspectorRef}
          className={cn(
            "h-full overflow-hidden border border-[#2a3647]/70 bg-transparent shadow-[0_18px_42px_rgba(0,0,0,0.24)]",
            inline ? "rounded-r rounded-l-sm" : "rounded",
            shouldShowRail ? "pointer-events-auto" : "pointer-events-none"
          )}
        >
          <div className="relative h-full min-h-0 overflow-hidden">
            <HoverArtOverlay
              objectId={validSelectedObjectId}
              suppressStableId={inspectorSuppressStableId}
              compact={inline}
              onPreferredWidthChange={inline ? setPreferredInlineWidth : null}
            />
          </div>
        </div>
        {shouldRenderExpandedInlineInspector && (
          <div
            ref={expandedInspectorRef}
            className={cn(
              "hand-inspector-inline-shell absolute inset-x-0 bottom-0 overflow-hidden border border-[#2a3647]/75 bg-[rgba(8,12,18,0.94)] shadow-[0_24px_48px_rgba(0,0,0,0.34)]",
              useExpandedInlineInspector ? "is-open" : "is-closed"
            )}
            style={{ height: "clamp(236px, 34vh, 392px)", transformOrigin: "bottom center" }}
          >
            <HoverArtOverlay
              objectId={validSelectedObjectId}
              suppressStableId={inspectorSuppressStableId}
              displayMode={forceInlineExpanded ? "full-art" : "inspector"}
            />
          </div>
        )}
      </div>
    </aside>
  );
}
