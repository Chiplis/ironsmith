import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useCombatArrows } from "@/context/CombatArrowContext";
import { animate, cancelMotion, createDrawable } from "@/lib/motion/anime";
import { getCardRect, getPlayerTargetRect, centerOf } from "@/hooks/useCardPositions";

function curvedArrowPath(x1, y1, x2, y2) {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const dist = Math.sqrt(dx * dx + dy * dy);
  if (dist < 1) return `M ${x1} ${y1} L ${x2} ${y2}`;
  const bow = dist * 0.18;
  const nx = -dy / dist;
  const ny = dx / dist;
  const cx = (x1 + x2) / 2 + nx * bow;
  const cy = (y1 + y2) / 2 + ny * bow;
  return `M ${x1} ${y1} Q ${cx} ${cy} ${x2} ${y2}`;
}

function segmentEntryOnRect(from, to, rect) {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const left = rect.left;
  const right = rect.right;
  const top = rect.top;
  const bottom = rect.bottom;
  let u1 = 0;
  let u2 = 1;

  const clip = (p, q) => {
    if (p === 0) return q >= 0;
    const r = q / p;
    if (p < 0) {
      if (r > u2) return false;
      if (r > u1) u1 = r;
      return true;
    }
    if (r < u1) return false;
    if (r < u2) u2 = r;
    return true;
  };

  if (
    !clip(-dx, from.x - left) ||
    !clip(dx, right - from.x) ||
    !clip(-dy, from.y - top) ||
    !clip(dy, bottom - from.y)
  ) {
    return null;
  }

  return {
    x: from.x + dx * u1,
    y: from.y + dy * u1,
  };
}

function pointBeforeRect(from, rect, gap = 10) {
  const targetCenter = centerOf(rect);
  const entry = segmentEntryOnRect(from, targetCenter, rect);
  if (!entry) return targetCenter;
  const vx = entry.x - from.x;
  const vy = entry.y - from.y;
  const len = Math.hypot(vx, vy);
  if (len < 1e-3) return entry;
  return {
    x: entry.x - (vx / len) * gap,
    y: entry.y - (vy / len) * gap,
  };
}

export default function ArrowOverlay() {
  const { arrows, dragArrow } = useCombatArrows();
  const [, setTick] = useState(0);
  const pathRefs = useRef(new Map());
  const drawAnimationsRef = useRef(new Map());
  const animatedKeysRef = useRef(new Set());
  const paths = (() => {
    const result = [];
    for (const arrow of arrows) {
      const fromRect = getCardRect(arrow.fromId);
      let toRect = null;
      if (arrow.toPlayerId != null) {
        toRect = getPlayerTargetRect(arrow.toPlayerId);
      } else if (arrow.toId != null) {
        toRect = getCardRect(arrow.toId);
      }
      if (!fromRect || !toRect) continue;

      const from = centerOf(fromRect);
      const to = arrow.toPlayerId != null
        ? pointBeforeRect(from, toRect, 9)
        : centerOf(toRect);
      const d = curvedArrowPath(from.x, from.y, to.x, to.y);
      result.push({ d, color: arrow.color || "#ff3b30", key: arrow.key });
    }
    return result;
  })();

  useEffect(() => {
    if (arrows.length === 0) return;
    const recalc = () => setTick((t) => t + 1);
    window.addEventListener("resize", recalc);
    window.addEventListener("scroll", recalc, true);
    return () => {
      window.removeEventListener("resize", recalc);
      window.removeEventListener("scroll", recalc, true);
    };
  }, [arrows.length]);

  useLayoutEffect(() => {
    const animationStore = drawAnimationsRef.current;
    const nextKeys = new Set(paths.map((path) => path.key));

    for (const [key, animation] of animationStore.entries()) {
      if (nextKeys.has(key)) continue;
      cancelMotion(animation);
      animationStore.delete(key);
      animatedKeysRef.current.delete(key);
      pathRefs.current.delete(key);
    }

    for (const path of paths) {
      if (animatedKeysRef.current.has(path.key)) continue;
      const node = pathRefs.current.get(path.key);
      if (!node) continue;

      const [drawable] = createDrawable(node);
      const animation = animate(drawable, {
        draw: ["0 0", "0 1"],
        ease: "out(3)",
        duration: 420,
      });
      animationStore.set(path.key, animation);
      animatedKeysRef.current.add(path.key);
    }

    return () => {
      for (const animation of animationStore.values()) {
        cancelMotion(animation);
      }
      animationStore.clear();
    };
  }, [paths]);

  // Build live drag arrow path
  let dragPath = null;
  if (dragArrow) {
    const fromRect = getCardRect(dragArrow.fromId);
    if (fromRect) {
      const from = centerOf(fromRect);
      dragPath = {
        d: curvedArrowPath(from.x, from.y, dragArrow.x, dragArrow.y),
        color: dragArrow.color || "#ff3b30",
      };
    }
  }

  if (paths.length === 0 && !dragPath) return null;

  return (
    <svg
      className="fixed inset-0 w-full h-full z-[90] pointer-events-none"
      style={{ overflow: "visible" }}
    >
      <defs>
        <filter id="arrow-glow" x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur in="SourceGraphic" stdDeviation="3" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
        <marker
          id="arrowhead-red"
          markerWidth="8"
          markerHeight="6"
          refX="7"
          refY="3"
          orient="auto"
        >
          <polygon points="0 0, 8 3, 0 6" fill="#ff3b30" />
        </marker>
        <marker
          id="arrowhead-blue"
          markerWidth="8"
          markerHeight="6"
          refX="7"
          refY="3"
          orient="auto"
        >
          <polygon points="0 0, 8 3, 0 6" fill="#3b82f6" />
        </marker>
        <marker
          id="arrowhead-drag"
          markerWidth="10"
          markerHeight="7"
          refX="9"
          refY="3.5"
          orient="auto"
        >
          <polygon points="0 0, 10 3.5, 0 7" fill="context-stroke" opacity="0.85" />
        </marker>
      </defs>

      {/* Confirmed arrows */}
      {paths.map((p) => (
        <path
          key={p.key}
          ref={(node) => {
            if (node) {
              pathRefs.current.set(p.key, node);
            } else {
              pathRefs.current.delete(p.key);
            }
          }}
          d={p.d}
          fill="none"
          stroke={p.color}
          strokeWidth={2.5}
          strokeLinecap="round"
          filter="url(#arrow-glow)"
          markerEnd={p.color.includes("3b82f6") ? "url(#arrowhead-blue)" : "url(#arrowhead-red)"}
        />
      ))}

      {/* Live drag arrow */}
      {dragPath && (
        <path
          d={dragPath.d}
          fill="none"
          stroke={dragPath.color}
          strokeWidth={3}
          strokeLinecap="round"
          strokeDasharray="8 4"
          filter="url(#arrow-glow)"
          opacity={0.85}
          markerEnd="url(#arrowhead-drag)"
        />
      )}
    </svg>
  );
}
