const DOCK_EDGE_PADDING = 14;
const OBSTACLE_CLEARANCE = 12;

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function intersectionArea(left, top, width, height, obstacle) {
  const overlapWidth = Math.max(0, Math.min(left + width, obstacle.right) - Math.max(left, obstacle.left));
  const overlapHeight = Math.max(0, Math.min(top + height, obstacle.bottom) - Math.max(top, obstacle.top));
  return overlapWidth * overlapHeight;
}

function axisCandidates(obstacles, start, end, preferred, size, nearKey, farKey) {
  const candidates = new Set([start, end, clamp(preferred, start, end)]);
  for (const obstacle of obstacles) {
    candidates.add(clamp(obstacle[nearKey] - size - OBSTACLE_CLEARANCE, start, end));
    candidates.add(clamp(obstacle[farKey] + OBSTACLE_CLEARANCE, start, end));
  }
  return [...candidates];
}

/**
 * Find a viewport position for a floating action dock that avoids visible cards,
 * hand cards, and zone piles. When the board is completely full, return the
 * least-overlapping position so the controls remain available.
 */
export function findFloatingDockPosition({
  viewportWidth,
  viewportHeight,
  dockWidth,
  dockHeight,
  obstacles = [],
  preferredLeft,
  preferredTop,
}) {
  const minLeft = DOCK_EDGE_PADDING;
  const maxLeft = Math.max(minLeft, viewportWidth - dockWidth - DOCK_EDGE_PADDING);
  const minTop = 48;
  const maxTop = Math.max(minTop, viewportHeight - dockHeight - DOCK_EDGE_PADDING);
  const safeObstacles = obstacles
    .filter((obstacle) => obstacle
      && [obstacle.left, obstacle.top, obstacle.right, obstacle.bottom].every(Number.isFinite))
    .map((obstacle) => ({
      left: obstacle.left - OBSTACLE_CLEARANCE,
      top: obstacle.top - OBSTACLE_CLEARANCE,
      right: obstacle.right + OBSTACLE_CLEARANCE,
      bottom: obstacle.bottom + OBSTACLE_CLEARANCE,
      protected: Boolean(obstacle.protected),
    }));
  const targetLeft = clamp(preferredLeft ?? maxLeft, minLeft, maxLeft);
  const targetTop = clamp(preferredTop ?? maxTop, minTop, maxTop);
  const lefts = axisCandidates(safeObstacles, minLeft, maxLeft, targetLeft, dockWidth, "left", "right");
  const tops = axisCandidates(safeObstacles, minTop, maxTop, targetTop, dockHeight, "top", "bottom");

  let best = null;
  for (const top of tops) {
    for (const left of lefts) {
      const protectedOverlap = safeObstacles.reduce(
        (total, obstacle) => obstacle.protected
          ? total + intersectionArea(left, top, dockWidth, dockHeight, obstacle)
          : total,
        0
      );
      const overlap = safeObstacles.reduce(
        (total, obstacle) => total + intersectionArea(left, top, dockWidth, dockHeight, obstacle),
        0
      );
      const distance = Math.abs(left - targetLeft) * 0.72 + Math.abs(top - targetTop);
      const candidate = { left, top, protectedOverlap, overlap, distance };
      const betterProtected = !best || candidate.protectedOverlap < best.protectedOverlap;
      const sameProtected = best && candidate.protectedOverlap === best.protectedOverlap;
      const betterGeneral = sameProtected && (
        (candidate.overlap === 0 && best.overlap > 0)
        || ((candidate.overlap === 0) === (best.overlap === 0)
          && (candidate.overlap < best.overlap
            || (candidate.overlap === best.overlap && candidate.distance < best.distance)))
      );
      if (betterProtected || betterGeneral) {
        best = candidate;
      }
    }
  }

  return best ? { left: Math.round(best.left), top: Math.round(best.top), overlaps: best.overlap > 0 } : null;
}
