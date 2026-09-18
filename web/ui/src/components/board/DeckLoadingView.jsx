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

const fieldClass =
  "w-full border border-[rgba(154,126,82,0.46)] bg-[#0b0d0e] px-3 py-2 text-[13px] text-[#e7d9bc] outline-none transition-colors placeholder:text-[#8b806b] focus:border-[#d8bf7a]/75";
const selectClass = `${fieldClass} pr-12`;
const selectStyle = {
  appearance: "none",
  backgroundImage: "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 20 20' fill='none' stroke='%23b8aa8e' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.8'%3E%3Cpath d='m5 7 5 5 5-5'/%3E%3C/svg%3E\")",
  backgroundPosition: "right 1.35rem center",
  backgroundRepeat: "no-repeat",
  backgroundSize: "0.9rem",
};

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
    setEditorPlayerCount(nextCount);
    const nextEmptyIndex = nextTexts.findIndex((text) => !String(text || "").trim());
    setCatalogTargetIndex(nextEmptyIndex >= 0 ? Math.min(nextEmptyIndex, Math.max(0, nextCount - 1)) : 0);
    showActionNotice(`Mazo cargado en ${players[nextEmptyIndex >= 0 ? nextEmptyIndex : 0]?.name || "el editor"}`);
  };

  const saveCurrentPreset = useCallback((requestedName) => {
    const normalizedPresetName = String(requestedName || "").trim();
    if (!normalizedPresetName) {
      setStatus("Elegí un nombre para guardar este mazo.");
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
          ? `Updated saved deck "${saveResult.entry.name}"`
          : `Saved deck "${saveResult.entry.name}"`
      );
      showActionNotice(saveResult.replaced ? "Mazo guardado actualizado" : "Mazo guardado correctamente");
      return true;
    }
    if (saveResult.reason === "limit") {
      setStatus(`Session limit reached (${SAVED_DECK_PRESETS_LIMIT} decks). Delete one saved deck to add another.`);
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
    setCopiedPlayerIndex((current) => current === playerIndex ? null : current);
    setCatalogTargetIndex(playerIndex);
    showActionNotice(`Mazo de ${players[playerIndex]?.name || "jugador"} eliminado del editor`);
  }, [players, showActionNotice]);

  const handleEditorPlayerCountChange = useCallback((count) => {
    setEditorPlayerCount(count);
    setCatalogTargetIndex((current) => Math.min(current, Math.max(0, count - 1)));
  }, []);

  const handleCopyMtgo = useCallback(async (playerIndex = catalogTargetIndex) => {
    const text = String(texts[playerIndex] || "").trim();
    if (!text) {
      setStatus("No hay un deck para copiar.");
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
      showActionNotice("MTGO copiado");
    } catch {
      setStatus("No se pudo copiar el deck.");
    }
  }, [catalogTargetIndex, setStatus, showActionNotice, texts]);

  const runAction = useCallback((key, action) => {
    if (actionBusy) return;
    setActionBusy(key);
    let result;
    try {
      result = action();
    } catch (error) {
      setStatus(error?.message || "No se pudo completar la acción.");
      setActionBusy("");
      return;
    }
    Promise.resolve(result)
      .catch((error) => setStatus(error?.message || "No se pudo completar la acción."))
      .finally(() => {
        window.setTimeout(() => setActionBusy((current) => current === key ? "" : current), 180);
      });
  }, [actionBusy, setStatus]);

  const handleCatalogSelect = useCallback(({ deckText }) => {
    const target = visiblePlayers.length ? Math.min(catalogTargetIndex, visiblePlayers.length - 1) : 0;
    const importedText = stripDeckHeader(deckText);
    const nextTexts = [...texts];
    nextTexts[target] = importedText;
    handleTextChange(target, importedText);

    const findEmptyPlayer = (count) => players.slice(0, count).findIndex((_, index) => !String(nextTexts[index] || "").trim());
    let nextCount = visiblePlayerCount;
    let nextIndex = findEmptyPlayer(nextCount);
    if (nextIndex < 0 && nextCount < players.length) {
      nextCount = [1, 2, 4].find((count) => count > nextCount && count <= players.length) || players.length;
      nextIndex = findEmptyPlayer(nextCount);
    }
    setEditorPlayerCount(nextCount);
    setCatalogTargetIndex(nextIndex >= 0 ? nextIndex : (target + 1) % Math.max(1, nextCount));
    showActionNotice(`Mazo cargado en ${players[target]?.name || "el editor"}`);
  }, [catalogTargetIndex, handleTextChange, players, showActionNotice, texts, visiblePlayerCount, visiblePlayers.length]);

  const handleDeleteSavedPreset = useCallback(() => {
    if (!selectedPreset) return;
    if (!window.confirm(ui('Delete saved deck "{0}"?', { 0: selectedPreset.name }))) return;
    setSavedPresets(removeSavedDeckPreset(selectedPreset.name));
    setSelectedPresetName("");
    setStatus(`Deleted saved deck "${selectedPreset.name}"`);
    showActionNotice("Mazo guardado eliminado");
  }, [selectedPreset, setStatus, showActionNotice, ui]);

  const handleTestInGame = useCallback(() => {
    const decks = texts.map(parseDeckList);
    const sideboards = texts.map(parseSideboardList);
    if (!decks.some((deck) => deck.length > 0)) {
      setStatus("Pegá al menos un mazo para probarlo en la partida.");
      return false;
    }
    const filledCount = decks.filter((deck) => deck.length > 0).length;
    const playerCount = Math.max(2, Math.min(4, filledCount));
    const perspectivePlayerIndex = Math.max(0, decks.findIndex((deck) => deck.length > 0));
    return onTestDecks?.({
      decks: decks.slice(0, playerCount),
      sideboards: sideboards.slice(0, playerCount),
      playerCount,
      perspectivePlayerIndex,
      preserveMissingDecks: true,
      seedTestPosition: true,
    });
  }, [onTestDecks, setStatus, texts]);

  const filledDeckCount = texts.filter((text) => String(text || "").trim()).length;
  const lobbyPlayerCount = Math.max(2, Math.min(4, filledDeckCount || 2));
  const handleConfirmLobby = useCallback(() => {
    setShowLobbyConfirm(false);
    onOpenLobby?.(texts);
  }, [onOpenLobby, texts]);

  return (
    <main
      className="setup-screen deck-loading-screen table-gradient relative flex h-full min-h-0 flex-col overflow-y-auto border border-[rgba(154,126,82,0.46)] bg-[linear-gradient(180deg,rgba(55,49,39,0.98),rgba(20,18,15,0.98))] p-3 pb-24"
    >
      {actionNotice ? (
        <div className="pointer-events-none sticky top-0 z-30 flex justify-end" role="status" aria-live="polite">
          <div className="border border-[#d8bf7a]/55 bg-[#211a10]/95 px-3 py-1.5 text-[11px] font-bold uppercase tracking-wide text-[#f2d9a3] shadow-lg">
            {actionNotice}
          </div>
        </div>
      ) : null}
      <div className="mb-3 shrink-0 border-b border-[rgba(154,126,82,0.34)] pb-3">
        <h1 className="text-[18px] font-bold uppercase tracking-wide text-[#f2d9a3]">{ui("Load Decks")}</h1>
        <div className="mt-1 text-[12px] font-semibold text-[#b8aa8e]">{ui("Paste main deck lists with optional Sideboard sections.")}</div>
      </div>
      <section className="mb-3 shrink-0 border-b border-[rgba(154,126,82,0.34)] pb-3" aria-label="Mazos guardados">
        <div className="mb-2 flex flex-wrap items-baseline justify-between gap-2">
          <h2 className="text-[12px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">Mazos guardados</h2>
          <span className="text-[10px] uppercase tracking-wide text-[#8b806b]">{savedPresets.length}/{SAVED_DECK_PRESETS_LIMIT} disponibles en esta sesión</span>
        </div>
        <div className="grid gap-2 md:grid-cols-[minmax(200px,1fr)_auto_auto]">
          <select
            className={selectClass}
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
            className="h-9 max-w-[96px] truncate border border-[#9a7e52]/55 px-3 text-[11px] font-bold uppercase tracking-wide text-[#d8bf7a] hover:bg-[#2c2317] disabled:text-[#8b806b]"
            disabled={!selectedPreset || Boolean(actionBusy)}
            onClick={() => runAction("saved-use", handleApplySavedPreset)}
          >{actionBusy === "saved-use" ? <ActionSpinner /> : ui("Use")}</Button>
          <div className="flex gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-9 max-w-[96px] truncate border border-white/15 px-3 text-[11px] font-bold uppercase tracking-wide text-[#b8aa8e] hover:bg-[#2c2317] disabled:text-[#665d50]"
              disabled={!selectedPreset || Boolean(actionBusy)}
              onClick={() => runAction("saved-delete", handleDeleteSavedPreset)}
            >{actionBusy === "saved-delete" ? <ActionSpinner /> : ui("Delete")}</Button>
          </div>
        </div>
      </section>
      <div className="mb-3 shrink-0">
        <CompetitiveDeckBrowser
          onSelect={handleCatalogSelect}
        />
      </div>
      <section className="grid min-h-[420px] min-w-0 shrink-0 gap-2 border border-[rgba(154,126,82,0.42)] bg-[rgba(8,9,9,0.55)] p-2" aria-label="Editor manual de decklists">
        <div className="flex flex-wrap items-center justify-between gap-2 border-b border-white/10 px-1 pb-2">
          <div>
            <h2 className="text-[12px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">Editor manual MTGO</h2>
            <p className="text-[11px] text-[#8b806b]">Pegá líneas como <span className="font-mono">4 Counterspell</span> y una sección opcional <span className="font-mono">Sideboard</span>.</p>
            <div className="mt-1.5 flex flex-wrap gap-1.5" aria-label="Cantidad de jugadores a editar">
              {playerCountModes.map((count) => (
                <button
                  key={count}
                  type="button"
                  className={`rounded-full border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide transition-colors ${visiblePlayerCount === count ? "border-[#d8bf7a]/80 bg-[#d8bf7a]/15 text-[#f2d9a3]" : "border-white/10 text-[#b8aa8e] hover:border-[#d8bf7a]/45 hover:text-[#f2d9a3]"}`}
                  aria-pressed={visiblePlayerCount === count}
                  onClick={() => handleEditorPlayerCountChange(count)}
                >
                  x{count}
                </button>
              ))}
            </div>
          </div>
          <span className="text-[10px] uppercase tracking-wide text-[#8b806b]">También podés editar después de usar un deck</span>
        </div>
        <div className="grid grid-cols-1 gap-3 pr-1 xl:grid-cols-2">
          {visiblePlayers.map((player, i) => (
            <div
              key={player.id}
              className="setup-editor grid min-h-[290px] gap-2 border border-[rgba(154,126,82,0.42)] bg-[linear-gradient(180deg,rgba(17,17,15,0.94),rgba(8,9,9,0.96))] p-3"
              style={{ gridTemplateRows: "auto minmax(240px,1fr)" }}
            >
              <div className="flex items-baseline justify-between gap-3">
                <span className="min-w-0 truncate text-[15px] font-bold uppercase tracking-wide text-[#f2d9a3]">
                  {player.name}
                </span>
                <div className="shrink-0 text-right text-[12px] font-semibold text-[#b8aa8e]">
                  <span>{cardCounts[i]}{" " + ui("main")}</span>
                  <span className="mx-1.5 text-[#776b58]">/</span>
                  <span>{sideboardCounts[i]}{" " + ui("sideboard")}</span>
                </div>
              </div>
              <textarea
                aria-label={ui("{0} decklist", { 0: player.name })}
                spellCheck={false}
                className="min-h-[240px] w-full resize-y border border-[rgba(154,126,82,0.48)] bg-[#080b0d] p-2 font-mono text-[13px] leading-snug text-[#e7d9bc] outline-none transition-colors placeholder:text-[#8b806b] focus:border-[#d8bf7a]/75"
                placeholder={stripDeckHeader(ui("Paste {0}'s list...\n\nDeck\n4 Lightning Bolt\n2 Counterspell\n20 Island\n\nSideboard\n2 Pyroblast\n1 Tormod's Crypt", { 0: player.name }))}
                value={texts[i] || ""}
                onChange={(e) => handleTextChange(i, e.target.value)}
              />
              <div className="flex flex-wrap items-center justify-end gap-2">
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-8 max-w-[132px] truncate border border-white/15 px-2 text-[10px] font-bold uppercase tracking-wide text-[#b8aa8e] hover:bg-[#2c2317] disabled:text-[#665d50]"
                  disabled={cardCounts[i] === 0 || Boolean(actionBusy)}
                  onClick={() => runAction(`copy-${i}`, () => handleCopyMtgo(i))}
                  title={`Copiar MTGO de ${player.name}`}
                  aria-label={`Copiar MTGO de ${player.name}`}
                >
                  <svg viewBox="0 0 20 20" className="mr-1 h-3.5 w-3.5" aria-hidden="true"><rect x="6.5" y="6.5" width="9" height="10" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.4" /><path d="M13 6.5V4.8A1.3 1.3 0 0 0 11.7 3.5H5A1.5 1.5 0 0 0 3.5 5v8A1.3 1.3 0 0 0 4.8 14.3h1.7" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.4" /></svg>
                  {actionBusy === `copy-${i}` ? <ActionSpinner /> : copiedPlayerIndex === i ? "Copiado" : "Copiar"}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-8 w-8 max-w-8 rounded-full border border-white/15 p-0 text-[#b8aa8e] hover:border-[#d8bf7a]/55 hover:text-[#f2d9a3] disabled:text-[#665d50]"
                  disabled={cardCounts[i] === 0 || Boolean(actionBusy)}
                  onClick={() => handleClearPlayer(i)}
                  title={`Vaciar mazo de ${player.name}`}
                  aria-label={`Vaciar mazo de ${player.name}`}
                ><svg viewBox="0 0 20 20" className="h-3.5 w-3.5" aria-hidden="true"><path d="m5 5 10 10M15 5 5 15" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.8" /></svg></Button>
              </div>
            </div>
          ))}
        </div>
        <div className="flex flex-wrap items-center justify-end gap-2 border-t border-white/10 px-1 pt-2">
          <span className="mr-auto text-[10px] uppercase tracking-wide text-[#8b806b]">Guardá los mazos de todos los jugadores como una configuración.</span>
          <input
            className={`${fieldClass} max-w-[240px] py-1.5 text-[11px]`}
            placeholder="Nombre tu mazo"
            value={presetName}
            onChange={(event) => setPresetName(event.target.value)}
            aria-label="Nombre tu mazo"
          />
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-8 max-w-[96px] truncate border border-[#9a7e52]/55 px-3 text-[10px] font-bold uppercase tracking-wide text-[#d8bf7a] hover:bg-[#2c2317] disabled:text-[#8b806b]"
            disabled={!presetName.trim() || totalCards === 0 || Boolean(actionBusy)}
            onClick={() => runAction("saved-save", handleSavePreset)}
          >{actionBusy === "saved-save" ? <ActionSpinner /> : "Guardar"}</Button>
        </div>
      </section>
      <div className="mt-3 flex shrink-0 justify-end border-t border-[rgba(154,126,82,0.34)] pb-4 pt-3 pr-48">
        {showLobbyConfirm ? (
          <div className="mr-2 flex flex-wrap items-center justify-end gap-2 border border-[#d8bf7a]/45 bg-[#211a10] px-2 py-1.5">
            <span className="mr-1 text-[10px] font-semibold uppercase tracking-wide text-[#f2d9a3]">{ui("Create a {0}-player lobby?", { 0: lobbyPlayerCount })}</span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 border border-[#f2d9a3]/55 px-3 text-[10px] font-bold uppercase tracking-wide text-[#f2d9a3] hover:bg-[#342817]"
              disabled={Boolean(actionBusy)}
              onClick={handleConfirmLobby}
            >{ui("Confirm")}</Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8 border border-white/15 px-2 text-[10px] font-bold uppercase tracking-wide text-[#b8aa8e] hover:bg-[#2c2317]"
              disabled={Boolean(actionBusy)}
              onClick={() => setShowLobbyConfirm(false)}
            >{ui("Cancel")}</Button>
          </div>
        ) : showContinueChoices ? (
          <div className="mr-2 flex flex-wrap items-center justify-end gap-2">
            <span className="mr-1 text-[10px] font-semibold uppercase tracking-wide text-[#b8aa8e]">{ui("Continue with these decks")}</span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-9 border border-[#f2d9a3]/45 bg-[#211a10] px-3 text-[11px] font-bold uppercase tracking-wide text-[#f2d9a3] hover:bg-[#342817]"
              disabled={Boolean(actionBusy)}
              onClick={() => setShowLobbyConfirm(true)}
            >{ui("Lobby and share")}</Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-9 border border-[#9a7e52]/55 px-3 text-[11px] font-bold uppercase tracking-wide text-[#d8bf7a] hover:bg-[#2c2317]"
              disabled={Boolean(actionBusy)}
              onClick={() => runAction("test", handleTestInGame)}
            >{actionBusy === "test" ? <ActionSpinner /> : ui("Test in game")}</Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-9 border border-white/15 px-2 text-[11px] font-bold uppercase tracking-wide text-[#b8aa8e] hover:bg-[#2c2317]"
              disabled={Boolean(actionBusy)}
              onClick={() => setShowContinueChoices(false)}
            >{ui("Back")}</Button>
          </div>
        ) : (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="mr-2 h-9 border border-[#f2d9a3]/45 bg-[#211a10] px-3 text-[12px] font-bold uppercase tracking-wide text-[#f2d9a3] hover:bg-[#342817]"
            disabled={Boolean(actionBusy)}
            onClick={() => setShowContinueChoices(true)}
          >{ui("Build lobby")}</Button>
        )}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-9 border border-[#9a7e52]/45 px-3 text-[12px] font-bold uppercase tracking-wide text-[#d8bf7a] hover:bg-[#2c2317]"
          disabled={Boolean(actionBusy)}
          onClick={onCancel}
        >{ui("Cancel")}</Button>
      </div>
    </main>
  );
}
