import { useRef, useLayoutEffect, useEffect, useCallback, useMemo } from "react";
import { useHover } from "@/context/HoverContext";
import { useCombatArrows } from "@/context/CombatArrowContext";
import useNewCards from "@/hooks/useNewCards";
import GameCard from "@/components/cards/GameCard";

export default function BattlefieldRow({ cards = [], compact = false, selectedObjectId, onInspect, onCardClick, activatableMap }) {
  const rowRef = useRef(null);
  const { hoveredObjectId, hoverCard, clearHover } = useHover();
  const { combatMode, combatModeRef, startDragArrow, updateDragArrow, endDragArrow } = useCombatArrows();
  const cardIds = useMemo(() => cards.map((c) => c.id), [cards]);
  const { newIds, bumpedIds } = useNewCards(cardIds);
  const dragRef = useRef(null);

  const fitCards = useCallback(() => {
    const row = rowRef.current;
    if (!row || !cards.length) {
      if (row) {
        row.style.removeProperty("--bf-cols");
        row.style.removeProperty("--bf-card-width");
        row.style.removeProperty("--bf-card-height");
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
    const maxRows = Math.min(cards.length, compact ? 8 : 10);

    let best = null;
    for (let rows = 1; rows <= maxRows; rows++) {
      const cols = Math.ceil(cards.length / rows);
      const widthLimit = (width - (cols - 1) * gap) / cols;
      const heightLimit = ((height - (rows - 1) * gap) / rows) * aspect;
      const cardWidth = Math.floor(Math.min(widthLimit, heightLimit));
      const cardHeight = Math.floor(cardWidth / aspect);
      if (!Number.isFinite(cardWidth) || !Number.isFinite(cardHeight)) continue;
      if (cardWidth < minWidth || cardHeight < minHeight) continue;
      if (!best || cardWidth > best.cardWidth) {
        best = { rows, cols, cardWidth, cardHeight };
      }
    }

    if (!best) {
      const rows = maxRows;
      const cols = Math.ceil(cards.length / rows);
      const widthLimit = (width - (cols - 1) * gap) / cols;
      const heightLimit = ((height - (rows - 1) * gap) / rows) * aspect;
      const cardWidth = Math.max(22, Math.floor(Math.min(widthLimit, heightLimit)));
      const cardHeight = Math.max(30, Math.floor(cardWidth / aspect));
      best = { rows, cols, cardWidth, cardHeight };
    }

    row.style.setProperty("--bf-cols", String(best.cols));
    row.style.setProperty("--bf-card-width", `${best.cardWidth}px`);
    row.style.setProperty("--bf-card-height", `${best.cardHeight}px`);
  }, [cards.length, compact]);

  useLayoutEffect(fitCards, [fitCards]);

  useEffect(() => {
    window.addEventListener("resize", fitCards);
    return () => window.removeEventListener("resize", fitCards);
  }, [fitCards]);

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
      } else if (!dt.dragging) {
        // Click (no drag) — toggle via onClick or fall through to onCardClick
        if (curMode?.onClick) {
          curMode.onClick(dt.cardId);
        }
      }
    };

    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
  }, [combatModeRef, startDragArrow, updateDragArrow, endDragArrow]);

  return (
    <div
      ref={rowRef}
      className="grid gap-1.5 content-start justify-start min-h-0 h-full overflow-visible"
      style={{
        gridTemplateColumns: `repeat(var(--bf-cols, 1), minmax(0, var(--bf-card-width, 124px)))`,
        gridAutoRows: "var(--bf-card-height, 96px)",
      }}
    >
      {cards.map((card, i) => {
        const isActivatable = activatableMap?.has(Number(card.id));
        const cardId = String(card.id);
        const isNew = newIds.has(card.id);
        const isBumped = bumpedIds.has(card.id);
        let bumpDir = 0;
        if (isBumped) {
          if (i > 0 && newIds.has(cards[i - 1].id)) bumpDir = 1;
          else if (i < cards.length - 1 && newIds.has(cards[i + 1].id)) bumpDir = -1;
        }

        const isCombatCandidate = combatMode?.candidates?.has(Number(card.id));
        const combatGlowKind = isCombatCandidate
          ? (combatMode.mode === "attackers" ? "attack-candidate" : "blocker-candidate")
          : null;

        // Determine ability glow kind: mana vs non-mana
        let abilityGlow = null;
        if (isActivatable) {
          const actions = activatableMap.get(Number(card.id)) || [];
          const hasMana = actions.some((a) => a.kind === "activate_mana_ability");
          const hasNonMana = actions.some((a) => a.kind === "activate_ability");
          abilityGlow = hasMana && !hasNonMana ? "mana" : hasNonMana ? "ability" : "mana";
        }

        return (
          <GameCard
            key={card.id}
            card={card}
            compact={compact}
            isInspected={selectedObjectId === card.id}
            isPlayable={isActivatable || isCombatCandidate}
            glowKind={isCombatCandidate ? combatGlowKind : abilityGlow}
            isHovered={hoveredObjectId === cardId}
            isNew={isNew}
            isBumped={isBumped}
            bumpDirection={bumpDir}
            onClick={onCardClick ? (e) => onCardClick(e, card) : () => onInspect?.(card.id)}
            onPointerDown={isCombatCandidate ? (e) => handleCombatPointerDown(e, card) : undefined}
            onMouseEnter={() => hoverCard(card.id)}
            onMouseLeave={clearHover}
            style={{
              width: "var(--bf-card-width, 124px)",
              minWidth: "var(--bf-card-width, 124px)",
              height: "var(--bf-card-height, 96px)",
              minHeight: "var(--bf-card-height, 96px)",
              cursor: isCombatCandidate ? "grab" : undefined,
            }}
          />
        );
      })}
    </div>
  );
}
