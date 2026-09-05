import { Fragment, useRef, useMemo, useEffect, useLayoutEffect, useCallback, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useHover } from "@/context/HoverContext";
import { useDragActions, useDragState } from "@/context/DragContext";
import useNewCards from "@/hooks/useNewCards";
import useManabrewHandScale, {
  MANABREW_HAND_CARD_BASE,
  MANABREW_HAND_FAN_PARAMS,
} from "@/hooks/useManabrewHandScale";
import GameCard from "@/components/cards/GameCard";
import useHandReflow from "@/hooks/useHandReflow";
import { samePlayerId } from "@/lib/player-display";
import { handCardSourcePoint, plainRect } from "@/lib/hand-drag-intent";

const HAND_ROULETTE_THRESHOLD = 10;
const HAND_ROULETTE_VISIBLE_CARDS = 7;
const HAND_ROULETTE_EDGE_PADDING = 12;
const HAND_ROULETTE_WRAP_GAP = 20;
const HAND_ROULETTE_CYCLE_COUNT = 3;
const HAND_ROULETTE_CENTER_CYCLE = 1;
const HAND_SELECTED_LAYOUT_Z_INDEX = 20000;
const HAND_DRAG_START_DISTANCE_SQ = 14 * 14;
const DRAW_TO_HAND_REVEAL_DELAY_MS = 1080;
// Keep in sync with the card-flight stagger in GameEffectAnimations.
const DRAW_TO_HAND_REVEAL_STAGGER_MS = 150;
const HAND_ACTION_HOVER_EVENT = "ironsmith:hand-action-hover";

/** Map card_types array to a glow kind for hand display. */
function handGlowFromTypes(cardTypes) {
  if (!cardTypes || cardTypes.length === 0) return "spell";
  // Priority order: more specific types win
  if (cardTypes.includes("instant")) return "instant";
  if (cardTypes.includes("sorcery")) return "sorcery";
  if (cardTypes.includes("creature")) return "creature";
  if (cardTypes.includes("enchantment")) return "enchantment";
  if (cardTypes.includes("battle")) return "battle";
  if (cardTypes.includes("planeswalker")) return "planeswalker";
  if (cardTypes.includes("artifact")) return "artifact";
  if (cardTypes.includes("land")) return "land";
  return "spell";
}

/**
 * Build a map of objectId → actions for all interactable hand cards.
 * Also builds a list of "extra" pseudo-hand cards from non-hand zones.
 */
function buildPlayableMaps(state, player) {
  const handPlayable = new Map();   // objectId → actions[] (from hand)
  const extraPlayable = new Map();  // objectId → { name, card, actions[], fromZone, glowKind }

  const actions =
    state?.decision?.kind === "priority"
    && samePlayerId(state?.decision?.player, state?.perspective)
    && Array.isArray(state?.decision?.actions)
      ? state.decision.actions
      : [];

  const handIds = new Set((player?.hand_cards || []).map((c) => Number(c.id)));

  // Build zone lookup for cards across all visible zones so pseudo-hand can
  // surface cross-owner play-from cards such as Ragavan hits in exile.
  const cardNameById = new Map();
  const cardSnapshotById = new Map();
  const addZoneCards = (cards) => {
    for (const card of cards || []) {
      const objId = Number(card?.id);
      if (!Number.isFinite(objId)) continue;
      cardNameById.set(objId, card.name);
      cardSnapshotById.set(objId, card);
    }
  };
  for (const snapshotPlayer of state?.players || []) {
    addZoneCards(snapshotPlayer?.graveyard_cards);
    addZoneCards(snapshotPlayer?.exile_cards);
    addZoneCards(snapshotPlayer?.command_cards);
    addZoneCards(snapshotPlayer?.ante_cards);
    addZoneCards(snapshotPlayer?.hand_cards);
  }

  for (const action of actions) {
    if (action.object_id == null) {
      continue;
    }

    const objId = Number(action.object_id);
    const isHandCard = handIds.has(objId);
    const isHandInteraction =
      action.kind === "cast_spell"
      || action.kind === "play_land"
      || action.kind === "special_action"
      || action.kind === "activate_ability"
      || action.kind === "activate_mana_ability"
      || action.kind === "serum_powder_mulligan"
      || action.kind === "begin_with_gemstone_caverns";

    if (isHandCard && isHandInteraction) {
      if (!handPlayable.has(objId)) handPlayable.set(objId, []);
      handPlayable.get(objId).push(action);
      continue;
    }

    // Card from another zone (graveyard flashback, exile, etc.)
    // Keep this list focused on cast/play actions so battlefield activations
    // don't show up as extra pseudo-hand cards.
    if (action.kind === "cast_spell" || action.kind === "play_land") {
      if (!extraPlayable.has(objId)) {
        const card = cardSnapshotById.get(objId);
        extraPlayable.set(objId, {
          name: cardNameById.get(objId) || action.label?.replace(/^(Cast|Play)\s+/i, "") || `Card ${objId}`,
          card: card || { id: objId, name: cardNameById.get(objId) || `Card ${objId}` },
          actions: [],
          fromZone: action.from_zone || "other",
          glowKind: card?.pseudo_hand_glow_kind || "extra",
        });
      }
      extraPlayable.get(objId).actions.push(action);
    }
  }

  // Surface non-hand cards that currently have permission to be played/cast
  // from their zone, even if they aren't payable right now.
  const addPseudoHandCandidates = (cards, fromZone) => {
    for (const card of cards || []) {
      if (!card?.show_in_pseudo_hand) continue;
      const objId = Number(card.id);
      if (!Number.isFinite(objId) || handIds.has(objId)) continue;
      if (!extraPlayable.has(objId)) {
        extraPlayable.set(objId, {
          name: card.name || cardNameById.get(objId) || `Card ${objId}`,
          card,
          actions: [],
          fromZone,
          glowKind: card.pseudo_hand_glow_kind || "extra",
        });
      }
    }
  };

  for (const snapshotPlayer of state?.players || []) {
    addPseudoHandCandidates(snapshotPlayer?.graveyard_cards, "graveyard");
    addPseudoHandCandidates(snapshotPlayer?.exile_cards, "exile");
    addPseudoHandCandidates(snapshotPlayer?.command_cards, "command");
    addPseudoHandCandidates(snapshotPlayer?.ante_cards, "ante");
  }

  return { handPlayable, extraPlayable };
}

function computeManabrewHandDimensions(scale) {
  return {
    cardW: Math.round(MANABREW_HAND_CARD_BASE.cardW * scale),
    cardH: Math.round(MANABREW_HAND_CARD_BASE.cardH * scale),
    hoverLift: Math.round(MANABREW_HAND_FAN_PARAMS.hoverLift * scale),
    neighborPush: Math.round(MANABREW_HAND_FAN_PARAMS.neighborPush * scale),
    maxSpread: Math.round(MANABREW_HAND_FAN_PARAMS.maxSpread * scale),
    minSpread: Math.round(MANABREW_HAND_FAN_PARAMS.minSpread * scale),
    spreadWidth: Math.round(MANABREW_HAND_FAN_PARAMS.spreadWidth * scale),
  };
}

function computeManabrewSpread(total, dims) {
  if (total <= 1) return 0;
  return Math.max(
    dims.minSpread,
    Math.min(dims.maxSpread, Math.floor((dims.spreadWidth - dims.cardW) / (total - 1)))
  );
}

function computeManabrewBaseLayout(total, dims) {
  if (total === 0) return [];
  if (total === 1) return [{ x: 0, drop: 0, rot: 0 }];

  const spread = computeManabrewSpread(total, dims);
  const totalWidth = (total - 1) * spread;
  const arcDeg = Math.min(MANABREW_HAND_FAN_PARAMS.maxArcDeg, total * 2.5);

  return Array.from({ length: total }, (_, index) => {
    const t = (index / (total - 1)) * 2 - 1;
    return {
      x: -totalWidth / 2 + index * spread,
      drop: (1 - Math.cos((t * Math.PI) / 2)) * (MANABREW_HAND_FAN_PARAMS.arcRadius * 0.015),
      rot: t * (arcDeg / 2),
    };
  });
}

function computeRouletteWidth(total, dims) {
  const visibleCards = Math.min(HAND_ROULETTE_VISIBLE_CARDS, total);
  const stride = computeManabrewSpread(total, dims);
  return Math.round(
    dims.cardW
    + Math.max(0, visibleCards - 1) * stride
    + (HAND_ROULETTE_EDGE_PADDING * 2 * (dims.cardW / MANABREW_HAND_CARD_BASE.cardW))
  );
}

function computeHandRowWidth(total, dims) {
  if (total <= 0) return 0;
  const stride = computeManabrewSpread(total, dims);
  return Math.round(dims.cardW + Math.max(0, total - 1) * stride);
}

function buildHandCardRowStyle(index, total, { dims, activeIndex = null } = {}) {
  const spread = computeManabrewSpread(total, dims);
  const baseLayout = computeManabrewBaseLayout(total, dims);
  const base = baseLayout[index] || { drop: 0, rot: 0 };
  const isActive = activeIndex === index;

  let pushX = 0;
  if (activeIndex !== null && activeIndex >= 0 && index !== activeIndex) {
    const distance = Math.abs(index - activeIndex);
    const sign = index < activeIndex ? -1 : 1;
    pushX = sign * Math.max(0, dims.neighborPush - distance * 6);
  }

  const fanRotate = isActive ? "0deg" : `${base.rot.toFixed(2)}deg`;
  const fanTranslateX = `${pushX.toFixed(1)}px`;
  const fanTranslateY = isActive
    ? `${(-dims.hoverLift).toFixed(1)}px`
    : `${base.drop.toFixed(1)}px`;
  const cardScale = isActive ? MANABREW_HAND_FAN_PARAMS.hoverScale : 1;

  return {
    flex: `0 0 ${dims.cardW}px`,
    width: `${dims.cardW}px`,
    minWidth: `${dims.cardW}px`,
    maxWidth: `${dims.cardW}px`,
    height: `${dims.cardH}px`,
    minHeight: `${dims.cardH}px`,
    maxHeight: `${dims.cardH}px`,
    marginLeft: index === 0 ? "0px" : `${(spread - dims.cardW).toFixed(1)}px`,
    zIndex: isActive ? HAND_SELECTED_LAYOUT_Z_INDEX : index + 2,
    "--card-rotate": fanRotate,
    "--card-translate-x": fanTranslateX,
    "--card-translate-y": fanTranslateY,
    "--card-scale": String(cardScale),
  };
}

function splitHandCardRowStyle(style, { scrollSnapAlign } = {}) {
  const {
    flex,
    width,
    minWidth,
    maxWidth,
    height,
    minHeight,
    maxHeight,
    marginLeft,
    zIndex,
    "--card-rotate": cardRotate,
    "--card-translate-x": cardTranslateX,
    "--card-translate-y": cardTranslateY,
    "--card-scale": cardScale,
  } = style;

  return {
    wrapperStyle: {
      flex,
      width,
      minWidth,
      maxWidth,
      height,
      minHeight,
      maxHeight,
      marginLeft,
      zIndex,
      scrollSnapAlign,
    },
    cardStyle: {
      flex,
      width: "100%",
      minWidth: "100%",
      maxWidth: "100%",
      height: "100%",
      minHeight: "100%",
      maxHeight: "100%",
      "--card-rotate": cardRotate,
      "--card-translate-x": cardTranslateX,
      "--card-translate-y": cardTranslateY,
      "--card-scale": cardScale,
    },
  };
}

function elevateHandCardWrapperStyle(wrapperStyle, isInspected) {
  if (!isInspected) return wrapperStyle;
  return {
    ...wrapperStyle,
    zIndex: HAND_SELECTED_LAYOUT_Z_INDEX,
  };
}

function addValuesToSet(current, values) {
  let next = current;
  for (const value of values) {
    if (next.has(value)) continue;
    if (next === current) next = new Set(current);
    next.add(value);
  }
  return next;
}

function rectContainsPoint(rect, x, y) {
  return (
    x >= rect.left
    && x <= rect.right
    && y >= rect.top
    && y <= rect.bottom
  );
}

function stableHandSlotObjectIdAtPoint(handList, hoverableHandObjectIds, selectedObjectIdKey, clientX, clientY) {
  const candidates = [];
  const items = handList.querySelectorAll(".hand-layout-item[data-hand-object-id]");

  for (const item of items) {
    const rect = item.getBoundingClientRect();
    if (!rectContainsPoint(rect, clientX, clientY)) continue;

    const objectId = item.getAttribute("data-hand-object-id");
    if (!objectId || !hoverableHandObjectIds.has(objectId)) continue;

    const centerX = rect.left + (rect.width / 2);
    const centerY = rect.top + (rect.height / 2);
    candidates.push({
      objectId,
      distance: Math.abs(clientX - centerX) + (Math.abs(clientY - centerY) * 0.35),
      isSelected: selectedObjectIdKey != null && objectId === selectedObjectIdKey,
    });
  }

  candidates.sort((a, b) => a.distance - b.distance);
  return candidates.find((candidate) => !candidate.isSelected)?.objectId
    || candidates[0]?.objectId
    || null;
}

export default function HandZone({
  player,
  selectedObjectId,
  onInspect,
  isExpanded = false,
  layout = "fan",
}) {
  const { state, multiplayer } = useGame();
  const { hoveredObjectId, hoveredLinkedObjectIds, hoverCard, clearHover } = useHover();
  const { startDrag, updateDrag, endDrag } = useDragActions();
  const dragState = useDragState();
  const handScale = useManabrewHandScale(layout === "mobile-fullscreen");
  const dragThresholdRef = useRef(null);
  const activePointerIdRef = useRef(null);
  const dragHandlersRef = useRef(null);
  const dragScrollLockRef = useRef(null);
  const hoverClearTimerRef = useRef(null);
  const handListRef = useRef(null);
  const handScrollRef = useRef(null);
  const centerCycleRef = useRef(null);
  const collapsedCardRectsRef = useRef(new Map());
  const rouletteCycleSpanRef = useRef(0);
  const rouletteRecenteringRef = useRef(false);
  const mobileSelectedPreviewRafRef = useRef(null);
  const drawRevealTimersRef = useRef(new Map());
  const tuckHideTimersRef = useRef(new Map());
  const [persistHiddenDrawCardIds, setPersistHiddenDrawCardIds] = useState(() => new Set());
  const [departingTuckCardIds, setDepartingTuckCardIds] = useState(() => new Set());
  const [seenHandTransitionIds, setSeenHandTransitionIds] = useState(() => new Set());
  const [handTransitionsHydrated, setHandTransitionsHydrated] = useState(false);
  const [menuHoveredHandObjectId, setMenuHoveredHandObjectId] = useState(null);
  const [hoveredHandObjectId, setHoveredHandObjectId] = useState(null);
  const rawHandCards = useMemo(
    () => (player?.can_view_hand && player?.hand_cards) || [],
    [player?.can_view_hand, player?.hand_cards]
  );
  const drawTransitionItems = useMemo(() => {
    const items = [];
    const playerKeys = new Set(
      [player?.id, player?.index]
        .filter((value) => value != null)
        .map((value) => String(value))
    );
    for (const transition of state?.zone_transitions || []) {
      const fromZone = String(transition?.from_zone ?? transition?.fromZone ?? "").trim().toLowerCase();
      const toZone = String(transition?.to_zone ?? transition?.toZone ?? "").trim().toLowerCase();
      if (fromZone !== "library" || toZone !== "hand") continue;
      const ownerKey = transition?.owner ?? transition?.controller;
      if (playerKeys.size > 0 && ownerKey != null && !playerKeys.has(String(ownerKey))) continue;
      const transitionId = transition?.id;
      if (transitionId == null) continue;
      const objectId = transition?.new_object_id ?? transition?.newObjectId ?? transition?.card?.id ?? null;
      if (objectId != null) items.push({ transitionId: String(transitionId), objectId: String(objectId) });
    }
    return items;
  }, [player?.id, player?.index, state?.zone_transitions]);
  const tuckTransitionItems = useMemo(() => {
    const items = [];
    const playerKeys = new Set(
      [player?.id, player?.index]
        .filter((value) => value != null)
        .map((value) => String(value))
    );
    for (const transition of state?.zone_transitions || []) {
      const fromZone = String(transition?.from_zone ?? transition?.fromZone ?? "").trim().toLowerCase();
      const toZone = String(transition?.to_zone ?? transition?.toZone ?? "").trim().toLowerCase();
      if (fromZone !== "hand" || toZone !== "library") continue;
      const ownerKey = transition?.owner ?? transition?.controller;
      if (playerKeys.size > 0 && ownerKey != null && !playerKeys.has(String(ownerKey))) continue;
      const transitionId = transition?.id;
      if (transitionId == null) continue;
      const objectId = transition?.old_object_id
        ?? transition?.oldObjectId
        ?? transition?.card?.id
        ?? transition?.new_object_id
        ?? transition?.newObjectId
        ?? null;
      if (objectId != null) items.push({ transitionId: String(transitionId), objectId: String(objectId) });
    }
    return items;
  }, [player?.id, player?.index, state?.zone_transitions]);
  const rawHandCardIdsSignature = useMemo(
    () => rawHandCards.map((card) => String(card.id)).join("|"),
    [rawHandCards]
  );
  const rawHandCardIdSet = useMemo(
    () => new Set(rawHandCards.map((card) => String(card.id))),
    [rawHandCards]
  );
  const pendingDrawCardIds = useMemo(() => {
    if (!player?.can_view_hand || !handTransitionsHydrated) return new Set();
    const ids = new Set();
    for (const item of drawTransitionItems) {
      if (seenHandTransitionIds.has(item.transitionId)) continue;
      if (!rawHandCardIdSet.has(item.objectId)) continue;
      ids.add(item.objectId);
    }
    return ids;
  }, [drawTransitionItems, handTransitionsHydrated, player?.can_view_hand, rawHandCardIdSet, seenHandTransitionIds]);
  useLayoutEffect(() => {
    void rawHandCardIdsSignature;
    if (!player?.can_view_hand) return undefined;

    const transitionItems = [...drawTransitionItems, ...tuckTransitionItems];
    if (!handTransitionsHydrated) {
      const transitionIds = transitionItems.map((item) => item.transitionId);
      queueMicrotask(() => {
        setSeenHandTransitionIds((current) => addValuesToSet(current, transitionIds));
        setHandTransitionsHydrated(true);
      });
      return undefined;
    }

    const addedHiddenIds = [];
    const departingHiddenIds = [];
    const newlySeenTransitionIds = [];
    const visibleHandIds = new Set(rawHandCards.map((card) => String(card.id)));
    for (const item of drawTransitionItems) {
      if (seenHandTransitionIds.has(item.transitionId)) continue;
      newlySeenTransitionIds.push(item.transitionId);
      if (!visibleHandIds.has(item.objectId) || drawRevealTimersRef.current.has(item.objectId)) continue;
      addedHiddenIds.push(item.objectId);
    }
    for (const item of tuckTransitionItems) {
      if (seenHandTransitionIds.has(item.transitionId)) continue;
      newlySeenTransitionIds.push(item.transitionId);
      if (!visibleHandIds.has(item.objectId) || tuckHideTimersRef.current.has(item.objectId)) continue;
      departingHiddenIds.push(item.objectId);
    }
    if (newlySeenTransitionIds.length === 0 && addedHiddenIds.length === 0 && departingHiddenIds.length === 0) {
      return undefined;
    }

    addedHiddenIds.forEach((id, index) => {
      const delay = DRAW_TO_HAND_REVEAL_DELAY_MS + (index * DRAW_TO_HAND_REVEAL_STAGGER_MS);
      const timerId = window.setTimeout(() => {
        drawRevealTimersRef.current.delete(id);
        setPersistHiddenDrawCardIds((current) => {
          if (!current.has(id)) return current;
          const next = new Set(current);
          next.delete(id);
          return next;
        });
      }, delay);
      drawRevealTimersRef.current.set(id, timerId);
    });
    departingHiddenIds.forEach((id) => {
      const timerId = window.setTimeout(() => {
        tuckHideTimersRef.current.delete(id);
        setDepartingTuckCardIds((current) => {
          if (!current.has(id)) return current;
          const next = new Set(current);
          next.delete(id);
          return next;
        });
      }, DRAW_TO_HAND_REVEAL_DELAY_MS);
      tuckHideTimersRef.current.set(id, timerId);
    });
    queueMicrotask(() => {
      setSeenHandTransitionIds((current) => addValuesToSet(current, newlySeenTransitionIds));
      setPersistHiddenDrawCardIds((current) => {
        let next = current;
        for (const id of addedHiddenIds) {
          if (next.has(id)) continue;
          if (next === current) next = new Set(current);
          next.add(id);
        }
        return next;
      });
      setDepartingTuckCardIds((current) => {
        let next = current;
        for (const id of departingHiddenIds) {
          if (next.has(id)) continue;
          if (next === current) next = new Set(current);
          next.add(id);
        }
        return next;
      });
    });

    return undefined;
  }, [
    drawTransitionItems,
    handTransitionsHydrated,
    player?.can_view_hand,
    rawHandCardIdsSignature,
    rawHandCards,
    seenHandTransitionIds,
    tuckTransitionItems,
  ]);
  useEffect(() => {
    const visibleIds = new Set(rawHandCards.map((card) => String(card.id)));
    const staleDrawIds = [];
    for (const id of persistHiddenDrawCardIds) {
      if (visibleIds.has(id)) continue;
      staleDrawIds.push(id);
      const timerId = drawRevealTimersRef.current.get(id);
      if (timerId != null) window.clearTimeout(timerId);
      drawRevealTimersRef.current.delete(id);
    }
    const staleTuckIds = [];
    for (const id of departingTuckCardIds) {
      if (visibleIds.has(id)) continue;
      staleTuckIds.push(id);
      const timerId = tuckHideTimersRef.current.get(id);
      if (timerId != null) window.clearTimeout(timerId);
      tuckHideTimersRef.current.delete(id);
    }
    if (staleDrawIds.length === 0 && staleTuckIds.length === 0) return;
    queueMicrotask(() => {
      setPersistHiddenDrawCardIds((current) => {
        let next = current;
        for (const id of staleDrawIds) {
          if (!next.has(id)) continue;
          if (next === current) next = new Set(current);
          next.delete(id);
        }
        return next;
      });
      setDepartingTuckCardIds((current) => {
        let next = current;
        for (const id of staleTuckIds) {
          if (!next.has(id)) continue;
          if (next === current) next = new Set(current);
          next.delete(id);
        }
        return next;
      });
    });
  }, [departingTuckCardIds, persistHiddenDrawCardIds, rawHandCards]);
  const hiddenDrawCardIds = useMemo(() => {
    const ids = new Set(departingTuckCardIds);
    for (const id of persistHiddenDrawCardIds) {
      ids.add(id);
    }
    for (const id of pendingDrawCardIds) {
      ids.add(id);
    }
    return ids;
  }, [departingTuckCardIds, pendingDrawCardIds, persistHiddenDrawCardIds]);
  useEffect(() => () => {
    for (const timerId of drawRevealTimersRef.current.values()) {
      window.clearTimeout(timerId);
    }
    for (const timerId of tuckHideTimersRef.current.values()) {
      window.clearTimeout(timerId);
    }
    drawRevealTimersRef.current.clear();
    tuckHideTimersRef.current.clear();
  }, []);
  // Reserve an invisible slot during the card flight so revealing it does not
  // change the fan geometry. Track visible cards separately for their entrance.
  const reserveDrawSlots = layout !== "vertical-rail";
  const handCards = useMemo(() => {
    const excludedIds = reserveDrawSlots ? departingTuckCardIds : hiddenDrawCardIds;
    return excludedIds.size === 0
      ? rawHandCards
      : rawHandCards.filter((card) => !excludedIds.has(String(card.id)));
  }, [departingTuckCardIds, hiddenDrawCardIds, rawHandCards, reserveDrawSlots]);
  const previousExpandedRef = useRef(isExpanded);
  const visibleHandCardIds = handCards
    .filter((card) => !hiddenDrawCardIds.has(String(card.id)))
    .map((card) => card.id);
  const { newIds, bumpedIds } = useNewCards(visibleHandCardIds);

  const isMe = player?.id === state?.perspective;

  const actionsPausedForSync = Boolean(multiplayer?.matchStarted && multiplayer?.submittingAction);
  const { handPlayable, extraPlayable } = useMemo(
    () => isMe && !actionsPausedForSync
      ? buildPlayableMaps(state, player)
      : { handPlayable: new Map(), extraPlayable: new Map() },
    [actionsPausedForSync, isMe, state, player]
  );
  const priorityActionObjectIds = useMemo(() => {
    const ids = new Set();
    const decision = state?.decision;
    if (!decision || decision.kind !== "priority" || !samePlayerId(decision.player, state?.perspective)) {
      return ids;
    }
    for (const action of decision.actions || []) {
      if (action.kind === "pass_priority" || action.object_id == null) continue;
      ids.add(String(action.object_id));
    }
    return ids;
  }, [state?.decision, state?.perspective]);

  // Extra playable cards as array for rendering
  const extraCards = useMemo(() => {
    const cards = [];
    for (const [objId, data] of extraPlayable) {
      cards.push({
        id: objId,
        name: data.name,
        card: data.card,
        fromZone: data.fromZone,
        actions: data.actions,
        glowKind: data.glowKind,
      });
    }
    return cards;
  }, [extraPlayable]);
  const hoverableHandObjectIds = useMemo(() => {
    const ids = new Set();
    for (const card of handCards) {
      if (!hiddenDrawCardIds.has(String(card.id))) ids.add(String(card.id));
    }
    for (const extra of extraCards) ids.add(String(extra.id));
    return ids;
  }, [extraCards, handCards, hiddenDrawCardIds]);
  useEffect(() => {
    if (typeof window === "undefined") return undefined;
    const handleHandActionHover = (event) => {
      const rawObjectId = event?.detail?.objectId ?? null;
      const normalizedObjectId = rawObjectId != null ? String(rawObjectId) : null;
      setMenuHoveredHandObjectId(
        normalizedObjectId && hoverableHandObjectIds.has(normalizedObjectId)
          ? normalizedObjectId
          : null
      );
    };
    window.addEventListener(HAND_ACTION_HOVER_EVENT, handleHandActionHover);
    return () => {
      window.removeEventListener(HAND_ACTION_HOVER_EVENT, handleHandActionHover);
    };
  }, [hoverableHandObjectIds]);
  const activeMenuHoveredHandObjectId = (
    menuHoveredHandObjectId && hoverableHandObjectIds.has(menuHoveredHandObjectId)
      ? menuHoveredHandObjectId
      : null
  );
  const selectedObjectIdKey = selectedObjectId != null ? String(selectedObjectId) : null;
  const handLayoutSignature = useMemo(
    () => [
      handCards.map((card) => card.id).join("|"),
      extraCards.map((card) => `extra-${card.id}`).join("|"),
      layout,
      handScale.toFixed(3),
    ].join("::"),
    [extraCards, handCards, handScale, layout]
  );
  const renderedHandCardCount = handCards.length + extraCards.length;
  const hasExtra = extraCards.length > 0;
  const isMobileFan = layout === "mobile-fan" || layout === "mobile-fullscreen";
  const isVerticalRail = layout === "vertical-rail";
  const isRoulette = !isVerticalRail && !isMobileFan && renderedHandCardCount >= HAND_ROULETTE_THRESHOLD;
  const handDimensions = useMemo(
    () => computeManabrewHandDimensions(handScale),
    [handScale]
  );
  const activeFanObjectId = dragState || isMobileFan
    ? null
    : activeMenuHoveredHandObjectId
      || selectedObjectIdKey
      || hoveredHandObjectId;
  const activeFanIndex = useMemo(() => {
    if (!activeFanObjectId) return null;
    const handIndex = handCards.findIndex((card) => String(card.id) === activeFanObjectId);
    if (handIndex >= 0) return handIndex;
    const extraIndex = extraCards.findIndex((card) => String(card.id) === activeFanObjectId);
    return extraIndex >= 0 ? handCards.length + extraIndex : null;
  }, [activeFanObjectId, extraCards, handCards]);
  const rouletteWidth = useMemo(
    () => computeRouletteWidth(renderedHandCardCount, handDimensions),
    [handDimensions, renderedHandCardCount]
  );
  const nonRouletteWidth = useMemo(
    () => computeHandRowWidth(renderedHandCardCount, handDimensions) + 32,
    [handDimensions, renderedHandCardCount]
  );
  const surfaceWidth = isVerticalRail
    ? "100%"
    : isMobileFan
    ? `min(${nonRouletteWidth}px, 100%)`
    : isRoulette
    ? `min(${rouletteWidth}px, calc(100vw - 290px))`
    : `min(${nonRouletteWidth}px, 100%)`;
  const rouletteCycleIndexes = isRoulette
    ? Array.from({ length: HAND_ROULETTE_CYCLE_COUNT }, (_, index) => index)
    : [HAND_ROULETTE_CENTER_CYCLE];
  const handEntries = useMemo(() => {
    const entries = handCards.map((card, visualIndex) => ({
      kind: "hand",
      key: `hand-${card.id}`,
      card,
      visualIndex,
    }));
    if (hasExtra && handCards.length > 0) {
      entries.push({ kind: "separator", key: "separator" });
    }
    for (let extraIndex = 0; extraIndex < extraCards.length; extraIndex += 1) {
      const extra = extraCards[extraIndex];
      entries.push({
        kind: "extra",
        key: `extra-${extra.id}`,
        extra,
        visualIndex: handCards.length + extraIndex,
      });
    }
    return entries;
  }, [extraCards, handCards, hasExtra]);

  useHandReflow(handListRef, handLayoutSignature, isRoulette || isVerticalRail);

  useLayoutEffect(() => {
    // Preserve each card's settled position in the collapsed fan. Pointerdown
    // happens after the fan has opened, so measuring there would anchor the
    // cast arrow to the enlarged/raised card instead of its resting slot.
    if (isExpanded || dragState) return;
    const measurementRoot = isRoulette ? centerCycleRef.current : handListRef.current;
    if (!measurementRoot) return;
    const nextRects = new Map();
    for (const item of measurementRoot.querySelectorAll(".hand-layout-item[data-hand-object-id]")) {
      const cardElement = item.querySelector(":scope > .game-card.hand-card");
      const rect = plainRect(cardElement?.getBoundingClientRect?.());
      if (rect) nextRects.set(String(item.dataset.handObjectId), rect);
    }
    if (nextRects.size > 0) collapsedCardRectsRef.current = nextRects;
  }, [dragState, handLayoutSignature, isExpanded, isRoulette]);

  const handleCardClick = (_e, card) => {
    const candidateObjectIds = [Number(card?.id)].filter((id) => Number.isFinite(id));
    onInspect?.(card.id, { candidateObjectIds, source: "hand" });
  };

  const releaseDragScrollLock = useCallback(() => {
    const scrollLock = dragScrollLockRef.current;
    if (!scrollLock) return;
    const { element, touchAction, overscrollBehavior } = scrollLock;
    if (element) {
      element.style.touchAction = touchAction;
      element.style.overscrollBehavior = overscrollBehavior;
    }
    dragScrollLockRef.current = null;
  }, []);

  const clearPendingDragListeners = () => {
    const handlers = dragHandlersRef.current;
    if (!handlers) return;
    document.removeEventListener("pointermove", handlers.onMove);
    document.removeEventListener("pointerup", handlers.onUp);
    document.removeEventListener("pointercancel", handlers.onCancel);
    dragHandlersRef.current = null;
    activePointerIdRef.current = null;
    releaseDragScrollLock();
  };

  useEffect(() => {
    return () => {
      if (hoverClearTimerRef.current) {
        clearTimeout(hoverClearTimerRef.current);
        hoverClearTimerRef.current = null;
      }
      const handlers = dragHandlersRef.current;
      if (!handlers) return;
      document.removeEventListener("pointermove", handlers.onMove);
      document.removeEventListener("pointerup", handlers.onUp);
      document.removeEventListener("pointercancel", handlers.onCancel);
      dragHandlersRef.current = null;
      activePointerIdRef.current = null;
      releaseDragScrollLock();
    };
  }, [releaseDragScrollLock]);

  useLayoutEffect(() => {
    const handList = handListRef.current;
    if (!handList) return undefined;

    const clearSelectedPreviewVars = () => {
      const cards = handList.querySelectorAll(".game-card.hand-card");
      for (const node of cards) {
        node.style.removeProperty("--mobile-hand-selected-shift-x");
        node.style.removeProperty("--mobile-hand-selected-shift-y");
      }
    };

    clearSelectedPreviewVars();

    if (!isMobileFan || selectedObjectId == null || typeof window === "undefined") {
      return clearSelectedPreviewVars;
    }

    mobileSelectedPreviewRafRef.current = window.requestAnimationFrame(() => {
      mobileSelectedPreviewRafRef.current = null;
      const selectedCard = handList.querySelector(".game-card.hand-card.inspected");
      if (!selectedCard) return;

      const rect = selectedCard.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const targetCenterX = Math.min(
        viewportWidth - rect.width * 0.65,
        Math.max(rect.width * 0.65, viewportWidth * 0.46)
      );
      const shiftX = Math.max(-260, Math.min(260, targetCenterX - (rect.left + (rect.width / 2))));
      const shiftY = Math.max(78, Math.min(116, window.innerHeight * 0.24));

      selectedCard.style.setProperty("--mobile-hand-selected-shift-x", `${shiftX.toFixed(1)}px`);
      selectedCard.style.setProperty("--mobile-hand-selected-shift-y", `${shiftY.toFixed(1)}px`);
    });

    return () => {
      if (mobileSelectedPreviewRafRef.current != null) {
        cancelAnimationFrame(mobileSelectedPreviewRafRef.current);
        mobileSelectedPreviewRafRef.current = null;
      }
      clearSelectedPreviewVars();
    };
  }, [isMobileFan, selectedObjectId]);


  const handleHoverEnter = useCallback((objectId) => {
    if (hoverClearTimerRef.current) {
      clearTimeout(hoverClearTimerRef.current);
      hoverClearTimerRef.current = null;
    }
    const normalizedObjectId = String(objectId);
    setHoveredHandObjectId(normalizedObjectId);
    hoverCard(normalizedObjectId);
  }, [hoverCard]);

  const handleHoverLeave = useCallback(() => {
    if (hoverClearTimerRef.current) {
      clearTimeout(hoverClearTimerRef.current);
    }
    // Small delay smooths hover-out when moving across dense hand cards.
    hoverClearTimerRef.current = setTimeout(() => {
      setHoveredHandObjectId(null);
      clearHover();
      hoverClearTimerRef.current = null;
    }, 110);
  }, [clearHover]);

  useEffect(() => {
    const wasExpanded = previousExpandedRef.current;
    previousExpandedRef.current = isExpanded;
    if (wasExpanded && !isExpanded && hoveredHandObjectId != null) {
      handleHoverLeave();
    }
  }, [handleHoverLeave, hoveredHandObjectId, isExpanded]);

  const resolveHandHoverObjectId = useCallback((clientX, clientY) => {
    const handList = handListRef.current;
    if (!handList || typeof document === "undefined" || typeof document.elementsFromPoint !== "function") {
      return null;
    }

    let selectedCandidate = null;
    for (const element of document.elementsFromPoint(clientX, clientY)) {
      const cardEl = element?.closest?.(".game-card.hand-card");
      if (!cardEl || !handList.contains(cardEl)) continue;

      const objectId = cardEl.getAttribute("data-object-id");
      if (!objectId || !hoverableHandObjectIds.has(objectId)) continue;
      if (selectedObjectIdKey != null && objectId === selectedObjectIdKey) {
        selectedCandidate = objectId;
        continue;
      }
      return objectId;
    }

    if (isMobileFan) {
      return stableHandSlotObjectIdAtPoint(
        handList,
        hoverableHandObjectIds,
        selectedObjectIdKey,
        clientX,
        clientY
      ) || selectedCandidate;
    }

    return selectedCandidate;
  }, [hoverableHandObjectIds, isMobileFan, selectedObjectIdKey]);

  const handleHandPointerMove = useCallback((event) => {
    if (event.pointerType === "touch" || activePointerIdRef.current != null) return;

    const objectId = resolveHandHoverObjectId(event.clientX, event.clientY);
    if (objectId == null) {
      if (hoveredHandObjectId != null) {
        handleHoverLeave();
      }
      return;
    }

    if (hoverClearTimerRef.current) {
      clearTimeout(hoverClearTimerRef.current);
      hoverClearTimerRef.current = null;
    }
    if (hoveredHandObjectId !== objectId) {
      setHoveredHandObjectId(objectId);
    }
  }, [handleHoverLeave, hoveredHandObjectId, resolveHandHoverObjectId]);

  const handleHandPointerLeave = useCallback((event) => {
    if (event.pointerType === "touch") return;
    handleHoverLeave();
  }, [handleHoverLeave]);

  const recenterRouletteIfNeeded = useCallback(() => {
    if (!isRoulette) return;
    const scrollEl = handScrollRef.current;
    const cycleSpan = rouletteCycleSpanRef.current;
    if (!scrollEl || cycleSpan <= 0 || rouletteRecenteringRef.current) return;

    const minScrollLeft = cycleSpan * 0.5;
    const maxScrollLeft = cycleSpan * 1.5;
    let nextScrollLeft = scrollEl.scrollLeft;

    while (nextScrollLeft < minScrollLeft) {
      nextScrollLeft += cycleSpan;
    }
    while (nextScrollLeft > maxScrollLeft) {
      nextScrollLeft -= cycleSpan;
    }

    if (Math.abs(nextScrollLeft - scrollEl.scrollLeft) < 0.5) return;

    rouletteRecenteringRef.current = true;
    scrollEl.scrollLeft = nextScrollLeft;
    requestAnimationFrame(() => {
      rouletteRecenteringRef.current = false;
    });
  }, [isRoulette]);

  useLayoutEffect(() => {
    if (!isRoulette) {
      rouletteCycleSpanRef.current = 0;
      rouletteRecenteringRef.current = false;
      return;
    }

    const scrollEl = handScrollRef.current;
    const centerCycleEl = centerCycleRef.current;
    if (!scrollEl || !centerCycleEl) return;

    const cycleSpan = centerCycleEl.offsetWidth + HAND_ROULETTE_WRAP_GAP;
    const previousSpan = rouletteCycleSpanRef.current;
    const offsetWithinCycle = previousSpan > 0 ? scrollEl.scrollLeft - previousSpan : 0;
    rouletteCycleSpanRef.current = cycleSpan;
    rouletteRecenteringRef.current = true;
    scrollEl.scrollLeft = cycleSpan + offsetWithinCycle;
    requestAnimationFrame(() => {
      rouletteRecenteringRef.current = false;
    });
  }, [handLayoutSignature, isRoulette]);

  useLayoutEffect(() => {
    if (isRoulette) return undefined;
    const scrollEl = handScrollRef.current;
    if (!scrollEl) return undefined;

    scrollEl.scrollLeft = 0;
    const frameId = requestAnimationFrame(() => {
      scrollEl.scrollLeft = 0;
    });
    return () => cancelAnimationFrame(frameId);
  }, [isRoulette]);

  const handleRouletteWheel = useCallback((event) => {
    if (!isRoulette) return;
    const scrollEl = handScrollRef.current;
    if (!scrollEl) return;
    const primaryDelta = Math.abs(event.deltaX) > Math.abs(event.deltaY)
      ? event.deltaX
      : event.deltaY;
    if (primaryDelta === 0) return;
    event.preventDefault();
    scrollEl.scrollBy({
      left: primaryDelta * 1.1,
      behavior: "auto",
    });
  }, [isRoulette]);

  const handleRouletteScroll = useCallback(() => {
    recenterRouletteIfNeeded();
  }, [recenterRouletteIfNeeded]);

  useEffect(() => {
    const scrollEl = handScrollRef.current;
    if (!scrollEl) return undefined;

    scrollEl.addEventListener("wheel", handleRouletteWheel, { passive: false });
    return () => {
      scrollEl.removeEventListener("wheel", handleRouletteWheel);
    };
  }, [handleRouletteWheel]);

  const handlePointerDown = (e, card, plays, glowKind) => {
    if (plays.length === 0) return;
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    // React nulls event.currentTarget after dispatch, so the deferred
    // pointerup/cancel closures below need their own reference.
    const captureTarget = e.currentTarget;
    try {
      captureTarget?.setPointerCapture?.(e.pointerId);
    } catch {
      // Touch browsers can reject capture during rapid gesture changes.
    }
    clearPendingDragListeners();
    activePointerIdRef.current = e.pointerId;
    const sx = e.clientX;
    const sy = e.clientY;
    const sourceRect = e.currentTarget?.closest?.(".game-card")?.getBoundingClientRect?.() || null;
    const collapsedSourceRect = collapsedCardRectsRef.current.get(String(card.id)) || plainRect(sourceRect);
    // Keep the outer hand bounds as a fallback for layouts without a cached
    // collapsed card slot (for example, a newly revealed mobile card).
    const sourceContainer = (
      e.currentTarget?.closest?.(".hand-reveal-shell")
      || e.currentTarget?.closest?.(".mobile-hand-rail-zone")
      || e.currentTarget?.closest?.(".hand-zone-surface")
    );
    const sourceContainerRect = sourceContainer?.getBoundingClientRect?.() || null;
    dragThresholdRef.current = {
      sx,
      sy,
      card,
      plays,
      glowKind,
      sourceRect: plainRect(sourceRect),
      sourceContainerRect: plainRect(sourceContainerRect),
      hiddenSourcePoint: handCardSourcePoint(collapsedSourceRect),
      dragging: false,
    };

    const onMove = (me) => {
      if (activePointerIdRef.current != null && me.pointerId !== activePointerIdRef.current) {
        return;
      }
      const dt = dragThresholdRef.current;
      if (!dt) return;
      const dx = me.clientX - dt.sx;
      const dy = me.clientY - dt.sy;
      if (!dt.dragging && (dx * dx + dy * dy) > HAND_DRAG_START_DISTANCE_SQ) {
        dt.dragging = true;
        if (
          dragScrollLockRef.current == null
          && (isVerticalRail || isMobileFan)
          && me.pointerType !== "mouse"
          && handScrollRef.current
        ) {
          dragScrollLockRef.current = {
            element: handScrollRef.current,
            touchAction: handScrollRef.current.style.touchAction,
            overscrollBehavior: handScrollRef.current.style.overscrollBehavior,
          };
          handScrollRef.current.style.touchAction = "none";
          handScrollRef.current.style.overscrollBehavior = "none";
        }
        startDrag(
          card.id,
          card.name,
          plays,
          glowKind,
          me.clientX,
          me.clientY,
          dt.sourceRect || null,
          {
            ...card,
            id: card.id,
            name: card.name,
            card_types: Array.isArray(card.card_types) ? [...card.card_types] : [],
            member_ids: Array.isArray(card.member_ids) ? [...card.member_ids] : [],
            member_stable_ids: Array.isArray(card.member_stable_ids) ? [...card.member_stable_ids] : [],
            type_line: card.type_line || null,
            mana_cost: card.mana_cost || null,
            oracle_text: card.oracle_text || null,
            effect_text: card.effect_text || null,
          },
          dt.sourceContainerRect || null,
          dt.hiddenSourcePoint || null,
        );
      }
      if (dt.dragging) {
        if (me.cancelable) {
          me.preventDefault();
        }
        updateDrag(me.clientX, me.clientY);
      }
    };

    const onUp = (ue) => {
      if (activePointerIdRef.current != null && ue.pointerId !== activePointerIdRef.current) {
        return;
      }
      try {
        captureTarget?.releasePointerCapture?.(ue.pointerId);
      } catch {
        // No-op if capture was never established.
      }
      clearPendingDragListeners();
      const dt = dragThresholdRef.current;
      dragThresholdRef.current = null;
      if (dt && !dt.dragging) {
        handleCardClick(ue, card);
      }
    };

    const onCancel = (ce) => {
      if (activePointerIdRef.current != null && ce.pointerId !== activePointerIdRef.current) {
        return;
      }
      try {
        captureTarget?.releasePointerCapture?.(ce.pointerId);
      } catch {
        // No-op if capture was never established.
      }
      clearPendingDragListeners();
      dragThresholdRef.current = null;
      endDrag();
    };

    dragHandlersRef.current = { onMove, onUp, onCancel };
    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
    document.addEventListener("pointercancel", onCancel);
  };

  if (!player) return null;

  if (player.can_view_hand) {
    const renderVerticalEntry = (entry) => {
      if (entry.kind === "separator") {
        return <div key={entry.key} className="mobile-hand-rail-divider" aria-hidden="true" />;
      }

      if (entry.kind === "hand") {
        const { card, visualIndex } = entry;
        const plays = handPlayable.get(Number(card.id)) || [];
        const isPlayable = plays.length > 0;
        const baseGlowKind = isPlayable ? handGlowFromTypes(card.card_types) : null;
        const isActionLinkedHover = (
          hoveredLinkedObjectIds.has(String(card.id))
          || (
            hoveredObjectId != null
            && String(hoveredObjectId) === String(card.id)
            && priorityActionObjectIds.has(String(card.id))
          )
        );
        const isMenuActionPreview = activeMenuHoveredHandObjectId === String(card.id);
        const glowKind = isMenuActionPreview
          ? baseGlowKind
          : isActionLinkedHover ? "action-link" : baseGlowKind;
        const cardObjectId = String(card.id);
        const isHovered = hoveredHandObjectId === cardObjectId;
        const isInspected = !isMobileFan && (
          (selectedObjectIdKey != null && cardObjectId === selectedObjectIdKey)
          || isMenuActionPreview
        );
        const isNew = newIds.has(card.id);
        const isBumped = bumpedIds.has(card.id);
        let bumpDir = 0;
        if (isBumped) {
          if (visualIndex > 0 && newIds.has(handCards[visualIndex - 1].id)) bumpDir = 1;
          else if (visualIndex < handCards.length - 1 && newIds.has(handCards[visualIndex + 1].id)) bumpDir = -1;
        }
        return (
          <GameCard
            key={entry.key}
            card={card}
            variant="hand"
            isPlayable={isPlayable}
            glowKind={glowKind}
            isNew={isNew}
            isBumped={isBumped}
            bumpDirection={bumpDir}
            handCircuitMode="full"
            suppressTooltip={isMobileFan}
            isHovered={isHovered}
            isInspected={isInspected}
            onClick={isPlayable ? undefined : (event) => handleCardClick(event, card)}
            onKeyboardActivate={(event) => handleCardClick(event, card)}
            onPointerDown={isPlayable ? (event) => handlePointerDown(event, card, plays, glowKind) : undefined}
            onMouseEnter={() => handleHoverEnter(card.id)}
            onMouseLeave={isMobileFan ? undefined : handleHoverLeave}
            className={`mobile-hand-rail-card${isPlayable ? " mobile-hand-rail-card--draggable" : ""}${String(dragState?.objectId) === cardObjectId ? " hand-card--drag-source" : ""} !w-full !max-w-none !min-w-0 !basis-auto !flex-none self-stretch p-1`}
            style={{
              width: "100%",
              minWidth: "0px",
              maxWidth: "100%",
              minHeight: "var(--mobile-hand-rail-card-height, 42px)",
              height: "var(--mobile-hand-rail-card-height, 42px)",
            }}
          />
        );
      }

      const { extra } = entry;
      const card = extra.card || { id: extra.id, name: extra.name };
      const plays = extra.actions;
      const isPlayable = plays.length > 0;
      const baseGlowKind = extra.glowKind || (isPlayable ? "extra" : null);
      const isActionLinkedHover = (
        hoveredLinkedObjectIds.has(String(extra.id))
        || (
          hoveredObjectId != null
          && String(hoveredObjectId) === String(extra.id)
          && priorityActionObjectIds.has(String(extra.id))
        )
      );
      const isMenuActionPreview = activeMenuHoveredHandObjectId === String(extra.id);
      const glowKind = isMenuActionPreview
        ? baseGlowKind
        : isActionLinkedHover ? "action-link" : baseGlowKind;
      const extraObjectId = String(extra.id);
      const isHovered = hoveredHandObjectId === extraObjectId;
      const isInspected = !isMobileFan && (
        (selectedObjectIdKey != null && extraObjectId === selectedObjectIdKey)
        || isMenuActionPreview
      );
      return (
        <GameCard
          key={entry.key}
          card={card}
          variant="hand"
          isPlayable={isPlayable}
          glowKind={glowKind}
          handCircuitMode="full"
          suppressTooltip={isMobileFan}
          isHovered={isHovered}
          isInspected={isInspected}
          onClick={isPlayable ? undefined : (event) => handleCardClick(event, card)}
          onKeyboardActivate={(event) => handleCardClick(event, card)}
          onPointerDown={plays.length > 0 ? (event) => handlePointerDown(event, card, plays, baseGlowKind || "extra") : undefined}
          onMouseEnter={() => handleHoverEnter(extra.id)}
          onMouseLeave={isMobileFan ? undefined : handleHoverLeave}
          className={`mobile-hand-rail-card mobile-hand-rail-card--extra${plays.length > 0 ? " mobile-hand-rail-card--draggable" : ""}${String(dragState?.objectId) === extraObjectId ? " hand-card--drag-source" : ""} !w-full !max-w-none !min-w-0 !basis-auto !flex-none self-stretch p-1`}
          style={{
            width: "100%",
            minWidth: "0px",
            maxWidth: "100%",
            minHeight: "var(--mobile-hand-rail-card-height, 42px)",
            height: "var(--mobile-hand-rail-card-height, 42px)",
          }}
        />
      );
    };

    const renderHandEntry = (entry, cycleIndex) => {
      const isPrimaryCycle = !isRoulette || cycleIndex === HAND_ROULETTE_CENTER_CYCLE;

      if (entry.kind === "separator") {
        return (
          <div
            key={`${cycleIndex}-${entry.key}`}
            className="mx-3 w-px self-stretch my-2 bg-[rgba(174,118,255,0.3)]"
          />
        );
      }

      if (entry.kind === "hand") {
        const { card, visualIndex } = entry;
        const plays = handPlayable.get(Number(card.id)) || [];
        const isPlayable = plays.length > 0;
        const baseGlowKind = isPlayable ? handGlowFromTypes(card.card_types) : null;
        const isActionLinkedHover = (
          hoveredLinkedObjectIds.has(String(card.id))
          || (
            hoveredObjectId != null
            && String(hoveredObjectId) === String(card.id)
            && priorityActionObjectIds.has(String(card.id))
          )
        );
        const isMenuActionPreview = activeMenuHoveredHandObjectId === String(card.id);
        const glowKind = isMenuActionPreview
          ? baseGlowKind
          : isActionLinkedHover ? "action-link" : baseGlowKind;
        const cardObjectId = String(card.id);
        const isHovered = hoveredHandObjectId === cardObjectId;
        const isInspected = !isMobileFan && (
          (selectedObjectIdKey != null && cardObjectId === selectedObjectIdKey)
          || isMenuActionPreview
        );
        const isDrawInFlight = reserveDrawSlots && hiddenDrawCardIds.has(cardObjectId);
        const isNew = isPrimaryCycle && !isDrawInFlight && newIds.has(card.id);
        const { wrapperStyle: baseWrapperStyle, cardStyle } = splitHandCardRowStyle(
          buildHandCardRowStyle(visualIndex, renderedHandCardCount, {
            dims: handDimensions,
            activeIndex: isPrimaryCycle ? activeFanIndex : null,
          }),
          { scrollSnapAlign: isRoulette ? "start" : undefined }
        );
        const wrapperStyle = elevateHandCardWrapperStyle(baseWrapperStyle, isInspected);
        return (
          <div
            key={`${cycleIndex}-${entry.key}`}
            className={`hand-layout-item shrink-0 overflow-visible${isInspected ? " hand-layout-item--selected" : ""}${isHovered && !isInspected ? " hand-layout-item--hovered" : ""}`}
            data-hand-object-id={cardObjectId}
            data-hand-draw-pending={isDrawInFlight ? "true" : undefined}
            aria-hidden={isDrawInFlight ? true : undefined}
            inert={isDrawInFlight ? true : undefined}
            style={isDrawInFlight ? { ...wrapperStyle, visibility: "hidden", pointerEvents: "none" } : wrapperStyle}
          >
            <GameCard
              card={card}
              variant="hand"
              isPlayable={isPlayable}
              glowKind={glowKind}
              isNew={isNew}
              handCircuitMode={isExpanded ? "full" : "top"}
              suppressTooltip={isMobileFan}
              isHovered={isHovered}
              isInspected={isInspected}
              onClick={isPlayable ? undefined : (e) => handleCardClick(e, card)}
              onKeyboardActivate={(event) => handleCardClick(event, card)}
              onPointerDown={isPlayable ? (e) => handlePointerDown(e, card, plays, glowKind) : undefined}
              onMouseEnter={() => handleHoverEnter(card.id)}
              onMouseLeave={isMobileFan ? undefined : handleHoverLeave}
              className={[
                isMobileFan && isPlayable ? "hand-card--mobile-draggable" : null,
                String(dragState?.objectId) === cardObjectId ? "hand-card--drag-source" : null,
              ].filter(Boolean).join(" ") || undefined}
              style={cardStyle}
            />
          </div>
        );
      }

      const { extra, visualIndex } = entry;
      const card = extra.card || { id: extra.id, name: extra.name };
      const plays = extra.actions;
      const isPlayable = plays.length > 0;
      const baseGlowKind = extra.glowKind || (isPlayable ? "extra" : null);
      const isActionLinkedHover = (
        hoveredLinkedObjectIds.has(String(extra.id))
        || (
          hoveredObjectId != null
          && String(hoveredObjectId) === String(extra.id)
          && priorityActionObjectIds.has(String(extra.id))
        )
      );
      const isMenuActionPreview = activeMenuHoveredHandObjectId === String(extra.id);
      const glowKind = isMenuActionPreview
        ? baseGlowKind
        : isActionLinkedHover ? "action-link" : baseGlowKind;
      const extraObjectId = String(extra.id);
      const isHovered = hoveredHandObjectId === extraObjectId;
      const isInspected = !isMobileFan && (
        (selectedObjectIdKey != null && extraObjectId === selectedObjectIdKey)
        || isMenuActionPreview
      );
      const { wrapperStyle: baseWrapperStyle, cardStyle } = splitHandCardRowStyle(
        buildHandCardRowStyle(visualIndex, renderedHandCardCount, {
          dims: handDimensions,
          activeIndex: isPrimaryCycle ? activeFanIndex : null,
        }),
        { scrollSnapAlign: isRoulette ? "start" : undefined }
      );
      const wrapperStyle = elevateHandCardWrapperStyle(baseWrapperStyle, isInspected);
      return (
        <div
          key={`${cycleIndex}-${entry.key}`}
          className={`hand-layout-item shrink-0 overflow-visible${isInspected ? " hand-layout-item--selected" : ""}${isHovered && !isInspected ? " hand-layout-item--hovered" : ""}`}
          data-hand-object-id={extraObjectId}
          style={wrapperStyle}
        >
          <GameCard
            card={card}
            variant="hand"
            isPlayable={isPlayable}
            glowKind={glowKind}
            isNew={isPrimaryCycle}
            handCircuitMode={isExpanded ? "full" : "top"}
            suppressTooltip={isMobileFan}
            isHovered={isHovered}
            isInspected={isInspected}
            onClick={isPlayable ? undefined : (e) => handleCardClick(e, card)}
            onKeyboardActivate={(event) => handleCardClick(event, card)}
            onPointerDown={plays.length > 0 ? (e) => handlePointerDown(e, card, plays, baseGlowKind || "extra") : undefined}
            onMouseEnter={() => handleHoverEnter(extra.id)}
            onMouseLeave={isMobileFan ? undefined : handleHoverLeave}
            className={[
              isMobileFan && isPlayable ? "hand-card--mobile-draggable" : null,
              String(dragState?.objectId) === extraObjectId ? "hand-card--drag-source" : null,
            ].filter(Boolean).join(" ") || undefined}
            style={cardStyle}
          />
        </div>
      );
    };

    if (isVerticalRail) {
      return (
        <section className="mobile-hand-rail-zone min-h-0 h-full overflow-hidden">
          <div className="mobile-hand-rail-scroll min-h-0 h-full overflow-y-auto overflow-x-hidden pr-0.5">
            <div
              ref={handListRef}
              className="mobile-hand-rail-list flex min-h-full flex-col items-stretch gap-1.5 pb-1"
            >
              {handEntries.map((entry) => renderVerticalEntry(entry))}
              {handCards.length === 0 && extraCards.length === 0 && (
                <div className="mobile-hand-rail-empty text-muted-foreground p-2 text-center text-[11px] italic">
                  Empty hand
                </div>
              )}
            </div>
          </div>
        </section>
      );
    }

      return (
        <section
          className={`hand-zone-surface min-w-0 bg-transparent px-2 py-1 h-full min-h-0 overflow-visible ${isRoulette ? "hand-zone-surface-roulette" : "max-w-full"} ${isMobileFan ? "hand-zone-surface-mobile-fan" : ""}`}
          style={{
            width: surfaceWidth,
            maxWidth: isRoulette ? surfaceWidth : "100%",
          }}
        >
        <div className={`hand-zone-viewport min-h-0 h-full w-full min-w-0 overflow-visible ${isRoulette ? "hand-zone-viewport-roulette" : ""} ${isMobileFan ? "hand-zone-viewport-mobile-fan" : ""}`}>
          <div
            ref={handScrollRef}
            className={`hand-zone-scroll min-h-0 h-full w-full min-w-0 -mx-2 px-2 overflow-x-auto overflow-y-hidden overscroll-x-contain ${isRoulette ? "hand-zone-scroll-roulette" : ""} ${isMobileFan ? "hand-zone-scroll-mobile-fan" : ""}`}
            onScroll={handleRouletteScroll}
            onPointerMove={handleHandPointerMove}
            onPointerLeave={handleHandPointerLeave}
          >
            <div
              ref={handListRef}
              className={`hand-zone-row flex min-h-full w-max flex-nowrap items-end pt-1 pb-2 overflow-visible ${isRoulette ? "hand-zone-row-roulette justify-start px-1.5" : "mx-auto min-w-full justify-center pl-4 pr-4"} ${isMobileFan ? "hand-zone-row-mobile-fan" : ""}`}
            >
              {rouletteCycleIndexes.map((cycleIndex) => (
                <Fragment key={`cycle-${cycleIndex}`}>
                  <div
                    ref={cycleIndex === HAND_ROULETTE_CENTER_CYCLE ? centerCycleRef : null}
                    className="hand-zone-cycle flex min-h-full flex-nowrap items-end overflow-visible"
                  >
                    {handEntries.map((entry) => renderHandEntry(entry, cycleIndex))}
                  </div>
                  {isRoulette && cycleIndex < HAND_ROULETTE_CYCLE_COUNT - 1 && (
                    <div
                      aria-hidden="true"
                      className="hand-zone-cycle-gap shrink-0"
                      style={{ width: `${HAND_ROULETTE_WRAP_GAP}px` }}
                    />
                  )}
                </Fragment>
              ))}

              {handCards.length === 0 && extraCards.length === 0 && (
                <div className="text-muted-foreground text-[17px] p-3 italic">Empty hand</div>
              )}
            </div>
          </div>
        </div>
      </section>
    );
  }

  // Opponent hand - show card backs
  const backs = Math.min(player.hand_size, 8);
  return (
    <section className="border border-[#41566f] bg-[#10161f] p-2 grid gap-1.5 h-full overflow-hidden" style={{ gridTemplateRows: "auto minmax(0,1fr)" }}>
      <h3 className="m-0 text-[#a4bdd7] uppercase tracking-wider text-[16px] font-semibold">
        Hand ({player.hand_size})
      </h3>
      <div className="flex gap-1.5 flex-nowrap pb-0.5 items-end min-h-0 overflow-hidden">
        {backs > 0
          ? Array.from({ length: backs }, (_, i) => (
              <div
                key={i}
                className="game-card p-1 text-[14px] grid content-end"
                style={{
                  width: `${handDimensions.cardW}px`,
                  minWidth: `${handDimensions.cardW}px`,
                  height: `${handDimensions.cardH}px`,
                  minHeight: `${handDimensions.cardH}px`,
                }}
              >
                <span className="card-label text-muted-foreground">Card</span>
              </div>
            ))
          : <div className="text-muted-foreground text-[17px] p-3 italic">Empty hand</div>
        }
      </div>
    </section>
  );
}
