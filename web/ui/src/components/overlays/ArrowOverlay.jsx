import { useState, useLayoutEffect, useEffect, useCallback } from "react";
import { useCombatArrows } from "@/context/CombatArrowContext";
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

export default function ArrowOverlay() {
  const { arrows, dragArrow } = useCombatArrows();
  const [paths, setPaths] = useState([]);
  const [tick, setTick] = useState(0);

  const computePaths = useCallback(() => {
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
      const to = centerOf(toRect);
      const d = curvedArrowPath(from.x, from.y, to.x, to.y);
      const len = Math.sqrt((to.x - from.x) ** 2 + (to.y - from.y) ** 2) * 1.15;
      result.push({ d, color: arrow.color || "#f0ce61", key: arrow.key, len });
    }
    setPaths(result);
  }, [arrows]);

  useLayoutEffect(computePaths, [computePaths]);

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
    if (tick > 0) computePaths();
  }, [tick, computePaths]);

  // Build live drag arrow path
  let dragPath = null;
  if (dragArrow) {
    const fromRect = getCardRect(dragArrow.fromId);
    if (fromRect) {
      const from = centerOf(fromRect);
      dragPath = {
        d: curvedArrowPath(from.x, from.y, dragArrow.x, dragArrow.y),
        color: dragArrow.color || "#f0ce61",
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
          id="arrowhead-yellow"
          markerWidth="8"
          markerHeight="6"
          refX="7"
          refY="3"
          orient="auto"
        >
          <polygon points="0 0, 8 3, 0 6" fill="#f0ce61" />
        </marker>
        <marker
          id="arrowhead-purple"
          markerWidth="8"
          markerHeight="6"
          refX="7"
          refY="3"
          orient="auto"
        >
          <polygon points="0 0, 8 3, 0 6" fill="#ae76ff" />
        </marker>
        <marker
          id="arrowhead-drag"
          markerWidth="10"
          markerHeight="7"
          refX="9"
          refY="3.5"
          orient="auto"
        >
          <polygon points="0 0, 10 3.5, 0 7" fill="currentColor" opacity="0.8" />
        </marker>
      </defs>

      {/* Confirmed arrows */}
      {paths.map((p) => (
        <path
          key={p.key}
          d={p.d}
          fill="none"
          stroke={p.color}
          strokeWidth={2.5}
          strokeLinecap="round"
          filter="url(#arrow-glow)"
          markerEnd={p.color.includes("ae76ff") ? "url(#arrowhead-purple)" : "url(#arrowhead-yellow)"}
          style={{
            strokeDasharray: p.len,
            strokeDashoffset: p.len,
            animation: `draw-arrow 400ms ease-out forwards`,
          }}
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
          style={{ color: dragPath.color }}
          markerEnd="url(#arrowhead-drag)"
        />
      )}
    </svg>
  );
}
