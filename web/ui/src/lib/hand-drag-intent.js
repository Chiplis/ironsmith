function finiteNumber(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

export function plainRect(rect) {
  if (!rect) return null;
  const left = finiteNumber(rect.left);
  const top = finiteNumber(rect.top);
  const right = finiteNumber(rect.right);
  const bottom = finiteNumber(rect.bottom);
  if ([left, top, right, bottom].some((value) => value == null)) return null;
  return {
    left,
    top,
    right,
    bottom,
    width: finiteNumber(rect.width) ?? Math.max(0, right - left),
    height: finiteNumber(rect.height) ?? Math.max(0, bottom - top),
  };
}

export function pointIsOutsideRect(rect, x, y) {
  const normalized = plainRect(rect);
  const px = finiteNumber(x);
  const py = finiteNumber(y);
  if (!normalized || px == null || py == null) return false;
  return px < normalized.left || px > normalized.right || py < normalized.top || py > normalized.bottom;
}

/** Anchor a hand gesture where the card emerges from its collapsed fan slot. */
export function handCardSourcePoint(rect) {
  const normalized = plainRect(rect);
  if (!normalized) return null;
  return {
    x: normalized.left + (normalized.width / 2),
    y: normalized.top,
  };
}

/** Project the held card's source position onto the battlefield-facing hand edge. */
export function rectBoundaryPointToward(rect, startX, startY, targetX, targetY) {
  const normalized = plainRect(rect);
  const sx = finiteNumber(startX);
  const sy = finiteNumber(startY);
  const tx = finiteNumber(targetX);
  const ty = finiteNumber(targetY);
  if (!normalized || [sx, sy, tx, ty].some((value) => value == null)) {
    return { x: sx ?? tx ?? 0, y: sy ?? ty ?? 0 };
  }

  const sourceX = Math.min(normalized.right, Math.max(normalized.left, sx));
  const sourceY = Math.min(normalized.bottom, Math.max(normalized.top, sy));

  // Preserve the source card's position along the hand edge. A ray/rectangle
  // intersection lets a distant target pull the start point sideways, making
  // an Elf at the right of the fan look as if it came from the middle.
  if (normalized.width >= normalized.height) {
    return {
      x: sourceX,
      y: ty <= ((normalized.top + normalized.bottom) / 2)
        ? normalized.top
        : normalized.bottom,
    };
  }
  return {
    x: tx <= ((normalized.left + normalized.right) / 2)
      ? normalized.left
      : normalized.right,
    y: sourceY,
  };
}

export function shouldBeginTargetCastIntent(actions) {
  if (!Array.isArray(actions) || actions.length === 0) return false;
  return actions.every((action) => (
    String(action?.kind || action?.action_ref?.kind || "") === "cast_spell"
    && action?.drag_requires_targets === true
    && action?.drag_requires_modes !== true
  ));
}

function commaSeparatedNumbers(value) {
  return String(value || "")
    .split(",")
    .map((part) => finiteNumber(part.trim()))
    .filter((part) => part != null);
}

/** Capture the raw board identity under a release point before the next UI snapshot arrives. */
export function dropTargetCandidateFromElement(element) {
  if (!element || typeof element.closest !== "function") return null;
  const card = element.closest(".game-card[data-object-id]");
  if (card) {
    const objectIds = [
      finiteNumber(card.getAttribute("data-object-id")),
      ...commaSeparatedNumbers(card.getAttribute("data-member-object-ids")),
    ].filter((value) => value != null);
    if (objectIds.length > 0) {
      return { kind: "object", objectIds: Array.from(new Set(objectIds)) };
    }
  }

  const player = element.closest("[data-player-target], [data-player-drop-target]");
  const playerId = finiteNumber(
    player?.getAttribute?.("data-player-target")
      ?? player?.getAttribute?.("data-player-drop-target")
  );
  return playerId == null ? null : { kind: "player", playerIds: [playerId] };
}

/** Prefer the specific card under a pointer to its enclosing player battlefield. */
export function dropTargetCandidateFromElements(elements) {
  const candidates = Array.from(elements || [])
    .map((element) => dropTargetCandidateFromElement(element))
    .filter(Boolean);
  return candidates.find((candidate) => candidate.kind === "object")
    || candidates.find((candidate) => candidate.kind === "player")
    || null;
}

export function legalTargetForDropCandidate(decision, candidate) {
  if (!decision || decision.kind !== "targets" || !candidate) return null;
  const legalTargets = (decision.requirements || []).flatMap((requirement) => requirement?.legal_targets || []);
  if (candidate.kind === "object") {
    const ids = new Set((candidate.objectIds || []).map(Number));
    const match = legalTargets.find((target) => (
      target?.kind === "object" && ids.has(Number(target.object))
    ));
    return match ? { kind: "object", object: Number(match.object), name: match.name } : null;
  }
  if (candidate.kind === "player") {
    const ids = new Set((candidate.playerIds || []).map(Number));
    const match = legalTargets.find((target) => (
      target?.kind === "player" && ids.has(Number(target.player))
    ));
    return match ? { kind: "player", player: Number(match.player), name: match.name } : null;
  }
  return null;
}

/** Resolve card candidates before broad player containers when both remain under a release point. */
export function legalTargetForDropCandidates(decision, candidates) {
  const ordered = Array.from(candidates || []).filter(Boolean);
  const objectCandidates = ordered.filter((candidate) => candidate.kind === "object");
  const playerCandidates = ordered.filter((candidate) => candidate.kind === "player");
  for (const candidate of [...objectCandidates, ...playerCandidates]) {
    const target = legalTargetForDropCandidate(decision, candidate);
    if (target) return target;
  }
  return null;
}

export function targetDropCompletesDecision(decision, target) {
  const requirements = decision?.kind === "targets" ? (decision.requirements || []) : [];
  if (requirements.length !== 1 || !target) return false;
  const requirement = requirements[0];
  const min = Number(requirement?.min_targets ?? 1);
  const max = Number(requirement?.max_targets ?? requirement?.legal_targets?.length ?? 1);
  if (min !== 1 || max !== 1) return false;
  return (requirement?.legal_targets || []).some((candidate) => (
    candidate?.kind === target.kind
    && (target.kind === "player"
      ? Number(candidate.player) === Number(target.player)
      : Number(candidate.object) === Number(target.object))
  ));
}
