import useUiText from "@/i18n/useUiText";
import { useMemo, useState } from "react";
import CompetitiveDeckPicker from "./CompetitiveDeckPicker";
import LobbyDeckCardList from "./LobbyDeckCardList";
import PreservingTextarea from "@/components/ui/PreservingTextarea";
import {
  LOBBY_DECK_SIZE,
  MATCH_FORMAT_COMMANDER,
  MATCH_FORMAT_NORMAL,
  MATCH_FORMAT_PLANECHASE,
  normalizeMatchFormat,
} from "@/lib/decklists";
import { commanderDeckTarget, formatDeckRequirement, formatName } from "@/lib/lobby-deck";
import { isLegalityFormat, usesCommanders } from "@/lib/relay/formats";

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

export function LobbyDeckCatalogPicker({ format, disabled = false, onChange }) {
  const matchFormat = normalizeMatchFormat(format);
  if (disabled || matchFormat === MATCH_FORMAT_PLANECHASE) return null;
  return (
    <CompetitiveDeckPicker
      format={matchFormat === MATCH_FORMAT_COMMANDER ? "commander" : matchFormat === MATCH_FORMAT_NORMAL ? "modern" : matchFormat}
      lockFormat={isLegalityFormat(matchFormat)}
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
  cardProblems = {},
  legalityText = "",
  onChange,
}) {
  const ui = useUiText();
  const activeFormat = normalizeMatchFormat(format);
  // The list view is the default; the paste box opens for an empty deck or
  // when the player asks to edit.
  const [editing, setEditing] = useState(false);
  const hasCards = Boolean(String(deckText || "").trim() || String(commanderText || "").trim());
  const showEditor = editing || !hasCards;
  const selectedDeckOptionId = useMemo(() => {
    const current = String(deckText || "");
    return deckOptions.find((option) => String(option?.deckText || "") === current)?.id || "";
  }, [deckOptions, deckText]);
  const activeCommanderTarget = commanderDeckTarget(commanderCount);
  return (
    <div className="lobby-sheet-deck lobby-sheet-panel fantasy-sheet-section grid gap-3 p-4">
      <div className="flex items-center justify-between">
        <span className="text-[10px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">{ui("Your Deck")}</span>
        <span className="flex items-center gap-3 text-[13px] text-muted-foreground">{ui("Format:") + " "}{ui(formatName(activeFormat))}
          {hasCards ? (
            <button
              type="button"
              className="lobby-deck-edit-toggle text-[11px] font-semibold uppercase tracking-[0.16em] text-[#d8bf7a] underline-offset-4 hover:underline disabled:opacity-50"
              disabled={disabled}
              aria-pressed={editing}
              onClick={() => setEditing((value) => !value)}
            >{editing ? ui("Show list") : ui("Edit list")}</button>
          ) : null}
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
              onChange({ deckText: option.deckText, commanderText: option.commanderText || "" });
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
      {showEditor ? null : (
        <LobbyDeckCardList
          deckText={deckText}
          commanderText={usesCommanders(activeFormat) ? commanderText : ""}
          format={activeFormat}
          cardProblems={cardProblems}
        />
      )}
      {showEditor ? <PreservingTextarea
        aria-label={ui("Your Deck")}
        className={`${lobbyTextareaClass} lobby-sheet-main-deck`}
        disabled={disabled}
        value={deckText}
        onChange={(event) => onChange({ deckText: event.target.value })}
        placeholder={
          ui(usesCommanders(activeFormat)
            ? `Paste your Commander main deck...\n\n1 Sol Ring\n1 Brainstorm\n33 Island`
            : `Paste a main deck with at least ${LOBBY_DECK_SIZE} cards...\n\n4 Swords to Plowshares\n4 Brainstorm\n24 Plains`)
        }
      /> : null}
      <div className={lobbyInfoTextClass}>
        <span>{ui("Main deck:")}{" "}
          {usesCommanders(activeFormat)
            ? `${deckCount}/${activeCommanderTarget}`
            : `${deckCount}/${LOBBY_DECK_SIZE}+`}
        </span>
        {usesCommanders(activeFormat)
        || activeFormat === MATCH_FORMAT_PLANECHASE ? (
          <>
            {showEditor || activeFormat === MATCH_FORMAT_PLANECHASE ? <PreservingTextarea
              className={lobbyCommanderTextareaClass}
              disabled={disabled}
              value={commanderText}
              onChange={(event) => onChange({ commanderText: event.target.value })}
              placeholder={
                ui(activeFormat === MATCH_FORMAT_PLANECHASE
                  ? "1 Plane or Phenomenon per line"
                  : "1 Commander\nor\nCommander One\nCommander Two")
              }
            /> : null}
            <span>
              {activeFormat === MATCH_FORMAT_PLANECHASE
                ? ui("Planar deck: {0}/10+", { 0: commanderCount })
                : ui("Commander(s): {0}/1-2", { 0: commanderCount })}
            </span>
          </>
        ) : null}
        {legalityText ? <span className="lobby-deck-legality is-error text-[#f0a9a0]" role="status">{legalityText}</span> : null}
        <span>{readyText ? ui(readyText) : formatDeckRequirement(activeFormat)}</span>
      </div>
    </div>
  );
}
