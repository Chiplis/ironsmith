import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { manaPaymentActionMap } from "@/lib/mana-payment-actions";
import { useDragState } from "@/context/DragContext";
import { useGame } from "@/context/GameContext";
import {
  useAnchoredCardPreview,
  useCardPreviewSuppressed,
  useHoverActions,
  useHoveredObjectId,
} from "@/context/HoverContext";
import HoverArtOverlay from "./HoverArtOverlay";
import useDisplayedCardImage from "@/hooks/useDisplayedCardImage";
import { cardArtCropUrl } from "@/lib/card-image-variants";
import { prepareCardFrame } from "@/lib/card-frame-preparation";
import { resolveScryfallImageUrl } from "@/lib/scryfall";
import { playerAccentVars } from "@/lib/player-colors";
import { samePlayerId } from "@/lib/player-display";
import { getVisibleStackObjects } from "@/lib/stack-targets";
import { canHoverInspectorObject, objectExistsInState } from "@/lib/inspector-selection";

const PREVIEW_OPEN_DELAY_MS = 500;
const PREVIEW_CLOSE_DELAY_MS = 240;
const PREVIEW_FADE_MS = 220;
const FALLBACK_SIZE = { width: 380, height: 531 };

function phaseToolbarTop(fallbackTop = 0) {
  if (typeof document === "undefined") return fallbackTop;
  const controls = document.querySelector(".topbar-shell");
  const rect = controls?.getBoundingClientRect?.();
  return rect && rect.width > 0 && rect.height > 0
    ? Math.max(fallbackTop, rect.top)
    : fallbackTop;
}

function previewLeftInset({
  top,
  height,
  minimumLeft = 8,
  gap = 12,
}) {
  if (typeof document === "undefined") return minimumLeft;
  const mainDecisionButton = document.querySelector(
    ".topbar-main-decision-host .decision-main-button"
  );
  const buttonRect = mainDecisionButton?.getBoundingClientRect?.();
  if (!buttonRect || buttonRect.width <= 0 || buttonRect.height <= 0) return minimumLeft;

  const previewBottom = top + height;
  const overlapsButtonVertically = top < buttonRect.bottom + gap
    && previewBottom > buttonRect.top - gap;
  return overlapsButtonVertically
    ? Math.max(minimumLeft, buttonRect.right + gap)
    : minimumLeft;
}

function snapPreviewToAdjacentCardCenter({
  source,
  left,
  top,
  width,
  height,
  side,
  minimumLeft,
  maximumLeft,
}) {
  const zoneMenu = source?.closest?.(".zone-pile-menu");
  const row = zoneMenu || source?.closest?.('.battlefield-row[data-bf-side="bottom"]');
  if (!row) return left;

  const sourceRect = source.getBoundingClientRect();
  const sourceCenterX = sourceRect.left + (sourceRect.width / 2);
  const previewBottom = top + height;
  const proposedEdge = side === "right" ? left : left + width;
  const candidateSelector = zoneMenu
    ? ".zone-pile-card-row[data-object-id]"
    : ".battlefield-row-card[data-object-id]";
  const candidateCenters = Array.from(row.querySelectorAll(candidateSelector))
    .map((card) => card.getBoundingClientRect())
    .filter((cardRect) => (
      cardRect.width > 0
      && cardRect.height > 0
      && cardRect.bottom > top
      && cardRect.top < previewBottom
      && (
        side === "right"
          ? cardRect.left + (cardRect.width / 2) > sourceCenterX
          : cardRect.left + (cardRect.width / 2) < sourceCenterX
      )
    ))
    .map((cardRect) => cardRect.left + (cardRect.width / 2))
    .filter((centerX) => {
      const snappedLeft = side === "right" ? centerX : centerX - width;
      return snappedLeft >= minimumLeft && snappedLeft <= maximumLeft;
    })
    .sort((leftCenter, rightCenter) => (
      Math.abs(leftCenter - proposedEdge) - Math.abs(rightCenter - proposedEdge)
    ));

  const centerX = candidateCenters[0];
  if (!Number.isFinite(centerX)) return left;
  return side === "right" ? centerX : centerX - width;
}

function objectFamilyIds(state, objectId) {
  const ids = new Set([String(objectId)]);
  for (const stackEntry of getVisibleStackObjects(state)) {
    const stackIds = [stackEntry?.id, stackEntry?.inspect_object_id]
      .filter((id) => id != null)
      .map(String);
    if (!stackIds.some((id) => ids.has(id))) continue;
    for (const id of stackIds) ids.add(id);
  }
  for (const player of state?.players || []) {
    for (const card of player?.battlefield || []) {
      const family = [card?.id, ...(card?.member_ids || [])]
        .filter((id) => id != null)
        .map(String);
      if (!family.some((id) => ids.has(id))) continue;
      for (const id of family) ids.add(id);
      return ids;
    }
  }
  return ids;
}

function zonePreviewLayout(anchorRect, size, source = null) {
  const margin = 8;
  const gap = 14;
  const minimumTop = phaseToolbarTop(margin);
  const availableHeight = Math.max(0, window.innerHeight - margin - minimumTop);
  const height = Math.min(size.height, availableHeight, (window.innerWidth - margin * 2) * 88 / 63);
  const width = Math.min(size.width, height * (63 / 88), window.innerWidth - (margin * 2));
  const top = Math.max(
    minimumTop,
    Math.min(window.innerHeight - height - margin, anchorRect.top + (anchorRect.height / 2) - (height / 2))
  );
  const minimumLeft = previewLeftInset({ top, height, minimumLeft: margin });
  const maximumLeft = Math.max(minimumLeft, window.innerWidth - width - margin);
  let side = "right";
  let left = anchorRect.right + gap;
  if (left + width > window.innerWidth - margin) {
    side = "left";
    left = anchorRect.left - width - gap;
  }
  const wasClampedPastDecisionButton = left < minimumLeft;
  left = Math.max(minimumLeft, Math.min(maximumLeft, left));
  if (source && !wasClampedPastDecisionButton) {
    // Match battlefield previews: align the frame to the adjacent card's
    // midpoint so the next card remains partially exposed and hoverable.
    left = snapPreviewToAdjacentCardCenter({
      source,
      left,
      top,
      width,
      height,
      side,
      minimumLeft,
      maximumLeft,
    });
  }
  return {
    left: Math.round(left),
    top: Math.round(top),
    right: "auto",
    maxHeight: `${Math.max(0, Math.floor(height))}px`,
  };
}

function zonePreviewPosition(source, size) {
  const anchorRect = source.getBoundingClientRect();
  return zonePreviewLayout(anchorRect, size, source);
}

function zoneAnchoredPreviewPosition(anchorRect, objectId, size) {
  if (!anchorRect || typeof document === "undefined" || typeof window === "undefined") return null;
  const zoneCard = Array.from(document.querySelectorAll("[data-zone-card][data-object-id]"))
    .find((element) => element.getAttribute("data-object-id") === String(objectId));
  return zoneCard
    ? zonePreviewPosition(zoneCard, size)
    : zonePreviewLayout(anchorRect, size);
}

function previewPosition(objectId, size) {
  if (objectId == null || typeof document === "undefined" || typeof window === "undefined") return null;
  const candidates = Array.from(document.querySelectorAll(".game-card[data-object-id], [data-zone-card][data-object-id]"))
    .filter((element) => element.getAttribute("data-object-id") === String(objectId));
  const source = candidates.find((element) => element.classList.contains("battlefield-row-card"))
    || candidates[0];
  if (!source) return null;
  if (source.hasAttribute("data-zone-card")) return zonePreviewPosition(source, size);

  const rect = source.getBoundingClientRect();
  const margin = 8;
  // Battlefield previews may cover the phase band, but the band itself is the
  // hard upper boundary so an inspector never reaches an opponent's zone.
  const minimumTop = phaseToolbarTop(margin);
  const availableHeight = Math.max(0, window.innerHeight - margin - minimumTop);
  const height = Math.min(size.height, availableHeight);
  const width = Math.min(size.width, height * (63 / 88), window.innerWidth - (margin * 2));
  const top = Math.max(
    minimumTop,
    Math.min(window.innerHeight - height - margin, rect.top + (rect.height / 2) - (height / 2))
  );
  const minimumLeft = previewLeftInset({
    top,
    height,
    minimumLeft: margin,
  });
  const maximumLeft = Math.max(minimumLeft, window.innerWidth - width - margin);
  const gap = 14;
  let side = "right";
  let left = rect.right + gap;
  if (left + width > window.innerWidth - margin) {
    side = "left";
    left = rect.left - width - gap;
  }
  const wasClampedPastDecisionButton = left < minimumLeft;
  left = Math.max(minimumLeft, Math.min(maximumLeft, left));
  if (!wasClampedPastDecisionButton) {
    left = snapPreviewToAdjacentCardCenter({
      source,
      left,
      top,
      width,
      height,
      side,
      minimumLeft,
      maximumLeft,
    });
  }
  return {
    left: Math.round(left),
    top: Math.round(top),
    right: "auto",
    maxHeight: `${Math.max(0, Math.floor(availableHeight))}px`,
  };
}

function anchoredPreviewPosition(anchorRect, size) {
  if (!anchorRect || typeof document === "undefined" || typeof window === "undefined") return null;
  const battlefield = document.querySelector(".table-shell[data-drop-zone]");
  const bounds = battlefield?.getBoundingClientRect?.() || {
    left: 0,
    top: 0,
    right: window.innerWidth,
    bottom: window.innerHeight,
  };
  const margin = 8;
  const gap = 10;
  const availableWidth = Math.max(0, bounds.right - bounds.left - (margin * 2));
  const anchorBottom = anchorRect.bottom;
  const top = Math.max(
    bounds.top + margin,
    anchorBottom + gap,
  );
  const availableHeight = Math.max(0, bounds.bottom - margin - top);
  const height = Math.min(size.height, availableHeight);
  const width = Math.min(size.width, height * (63 / 88), availableWidth);
  const maxLeft = Math.max(bounds.left + margin, bounds.right - width - margin);
  const left = Math.min(
    maxLeft,
    previewLeftInset({
      top,
      height,
      minimumLeft: bounds.left + margin,
    })
  );
  return {
    left: Math.round(left),
    top: Math.round(top),
    right: "auto",
    height: `${Math.max(0, Math.floor(height))}px`,
  };
}

export default function FloatingCardPreview({
  disabled: externallyDisabled = false,
  excludedObjectIds = [],
  pinnedObjectId = null,
  onRequestClose = null,
}) {
  const previewSuppressed = useCardPreviewSuppressed();
  const disabled = externallyDisabled || previewSuppressed;
  const { state, dispatch, cancelDecision } = useGame();
  const manaPaymentActions = useMemo(() => manaPaymentActionMap(state), [state]);
  const hoveredObjectId = useHoveredObjectId();
  const anchoredCardPreview = useAnchoredCardPreview();
  const { clearAnchoredCardPreview } = useHoverActions();
  const dragState = useDragState();
  const shellRef = useRef(null);
  const closeTimerRef = useRef(null);
  const [renderedObjectId, setRenderedObjectId] = useState(null);
  const [readyObjectId, setReadyObjectId] = useState(null);
  const onCardFrameReadyChange = useCallback(ready => {
    setReadyObjectId(ready ? renderedObjectId : null);
  }, [renderedObjectId]);
  const [size, setSize] = useState(FALLBACK_SIZE);
  const [accent, setAccent] = useState(null);
  const [previewHovered, setPreviewHovered] = useState(false);
  const excludedIds = useMemo(
    () => new Set(excludedObjectIds.filter((id) => id != null).map(String)),
    [excludedObjectIds]
  );
  const directlyRequestedObjectId = (
    !disabled
    && !dragState
    && !(typeof document !== "undefined" && document.querySelector(".priority-inline-panel:hover"))
    && hoveredObjectId != null
    && !manaPaymentActions.has(Number(hoveredObjectId))
    && canHoverInspectorObject(state, hoveredObjectId)
    && !excludedIds.has(String(hoveredObjectId))
  ) ? String(hoveredObjectId) : null;
  const directHandHover = useMemo(() => {
    if (directlyRequestedObjectId == null) return false;
    return (state?.players || []).some((player) => (
      (player?.hand_cards || []).some((card) => String(card?.id) === directlyRequestedObjectId)
    ));
  }, [directlyRequestedObjectId, state?.players]);
  const directZoneHover = useMemo(() => {
    if (directlyRequestedObjectId == null || typeof document === "undefined") return false;
    return Array.from(document.querySelectorAll("[data-zone-card][data-object-id]")).some((element) => (
      element.getAttribute("data-object-id") === directlyRequestedObjectId
    ));
  }, [directlyRequestedObjectId]);
  // Anchored previews are explicit card-name clicks, so they may inspect a
  // spell on the stack or a card in another zone even though passive hand
  // hovers remain excluded from this surface.
  const anchoredObjectId = !disabled && !dragState && objectExistsInState(state, anchoredCardPreview?.objectId)
    ? String(anchoredCardPreview.objectId)
    : null;
  // Explicit selections bypass passive-hover exclusions. Hand cards stay
  // excluded from hover previews, but clicking one opens this composed,
  // interactive inspector instead of enlarging the card art in place.
  const pinnedPreviewObjectId = !disabled && !dragState && state?.decision?.kind !== "mana_payment" && objectExistsInState(state, pinnedObjectId)
    ? String(pinnedObjectId)
    : null;
  const lockedObjectId = anchoredObjectId || pinnedPreviewObjectId;
  const requestedObjectId = lockedObjectId
    || directlyRequestedObjectId
    || (previewHovered && !disabled && !manaPaymentActions.has(Number(renderedObjectId)) && canHoverInspectorObject(state, renderedObjectId) ? renderedObjectId : null);
  const preparationCard = useMemo(() => {
    const matches = card => card && [card.id, card.inspect_object_id, ...(card.member_ids || [])]
      .some(id => id != null && String(id) === requestedObjectId);
    for (const player of state?.players || []) {
      for (const zone of ['battlefield', 'hand_cards', 'graveyard_cards', 'exile_cards', 'command_cards', 'ante_cards']) {
        const card = (player[zone] || []).find(matches);
        if (card) return card;
      }
    }
    const viewedCard = [
      ...(state?.viewed_cards?.cards || []),
      ...(state?.players || []).flatMap((player) => player?.persistent_look_cards || []),
    ].find(matches);
    if (viewedCard) return viewedCard;
    return getVisibleStackObjects(state).find(matches);
  }, [requestedObjectId, state]);
  const preparationName = preparationCard?.name;
  const preparationType = preparationCard?.type_line;
  const isStackSource = id => id != null && id === lockedObjectId
    && getVisibleStackObjects(state).some(entry => String(entry.id) === id);
  const requestedImageUrl = useDisplayedCardImage(requestedObjectId, isStackSource(requestedObjectId));
  const renderedImageUrl = useDisplayedCardImage(renderedObjectId, isStackSource(renderedObjectId));
  useEffect(() => {
    if (!preparationName) return undefined;
    let active = true;
    // Start asset work immediately, in parallel with the existing hover delay.
    (requestedImageUrl ? Promise.resolve(cardArtCropUrl(requestedImageUrl)) : resolveScryfallImageUrl(preparationName, 'art_crop'))
      .then(url => active ? prepareCardFrame(url, preparationType) : null)
      .catch(() => {});
    return () => { active = false; };
  }, [preparationName, preparationType, requestedImageUrl]);

  const interactiveActions = useMemo(() => {
    if (renderedObjectId == null) return [];
    const decision = state?.decision;
    if (
      decision?.kind !== "priority"
      || !samePlayerId(decision?.player, state?.perspective)
    ) {
      return [];
    }
    const familyIds = objectFamilyIds(state, renderedObjectId);
    return (decision.actions || []).filter((action) => (
      ["activate_ability", "activate_mana_ability", "untap_land"].includes(action?.kind)
      && action?.object_id != null
      && familyIds.has(String(action.object_id))
    ));
  }, [renderedObjectId, state]);

  useEffect(() => {
    if (closeTimerRef.current != null) {
      clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }

    if (requestedObjectId == null) {
      if (renderedObjectId == null) return undefined;
      closeTimerRef.current = window.setTimeout(() => {
        setRenderedObjectId(null);
        closeTimerRef.current = null;
      }, PREVIEW_CLOSE_DELAY_MS);
      return () => {
        if (closeTimerRef.current != null) {
          clearTimeout(closeTimerRef.current);
          closeTimerRef.current = null;
        }
      };
    }

    if (requestedObjectId === renderedObjectId) {
      return undefined;
    }

    // Keep the outgoing card mounted and positioned at its own source until
    // its fade completes. Swapping object content while that transition is
    // running creates a visible flash when moving quickly between cards.
    const openDelay = lockedObjectId != null || directHandHover || directZoneHover ? 0 : PREVIEW_OPEN_DELAY_MS;
    const delay = renderedObjectId == null
      ? openDelay
      : directHandHover || directZoneHover ? 0 : Math.max(openDelay, PREVIEW_FADE_MS);
    closeTimerRef.current = window.setTimeout(() => {
      setRenderedObjectId(requestedObjectId);
      closeTimerRef.current = null;
    }, delay);
    return () => {
      if (closeTimerRef.current != null) {
        clearTimeout(closeTimerRef.current);
        closeTimerRef.current = null;
      }
    };
  }, [directHandHover, directZoneHover, lockedObjectId, renderedObjectId, requestedObjectId]);

  useLayoutEffect(() => {
    const shell = shellRef.current;
    if (!shell) return undefined;
    const measure = () => {
      const rect = shell.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        setSize({ width: Math.round(rect.width), height: Math.round(rect.height) });
      }
    };
    measure();
    const observer = typeof ResizeObserver === "function" ? new ResizeObserver(measure) : null;
    observer?.observe(shell);
    return () => observer?.disconnect();
  }, []);

  const triggerInteractiveAction = (requestedAction) => {
    const decision = state?.decision;
    if (decision?.kind !== "priority") return;
    const liveAction = (decision.actions || []).find((action) => (
      Number(action?.index) === Number(requestedAction?.index)
    ));
    if (!liveAction) return;
    if (liveAction.kind === "untap_land") {
      cancelDecision();
      setPreviewHovered(false);
      setRenderedObjectId(null);
      clearAnchoredCardPreview();
      onRequestClose?.();
      return;
    }
    dispatch(
      { type: "priority_action", action_index: liveAction.index, action_ref: liveAction.action_ref },
      liveAction.label
    );
    setPreviewHovered(false);
    setRenderedObjectId(null);
    clearAnchoredCardPreview();
    onRequestClose?.();
  };

  const stackPreview = renderedObjectId != null && getVisibleStackObjects(state).some((entry) =>
    [entry.id, entry.inspect_object_id].some((id) => id != null && String(id) === String(renderedObjectId))
  );
  const zoneImagePreview = Boolean(renderedImageUrl && (
    directZoneHover || (anchoredObjectId === renderedObjectId && anchoredCardPreview?.placement === "zone")
  ));
  const visible = requestedObjectId != null && renderedObjectId === requestedObjectId
    && (readyObjectId === renderedObjectId || zoneImagePreview);
  const positionStyle = useMemo(
    () => (
      anchoredObjectId != null && renderedObjectId === anchoredObjectId
        ? anchoredCardPreview?.placement === "zone"
          ? zoneAnchoredPreviewPosition(anchoredCardPreview?.anchorRect, anchoredCardPreview?.objectId, size)
          : anchoredPreviewPosition(anchoredCardPreview?.anchorRect, size)
        : previewPosition(renderedObjectId, size)
    ),
    [anchoredCardPreview?.anchorRect, anchoredCardPreview?.objectId, anchoredCardPreview?.placement, anchoredObjectId, renderedObjectId, size]
  );
  const accentStyle = accent
    ? {
      ...playerAccentVars(accent),
      "--card-preview-accent-rgb": accent.rgb,
    }
    : {};

  if (previewSuppressed) return null;

  return (
    <aside
      ref={shellRef}
      className="floating-card-preview"
      data-card-hover-preview="true"
      data-stack-preview={stackPreview ? "true" : "false"}
      data-preview-object-id={renderedObjectId || undefined}
      data-visible={visible ? "true" : "false"}
      data-locked={lockedObjectId != null ? "true" : "false"}
      data-placement={anchoredObjectId != null ? "below-decision" : "near-card"}
      data-interactive={interactiveActions.length > 0 ? "true" : "false"}
      aria-hidden={!visible}
      inert={!visible}
      style={{ ...accentStyle, ...(positionStyle || {}) }}
      onMouseEnter={() => setPreviewHovered(true)}
      onMouseLeave={() => setPreviewHovered(false)}
    >
      {renderedObjectId != null ? (
        <HoverArtOverlay
          key={renderedObjectId}
          objectId={renderedObjectId}
          selectedStackEntry={String(pinnedObjectId) === String(renderedObjectId)
            ? getVisibleStackObjects(state).find(entry => String(entry.id) === String(pinnedObjectId))
            : null}
          displayMode="card-frame"
          sourceImageUrl={renderedImageUrl}
          availableInspectorWidth={size.width}
          availableInspectorHeight={size.height}
          onInspectorAccentChange={setAccent}
          onCardFrameReadyChange={onCardFrameReadyChange}
          interactiveActions={interactiveActions}
          onInteractiveAction={triggerInteractiveAction}
        />
      ) : null}
      {visible && zoneImagePreview && readyObjectId !== renderedObjectId ? (
        <img
          src={renderedImageUrl}
          alt={preparationName || "Card preview"}
          referrerPolicy="no-referrer"
          className="absolute inset-0 z-40 h-full w-full object-contain pointer-events-none"
        />
      ) : null}
    </aside>
  );
}
