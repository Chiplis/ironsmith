import { zonePreviewLayout as placeZonePreview } from "@/lib/zone-preview-layout";
import { classifyViewport } from "@/lib/viewport-layout";
import { placeCardPreview } from "@/lib/card-preview-placement";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { manaPaymentActionMap, manaPaymentFrameActions, manaActivationCommand } from "@/lib/mana-payment-actions";
import { useManaPaymentEditor } from "@/context/ManaPaymentEditorContext.shared";
import { useCastTargeting, useDragSession } from "@/context/DragContext";
import { useGame } from "@/context/GameContext";
import {
  useAnchoredCardPreview,
  useCardPreviewSuppressed,
  useHoverActions,
  useHoveredObjectId,
} from "@/context/HoverContext";
import HoverArtOverlay from "./HoverArtOverlay";
import FaceDownIdentityFrame from "./FaceDownIdentityFrame";
import { knownFaceDownNames } from "@/lib/look-pile";
import useDisplayedCardImage from "@/hooks/useDisplayedCardImage";
import { playerAccentVars } from "@/lib/player-colors";
import { samePlayerId } from "@/lib/player-display";
import { getVisibleStackObjects } from "@/lib/stack-targets";
import { canHoverInspectorObject, objectExistsInState, resolveStackInspectObjectId } from "@/lib/inspector-selection";

import { requestInspectorDetails } from "@/lib/inspector-details-cache";
import { stackFramePreview, visibleFrameCards } from "@/lib/stack-frame-preview";
import useCardStackMode from "@/hooks/useCardStackMode";
import { ComicTooltipHostVisible } from "@/components/ui/comic-tooltip";

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

// Include every expanded pile, not just the hovered card's source zone.
function previewObstacleBounds() {
  return [...document.querySelectorAll("[data-decision-menu], .priority-inline-panel, .zone-pile-menu, .battlefield-human-action-dock, .desktop-mana-payment, .decision-header-utility-controls, .topbar-main-decision-host, .mobile-decision-actions, .decision-main-button, .decision-submit-button, .decision-cancel-button, .combat-decision-actions, .mana-payment-editor, .mana-payment-warning-details")]
    .filter(element => getComputedStyle(element).visibility !== "hidden")
    .map(element => element.getBoundingClientRect())
    .filter(rect => rect.width > 0 && rect.height > 0);
}

function zonePreviewLayout(anchorRect, size, source = null) {
  return placeZonePreview({
    anchor: source?.closest?.(".zone-pile-menu")?.getBoundingClientRect() ?? anchorRect,
    viewportWidth: window.innerWidth,
    viewportHeight: window.innerHeight,
    minimumTop: phaseToolbarTop(8),
    height: Math.max(size.height, FALLBACK_SIZE.height),
    obstacles: previewObstacleBounds(),
  });
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

function decisionPreviewSource(objectId) {
  if (objectId == null || typeof document === "undefined") return null;
  return [...document.querySelectorAll("[data-decision-option-object]:hover, [data-decision-option-object]:focus-visible")]
    .find(element => element.dataset.decisionOptionObject === String(objectId));
}

function previewPosition(objectId, size, frames = 1) {
  if (objectId == null || typeof document === "undefined" || typeof window === "undefined") return null;
  const decisionSource = decisionPreviewSource(objectId);
  if (decisionSource) return battlefieldPreviewLayout(decisionSource.getBoundingClientRect(), size, frames);
  const candidates = Array.from(document.querySelectorAll(".game-card[data-object-id], [data-zone-card][data-object-id], .opponent-hand-card[data-object-id]"))
    .filter((element) => element.getAttribute("data-object-id") === String(objectId));
  const source = candidates.find((element) => element.classList.contains("battlefield-row-card"))
    || candidates[0];
  if (!source) return null;
  if (source.hasAttribute("data-zone-card")) return zonePreviewPosition(source, size);
  if (source.classList.contains("opponent-hand-card")) {
    // An opponent's held card mostly hangs above the table edge; place the
    // preview against the sliver that is on screen.
    const rect = source.getBoundingClientRect();
    const top = Math.max(0, rect.top);
    return battlefieldPreviewLayout({ left: rect.left, right: rect.right, width: rect.width,
      top, bottom: rect.bottom, height: Math.max(0, rect.bottom - top) }, size);
  }

  return battlefieldPreviewLayout(source.getBoundingClientRect(), size, frames);
}

function battlefieldPreviewLayout(rect, size, frames = 1) {
  const margin = 8;
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
  const gap = 8;
  let left = rect.right + gap;
  if (left + width > window.innerWidth - margin) {
    left = rect.left - width - gap;
  }
  left = Math.max(minimumLeft, Math.min(maximumLeft, left));
  return placeCardPreview({
    preferred: { left, top }, height,
    bounds: { left: margin, right: window.innerWidth - margin, top: minimumTop, bottom: window.innerHeight - margin },
    obstacles: [...previewObstacleBounds(), rect], frames,
  });
}

function fullCardsHud() {
  return document.querySelector('.table-shell[data-focused-hud="true"] .battlefield-row[data-battlefield-style="full"]') != null;
}

// The stack frame grows down to whatever sits under the stack column: the
// local mana row in the compact HUD, the local player bar with full cards
// (whose mana is a vertical rail beside the column instead).
function stackFrameFloor() {
  const table = document.querySelector('.table-shell[data-focused-hud="true"]');
  if (!table) return null;
  const bounds = table.querySelector(fullCardsHud() ? '.table-shared-player-header' : '.local-player-mana-dock')
    ?.getBoundingClientRect();
  return bounds?.width > 0 ? bounds : null;
}

// Both desktop layouts place the frame beside the stack column, top-aligned
// with the rail so expanding an entry never moves or covers its preview.
function stackAnchoredPreviewPosition(anchorRect, size) {
  if (!anchorRect || typeof window === "undefined") return null;
  const margin = 8, gap = 8;
  const rail = !classifyViewport(window.innerWidth, window.innerHeight).nonDesktopViewport
    ? document.querySelector('[data-my-zone] .my-zone-stack-rail') : null;
  const railBounds = rail?.getBoundingClientRect();
  if (railBounds?.width > 0) {
    const top = Math.max(margin, railBounds.top);
    // Frames may cover the phase track and extend into either player's board.
    // Keep the hand and player bar clear.
    const handTop = [...document.querySelectorAll('.hand-card')]
      .map(card => card.getBoundingClientRect()).filter(rect => rect.width > 0)
      .reduce((highest, rect) => Math.min(highest, rect.top), Infinity);
    const bottom = Math.min(stackFrameFloor()?.top ?? window.innerHeight, handTop) - margin;
    return placeCardPreview({
      preferred: { left: railBounds.right + 12, top }, height: size.height,
      bounds: { left: margin, right: window.innerWidth - margin, top: phaseToolbarTop(margin), bottom },
      obstacles: [...previewObstacleBounds(), railBounds],
    });
  }
  const minimumTop = phaseToolbarTop(margin);
  const availableHeight = Math.max(0, window.innerHeight - margin - minimumTop);
  const rightSpace = Math.max(0, window.innerWidth - margin - anchorRect.right - gap);
  const leftSpace = Math.max(0, anchorRect.left - gap - margin);
  const desiredWidth = Math.min(size.width, availableHeight * 63 / 88);
  const placeRight = rightSpace >= desiredWidth || rightSpace >= leftSpace;
  const availableWidth = placeRight ? rightSpace : leftSpace;
  const maximumHeight = Math.min(availableHeight, availableWidth * 88 / 63);
  const height = Math.min(size.height, maximumHeight);
  const width = Math.min(size.width, height * 63 / 88);
  return placeCardPreview({
    preferred: {
      left: placeRight ? anchorRect.right + gap : anchorRect.left - gap - width,
      top: Math.max(minimumTop, Math.min(anchorRect.top, window.innerHeight - margin - height)),
    },
    height: Math.min(size.height, availableHeight),
    bounds: { left: margin, right: window.innerWidth - margin, top: minimumTop, bottom: window.innerHeight - margin },
    obstacles: [...previewObstacleBounds(), { ...anchorRect, top: minimumTop, bottom: window.innerHeight }],
  });
}

function anchoredPreviewPosition(anchorRect, size, frames = 1) {
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
  const minimumTop = Math.max(bounds.top + margin, phaseToolbarTop(margin));
  const availableHeight = Math.max(0, bounds.bottom - margin - minimumTop);
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
  return placeCardPreview({
    preferred: { left, top }, height,
    bounds: { left: bounds.left + margin, right: bounds.right - margin, top: minimumTop, bottom: bounds.bottom - margin },
    obstacles: [...previewObstacleBounds(), anchorRect], frames,
  });
}

export default function FloatingCardPreview(props) {
  // Keep stack frames mounted while a separate surface inspects field cards.
  return <>
    <CardPreviewSurface {...props} automaticOnly />
    {!props.automaticOnly && <CardPreviewSurface {...props} automaticOnly={false} />}
  </>;
}

function CardPreviewSurface({
  disabled: externallyDisabled = false,
  automaticOnly = false,
  excludedObjectIds = [],
  pinnedObjectId = null,
  onRequestClose = null,
}) {
  const previewSuppressed = useCardPreviewSuppressed();
  const disabled = externallyDisabled || previewSuppressed;
  const { game, state, dispatch, cancelDecision, cancelBackgroundDispatch } = useGame();
  const [rememberedCards, setRememberedCards] = useState([]);
  // The desktop layouts’ card pile already shows every stack entry at
  // full frame size: no second frame opens for one.
  const cardStackMode = useCardStackMode();
  const automaticPreview = useMemo(
    () => (!automaticOnly || cardStackMode ? null : stackFramePreview(state, rememberedCards)),
    [automaticOnly, cardStackMode, state, rememberedCards],
  );
  useEffect(() => {
    if (!automaticOnly) return;
    // Keep public snapshots only, including sources that disappear on resolution.
    queueMicrotask(() => setRememberedCards(previous => {
      const cards = [...visibleFrameCards(state), ...previous];
      const seen = new Set();
      return cards.filter(card => {
        const key = card.stable_id != null ? `stable:${card.stable_id}` : `object:${card.id}`;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      }).slice(0, 512);
    }));
  }, [automaticOnly, state]);
  useEffect(() => {
    if (!automaticOnly) return undefined;
    const preview = stackFramePreview(state);
    const id = preview && resolveStackInspectObjectId(state, preview.entry);
    if (!game || id == null) return undefined;
    let active = true;
    // Capture full spell/source details while they still exist on the stack.
    // This shares the inspector request, so there is no extra engine query.
    requestInspectorDetails(game, state, id).then(details => {
      if (!active || !details) return;
      const card = { ...details, id, stable_id: preview.entry.source_stable_id ?? preview.entry.stable_id ?? details.stable_id };
      setRememberedCards(previous => [card, ...previous].slice(0, 512));
    }).catch(() => {});
    return () => { active = false; };
  }, [automaticOnly, game, state]);
  const paymentEditor = useManaPaymentEditor();
  const manaPaymentActions = useMemo(() => manaPaymentActionMap(state), [state]);
  const hoveredObjectId = useHoveredObjectId();
  const anchoredCardPreview = useAnchoredCardPreview();
  const { clearAnchoredCardPreview, cancelAnchoredCardPreviewClear } = useHoverActions();
  const dragState = useDragSession();
  const castIntent = useCastTargeting();
  const targetingMode = Boolean(castIntent) || state?.decision?.kind === "targets";
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
  const isFieldObject = useCallback(id => (state?.players || []).some(player =>
    (player.battlefield || []).some(card => String(card.id) === String(id))), [state?.players]);
  const directlyRequestedObjectId = (
    !disabled
    && !automaticOnly
    && !dragState
    // The priority panel suppresses previews that would cover it -- except for
    // an option that stands for an object, where seeing the card is the point.
    && !(
      typeof document !== "undefined"
      && document.querySelector(".priority-inline-panel:hover")
      && !document.querySelector("[data-decision-option-object]:hover")
    )
    && !(cardStackMode && typeof document !== "undefined" && document.querySelector(".stack-card:hover"))
    && hoveredObjectId != null
    && !manaPaymentActions.has(Number(hoveredObjectId))
    && (canHoverInspectorObject(state, hoveredObjectId)
      || (decisionPreviewSource(hoveredObjectId) && objectExistsInState(state, hoveredObjectId)))
    && (isFieldObject(hoveredObjectId) || !excludedIds.has(String(hoveredObjectId)))
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
  const directLookHover = directlyRequestedObjectId != null && typeof document !== "undefined"
    && !classifyViewport(window.innerWidth, window.innerHeight).nonDesktopViewport
    && [...document.querySelectorAll('[data-zone-card="look"][data-object-id]')]
      .some(element => element.dataset.objectId === directlyRequestedObjectId);
  // Anchored previews are explicit card-name clicks, so they may inspect a
  // spell on the stack or a card in another zone even though passive hand
  // hovers remain excluded from this surface.
  const anchoredObjectId = !automaticOnly && !disabled && !dragState
    && !(cardStackMode && anchoredCardPreview?.placement === "stack")
    && objectExistsInState(state, anchoredCardPreview?.objectId)
    ? String(anchoredCardPreview.objectId)
    : null;
  // Explicit selections bypass passive-hover exclusions. Hand cards stay
  // excluded from hover previews, but clicking one opens this composed,
  // interactive inspector instead of enlarging the card art in place.
  const pinnedPreviewObjectId = !automaticOnly && !disabled
    && !dragState
    && !targetingMode
    && state?.decision?.kind !== "mana_payment"
    && !(cardStackMode && !isFieldObject(pinnedObjectId) && getVisibleStackObjects(state).some((entry) => String(entry.id) === String(pinnedObjectId)))
    && objectExistsInState(state, pinnedObjectId)
    ? String(pinnedObjectId)
    : null;
  // Include stack depth so consecutive copies/triggers with shared presentation
  // ids can open independently, without reopening on ordinary state refreshes.
  const automaticPreviewKey = automaticPreview ? JSON.stringify([
    automaticPreview.entry.id,
    automaticPreview.entry.source_stable_id ?? automaticPreview.entry.stable_id,
    automaticPreview.entry.ability_text ?? automaticPreview.entry.effect_text,
    getVisibleStackObjects(state).length,
  ]) : null;
  const [dismissedAutomaticKey, setDismissedAutomaticKey] = useState(null);
  useEffect(() => {
    queueMicrotask(() => setDismissedAutomaticKey(null));
  }, [automaticPreviewKey]);
  useEffect(() => {
    if (automaticPreviewKey == null) return undefined;
    const dismissOnEmptySpace = event => {
      if (event.button !== 0 || !(event.target instanceof Element)) return;
      if (event.target.closest("[data-object-id], .zone-viewer, [data-card-inspector], .ironsmith-inspector-shell, .floating-card-preview, button, input, select, textarea, label, a, [role='button'], [role='dialog']")) return;
      setDismissedAutomaticKey(automaticPreviewKey);
      setPreviewHovered(false);
      setRenderedObjectId(null);
      clearAnchoredCardPreview();
      onRequestClose?.();
    };
    document.addEventListener("pointerdown", dismissOnEmptySpace, true);
    return () => document.removeEventListener("pointerdown", dismissOnEmptySpace, true);
  }, [automaticPreviewKey, clearAnchoredCardPreview, onRequestClose]);
  const automaticObjectId = !disabled && !dragState && automaticPreview
    && dismissedAutomaticKey !== automaticPreviewKey
    ? String(automaticPreview.entry.id) : null;
  const fieldHoverOverStack = directlyRequestedObjectId != null && isFieldObject(directlyRequestedObjectId)
    && typeof document !== "undefined"
    && [...document.querySelectorAll(".game-card[data-object-id]:hover")]
      .some(element => element.dataset.objectId === directlyRequestedObjectId)
    && (anchoredCardPreview?.placement === "stack"
      || (pinnedPreviewObjectId != null && !isFieldObject(pinnedPreviewObjectId)));
  const lockedObjectId = directLookHover || fieldHoverOverStack ? null : ((automaticOnly ? null : anchoredObjectId || pinnedPreviewObjectId)
    || automaticObjectId);
  const requestedObjectId = lockedObjectId
    || directlyRequestedObjectId
    // In target mode the preview is only a response to the source card's
    // hover. Do not let entering the enlarged frame keep it alive after the
    // source card has been left; the target highlight remains on the card.
    || (!targetingMode && previewHovered && !disabled && !manaPaymentActions.has(Number(renderedObjectId)) && canHoverInspectorObject(state, renderedObjectId) ? renderedObjectId : null);
  // A stack entry's id is minted from its source's object id (x2, +1 for an
  // ability), so it is routinely the id of some unrelated card as well: the
  // Wheel of Torture trigger can carry the number an Island has. A preview a
  // stack tile asked for (anchored or pinned) therefore resolves to the stack
  // entry before any zone is searched.
  const lockedStackEntry = useMemo(() => (
    lockedObjectId == null || (isFieldObject(lockedObjectId) && lockedObjectId !== automaticObjectId && anchoredCardPreview?.placement !== "stack")
      ? null
      : (lockedObjectId === automaticObjectId ? automaticPreview?.entry : null)
        || getVisibleStackObjects(state).find(entry => String(entry.id) === lockedObjectId) || null
  ), [lockedObjectId, automaticObjectId, automaticPreview?.entry, state, isFieldObject, anchoredCardPreview?.placement]);
  const isStackSource = id => id != null && lockedStackEntry != null && id === lockedObjectId;
  const renderedImageUrl = useDisplayedCardImage(renderedObjectId, isStackSource(renderedObjectId));

  const interactiveActions = useMemo(() => {
    if (renderedObjectId == null) return [];
    const decision = state?.decision;
    const familyIds = objectFamilyIds(state, renderedObjectId);
    if (decision?.kind === "mana_payment") {
      return manaPaymentFrameActions(state, familyIds);
    }
    if (
      decision?.kind !== "priority"
      || !samePlayerId(decision?.player, state?.perspective)
    ) {
      return [];
    }
    return (decision.actions || []).filter((action) => (
      ["activate_ability", "activate_mana_ability", "untap_land", "turn_face_up"].includes(action?.kind)
      && action?.object_id != null
      && familyIds.has(String(action.object_id))
    ));
  }, [renderedObjectId, state]);
  // A face-down permanent this viewer can identify shows its front face too.
  const faceDownIdentity = useMemo(() => {
    if (renderedObjectId == null || isStackSource(renderedObjectId)) return null;
    const faceDown = (state?.players || []).some(player => (player?.battlefield || [])
      .some(card => String(card?.id) === String(renderedObjectId) && card?.face_down));
    return faceDown ? knownFaceDownNames(state).get(String(renderedObjectId)) || null : null;
  }, [renderedObjectId, state, lockedStackEntry, lockedObjectId]);
  const previewFrames = faceDownIdentity ? 2 : 1;

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
      const saved = shell.style.cssText;
      shell.style.removeProperty("height");
      shell.style.removeProperty("width");
      shell.style.removeProperty("max-height");
      const style = getComputedStyle(shell);
      const width = Number.parseFloat(style.width), height = Number.parseFloat(style.height);
      shell.style.cssText = saved;
      if (width > 0 && height > 0) {
        setSize(previous => previous.width === width && previous.height === height ? previous : { width, height });
      }
    };
    measure();
    window.addEventListener("resize", measure);
    return () => window.removeEventListener("resize", measure);
  }, []);

  const triggerInteractiveAction = (requestedAction) => {
    const decision = state?.decision;
    if (decision?.kind === "mana_payment") {
      if (paymentEditor) {
        void paymentEditor.activate(requestedAction);
      } else {
        cancelBackgroundDispatch?.();
        dispatch(manaActivationCommand(requestedAction), requestedAction.label, { waitForPaymentReady: true });
      }
      return;
    }
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

  const stackPreview = isStackSource(renderedObjectId);
  const visible = requestedObjectId != null && renderedObjectId === requestedObjectId
    && readyObjectId === renderedObjectId;
  const [stackAnchorRect, setStackAnchorRect] = useState(null);
  useLayoutEffect(() => {
    if (!stackPreview) return undefined;
    let frame;
    const measure = () => {
      const rect = [...document.querySelectorAll('[data-stack-preview-anchor]')]
        .map(element => element.getBoundingClientRect())
        .find(bounds => bounds.width > 0 && bounds.height > 0);
      const topEntry = document.querySelector('[data-my-zone] .my-zone-stack-rail .stack-timeline-entry:not([data-leaving="true"])')?.getBoundingClientRect();
      const table = document.querySelector('.table-shell[data-focused-hud="true"]');
      const floor = stackFrameFloor();
      const rail = table?.querySelector('.my-zone-stack-rail')?.getBoundingClientRect();
      if (floor && rail?.width && topEntry) {
        const width = Math.max(0, Math.min(rail.width, (floor.top - 12 - topEntry.bottom) * 63 / 88));
        const nextWidth = `${Math.max(0, Math.floor(width) - 1)}px`;
        if (table.style.getPropertyValue('--stack-inspector-width') !== nextWidth) table.style.setProperty('--stack-inspector-width', nextWidth);
      }
      const next = rect ? { left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom, entryBottom: topEntry?.bottom } : null;
      setStackAnchorRect(previous => JSON.stringify(previous) === JSON.stringify(next) ? previous : next);
      frame = requestAnimationFrame(measure);
    };
    frame = requestAnimationFrame(measure);
    return () => cancelAnimationFrame(frame);
  }, [stackPreview]);
  // Collision adjustment and scrolling can move a popover without resizing it.
  const [, setZoneBounds] = useState(null);
  useLayoutEffect(() => {
    if (renderedObjectId == null) return undefined;
    let frame;
    const measure = () => {
      const source = [...document.querySelectorAll("[data-zone-card][data-object-id]")]
        .find(element => element.dataset.objectId === String(renderedObjectId));
      const rect = (source?.closest(".zone-pile-menu") ?? source)?.getBoundingClientRect();
      const next = [rect?.left, rect?.top, rect?.right, rect?.bottom, window.innerWidth, window.innerHeight, ...previewObstacleBounds().flatMap(r => [r.left, r.top, r.right, r.bottom])].join(",");
      setZoneBounds(previous => previous === next ? previous : next);
      frame = requestAnimationFrame(measure);
    };
    measure();
    return () => cancelAnimationFrame(frame);
  }, [renderedObjectId, stackPreview]);
  const positionStyle = (() => {
      const decisionSource = decisionPreviewSource(renderedObjectId);
      if (decisionSource) return battlefieldPreviewLayout(decisionSource.getBoundingClientRect(), size, previewFrames);
      if (anchoredObjectId != null && renderedObjectId === anchoredObjectId) {
        if (anchoredCardPreview?.placement === "zone") {
          return zoneAnchoredPreviewPosition(anchoredCardPreview?.anchorRect, anchoredCardPreview?.objectId, size);
        }
        if (anchoredCardPreview?.placement === "stack") {
          return stackAnchoredPreviewPosition(anchoredCardPreview?.anchorRect, size);
        }
        return anchoredPreviewPosition(anchoredCardPreview?.anchorRect, size, previewFrames);
      }
      // A stack preview locked in by a click keeps the place the hover preview
      // held, so committing to an entry never makes the frame jump.
      if (stackPreview && typeof document !== "undefined") {
        return stackAnchoredPreviewPosition(stackAnchorRect, size);
      }
      return previewPosition(renderedObjectId, size, previewFrames);
    })();
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
      data-automatic-stack-preview={lockedObjectId === automaticObjectId && automaticObjectId != null ? "true" : undefined}
      data-stack-preview={stackPreview ? "true" : "false"}
      data-preview-object-id={renderedObjectId || undefined}
      data-visible={visible ? "true" : "false"}
      data-locked={lockedObjectId != null ? "true" : "false"}
      data-placement={anchoredCardPreview?.placement === "stack" && anchoredObjectId != null
        ? "beside-stack"
        : anchoredObjectId != null
          ? "below-decision"
          : stackPreview
            ? "beside-stack"
            : "near-card"}
      data-interactive={interactiveActions.length > 0 ? "true" : "false"}
      data-frames={previewFrames > 1 ? previewFrames : undefined}
      aria-hidden={!visible}
      inert={!visible}
      // Closing is a fade, and the position keeps being recomputed underneath
      // it. Once there is nothing left to anchor to, previewPosition returns
      // null and the stylesheet's default corner takes over -- the frame jumps
      // across the screen for the few frames it is still on its way out. With
      // no place to be, it is not shown at all.
      style={{ ...accentStyle, ...(positionStyle || { visibility: "hidden" }), transform: "none" }}
      onMouseEnter={() => {
        // Always call off a pending close, targeting or not: reaching the frame
        // is exactly what the grace period was held open for.
        cancelAnchoredCardPreviewClear();
        if (!targetingMode) setPreviewHovered(true);
      }}
      onMouseLeave={() => {
        setPreviewHovered(false);
        if (anchoredObjectId != null) clearAnchoredCardPreview();
      }}
    >
      <ComicTooltipHostVisible.Provider value={visible}>
      {renderedObjectId != null && faceDownIdentity ? (
        <div className="floating-card-preview__frames">
          <div className="floating-card-preview__frame">
            <HoverArtOverlay
              key={renderedObjectId}
              objectId={renderedObjectId}
              displayMode="card-frame"
              sourceImageUrl={renderedImageUrl}
              availableInspectorWidth={size.width}
              availableInspectorHeight={size.height}
              onInspectorAccentChange={setAccent}
              onCardFrameReadyChange={onCardFrameReadyChange}
              interactiveActions={interactiveActions}
              onInteractiveAction={triggerInteractiveAction}
            />
          </div>
          <div className="floating-card-preview__frame">
            <FaceDownIdentityFrame name={faceDownIdentity} />
          </div>
        </div>
      ) : renderedObjectId != null ? (
        <HoverArtOverlay
          key={renderedObjectId}
          objectId={renderedObjectId}
          selectedStackEntry={isStackSource(renderedObjectId) ? lockedStackEntry : null}
          transientPreview={isStackSource(renderedObjectId) && lockedStackEntry === automaticPreview?.entry
            ? { card: automaticPreview.card } : null}
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
      </ComicTooltipHostVisible.Provider>
    </aside>
  );
}
