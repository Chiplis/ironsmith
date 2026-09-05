export const DEFAULT_PLAYER_ACCENT = { hex: "#b79cff", rgb: "183, 156, 255" };

const PLAYER_ACCENT_PALETTE = [
  DEFAULT_PLAYER_ACCENT,
  { hex: "#ff3b30", rgb: "255, 59, 48" },
  { hex: "#22c55e", rgb: "34, 197, 94" },
  { hex: "#f28c28", rgb: "242, 140, 40" },
];

function normalizeHexColor(color) {
  const raw = String(color || "").trim();
  const match = raw.match(/^#?([0-9a-f]{6})$/i);
  return match ? `#${match[1].toLowerCase()}` : null;
}

export function hexToRgbString(color) {
  const hex = normalizeHexColor(color);
  if (!hex) return null;
  const value = Number.parseInt(hex.slice(1), 16);
  return `${(value >> 16) & 255}, ${(value >> 8) & 255}, ${value & 255}`;
}

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

export function getPlayerAccent(players, playerId, perspectivePlayerId = null, accentOverrides = null) {
  const numericPlayerId = Number(playerId);
  const override = accentOverrides && numericPlayerId != null
    ? normalizeHexColor(accentOverrides[String(numericPlayerId)])
    : null;
  if (override) {
    return {
      hex: override,
      rgb: hexToRgbString(override),
      seatIndex: getPlayerSeatIndex(players, playerId),
    };
  }

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

export function decisionOptionAccentVars(accent) {
  if (!accent) return undefined;
  return {
    "--decision-option-accent": accent.hex,
    "--decision-option-accent-rgb": accent.rgb,
  };
}
