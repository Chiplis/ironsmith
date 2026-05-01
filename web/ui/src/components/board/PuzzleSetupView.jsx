import { useEffect, useMemo, useState } from "react";

import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { copyTextToClipboard } from "@/lib/clipboard";
import { ArrowLeft, Clipboard, Download, Eraser, Link2, Play, UserMinus, UserPlus, Users } from "lucide-react";
import {
  PUZZLE_ZONE_ORDER,
  buildPuzzlePayload,
  buildPuzzlePayloadFromGameState,
  buildPuzzleUrl,
  buildPuzzleZoneTextsFromPayload,
  clearSavedPuzzleDraft,
  createEmptyPuzzleZoneTexts,
  createPuzzlePlayers,
  fitPuzzleZoneTextsToPlayers,
  loadSavedPuzzleDraft,
  parsePuzzleCardList,
  saveSavedPuzzleDraft,
} from "@/lib/puzzles";

const fieldClass =
  "puzzle-setup-field";
const zoneLabelClass =
  "puzzle-setup-label";
const MAX_PLAYERS = 8;

function zoneTitle(zone) {
  switch (zone) {
    case "battlefield": return "Battlefield";
    case "hand": return "Hand";
    case "graveyard": return "Graveyard";
    case "exile": return "Exile";
    case "library": return "Library";
    case "command": return "Command";
    default: return zone;
  }
}

function puzzleDraftFromGameState(state) {
  const payload = buildPuzzlePayloadFromGameState(state);
  if (!payload) {
    const players = createPuzzlePlayers(2);
    return {
      players,
      zoneTexts: createEmptyPuzzleZoneTexts(players),
    };
  }

  const players = payload.players.map((player, index) => ({
    id: `puzzle-player-${index + 1}`,
    name: player.name,
    life: player.life,
  }));

  return {
    players,
    zoneTexts: fitPuzzleZoneTextsToPlayers(players, buildPuzzleZoneTextsFromPayload(payload)),
  };
}

export default function PuzzleSetupView({ onLoadPuzzle, onCancel }) {
  const { state, setStatus } = useGame();
  const initialDraft = useMemo(
    () => {
      const savedDraft = loadSavedPuzzleDraft();
      if (savedDraft) return { ...savedDraft, restored: true };
      return { ...puzzleDraftFromGameState(state), restored: false };
    },
    [state]
  );
  const [players, setPlayers] = useState(initialDraft.players);
  const [zoneTexts, setZoneTexts] = useState(initialDraft.zoneTexts);

  useEffect(() => {
    if (!initialDraft.restored) return;
    setStatus("Restored saved puzzle draft");
  }, [initialDraft.restored, setStatus]);

  useEffect(() => {
    saveSavedPuzzleDraft(players, zoneTexts);
  }, [players, zoneTexts]);

  const payload = useMemo(() => buildPuzzlePayload(players, zoneTexts), [players, zoneTexts]);
  const shareUrl = useMemo(() => buildPuzzleUrl(payload), [payload]);
  const totalCards = useMemo(
    () => payload.players.reduce(
      (count, player) => count + PUZZLE_ZONE_ORDER.reduce(
        (zoneCount, zone) => zoneCount + (player.zones?.[zone]?.length || 0),
        0
      ),
      0
    ),
    [payload]
  );

  const updateZoneText = (playerIndex, zone, value) => {
    setZoneTexts((current) => current.map((entry, index) => (
      index === playerIndex ? { ...entry, [zone]: value } : entry
    )));
  };

  const updatePlayerName = (playerIndex, value) => {
    setPlayers((current) => current.map((player, index) => (
      index === playerIndex ? { ...player, name: value } : player
    )));
  };

  const updatePlayerLife = (playerIndex, value) => {
    setPlayers((current) => current.map((player, index) => (
      index === playerIndex
        ? { ...player, life: Number(value) || 0 }
        : player
    )));
  };

  const adjustPlayerCount = (nextCount) => {
    const boundedCount = Math.max(1, Math.min(MAX_PLAYERS, Number(nextCount) || 1));
    setPlayers((current) => {
      if (current.length === boundedCount) return current;
      const nextPlayers = current.slice(0, boundedCount);
      for (let index = nextPlayers.length; index < boundedCount; index += 1) {
        nextPlayers.push({ id: `puzzle-player-${index + 1}`, name: `Player ${index + 1}`, life: 20 });
      }
      return nextPlayers;
    });
    setZoneTexts((current) => fitPuzzleZoneTextsToPlayers(createPuzzlePlayers(boundedCount), current));
  };

  const handleImportCurrentTable = () => {
    const imported = puzzleDraftFromGameState(state);
    setPlayers(imported.players);
    setZoneTexts(imported.zoneTexts);
    setStatus("Imported visible cards from the current table");
  };

  const handleClearDraft = () => {
    const cleared = puzzleDraftFromGameState(state);
    clearSavedPuzzleDraft();
    setPlayers(cleared.players);
    setZoneTexts(cleared.zoneTexts);
    setStatus("Cleared saved puzzle draft");
  };

  const handleCopyLink = async () => {
    if (!shareUrl) {
      setStatus("Could not generate a puzzle link", true);
      return;
    }

    const copied = await copyTextToClipboard(shareUrl);
    setStatus(copied ? "Copied puzzle link" : "Could not copy puzzle link", !copied);
  };

  const handleLoadHere = async () => {
    if (typeof onLoadPuzzle !== "function") return;
    await onLoadPuzzle(payload, "Puzzle loaded");
  };

  return (
    <main
      className="puzzle-setup-shell table-gradient"
    >
      <section className="puzzle-setup-hero">
        <div className="puzzle-setup-copy">
          <div className="grid gap-2">
            <div className="puzzle-setup-kicker">
              Puzzle Setup
            </div>
            <h1 className="puzzle-setup-title">
              Share A Board Position
            </h1>
            <p className="puzzle-setup-description">
              Fill any zone for each player, then copy the generated `?puzzle=` link. Loading that
              link resets the table and places the listed cards directly
              into those zones without triggering ETBs.
            </p>
            <p className="puzzle-setup-note">
              Importing from the current table includes visible zones only. Libraries and hidden
              opponent hands stay blank unless you type them in here.
            </p>
          </div>

          <div className="puzzle-setup-control-strip">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="secondary" className="stone-pill px-3 uppercase">
                <Users className="size-3.5" />
                {players.length} player{players.length === 1 ? "" : "s"}
              </Badge>
              <Badge variant="secondary" className="stone-pill px-3 uppercase">
                {totalCards} card{totalCards === 1 ? "" : "s"}
              </Badge>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Button
                type="button"
                variant="secondary"
                className="stone-pill"
                disabled={players.length <= 1}
                onClick={() => adjustPlayerCount(players.length - 1)}
              >
                <UserMinus className="size-4" />
                Remove Player
              </Button>
              <Button
                type="button"
                variant="secondary"
                className="stone-pill"
                disabled={players.length >= MAX_PLAYERS}
                onClick={() => adjustPlayerCount(players.length + 1)}
              >
                <UserPlus className="size-4" />
                Add Player
              </Button>
              <Button type="button" variant="secondary" className="stone-pill" onClick={handleImportCurrentTable}>
                <Download className="size-4" />
                Import Current Table
              </Button>
              <Button type="button" variant="secondary" className="stone-pill" onClick={handleClearDraft}>
                <Eraser className="size-4" />
                Clear Draft
              </Button>
            </div>
          </div>
        </div>

        <aside className="puzzle-share-panel">
          <label className={zoneLabelClass}>
            <span className="inline-flex items-center gap-2">
              <Link2 className="size-3.5" />
              Share Link
            </span>
            <textarea
              className="puzzle-setup-field puzzle-share-link"
              readOnly
              value={shareUrl}
            />
          </label>
          <div className="grid gap-2 sm:grid-cols-2">
            <Button type="button" variant="secondary" className="stone-pill" onClick={handleCopyLink}>
              <Clipboard className="size-4" />
              Copy Link
            </Button>
            <Button type="button" variant="secondary" className="stone-pill" onClick={handleLoadHere}>
              <Play className="size-4" />
              Load Here
            </Button>
          </div>
        </aside>
      </section>

      <div
        className="puzzle-player-grid"
        style={{ gridTemplateColumns: `repeat(${Math.max(players.length, 1)}, minmax(620px, 1fr))` }}
      >
        {players.map((player, playerIndex) => {
          const playerPayload = payload.players[playerIndex];
          return (
            <section
              key={player.id}
              className="puzzle-player-panel"
            >
              <div className="puzzle-player-header">
                <label className={zoneLabelClass}>
                  Player Name
                  <input
                    className={fieldClass}
                    value={player.name}
                    onChange={(event) => updatePlayerName(playerIndex, event.target.value)}
                    placeholder={`Player ${playerIndex + 1}`}
                  />
                </label>
                <label className={zoneLabelClass}>
                  Life
                  <input
                    className={fieldClass}
                    type="number"
                    value={player.life}
                    onChange={(event) => updatePlayerLife(playerIndex, event.target.value)}
                  />
                </label>
                <div className="puzzle-player-summary">
                  Life {playerPayload?.life ?? 20} - {" "}
                  {PUZZLE_ZONE_ORDER.reduce(
                    (count, zone) => count + (playerPayload?.zones?.[zone]?.length || 0),
                    0
                  )} cards encoded
                </div>
              </div>

              <div className="puzzle-zone-grid">
                {PUZZLE_ZONE_ORDER.map((zone) => (
                  <label key={`${player.id}:${zone}`} className="puzzle-zone-editor">
                    <span className="puzzle-zone-header">
                      <span>{zoneTitle(zone)}</span>
                      <span className="puzzle-zone-count">
                        {parsePuzzleCardList(zoneTexts[playerIndex]?.[zone]).length}
                      </span>
                    </span>
                    <textarea
                      className={`${fieldClass} puzzle-zone-textarea`}
                      placeholder={`1 ${player.name || `Player ${playerIndex + 1}`} card per line`}
                      value={zoneTexts[playerIndex]?.[zone] || ""}
                      onChange={(event) => updateZoneText(playerIndex, zone, event.target.value)}
                    />
                  </label>
                ))}
              </div>
            </section>
          );
        })}
      </div>

      <footer className="puzzle-setup-footer">
        <Button type="button" variant="secondary" className="stone-pill" onClick={onCancel}>
          <ArrowLeft className="size-4" />
          Back To Table
        </Button>
      </footer>
    </main>
  );
}
