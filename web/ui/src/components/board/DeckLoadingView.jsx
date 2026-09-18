import useUiText from "@/i18n/useUiText";
import { useCallback, useMemo, useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import {
  findSavedDeckPreset,
  listSavedDeckPresets,
  parseDeckList,
  parseDeckPrintPreferences,
  parseSideboardList,
  removeSavedDeckPreset,
  saveSavedDeckPreset,
  SAVED_DECK_PRESETS_LIMIT,
} from "@/lib/decklists";
import { setPreferredCardPrints } from "@/lib/scryfall";
import CompetitiveDeckBrowser from "./CompetitiveDeckBrowser";

const fieldClass =
  "w-full border border-[rgba(154,126,82,0.46)] bg-[#0b0d0e] px-3 py-2 text-[13px] text-[#e7d9bc] outline-none transition-colors placeholder:text-[#8b806b] focus:border-[#d8bf7a]/75";

function samePresetTexts(left, right) {
  const leftTexts = Array.isArray(left) ? left : [];
  const rightTexts = Array.isArray(right) ? right : [];
  if (leftTexts.length !== rightTexts.length) return false;
  return leftTexts.every((text, index) => String(text || "") === String(rightTexts[index] || ""));
}

function fitTextsToPlayers(players, texts) {
  return players.map((_, index) => String(texts?.[index] || ""));
}

export default function DeckLoadingView({ onLoad, onCancel }) {
  const ui = useUiText();
  const {
    state,
    setStatus,
    semanticThreshold,
    setSemanticThreshold,
    cardsMeetingThreshold,
  } = useGame();
  const players = useMemo(() => state?.players || [], [state?.players]);
  const [texts, setTexts] = useState(() => players.map(() => ""));
  const [savedPresets, setSavedPresets] = useState(() => listSavedDeckPresets());
  const [selectedPresetName, setSelectedPresetName] = useState("");
  const [presetName, setPresetName] = useState("");
  const [playerSaveNames, setPlayerSaveNames] = useState({});
  const [submitting, setSubmitting] = useState(false);
  const [catalogTargetIndex, setCatalogTargetIndex] = useState(0);

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
  const totalSideboardCards = sideboardCounts.reduce((a, b) => a + b, 0);

  const selectedPreset = useMemo(
    () =>
      savedPresets.find(
        (preset) => preset.name === selectedPresetName
      ) || null,
    [savedPresets, selectedPresetName]
  );

  const handleApplySavedPreset = () => {
    if (!selectedPreset) return;
    setTexts(fitTextsToPlayers(players, selectedPreset.texts));
    setPresetName(selectedPreset.name);
    setPlayerSaveNames(Object.fromEntries(players.map((_, index) => [index, selectedPreset.name])));
  };

  const saveCurrentPreset = useCallback((requestedName = presetName) => {
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

    const saveResult = saveSavedDeckPreset(normalizedPresetName, nextTexts);
    if (saveResult.saved) {
      setSavedPresets(saveResult.entries);
      setSelectedPresetName(saveResult.entry.name);
      setPresetName(saveResult.entry.name);
      setStatus(
        saveResult.replaced
          ? `Updated saved deck "${saveResult.entry.name}"`
          : `Saved deck "${saveResult.entry.name}"`
      );
      return true;
    }
    if (saveResult.reason === "limit") {
      setStatus(`Session limit reached (${SAVED_DECK_PRESETS_LIMIT} decks). Delete one saved deck to add another.`);
    }
    return false;
  }, [players, presetName, setStatus, texts, ui]);

  const handleSavePreset = useCallback(() => {
    saveCurrentPreset(presetName);
  }, [presetName, saveCurrentPreset]);

  const handleSavePlayerPreset = useCallback((playerIndex) => {
    saveCurrentPreset(playerSaveNames[playerIndex]);
  }, [playerSaveNames, saveCurrentPreset]);

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
      setStatus(`Deck de ${players[playerIndex]?.name || "jugador"} copiado en formato MTGO.`);
    } catch {
      setStatus("No se pudo copiar el deck.");
    }
  }, [catalogTargetIndex, players, setStatus, texts]);

  const handleLoad = async () => {
    if (submitting) return;
    const decks = texts.map(parseDeckList);
    const sideboards = texts.map(parseSideboardList);
    setPreferredCardPrints(texts.flatMap(parseDeckPrintPreferences));

    if (presetName.trim()) saveCurrentPreset();

    setSubmitting(true);
    try {
      await onLoad({ decks, sideboards });
    } finally {
      setSubmitting(false);
    }
  };

  const handleCatalogSelect = useCallback(({ deckText }) => {
    handleTextChange(catalogTargetIndex, deckText);
    setPresetName("");
  }, [catalogTargetIndex, handleTextChange]);

  const handleDeleteSavedPreset = useCallback(() => {
    if (!selectedPreset) return;
    if (!window.confirm(ui('Delete saved deck "{0}"?', { 0: selectedPreset.name }))) return;
    setSavedPresets(removeSavedDeckPreset(selectedPreset.name));
    setSelectedPresetName("");
    setPresetName("");
    setStatus(`Deleted saved deck "${selectedPreset.name}"`);
  }, [selectedPreset, setStatus, ui]);

  return (
    <main
      className="setup-screen deck-loading-screen table-gradient flex h-full min-h-0 flex-col overflow-y-auto border border-[rgba(154,126,82,0.46)] bg-[linear-gradient(180deg,rgba(55,49,39,0.98),rgba(20,18,15,0.98))] p-3"
    >
      <div className="mb-3 shrink-0 border-b border-[rgba(154,126,82,0.34)] pb-3">
        <h1 className="text-[18px] font-bold uppercase tracking-wide text-[#f2d9a3]">{ui("Load Decks")}</h1>
        <div className="mt-1 text-[12px] font-semibold text-[#b8aa8e]">{ui("Paste main deck lists with optional Sideboard sections.")}</div>
      </div>
      <section className="mb-3 shrink-0 border-b border-[rgba(154,126,82,0.34)] pb-3" aria-label="Mazos guardados">
        <div className="mb-2 flex flex-wrap items-baseline justify-between gap-2">
          <h2 className="text-[12px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">Mazos guardados</h2>
          <span className="text-[10px] uppercase tracking-wide text-[#8b806b]">{savedPresets.length}/{SAVED_DECK_PRESETS_LIMIT} disponibles en esta sesión</span>
        </div>
        <div className="grid gap-2 md:grid-cols-[minmax(220px,1fr)_auto_minmax(180px,260px)_auto]">
          <select
            className={fieldClass}
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
            disabled={!selectedPreset}
            onClick={handleApplySavedPreset}
          >{ui("Use")}</Button>
          <input
            className={fieldClass}
            placeholder="Nombre tu mazo"
            value={presetName}
            onChange={(event) => setPresetName(event.target.value)}
            aria-label="Nombre tu mazo"
          />
          <div className="flex gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-9 max-w-[128px] truncate border border-[#9a7e52]/55 px-3 text-[11px] font-bold uppercase tracking-wide text-[#d8bf7a] hover:bg-[#2c2317] disabled:text-[#8b806b]"
              disabled={!presetName.trim() || totalCards === 0}
              onClick={handleSavePreset}
            >Guardar</Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-9 max-w-[96px] truncate border border-white/15 px-3 text-[11px] font-bold uppercase tracking-wide text-[#b8aa8e] hover:bg-[#2c2317] disabled:text-[#665d50]"
              disabled={!selectedPreset}
              onClick={handleDeleteSavedPreset}
            >{ui("Delete")}</Button>
          </div>
        </div>
      </section>
      <div className="mb-3 shrink-0">
        <CompetitiveDeckBrowser
          players={players}
          targetIndex={catalogTargetIndex}
          onTargetChange={setCatalogTargetIndex}
          onSelect={handleCatalogSelect}
        />
      </div>
      <section className="grid min-h-[420px] min-w-0 shrink-0 gap-2 border border-[rgba(154,126,82,0.42)] bg-[rgba(8,9,9,0.55)] p-2" aria-label="Editor manual de decklists">
        <div className="flex flex-wrap items-center justify-between gap-2 border-b border-white/10 px-1 pb-2">
          <div>
            <h2 className="text-[12px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">Editor manual MTGO</h2>
            <p className="text-[11px] text-[#8b806b]">Pegá líneas como <span className="font-mono">4 Counterspell</span> y una sección opcional <span className="font-mono">Sideboard</span>.</p>
          </div>
          <span className="text-[10px] uppercase tracking-wide text-[#8b806b]">También podés editar después de usar un deck</span>
        </div>
        <div className="grid grid-cols-1 gap-3 pr-1 xl:grid-cols-2">
          {players.map((player, i) => (
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
                placeholder={ui("Paste {0}'s list...\n\nDeck\n4 Lightning Bolt\n2 Counterspell\n20 Island\n\nSideboard\n2 Pyroblast\n1 Tormod's Crypt", { 0: player.name })}
                value={texts[i] || ""}
                onChange={(e) => handleTextChange(i, e.target.value)}
              />
              <div className="flex flex-wrap items-center justify-end gap-2">
                <input
                  className={`${fieldClass} max-w-[220px] py-1.5 text-[11px]`}
                  placeholder="Nombre tu mazo"
                  value={playerSaveNames[i] || ""}
                  onChange={(event) => setPlayerSaveNames((current) => ({ ...current, [i]: event.target.value }))}
                  aria-label={`Nombre del mazo de ${player.name}`}
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-8 max-w-[132px] truncate border border-white/15 px-2 text-[10px] font-bold uppercase tracking-wide text-[#b8aa8e] hover:bg-[#2c2317] disabled:text-[#665d50]"
                  disabled={cardCounts[i] === 0}
                  onClick={() => handleCopyMtgo(i)}
                  title={`Copiar MTGO de ${player.name}`}
                  aria-label={`Copiar MTGO de ${player.name}`}
                >
                  <svg viewBox="0 0 20 20" className="mr-1 h-3.5 w-3.5" aria-hidden="true"><rect x="6.5" y="6.5" width="9" height="10" rx="1.5" fill="none" stroke="currentColor" strokeWidth="1.4" /><path d="M13 6.5V4.8A1.3 1.3 0 0 0 11.7 3.5H5A1.5 1.5 0 0 0 3.5 5v8A1.3 1.3 0 0 0 4.8 14.3h1.7" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.4" /></svg>
                  Copiar
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="h-8 max-w-[88px] truncate border border-[#9a7e52]/55 px-2 text-[10px] font-bold uppercase tracking-wide text-[#d8bf7a] hover:bg-[#2c2317] disabled:text-[#8b806b]"
                  disabled={!playerSaveNames[i]?.trim() || cardCounts[i] === 0}
                  onClick={() => handleSavePlayerPreset(i)}
                >Guardar</Button>
              </div>
            </div>
          ))}
        </div>
      </section>
      <div className="mt-3 flex shrink-0 flex-wrap items-center justify-between gap-3 border-t border-[rgba(154,126,82,0.34)] pt-3">
        <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2">
          <span className="whitespace-nowrap text-[12px] font-semibold uppercase tracking-wide text-[#d8bf7a]">{ui("Min similarity")}</span>
          <Slider
            aria-label={ui("Card fidelity threshold")}
            className="w-28"
            min={0}
            max={100}
            step={1}
            value={[Math.round(semanticThreshold)]}
            onValueChange={([value]) => setSemanticThreshold(value)}
          />
          <span className="whitespace-nowrap text-[12px] text-[#b8aa8e]">
            {semanticThreshold > 0 ? `${Math.round(semanticThreshold)}%` : ui("Off")} ({cardsMeetingThreshold})
          </span>
        </div>
        <div className="flex items-center justify-center gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="ui-primary-action h-9 border border-[#f2d9a3]/45 bg-[#211a10] px-4 text-[12px] font-bold uppercase tracking-wide text-[#f2d9a3] hover:bg-[#342817]"
            disabled={totalCards === 0 || submitting}
            onClick={handleLoad}
          >{submitting ? ui("Loading…") : ui("Load")}{!submitting && totalCards > 0 ? ui(" ({0} main{1})", { 0: totalCards, 1: totalSideboardCards > 0 ? `, ${totalSideboardCards} sideboard` : "" }) : ""}
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-9 border border-[#9a7e52]/45 px-3 text-[12px] font-bold uppercase tracking-wide text-[#d8bf7a] hover:bg-[#2c2317]"
            onClick={onCancel}
          >{ui("Cancel")}</Button>
        </div>
      </div>
    </main>
  );
}
