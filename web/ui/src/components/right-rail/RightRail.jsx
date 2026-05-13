import { useLayoutEffect, useMemo, useRef, useState } from "react";
import HoverArtOverlay from "./HoverArtOverlay";
import { useHover } from "@/context/HoverContext";
import { useGame } from "@/context/GameContext";
import { animate, cancelMotion, uiSpring } from "@/lib/motion/anime";
import { playerAccentVars } from "@/lib/player-colors";
import { getVisibleStackObjects } from "@/lib/stack-targets";
import { cn } from "@/lib/utils";

const INSPECTOR_OVERLAY_WIDTH = "25vw";
const INSPECTOR_INLINE_MAX_WIDTH_PX = 420;
const INLINE_EXPANDED_MIN_WIDTH = 220;
const INLINE_EXPANDED_MAX_WIDTH_PX = 1800;
const INLINE_EXPANDED_MIN_HAND_WIDTH = 168;
const DEFAULT_INSPECTOR_BOTTOM_OFFSET = 8;
const INLINE_EXPANDED_DEFAULT_HEIGHT = 248;
const INLINE_EXPANDED_MIN_HEIGHT = 152;
const INLINE_EXPANDED_SAFE_GAP = 12;
const INLINE_EXPANDED_BOTTOM_GAP = 4;
const INLINE_EXPANDED_RIGHT_BLEED = 14;

/* Viewport-tier overrides for inspector sizing */
const TABLET_COMPACT_QUERY = "(min-width: 721px) and (max-width: 1023px)";
const SMALL_DESKTOP_QUERY = "(min-width: 1024px) and (max-width: 1439px)";
const LARGE_DESKTOP_QUERY = "(min-width: 1800px)";

function getViewportTierInspectorOverrides() {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return {};
  if (window.matchMedia(TABLET_COMPACT_QUERY).matches) {
    return { minWidth: 180, widthFraction: 0.22, expandedMaxWidth: 600, minHandWidth: 120 };
  }
  if (window.matchMedia(SMALL_DESKTOP_QUERY).matches) {
    return { minWidth: 200, widthFraction: 0.22, expandedMaxWidth: 1000, minHandWidth: 140 };
  }
  if (window.matchMedia(LARGE_DESKTOP_QUERY).matches) {
    return { minWidth: 260, widthFraction: 0.20, expandedMaxWidth: 2200, minHandWidth: 180 };
  }
  return {};
}
function inspectorBorderStyle(accent) {
  if (!accent) return undefined;
  return {
    ...playerAccentVars(accent),
    borderColor: `rgba(${accent.rgb}, 0.48)`,
    boxShadow: `0 0 0 1px rgba(${accent.rgb}, 0.18), 0 18px 42px rgba(0,0,0,0.28)`,
  };
}

function viewportInspectorTargetWidthPx() {
  if (typeof window === "undefined" || !Number.isFinite(window.innerWidth)) {
    return 300;
  }
  const overrides = getViewportTierInspectorOverrides();
  const minW = overrides.minWidth ?? 220;
  const fraction = overrides.widthFraction ?? 0.25;
  return Math.max(minW, Math.floor(window.innerWidth * fraction));
}

function viewedCardIds(state) {
  const ids = new Set();

  for (const id of state?.viewed_cards?.card_ids || []) {
    ids.add(String(id));
  }

  for (const card of state?.viewed_cards?.cards || []) {
    if (card?.id != null) {
      ids.add(String(card.id));
    }
  }

  return ids;
}

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
      player?.command_cards || [],
      player?.sideboard_cards || [],
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

  for (const entry of getVisibleStackObjects(state)) {
    if (String(entry?.id) === needle) return true;
    if (String(entry?.inspect_object_id) === needle) return true;
  }

  if (viewedCardIds(state).has(needle)) {
    return true;
  }

  return false;
}

function isViewedCardObject(state, objectId) {
  if (objectId == null) return false;
  return viewedCardIds(state).has(String(objectId));
}

function locateObjectInState(state, objectId) {
  if (objectId == null) return null;
  const needle = String(objectId);
  const viewedCards = state?.viewed_cards || null;
  if (viewedCardIds(state).has(needle)) {
    return {
      side: viewedCards?.visibility === "public" ? "public-view" : "private-view",
      zone: String(viewedCards?.zone || "").toLowerCase(),
      viewVisibility: viewedCards?.visibility === "public" ? "public" : "private",
    };
  }

  const perspective = state?.perspective;
  const players = state?.players || [];
  const zonesByPlayer = [
    ["battlefield", (player) => player?.battlefield || []],
    ["hand", (player) => player?.hand_cards || []],
    ["graveyard", (player) => player?.graveyard_cards || []],
    ["exile", (player) => player?.exile_cards || []],
    ["command", (player) => player?.command_cards || []],
    ["sideboard", (player) => player?.sideboard_cards || []],
  ];

  for (const player of players) {
    const side = player?.id === perspective ? "self" : "opponent";
    for (const [zone, readCards] of zonesByPlayer) {
      for (const card of readCards(player)) {
        if (String(card?.id) === needle) {
          return { side, zone };
        }
        if (Array.isArray(card?.member_ids) && card.member_ids.some((id) => String(id) === needle)) {
          return { side, zone };
        }
      }
    }
  }

  for (const entry of getVisibleStackObjects(state)) {
    if (String(entry?.id) === needle || String(entry?.inspect_object_id) === needle) {
      return { side: "stack", zone: "stack" };
    }
  }

  return null;
}

function canPersistPinnedInspector(location) {
  if (!location) return false;
  if (location.viewVisibility === "public") return true;
  if (location.viewVisibility === "private") return false;
  if (location.zone === "hand") return location.side === "self";
  return true;
}

function preferredInlinePlacement(location) {
  if (location?.viewVisibility === "private") {
    return { dock: "bottom", side: "right" };
  }
  if (location?.viewVisibility === "public") {
    return { dock: "top", side: "right" };
  }
  return {
    dock: location?.side === "self" && location?.zone !== "stack" ? "top" : "bottom",
    side: "right",
  };
}

function fixedInlinePlacementForVariant(inspectorVariant) {
  if (inspectorVariant === "debug") {
    return { dock: "bottom", side: "right" };
  }
  return { dock: "top", side: "right" };
}

function linkedInspectorLocationPriority(location) {
  if (!location) return -1;
  if (location.viewVisibility === "public") return 6;
  if (location.viewVisibility === "private") return 5;
  if (location.zone === "hand" && location.side === "self") return 4;
  if (location.zone === "battlefield") return 3;
  if (
    location.zone === "graveyard"
    || location.zone === "exile"
    || location.zone === "command"
    || location.zone === "sideboard"
  ) return 2;
  if (location.zone === "stack") return 0;
  return 1;
}

function resolveLinkedInspectorObjectId(state, hoveredObjectId, hoveredLinkedObjectIds) {
  if (!hoveredLinkedObjectIds || hoveredLinkedObjectIds.size === 0) return null;

  let bestObjectId = null;
  let bestPriority = -1;

  for (const linkedObjectId of hoveredLinkedObjectIds) {
    if (linkedObjectId == null) continue;
    const normalizedId = String(linkedObjectId);
    const location = locateObjectInState(state, normalizedId);
    if (!location) continue;

    let priority = linkedInspectorLocationPriority(location);
    if (hoveredObjectId != null && normalizedId === String(hoveredObjectId)) {
      priority -= 1;
    }

    if (priority > bestPriority) {
      bestPriority = priority;
      bestObjectId = normalizedId;
    }
  }

  return bestObjectId;
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
    return (decision.options || []).some((opt) => (
      String(opt?.object_id) === needle
      || (
        Array.isArray(opt?.related_object_ids)
        && opt.related_object_ids.some((id) => String(id) === needle)
      )
    ));
  }

  return false;
}

function objectInspectableInCurrentContext(state, decision, objectId) {
  if (objectId == null) return false;
  return objectExistsInState(state, objectId) || decisionReferencesObject(decision, objectId);
}

export default function RightRail({
  pinnedObjectId,
  transientInspectorPreview = null,
  transientInspectorPreviewIndex = 0,
  transientInspectorPreviewCount = 0,
  onShowPreviousTransientInspectorPreview = null,
  onShowNextTransientInspectorPreview = null,
  suppressFallback = false,
  inspectorBottomOffset = DEFAULT_INSPECTOR_BOTTOM_OFFSET,
  inline = false,
  inlineDockPlacement = "bottom",
  inlineHostSide = "right",
  inlineExpandedSide = "right",
  inlineExpandedAnchor = "bottom",
  inlineExpandedMaxHeight = null,
  expandInlineToZoneViewer = false,
  inlineFillWidth = false,
  inlineFillHeight = false,
  allowTopInlinePlacement = false,
  dockRole = "primary",
  inspectorVariant = "normal",
}) {
  const { state } = useGame();
  const [preferredExpandedInlineWidth, setPreferredExpandedInlineWidth] = useState(null);
  const [preferredExpandedInlineHeight, setPreferredExpandedInlineHeight] = useState(null);
  const [maxExpandedInlineWidth, setMaxExpandedInlineWidth] = useState(INLINE_EXPANDED_MAX_WIDTH_PX);
  const railRef = useRef(null);
  const expandedInspectorRef = useRef(null);
  const railMotionRef = useRef(null);
  const expandedMotionRef = useRef(null);
  const [expandedInlineHeight, setExpandedInlineHeight] = useState(INLINE_EXPANDED_DEFAULT_HEIGHT);
  const [inspectorAccent, setInspectorAccent] = useState(null);
  const { hoveredObjectId, hoveredLinkedObjectIds } = useHover();
  const decision = state?.decision || null;
  const transientPreviewObjectId = transientInspectorPreview?.objectId != null
    ? String(transientInspectorPreview.objectId)
    : null;
  const hasTransientInspectorPreview = Boolean(transientInspectorPreview?.card);
  const stackObjects = getVisibleStackObjects(state);
  const hasStackEntries = stackObjects.length > 0 || (state?.stack_preview || []).length > 0;
  const topStackObject = stackObjects[0];
  const topStackObjectId = topStackObject
    ? String(topStackObject.inspect_object_id ?? topStackObject.id)
    : null;
  const resolvingCastObjectId = state?.stack_size > 0 && topStackObject && !topStackObject.ability_kind
    ? String(topStackObject.inspect_object_id ?? topStackObject.id)
    : null;
  const linkedInspectorObjectId = useMemo(
    () => resolveLinkedInspectorObjectId(state, hoveredObjectId, hoveredLinkedObjectIds),
    [state, hoveredLinkedObjectIds, hoveredObjectId]
  );
  const pinnedInspectorObjectId = pinnedObjectId != null ? String(pinnedObjectId) : null;
  const focusedDecision = isFocusedDecision(decision);
  const pinnedInspectorIsViewedCard = isViewedCardObject(state, pinnedInspectorObjectId);
  const pinnedInspectorLocation = useMemo(
    () => locateObjectInState(state, pinnedInspectorObjectId),
    [pinnedInspectorObjectId, state]
  );
  const pinnedInspectorCanPersist = canPersistPinnedInspector(pinnedInspectorLocation);
  const relevantPinnedObjectId = focusedDecision && pinnedInspectorObjectId != null
    ? (
      decisionReferencesObject(decision, pinnedInspectorObjectId)
      || pinnedInspectorIsViewedCard
      || pinnedInspectorCanPersist
        ? pinnedInspectorObjectId
        : null
    )
    : pinnedInspectorObjectId;
  const directHoveredInspectorObjectId = (
    hoveredObjectId != null && objectInspectableInCurrentContext(state, decision, hoveredObjectId)
      ? String(hoveredObjectId)
      : null
  );
  const relevantHoveredObjectId = directHoveredInspectorObjectId ?? linkedInspectorObjectId;
  const fallbackDecisionObjectId = suppressFallback ? null : (resolvingCastObjectId ?? topStackObjectId);
  // During focused decision steps, keep the resolving stack object as a fallback.
  // Live hover should always win, even if the current decision does not reference it.
  const decisionLockedObjectId = focusedDecision
    ? (relevantHoveredObjectId ?? relevantPinnedObjectId ?? fallbackDecisionObjectId)
    : null;

  const selectedObjectId = focusedDecision
    ? decisionLockedObjectId
    : (relevantHoveredObjectId ?? relevantPinnedObjectId ?? fallbackDecisionObjectId);
  const validSelectedObjectId = objectInspectableInCurrentContext(state, decision, selectedObjectId)
    ? selectedObjectId
    : null;
  const transientPreviewSuppressedByHover = relevantHoveredObjectId != null;
  const hasActiveTransientInspectorPreview =
    hasTransientInspectorPreview && !transientPreviewSuppressedByHover;
  const selectedObjectLocation = useMemo(() => {
    if (hasActiveTransientInspectorPreview) {
      return locateObjectInState(state, transientPreviewObjectId);
    }
    const isCastingSpellFocus = (
      focusedDecision
      && validSelectedObjectId != null
      && resolvingCastObjectId != null
      && String(validSelectedObjectId) === String(resolvingCastObjectId)
      && decision?.player != null
    );
    if (isCastingSpellFocus) {
      return {
        side: Number(decision.player) === Number(state?.perspective) ? "self" : "opponent",
        zone: "casting",
      };
    }
    return locateObjectInState(state, validSelectedObjectId);
  }, [
    decision?.player,
    focusedDecision,
    hasActiveTransientInspectorPreview,
    resolvingCastObjectId,
    state,
    transientPreviewObjectId,
    validSelectedObjectId,
  ]);
  const forcedInlinePlacement = inline ? fixedInlinePlacementForVariant(inspectorVariant) : null;
  const preferredPlacement = useMemo(
    () => forcedInlinePlacement ?? preferredInlinePlacement(selectedObjectLocation),
    [forcedInlinePlacement, selectedObjectLocation]
  );
  const resolvedInlineDockPlacement = (
    preferredPlacement.dock === "top" && !allowTopInlinePlacement
      ? "bottom"
      : preferredPlacement.dock
  );
  const activeDockPlacement = !forcedInlinePlacement && dockRole === "opposite"
    ? (resolvedInlineDockPlacement === "top" ? "bottom" : "top")
    : resolvedInlineDockPlacement;
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
  const shouldShowInspector =
    hasActiveTransientInspectorPreview
    || (validSelectedObjectId != null && !suppressDirectResolvingCastInspector);
  const renderedInspectorObjectId = hasActiveTransientInspectorPreview
    ? transientPreviewObjectId
    : validSelectedObjectId;
  const renderedTransientInspectorPreview = hasActiveTransientInspectorPreview
    ? transientInspectorPreview
    : null;
  const renderedTransitionToken = renderedTransientInspectorPreview?.token || undefined;
  const inspectorShellShaderReveal = (
    renderedTransientInspectorPreview?.inspectorShaderReveal === true
    && renderedTransientInspectorPreview?.inspectorRevealScope === "inspector"
  );
  const inspectorShellShaderRevealDelayMs = inspectorShellShaderReveal
    ? Math.max(0, Number(renderedTransientInspectorPreview?.inspectorRevealDelayMs) || 0)
    : 0;
  const inspectorShellShaderRevealDurationMs = 780;
  const shouldShowRail = shouldShowInspector && (
    !inline
    || (
      inlineDockPlacement === activeDockPlacement
      && inlineHostSide === preferredPlacement.side
    )
  );
  const anchorExpandedInlineToTop = inlineExpandedAnchor === "top";
  const baseInlineWidthPx = useMemo(() => {
    return Math.min(INSPECTOR_INLINE_MAX_WIDTH_PX, viewportInspectorTargetWidthPx());
  }, []);
  const expandedInlineWidth = useMemo(() => {
    const effectiveMaxWidth = getViewportTierInspectorOverrides().expandedMaxWidth ?? INLINE_EXPANDED_MAX_WIDTH_PX;
    const baseWidth = Math.max(baseInlineWidthPx, INLINE_EXPANDED_MIN_WIDTH);
    const contentPreferredWidth = Number(preferredExpandedInlineWidth);
    const hasPreferredWidth = Number.isFinite(contentPreferredWidth) && contentPreferredWidth > 0;
    const preferredWidth = hasPreferredWidth ? Math.ceil(contentPreferredWidth) : baseWidth;
    const measuredMaxWidth = Math.round(maxExpandedInlineWidth || effectiveMaxWidth);
    const viewportTargetWidth = viewportInspectorTargetWidthPx();
    const defaultWidthCap = Math.min(
      measuredMaxWidth,
      expandInlineToZoneViewer
        ? effectiveMaxWidth
        : viewportTargetWidth
    );
    const preferredWidthCap = Math.min(
      measuredMaxWidth,
      effectiveMaxWidth,
      expandInlineToZoneViewer
        ? effectiveMaxWidth
        : Math.max(viewportTargetWidth, preferredWidth)
    );
    const defaultWidth = Math.max(baseWidth, defaultWidthCap);
    if (!hasPreferredWidth) {
      return defaultWidth;
    }

    return Math.max(baseWidth, Math.min(preferredWidth, preferredWidthCap));
  }, [baseInlineWidthPx, expandInlineToZoneViewer, maxExpandedInlineWidth, preferredExpandedInlineWidth]);

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
    const expandedEl = expandedInspectorRef.current;
    if (!expandedEl) return undefined;

    cancelMotion(expandedMotionRef.current);
    expandedMotionRef.current = animate(expandedEl, {
      opacity: shouldShowRail ? 1 : 0,
      x: shouldShowRail ? 0 : 32,
      y: shouldShowRail ? 0 : (anchorExpandedInlineToTop ? -10 : 10),
      scale: shouldShowRail ? 1 : 0.965,
      rotateY: shouldShowRail ? 0 : -18,
      rotateZ: shouldShowRail ? 0 : 1.8,
      duration: shouldShowRail ? 420 : 280,
      ease: uiSpring({ duration: shouldShowRail ? 420 : 280, bounce: 0.12 }),
    });

    return () => {
      cancelMotion(expandedMotionRef.current);
      expandedMotionRef.current = null;
    };
  }, [anchorExpandedInlineToTop, shouldShowRail]);

  useLayoutEffect(() => {
    if (!inline) return undefined;
    const railEl = railRef.current;
    if (!railEl) return undefined;

    const workspaceEl = railEl.closest("[data-workspace-shell]") ?? railEl.closest("section");
    const dockEl = railEl.closest("[data-inspector-dock]");
    const handDockEl = dockEl?.querySelector("[data-hand-dock-lane]");
    const stripEl = workspaceEl?.querySelector(".priority-inline-panel");
    const stackEl = workspaceEl?.querySelector("[data-my-zone] [data-inspector-stack-timeline]");
    const zoneViewerEl = expandInlineToZoneViewer
      ? workspaceEl?.querySelector('[data-zone-viewer="embedded"]')
      : null;
    let rafId = null;

    const measureExpandedLayout = () => {
      const hostRect = (workspaceEl || railEl).getBoundingClientRect();
      const railRect = railEl.getBoundingClientRect();
      const dockRect = dockEl?.getBoundingClientRect?.() || null;
      const stripRect = stripEl?.getBoundingClientRect?.() || null;
      const stackRect = stackEl?.getBoundingClientRect?.() || null;
      const zoneViewerRect = zoneViewerEl?.getBoundingClientRect?.() || null;
      const safeTop = anchorExpandedInlineToTop
        ? ((dockRect || railEl.getBoundingClientRect()).top + INLINE_EXPANDED_SAFE_GAP)
        : (
          inlineDockPlacement === "top"
            ? hostRect.top + INLINE_EXPANDED_SAFE_GAP
            : Math.max(
              stripRect ? stripRect.bottom + INLINE_EXPANDED_SAFE_GAP : hostRect.top + INLINE_EXPANDED_SAFE_GAP,
              stackRect && stackRect.height > 0
                ? stackRect.bottom + INLINE_EXPANDED_SAFE_GAP
                : hostRect.top + INLINE_EXPANDED_SAFE_GAP
            )
        );
      const safeBottom = anchorExpandedInlineToTop
        ? (
          inlineDockPlacement === "top" && dockRect
            ? dockRect.bottom - INLINE_EXPANDED_BOTTOM_GAP
            : hostRect.bottom - INLINE_EXPANDED_BOTTOM_GAP
        )
        : (
          inlineDockPlacement === "top"
            ? ((dockRect || railEl.getBoundingClientRect()).bottom - INLINE_EXPANDED_BOTTOM_GAP)
            : hostRect.bottom - INLINE_EXPANDED_BOTTOM_GAP
        );
      const fillHeightRect = dockRect || railRect;
      const availableHeight = inlineFillHeight
        ? Math.max(0, Math.floor(fillHeightRect.height))
        : Math.max(0, Math.floor(safeBottom - safeTop));
      const minimumHeight = Math.min(INLINE_EXPANDED_MIN_HEIGHT, availableHeight);
      const defaultExpandedHeight = inlineExpandedMaxHeight == null
        ? INLINE_EXPANDED_DEFAULT_HEIGHT
        : Math.min(INLINE_EXPANDED_DEFAULT_HEIGHT, inlineExpandedMaxHeight);
      const contentPreferredHeight = Number(preferredExpandedInlineHeight);
      const hasPreferredHeight = Number.isFinite(contentPreferredHeight) && contentPreferredHeight > 0;
      const preferredHeight = hasPreferredHeight
        ? Math.ceil(contentPreferredHeight + 4)
        : defaultExpandedHeight;
      const heightCap = inlineExpandedMaxHeight == null
        ? availableHeight
        : Math.min(inlineExpandedMaxHeight, availableHeight);
      const nextHeight = inlineFillHeight
        ? availableHeight
        : Math.max(
          minimumHeight,
          Math.min(Math.max(defaultExpandedHeight, preferredHeight), heightCap)
        );

      setExpandedInlineHeight((currentHeight) => (
        Math.abs(currentHeight - nextHeight) >= 1 ? nextHeight : currentHeight
      ));

      const tierOverrides = getViewportTierInspectorOverrides();
      const effectiveExpandedMaxWidth = tierOverrides.expandedMaxWidth ?? INLINE_EXPANDED_MAX_WIDTH_PX;
      const effectiveMinHandWidth = tierOverrides.minHandWidth ?? INLINE_EXPANDED_MIN_HAND_WIDTH;
      const dockGap = dockEl
        ? parseFloat(getComputedStyle(dockEl).columnGap || getComputedStyle(dockEl).gap || "0")
        : 0;
      const availableWidth = dockRect
        ? (
          (() => {
            const dockAvailableWidth = inlineDockPlacement === "top"
              ? dockRect.width
              : dockRect.width - effectiveMinHandWidth - dockGap;
            if (!expandInlineToZoneViewer || !zoneViewerRect || zoneViewerRect.right >= dockRect.right) {
              return dockAvailableWidth;
            }

            const zoneViewerBoundedWidth = dockRect.right - zoneViewerRect.right - INLINE_EXPANDED_SAFE_GAP;
            return Math.max(dockAvailableWidth, zoneViewerBoundedWidth);
          })()
        )
        : effectiveExpandedMaxWidth;
      const nextMaxWidth = Math.max(
        Math.max(baseInlineWidthPx, INLINE_EXPANDED_MIN_WIDTH),
        Math.min(Math.floor(availableWidth), effectiveExpandedMaxWidth)
      );
      setMaxExpandedInlineWidth((currentWidth) => (
        Math.abs(currentWidth - nextMaxWidth) >= 1 ? nextMaxWidth : currentWidth
      ));
    };

    const scheduleMeasure = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = null;
        measureExpandedLayout();
      });
    };

    scheduleMeasure();

    const observer = new ResizeObserver(scheduleMeasure);
    observer.observe(railEl);
    if (workspaceEl) observer.observe(workspaceEl);
    if (dockEl) observer.observe(dockEl);
    if (handDockEl) observer.observe(handDockEl);
    if (stripEl) observer.observe(stripEl);
    if (stackEl) observer.observe(stackEl);
    if (zoneViewerEl) observer.observe(zoneViewerEl);
    window.addEventListener("resize", scheduleMeasure);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      observer.disconnect();
      window.removeEventListener("resize", scheduleMeasure);
    };
  }, [
    anchorExpandedInlineToTop,
    baseInlineWidthPx,
    inline,
    inlineDockPlacement,
    inlineExpandedMaxHeight,
    inlineFillHeight,
    preferredExpandedInlineHeight,
    expandInlineToZoneViewer,
    shouldShowRail,
  ]);

  const containerStyle = useMemo(
    () => (inline
      ? {
        width: shouldShowRail
          ? (
            inlineFillWidth
              ? "100%"
              : `${expandedInlineWidth}px`
          )
          : "0px",
      }
      : {
        width: INSPECTOR_OVERLAY_WIDTH,
        top: 8,
        bottom: inspectorBottomOffset,
      }),
    [
      expandedInlineWidth,
      inline,
      inlineFillWidth,
      inspectorBottomOffset,
      shouldShowRail,
    ]
  );
  const expandedInlineShellOffset = inlineExpandedSide === "left"
    ? {
      left: `-${INLINE_EXPANDED_RIGHT_BLEED}px`,
      right: "auto",
      top: anchorExpandedInlineToTop ? "0" : "auto",
      bottom: anchorExpandedInlineToTop ? "auto" : "0",
      transformOrigin: anchorExpandedInlineToTop ? "top left" : "bottom left",
    }
    : {
      left: "auto",
      right: `-${INLINE_EXPANDED_RIGHT_BLEED}px`,
      top: anchorExpandedInlineToTop ? "0" : "auto",
      bottom: anchorExpandedInlineToTop ? "auto" : "0",
      transformOrigin: anchorExpandedInlineToTop ? "top right" : "bottom right",
    };
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
          ref={expandedInspectorRef}
          data-card-inspector="true"
          data-zone-transition-token={renderedTransitionToken}
          className={cn(
            "ironsmith-inspector-shell overflow-hidden border border-[#2a3647]/75 bg-[rgba(8,12,18,0.94)] shadow-[0_18px_42px_rgba(0,0,0,0.28)]",
            inline
              ? "hand-inspector-inline-shell ironsmith-inspector-shell--expanded absolute rounded-none"
              : "h-full rounded-none",
            inspectorShellShaderReveal && "ironsmith-inspector-shell--shader-reveal",
            shouldShowRail ? "pointer-events-auto z-[60]" : "pointer-events-none z-0"
          )}
          style={{
            ...(inline
              ? {
                width: "100%",
                height: inlineFillHeight ? "100%" : `${expandedInlineHeight}px`,
                ...expandedInlineShellOffset,
              }
              : { width: "100%", height: "100%" }),
            ...(inspectorShellShaderReveal
              ? {
                "--inspector-shader-reveal-delay": `${inspectorShellShaderRevealDelayMs}ms`,
                "--inspector-shell-reveal-duration": `${inspectorShellShaderRevealDurationMs}ms`,
              }
              : undefined),
            ...inspectorBorderStyle(inspectorAccent),
          }}
        >
          <div className="flex h-full min-h-0 flex-col overflow-hidden">
            <div className="min-h-0 flex-1 overflow-hidden">
              <HoverArtOverlay
                objectId={shouldShowRail ? renderedInspectorObjectId : null}
                transientPreview={renderedTransientInspectorPreview}
                transientPreviewIndex={transientInspectorPreviewIndex}
                transientPreviewCount={transientInspectorPreviewCount}
                onShowPreviousTransientPreview={onShowPreviousTransientInspectorPreview}
                onShowNextTransientPreview={onShowNextTransientInspectorPreview}
                displayMode="inspector"
                inspectorVariant={inspectorVariant}
                availableInspectorWidth={inline ? expandedInlineWidth : undefined}
                availableInspectorHeight={inline ? expandedInlineHeight : undefined}
                onOracleTextHeightChange={inline ? setPreferredExpandedInlineHeight : null}
                onPreferredInspectorWidthChange={inline ? setPreferredExpandedInlineWidth : null}
                onInspectorAccentChange={setInspectorAccent}
              />
            </div>
          </div>
        </div>
      </div>
    </aside>
  );
}
