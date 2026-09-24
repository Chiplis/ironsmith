import useUiText from "@/i18n/useUiText";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import {
  findSavedDeckPreset,
  listSavedDeckPresets,
  parseDeckList,
  parseSideboardList,
  removeSavedDeckPreset,
  saveSavedDeckPreset,
  SAVED_DECK_PRESETS_LIMIT,
} from "@/lib/decklists";
import CompetitiveDeckBrowser from "./CompetitiveDeckBrowser";
import PreservingTextarea from "@/components/ui/PreservingTextarea";

const fieldClass =
  "w-full bg-[#050607] px-3 py-2 text-[13px] text-[#e7d9bc] outline-none transition-colors placeholder:text-[#6f6759] focus:bg-[#101114] focus-visible:ring-1 focus-visible:ring-[#d8bf7a]/35";
const selectClass = `${fieldClass} pr-12`;
const selectStyle = {
  appearance: "none",
  backgroundImage: "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 20 20' fill='none' stroke='%23b8aa8e' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.8'%3E%3Cpath d='m5 7 5 5 5-5'/%3E%3C/svg%3E\")",
  backgroundPosition: "right 1.35rem center",
  backgroundRepeat: "no-repeat",
  backgroundSize: "0.9rem",
};

// A decklist line, not interface copy: it stays in MTGO's own wording.
const MTGO_EXAMPLE_LINE = "4 Counterspell";

function ActionSpinner() {
  return <svg viewBox="0 0 20 20" className="h-3.5 w-3.5 animate-spin" aria-hidden="true"><circle cx="10" cy="10" r="7" fill="none" stroke="currentColor" strokeOpacity="0.25" strokeWidth="2" /><path d="M17 10a7 7 0 0 0-7-7" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="2" /></svg>;
}

function stripDeckHeader(text) {
  return String(text || "").replace(/^\s*Deck\s*\r?\n/i, "");
}

function samePresetTexts(left, right) {
  const leftTexts = Array.isArray(left) ? left : [];
  const rightTexts = Array.isArray(right) ? right : [];
  if (leftTexts.length !== rightTexts.length) return false;
  return leftTexts.every((text, index) => String(text || "") === String(rightTexts[index] || ""));
}

function fitTextsToPlayers(players, texts) {
  return players.map((_, index) => stripDeckHeader(texts?.[index]));
}

function editorCountForTexts(players, texts) {
  const highestFilledIndex = texts.reduce(
    (highest, text, index) => String(text || "").trim() ? index : highest,
    -1,
  );
  if (highestFilledIndex < 1) return Math.min(1, players.length);
  if (highestFilledIndex < 2) return Math.min(2, players.length);
  return Math.min(4, players.length);
}

export default function DeckLoadingView({ onOpenLobby, onTestDecks, onCancel }) {
  const ui = useUiText();
  const {
    state,
    setStatus,
  } = useGame();
  const players = useMemo(() => state?.players || [], [state?.players]);
  const [texts, setTexts] = useState(() => players.map(() => ""));
  const [deckLabels, setDeckLabels] = useState(() => players.map(() => ""));
  const [savedPresets, setSavedPresets] = useState(() => listSavedDeckPresets());
  const [selectedPresetName, setSelectedPresetName] = useState("");
  const [presetName, setPresetName] = useState("");
  const [actionBusy, setActionBusy] = useState("");
  const [showContinueChoices, setShowContinueChoices] = useState(false);
  const [showLobbyConfirm, setShowLobbyConfirm] = useState(false);
  const [actionNotice, setActionNotice] = useState("");
  const actionNoticeTimerRef = useRef(null);
  const [copiedPlayerIndex, setCopiedPlayerIndex] = useState(null);
  const [catalogTargetIndex, setCatalogTargetIndex] = useState(0);
  const [editorPlayerCount, setEditorPlayerCount] = useState(1);

  const handleTextChange = useCallback((index, value) => {
    setTexts((prev) => {
      const next = [...prev];
      next[index] = value;
      return next;
    });
  }, []);

  const cardCounts = useMemo(
    () => texts.map((t) => parseDeckList(t).length),
    [texts]
  );
  const sideboardCounts = useMemo(
    () => texts.map((t) => parseSideboardList(t).length),
    [texts]
  );
  const totalCards = cardCounts.reduce((a, b) => a + b, 0);
  const visiblePlayerCount = Math.min(editorPlayerCount, players.length || 1);
  const visiblePlayers = useMemo(() => players.slice(0, visiblePlayerCount), [players, visiblePlayerCount]);
  const playerCountModes = [1, 2, 4].filter((count) => count <= players.length);
  const targetIndex = Math.min(catalogTargetIndex, Math.max(0, visiblePlayerCount - 1));
  const targetPlayer = visiblePlayers[targetIndex] || null;
  const targetPlayerName = targetPlayer?.name || "";

  const showActionNotice = useCallback((message) => {
    setActionNotice(String(message || ""));
    if (actionNoticeTimerRef.current) window.clearTimeout(actionNoticeTimerRef.current);
    actionNoticeTimerRef.current = window.setTimeout(() => {
      setActionNotice("");
      actionNoticeTimerRef.current = null;
    }, 1800);
  }, []);

  useEffect(() => () => {
    if (actionNoticeTimerRef.current) window.clearTimeout(actionNoticeTimerRef.current);
  }, []);

  const selectedPreset = useMemo(
    () =>
      savedPresets.find(
        (preset) => preset.name === selectedPresetName
      ) || null,
    [savedPresets, selectedPresetName]
  );

  const handleApplySavedPreset = () => {
    if (!selectedPreset) return;
    const nextTexts = fitTextsToPlayers(players, selectedPreset.texts);
    const nextCount = editorCountForTexts(players, nextTexts);
    setTexts(nextTexts);
    setDeckLabels(nextTexts.map((text) => String(text || "").trim() ? selectedPreset.name : ""));
    setEditorPlayerCount(nextCount);
    const nextEmptyIndex = nextTexts.findIndex((text) => !String(text || "").trim());
    setCatalogTargetIndex(nextEmptyIndex >= 0 ? Math.min(nextEmptyIndex, Math.max(0, nextCount - 1)) : 0);
    showActionNotice(ui("Deck loaded into {0}", { 0: players[nextEmptyIndex >= 0 ? nextEmptyIndex : 0]?.name || ui("the editor") }));
  };

  const saveCurrentPreset = useCallback((requestedName) => {
    const normalizedPresetName = String(requestedName || "").trim();
    if (!normalizedPresetName) {
      setStatus(ui("Choose a name to save this deck."));
      return false;
    }

    const existingPreset = findSavedDeckPreset(normalizedPresetName);
    const nextTexts = fitTextsToPlayers(players, texts);
    const shouldConfirmOverride =
      existingPreset && !samePresetTexts(existingPreset.texts, nextTexts);
    if (
      shouldConfirmOverride
      && !window.confirm(ui('A saved deck named "{0}" already exists. Override it?', { 0: existingPreset.name }))
    ) {
      return false;
    }

    const saveResult = saveSavedDeckPreset(normalizedPresetName, nextTexts, players.map((player) => player.name));
    if (saveResult.saved) {
      setSavedPresets(saveResult.entries);
      setSelectedPresetName(saveResult.entry.name);
      setStatus(
        saveResult.replaced
          ? ui('Updated saved deck "{0}"', { 0: saveResult.entry.name })
          : ui('Saved deck "{0}"', { 0: saveResult.entry.name })
      );
      showActionNotice(saveResult.replaced ? ui("Saved deck updated") : ui("Deck saved"));
      return true;
    }
    if (saveResult.reason === "limit") {
      setStatus(ui("Session limit reached ({0} decks). Delete one saved deck to add another.", { 0: SAVED_DECK_PRESETS_LIMIT }));
    }
    return false;
  }, [players, setStatus, showActionNotice, texts, ui]);

  const handleSavePreset = useCallback(() => {
    if (saveCurrentPreset(presetName)) setPresetName("");
  }, [presetName, saveCurrentPreset]);

  const handleClearPlayer = useCallback((playerIndex) => {
    setTexts((current) => {
      const next = [...current];
      next[playerIndex] = "";
      return next;
    });
    setDeckLabels((current) => {
      const next = [...current];
      next[playerIndex] = "";
      return next;
    });
    setCopiedPlayerIndex((current) => current === playerIndex ? null : current);
    setCatalogTargetIndex(playerIndex);
    showActionNotice(ui("Removed {0}'s deck from the editor", { 0: players[playerIndex]?.name || ui("player") }));
  }, [players, showActionNotice, ui]);

  const handleEditorPlayerCountChange = useCallback((count) => {
    setEditorPlayerCount(count);
    setCatalogTargetIndex((current) => Math.min(current, Math.max(0, count - 1)));
  }, []);

  const handleCopyMtgo = useCallback(async (playerIndex = catalogTargetIndex) => {
    const text = String(texts[playerIndex] || "").trim();
    if (!text) {
      setStatus(ui("There is no deck to copy."));
      return;
    }

    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else {
        const textarea = document.createElement("textarea");
        textarea.value = text;
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        textarea.remove();
      }
      setCopiedPlayerIndex(playerIndex);
      window.setTimeout(() => setCopiedPlayerIndex((current) => current === playerIndex ? null : current), 900);
      showActionNotice(ui("MTGO list copied"));
    } catch {
      setStatus(ui("Could not copy the deck."));
    }
  }, [catalogTargetIndex, setStatus, showActionNotice, texts, ui]);

  const runAction = useCallback((key, action) => {
    if (actionBusy) return;
    setActionBusy(key);
    let result;
    try {
      result = action();
    } catch (error) {
      setStatus(error?.message || ui("Could not complete the action."));
      setActionBusy("");
      return;
    }
    Promise.resolve(result)
      .catch((error) => setStatus(error?.message || ui("Could not complete the action.")))
      .finally(() => {
        window.setTimeout(() => setActionBusy((current) => current === key ? "" : current), 180);
      });
  }, [actionBusy, setStatus, ui]);

  const handleCatalogSelect = useCallback(({ deckText, deckName, name, archetype }) => {
    const target = visiblePlayers.length ? Math.min(catalogTargetIndex, visiblePlayers.length - 1) : 0;
    const importedText = stripDeckHeader(deckText);
    const importedName = String(deckName || name || archetype || "").trim();
    const nextTexts = [...texts];
    nextTexts[target] = importedText;
    handleTextChange(target, importedText);
    if (importedName) {
      setDeckLabels((current) => {
        const next = [...current];
        next[target] = importedName;
        return next;
      });
    }

    const findEmptyPlayer = (count) => players.slice(0, count).findIndex((_, index) => !String(nextTexts[index] || "").trim());
    let nextCount = visiblePlayerCount;
    let nextIndex = findEmptyPlayer(nextCount);
    if (nextIndex < 0 && nextCount < players.length) {
      nextCount = [1, 2, 4].find((count) => count > nextCount && count <= players.length) || players.length;
      nextIndex = findEmptyPlayer(nextCount);
    }
    setEditorPlayerCount(nextCount);
    setCatalogTargetIndex(nextIndex >= 0 ? nextIndex : (target + 1) % Math.max(1, nextCount));
    showActionNotice(ui("Deck loaded into {0}", { 0: players[target]?.name || ui("the editor") }));
  }, [catalogTargetIndex, handleTextChange, players, showActionNotice, texts, ui, visiblePlayerCount, visiblePlayers.length]);

  const handleDeleteSavedPreset = useCallback(() => {
    if (!selectedPreset) return;
    if (!window.confirm(ui('Delete saved deck "{0}"?', { 0: selectedPreset.name }))) return;
    setSavedPresets(removeSavedDeckPreset(selectedPreset.name));
    setSelectedPresetName("");
    setStatus(ui('Deleted saved deck "{0}"', { 0: selectedPreset.name }));
    showActionNotice(ui("Saved deck deleted"));
  }, [selectedPreset, setStatus, showActionNotice, ui]);

  const handleTestInGame = useCallback(() => {
    const decks = texts.map(parseDeckList);
    const sideboards = texts.map(parseSideboardList);
    if (!decks.some((deck) => deck.length > 0)) {
      setStatus(ui("Paste at least one deck to test it in a game."));
      return false;
    }
    // The x1/x2/x4 selector is the source of truth for the match size. Empty
    // slots are intentional: the engine can preserve the bot/demo deck for
    // those players, while slicing by filled decks silently dropped them.
    const playerCount = Math.max(2, Math.min(4, visiblePlayerCount));
    const perspectivePlayerIndex = Math.max(0, decks.findIndex((deck) => deck.length > 0));
    const playerDecks = decks.slice(0, playerCount);
    const playerSideboards = sideboards.slice(0, playerCount);
    const seedPlayerIndices = playerDecks.reduce(
      (indices, deck, index) => (deck.length > 0 ? [...indices, index] : indices),
      [],
    );
    return onTestDecks?.({
      decks: playerDecks,
      sideboards: playerSideboards,
      playerCount,
      perspectivePlayerIndex,
      seedPlayerIndices,
      preserveMissingDecks: true,
      allowPartialDecks: true,
      seedTestPosition: true,
    });
  }, [onTestDecks, setStatus, texts, ui, visiblePlayerCount]);

  const lobbyPlayerCount = Math.max(2, Math.min(4, visiblePlayerCount));
  const lobbyDeckOptions = useMemo(
    () => texts
      .map((text, index) => ({
        id: `editor-${index}`,
        label: `${deckLabels[index] || ui("Deck {0}", { 0: index + 1 })} (${players[index]?.name || ui("Player {0}", { 0: index + 1 })})`,
        deckText: String(text || ""),
      }))
      .filter((option) => option.deckText.trim()),
    [deckLabels, players, texts, ui],
  );
  const handleConfirmLobby = useCallback(() => {
    setShowLobbyConfirm(false);
    onOpenLobby?.(texts, lobbyPlayerCount, lobbyDeckOptions);
  }, [lobbyDeckOptions, lobbyPlayerCount, onOpenLobby, texts]);

  return (
    <main
      className="setup-screen deck-loading-screen relative flex h-full min-h-0 flex-col overflow-y-auto bg-[#08090a] p-3 pb-24 lg:overflow-hidden lg:pb-3"
    >
      {actionNotice ? (
        <div className="pointer-events-none sticky top-0 z-30 flex justify-end" role="status" aria-live="polite">
          <div className="bg-[#2e2416] px-3 py-1.5 text-[11px] font-bold uppercase tracking-wide text-[#f2d9a3] shadow-lg">
            {actionNotice}
          </div>
        </div>
      ) : null}
      <div className="mb-3 flex shrink-0 flex-wrap items-start justify-between gap-3 pb-3">
        <div className="min-w-0">
          <h1 className="text-[18px] font-bold uppercase tracking-wide text-[#f2d9a3]">{ui("Load Decks")}</h1>
          <div className="mt-1 text-[12px] font-semibold text-[#b8aa8e]">{ui("Select decks from the catalog or paste lists to assign them to players.")}</div>
        </div>
        {/* The match is still built in one screen; the steps only say where
            the player is in that flow. */}
        <ol className="flex flex-wrap items-center gap-2 text-[10px] font-bold uppercase tracking-[0.16em]" aria-label={ui("Steps")}>
          {[ui("Load Decks"), ui("Configure"), ui("Start")].map((step, index) => (
            <li key={step} className="flex items-center gap-2">
              {index ? <span className="text-[#5b5245]" aria-hidden="true">›</span> : null}
              <span
                className={index === 0 ? "rounded-sm bg-[#2e2416] px-1.5 py-0.5 text-[#f2d9a3]" : "text-[#5b5449]"}
                aria-current={index === 0 ? "step" : undefined}
              >{index + 1}. {step}</span>
            </li>
          ))}
        </ol>
      </div>

      <section
        className="mb-2 flex min-h-0 flex-1 flex-col gap-3 lg:flex-row"
        aria-label={ui("Deck catalog and player decks")}
        data-deck-workspace=""
      >
        <div className="flex min-h-[360px] min-w-0 flex-col bg-[#0e0f11] p-2.5 lg:min-h-0 lg:w-[70%] lg:shrink-0">
          <CompetitiveDeckBrowser
            onSelect={handleCatalogSelect}
            targetName={targetPlayerName}
            savedDecks={savedPresets}
          />
        </div>

        <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 bg-[#0e0f11] p-2.5">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="min-w-0">
              <h2 className="text-[12px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">{ui("Players / deck assignment")}</h2>
              <p className="max-w-[46ch] text-[11px] text-[#8b806b]">{ui("Pick a player and assign their deck. You can paste a list, use a catalog deck, or import from MTGO.")}</p>
            </div>
            <div className="flex shrink-0 flex-wrap gap-1" aria-label={ui("Players")} data-player-tabs="">
              {visiblePlayers.map((player, index) => {
                const isTarget = index === targetIndex;
                return (
                  <button
                    key={player.id}
                    type="button"
                    className={`flex items-center gap-1.5 rounded-sm px-2.5 py-1.5 text-[11px] font-bold uppercase tracking-wide transition-colors ${isTarget
                      ? "bg-[#2e2416] text-[#f2d9a3]"
                      : "bg-[#131418] text-[#6f6759] hover:bg-[#1d1e22] hover:text-[#e7d9bc]"}`}
                    aria-pressed={isTarget}
                    data-player-tab={index}
                    onClick={() => setCatalogTargetIndex(index)}
                  >
                    <svg viewBox="0 0 20 20" className="h-3.5 w-3.5" aria-hidden="true"><circle cx="10" cy="7" r="3.2" fill="none" stroke="currentColor" strokeWidth="1.5" /><path d="M4.5 16.5a5.5 5.5 0 0 1 11 0" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.5" /></svg>
                    <span className="max-w-[9ch] truncate">{player.name}</span>
                    {cardCounts[index] > 0 ? <span className="text-[#d8bf7a]" aria-hidden="true">•</span> : null}
                  </button>
                );
              })}
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-1.5" aria-label={ui("Number of players to edit")}>
            <span className="text-[10px] uppercase tracking-wide text-[#8b806b]">{ui("Players")}</span>
            {playerCountModes.map((count) => (
              <button
                key={count}
                type="button"
                className={`rounded-full px-2.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide transition-colors ${visiblePlayerCount === count ? "bg-[#3d2f1c] text-[#f2d9a3]" : "bg-[#131418] text-[#6f6759] hover:bg-[#1d1e22] hover:text-[#e7d9bc]"}`}
                aria-pressed={visiblePlayerCount === count}
                onClick={() => handleEditorPlayerCountChange(count)}
              >
                x{count}
              </button>
            ))}
          </div>

          {targetPlayer ? (
            <div className="flex min-h-0 flex-1 flex-col gap-3 bg-[#131418] p-3" data-player-panel={targetIndex}>
              <div className="flex items-start justify-between gap-2">
                <div className="flex min-w-0 items-center gap-2.5">
                  <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-[#211a10] text-[16px] font-bold text-[#d8bf7a]" aria-hidden="true">
                    {targetPlayer.name.slice(0, 1).toLocaleUpperCase("en-US")}
                  </span>
                  <div className="min-w-0">
                    <div className="flex min-w-0 items-baseline gap-2">
                      <span className="min-w-0 truncate text-[15px] font-bold uppercase tracking-wide text-[#f2d9a3]">{targetPlayer.name}</span>
                      <span className="shrink-0 rounded-full bg-[#3d2f1c] px-1.5 text-[9px] font-bold uppercase tracking-wide text-[#f2d9a3]">{ui("Target")}</span>
                    </div>
                    <div className="truncate text-[11px] text-[#b8aa8e]">{deckLabels[targetIndex] || ui("No deck assigned")}</div>
                    <div className="text-[11px] font-semibold text-[#8b806b]">
                      <span className="text-[#e7d9bc]">{cardCounts[targetIndex]}</span> {ui("main")}
                      <span className="mx-1 text-[#776b58]">/</span>
                      <span className="text-[#e7d9bc]">{sideboardCounts[targetIndex]}</span> {ui("sideboard")}
                    </div>
                  </div>
                </div>
                <div className="flex shrink-0 gap-1.5">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="h-7 rounded-sm px-2 text-[10px] font-bold uppercase tracking-wide flat-button"
                    disabled={cardCounts[targetIndex] === 0 || Boolean(actionBusy)}
                    onClick={() => runAction(`copy-${targetIndex}`, () => handleCopyMtgo(targetIndex))}
                    title={ui("Copy {0}'s MTGO list", { 0: targetPlayer.name })}
                    aria-label={ui("Copy {0}'s MTGO list", { 0: targetPlayer.name })}
                  >
                    <svg viewBox="0 0 20 20" className="mr-1 h-3.5 w-3.5" aria-hidden="true"><rect x="6.5" y="6.5" width="9" height="10" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.4" /><path d="M13 6.5V4.8A1.3 1.3 0 0 0 11.7 3.5H5A1.5 1.5 0 0 0 3.5 5v8A1.3 1.3 0 0 0 4.8 14.3h1.7" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.4" /></svg>
                    {actionBusy === `copy-${targetIndex}` ? <ActionSpinner /> : copiedPlayerIndex === targetIndex ? ui("Copied") : ui("Copy")}
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="h-7 rounded-sm px-2 text-[10px] font-bold uppercase tracking-wide flat-button"
                    disabled={cardCounts[targetIndex] === 0 || Boolean(actionBusy)}
                    onClick={() => handleClearPlayer(targetIndex)}
                    title={ui("Clear {0}'s deck", { 0: targetPlayer.name })}
                    aria-label={ui("Clear {0}'s deck", { 0: targetPlayer.name })}
                  >
                    <svg viewBox="0 0 20 20" className="mr-1 h-3.5 w-3.5" aria-hidden="true"><path d="M5 6.5h10M8 6.5V5h4v1.5M6.5 6.5 7 16h6l.5-9.5" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.4" /></svg>
                    {ui("Clear")}
                  </Button>
                </div>
              </div>

              <PreservingTextarea
                aria-label={ui("{0} decklist", { 0: targetPlayer.name })}
                spellCheck={false}
                className="min-h-[160px] w-full flex-1 resize-none bg-[#050607] p-2.5 font-mono text-[13px] leading-snug text-[#e7d9bc] outline-none transition-colors placeholder:text-[#6f6759] focus:bg-[#0a0b0d] focus-visible:ring-1 focus-visible:ring-[#d8bf7a]/35"
                placeholder={stripDeckHeader(ui("Paste {0}'s list...\n\nDeck\n4 Lightning Bolt\n2 Counterspell\n20 Island\n\nSideboard\n2 Pyroblast\n1 Tormod's Crypt", { 0: targetPlayer.name }))}
                value={texts[targetIndex] || ""}
                onChange={(event) => handleTextChange(targetIndex, event.target.value)}
              />
            </div>
          ) : null}

          {/* The column is narrow now, so this panel stacks instead of trying
              to sit three across. */}
          <div className="grid shrink-0 gap-2 bg-[#131418] p-3">
            <div className="min-w-0">
              <h3 className="text-[11px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">{ui("Save configuration")}</h3>
              <p className="text-[11px] text-[#8b806b]">{ui("Save every player's deck as one configuration.")}</p>
            </div>
            <div className="flex min-w-0 items-center gap-2">
            <input
              className={`${fieldClass} min-w-0 flex-1 py-1.5 text-[11px]`}
              placeholder={ui("Configuration name…")}
              value={presetName}
              onChange={(event) => setPresetName(event.target.value)}
              aria-label={ui("Name your deck")}
            />
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-9 shrink-0 max-w-[120px] truncate rounded-sm px-3 text-[10px] font-bold uppercase tracking-wide flat-button-gold"
              disabled={!presetName.trim() || totalCards === 0 || Boolean(actionBusy)}
              onClick={() => runAction("saved-save", handleSavePreset)}
            >{actionBusy === "saved-save" ? <ActionSpinner /> : ui("Save")}</Button>
            </div>
          </div>
        </div>
      </section>

      <div className="flex shrink-0 flex-wrap items-center justify-end gap-2 pb-4 pr-40 pt-2 lg:pb-0">
        {savedPresets.length ? (
          <div className="mr-auto flex min-w-0 items-center gap-2">
            <select
              className={`${selectClass} max-w-[240px] py-1.5 text-[11px]`}
              style={selectStyle}
              value={selectedPresetName}
              onChange={(event) => setSelectedPresetName(event.target.value)}
              aria-label={ui("Saved Deck")}
            >
              <option value="">{ui("Select a saved deck")}</option>
              {savedPresets.map((preset) => (
                <option key={preset.name} value={preset.name}>{preset.name}</option>
              ))}
            </select>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 max-w-[96px] truncate rounded-sm px-3 text-[10px] font-bold uppercase tracking-wide flat-button-gold"
              disabled={!selectedPreset || Boolean(actionBusy)}
              onClick={() => runAction("saved-use", handleApplySavedPreset)}
            >{actionBusy === "saved-use" ? <ActionSpinner /> : ui("Use")}</Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 max-w-[96px] truncate rounded-sm px-3 text-[10px] font-bold uppercase tracking-wide flat-button"
              disabled={!selectedPreset || Boolean(actionBusy)}
              onClick={() => runAction("saved-delete", handleDeleteSavedPreset)}
            >{actionBusy === "saved-delete" ? <ActionSpinner /> : ui("Delete")}</Button>
          </div>
        ) : null}
        {showLobbyConfirm ? (
          <div className="flex flex-wrap items-center justify-end gap-2 rounded-sm bg-[#211a10] px-2 py-1.5">
            <span className="mr-1 text-[10px] font-semibold uppercase tracking-wide text-[#f2d9a3]">{ui("Create a {0}-player lobby?", { 0: lobbyPlayerCount })}</span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 rounded-sm px-3 text-[10px] font-bold uppercase tracking-wide flat-button-primary"
              disabled={Boolean(actionBusy)}
              onClick={handleConfirmLobby}
            >{ui("Confirm")}</Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 rounded-sm px-2 text-[10px] font-bold uppercase tracking-wide flat-button"
              disabled={Boolean(actionBusy)}
              onClick={() => setShowLobbyConfirm(false)}
            >{ui("Cancel")}</Button>
          </div>
        ) : showContinueChoices ? (
          <div className="flex flex-wrap items-center justify-end gap-2">
            <span className="mr-1 text-[10px] font-semibold uppercase tracking-wide text-[#b8aa8e]">{ui("Continue with these decks")}</span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-9 rounded-sm px-3 text-[11px] font-bold uppercase tracking-wide flat-button-primary"
              disabled={Boolean(actionBusy)}
              onClick={() => setShowLobbyConfirm(true)}
            >{ui("Lobby and share")}</Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-9 rounded-sm px-3 text-[11px] font-bold uppercase tracking-wide flat-button-gold"
              disabled={Boolean(actionBusy)}
              onClick={() => runAction("test", handleTestInGame)}
            >{actionBusy === "test" ? <ActionSpinner /> : ui("Test in game")}</Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-9 rounded-sm px-2 text-[11px] font-bold uppercase tracking-wide flat-button"
              disabled={Boolean(actionBusy)}
              onClick={() => setShowContinueChoices(false)}
            >{ui("Back")}</Button>
          </div>
        ) : (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-10 rounded-sm px-5 text-[12px] font-bold uppercase tracking-wide flat-button-primary"
            disabled={Boolean(actionBusy)}
            onClick={() => setShowContinueChoices(true)}
          >{ui("Build lobby")}</Button>
        )}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-10 rounded-sm px-4 text-[12px] font-bold uppercase tracking-wide flat-button-gold"
          disabled={Boolean(actionBusy)}
          onClick={onCancel}
        >{ui("Cancel")}</Button>
      </div>
    </main>
  );
}
