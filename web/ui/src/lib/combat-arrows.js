// Combat arrows the whole table sees. The declaring seat draws its own
// in-progress arrows from AttackersDecision / BlockersDecision; once a
// declaration is accepted the engine's combat state carries it, and these
// arrows keep every seat's attackers and blockers on screen until combat ends.

export const ATTACKER_COLOR = "#ff6b5f";
export const BLOCKER_COLOR = "#ff8b63";

function toNumberOrNull(value) {
  if (value == null) return null;
  const n = Number(value);
  return Number.isFinite(n) ? n : null;
}

// `objectControllerById` maps String(objectId) -> playerId. It only anchors an
// arrow to the controller's seat when the attacked planeswalker or battle has no
// element on screen (for example while its pile is closed).
export function buildCombatStateArrows(combat, objectControllerById = null) {
  const arrows = [];
  if (!combat) return arrows;

  for (const attacker of combat.attackers || []) {
    const fromId = toNumberOrNull(attacker?.creature);
    if (fromId == null) continue;
    const target = attacker.target || {};
    const toPlayerId = target.kind === "player" ? toNumberOrNull(target.player) : null;
    const toId = target.kind === "player" ? null : toNumberOrNull(target.object);
    if (toPlayerId == null && toId == null) continue;
    const toFallbackPlayerId = toId != null
      ? toNumberOrNull(objectControllerById?.get?.(String(toId)))
      : null;
    arrows.push({
      fromId,
      toId,
      toPlayerId,
      toFallbackPlayerId,
      color: ATTACKER_COLOR,
      // "atk-" and "blk-" prefixes select the dashed combat stroke in ArrowOverlay.
      key: `atk-state-${fromId}`,
    });
  }

  for (const block of combat.blockers || []) {
    const fromId = toNumberOrNull(block?.blocker);
    const toId = toNumberOrNull(block?.blocking);
    if (fromId == null || toId == null) continue;
    arrows.push({
      fromId,
      toId,
      toPlayerId: null,
      color: BLOCKER_COLOR,
      key: `blk-state-${fromId}-${toId}`,
    });
  }

  return arrows;
}

export function combatStateArrowSignature(arrows) {
  return arrows.map((arrow) => `${arrow.key}>${arrow.toId ?? ""}:${arrow.toPlayerId ?? ""}:${arrow.toFallbackPlayerId ?? ""}`).join("|");
}
