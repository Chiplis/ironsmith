import { useRef, useLayoutEffect, useEffect, useCallback } from "react";
import GameCard from "@/components/cards/GameCard";

export default function BattlefieldRow({ cards = [], compact = false, selectedObjectId, onInspect }) {
  const rowRef = useRef(null);

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

  return (
    <div
      ref={rowRef}
      className="grid gap-1.5 content-start justify-start min-h-0 h-full overflow-visible"
      style={{
        gridTemplateColumns: `repeat(var(--bf-cols, 1), minmax(0, var(--bf-card-width, 124px)))`,
        gridAutoRows: "var(--bf-card-height, 96px)",
      }}
    >
      {cards.map((card) => (
        <GameCard
          key={card.id}
          card={card}
          compact={compact}
          isInspected={selectedObjectId === card.id}
          onClick={() => onInspect?.(card.id)}
          style={{
            width: "var(--bf-card-width, 124px)",
            minWidth: "var(--bf-card-width, 124px)",
            height: "var(--bf-card-height, 96px)",
            minHeight: "var(--bf-card-height, 96px)",
          }}
        />
      ))}
    </div>
  );
}
