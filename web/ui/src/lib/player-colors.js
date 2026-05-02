const PLAYER_ACCENT_PALETTE = [
  { hex: "#ff3b30", rgb: "255, 59, 48" },
  { hex: "#3b82f6", rgb: "59, 130, 246" },
  { hex: "#f4c430", rgb: "244, 196, 48" },
  { hex: "#22c55e", rgb: "34, 197, 94" },
];

function modulo(value, size) {
  return ((value % size) + size) % size;
}

export function getPlayerSeatIndex(players, playerId) {
  const numericPlayerId = Number(playerId);
  const seatIndex = Array.isArray(players)
    ? players.findIndex((player) => Number(player?.id) === numericPlayerId)
    : -1;
  if (seatIndex >= 0) return seatIndex;
  if (Number.isFinite(numericPlayerId)) return numericPlayerId;
  return 0;
}

export function getPlayerAccent(players, playerId, perspectivePlayerId = null) {
  if (PLAYER_ACCENT_PALETTE.length === 0) return null;
  const seatIndex = getPlayerSeatIndex(players, playerId);
  const perspectiveSeatIndex = perspectivePlayerId == null
    ? 0
    : getPlayerSeatIndex(players, perspectivePlayerId);
  const playerCount = Array.isArray(players) && players.length > 0
    ? players.length
    : PLAYER_ACCENT_PALETTE.length;
  const relativeSeatIndex = modulo(seatIndex - perspectiveSeatIndex, playerCount);
  const paletteIndex = modulo(relativeSeatIndex, PLAYER_ACCENT_PALETTE.length);
  return {
    ...PLAYER_ACCENT_PALETTE[paletteIndex],
    seatIndex,
  };
}

export function playerAccentVars(accent) {
  if (!accent) return undefined;
  return {
    "--player-accent": accent.hex,
    "--player-accent-rgb": accent.rgb,
  };
}
