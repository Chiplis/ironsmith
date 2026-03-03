import { useRef, useEffect } from "react";

/**
 * Tracks which card IDs just appeared vs the previous render.
 * Returns { newIds: Set, bumpedIds: Set } where bumpedIds are
 * immediate neighbors of new cards (for a nudge animation).
 *
 * Skips the first render so all initial cards are treated as existing.
 */
export default function useNewCards(cardIds) {
  const prevRef = useRef(null); // null = first render
  const newIds = new Set();
  const bumpedIds = new Set();

  if (prevRef.current !== null) {
    const prev = prevRef.current;
    for (const id of cardIds) {
      if (!prev.has(id)) newIds.add(id);
    }

    // Neighbors of new cards get bumped
    for (let i = 0; i < cardIds.length; i++) {
      if (newIds.has(cardIds[i])) {
        if (i > 0 && !newIds.has(cardIds[i - 1])) bumpedIds.add(cardIds[i - 1]);
        if (i < cardIds.length - 1 && !newIds.has(cardIds[i + 1])) bumpedIds.add(cardIds[i + 1]);
      }
    }
  }

  useEffect(() => {
    prevRef.current = new Set(cardIds);
  });

  return { newIds, bumpedIds };
}
