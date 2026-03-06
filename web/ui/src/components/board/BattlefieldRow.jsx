import { useRef, useLayoutEffect, useEffect, useCallback, useMemo } from "react";
import { useHover } from "@/context/HoverContext";
import { useCombatArrows } from "@/context/CombatArrowContext";
import { useGame } from "@/context/GameContext";
import useNewCards from "@/hooks/useNewCards";
import GameCard from "@/components/cards/GameCard";

const PAPER_ROW_GROUPS = [
  {
    id: "front",
    lanes: ["creatures", "enchantments", "planeswalkers", "battles", "other"],
  },
  {
    id: "back",
    lanes: ["lands", "artifacts"],
  },
];
const ALL_PAPER_LANES = PAPER_ROW_GROUPS.flatMap((row) => row.lanes);
const BOTTOM_BATTLEFIELD_SAFE_INSET = 60;

function normalizeBattlefieldLane(lane) {
  const normalized = String(lane || "").toLowerCase();
  return ALL_PAPER_LANES.includes(normalized) ? normalized : "other";
}

function buildPaperBattlefieldLayout(cards, battlefieldSide) {
  const rowGroups = battlefieldSide === "top"
    ? [...PAPER_ROW_GROUPS].reverse()
    : PAPER_ROW_GROUPS;
  const buckets = new Map(ALL_PAPER_LANES.map((lane) => [lane, []]));

  for (const card of cards) {
    const lane = normalizeBattlefieldLane(card?.lane);
    buckets.get(lane).push(card);
  }

  const occupiedRows = rowGroups
    .map((row) => ({
      id: row.id,
      cards: row.lanes.flatMap((lane) => buckets.get(lane) || []),
      signature: row.lanes.map((lane) => `${lane}:${(buckets.get(lane) || []).length}`).join(","),
    }))
    .filter((row) => row.cards.length > 0);
  const orderedCards = [];
  const gridPositionById = new Map();
  const maxCols = Math.max(1, ...occupiedRows.map((row) => row.cards.length));

  occupiedRows.forEach((row, rowIndex) => {
    const startColumn = row.id === "back"
      ? 1
      : Math.floor((maxCols - row.cards.length) / 2) + 1;

    row.cards.forEach((card, columnIndex) => {
      orderedCards.push(card);
      gridPositionById.set(String(card.id), {
        row: rowIndex + 1,
        column: startColumn + columnIndex,
      });
    });
  });

  return {
    orderedCards,
    gridPositionById,
    rowCount: Math.max(occupiedRows.length, 1),
    maxCols,
    signature: occupiedRows
      .map((row) => `${row.id}:${row.signature}`)
      .join("|"),
  };
}

export default function BattlefieldRow({
  cards = [],
  compact = false,
  battlefieldSide = "bottom",
  selectedObjectId,
  onInspect,
  onCardClick,
  onExpandInspector,
  activatableMap,
  legalTargetObjectIds = new Set(),
  allowVerticalScroll = false,
}) {
  const rowRef = useRef(null);
  const { state } = useGame();
  const { hoverCard, clearHover, hoveredObjectId, hoveredLinkedObjectIds } = useHover();
  const { combatMode, combatModeRef, dragArrow, startDragArrow, updateDragArrow, endDragArrow } = useCombatArrows();
  const isPaperBattlefieldLayout = !compact;
  const paperLayout = useMemo(
    () => buildPaperBattlefieldLayout(cards, battlefieldSide),
    [battlefieldSide, cards]
  );
  const displayCards = isPaperBattlefieldLayout ? paperLayout.orderedCards : cards;
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
  const cardIds = useMemo(() => displayCards.map((c) => c.id), [displayCards]);
  const { newIds, bumpedIds } = useNewCards(cardIds);
  const dragRef = useRef(null);
  const fitRafRef = useRef(null);
  const pendingForceFitRef = useRef(false);
  const lastLayoutRef = useRef({
    width: -1,
    height: -1,
    cardsLength: -1,
    compact: null,
    allowVerticalScroll: null,
    layoutSignature: "",
  });
  const syncOverflowMode = useCallback((layout) => {
    const row = rowRef.current;
    if (!row) return;
    if (!allowVerticalScroll || !layout) {
      row.style.overflowY = "visible";
      row.style.overflowX = "visible";
      return;
    }
    const contentHeight =
      (layout.rows * layout.cardHeight) + (Math.max(layout.rows - 1, 0) * layout.gap);
    row.style.overflowY = contentHeight > (layout.viewportHeight + 1) ? "auto" : "visible";
    row.style.overflowX = "visible";
  }, [allowVerticalScroll]);

  const fitCards = useCallback(() => {
    const row = rowRef.current;
    if (!row || !cards.length) {
      if (row) {
        row.style.removeProperty("--bf-cols");
        row.style.removeProperty("--bf-rows");
        row.style.removeProperty("--bf-card-width");
        row.style.removeProperty("--bf-card-height");
        row.style.removeProperty("--bf-card-overlap");
        row.style.overflowY = "visible";
        row.style.overflowX = "visible";
      }
      return;
    }

    const width = row.clientWidth;
    const height = row.clientHeight;
    if (width <= 0 || height <= 0) return;

    const aspect = 124 / 96;
    const gap = 6;
    const minWidth = compact ? 30 : 44;
    const minHeight = compact ? 42 : 34;
    const effectiveHeight = Math.max(
      minHeight,
      height - (
        isPaperBattlefieldLayout && battlefieldSide === "bottom"
          ? BOTTOM_BATTLEFIELD_SAFE_INSET
          : 0
      )
    );
    let best = null;

    if (isPaperBattlefieldLayout) {
      const rows = paperLayout.rowCount;
      const cols = paperLayout.maxCols;
      const widthLimit = (width - (cols - 1) * gap) / cols;
      const heightLimit = ((effectiveHeight - (rows - 1) * gap) / rows) * aspect;
      const cardWidth = Math.floor(Math.min(widthLimit, heightLimit));
      const cardHeight = Math.floor(cardWidth / aspect);
      if (Number.isFinite(cardWidth) && Number.isFinite(cardHeight)) {
        best = {
          rows,
          cols,
          cardWidth: Math.max(22, cardWidth),
          cardHeight: Math.max(minHeight, cardHeight),
        };
      }
    } else {
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
        const cardWidth = Math.max(22, Math.floor(Math.min(widthLimit, heightLimit)));
        const cardHeight = Math.max(minHeight, Math.floor(cardWidth / aspect));
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

    row.style.setProperty("--bf-cols", String(best.cols));
    row.style.setProperty("--bf-rows", String(best.rows));
    row.style.setProperty("--bf-card-width", `${best.cardWidth}px`);
    row.style.setProperty("--bf-card-height", `${best.cardHeight}px`);
    const overlapPx = (compact || isPaperBattlefieldLayout)
      ? 0
      : Math.min(14, Math.max(8, Math.floor(best.cardWidth * 0.11)));
    row.style.setProperty("--bf-card-overlap", `${overlapPx}px`);
    syncOverflowMode({
      rows: best.rows,
      cardHeight: best.cardHeight,
      gap,
      viewportHeight: effectiveHeight,
    });
  }, [
    battlefieldSide,
    cards.length,
    compact,
    isPaperBattlefieldLayout,
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
        || prev.layoutSignature !== paperLayout.signature
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
        layoutSignature: paperLayout.signature,
      };
      fitCards();
    });
  }, [allowVerticalScroll, cards.length, compact, fitCards, paperLayout.signature]);

  useLayoutEffect(() => {
    scheduleFitCards(true);
  }, [scheduleFitCards]);

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
        if (cm.mode === "attackers") {
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
  }, [combatModeRef, startDragArrow, updateDragArrow, endDragArrow, hoverCard, clearHover]);

  return (
    <div
      ref={rowRef}
      className="battlefield-row grid gap-1.5 content-start justify-center min-h-0 h-full"
      style={{
        gridTemplateColumns: `repeat(var(--bf-cols, 1), minmax(0, calc(var(--bf-card-width, 124px) - var(--bf-card-overlap, 0px))))`,
        gridTemplateRows: isPaperBattlefieldLayout
          ? `repeat(var(--bf-rows, 1), var(--bf-card-height, 96px))`
          : undefined,
        gridAutoRows: isPaperBattlefieldLayout ? undefined : "var(--bf-card-height, 96px)",
        scrollbarGutter: allowVerticalScroll ? "stable" : "auto",
      }}
    >
      {displayCards.map((card, i) => {
        const isActivatable = activatableMap?.has(Number(card.id));
        const cardObjectIds = [Number(card.id)];
        if (Array.isArray(card.member_ids)) {
          for (const memberId of card.member_ids) {
            cardObjectIds.push(Number(memberId));
          }
        }
        const isLegalTarget = cardObjectIds.some((id) => legalTargetObjectIds.has(id));
        const hasLinkedPriorityAction = cardObjectIds.some((id) => priorityActionObjectIds.has(String(id)));
        const isNew = newIds.has(card.id);
        const isBumped = bumpedIds.has(card.id);
        let bumpDir = 0;
        if (isBumped) {
          if (i > 0 && newIds.has(displayCards[i - 1].id)) bumpDir = 1;
          else if (i < displayCards.length - 1 && newIds.has(displayCards[i + 1].id)) bumpDir = -1;
        }

        const isCombatCandidate = combatMode?.candidates?.has(Number(card.id));
        const activeAttackerId = (
          combatMode?.mode === "attackers"
            ? Number(combatMode?.selectedAttacker ?? dragArrow?.fromId ?? NaN)
            : NaN
        );
        const activeTargetObjects = (
          Number.isFinite(activeAttackerId)
            ? combatMode?.validTargetObjectsByAttacker?.[activeAttackerId]
            : combatMode?.validTargetObjects
        );
        const isAttackHoverTarget = (
          combatMode?.mode === "attackers" &&
          Number.isFinite(activeAttackerId) &&
          !!activeTargetObjects?.has?.(Number(card.id)) &&
          hoveredObjectId != null &&
          String(card.id) === String(hoveredObjectId)
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
          const actions = activatableMap.get(Number(card.id)) || [];
          const hasMana = actions.some((a) => a.kind === "activate_mana_ability");
          const hasNonMana = actions.some((a) => a.kind === "activate_ability");
          abilityGlow = hasMana && !hasNonMana ? "mana" : hasNonMana ? "ability" : "mana";
        }
        const isInteractable = isActivatable || isCombatCandidate;
        const isSelectedAttacker = combatMode?.selectedAttacker === Number(card.id);
        const combatGlowKind = isSelectedAttacker
          ? "attack-selected"
          : isAttackHoverTarget
            ? "attack-selected"
            : isCombatCandidate
              ? (combatMode.mode === "attackers" ? "attack-candidate" : "blocker-candidate")
              : null;
        const appliedGlowKind = isActionLinkedHover
          ? "action-link"
          : isLegalTarget
            ? "spell"
            : isAttackHoverTarget
              ? "attack-selected"
              : (isCombatCandidate ? combatGlowKind : abilityGlow);
        const paperGridPosition = isPaperBattlefieldLayout
          ? paperLayout.gridPositionById.get(String(card.id))
          : null;

        return (
          <GameCard
            key={card.id}
            card={card}
            compact={compact}
            isInspected={isInteractable && selectedObjectId === card.id}
            isPlayable={isInteractable}
            glowKind={appliedGlowKind}
            isHovered={isAttackHoverTarget || isActionLinkedHover}
            isNew={isNew}
            isBumped={isBumped}
            bumpDirection={bumpDir}
            onContextMenu={!compact && onExpandInspector ? (e) => {
              e.preventDefault();
              e.stopPropagation();
              onExpandInspector(card.id);
            } : undefined}
            onClick={onCardClick ? (e) => onCardClick(e, card) : () => onInspect?.(card.id)}
            onPointerDown={isCombatCandidate ? (e) => handleCombatPointerDown(e, card) : undefined}
            onMouseEnter={() => hoverCard(card.id)}
            onMouseLeave={clearHover}
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
              cursor: isCombatCandidate ? "pointer" : undefined,
            }}
          />
        );
      })}
    </div>
  );
}
