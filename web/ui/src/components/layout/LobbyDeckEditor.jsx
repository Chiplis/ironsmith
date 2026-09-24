import useUiText from "@/i18n/useUiText";
import { useMemo } from "react";
import CompetitiveDeckPicker from "./CompetitiveDeckPicker";
import PreservingTextarea from "@/components/ui/PreservingTextarea";
import {
  LOBBY_DECK_SIZE,
  MATCH_FORMAT_COMMANDER,
  MATCH_FORMAT_NORMAL,
  MATCH_FORMAT_PLANECHASE,
  normalizeMatchFormat,
} from "@/lib/decklists";
import { commanderDeckTarget, formatDeckRequirement, formatName } from "@/lib/lobby-deck";

// The deck a player brings to a lobby seat: the lobby sheet shows it when they
// join, and the rematch screen shows the same controls between games.

const lobbyLabelClass =
  "grid gap-1 text-[12px] uppercase tracking-[0.18em] text-muted-foreground";
const lobbyInputClass =
  "fantasy-field w-full px-3 py-2 text-[14px] text-foreground outline-none";
const lobbyTextareaClass =
  "fantasy-field min-h-[220px] w-full p-3 text-[14px] text-foreground outline-none font-mono resize-none";
const lobbyCommanderTextareaClass =
  "fantasy-field lobby-sheet-commander-input min-h-[108px] w-full p-3 text-[14px] text-foreground outline-none font-mono resize-none";
const lobbyInfoTextClass = "grid gap-1 text-[13px] leading-6 text-muted-foreground";

// Catalog decks are Modern lists, so the picker only serves normal tables.
export function LobbyDeckCatalogPicker({ format, disabled = false, onChange }) {
  if (disabled || normalizeMatchFormat(format) !== MATCH_FORMAT_NORMAL) return null;
  return (
    <CompetitiveDeckPicker
      format="modern"
      onApply={({ deckText, commanderText }) => {
        onChange({ deckText, commanderText: commanderText || "" });
      }}
    />
  );
}

export default function LobbyDeckEditor({
  format,
  deckText,
  commanderText,
  deckCount,
  commanderCount,
  deckOptions = [],
  disabled = false,
  readyText = "",
  onChange,
}) {
  const ui = useUiText();
  const activeFormat = normalizeMatchFormat(format);
  const selectedDeckOptionId = useMemo(() => {
    const current = String(deckText || "");
    return deckOptions.find((option) => String(option?.deckText || "") === current)?.id || "";
  }, [deckOptions, deckText]);
  const activeCommanderTarget = commanderDeckTarget(commanderCount);
  return (
    <div className="lobby-sheet-deck lobby-sheet-panel fantasy-sheet-section grid gap-3 p-4">
      <div className="flex items-center justify-between">
        <span className="text-[10px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">{ui("Your Deck")}</span>
        <span className="text-[13px] text-muted-foreground">{ui("Format:") + " "}{ui(formatName(activeFormat))}
        </span>
      </div>
      {deckOptions.length > 1 ? (
        <label className={lobbyLabelClass}>
          {ui("Available deck")}
          <select
            className={lobbyInputClass}
            value={selectedDeckOptionId}
            disabled={disabled}
            onChange={(event) => {
              const option = deckOptions.find((entry) => entry.id === event.target.value);
              if (!option) return;
              onChange({ deckText: option.deckText, commanderText: "" });
            }}
          >
            <option value="">{ui("Custom / edit below")}</option>
            {/* A prepared deck's label is a deck name and a
                player name, so it never goes through the
                translation catalog. */}
            {deckOptions.map((option) => (
              <option key={option.id} value={option.id}>{option.label}</option>
            ))}
          </select>
        </label>
      ) : null}
      <PreservingTextarea
        aria-label={ui("Your Deck")}
        className={`${lobbyTextareaClass} lobby-sheet-main-deck`}
        disabled={disabled}
        value={deckText}
        onChange={(event) => onChange({ deckText: event.target.value })}
        placeholder={
          ui(activeFormat === MATCH_FORMAT_COMMANDER
            ? `Paste your Commander main deck...\n\n1 Sol Ring\n1 Brainstorm\n33 Island`
            : `Paste a main deck with at least ${LOBBY_DECK_SIZE} cards...\n\n4 Swords to Plowshares\n4 Brainstorm\n24 Plains`)
        }
      />
      <div className={lobbyInfoTextClass}>
        <span>{ui("Main deck:")}{" "}
          {activeFormat === MATCH_FORMAT_COMMANDER
            ? `${deckCount}/${activeCommanderTarget}`
            : `${deckCount}/${LOBBY_DECK_SIZE}+`}
        </span>
        {activeFormat === MATCH_FORMAT_COMMANDER
        || activeFormat === MATCH_FORMAT_PLANECHASE ? (
          <>
            <PreservingTextarea
              className={lobbyCommanderTextareaClass}
              disabled={disabled}
              value={commanderText}
              onChange={(event) => onChange({ commanderText: event.target.value })}
              placeholder={
                ui(activeFormat === MATCH_FORMAT_PLANECHASE
                  ? "1 Plane or Phenomenon per line"
                  : "1 Commander\nor\nCommander One\nCommander Two")
              }
            />
            <span>
              {activeFormat === MATCH_FORMAT_PLANECHASE
                ? ui("Planar deck: {0}/10+", { 0: commanderCount })
                : ui("Commander(s): {0}/1-2", { 0: commanderCount })}
            </span>
          </>
        ) : null}
        <span>{readyText ? ui(readyText) : formatDeckRequirement(activeFormat)}</span>
      </div>
    </div>
  );
}
