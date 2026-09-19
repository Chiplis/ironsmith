import { useCallback, useEffect, useMemo, useState } from "react";
import useUiText from "@/i18n/useUiText";
import { DeckArt, ManaPips } from "@/components/deck/DeckCatalogParts";
import { loadCatalogDeckDetail, loadCatalogIndex, searchCatalogEntries } from "@/lib/catalog-client";
import { entryColors } from "@/lib/deck-catalog-view";
import { importDeckCatalogEntry } from "@/lib/deck-catalog-import";
import { listSavedDeckPresets, parseDeckList } from "@/lib/decklists";

const fieldClass = "w-full bg-[#050607] px-3 py-2 text-[13px] text-[#e7d9bc] outline-none placeholder:text-[#6f6759]";
const catalogFormats = ["modern", "pioneer", "standard", "legacy", "pauper"];

export default function CompetitiveDeckPicker({ onApply, format = "modern" }) {
  const ui = useUiText();
  const [catalogFormat, setCatalogFormat] = useState(format);
  const [catalog, setCatalog] = useState(null);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [busyId, setBusyId] = useState("");
  const [savedPresets] = useState(() => listSavedDeckPresets());
  const [selectedSavedDeckKey, setSelectedSavedDeckKey] = useState("");

  useEffect(() => {
    let active = true;
    setCatalog(null);
    setLoading(true);
    setError("");
    loadCatalogIndex({ format: catalogFormat })
      .then((nextCatalog) => {
        if (active) setCatalog(nextCatalog);
      })
      .catch((loadError) => {
        if (active) setError(loadError.message || ui("Could not load the catalog"));
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
  }, [catalogFormat, ui]);

  // An empty query still lists decks: the search ranks the whole index and the
  // newest entries come first, so the panel is useful before anyone types.
  const decks = useMemo(
    () => searchCatalogEntries(catalog?.decks, query, { limit: 24, searchIndex: catalog?.searchIndex }),
    [catalog, query],
  );

  const savedDeckOptions = useMemo(
    () => savedPresets.flatMap((preset) => (preset.texts || [])
      .map((text, playerIndex) => ({
        key: `${preset.name}:${playerIndex}`,
        label: `${preset.name} · ${preset.playerNames?.[playerIndex] || ui("Player {0}", { 0: playerIndex + 1 })}`,
        text,
      }))
      .filter((option) => parseDeckList(option.text).length > 0)),
    [savedPresets, ui],
  );

  const applyDeck = useCallback(async (entry) => {
    if (busyId) return;
    setBusyId(entry.id);
    setError("");
    try {
      const detail = await loadCatalogDeckDetail(entry, { format: catalogFormat });
      const imported = importDeckCatalogEntry(detail);
      onApply?.({ deckText: imported.deckText, commanderText: imported.commanderText, deck: imported });
    } catch (applyError) {
      setError(applyError.message || ui("Could not import this deck"));
    } finally {
      setBusyId("");
    }
  }, [busyId, catalogFormat, onApply, ui]);

  function applySavedDeck(option) {
    if (!option) return;
    setSelectedSavedDeckKey(option.key);
    onApply?.({ deckText: option.text, commanderText: "", deck: option });
  }

  return (
    <section className="lobby-deck-picker grid gap-2 bg-[#0e0f11] p-3" aria-label={ui("Competitive deck catalog")}>
      <div className="flex items-baseline justify-between gap-3">
        <div className="min-w-0">
          <h3 className="text-[12px] font-bold uppercase tracking-[0.16em] text-[#f2d9a3]">
            {ui("Find a competitive deck")}
          </h3>
          <p className="truncate text-[11px] text-[#6f6759]">
            {ui("Local catalog refreshed by the synchronizer")}
          </p>
        </div>
        <select
          className="shrink-0 bg-[#050607] px-2 py-1 text-[11px] uppercase tracking-[0.12em] text-[#e7d9bc] outline-none"
          value={catalogFormat}
          onChange={(event) => setCatalogFormat(event.target.value)}
          aria-label={ui("Catalog format")}
        >
          {catalogFormats.map((formatOption) => <option key={formatOption} value={formatOption}>{formatOption}</option>)}
        </select>
      </div>
      {savedDeckOptions.length ? (
        <label className="grid gap-1 text-[10px] font-bold uppercase tracking-[0.14em] text-[#6f6759]">
          {ui("Saved decks from this session")}
          <select
            className={fieldClass}
            value={selectedSavedDeckKey}
            onChange={(event) => applySavedDeck(savedDeckOptions.find((option) => option.key === event.target.value))}
            aria-label={ui("Choose a saved deck for the lobby")}
          >
            <option value="">{ui("Choose a saved deck")}</option>
            {savedDeckOptions.map((option) => <option key={option.key} value={option.key}>{option.label}</option>)}
          </select>
        </label>
      ) : null}
      <input
        className={fieldClass}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder={ui("Search by name, archetype, card, event…")}
        aria-label={ui("Search decks")}
      />
      {loading ? <p className="text-[12px] text-[#b8aa8e]">{ui("Loading catalog…")}</p> : null}
      {error ? <p className="text-[12px] text-red-300">{error}</p> : null}
      {!loading && !error && decks.length === 0 ? (
        <p className="text-[12px] text-[#b8aa8e]">{ui("No decks for this search.")}</p>
      ) : null}
      <div className="grid max-h-[320px] auto-rows-min gap-x-3 overflow-y-auto overflow-x-hidden pr-1 xl:grid-cols-2" data-lobby-catalog-list="">
        {decks.map((deck) => {
          const source = [deck.event || ui("Unknown event"), String(deck.format || "").toUpperCase()].filter(Boolean).join(" · ");
          return (
            <article key={deck.id} className="flex min-w-0 items-center gap-2.5 rounded-sm p-1.5 transition-colors hover:bg-[#131418]" data-lobby-deck-row={deck.id}>
              <DeckArt entry={deck} className="h-[40px] w-[56px] rounded-sm" />
              <div className="min-w-0 flex-1">
                <div className="flex min-w-0 items-baseline gap-2">
                  <span className="min-w-0 flex-1 truncate text-[12px] font-bold text-[#e7d9bc]">{deck.name || deck.archetype || ui("Unnamed deck")}</span>
                  <span className="shrink-0 text-[10px] text-[#6f6759]">{deck.date || ui("no date")}</span>
                </div>
                <div className="flex min-w-0 items-center gap-2">
                  <ManaPips colors={entryColors(deck)} size={11} />
                  <span className="min-w-0 truncate text-[10px] text-[#8b806b]">{source}</span>
                </div>
              </div>
              <button
                type="button"
                className="flat-button-gold h-7 w-[64px] shrink-0 rounded-sm text-[10px] font-bold uppercase tracking-wide disabled:opacity-60"
                disabled={Boolean(busyId)}
                onClick={() => applyDeck(deck)}
              >
                {busyId === deck.id ? "…" : ui("Use")}
              </button>
            </article>
          );
        })}
      </div>
    </section>
  );
}
