import { useLayoutEffect, useRef } from "react";

const selectors = [".topbar-phase-status", ".table-shared-player-header"];

/** Carry controls between the normal and decision layouts without scaling text. */
export default function useDecisionControlMotion(rootRef, expanded) {
  const positions = useRef(new Map());

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const entries = new Map();
    for (const selector of selectors) {
      const node = root.querySelector(selector);
      const previous = positions.current.get(selector);
      if (previous?.node === node && previous.expanded === expanded) {
        entries.set(selector, previous);
        continue;
      }
      // A rapid reversal starts where the previous animation is currently drawn,
      // even when React has replaced its DOM node during the layout change.
      const progress = previous?.animation?.effect?.getComputedTiming().progress ?? 1;
      previous?.animation?.cancel();
      if (!node) continue;
      const rect = node.getBoundingClientRect();
      const dx = previous ? previous.x + previous.dx * (1 - progress) - rect.x : 0;
      const dy = previous ? previous.y + previous.dy * (1 - progress) - rect.y : 0;
      const entry = { x: rect.x, y: rect.y, dx, dy, node };
      if (previous && previous.expanded !== expanded && !reducedMotion && (Math.abs(dx) > 0.5 || Math.abs(dy) > 0.5)) {
        entry.animation = node.animate([
          { translate: `${dx}px ${dy}px` },
          { translate: "0px 0px" },
        ], { duration: 300, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" });
      }
      entry.expanded = expanded;
      entries.set(selector, entry);
    }
    positions.current = entries;

    // Keep the starting positions fresh when the viewport or control size changes.
    const observer = new ResizeObserver(() => {
      for (const entry of entries.values()) {
        if (entry.animation?.playState === "running") continue;
        const rect = entry.node.getBoundingClientRect();
        entry.x = rect.x;
        entry.y = rect.y;
      }
    });
    observer.observe(root);
    for (const entry of entries.values()) observer.observe(entry.node);
    return () => observer.disconnect();
  });

  useLayoutEffect(() => () => {
    for (const entry of positions.current.values()) entry.animation?.cancel();
    positions.current.clear();
  }, []);
}
