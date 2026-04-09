import { startTransition, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useCombatArrows } from "@/context/useCombatArrows";
import { useDragActions } from "@/context/DragContext";
import { useHoverActions } from "@/context/HoverContext";
import useViewportLayout from "@/hooks/useViewportLayout";
import TableCore from "@/components/board/TableCore";
import HandZone from "@/components/board/HandZone";
import RightRail from "@/components/right-rail/RightRail";
import DragOverlay from "@/components/overlays/DragOverlay";
import CastParticles from "@/components/overlays/CastParticles";
import ArrowOverlay from "@/components/overlays/ArrowOverlay";
import { animate, cancelMotion, uiSpring } from "@/lib/motion/anime";
import { copyTextToClipboard } from "@/lib/clipboard";
import {
  buildStackTargetPresentation,
  getVisibleStackObjects,
  normalizeZoneViews,
} from "@/lib/stack-targets";

const HAND_PEEK_HEIGHT_DEFAULT = 46;
const HAND_REVEAL_HEIGHT_DEFAULT = 164;
const HAND_LANE_HOVER_FUZZ = 6;
const TRANSITION_TRACKED_ZONE_IDS = ["battlefield", "hand", "graveyard", "exile", "command"];
const SINGLE_ACTION_AUTO_DROP_MIN_DISTANCE_SQ = 18 * 18;

const ZONE_TRANSITION_LABELS = {
  battlefield: "Battlefield",
  hand: "Hand",
  graveyard: "Graveyard",
  exile: "Exile",
  command: "Command",
  hidden: "Hidden",
};

function objectExistsInState(state, objectId) {
  if (!state || objectId == null) return false;
  const needle = String(objectId);
  const players = state?.players || [];

  for (const player of players) {
    const zones = [
      player?.battlefield || [],
      player?.hand_cards || [],
      player?.graveyard_cards || [],
      player?.exile_cards || [],
      player?.command_cards || [],
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

  if ((state?.viewed_cards?.card_ids || []).some((id) => String(id) === needle)) {
    return true;
  }

  return false;
}

function shouldExpandInlineInspector(players, objectId) {
  if (!Array.isArray(players) || objectId == null) return false;
  const needle = String(objectId);

  for (const player of players) {
    if ((player?.hand_cards || []).some((card) => String(card?.id) === needle)) {
      return true;
    }

    for (const zone of [player?.graveyard_cards || [], player?.exile_cards || [], player?.command_cards || []]) {
      for (const card of zone) {
        if (String(card?.id) === needle && card?.show_in_pseudo_hand) {
          return true;
        }
      }
    }
  }

  return false;
}

function rectContainsPoint(rect, x, y, fuzz = 0) {
  if (!rect) return false;
  return (
    x >= (rect.left - fuzz)
    && x <= (rect.right + fuzz)
    && y >= (rect.top - fuzz)
    && y <= (rect.bottom + fuzz)
  );
}

function rectIntersectsRect(a, b, fuzz = 0) {
  if (!a || !b) return false;
  return !(
    a.right < (b.left - fuzz)
    || a.left > (b.right + fuzz)
    || a.bottom < (b.top - fuzz)
    || a.top > (b.bottom + fuzz)
  );
}

function getMobileDragPreviewRect(dragState) {
  if (!dragState) return null;
  const x = Number(dragState.currentX);
  const y = Number(dragState.currentY);
  if (!Number.isFinite(x) || !Number.isFinite(y)) return null;

  // The mobile drag preview is rendered at 180x140 and translated by about
  // half its width / 60% of its height relative to the pointer.
  return {
    left: x - 90,
    right: x + 90,
    top: y - 84,
    bottom: y + 56,
  };
}

function stackSelectionKeys(entry) {
  const keys = [entry?.id, entry?.inspect_object_id]
    .filter((value) => value != null)
    .map((value) => String(value));
  return Array.from(new Set(keys));
}

function getTrackedZoneCards(player, zone) {
  switch (zone) {
    case "battlefield":
      return player?.battlefield || [];
    case "hand":
      return player?.hand_cards || [];
    case "graveyard":
      return player?.graveyard_cards || [];
    case "exile":
      return player?.exile_cards || [];
    case "command":
      return player?.command_cards || [];
    default:
      return [];
  }
}

function cloneZoneCardSnapshot(card) {
  if (!card || typeof card !== "object") return null;
  return {
    ...card,
    member_ids: Array.isArray(card.member_ids) ? [...card.member_ids] : card.member_ids,
    member_stable_ids: Array.isArray(card.member_stable_ids)
      ? [...card.member_stable_ids]
      : card.member_stable_ids,
  };
}

function collectCardTrackingKeys(card) {
  const keys = [];
  if (Array.isArray(card?.member_stable_ids) && card.member_stable_ids.length > 0) {
    for (const stableId of card.member_stable_ids) {
      const normalized = Number(stableId);
      if (Number.isFinite(normalized)) {
        keys.push(`stable:${normalized}`);
      }
    }
  }
  const stableId = Number(card?.stable_id);
  if (Number.isFinite(stableId)) {
    keys.push(`stable:${stableId}`);
  }
  const objectId = Number(card?.id);
  if (Number.isFinite(objectId)) {
    keys.push(`object:${objectId}`);
  }
  return Array.from(new Set(keys));
}

function zoneTransitionLabel(zone) {
  return ZONE_TRANSITION_LABELS[String(zone || "").toLowerCase()] || "Hidden";
}

function shouldShowTransitionPreviewForZones(fromZone, toZone) {
  if (fromZone === toZone) return false;
  if (fromZone === "hidden" && toZone === "hidden") return false;
  if ((fromZone === "hidden" && toZone === "hand") || (fromZone === "hand" && toZone === "hidden")) {
    return false;
  }
  return true;
}

function buildZoneTransitionSnapshot(players) {
  const snapshot = {};
  for (const player of players || []) {
    const playerKey = String(player?.id ?? player?.index ?? "");
    if (!playerKey) continue;
    snapshot[playerKey] = {};
    for (const zone of TRANSITION_TRACKED_ZONE_IDS) {
      snapshot[playerKey][zone] = getTrackedZoneCards(player, zone)
        .map((card) => cloneZoneCardSnapshot(card))
        .filter(Boolean);
    }
  }
  return snapshot;
}

function buildZoneCardLocationMap(snapshot) {
  const locationMap = new Map();
  for (const zone of TRANSITION_TRACKED_ZONE_IDS) {
    for (const card of snapshot?.[zone] || []) {
      const trackingKeys = collectCardTrackingKeys(card);
      for (const key of trackingKeys) {
        if (locationMap.has(key)) continue;
        locationMap.set(key, { zone, card });
      }
    }
  }
  return locationMap;
}

function normalizeTransitionCardName(card) {
  return String(card?.name || "")
    .trim()
    .toLowerCase();
}

function buildTransitionCardFingerprint(card, relaxed = false) {
  const name = normalizeTransitionCardName(card);
  if (!name) return null;

  const owner = Number(card?.owner);
  const controller = Number(card?.controller);
  const typeLine = String(card?.type_line || "").trim().toLowerCase();
  const power = card?.power != null ? String(card.power) : "";
  const toughness = card?.toughness != null ? String(card.toughness) : "";

  if (relaxed) {
    return [
      name,
      Number.isFinite(owner) ? owner : "?",
    ].join("|");
  }

  return [
    name,
    Number.isFinite(owner) ? owner : "?",
    Number.isFinite(controller) ? controller : "?",
    typeLine,
    power,
    toughness,
  ].join("|");
}

function buildZoneCardEntries(snapshot) {
  const entries = [];
  for (const zone of TRANSITION_TRACKED_ZONE_IDS) {
    const zoneCards = Array.isArray(snapshot?.[zone]) ? snapshot[zone] : [];
    for (const [index, card] of zoneCards.entries()) {
      if (!card) continue;
      entries.push({
        entryKey: `${zone}:${index}`,
        zone,
        card,
        trackingKeys: collectCardTrackingKeys(card),
        strictFingerprint: buildTransitionCardFingerprint(card, false),
        relaxedFingerprint: buildTransitionCardFingerprint(card, true),
      });
    }
  }
  return entries;
}

function chooseFallbackTransitionMatch(previousEntry, candidateEntries) {
  if (!Array.isArray(candidateEntries) || candidateEntries.length === 0) return null;
  if (candidateEntries.length === 1) return candidateEntries[0];

  const preferredZones = previousEntry?.zone === "battlefield"
    ? ["graveyard", "exile", "command", "hand", "battlefield"]
    : [previousEntry?.zone, "graveyard", "exile", "command", "hand", "battlefield"];

  for (const zone of preferredZones) {
    const match = candidateEntries.find((entry) => entry.zone === zone);
    if (match) return match;
  }

  return candidateEntries[0];
}

function buildTransitionPreview(playerKey, previousEntry, currentEntry, tokenSeed) {
  const fromZone = previousEntry?.zone || "hidden";
  const toZone = currentEntry?.zone || "hidden";
  if (!shouldShowTransitionPreviewForZones(fromZone, toZone)) return null;

  const card = cloneZoneCardSnapshot(currentEntry?.card || previousEntry?.card);
  if (!card) return null;

  return {
    token: `${playerKey}:${tokenSeed}:${Date.now()}:${Math.random().toString(36).slice(2, 8)}`,
    objectId: currentEntry?.card?.id ?? previousEntry?.card?.id ?? null,
    card,
    title: `${zoneTransitionLabel(fromZone)} -> ${zoneTransitionLabel(toZone)}`,
  };
}

function buildZoneTransitionPreviews(previousSnapshot, currentSnapshot, playerKey) {
  const previousEntries = buildZoneCardEntries(previousSnapshot);
  const currentEntries = buildZoneCardEntries(currentSnapshot);
  const previousMatched = new Set();
  const currentMatched = new Set();
  const previews = [];

  const currentEntriesByTrackingKey = new Map();
  for (const entry of currentEntries) {
    for (const trackingKey of entry.trackingKeys) {
      if (!currentEntriesByTrackingKey.has(trackingKey)) {
        currentEntriesByTrackingKey.set(trackingKey, []);
      }
      currentEntriesByTrackingKey.get(trackingKey).push(entry);
    }
  }

  for (const previousEntry of previousEntries) {
    let matchedCurrentEntry = null;
    let matchedTrackingKey = null;

    for (const trackingKey of previousEntry.trackingKeys) {
      const candidates = (currentEntriesByTrackingKey.get(trackingKey) || [])
        .filter((entry) => !currentMatched.has(entry.entryKey));
      if (candidates.length === 0) continue;
      matchedCurrentEntry = candidates[0];
      matchedTrackingKey = trackingKey;
      break;
    }

    if (!matchedCurrentEntry) continue;
    previousMatched.add(previousEntry.entryKey);
    currentMatched.add(matchedCurrentEntry.entryKey);
    const preview = buildTransitionPreview(playerKey, previousEntry, matchedCurrentEntry, matchedTrackingKey);
    if (preview) previews.push(preview);
  }

  const unmatchedPreviousEntries = previousEntries.filter((entry) => !previousMatched.has(entry.entryKey));
  const unmatchedCurrentEntries = currentEntries.filter((entry) => !currentMatched.has(entry.entryKey));

  const currentEntriesByStrictFingerprint = new Map();
  const currentEntriesByRelaxedFingerprint = new Map();
  for (const entry of unmatchedCurrentEntries) {
    if (entry.strictFingerprint) {
      if (!currentEntriesByStrictFingerprint.has(entry.strictFingerprint)) {
        currentEntriesByStrictFingerprint.set(entry.strictFingerprint, []);
      }
      currentEntriesByStrictFingerprint.get(entry.strictFingerprint).push(entry);
    }
    if (entry.relaxedFingerprint) {
      if (!currentEntriesByRelaxedFingerprint.has(entry.relaxedFingerprint)) {
        currentEntriesByRelaxedFingerprint.set(entry.relaxedFingerprint, []);
      }
      currentEntriesByRelaxedFingerprint.get(entry.relaxedFingerprint).push(entry);
    }
  }

  for (const previousEntry of unmatchedPreviousEntries) {
    let matchedCurrentEntry = null;
    if (previousEntry.strictFingerprint) {
      matchedCurrentEntry = chooseFallbackTransitionMatch(
        previousEntry,
        (currentEntriesByStrictFingerprint.get(previousEntry.strictFingerprint) || [])
          .filter((entry) => !currentMatched.has(entry.entryKey))
      );
    }
    if (!matchedCurrentEntry && previousEntry.relaxedFingerprint) {
      matchedCurrentEntry = chooseFallbackTransitionMatch(
        previousEntry,
        (currentEntriesByRelaxedFingerprint.get(previousEntry.relaxedFingerprint) || [])
          .filter((entry) => !currentMatched.has(entry.entryKey))
      );
    }

    if (matchedCurrentEntry) {
      currentMatched.add(matchedCurrentEntry.entryKey);
      const preview = buildTransitionPreview(
        playerKey,
        previousEntry,
        matchedCurrentEntry,
        previousEntry.relaxedFingerprint || previousEntry.strictFingerprint || previousEntry.entryKey
      );
      if (preview) previews.push(preview);
      continue;
    }

    const hiddenPreview = buildTransitionPreview(playerKey, previousEntry, null, previousEntry.entryKey);
    if (hiddenPreview) previews.push(hiddenPreview);
  }

  for (const currentEntry of unmatchedCurrentEntries) {
    if (currentMatched.has(currentEntry.entryKey)) continue;
    const hiddenPreview = buildTransitionPreview(playerKey, null, currentEntry, currentEntry.entryKey);
    if (hiddenPreview) previews.push(hiddenPreview);
  }

  return previews;
}

export default function Workspace({
  zoneViews,
  deckLoadingMode,
  puzzleSetupMode = false,
  onLoadDecks,
  onCancelDeckLoading,
  onLoadPuzzle,
  onCancelPuzzleSetup,
  notices = [],
  onDismissNotice,
  mobileOpponentIndex = 0,
  setMobileOpponentIndex,
}) {
  const [selectedObjectId, setSelectedObjectId] = useState(null);
  const [focusedStackObjectId, setFocusedStackObjectId] = useState(null);
  const [pinnedInspectorObjectId, setPinnedInspectorObjectId] = useState(null);
  const [suppressFallbackInspector, setSuppressFallbackInspector] = useState(false);
  const [handLaneHovered, setHandLaneHovered] = useState(false);
  const [zoneActivityByPlayer, setZoneActivityByPlayer] = useState({});
  const [transientInspectorPreviews, setTransientInspectorPreviews] = useState([]);
  const [transientInspectorPreviewIndex, setTransientInspectorPreviewIndex] = useState(0);
  const [opponentsInspectorDockTop, setOpponentsInspectorDockTop] = useState(null);
  const [opponentsZoneHostRect, setOpponentsZoneHostRect] = useState(null);
  const [myZoneHostRect, setMyZoneHostRect] = useState(null);
  const workspaceRef = useRef(null);
  const previousStackIdsRef = useRef([]);
  const previousZoneTransitionSnapshotRef = useRef(null);
  const transitionInspectorRestoreRef = useRef(null);
  const handRevealShellRef = useRef(null);
  const handRevealMotionRef = useRef(null);
  const handHoverCloseTimerRef = useRef(null);
  const {
    game,
    state,
    dispatch,
    refresh,
    runWasmInteraction,
    setStatus,
    inspectorDebug,
    multiplayer,
  } = useGame();
  const { updateStackArrows, clearStackArrows } = useCombatArrows();
  const { endDrag } = useDragActions();
  const { clearHover, hoverCard } = useHoverActions();
  const { nonDesktopViewport, tabletCompactViewport, smallDesktopViewport, largeDesktopViewport } = useViewportLayout();
  const HAND_PEEK_HEIGHT = tabletCompactViewport ? 40 : (smallDesktopViewport ? 44 : (largeDesktopViewport ? 52 : HAND_PEEK_HEIGHT_DEFAULT));
  const HAND_REVEAL_HEIGHT = tabletCompactViewport ? 140 : (smallDesktopViewport ? 152 : (largeDesktopViewport ? 180 : HAND_REVEAL_HEIGHT_DEFAULT));
  const HAND_COLLAPSED_SHELL_HEIGHT = HAND_PEEK_HEIGHT;
  const showSideDocks = !nonDesktopViewport && !tabletCompactViewport && !smallDesktopViewport;
  const showTopDock = !nonDesktopViewport && !tabletCompactViewport;

  const players = useMemo(() => state?.players || [], [state?.players]);
  const perspective = state?.perspective;
  const me = players.find((p) => p.id === perspective) || players[0];
  const selectedObjectIsValid = objectExistsInState(state, selectedObjectId);
  const inlineInspectorExpanded = shouldExpandInlineInspector(players, selectedObjectId);
  const handLaneOpen = !nonDesktopViewport && handLaneHovered;
  const decision = state?.decision || null;
  const combatDeclarationActive = decision?.kind === "attackers" || decision?.kind === "blockers";
  const legalTargetObjectIds = useMemo(() => {
    const ids = new Set();
    if (!decision || decision.kind !== "targets") return ids;
    for (const req of decision.requirements || []) {
      for (const target of req.legal_targets || []) {
        if (target.kind === "object" && target.object != null) {
          ids.add(Number(target.object));
        }
      }
    }
    return ids;
  }, [decision]);
  const legalTargetPlayerIds = useMemo(() => {
    const ids = new Set();
    if (!decision || decision.kind !== "targets") return ids;
    for (const req of decision.requirements || []) {
      for (const target of req.legal_targets || []) {
        if (target.kind === "player" && target.player != null) {
          ids.add(Number(target.player));
        }
      }
    }
    return ids;
  }, [decision]);
  const stackTargetPresentation = useMemo(
    () => buildStackTargetPresentation(state, zoneViews, focusedStackObjectId ?? selectedObjectId),
    [focusedStackObjectId, selectedObjectId, state, zoneViews]
  );
  const temporaryZoneViews = useMemo(
    () => (combatDeclarationActive ? [] : stackTargetPresentation.temporaryZoneViews),
    [combatDeclarationActive, stackTargetPresentation.temporaryZoneViews]
  );
  const effectiveZoneViews = useMemo(() => {
    const merged = new Set(normalizeZoneViews(zoneViews));
    for (const zone of temporaryZoneViews) {
      merged.add(zone);
    }
    return normalizeZoneViews(Array.from(merged));
  }, [temporaryZoneViews, zoneViews]);
  const stackArrowSignature = useMemo(
    () => stackTargetPresentation.arrows.map((arrow) => arrow.key).join("|"),
    [stackTargetPresentation.arrows]
  );
  const hasTransientInspectorPreview = transientInspectorPreviews.length > 0;
  const activeTransientInspectorPreview = hasTransientInspectorPreview
    ? transientInspectorPreviews[Math.min(transientInspectorPreviewIndex, transientInspectorPreviews.length - 1)] || null
    : null;

  const clearTransientInspectorPreviews = useCallback(() => {
    transitionInspectorRestoreRef.current = null;
    setTransientInspectorPreviews([]);
    setTransientInspectorPreviewIndex(0);
  }, []);

  const restoreInspectorBeforeTransitionPreview = useCallback(() => {
    const restoreState = transitionInspectorRestoreRef.current;
    transitionInspectorRestoreRef.current = null;
    setTransientInspectorPreviews([]);
    setTransientInspectorPreviewIndex(0);
    if (!restoreState) return;

    setSelectedObjectId(restoreState.selectedObjectId);
    setFocusedStackObjectId(restoreState.focusedStackObjectId);
    setPinnedInspectorObjectId(restoreState.pinnedInspectorObjectId);
    setSuppressFallbackInspector(Boolean(restoreState.suppressFallbackInspector));
  }, []);

  const showTransitionInspectorPreviews = useCallback((previews) => {
    if (!Array.isArray(previews) || previews.length === 0) return;

    if (!transitionInspectorRestoreRef.current) {
      transitionInspectorRestoreRef.current = {
        selectedObjectId,
        focusedStackObjectId,
        pinnedInspectorObjectId,
        suppressFallbackInspector,
      };
    }

    setSuppressFallbackInspector(true);
    setTransientInspectorPreviews(previews);
    setTransientInspectorPreviewIndex(0);
    setZoneActivityByPlayer({});
  }, [focusedStackObjectId, pinnedInspectorObjectId, selectedObjectId, suppressFallbackInspector]);

  const showPreviousTransientInspectorPreview = useCallback(() => {
    setTransientInspectorPreviewIndex((currentIndex) => {
      const count = transientInspectorPreviews.length;
      if (count <= 1) return currentIndex;
      return (currentIndex - 1 + count) % count;
    });
  }, [transientInspectorPreviews.length]);

  const showNextTransientInspectorPreview = useCallback(() => {
    setTransientInspectorPreviewIndex((currentIndex) => {
      const count = transientInspectorPreviews.length;
      if (count <= 1) return currentIndex;
      return (currentIndex + 1) % count;
    });
  }, [transientInspectorPreviews.length]);

  useEffect(() => {
    if (selectedObjectId == null) return;
    if (selectedObjectIsValid) return;
    const invalidSelection = String(selectedObjectId);
    queueMicrotask(() => {
      setSelectedObjectId((currentSelection) => (
        String(currentSelection) === invalidSelection ? null : currentSelection
      ));
      setPinnedInspectorObjectId((currentPinned) => (
        currentPinned != null && String(currentPinned) === invalidSelection ? null : currentPinned
      ));
    });
  }, [selectedObjectId, selectedObjectIsValid]);

  useEffect(() => {
    const stackObjects = getVisibleStackObjects(state);
    const currentStackIds = stackObjects.flatMap((entry) => stackSelectionKeys(entry));
    const previousStackIds = previousStackIdsRef.current;
    const removedIds = previousStackIds.filter((id) => !currentStackIds.includes(id));

    if (
      removedIds.length > 0
      && selectedObjectId != null
      && !combatDeclarationActive
      && previousStackIds.includes(String(selectedObjectId))
    ) {
      const nextTopId = stackObjects[0]?.id ?? null;
      const selectedSnapshot = String(selectedObjectId);
      queueMicrotask(() => {
        setSelectedObjectId((currentSelection) => {
          if (String(currentSelection) !== selectedSnapshot) return currentSelection;
          return nextTopId;
        });
        setPinnedInspectorObjectId(null);
      });
    }

    previousStackIdsRef.current = currentStackIds;
  }, [state, selectedObjectId, combatDeclarationActive]);

  useEffect(() => {
    if (focusedStackObjectId == null) return;
    const visibleStackKeys = new Set(
      getVisibleStackObjects(state).flatMap((entry) => stackSelectionKeys(entry))
    );
    if (visibleStackKeys.has(String(focusedStackObjectId))) return;
    queueMicrotask(() => {
      setFocusedStackObjectId((currentFocused) => (
        String(currentFocused) === String(focusedStackObjectId) ? null : currentFocused
      ));
    });
  }, [focusedStackObjectId, state]);

  useEffect(() => {
    const currentSnapshot = buildZoneTransitionSnapshot(players);
    const previousSnapshot = previousZoneTransitionSnapshotRef.current;
    previousZoneTransitionSnapshotRef.current = currentSnapshot;

    if (deckLoadingMode || puzzleSetupMode || players.length === 0 || !previousSnapshot) {
      return;
    }

    if (Object.keys(previousSnapshot).length !== players.length) {
      return;
    }

    const nextPreviews = [];
    for (const player of players) {
      const playerKey = String(player?.id ?? player?.index ?? "");
      const previousPlayerSnapshot = previousSnapshot[playerKey];
      const currentPlayerSnapshot = currentSnapshot[playerKey];
      if (!previousPlayerSnapshot || !currentPlayerSnapshot) continue;

      nextPreviews.push(...buildZoneTransitionPreviews(
        previousPlayerSnapshot,
        currentPlayerSnapshot,
        playerKey
      ));
    }

    if (nextPreviews.length === 0) {
      return;
    }

    queueMicrotask(() => {
      startTransition(() => {
        showTransitionInspectorPreviews(nextPreviews);
      });
    });
  }, [deckLoadingMode, players, puzzleSetupMode, showTransitionInspectorPreviews]);

  useEffect(() => {
    if (!combatDeclarationActive) return;
    queueMicrotask(() => {
      clearTransientInspectorPreviews();
      setFocusedStackObjectId(null);
      setSelectedObjectId(null);
      setPinnedInspectorObjectId(null);
    });
  }, [clearTransientInspectorPreviews, combatDeclarationActive]);

  useEffect(() => {
    if (combatDeclarationActive || stackTargetPresentation.arrows.length === 0) {
      clearStackArrows();
      return undefined;
    }

    let firstFrameId = 0;
    let secondFrameId = 0;
    firstFrameId = window.requestAnimationFrame(() => {
      secondFrameId = window.requestAnimationFrame(() => {
        updateStackArrows(stackTargetPresentation.arrows);
      });
    });

    return () => {
      if (firstFrameId) window.cancelAnimationFrame(firstFrameId);
      if (secondFrameId) window.cancelAnimationFrame(secondFrameId);
    };
  }, [
    clearStackArrows,
    combatDeclarationActive,
    effectiveZoneViews,
    stackArrowSignature,
    stackTargetPresentation.arrows,
    updateStackArrows,
  ]);

  useEffect(() => () => {
    if (handHoverCloseTimerRef.current) {
      clearTimeout(handHoverCloseTimerRef.current);
      handHoverCloseTimerRef.current = null;
    }
  }, []);

  useLayoutEffect(() => {
    const shellEl = handRevealShellRef.current;
    if (!shellEl) return undefined;

    cancelMotion(handRevealMotionRef.current);
    handRevealMotionRef.current = animate(shellEl, {
      height: handLaneOpen ? HAND_REVEAL_HEIGHT : HAND_COLLAPSED_SHELL_HEIGHT,
      duration: 420,
      ease: uiSpring({ duration: 420, bounce: 0.16 }),
    });

    return () => {
      cancelMotion(handRevealMotionRef.current);
      handRevealMotionRef.current = null;
    };
  }, [handLaneOpen]);

  useLayoutEffect(() => {
    const root = workspaceRef.current;
    if (!root || deckLoadingMode || nonDesktopViewport) return undefined;

    let rafId = null;
    let resizeObserver = null;

    const measureDockTop = () => {
      const opponentsEl = root.querySelector("[data-opponents-zones]");
      const myZoneEl = root.querySelector("[data-my-zone]");
      if (!opponentsEl) {
        setOpponentsInspectorDockTop(null);
        setOpponentsZoneHostRect(null);
        return;
      }

      const opponentsRect = opponentsEl.getBoundingClientRect();
      const nextTop = Math.max(0, Math.round(opponentsRect.bottom - HAND_PEEK_HEIGHT));
      setOpponentsInspectorDockTop((currentTop) => (
        currentTop == null || Math.abs(currentTop - nextTop) >= 1 ? nextTop : currentTop
      ));
      const nextOpponentsRect = {
        top: Math.round(opponentsRect.top),
        height: Math.round(opponentsRect.height),
      };
      setOpponentsZoneHostRect((currentRect) => (
        currentRect == null
        || currentRect.top !== nextOpponentsRect.top
        || currentRect.height !== nextOpponentsRect.height
          ? nextOpponentsRect
          : currentRect
      ));
      if (!myZoneEl) {
        setMyZoneHostRect(null);
        return;
      }
      const myZoneRect = myZoneEl.getBoundingClientRect();
      const nextMyZoneRect = {
        top: Math.round(myZoneRect.top),
        height: Math.round(myZoneRect.height),
      };
      setMyZoneHostRect((currentRect) => (
        currentRect == null
        || currentRect.top !== nextMyZoneRect.top
        || currentRect.height !== nextMyZoneRect.height
          ? nextMyZoneRect
          : currentRect
      ));
    };

    const scheduleMeasure = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        rafId = null;
        measureDockTop();
      });
    };

    scheduleMeasure();

    resizeObserver = new ResizeObserver(scheduleMeasure);
    resizeObserver.observe(root);
    const tableEl = root.querySelector("[data-drop-zone]");
    const opponentsEl = root.querySelector("[data-opponents-zones]");
    if (tableEl) resizeObserver.observe(tableEl);
    if (opponentsEl) resizeObserver.observe(opponentsEl);
    window.addEventListener("resize", scheduleMeasure);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      resizeObserver?.disconnect();
      window.removeEventListener("resize", scheduleMeasure);
    };
  }, [deckLoadingMode, effectiveZoneViews, nonDesktopViewport, players.length]);

  const handleInspectObject = useCallback(
    async (objectId, options = null) => {
      if (combatDeclarationActive) return;
      if (
        decision?.kind === "targets"
        && decision.player === state?.perspective
        && objectId != null
        && legalTargetObjectIds.has(Number(objectId))
      ) {
        window.dispatchEvent(
          new CustomEvent("ironsmith:target-choice", {
            detail: { target: { kind: "object", object: Number(objectId) } },
          })
        );
        return;
      }
      if (
        decision?.kind === "select_objects"
        && decision.player === state?.perspective
      ) {
        const candidateIds = Array.isArray(options?.candidateObjectIds) && options.candidateObjectIds.length > 0
          ? options.candidateObjectIds
          : [objectId];
        const matchedCandidate = (decision.candidates || []).find((candidate) =>
          candidate?.legal !== false
          && candidateIds.some((candidateId) => String(candidate?.id) === String(candidateId))
        );
        if (matchedCandidate) {
          window.dispatchEvent(
            new CustomEvent("ironsmith:select-object-choice", {
              detail: { objectId: matchedCandidate.id },
            })
          );
          return;
        }
      }
      const stackEntry = options?.source === "stack" ? options?.stackEntry : null;
      if (
        stackEntry
        && !multiplayer.matchStarted
        && game
        && Number.isFinite(Number(stackEntry.controller))
        && Number(stackEntry.controller) !== Number(state?.perspective)
      ) {
        const nextPerspective = Number(stackEntry.controller);
        const perspectiveChanged = await runWasmInteraction(async () => {
          try {
            await game.setPerspective(nextPerspective);
            await refresh(`Viewing as player ${nextPerspective}`);
            return true;
          } catch (err) {
            setStatus(`Change player failed: ${err}`, true);
            return false;
          }
        });
        if (!perspectiveChanged) {
          return;
        }
      }
      clearTransientInspectorPreviews();
      setSelectedObjectId(objectId);
      setFocusedStackObjectId(stackEntry?.id != null ? String(stackEntry.id) : null);
      setPinnedInspectorObjectId(objectId == null ? null : String(objectId));
      setSuppressFallbackInspector(false);
      if (objectId != null) hoverCard(objectId);
    },
    [
      combatDeclarationActive,
      decision,
      game,
      hoverCard,
      legalTargetObjectIds,
      multiplayer.matchStarted,
      refresh,
      runWasmInteraction,
      setStatus,
      state?.perspective,
      clearTransientInspectorPreviews,
    ]
  );

  const handleFocusStackObject = useCallback((stackEntry) => {
    const stackObjectId = stackEntry?.id;
    if (stackObjectId == null) return;
    clearTransientInspectorPreviews();
    clearHover();
    setSelectedObjectId(null);
    setPinnedInspectorObjectId(null);
    setSuppressFallbackInspector(false);
    setFocusedStackObjectId((currentFocused) => (
      String(currentFocused) === String(stackObjectId)
        ? null
        : String(stackObjectId)
    ));
  }, [clearHover, clearTransientInspectorPreviews]);

  const mobileZoneHeaderControls = null;

  const handleNoticeCopy = useCallback(
    async (copyTarget) => {
      if (!copyTarget?.copyText) return;
      const copied = await copyTextToClipboard(copyTarget.copyText);
      if (copied) {
        setStatus(copyTarget.copyStatusMessage || "Copied to clipboard");
      } else {
        setStatus("Could not copy to clipboard", true);
      }
    },
    [setStatus]
  );

  const handleHandLaneEnter = useCallback(() => {
    if (handHoverCloseTimerRef.current) {
      clearTimeout(handHoverCloseTimerRef.current);
      handHoverCloseTimerRef.current = null;
    }
    setHandLaneHovered((currentHovered) => (currentHovered ? currentHovered : true));
  }, []);

  const handleHandLaneLeave = useCallback(() => {
    if (handHoverCloseTimerRef.current) {
      clearTimeout(handHoverCloseTimerRef.current);
    }
    handHoverCloseTimerRef.current = setTimeout(() => {
      setHandLaneHovered(false);
      handHoverCloseTimerRef.current = null;
    }, 90);
  }, []);

  const collapseHandLane = useCallback(() => {
    if (handHoverCloseTimerRef.current) {
      clearTimeout(handHoverCloseTimerRef.current);
      handHoverCloseTimerRef.current = null;
    }
    setHandLaneHovered((currentHovered) => (currentHovered ? false : currentHovered));
  }, []);

  useEffect(() => {
    const onPointerMove = (event) => {
      const shellEl = handRevealShellRef.current;
      if (!shellEl) return;

      const target = event.target;
      const insideHandLaneTarget = target instanceof Element
        && target.closest(".hand-reveal-shell");
      const insideExpandedShell = handLaneOpen && rectContainsPoint(
        shellEl.getBoundingClientRect(),
        event.clientX,
        event.clientY,
        HAND_LANE_HOVER_FUZZ
      );

      if (insideHandLaneTarget || insideExpandedShell) {
        handleHandLaneEnter();
        return;
      }

      if (handLaneOpen) {
        handleHandLaneLeave();
      }
    };

    document.addEventListener("pointermove", onPointerMove, { passive: true });
    return () => {
      document.removeEventListener("pointermove", onPointerMove);
    };
  }, [handLaneOpen, handleHandLaneEnter, handleHandLaneLeave]);

  // Handle drag drop — if user drops on the battlefield area, dispatch the action
  useEffect(() => {
    const onPointerUp = (e) => {
      const ds = endDrag();
      if (!ds || !ds.actions || ds.actions.length === 0) return;
      const currentDecision = state?.decision || null;
      if (currentDecision?.kind !== "priority") {
        return;
      }

      // Check if dropped over the table area (anywhere above the hand)
      const el = document.elementFromPoint(e.clientX, e.clientY);
      const isOverTable = !!el?.closest("[data-drop-zone]");

      let isOverMobileSelfZoneDropTarget = false;
      if (nonDesktopViewport) {
        const dropTargets = Array.from(
          document.querySelectorAll("[data-mobile-hand-drop-target]")
        );
        const previewRect = getMobileDragPreviewRect(ds);
        isOverMobileSelfZoneDropTarget = dropTargets.some((target) => {
          const rect = target.getBoundingClientRect();
          return (
            rectContainsPoint(rect, e.clientX, e.clientY, 8)
            || rectIntersectsRect(previewRect, rect, 8)
          );
        });
      }

      if (!isOverTable && !isOverMobileSelfZoneDropTarget) return;

      collapseHandLane();

      const currentActionIndices = new Set(
        (currentDecision.actions || []).map((action) => Number(action?.index))
      );

      if (ds.actions.length === 1) {
        const onlyAction = ds.actions[0];
        if (!currentActionIndices.has(Number(onlyAction?.index))) {
          return;
        }
        const dx = Number(ds.currentX) - Number(ds.startX);
        const dy = Number(ds.currentY) - Number(ds.startY);
        if (!Number.isFinite(dx) || !Number.isFinite(dy) || ((dx * dx) + (dy * dy)) < SINGLE_ACTION_AUTO_DROP_MIN_DISTANCE_SQ) {
          return;
        }
        window.__castParticles?.(e.clientX, e.clientY, ds.glowKind || "spell");
        dispatch(
          { type: "priority_action", action_index: onlyAction.index },
          onlyAction.label
        );
        if (!combatDeclarationActive && ds.objectId != null && !nonDesktopViewport) {
          setSelectedObjectId(ds.objectId);
          setPinnedInspectorObjectId(null);
          setSuppressFallbackInspector(false);
        }
        return;
      }

      // Multiple possible actions: pin inspector to this card while actions
      // remain available in the action strip.
      const hasCurrentAction = ds.actions.some((action) =>
        currentActionIndices.has(Number(action?.index))
      );
      if (!hasCurrentAction) {
        return;
      }
      if (!combatDeclarationActive) {
        setSelectedObjectId(ds.objectId != null ? ds.objectId : null);
        setPinnedInspectorObjectId(null);
        setSuppressFallbackInspector(false);
      }
      clearHover();
    };

    const onPointerCancel = () => {
      endDrag();
    };

    const onWindowBlur = () => {
      endDrag();
    };

    document.addEventListener("pointerup", onPointerUp);
    document.addEventListener("pointercancel", onPointerCancel);
    window.addEventListener("blur", onWindowBlur);
    return () => {
      document.removeEventListener("pointerup", onPointerUp);
      document.removeEventListener("pointercancel", onPointerCancel);
      window.removeEventListener("blur", onWindowBlur);
    };
  }, [
    clearHover,
    collapseHandLane,
    combatDeclarationActive,
    dispatch,
    endDrag,
    nonDesktopViewport,
    state?.decision,
  ]);

  useEffect(() => {
    const onDeadZonePointerDown = (event) => {
      if (event.button !== 0) return;
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (decision && decision.player === state?.perspective && decision.kind !== "priority") return;
      if (target.closest("[data-object-id]")) return;
      if (target.closest(".zone-viewer")) return;
      if (target.closest(".priority-inline-panel")) return;
      if (target.closest("button, input, label, a, [role='button']")) return;

      const inDeadZone = (
        target.closest("[data-drop-zone]")
        || target.closest(".table-gradient")
        || target.closest(".board-zone-bg")
      );
      if (!inDeadZone) return;

      if (hasTransientInspectorPreview) {
        clearHover();
        restoreInspectorBeforeTransitionPreview();
        return;
      }

      setSelectedObjectId(null);
      setPinnedInspectorObjectId(null);
      setSuppressFallbackInspector(true);
      clearHover();
    };

    document.addEventListener("pointerdown", onDeadZonePointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", onDeadZonePointerDown, true);
    };
  }, [
    clearHover,
    decision,
    hasTransientInspectorPreview,
    restoreInspectorBeforeTransitionPreview,
    state?.perspective,
  ]);

  return (
    <section
      ref={workspaceRef}
      className="relative min-h-0 h-full w-full min-w-0 overflow-visible"
      data-workspace-shell
    >
      <DragOverlay />
      <CastParticles />
      <ArrowOverlay />
      {notices.length > 0 && (
        <div className="absolute top-2 right-2 z-[120] flex max-w-[min(460px,clamp(52vw,58vw,65vw))] flex-col gap-2">
          {notices.map((notice) => {
            const toneClasses = notice.tone === "success"
              ? "workspace-notice workspace-notice--success"
              : notice.tone === "error"
                ? "workspace-notice workspace-notice--error"
                : notice.tone === "warning"
                  ? "workspace-notice workspace-notice--warning"
                : "workspace-notice workspace-notice--info";
            const actions = Array.isArray(notice.actions)
              ? notice.actions.filter((action) => action?.copyText)
              : [];
            const clickable = Boolean(notice.copyText) && actions.length === 0;
            return (
              <div
                key={notice.id}
                className={`relative overflow-hidden border shadow-[0_10px_26px_rgba(0,0,0,0.45)] ${toneClasses}`}
              >
                {clickable ? (
                  <button
                    type="button"
                    className="workspace-notice-body w-full px-3 py-2 pr-9 text-left transition-colors"
                    onClick={() => handleNoticeCopy(notice)}
                    title="Click to copy"
                  >
                    <div className="workspace-notice-title text-[13px] font-bold uppercase tracking-wide">
                      {notice.title}
                    </div>
                    {notice.body ? (
                      <div className="workspace-notice-text mt-1 text-[13px] font-semibold leading-tight">
                        {notice.body}
                      </div>
                    ) : null}
                  </button>
                ) : (
                  <div className="workspace-notice-body px-3 py-2 pr-9 text-left">
                    <div className="workspace-notice-title text-[13px] font-bold uppercase tracking-wide">
                      {notice.title}
                    </div>
                    {notice.body ? (
                      <div className="workspace-notice-text mt-1 text-[13px] font-semibold leading-tight">
                        {notice.body}
                      </div>
                    ) : null}
                  </div>
                )}
                {actions.length > 0 ? (
                  <div className="flex gap-2 overflow-x-auto px-3 pb-3 pr-9">
                    {actions.map((action, index) => (
                      <button
                        key={`${notice.id}:${action.label}:${index}`}
                        type="button"
                        className="workspace-notice-action shrink-0 border px-2.5 py-1 text-[11px] font-bold uppercase tracking-wide transition-colors"
                        onClick={() => handleNoticeCopy(action)}
                        title={action.label}
                      >
                        {action.label}
                      </button>
                    ))}
                  </div>
                ) : null}
                <button
                  type="button"
                  className="workspace-notice-dismiss absolute right-1.5 top-1.5 px-1 text-[12px] font-bold text-current opacity-80 transition-opacity hover:opacity-100"
                  onClick={() => onDismissNotice?.(notice.id)}
                  aria-label={`Dismiss ${notice.title}`}
                >
                  x
                </button>
              </div>
            );
          })}
        </div>
      )}
      <div className="min-h-0 h-full overflow-visible">
        <TableCore
          selectedObjectId={selectedObjectId}
          onInspect={handleInspectObject}
          focusedStackObjectId={focusedStackObjectId}
          onFocusStackObject={handleFocusStackObject}
          zoneViews={effectiveZoneViews}
          zoneActivityByPlayer={zoneActivityByPlayer}
          deckLoadingMode={deckLoadingMode}
          puzzleSetupMode={puzzleSetupMode}
          onLoadDecks={onLoadDecks}
          onCancelDeckLoading={onCancelDeckLoading}
          onLoadPuzzle={onLoadPuzzle}
          onCancelPuzzleSetup={onCancelPuzzleSetup}
          legalTargetPlayerIds={legalTargetPlayerIds}
          legalTargetObjectIds={legalTargetObjectIds}
          myZoneHeaderControls={mobileZoneHeaderControls}
          mobileOpponentIndex={mobileOpponentIndex}
          setMobileOpponentIndex={setMobileOpponentIndex}
        />
      </div>
      {showTopDock && !deckLoadingMode && opponentsInspectorDockTop != null && (
        <div
          className="pointer-events-none fixed inset-x-0 z-30 flex items-end justify-end overflow-visible px-2"
          style={{ top: `${opponentsInspectorDockTop}px`, height: `${HAND_PEEK_HEIGHT}px` }}
          data-inspector-dock="top"
          data-opponents-inspector-dock
        >
          <div className="pointer-events-none relative flex shrink-0 items-end gap-1.5 self-end overflow-visible">
          <RightRail
              pinnedObjectId={pinnedInspectorObjectId}
              transientInspectorPreview={activeTransientInspectorPreview}
              transientInspectorPreviewIndex={transientInspectorPreviewIndex}
              transientInspectorPreviewCount={transientInspectorPreviews.length}
              onShowPreviousTransientInspectorPreview={showPreviousTransientInspectorPreview}
              onShowNextTransientInspectorPreview={showNextTransientInspectorPreview}
              onInspectObject={handleInspectObject}
              suppressFallback={suppressFallbackInspector}
              inline
              inlineDockPlacement="top"
              allowTopInlinePlacement
              inlineExpanded={inlineInspectorExpanded}
            />
            {inspectorDebug && (
              <RightRail
                pinnedObjectId={pinnedInspectorObjectId}
                transientInspectorPreview={activeTransientInspectorPreview}
                transientInspectorPreviewIndex={transientInspectorPreviewIndex}
                transientInspectorPreviewCount={transientInspectorPreviews.length}
                onShowPreviousTransientInspectorPreview={showPreviousTransientInspectorPreview}
                onShowNextTransientInspectorPreview={showNextTransientInspectorPreview}
                onInspectObject={handleInspectObject}
                suppressFallback={suppressFallbackInspector}
                inline
                inlineDockPlacement="top"
                allowTopInlinePlacement
                dockRole="opposite"
                inspectorVariant="debug"
                inlineExpanded={inlineInspectorExpanded}
              />
            )}
          </div>
        </div>
      )}
      {showSideDocks && !deckLoadingMode && opponentsZoneHostRect != null && (
        <div
          className="pointer-events-none fixed inset-x-0 z-30 flex items-start justify-start overflow-visible px-2"
          style={{
            top: `${opponentsZoneHostRect.top}px`,
            height: `${opponentsZoneHostRect.height}px`,
          }}
        >
          <div className="pointer-events-none relative flex h-full shrink-0 items-start gap-1.5 self-start overflow-visible">
            <RightRail
              pinnedObjectId={pinnedInspectorObjectId}
              transientInspectorPreview={activeTransientInspectorPreview}
              transientInspectorPreviewIndex={transientInspectorPreviewIndex}
              transientInspectorPreviewCount={transientInspectorPreviews.length}
              onShowPreviousTransientInspectorPreview={showPreviousTransientInspectorPreview}
              onShowNextTransientInspectorPreview={showNextTransientInspectorPreview}
              onInspectObject={handleInspectObject}
              suppressFallback={suppressFallbackInspector}
              inline
              inlineDockPlacement="top"
              inlineHostSide="left"
              inlineExpandedSide="left"
              allowTopInlinePlacement
              inlineExpanded={inlineInspectorExpanded}
            />
            {inspectorDebug && (
              <RightRail
                pinnedObjectId={pinnedInspectorObjectId}
                transientInspectorPreview={activeTransientInspectorPreview}
                transientInspectorPreviewIndex={transientInspectorPreviewIndex}
                transientInspectorPreviewCount={transientInspectorPreviews.length}
                onShowPreviousTransientInspectorPreview={showPreviousTransientInspectorPreview}
                onShowNextTransientInspectorPreview={showNextTransientInspectorPreview}
                onInspectObject={handleInspectObject}
                suppressFallback={suppressFallbackInspector}
                inline
                inlineDockPlacement="top"
                inlineHostSide="left"
                inlineExpandedSide="left"
                allowTopInlinePlacement
                dockRole="opposite"
                inspectorVariant="debug"
                inlineExpanded={inlineInspectorExpanded}
              />
            )}
          </div>
        </div>
      )}
      {!nonDesktopViewport && !puzzleSetupMode && (
        <div
          className="pointer-events-none fixed inset-x-0 bottom-2 z-30 flex items-end gap-1.5 overflow-visible px-2"
          style={{ height: `${HAND_PEEK_HEIGHT}px` }}
          data-bottom-dock
          data-inspector-dock="bottom"
        >
          <div
            className="pointer-events-none relative min-w-0 flex-1 h-full overflow-visible"
            data-hand-dock-lane
          >
            <div
              ref={handRevealShellRef}
              className="hand-reveal-shell absolute left-0 bottom-0"
              data-open={handLaneOpen ? "true" : "false"}
              aria-expanded={handLaneOpen}
              style={{
                height: `${handLaneOpen ? HAND_REVEAL_HEIGHT : HAND_COLLAPSED_SHELL_HEIGHT}px`,
                "--hand-shell-offset-x": "3vw",
              }}
              onMouseEnter={handleHandLaneEnter}
              onMouseLeave={handleHandLaneLeave}
              onFocusCapture={handleHandLaneEnter}
              onBlurCapture={(event) => {
                if (event.currentTarget.contains(event.relatedTarget)) return;
                handleHandLaneLeave();
              }}
            >
              <div
                className="hand-reveal-body"
                style={{ height: "100%" }}
              >
                <HandZone
                  player={me}
                  selectedObjectId={selectedObjectId}
                  onInspect={handleInspectObject}
                  isExpanded={handLaneOpen}
                />
              </div>
            </div>
          </div>
          <div className="pointer-events-none relative flex shrink-0 items-end gap-1.5 self-end overflow-visible">
            <RightRail
              pinnedObjectId={pinnedInspectorObjectId}
              transientInspectorPreview={activeTransientInspectorPreview}
              transientInspectorPreviewIndex={transientInspectorPreviewIndex}
              transientInspectorPreviewCount={transientInspectorPreviews.length}
              onShowPreviousTransientInspectorPreview={showPreviousTransientInspectorPreview}
              onShowNextTransientInspectorPreview={showNextTransientInspectorPreview}
              onInspectObject={handleInspectObject}
              suppressFallback={suppressFallbackInspector}
              inline
              allowTopInlinePlacement={opponentsInspectorDockTop != null}
              inlineExpanded={inlineInspectorExpanded}
            />
            {inspectorDebug && (
              <RightRail
                pinnedObjectId={pinnedInspectorObjectId}
                transientInspectorPreview={activeTransientInspectorPreview}
                transientInspectorPreviewIndex={transientInspectorPreviewIndex}
                transientInspectorPreviewCount={transientInspectorPreviews.length}
                onShowPreviousTransientInspectorPreview={showPreviousTransientInspectorPreview}
                onShowNextTransientInspectorPreview={showNextTransientInspectorPreview}
                onInspectObject={handleInspectObject}
                suppressFallback={suppressFallbackInspector}
                inline
                allowTopInlinePlacement={opponentsInspectorDockTop != null}
                dockRole="opposite"
                inspectorVariant="debug"
                inlineExpanded={inlineInspectorExpanded}
              />
            )}
          </div>
        </div>
      )}
      {showSideDocks && !deckLoadingMode && myZoneHostRect != null && (
        <div
          className="pointer-events-none fixed inset-x-0 z-30 flex items-start justify-start overflow-visible px-2"
          style={{
            top: `${myZoneHostRect.top}px`,
            height: `${myZoneHostRect.height}px`,
          }}
        >
          <div className="pointer-events-none relative flex h-full shrink-0 items-start gap-1.5 self-start overflow-visible">
            <RightRail
              pinnedObjectId={pinnedInspectorObjectId}
              transientInspectorPreview={activeTransientInspectorPreview}
              transientInspectorPreviewIndex={transientInspectorPreviewIndex}
              transientInspectorPreviewCount={transientInspectorPreviews.length}
              onShowPreviousTransientInspectorPreview={showPreviousTransientInspectorPreview}
              onShowNextTransientInspectorPreview={showNextTransientInspectorPreview}
              onInspectObject={handleInspectObject}
              suppressFallback={suppressFallbackInspector}
              inline
              inlineHostSide="left"
              inlineExpandedSide="left"
              allowTopInlinePlacement={opponentsInspectorDockTop != null}
              inlineExpanded={inlineInspectorExpanded}
            />
            {inspectorDebug && (
              <RightRail
                pinnedObjectId={pinnedInspectorObjectId}
                transientInspectorPreview={activeTransientInspectorPreview}
                transientInspectorPreviewIndex={transientInspectorPreviewIndex}
                transientInspectorPreviewCount={transientInspectorPreviews.length}
                onShowPreviousTransientInspectorPreview={showPreviousTransientInspectorPreview}
                onShowNextTransientInspectorPreview={showNextTransientInspectorPreview}
                onInspectObject={handleInspectObject}
                suppressFallback={suppressFallbackInspector}
                inline
                inlineHostSide="left"
                inlineExpandedSide="left"
                allowTopInlinePlacement={opponentsInspectorDockTop != null}
                dockRole="opposite"
                inspectorVariant="debug"
                inlineExpanded={inlineInspectorExpanded}
              />
            )}
          </div>
        </div>
      )}
    </section>
  );
}
