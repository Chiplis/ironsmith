import { useMemo, useState } from "react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import { Slider } from "@/components/ui/slider";
import {
  findSavedDeckPreset,
  listSavedDeckPresets,
  parseDeckList,
  parseSideboardList,
  saveSavedDeckPreset,
} from "@/lib/decklists";

const fieldClass =
  "w-full border border-[rgba(154,126,82,0.46)] bg-[#0b0d0e] px-3 py-2 text-[13px] text-[#e7d9bc] outline-none transition-colors placeholder:text-[#8b806b] focus:border-[#d8bf7a]/75";
const labelClass = "grid gap-1 text-[11px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]";

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
  const {
    state,
    setStatus,
    semanticThreshold,
    setSemanticThreshold,
    cardsMeetingThreshold,
  } = useGame();
  const players = state?.players || [];
  const [texts, setTexts] = useState(() => players.map(() => ""));
  const [savedPresets, setSavedPresets] = useState(() => listSavedDeckPresets());
  const [selectedPresetName, setSelectedPresetName] = useState("");
  const [presetName, setPresetName] = useState("");

  const handleTextChange = (index, value) => {
    setTexts((prev) => {
      const next = [...prev];
      next[index] = value;
      return next;
    });
  };

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
  };

  const handleLoad = () => {
    const decks = texts.map(parseDeckList);
    const sideboards = texts.map(parseSideboardList);
    const normalizedPresetName = presetName.trim();

    if (normalizedPresetName) {
      const existingPreset = findSavedDeckPreset(normalizedPresetName);
      const nextTexts = fitTextsToPlayers(players, texts);
      const shouldConfirmOverride =
        existingPreset && !samePresetTexts(existingPreset.texts, nextTexts);
      if (
        shouldConfirmOverride
        && !window.confirm(`A saved deck named "${existingPreset.name}" already exists. Override it?`)
      ) {
        onLoad({ decks, sideboards });
        return;
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
      }
    }

    onLoad({ decks, sideboards });
  };

  return (
    <main
      className="table-gradient flex h-full min-h-0 flex-col overflow-hidden border border-[rgba(154,126,82,0.46)] bg-[linear-gradient(180deg,rgba(55,49,39,0.98),rgba(20,18,15,0.98))] p-3"
    >
      <div className="mb-3 grid shrink-0 gap-3 border-b border-[rgba(154,126,82,0.34)] pb-3 xl:grid-cols-[minmax(180px,260px)_minmax(0,1fr)]">
        <div className="min-w-[220px]">
          <h1 className="text-[18px] font-bold uppercase tracking-wide text-[#f2d9a3]">
            Load Decks
          </h1>
          <div className="mt-1 text-[12px] font-semibold text-[#b8aa8e]">
            Paste main deck lists with optional Sideboard sections.
          </div>
        </div>
        <div className="grid min-w-0 gap-2 md:grid-cols-[minmax(0,1fr)_minmax(220px,300px)]">
          <label className={labelClass}>
            Saved Deck
            <div className="flex gap-2">
              <select
                className={fieldClass}
                value={selectedPresetName}
                onChange={(event) => setSelectedPresetName(event.target.value)}
              >
                <option value="">Select a saved deck</option>
                {savedPresets.map((preset) => (
                  <option key={preset.name} value={preset.name}>
                    {preset.name}
                  </option>
                ))}
              </select>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-9 shrink-0 border border-[#9a7e52]/55 px-3 text-[12px] font-bold uppercase tracking-wide text-[#d8bf7a] hover:bg-[#2c2317] disabled:text-[#8b806b]"
                disabled={!selectedPreset}
                onClick={handleApplySavedPreset}
              >
                Use
              </Button>
            </div>
          </label>
          <label className={labelClass}>
            Save As
            <input
              className={fieldClass}
              placeholder="Friday gauntlet"
              value={presetName}
              onChange={(event) => setPresetName(event.target.value)}
            />
          </label>
        </div>
      </div>
      <div
        className="grid min-h-0 flex-1 grid-cols-1 gap-3 overflow-y-auto pr-1 xl:grid-cols-2"
      >
        {players.map((player, i) => (
          <div
            key={player.id}
            className="grid min-h-[260px] gap-2 border border-[rgba(154,126,82,0.42)] bg-[linear-gradient(180deg,rgba(17,17,15,0.94),rgba(8,9,9,0.96))] p-3"
            style={{ gridTemplateRows: "auto minmax(180px,1fr)" }}
          >
            <div className="flex items-baseline justify-between gap-3">
              <span className="min-w-0 truncate text-[15px] font-bold uppercase tracking-wide text-[#f2d9a3]">
                {player.name}
              </span>
              <div className="shrink-0 text-right text-[12px] font-semibold text-[#b8aa8e]">
                <span>{cardCounts[i]} main</span>
                <span className="mx-1.5 text-[#776b58]">/</span>
                <span>{sideboardCounts[i]} sideboard</span>
              </div>
            </div>
            <textarea
              className="h-full min-h-0 w-full resize-none border border-[rgba(154,126,82,0.48)] bg-[#080b0d] p-2 font-mono text-[13px] leading-snug text-[#e7d9bc] outline-none transition-colors placeholder:text-[#8b806b] focus:border-[#d8bf7a]/75"
              placeholder={`Paste ${player.name}'s list...\n\nDeck\n4 Lightning Bolt\n2 Counterspell\n20 Island\n\nSideboard\n2 Pyroblast\n1 Tormod's Crypt`}
              value={texts[i] || ""}
              onChange={(e) => handleTextChange(i, e.target.value)}
            />
          </div>
        ))}
      </div>
      <div className="mt-3 flex shrink-0 flex-wrap items-center justify-between gap-3 border-t border-[rgba(154,126,82,0.34)] pt-3">
        <div className="flex min-w-0 flex-1 flex-wrap items-center gap-2">
          <span className="whitespace-nowrap text-[12px] font-semibold uppercase tracking-wide text-[#d8bf7a]">
            Min similarity
          </span>
          <Slider
            className="w-28"
            min={0}
            max={100}
            step={1}
            value={[Math.round(semanticThreshold)]}
            onValueChange={([value]) => setSemanticThreshold(value)}
          />
          <span className="whitespace-nowrap text-[12px] text-[#b8aa8e]">
            {semanticThreshold > 0 ? `${Math.round(semanticThreshold)}%` : "Off"} ({cardsMeetingThreshold})
          </span>
        </div>
        <div className="flex items-center justify-center gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-9 border border-[#f2d9a3]/45 bg-[#211a10] px-4 text-[12px] font-bold uppercase tracking-wide text-[#f2d9a3] hover:bg-[#342817]"
            onClick={handleLoad}
          >
            Load{totalCards > 0 ? ` (${totalCards} main${totalSideboardCards > 0 ? `, ${totalSideboardCards} sideboard` : ""})` : ""}
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="h-9 border border-[#9a7e52]/45 px-3 text-[12px] font-bold uppercase tracking-wide text-[#d8bf7a] hover:bg-[#2c2317]"
            onClick={onCancel}
          >
            Cancel
          </Button>
        </div>
      </div>
    </main>
  );
}
