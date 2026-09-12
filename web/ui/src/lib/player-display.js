import { translateUiText } from "../i18n/catalog.js";
export function samePlayerId(left, right) {
  const leftNumber = Number(left);
  const rightNumber = Number(right);
  return Number.isFinite(leftNumber) && Number.isFinite(rightNumber) && leftNumber === rightNumber;
}

export function playerDisplayName(players, playerOrId) {
  const entries = Array.isArray(players) ? players : [];
  const id = typeof playerOrId === "object" && playerOrId !== null
    ? playerOrId.id
    : playerOrId;
  const player = typeof playerOrId === "object" && playerOrId !== null
    ? playerOrId
    : entries.find((entry) => samePlayerId(entry?.id, id) || samePlayerId(entry?.index, id));
  if (!player) return "?";

  const name = String(player.name || translateUiText("Player {0}", { 0: Number(player.id ?? player.index ?? 0) + 1 }));
  const duplicateCount = entries.filter((entry) =>
    String(entry?.name || "").trim().toLowerCase() === name.trim().toLowerCase()
  ).length;
  if (duplicateCount <= 1) return name;

  const seat = Number(player.id ?? player.index);
  return Number.isFinite(seat) ? `${name} ${translateUiText("P{0}", { 0: seat + 1 })}` : name;
}
