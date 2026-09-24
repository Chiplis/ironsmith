import { useMemo } from "react";
import { PUBLIC_FORMATS } from "@/lib/relay/formats";
import {
  COMMANDER_DECK_SIZE,
  LOBBY_DECK_SIZE,
  MATCH_FORMAT_COMMANDER,
  MATCH_FORMAT_PLANECHASE,
  PARTNER_DECK_SIZE,
  listSavedDeckPresets,
  normalizeMatchFormat,
} from "@/lib/decklists";

// Wording and deck choices shared by the lobby sheet and the rematch screen.

export function formatName(format) {
  if (PUBLIC_FORMATS[format]) return PUBLIC_FORMATS[format].label;
  const normalized = normalizeMatchFormat(format);
  if (normalized === MATCH_FORMAT_COMMANDER) return "Commander";
  if (normalized === MATCH_FORMAT_PLANECHASE) return "Planechase";
  return "Normal";
}

export function commanderDeckTarget(commanderCount) {
  return commanderCount === 2 ? PARTNER_DECK_SIZE : COMMANDER_DECK_SIZE;
}

export function formatDeckRequirement(format) {
  if (PUBLIC_FORMATS[format] && format !== MATCH_FORMAT_COMMANDER) return "At least 60 main-deck cards; up to 15 sideboard cards. Format bans and copy limits apply.";
  const normalized = normalizeMatchFormat(format);
  if (normalized === MATCH_FORMAT_COMMANDER) {
    return `Submit a ${COMMANDER_DECK_SIZE}-card main deck plus 1 commander, or a ${PARTNER_DECK_SIZE}-card main deck plus 2 commanders.`;
  }
  if (normalized === MATCH_FORMAT_PLANECHASE) {
    return `Submit at least ${LOBBY_DECK_SIZE} main-deck cards plus at least 10 uniquely named Plane or Phenomenon cards.`;
  }
  return `Submit at least ${LOBBY_DECK_SIZE} main-deck cards.`;
}

// The host's prepared decks first, then the player's saved presets, without
// repeating a list.
export function useLobbyDeckOptions(preparedOptions) {
  const savedDeckOptions = useMemo(
    () => listSavedDeckPresets().flatMap((preset) => (Array.isArray(preset?.texts) ? preset.texts : [])
      .map((deckText, index) => ({
        id: `saved:${preset.name}:${index}`,
        label: `${preset.name}${preset.texts.length > 1 ? ` #${index + 1}` : ""} (${preset.playerNames?.[index] || `Jugador ${index + 1}`})`,
        deckText: String(deckText || ""),
      }))
      .filter((option) => option.deckText.trim())),
    [],
  );
  return useMemo(() => {
    const seen = new Set();
    return [...(preparedOptions || []), ...savedDeckOptions]
      .filter((option) => {
        const key = String(option?.deckText || "");
        if (!key.trim() || seen.has(key)) return false;
        seen.add(key);
        return true;
      })
      .slice(0, 12);
  }, [preparedOptions, savedDeckOptions]);
}

