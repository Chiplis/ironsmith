import { useRef, useLayoutEffect, useEffect, useCallback, useMemo, useState } from "react";
import { Undo2 } from "lucide-react";
import { useHover } from "@/context/HoverContext";
import { useCombatArrows } from "@/context/useCombatArrows";
import { useGame } from "@/context/GameContext";
import useNewCards from "@/hooks/useNewCards";
import { cancelMotion, createTimeline, uiSpring } from "@/lib/motion/anime";
import {
  ALL_PAPER_LANES,
  PAPER_BACK_LANES,
  PAPER_FRONT_LANES,
  normalizeBattlefieldLane,
} from "@/lib/battlefield-layout";
import { isTriggerOrderingDecision } from "@/lib/trigger-ordering";
import GameCard from "@/components/cards/GameCard";
import { Button } from "@/components/ui/button";

const BOTTOM_BATTLEFIELD_SAFE_INSET = 60;
const LIVE_DAMAGE_ANIMATION_MS = 300;
const GHOST_BASE_ANIMATION_MS = 520;
const MAX_BATTLEFIELD_CARD_ZONE_WIDTH_RATIO = 0.15;
const BATTLEFIELD_GRID_GAP_PX = 4;
const COMPACT_SCROLL_COLUMN_MAX_WIDTH = 200;
const ABSOLUTE_MIN_CARD_WIDTH = 10;
const ABSOLUTE_MIN_CARD_HEIGHT = 14;
const EMPTY_PAPER_SLOT_COLUMNS = 6;
const MOBILE_OBJECT_LONG_PRESS_MS = 380;
const MOBILE_BOTTOM_BACK_ROW_TRANSLATE_Y_PX = 32;
const MOBILE_BOTTOM_MIN_VISIBLE_BACK_ROW_RATIO = 0.6;
const MOBILE_BOTTOM_BACK_ROW_SCALE = 0.96;
const MOBILE_BOTTOM_DOCK_CLEARANCE_PX = 10;
const MOBILE_BATTLEFIELD_TOKEN_HIT_SLOP_X = 16;
const MOBILE_BATTLEFIELD_TOKEN_HIT_SLOP_Y = 16;

function buildPaperRowGroups(battlefieldSide, buckets, options = {}) {
  const singleRow = options.singleRow === true;
  const mobileBattleMode = options.mobileBattleMode || "default";
  const minSlotsPerRow = Math.max(1, Number(options.minSlotsPerRow) || EMPTY_PAPER_SLOT_COLUMNS);
  if (singleRow) {
    return [
      { id: "main", lanes: ALL_PAPER_LANES, rowCount: 1, minSlotsPerRow },
    ];
  }
  if (mobileBattleMode === "top-dense") {
    return [
      { id: "back", lanes: PAPER_BACK_LANES, rowCount: 1, minSlotsPerRow: Math.max(minSlotsPerRow, 7) },
      { id: "front", lanes: PAPER_FRONT_LANES, rowCount: 1, minSlotsPerRow: Math.max(minSlotsPerRow, 5) },
    ];
  }
  if (mobileBattleMode === "bottom-dense") {
    return [
      { id: "front", lanes: PAPER_FRONT_LANES, rowCount: 1, minSlotsPerRow: Math.max(minSlotsPerRow, 5) },
      { id: "back", lanes: PAPER_BACK_LANES, rowCount: 1, minSlotsPerRow: Math.max(minSlotsPerRow, 7) },
    ];
  }
  const frontCount = PAPER_FRONT_LANES.reduce((total, lane) => total + ((buckets.get(lane) || []).length), 0);
  const backCount = PAPER_BACK_LANES.reduce((total, lane) => total + ((buckets.get(lane) || []).length), 0);
  const shouldSplitOpponentRows = battlefieldSide === "top"
    && (frontCount > EMPTY_PAPER_SLOT_COLUMNS || backCount > EMPTY_PAPER_SLOT_COLUMNS);

  return shouldSplitOpponentRows
    ? [
      { id: "front", lanes: PAPER_FRONT_LANES, rowCount: 2, minSlotsPerRow: EMPTY_PAPER_SLOT_COLUMNS },
      { id: "back", lanes: PAPER_BACK_LANES, rowCount: 2, minSlotsPerRow: EMPTY_PAPER_SLOT_COLUMNS },
    ]
    : [
      { id: "front", lanes: PAPER_FRONT_LANES, rowCount: 1, minSlotsPerRow: EMPTY_PAPER_SLOT_COLUMNS },
      { id: "back", lanes: PAPER_BACK_LANES, rowCount: 1, minSlotsPerRow: EMPTY_PAPER_SLOT_COLUMNS },
    ];
}

function splitCardsIntoRows(cards, rowCount) {
  const rows = Array.from({ length: rowCount }, () => []);
  if (!Array.isArray(cards) || cards.length === 0) return rows;
  const chunkSize = Math.max(1, Math.ceil(cards.length / rowCount));
  for (let index = 0; index < cards.length; index += 1) {
    const rowIndex = Math.min(rowCount - 1, Math.floor(index / chunkSize));
    rows[rowIndex].push(cards[index]);
  }
  return rows;
}

function buildPaperBattlefieldLayout(cards, battlefieldSide, alignStart = false, options = {}) {
  const buckets = new Map(ALL_PAPER_LANES.map((lane) => [lane, []]));

  for (const card of cards) {
    const lane = normalizeBattlefieldLane(card?.lane);
    buckets.get(lane).push(card);
  }
  const rowGroups = buildPaperRowGroups(battlefieldSide, buckets, options);

  const orderedRows = rowGroups.flatMap((group) => {
    const groupedCards = group.lanes.flatMap((lane) => buckets.get(lane) || []);
    const splitRows = splitCardsIntoRows(groupedCards, group.rowCount);
    return splitRows.map((rowCards, rowIndex) => ({
      id: `${group.id}-${rowIndex + 1}`,
      groupId: group.id,
      cards: rowCards,
      minSlots: group.minSlotsPerRow,
      signature: `${group.id}:${rowIndex + 1}:${group.lanes.map((lane) => `${lane}:${(buckets.get(lane) || []).length}`).join(",")}:${rowCards.length}`,
    }));
  });
  const orderedCards = [];
  const gridPositionById = new Map();
  const slotCells = [];
  const maxCols = Math.max(
    1,
    ...orderedRows.map((row) => Math.max(row.cards.length, row.minSlots || 0)),
    cards.length === 0 ? EMPTY_PAPER_SLOT_COLUMNS : 0
  );

  orderedRows.forEach((row, rowIndex) => {
    for (let column = 1; column <= maxCols; column += 1) {
      slotCells.push({
        key: `${row.id}-slot-${column}`,
        row: rowIndex + 1,
        column,
        groupId: row.groupId,
      });
    }
    if (row.cards.length === 0) return;
    const startColumn = alignStart
      ? 1
      : Math.floor((maxCols - row.cards.length) / 2) + 1;

    row.cards.forEach((card, columnIndex) => {
      orderedCards.push(card);
      gridPositionById.set(String(card.id), {
        row: rowIndex + 1,
        column: startColumn + columnIndex,
        groupId: row.groupId,
      });
    });
  });

  return {
    orderedCards,
    gridPositionById,
    slotCells,
    rowCount: orderedRows.length,
    maxCols,
    signature: orderedRows
      .map((row) => `${row.id}:${row.signature}`)
      .join("|"),
  };
}

function stableIdsForCard(card) {
  if (Array.isArray(card?.member_stable_ids) && card.member_stable_ids.length > 0) {
    return card.member_stable_ids.map((stableId) => String(stableId));
  }
  if (card?.stable_id != null) return [String(card.stable_id)];
  if (card?.id != null) return [String(card.id)];
  return [];
}

function collectActivatableActionsForCard(card, activatableMap) {
  if (!activatableMap || !card) return [];

  const actions = [];
  const seenActionIndices = new Set();
  const objectIds = [Number(card?.id)];
  if (Array.isArray(card?.member_ids)) {
    for (const memberId of card.member_ids) {
      objectIds.push(Number(memberId));
    }
  }

  for (const objectId of objectIds) {
    if (!Number.isFinite(objectId) || !activatableMap.has(objectId)) continue;
    for (const action of activatableMap.get(objectId) || []) {
      const actionIndex = Number(action?.index);
      if (Number.isFinite(actionIndex)) {
        if (seenActionIndices.has(actionIndex)) continue;
        seenActionIndices.add(actionIndex);
      }
      actions.push(action);
    }
  }

  return actions;
}

function indexCardsByStableId(cards) {
  const index = new Map();
  for (const card of cards || []) {
    for (const stableId of stableIdsForCard(card)) {
      index.set(String(stableId), card);
    }
  }
  return index;
}

function groupBattlefieldTransitions(transitions) {
  const grouped = new Map();
  for (const transition of transitions || []) {
    const stableId = transition?.stable_id == null ? null : String(transition.stable_id);
    if (!stableId) continue;
    const entry = grouped.get(stableId) || {
      stableId,
      damaged: false,
      leaveKind: null,
    };
    if (transition.kind === "damaged") {
      entry.damaged = true;
    } else if (
      transition.kind === "destroyed"
      || transition.kind === "sacrificed"
      || transition.kind === "exiled"
    ) {
      entry.leaveKind = transition.kind;
    }
    grouped.set(stableId, entry);
  }
  return grouped;
}

function cloneLeavingCard(card, stableId) {
  return {
    ...card,
    stable_id: Number(stableId),
    member_stable_ids: [Number(stableId)],
    count: 1,
  };
}

function normalizeNumericId(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function isTriggeredDecision(decision) {
  if (!decision) return false;
  if (isTriggerOrderingDecision(decision)) return true;

  const description = String(decision.description || "").toLowerCase();
  const reason = String(decision.reason || "").toLowerCase();
  const contextText = String(decision.context_text || "").toLowerCase();
  const consequenceText = String(decision.consequence_text || "").toLowerCase();
  const sourceName = String(decision.source_name || "").toLowerCase();

  return (
    description.includes("trigger")
    || reason.includes("trigger")
    || contextText.includes("trigger")
    || consequenceText.includes("trigger")
    || sourceName.includes("triggered ability")
  );
}

function resetLiveCardFxVars(node) {
  if (!node) return;
  node.style.removeProperty("--card-jolt-x");
  node.style.removeProperty("--card-jolt-y");
  node.style.removeProperty("--card-jolt-scale");
  node.style.removeProperty("--card-flash-brightness");
  node.style.removeProperty("--card-flash-saturate");
}

function findCardElementForStableId(row, stableId) {
  if (!row || stableId == null) return null;
  const needle = String(stableId);
  const nodes = row.querySelectorAll(".battlefield-row-card[data-member-stable-ids]");
  for (const node of nodes) {
    const stableIds = String(node.dataset.memberStableIds || "")
      .split(",")
      .map((value) => value.trim())
      .filter(Boolean);
    if (stableIds.includes(needle)) return node;
  }
  return null;
}

function measureLiveCardPositions(row) {
  const positions = new Map();
  if (!row) return positions;
  const rowRect = row.getBoundingClientRect();
  const nodes = row.querySelectorAll(".battlefield-row-card[data-member-stable-ids]");
  for (const node of nodes) {
    const rect = node.getBoundingClientRect();
    const stableIds = String(node.dataset.memberStableIds || "")
      .split(",")
      .map((value) => value.trim())
      .filter(Boolean);
    const position = {
      left: rect.left - rowRect.left + row.scrollLeft,
      top: rect.top - rowRect.top + row.scrollTop,
      width: rect.width,
      height: rect.height,
    };
    for (const stableId of stableIds) {
      positions.set(stableId, position);
    }
  }
  return positions;
}

function computeMobileBottomOverlapPx(cardWidth) {
  return Math.min(20, Math.max(12, Math.floor(cardWidth * 0.24)));
}

function computePaperVisualGridWidth(cols, cardWidth, gap, overlapPx = 0) {
  if (!Number.isFinite(cols) || cols <= 0) return 0;
  if (!Number.isFinite(cardWidth) || cardWidth <= 0) return 0;
  const safeGap = Number.isFinite(gap) ? gap : 0;
  const safeOverlap = Number.isFinite(overlapPx) ? overlapPx : 0;
  return (cols * cardWidth) - (Math.max(0, cols - 1) * safeOverlap) + (Math.max(0, cols - 1) * safeGap);
}

function playLiveDamageAnimation(node, motionStore, stableId) {
  if (!node || stableId == null) return;
  const key = String(stableId);
  cancelMotion(motionStore.get(key));
  resetLiveCardFxVars(node);

  const motion = createTimeline({ autoplay: true })
    .add(node, {
      keyframes: [
        {
          "--card-flash-brightness": 1.55,
          "--card-flash-saturate": 1.45,
          "--card-jolt-scale": 1.045,
          duration: 90,
        },
        {
          "--card-flash-brightness": 1,
          "--card-flash-saturate": 1,
          "--card-jolt-scale": 1,
          duration: 210,
        },
      ],
      ease: uiSpring({ duration: LIVE_DAMAGE_ANIMATION_MS, bounce: 0.16 }),
    })
    .add(node, {
      keyframes: [
        { "--card-jolt-x": "-6px", "--card-jolt-y": "-1px", duration: 54 },
        { "--card-jolt-x": "5px", "--card-jolt-y": "1px", duration: 64 },
        { "--card-jolt-x": "-3px", "--card-jolt-y": "0px", duration: 72 },
        { "--card-jolt-x": "0px", "--card-jolt-y": "0px", duration: 110 },
      ],
      ease: "out(3)",
      onComplete: () => {
        resetLiveCardFxVars(node);
        motionStore.delete(key);
      },
    }, 0);

  motionStore.set(key, motion);
}

function BattlefieldGhostCard({ ghost, compact, onDone }) {
  const shellRef = useRef(null);
  const motionRef = useRef(null);

  useLayoutEffect(() => {
    const node = shellRef.current;
    if (!node) return undefined;

    cancelMotion(motionRef.current);
    node.style.opacity = "";
    node.style.transform = "";
    node.style.filter = "";

    const timeline = createTimeline({ autoplay: true });
    if (ghost.includeDamage) {
      timeline.add(node, {
        keyframes: [
          { scale: 1.05, filter: "brightness(1.6) saturate(1.45)", duration: 80 },
          { scale: 1, filter: "brightness(1.12) saturate(1.15)", duration: 100 },
        ],
        ease: "out(3)",
      });
    }

    if (ghost.kind === "sacrificed") {
      timeline.add(node, {
        keyframes: [
          { translateY: -4, scale: 1.02, duration: 100 },
          { translateY: 26, scale: 0.62, rotateZ: 7, opacity: 0, duration: ghost.duration },
        ],
        ease: uiSpring({ duration: ghost.duration, bounce: 0.08 }),
      });
    } else if (ghost.kind === "destroyed") {
      timeline.add(node, {
        keyframes: [
          { scale: 1.07, duration: 85 },
          {
            translateY: -16,
            scale: 0.74,
            rotateZ: -9,
            opacity: 0,
            filter: "brightness(1.7) saturate(0.38) blur(2px)",
            duration: ghost.duration,
          },
        ],
        ease: "out(4)",
      });
    } else {
      timeline.add(node, {
        keyframes: [
          { translateY: -10, scale: 1.03, duration: 95 },
          {
            translateY: -32,
            scale: 0.82,
            opacity: 0,
            filter: "brightness(1.9) saturate(0.18) blur(2.2px)",
            duration: ghost.duration,
          },
        ],
        ease: "out(3)",
      });
    }

    motionRef.current = timeline;
    const timeout = window.setTimeout(() => onDone?.(ghost.key), ghost.totalDuration);
    return () => {
      window.clearTimeout(timeout);
      cancelMotion(motionRef.current);
      motionRef.current = null;
    };
  }, [ghost, onDone]);

  return (
    <div
      ref={shellRef}
      className="pointer-events-none absolute z-[18]"
      style={{
        left: `${ghost.left}px`,
        top: `${ghost.top}px`,
        width: `${ghost.width}px`,
        height: `${ghost.height}px`,
        transformOrigin: "50% 50%",
      }}
    >
      <GameCard
        card={ghost.card}
        compact={compact}
        className="battlefield-ghost-card"
        hideDebugBadge
        style={{
          width: "100%",
          minWidth: "100%",
          height: "100%",
          minHeight: "100%",
        }}
      />
    </div>
  );
}

export default function BattlefieldRow({
  cards = [],
  compact = false,
  battlefieldSide = "bottom",
  alignStart = false,
  paperLayoutMode = "default",
  paperMinSlotsPerRow = null,
  layoutOverride = null,
  bottomSafeInset = BOTTOM_BATTLEFIELD_SAFE_INSET,
  bottomOcclusionViewportTop = null,
  selectedObjectId,
  onInspect,
  onCardClick,
  onCardPointerDown,
  onMobileCardActionMenu = null,
  onMobileCardLongPress = null,
  activatableMap,
  legalTargetObjectIds = new Set(),
  allowVerticalScroll = false,
  forceSingleColumn = false,
}) {
  const rowRef = useRef(null);
  const previousCardsRef = useRef(cards);
  const previousPositionsRef = useRef(new Map());
  const lastProcessedSnapshotIdRef = useRef(null);
  const liveDamageMotionsRef = useRef(new Map());
  const { state, cancelDecision } = useGame();
  const { hoverCard, clearHover, hoveredObjectId, hoveredLinkedObjectIds } = useHover();
  const { combatMode, combatModeRef, dragArrow, startDragArrow, updateDragArrow, endDragArrow } = useCombatArrows();
  const [ghosts, setGhosts] = useState([]);
  const isPaperBattlefieldLayout = !compact;
  const isMobileBattleTopLayout = paperLayoutMode === "mobile-battle-top";
  const isMobileBattleBottomLayout = paperLayoutMode === "mobile-battle-bottom";
  const canShowBattlefieldUndo = isPaperBattlefieldLayout && battlefieldSide === "bottom";
  const normalizedLayoutOverride = useMemo(() => {
    if (!layoutOverride || typeof layoutOverride !== "object") return null;
    const cols = Math.max(1, Math.floor(Number(layoutOverride.cols) || 1));
    const rows = Math.max(1, Math.floor(Number(layoutOverride.rows) || 1));
    const cardWidth = Math.max(ABSOLUTE_MIN_CARD_WIDTH, Math.floor(Number(layoutOverride.cardWidth) || 0));
    const cardHeight = Math.max(ABSOLUTE_MIN_CARD_HEIGHT, Math.floor(Number(layoutOverride.cardHeight) || 0));
    const overlapPx = Math.max(0, Math.floor(Number(layoutOverride.overlapPx) || 0));
    if (cardWidth <= 0 || cardHeight <= 0) return null;
    return {
      rows,
      cols,
      cardWidth,
      cardHeight,
      overlapPx,
      centerOffset: Math.floor(Number(layoutOverride.centerOffset) || 0),
    };
  }, [layoutOverride]);
  const isMobileBattleSingleRowLayout = paperLayoutMode === "single-row" && normalizedLayoutOverride != null;
  const useMobileBattlefieldToken = (
    isMobileBattleTopLayout
    || isMobileBattleBottomLayout
    || isMobileBattleSingleRowLayout
  );
  const suppressTooltip = isMobileBattleTopLayout || isMobileBattleBottomLayout || isMobileBattleSingleRowLayout;
  const paperLayout = useMemo(
    () => buildPaperBattlefieldLayout(cards, battlefieldSide, alignStart, {
      singleRow: paperLayoutMode === "single-row",
      mobileBattleMode:
        paperLayoutMode === "mobile-battle-top"
          ? "top-dense"
          : paperLayoutMode === "mobile-battle-bottom"
            ? "bottom-dense"
            : "default",
      minSlotsPerRow: paperMinSlotsPerRow,
    }),
    [alignStart, battlefieldSide, cards, paperLayoutMode, paperMinSlotsPerRow]
  );
  const displayCards = isPaperBattlefieldLayout ? paperLayout.orderedCards : cards;
  const displayCardById = useMemo(() => {
    const index = new Map();
    for (const card of displayCards) {
      index.set(String(card?.id), card);
    }
    return index;
  }, [displayCards]);
  const hasMobileBottomBackRowCards = useMemo(
    () => (
      isMobileBattleBottomLayout
      && displayCards.some((card) => paperLayout.gridPositionById.get(String(card.id))?.row === 2)
    ),
    [displayCards, isMobileBattleBottomLayout, paperLayout.gridPositionById]
  );
  const priorityActionObjectIds = useMemo(() => {
    const ids = new Set();
    const decision = state?.decision;
    if (!decision || decision.kind !== "priority" || decision.player !== state?.perspective) {
      return ids;
    }
    for (const action of decision.actions || []) {
      if (action.kind === "pass_priority" || action.object_id == null) continue;
      ids.add(String(action.object_id));
    }
    return ids;
  }, [state?.decision, state?.perspective]);
  const decisionSourceObjectId = useMemo(
    () => normalizeNumericId(state?.decision?.source_id),
    [state?.decision?.source_id]
  );
  const decisionSourceIsTriggered = useMemo(
    () => isTriggeredDecision(state?.decision),
    [state?.decision]
  );
  const undoTargetStableId = canShowBattlefieldUndo
    && state?.cancelable
    && state?.undo_land_stable_id != null
    ? String(state.undo_land_stable_id)
    : null;
  const cardIds = useMemo(() => displayCards.map((c) => c.id), [displayCards]);
  const { newIds, bumpedIds } = useNewCards(cardIds);
  const dragRef = useRef(null);
  const mobileCardPressRef = useRef({
    timer: null,
    cardId: null,
    suppressCardId: null,
  });
  const fitRafRef = useRef(null);
  const deferredFitRafRef = useRef(null);
  const settledFitRafRef = useRef(null);
  const pendingForceFitRef = useRef(false);
  const lastLayoutRef = useRef({
    width: -1,
    height: -1,
    cardsLength: -1,
    compact: null,
    allowVerticalScroll: null,
    forceSingleColumn: null,
    layoutSignature: "",
    bottomOcclusionViewportTop: null,
    selectedObjectId: null,
    layoutOverrideSignature: "",
  });
  const layoutOverrideSignature = normalizedLayoutOverride
    ? [
      normalizedLayoutOverride.rows,
      normalizedLayoutOverride.cols,
      normalizedLayoutOverride.cardWidth,
      normalizedLayoutOverride.cardHeight,
      normalizedLayoutOverride.overlapPx,
      normalizedLayoutOverride.centerOffset,
    ].join(":")
    : "";
  const syncOverflowMode = useCallback((layout) => {
    const row = rowRef.current;
    if (!row) return;
    if (!allowVerticalScroll || !layout) {
      row.style.overflowY = "visible";
      row.style.overflowX = "visible";
      row.style.overscrollBehaviorY = "";
      return;
    }
    row.style.overflowY = "auto";
    row.style.overflowX = "hidden";
    row.style.overscrollBehaviorY = "contain";
  }, [allowVerticalScroll]);
  const handleGhostDone = useCallback((ghostKey) => {
    setGhosts((existing) => existing.filter((entry) => entry.key !== ghostKey));
  }, []);
  const mobileObjectGesturesEnabled = (
    (isMobileBattleTopLayout || isMobileBattleBottomLayout || isMobileBattleSingleRowLayout)
    && (typeof onMobileCardActionMenu === "function" || typeof onMobileCardLongPress === "function")
  );

  const clearMobileCardPress = useCallback((options = {}) => {
    const { preserveSuppressCardId = false } = options;
    const current = mobileCardPressRef.current;
    if (current.timer) {
      clearTimeout(current.timer);
    }
    mobileCardPressRef.current = {
      timer: null,
      cardId: null,
      suppressCardId: preserveSuppressCardId ? current.suppressCardId : null,
    };
  }, []);

  useEffect(() => () => {
    clearMobileCardPress();
  }, [clearMobileCardPress]);

  const fitCards = useCallback(() => {
    const row = rowRef.current;
    if (!row) return;

    if (normalizedLayoutOverride) {
      row.style.setProperty("--bf-cols", String(normalizedLayoutOverride.cols));
      row.style.setProperty("--bf-rows", String(normalizedLayoutOverride.rows));
      row.style.setProperty("--bf-card-width", `${normalizedLayoutOverride.cardWidth}px`);
      row.style.setProperty("--bf-card-height", `${normalizedLayoutOverride.cardHeight}px`);
      row.style.setProperty("--bf-card-overlap", `${normalizedLayoutOverride.overlapPx}px`);
      if (normalizedLayoutOverride.centerOffset > 0) {
        row.style.setProperty(
          "--mobile-battle-bottom-inline-offset",
          `${normalizedLayoutOverride.centerOffset}px`
        );
      } else {
        row.style.removeProperty("--mobile-battle-bottom-inline-offset");
      }
      syncOverflowMode({
        rows: normalizedLayoutOverride.rows,
        cardHeight: normalizedLayoutOverride.cardHeight,
        gap: BATTLEFIELD_GRID_GAP_PX,
        viewportHeight: row.clientHeight,
      });
      return;
    }

    const width = row.clientWidth;
    const height = row.clientHeight;
    if (width <= 0 || height <= 0) return;

    const aspect = 124 / 96;
    const gap = BATTLEFIELD_GRID_GAP_PX;
    const hasCards = cards.length > 0;
    const minWidth = compact ? 30 : 44;
    const minHeight = compact ? 42 : 34;
    const rowRect = row.getBoundingClientRect();
    const rowStyles = window.getComputedStyle(row);
    const rowPaddingTop = Number.parseFloat(rowStyles.paddingTop || "0") || 0;
    const hasMeasuredBottomOcclusion = (
      isPaperBattlefieldLayout
      && battlefieldSide === "bottom"
      && Number.isFinite(bottomOcclusionViewportTop)
    );
    const visibleBoundaryFromBottomOcclusion = hasMeasuredBottomOcclusion
      ? Math.max(0, Math.min(height, bottomOcclusionViewportTop - rowRect.top))
      : null;
    const effectiveHeight = Math.max(
      minHeight,
      height - (
        isPaperBattlefieldLayout && battlefieldSide === "bottom" && !hasMeasuredBottomOcclusion
          ? bottomSafeInset
          : 0
      )
    );
    let best = null;

    if (isPaperBattlefieldLayout) {
      const rows = paperLayout.rowCount;
      const cols = paperLayout.maxCols;
      const widthLimit = (width - (cols - 1) * gap) / cols;
      const heightLimit = ((effectiveHeight - (rows - 1) * gap) / rows) * aspect;
      const bottomOcclusionWidthLimit = (
        hasMobileBottomBackRowCards
        && visibleBoundaryFromBottomOcclusion != null
      )
        ? (
          (
            visibleBoundaryFromBottomOcclusion
            - rowPaddingTop
            - gap
            - MOBILE_BOTTOM_BACK_ROW_TRANSLATE_Y_PX
          ) / (1 + (MOBILE_BOTTOM_MIN_VISIBLE_BACK_ROW_RATIO * MOBILE_BOTTOM_BACK_ROW_SCALE))
        ) * aspect
        : Infinity;
      const cardWidth = Math.floor(Math.min(widthLimit, heightLimit, bottomOcclusionWidthLimit));
      const cardHeight = Math.floor(cardWidth / aspect);
      if (Number.isFinite(cardWidth) && Number.isFinite(cardHeight)) {
        best = {
          rows,
          cols,
          cardWidth: Math.max(ABSOLUTE_MIN_CARD_WIDTH, cardWidth),
          cardHeight: Math.max(ABSOLUTE_MIN_CARD_HEIGHT, cardHeight),
        };
      }
    } else if (forceSingleColumn) {
      if (!hasCards) {
        row.style.removeProperty("--bf-cols");
        row.style.removeProperty("--bf-rows");
        row.style.removeProperty("--bf-card-width");
        row.style.removeProperty("--bf-card-height");
        row.style.removeProperty("--bf-card-overlap");
        row.style.removeProperty("--mobile-battle-bottom-inline-offset");
        row.style.overflowY = "visible";
        row.style.overflowX = "visible";
        return;
      }
      const cardWidth = Math.max(
        22,
        Math.floor(Math.min(width, COMPACT_SCROLL_COLUMN_MAX_WIDTH))
      );
      const cardHeight = Math.max(minHeight, Math.floor(cardWidth / aspect));
      best = {
        rows: cards.length,
        cols: 1,
        cardWidth,
        cardHeight,
      };
    } else {
      if (!hasCards) {
        row.style.removeProperty("--bf-cols");
        row.style.removeProperty("--bf-rows");
        row.style.removeProperty("--bf-card-width");
        row.style.removeProperty("--bf-card-height");
        row.style.removeProperty("--bf-card-overlap");
        row.style.removeProperty("--mobile-battle-bottom-inline-offset");
        row.style.overflowY = "visible";
        row.style.overflowX = "visible";
        return;
      }
      const maxRows = Math.min(cards.length, compact ? 8 : 10);
      for (let rows = 1; rows <= maxRows; rows++) {
        const cols = Math.ceil(cards.length / rows);
        const widthLimit = (width - (cols - 1) * gap) / cols;
        const heightLimit = ((effectiveHeight - (rows - 1) * gap) / rows) * aspect;
        const cardWidth = Math.floor(Math.min(widthLimit, heightLimit));
        const cardHeight = Math.floor(cardWidth / aspect);
        if (!Number.isFinite(cardWidth) || !Number.isFinite(cardHeight)) continue;
        if (cardWidth < minWidth || cardHeight < minHeight) continue;
        if (!best || cardWidth > best.cardWidth) {
          best = { rows, cols, cardWidth, cardHeight };
        }
      }
    }

    if (!best) {
      if (isPaperBattlefieldLayout) {
        const cols = Math.max(1, paperLayout.maxCols);
        const rows = Math.max(1, paperLayout.rowCount);
        const widthLimit = (width - (cols - 1) * gap) / cols;
        const heightLimit = ((effectiveHeight - (rows - 1) * gap) / rows) * aspect;
        const bottomOcclusionWidthLimit = (
          hasMobileBottomBackRowCards
          && visibleBoundaryFromBottomOcclusion != null
        )
          ? (
            (
              visibleBoundaryFromBottomOcclusion
              - rowPaddingTop
              - gap
              - MOBILE_BOTTOM_BACK_ROW_TRANSLATE_Y_PX
            ) / (1 + (MOBILE_BOTTOM_MIN_VISIBLE_BACK_ROW_RATIO * MOBILE_BOTTOM_BACK_ROW_SCALE))
          ) * aspect
          : Infinity;
        const cardWidth = Math.max(
          ABSOLUTE_MIN_CARD_WIDTH,
          Math.floor(Math.min(widthLimit, heightLimit, bottomOcclusionWidthLimit))
        );
        const cardHeight = Math.max(ABSOLUTE_MIN_CARD_HEIGHT, Math.floor(cardWidth / aspect));
        best = { rows, cols, cardWidth, cardHeight };
      } else {
        const cols = Math.max(1, Math.floor((width + gap) / (minWidth + gap)));
        const rows = Math.ceil(cards.length / cols);
        const widthLimit = (width - (cols - 1) * gap) / cols;
        const cardWidth = Math.max(22, Math.floor(widthLimit));
        const cardHeight = Math.max(minHeight, Math.floor(cardWidth / aspect));
        best = { rows, cols, cardWidth, cardHeight };
      }
    }

    const mobileBattleWidthRatio = (isMobileBattleTopLayout || isMobileBattleBottomLayout)
      ? 0.086
      : MAX_BATTLEFIELD_CARD_ZONE_WIDTH_RATIO;
    const maxCardWidth = forceSingleColumn
      ? Math.max(ABSOLUTE_MIN_CARD_WIDTH, Math.floor(width - 4))
      : Math.max(ABSOLUTE_MIN_CARD_WIDTH, Math.floor(width * mobileBattleWidthRatio));
    const clampedCardWidth = Math.max(
      isPaperBattlefieldLayout ? ABSOLUTE_MIN_CARD_WIDTH : 22,
      Math.min(best.cardWidth, maxCardWidth)
    );
    best = {
      ...best,
      cardWidth: clampedCardWidth,
      cardHeight: Math.max(
        isPaperBattlefieldLayout ? ABSOLUTE_MIN_CARD_HEIGHT : minHeight,
        Math.floor(clampedCardWidth / aspect)
      ),
    };

    row.style.setProperty("--bf-cols", String(best.cols));
    row.style.setProperty("--bf-rows", String(best.rows));
    row.style.setProperty("--bf-card-width", `${best.cardWidth}px`);
    row.style.setProperty("--bf-card-height", `${best.cardHeight}px`);
    const overlapPx = isPaperBattlefieldLayout
      ? (
        isMobileBattleTopLayout
          ? Math.min(28, Math.max(18, Math.floor(best.cardWidth * 0.26)))
          : isMobileBattleBottomLayout
            ? computeMobileBottomOverlapPx(best.cardWidth)
            : 0
      )
      : Math.min(14, Math.max(8, Math.floor(best.cardWidth * 0.11)));
    row.style.setProperty("--bf-card-overlap", `${overlapPx}px`);
    if (isMobileBattleBottomLayout) {
      const visualWidth = computePaperVisualGridWidth(best.cols, best.cardWidth, gap, overlapPx);
      const centeredOffset = Math.max(0, Math.floor((width - visualWidth) / 2));
      row.style.setProperty(
        "--mobile-battle-bottom-inline-offset",
        `${centeredOffset}px`
      );
    } else {
      row.style.removeProperty("--mobile-battle-bottom-inline-offset");
    }
    syncOverflowMode({
      rows: best.rows,
      cardHeight: best.cardHeight,
      gap,
      viewportHeight: effectiveHeight,
    });
  }, [
    battlefieldSide,
    bottomSafeInset,
    bottomOcclusionViewportTop,
    cards.length,
    compact,
    forceSingleColumn,
    hasMobileBottomBackRowCards,
    isPaperBattlefieldLayout,
    isMobileBattleBottomLayout,
    isMobileBattleTopLayout,
    normalizedLayoutOverride,
    paperLayout.maxCols,
    paperLayout.rowCount,
    syncOverflowMode,
  ]);

  const scheduleFitCards = useCallback((force = false) => {
    pendingForceFitRef.current = pendingForceFitRef.current || force;
    if (fitRafRef.current != null) return;
    fitRafRef.current = window.requestAnimationFrame(() => {
      fitRafRef.current = null;
      const row = rowRef.current;
      if (!row) return;

      const width = row.clientWidth;
      const height = row.clientHeight;
      const prev = lastLayoutRef.current;
      const layoutChanged = (
        Math.abs(width - prev.width) >= 2
        || Math.abs(height - prev.height) >= 2
        || prev.cardsLength !== cards.length
        || prev.compact !== compact
        || prev.allowVerticalScroll !== allowVerticalScroll
        || prev.forceSingleColumn !== forceSingleColumn
        || prev.layoutSignature !== paperLayout.signature
        || prev.bottomOcclusionViewportTop !== bottomOcclusionViewportTop
        || prev.selectedObjectId !== selectedObjectId
        || prev.layoutOverrideSignature !== layoutOverrideSignature
      );
      const forceNow = pendingForceFitRef.current;
      pendingForceFitRef.current = false;
      if (!forceNow && !layoutChanged) return;

      lastLayoutRef.current = {
        width,
        height,
        cardsLength: cards.length,
        compact,
        allowVerticalScroll,
        forceSingleColumn,
        layoutSignature: paperLayout.signature,
        bottomOcclusionViewportTop,
        selectedObjectId,
        layoutOverrideSignature,
      };
      fitCards();
    });
  }, [
    allowVerticalScroll,
    bottomOcclusionViewportTop,
    cards.length,
    compact,
    fitCards,
    forceSingleColumn,
    layoutOverrideSignature,
    paperLayout.signature,
    selectedObjectId,
  ]);

  const scheduleSettledFit = useCallback(() => {
    scheduleFitCards(true);
    if (deferredFitRafRef.current != null) {
      window.cancelAnimationFrame(deferredFitRafRef.current);
      deferredFitRafRef.current = null;
    }
    if (settledFitRafRef.current != null) {
      window.cancelAnimationFrame(settledFitRafRef.current);
      settledFitRafRef.current = null;
    }
    deferredFitRafRef.current = window.requestAnimationFrame(() => {
      deferredFitRafRef.current = null;
      scheduleFitCards(true);
      settledFitRafRef.current = window.requestAnimationFrame(() => {
        settledFitRafRef.current = null;
        scheduleFitCards(true);
      });
    });
  }, [scheduleFitCards]);

  useLayoutEffect(() => {
    scheduleFitCards(true);
  }, [scheduleFitCards]);

  useEffect(() => {
    scheduleSettledFit();
  }, [scheduleSettledFit, state?.decision?.actions?.length, state?.decision?.kind]);

  useEffect(() => {
    if (!isMobileBattleBottomLayout) return undefined;
    scheduleSettledFit();
    return undefined;
  }, [bottomOcclusionViewportTop, isMobileBattleBottomLayout, scheduleSettledFit]);

  useEffect(() => {
    if (!isMobileBattleBottomLayout && !isMobileBattleTopLayout) return undefined;
    scheduleSettledFit();
    return undefined;
  }, [
    isMobileBattleBottomLayout,
    isMobileBattleTopLayout,
    scheduleSettledFit,
    selectedObjectId,
  ]);

  useEffect(() => {
    if (!isMobileBattleBottomLayout || typeof window === "undefined") return undefined;
    const handleHandBoundsChange = () => {
      scheduleSettledFit();
    };
    window.addEventListener("ironsmith:mobile-hand-bounds-change", handleHandBoundsChange);
    return () => {
      window.removeEventListener("ironsmith:mobile-hand-bounds-change", handleHandBoundsChange);
    };
  }, [isMobileBattleBottomLayout, scheduleSettledFit]);

  useEffect(() => {
    const row = rowRef.current;
    if (!row) return;
    const observer = new ResizeObserver(() => {
      scheduleFitCards();
    });
    observer.observe(row);
    return () => {
      observer.disconnect();
    };
  }, [scheduleFitCards]);

  useEffect(() => {
    const onResize = () => scheduleFitCards();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, [scheduleFitCards]);

  useEffect(() => () => {
    if (fitRafRef.current != null) {
      window.cancelAnimationFrame(fitRafRef.current);
      fitRafRef.current = null;
    }
    if (deferredFitRafRef.current != null) {
      window.cancelAnimationFrame(deferredFitRafRef.current);
      deferredFitRafRef.current = null;
    }
    if (settledFitRafRef.current != null) {
      window.cancelAnimationFrame(settledFitRafRef.current);
      settledFitRafRef.current = null;
    }
  }, []);

  useLayoutEffect(() => {
    const row = rowRef.current;
    const snapshotId = state?.snapshot_id ?? null;
    if (!row || snapshotId == null || lastProcessedSnapshotIdRef.current === snapshotId) {
      return;
    }
    if (!isPaperBattlefieldLayout) {
      previousPositionsRef.current = new Map();
      previousCardsRef.current = displayCards;
      lastProcessedSnapshotIdRef.current = snapshotId;
      return;
    }

    const previousCards = previousCardsRef.current || [];
    const previousCardsByStableId = indexCardsByStableId(previousCards);
    const currentCardsByStableId = indexCardsByStableId(displayCards);
    const transitionGroups = groupBattlefieldTransitions(state?.battlefield_transitions);
    const ghostsToAdd = [];
    const offsetsByCardId = new Map();

    for (const transition of transitionGroups.values()) {
      const stableId = transition.stableId;
      if (transition.leaveKind) {
        const previousCard = previousCardsByStableId.get(stableId);
        const previousPosition = previousPositionsRef.current.get(stableId);
        if (previousCard && previousPosition) {
          const offsetIndex = offsetsByCardId.get(previousCard.id) || 0;
          offsetsByCardId.set(previousCard.id, offsetIndex + 1);
          ghostsToAdd.push({
            key: `ghost-${snapshotId}-${stableId}-${transition.leaveKind}`,
            card: cloneLeavingCard(previousCard, stableId),
            kind: transition.leaveKind,
            includeDamage: transition.damaged,
            duration: GHOST_BASE_ANIMATION_MS,
            totalDuration: GHOST_BASE_ANIMATION_MS + (transition.damaged ? 180 : 0),
            left: previousPosition.left + (offsetIndex * 5),
            top: previousPosition.top - (offsetIndex * 3),
            width: previousPosition.width,
            height: previousPosition.height,
          });
        }
        continue;
      }

      if (!transition.damaged) continue;
      if (!currentCardsByStableId.has(stableId)) continue;
      const node = findCardElementForStableId(row, stableId);
      if (node) {
        playLiveDamageAnimation(node, liveDamageMotionsRef.current, stableId);
      }
    }

    if (ghostsToAdd.length > 0) {
      setGhosts((existing) => [...existing, ...ghostsToAdd]);
    }

    previousPositionsRef.current = measureLiveCardPositions(row);
    previousCardsRef.current = displayCards;
    lastProcessedSnapshotIdRef.current = snapshotId;
  }, [
    displayCards,
    isPaperBattlefieldLayout,
    state?.battlefield_transitions,
    state?.snapshot_id,
  ]);

  useEffect(() => () => {
    for (const motion of liveDamageMotionsRef.current.values()) {
      cancelMotion(motion);
    }
    liveDamageMotionsRef.current.clear();
  }, []);

  // Combat drag handlers
  const handleCombatPointerDown = useCallback((e, card) => {
    const cm = combatModeRef.current;
    if (!cm || !cm.candidates.has(Number(card.id))) return;
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();

    const sx = e.clientX;
    const sy = e.clientY;
    dragRef.current = { sx, sy, cardId: Number(card.id), dragging: false };

    const onMove = (me) => {
      const dt = dragRef.current;
      if (!dt) return;
      const dx = me.clientX - dt.sx;
      const dy = me.clientY - dt.sy;
      if (!dt.dragging && (dx * dx + dy * dy) > 36) {
        dt.dragging = true;
        startDragArrow(dt.cardId, me.clientX, me.clientY, cm.color);
      }
      if (dt.dragging) {
        updateDragArrow(me.clientX, me.clientY);
        if (cm.mode === "attackers" || cm.mode === "blockers") {
          const hoverEl = document
            .elementFromPoint(me.clientX, me.clientY)
            ?.closest?.(".game-card[data-object-id]");
          if (hoverEl) {
            const hoverId = Number(hoverEl.dataset.objectId);
            if (Number.isFinite(hoverId)) hoverCard(hoverId);
            else clearHover();
          } else {
            clearHover();
          }
        }
      }
    };

    const onUp = (ue) => {
      document.removeEventListener("pointermove", onMove);
      document.removeEventListener("pointerup", onUp);
      document.removeEventListener("pointercancel", onUp);
      const dt = dragRef.current;
      dragRef.current = null;
      endDragArrow();

      const curMode = combatModeRef.current;
      if (!dt) return;

      if (dt.dragging && curMode?.onDrop) {
        curMode.onDrop(dt.cardId, ue.clientX, ue.clientY);
        clearHover();
      } else if (!dt.dragging) {
        // Click (no drag) — toggle via onClick or fall through to onCardClick
        if (curMode?.onClick) {
          curMode.onClick(dt.cardId);
        }
      }
    };

    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
    document.addEventListener("pointercancel", onUp);
  }, [combatModeRef, startDragArrow, updateDragArrow, endDragArrow, hoverCard, clearHover]);

  const handleCardSelectionClick = useCallback((event, card) => {
    if (mobileCardPressRef.current.suppressCardId === String(card.id)) {
      event.preventDefault();
      event.stopPropagation();
      clearMobileCardPress();
      return;
    }

    const cm = combatModeRef.current;
    if (cm?.onTargetCardClick) {
      const hasActiveSelection = cm.mode === "attackers"
        ? cm.selectedAttacker != null
        : cm.selectedBlocker != null;
      if (hasActiveSelection && cm.onTargetCardClick(Number(card.id))) {
        event.preventDefault();
        event.stopPropagation();
        return;
      }
    }

    const cardObjectIds = [Number(card?.id)];
    if (Array.isArray(card?.member_ids)) {
      for (const memberId of card.member_ids) {
        cardObjectIds.push(Number(memberId));
      }
    }
    const isLegalTargetCard = cardObjectIds.some((id) => legalTargetObjectIds.has(id));
    const cardActions = collectActivatableActionsForCard(card, activatableMap);

    if (mobileObjectGesturesEnabled && isLegalTargetCard && onCardClick) {
      onCardClick(event, card);
      return;
    }

    if (mobileObjectGesturesEnabled && typeof onMobileCardActionMenu === "function") {
      const didOpenMenu = onMobileCardActionMenu({
        card,
        actions: cardActions,
        anchorRect: event.currentTarget?.getBoundingClientRect?.() || null,
      });
      if (didOpenMenu) {
        event.preventDefault();
        event.stopPropagation();
        return;
      }
    }

    if (mobileObjectGesturesEnabled) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }

    if (onCardClick) {
      onCardClick(event, card);
      return;
    }

    onInspect?.(card.id);
  }, [
    activatableMap,
    clearMobileCardPress,
    combatModeRef,
    mobileObjectGesturesEnabled,
    onCardClick,
    onInspect,
    onMobileCardActionMenu,
    legalTargetObjectIds,
  ]);

  const handleRowClickFallback = useCallback((event) => {
    if (!isMobileBattleSingleRowLayout) return;
    if (event.defaultPrevented) return;
    if (!(event.target instanceof Element)) return;
    if (event.target.closest(".battlefield-row-card[data-object-id]")) return;
    if (event.target.closest("button, a, input, textarea, select, [role='button']")) return;

    const hitElement = document.elementFromPoint(event.clientX, event.clientY);
    let hitCardEl = hitElement?.closest?.(".battlefield-row-card[data-object-id]") || null;

    if (!hitCardEl && rowRef.current) {
      const withinExpandedRect = (rect, x, y) => (
        x >= (rect.left - MOBILE_BATTLEFIELD_TOKEN_HIT_SLOP_X)
        && x <= (rect.right + MOBILE_BATTLEFIELD_TOKEN_HIT_SLOP_X)
        && y >= (rect.top - MOBILE_BATTLEFIELD_TOKEN_HIT_SLOP_Y)
        && y <= (rect.bottom + MOBILE_BATTLEFIELD_TOKEN_HIT_SLOP_Y)
      );

      let bestMatch = null;
      let bestDistanceSq = Infinity;
      const cardNodes = rowRef.current.querySelectorAll(".battlefield-row-card[data-object-id]");

      for (const node of cardNodes) {
        const rect = node.getBoundingClientRect();
        if (!withinExpandedRect(rect, event.clientX, event.clientY)) continue;

        const centerX = rect.left + (rect.width / 2);
        const centerY = rect.top + (rect.height / 2);
        const distanceSq = ((event.clientX - centerX) ** 2) + ((event.clientY - centerY) ** 2);
        if (distanceSq < bestDistanceSq) {
          bestMatch = node;
          bestDistanceSq = distanceSq;
        }
      }

      hitCardEl = bestMatch;
    }

    const fallbackCardId = hitCardEl?.dataset?.objectId;
    if (!fallbackCardId) return;

    const fallbackCard = displayCardById.get(String(fallbackCardId));
    if (!fallbackCard) return;

    event.preventDefault();
    event.stopPropagation();
    handleCardSelectionClick(event, fallbackCard);
  }, [displayCardById, handleCardSelectionClick, isMobileBattleSingleRowLayout]);

  const handleCardPointerPressStart = useCallback((event, card, isCombatCandidate = false) => {
    if (isCombatCandidate) {
      handleCombatPointerDown(event, card);
      return;
    }

    onCardPointerDown?.(event, card);

    if (!mobileObjectGesturesEnabled || typeof onMobileCardLongPress !== "function") return;
    if (event.defaultPrevented || event.button > 0 || event.isPrimary === false) return;

    clearMobileCardPress();
    const target = event.currentTarget;
    mobileCardPressRef.current = {
      timer: window.setTimeout(() => {
        mobileCardPressRef.current = {
          timer: null,
          cardId: String(card.id),
          suppressCardId: String(card.id),
        };
        onMobileCardLongPress({
          card,
          anchorRect: target?.getBoundingClientRect?.() || null,
        });
      }, MOBILE_OBJECT_LONG_PRESS_MS),
      cardId: String(card.id),
      suppressCardId: null,
    };
  }, [
    clearMobileCardPress,
    handleCombatPointerDown,
    mobileObjectGesturesEnabled,
    onCardPointerDown,
    onMobileCardLongPress,
  ]);

  const handleCardPointerPressEnd = useCallback(() => {
    clearMobileCardPress({ preserveSuppressCardId: true });
  }, [clearMobileCardPress]);

  return (
    <div
      ref={rowRef}
      className={`battlefield-row ${displayCards.length === 0 ? "battlefield-row-empty" : ""} ${alignStart ? "battlefield-row--align-start" : ""} ${isMobileBattleBottomLayout ? "battlefield-row--mobile-bottom-inline-fit" : ""} relative grid gap-1.5 content-start justify-center min-h-0 h-full`}
      data-bf-side={battlefieldSide}
      onClick={handleRowClickFallback}
      style={{
        "--bf-gap": `${BATTLEFIELD_GRID_GAP_PX}px`,
        gap: `${BATTLEFIELD_GRID_GAP_PX}px`,
        gridTemplateColumns: `repeat(var(--bf-cols, 1), minmax(0, calc(var(--bf-card-width, 124px) - var(--bf-card-overlap, 0px))))`,
        gridTemplateRows: isPaperBattlefieldLayout
          ? `repeat(var(--bf-rows, 1), var(--bf-card-height, 96px))`
          : undefined,
        gridAutoRows: isPaperBattlefieldLayout ? undefined : "var(--bf-card-height, 96px)",
        scrollbarGutter: allowVerticalScroll ? "stable" : "auto",
      }}
    >
      {isPaperBattlefieldLayout ? paperLayout.slotCells.map((slot) => (
        <div
          key={slot.key}
          className="battlefield-slot"
          data-bf-group={slot.groupId}
          data-bf-row={slot.row}
          style={{
            gridRow: String(slot.row),
            gridColumn: String(slot.column),
            width: "var(--bf-card-width, 124px)",
            minWidth: "var(--bf-card-width, 124px)",
            height: "var(--bf-card-height, 96px)",
            minHeight: "var(--bf-card-height, 96px)",
          }}
          aria-hidden="true"
        />
      )) : null}
      {displayCards.map((card, i) => {
        const cardActions = collectActivatableActionsForCard(card, activatableMap);
        const isActivatable = cardActions.length > 0;
        const cardObjectIds = [Number(card.id)];
        if (Array.isArray(card.member_ids)) {
          for (const memberId of card.member_ids) {
            cardObjectIds.push(Number(memberId));
          }
        }
        const isLegalTarget = cardObjectIds.some((id) => legalTargetObjectIds.has(id));
        const hasLinkedPriorityAction = cardObjectIds.some((id) => priorityActionObjectIds.has(String(id)));
        const isTriggeredSource = (
          decisionSourceIsTriggered
          && decisionSourceObjectId != null
          && cardObjectIds.some((id) => id === decisionSourceObjectId)
        );
        const isNew = newIds.has(card.id);
        const isBumped = bumpedIds.has(card.id);
        let bumpDir = 0;
        if (isBumped) {
          if (i > 0 && newIds.has(displayCards[i - 1].id)) bumpDir = 1;
          else if (i < displayCards.length - 1 && newIds.has(displayCards[i + 1].id)) bumpDir = -1;
        }

        const isCombatCandidate = combatMode?.candidates?.has(Number(card.id));
        const activeSourceId = combatMode?.mode === "attackers"
          ? Number(combatMode?.selectedAttacker ?? dragArrow?.fromId ?? NaN)
          : combatMode?.mode === "blockers"
            ? Number(combatMode?.selectedBlocker ?? dragArrow?.fromId ?? NaN)
            : NaN;
        const activeTargetObjects = (
          Number.isFinite(activeSourceId)
            ? (
              combatMode?.mode === "attackers"
                ? combatMode?.validTargetObjectsByAttacker?.[activeSourceId]
                : combatMode?.validTargetObjectsByBlocker?.[activeSourceId]
            )
            : combatMode?.validTargetObjects
        );
        const isCombatHoverTarget = (
          (combatMode?.mode === "attackers" || combatMode?.mode === "blockers") &&
          Number.isFinite(activeSourceId) &&
          !!activeTargetObjects?.has?.(Number(card.id)) &&
          hoveredObjectId != null &&
          String(card.id) === String(hoveredObjectId)
        );
        const isCombatTargetCard = (
          (combatMode?.mode === "attackers" || combatMode?.mode === "blockers")
          && Number.isFinite(activeSourceId)
          && !!activeTargetObjects?.has?.(Number(card.id))
        );
        const isActionLinkedHover = (
          cardObjectIds.some((id) => hoveredLinkedObjectIds.has(String(id)))
          || (
          hoveredObjectId != null
          && hasLinkedPriorityAction
          && cardObjectIds.some((id) => String(id) === String(hoveredObjectId))
          )
        );
        // Determine ability glow kind: mana vs non-mana
        let abilityGlow = null;
        if (isActivatable) {
          const hasMana = cardActions.some((a) => a.kind === "activate_mana_ability");
          const hasNonMana = cardActions.some((a) => a.kind === "activate_ability");
          abilityGlow = hasMana && !hasNonMana ? "mana" : hasNonMana ? "ability" : "mana";
        }
        const isInteractable = isActivatable || isCombatCandidate || isCombatTargetCard;
        const isSelectedCombatSource = (
          combatMode?.selectedAttacker === Number(card.id)
          || combatMode?.selectedBlocker === Number(card.id)
        );
        const combatGlowKind = isSelectedCombatSource
          ? "attack-selected"
          : isCombatHoverTarget
            ? "attack-selected"
            : isCombatCandidate
              ? (combatMode.mode === "attackers" ? "attack-candidate" : "blocker-candidate")
              : null;
        const appliedGlowKind = isActionLinkedHover
          ? "action-link"
          : isLegalTarget
            ? "spell"
            : isCombatHoverTarget
              ? "attack-selected"
              : (isCombatCandidate ? combatGlowKind : (isTriggeredSource ? "ability" : abilityGlow));
        const paperGridPosition = isPaperBattlefieldLayout
          ? paperLayout.gridPositionById.get(String(card.id))
          : null;
        const showsUndoOverlay = canShowBattlefieldUndo
          && undoTargetStableId != null
          && card?.tapped
          && stableIdsForCard(card).includes(String(undoTargetStableId));

        return (
          <GameCard
            key={card.id}
            card={card}
            compact={compact}
            className={[
              "battlefield-row-card",
              isPaperBattlefieldLayout && paperGridPosition?.row
                ? `battlefield-row-card--paper-row-${paperGridPosition.row}`
                : "",
              isPaperBattlefieldLayout && paperGridPosition?.groupId
                ? `battlefield-row-card--paper-group-${paperGridPosition.groupId}`
                : "",
            ].filter(Boolean).join(" ")}
            isInspected={selectedObjectId != null && cardObjectIds.some((id) => String(id) === String(selectedObjectId))}
            isPlayable={isInteractable}
            glowKind={appliedGlowKind}
            isHovered={isCombatHoverTarget || isActionLinkedHover}
            isNew={isNew}
            isBumped={isBumped}
            bumpDirection={bumpDir}
            battlefieldVisualMode={useMobileBattlefieldToken ? "mobile-token" : "classic"}
            suppressTooltip={suppressTooltip}
            onClick={(event) => handleCardSelectionClick(event, card)}
            onPointerDown={(event) => handleCardPointerPressStart(event, card, isCombatCandidate)}
            onPointerUp={handleCardPointerPressEnd}
            onPointerCancel={handleCardPointerPressEnd}
            onPointerLeave={handleCardPointerPressEnd}
            onMouseEnter={() => hoverCard(card.id)}
            onMouseLeave={clearHover}
            centerOverlay={showsUndoOverlay ? (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="decision-neon-button decision-neon-button--danger decision-cancel-button h-8 w-8 rounded-none p-0"
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  cancelDecision();
                }}
                title="Undo"
                aria-label={`Undo tap of ${card.name || "land"}`}
              >
                <Undo2 className="h-4 w-4" />
              </Button>
            ) : null}
            style={{
              ...(paperGridPosition
                ? {
                  gridRow: String(paperGridPosition.row),
                  gridColumn: String(paperGridPosition.column),
                }
                : undefined),
              width: "var(--bf-card-width, 124px)",
              minWidth: "var(--bf-card-width, 124px)",
              height: "var(--bf-card-height, 96px)",
              minHeight: "var(--bf-card-height, 96px)",
              cursor: isCombatCandidate || isCombatTargetCard ? "pointer" : undefined,
            }}
          />
        );
      })}
      {ghosts.map((ghost) => (
        <BattlefieldGhostCard
          key={ghost.key}
          ghost={ghost}
          compact={compact}
          onDone={handleGhostDone}
        />
      ))}
    </div>
  );
}
