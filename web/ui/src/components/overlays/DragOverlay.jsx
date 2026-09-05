import { useDragState } from "@/context/DragContext";
import useScryfallImageUrl from "@/hooks/useScryfallImageUrl";
import {
  battlefieldGridSlotAtPoint,
  battlefieldPlacementForDrag,
} from "@/lib/battlefield-layout";
import { rectBoundaryPointToward } from "@/lib/hand-drag-intent";

const GLOW_COLORS = {
  land: "#59d6a6",
  spell: "#64a9ff",
  ability: "#e0e8f0",
  instant: "#50dcf0",
  sorcery: "#b464ff",
  creature: "#f0be50",
  enchantment: "#f064b4",
  battle: "#f0503c",
  artifact: "#bed2e6",
  planeswalker: "#f7a040",
  extra: "#ae76ff",
};

function pointInsideRect(x, y, rect) {
  return Boolean(
    rect
    && x >= rect.left
    && x <= rect.right
    && y >= rect.top
    && y <= rect.bottom
  );
}

function placementTargetAtPoint(x, y) {
  if (typeof document === "undefined") return null;
  const grid = document.querySelector('[data-battlefield-drop-grid="true"]');
  const gridRect = grid?.getBoundingClientRect?.();
  if (!pointInsideRect(x, y, gridRect)) return null;

  const styles = window.getComputedStyle(grid);
  const cardWidth = Number.parseFloat(styles.getPropertyValue("--bf-card-width")) || 72;
  const cardHeight = Number.parseFloat(styles.getPropertyValue("--bf-card-height")) || 101;
  const gap = Number.parseFloat(styles.getPropertyValue("--bf-gap")) || 4;
  const overlap = Number.parseFloat(styles.getPropertyValue("--bf-card-overlap")) || 0;
  const topSafeInset = Number.parseFloat(styles.getPropertyValue("--bf-top-safe-inset")) || 0;
  const slot = battlefieldGridSlotAtPoint({
    x,
    y,
    left: gridRect.left,
    top: gridRect.top + Math.max(0, topSafeInset),
    width: gridRect.width,
    rows: Number(grid.dataset.battlefieldGridRows),
    columns: Number(grid.dataset.battlefieldGridColumns),
    cardWidth,
    cardHeight,
    gap,
    overlap,
  });
  const slotElement = slot
    ? grid.querySelector(
      `[data-battlefield-drop-slot][data-row="${slot.row}"][data-column="${slot.column}"]`
    )
    : null;
  const slotRect = slotElement?.getBoundingClientRect?.();
  return {
    inside: true,
    x: slotRect ? slotRect.left + (slotRect.width / 2) : x,
    y: slotRect ? slotRect.top + (slotRect.height / 2) : y,
  };
}

function placementArrowPath(source, target) {
  const startX = source.x;
  const startY = source.y;
  const endX = target.x;
  const endY = target.y;
  const rise = Math.max(76, Math.min(220, Math.abs(startY - endY) * 0.42));
  const bend = Math.max(-180, Math.min(180, (endX - startX) * 0.24));
  return `M ${startX} ${startY} C ${startX + bend} ${startY - rise}, ${endX - bend} ${endY + rise * 0.45}, ${endX} ${endY}`;
}

export default function DragOverlay() {
  const dragState = useDragState();
  const imageUrl = useScryfallImageUrl(dragState?.cardName || "", "normal");
  if (!dragState) return null;

  const {
    cardName,
    glowKind,
    currentX,
    currentY,
    sourceRect,
    sourceContainerRect,
    hiddenSourcePoint,
    startX,
    startY,
    castIntent,
  } = dragState;
  const placement = battlefieldPlacementForDrag(dragState);
  const isBattlefieldMove = placement?.kind === "move_battlefield";
  const placementTarget = placement ? placementTargetAtPoint(currentX, currentY) : null;
  const color = castIntent ? "#67c7ff" : (GLOW_COLORS[glowKind] || "#c8d2dc");
  const sourcePoint = castIntent?.sourcePoint || hiddenSourcePoint || rectBoundaryPointToward(
    sourceContainerRect || sourceRect,
    startX,
    startY,
    currentX,
    currentY,
  );
  const arrowTarget = castIntent
    ? { x: currentX, y: currentY }
    : (!isBattlefieldMove && placement && placementTarget?.inside ? placementTarget : null);

  if (arrowTarget) {
    const path = placementArrowPath(sourcePoint, arrowTarget);
    return (
      <svg
        className={`${castIntent ? "cast-intent-drag-arrow" : "placement-drag-arrow"} fixed inset-0 h-screen w-screen pointer-events-none`}
        viewBox={`0 0 ${window.innerWidth} ${window.innerHeight}`}
        aria-hidden="true"
      >
        <defs>
          <filter id="hand-drag-arrow-glow" x="-50%" y="-50%" width="200%" height="200%">
            <feGaussianBlur stdDeviation="3" result="blur" />
            <feMerge>
              <feMergeNode in="blur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
          <marker
            id="hand-drag-arrowhead"
            viewBox="0 0 12 12"
            refX="10"
            refY="6"
            markerWidth="7"
            markerHeight="7"
            orient="auto-start-reverse"
          >
            <path d="M 0 1 L 11 6 L 0 11 z" fill={color} />
          </marker>
        </defs>
        <path
          className="placement-drag-arrow__shadow"
          d={path}
          fill="none"
          stroke="rgba(0,0,0,0.72)"
          strokeWidth="7"
          strokeLinecap="round"
        />
        <path
          className="placement-drag-arrow__line"
          d={path}
          fill="none"
          stroke={color}
          strokeWidth="3"
          strokeLinecap="round"
          strokeDasharray="11 9"
          markerEnd="url(#hand-drag-arrowhead)"
          filter="url(#hand-drag-arrow-glow)"
        />
      </svg>
    );
  }

  const width = Math.max(88, Math.min(148, Number(sourceRect?.width) || 112));
  const height = width * (88 / 63);
  const rotation = Math.max(-7, Math.min(7, (currentX - startX) * 0.035));

  return (
    <div
      className="fixed z-[998] pointer-events-none"
      style={{ left: currentX, top: currentY }}
      aria-hidden="true"
    >
      <div
        className={`dragged-full-card overflow-hidden rounded-[5%]${isBattlefieldMove ? " dragged-full-card--battlefield-move" : ""}`}
        style={{
          width,
          height,
          transform: `translate(-50%, -58%) rotate(${rotation}deg)`,
          border: `2px solid ${color}`,
          boxShadow: `0 0 24px ${color}88, 0 18px 36px rgba(0,0,0,0.62)`,
          opacity: isBattlefieldMove ? 0.66 : 1,
        }}
      >
        {imageUrl ? (
          <img className="h-full w-full object-cover" src={imageUrl} alt={cardName} draggable="false" />
        ) : (
          <div className="flex h-full w-full items-start bg-[#0b1119] p-2 text-[12px] font-bold text-[#d8e8ff]">
            {cardName}
          </div>
        )}
      </div>
    </div>
  );
}
