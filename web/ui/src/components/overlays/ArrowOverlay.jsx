import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useCombatArrows } from "@/context/useCombatArrows";
import { animate, cancelMotion } from "@/lib/motion/anime";
import { getCardElement, getCardRect, getPlayerTargetRect, centerOf } from "@/hooks/useCardPositions";

const ARROW_DASH_ARRAY = "8 4";
const STACK_ROUTE_GAP = 6;
const TARGETING_ARROW_OPACITY = 0.7;

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

function stackedRoutedArrowPath(from, to, fromRect, toRect) {
  const baseY = Math.max(fromRect.bottom, toRect.bottom) + STACK_ROUTE_GAP;
  const span = Math.abs(to.x - from.x);
  const horizontalLead = Math.max(26, Math.min(72, span * 0.32));
  const verticalDip = Math.max(14, Math.min(44, span * 0.15));
  const c1x = from.x + (to.x >= from.x ? horizontalLead : -horizontalLead);
  const c1y = baseY + verticalDip;
  const c2x = to.x - (to.x >= from.x ? horizontalLead : -horizontalLead);
  const c2y = baseY + verticalDip;
  return `M ${from.x} ${from.y} C ${c1x} ${c1y}, ${c2x} ${c2y}, ${to.x} ${to.y}`;
}

function rectBottomAnchor(rect, gap = 0) {
  return {
    x: (rect.left + rect.right) / 2,
    y: rect.bottom + gap,
  };
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

function segmentExitOnRect(from, to, rect) {
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
    x: from.x + dx * u2,
    y: from.y + dy * u2,
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

function pointAfterRect(rect, to, gap = 10) {
  const sourceCenter = centerOf(rect);
  const exit = segmentExitOnRect(sourceCenter, to, rect);
  if (!exit) return sourceCenter;
  const vx = to.x - sourceCenter.x;
  const vy = to.y - sourceCenter.y;
  const len = Math.hypot(vx, vy);
  if (len < 1e-3) return exit;
  return {
    x: exit.x + (vx / len) * gap,
    y: exit.y + (vy / len) * gap,
  };
}

export default function ArrowOverlay() {
  const { arrows, dragArrow } = useCombatArrows();
  const [, setTick] = useState(0);
  const pathRefs = useRef(new Map());
  const pathAnimationsRef = useRef(new Map());
  const animatedKeysRef = useRef(new Set());
  const overlayActive = arrows.length > 0 || !!dragArrow;
  const paths = (() => {
    const result = [];
    for (const arrow of arrows) {
      const fromEl = getCardElement(arrow.fromId);
      const fromRect = getCardRect(arrow.fromId);
      let toRect = null;
      let toEl = null;
      if (arrow.toPlayerId != null) {
        toRect = getPlayerTargetRect(arrow.toPlayerId);
      } else if (arrow.toId != null) {
        toEl = getCardElement(arrow.toId);
        toRect = getCardRect(arrow.toId);
      }
      if (!fromRect || !toRect || !fromEl) continue;

      const sourceCenter = centerOf(fromRect);
      const sourceIsStackAnchor = fromEl.getAttribute("data-arrow-anchor") === "stack";
      const targetIsStackAnchor = toEl?.getAttribute("data-arrow-anchor") === "stack";
      const initialTo = arrow.toPlayerId != null
        || targetIsStackAnchor
        ? pointBeforeRect(sourceCenter, toRect, targetIsStackAnchor ? 14 : 9)
        : centerOf(toRect);
      const stackToStack = sourceIsStackAnchor && targetIsStackAnchor;
      const from = stackToStack
        ? rectBottomAnchor(fromRect, 2)
        : (
          sourceIsStackAnchor
            ? pointAfterRect(fromRect, initialTo, 10)
            : sourceCenter
        );
      const to = stackToStack
        ? rectBottomAnchor(toRect, 2)
        : (
          arrow.toPlayerId != null || targetIsStackAnchor
            ? pointBeforeRect(from, toRect, targetIsStackAnchor ? 14 : 9)
            : centerOf(toRect)
        );
      const d = stackToStack
        ? stackedRoutedArrowPath(from, to, fromRect, toRect)
        : curvedArrowPath(from.x, from.y, to.x, to.y);
      result.push({ d, color: arrow.color || "#ff3b30", key: arrow.key });
    }
    return result;
  })();
  const pathSignature = paths.map((path) => path.key).join("|");
  const pathKeys = useMemo(
    () => (pathSignature ? pathSignature.split("|") : []),
    [pathSignature]
  );

  useEffect(() => {
    if (!overlayActive) return;

    let frameId = 0;
    const recalc = () => setTick((t) => t + 1);
    const tick = () => {
      recalc();
      frameId = window.requestAnimationFrame(tick);
    };

    frameId = window.requestAnimationFrame(tick);
    window.addEventListener("resize", recalc);
    window.addEventListener("scroll", recalc, true);
    return () => {
      if (frameId) {
        window.cancelAnimationFrame(frameId);
      }
      window.removeEventListener("resize", recalc);
      window.removeEventListener("scroll", recalc, true);
    };
  }, [overlayActive]);

  useLayoutEffect(() => {
    const animationStore = pathAnimationsRef.current;
    const nextKeys = new Set(pathKeys);

    for (const [key, animation] of animationStore.entries()) {
      if (nextKeys.has(key)) continue;
      cancelMotion(animation);
      animationStore.delete(key);
      animatedKeysRef.current.delete(key);
      pathRefs.current.delete(key);
    }

    for (const key of pathKeys) {
      if (animatedKeysRef.current.has(key)) continue;
      const node = pathRefs.current.get(key);
      if (!node) continue;

      const animation = animate(node, {
        opacity: [0, 1],
        ease: "out(3)",
        duration: 220,
      });
      animationStore.set(key, animation);
      animatedKeysRef.current.add(key);
    }

    return () => {
      for (const animation of animationStore.values()) {
        cancelMotion(animation);
      }
      animationStore.clear();
    };
  }, [pathKeys]);

  // Build live drag arrow path
  let dragPath = null;
  if (dragArrow) {
    const fromEl = getCardElement(dragArrow.fromId);
    const fromRect = getCardRect(dragArrow.fromId);
    if (fromRect) {
      const dragTarget = { x: dragArrow.x, y: dragArrow.y };
      const from = fromEl?.getAttribute("data-arrow-anchor") === "stack"
        ? pointAfterRect(fromRect, dragTarget, 10)
        : centerOf(fromRect);
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
          id="arrowhead-confirmed"
          markerWidth="8"
          markerHeight="6"
          refX="7"
          refY="3"
          orient="auto"
        >
          <polygon points="0 0, 8 3, 0 6" fill="context-stroke" opacity={TARGETING_ARROW_OPACITY} />
        </marker>
        <marker
          id="arrowhead-drag"
          markerWidth="10"
          markerHeight="7"
          refX="9"
          refY="3.5"
          orient="auto"
        >
          <polygon points="0 0, 10 3.5, 0 7" fill="context-stroke" opacity={TARGETING_ARROW_OPACITY} />
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
          strokeDasharray={ARROW_DASH_ARRAY}
          filter="url(#arrow-glow)"
          opacity={TARGETING_ARROW_OPACITY}
          markerEnd="url(#arrowhead-confirmed)"
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
          strokeDasharray={ARROW_DASH_ARRAY}
          filter="url(#arrow-glow)"
          opacity={TARGETING_ARROW_OPACITY}
          markerEnd="url(#arrowhead-drag)"
        />
      )}
    </svg>
  );
}
