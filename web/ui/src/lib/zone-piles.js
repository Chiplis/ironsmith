export const PILE_ZONES = ["graveyard", "exile"];

export function zoneHasLegalTargets(state, decision, candidate) {
  if (candidate?.kind !== "zone" || decision?.kind !== "targets") return false;
  const player = state?.players?.find((entry) => String(entry.id ?? entry.index) === String(candidate.playerId));
  const ids = new Set(zonePileCards(player, candidate.zone).map((card) => String(card.id)));
  return (decision.requirements || []).some((req) => (req.legal_targets || []).some((target) =>
    target.kind === "object" && ids.has(String(target.object))
  ));
}

export function isFaceUpZoneCard(card) {
  return Boolean(card?.name)
    && !card.face_down && !card.is_face_down
    && !/^(hidden card|face.down card|unknown card)$/i.test(card.name.trim());
}

// The engine serializes both zones in reverse insertion order already.
export function zonePileCards(player, zone) {
  return Array.isArray(player?.[`${zone}_cards`]) ? player[`${zone}_cards`] : [];
}

export function zonePileDestination(preview) {
  if (!PILE_ZONES.includes(preview?.toZone)) return null;
  const owner = preview.card?.owner ?? preview.owner ?? preview.playerKey;
  return owner == null ? null : { playerId: String(owner), zone: preview.toZone };
}

const commandZoneSize = (player) =>
  Number(player?.command_size ?? (Array.isArray(player?.command_cards) ? player.command_cards.length : 0)) || 0;

// The snapshot carries no format flag, so a table counts as a commander game
// once any player has a card in the command zone, and stays one while the
// commanders are out on the battlefield. A new game (turn number going back,
// or an opening turn with an empty command zone) clears it.
let commanderTableSeen = false;
let commanderTableTurn = -1;
export function isCommanderTable(state) {
  const turn = Number(state?.turn_number) || 0;
  const occupied = (state?.players || []).some((player) => commandZoneSize(player) > 0);
  if (turn < commanderTableTurn || (turn <= 1 && !occupied)) commanderTableSeen = false;
  commanderTableTurn = turn;
  if (occupied) commanderTableSeen = true;
  return commanderTableSeen;
}

export function pileZonesFor(state) {
  return isCommanderTable(state) ? [...PILE_ZONES, "command"] : PILE_ZONES;
}
