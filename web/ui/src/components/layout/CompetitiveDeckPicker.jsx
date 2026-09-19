import { useCallback, useEffect, useMemo, useState } from "react";
import { loadCatalogDeckDetail, loadCatalogIndex, searchCatalogEntries } from "@/lib/catalog-client";
import { importDeckCatalogEntry } from "@/lib/deck-catalog-import";
import { listSavedDeckPresets, parseDeckList } from "@/lib/decklists";

const fieldClass = "fantasy-field w-full px-3 py-2 text-[13px] text-foreground outline-none";
const catalogFormats = ["modern", "pioneer", "standard", "legacy", "pauper"];

export default function CompetitiveDeckPicker({ onApply, format = "modern" }) {
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
        if (active) setError(loadError.message || "Could not load the deck catalog");
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => {
      active = false;
    };
  }, [catalogFormat]);

  // An empty query still lists decks: the search ranks the whole index and the
  // newest entries come first, so the panel is useful before anyone types.
  const decks = useMemo(
    () => searchCatalogEntries(catalog?.decks, query, { limit: 12, searchIndex: catalog?.searchIndex }),
    [catalog, query],
  );

  const savedDeckOptions = useMemo(
    () => savedPresets.flatMap((preset) => (preset.texts || [])
      .map((text, playerIndex) => ({
        key: `${preset.name}:${playerIndex}`,
        label: `${preset.name} · ${preset.playerNames?.[playerIndex] || `Jugador ${playerIndex + 1}`}`,
        text,
      }))
      .filter((option) => parseDeckList(option.text).length > 0)),
    [savedPresets],
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
      setError(applyError.message || "No se pudo importar este deck");
    } finally {
      setBusyId("");
    }
  }, [busyId, catalogFormat, onApply]);

  function applySavedDeck(option) {
    if (!option) return;
    setSelectedSavedDeckKey(option.key);
    onApply?.({ deckText: option.text, commanderText: "", deck: option });
  }

  return (
    <section className="lobby-sheet-panel fantasy-sheet-section grid gap-3 p-3" aria-label="Competitive deck catalog">
      <div className="flex items-baseline justify-between gap-3">
        <div>
          <h3 className="text-[13px] font-semibold uppercase tracking-[0.16em] text-foreground">
            Buscar deck competitivo
          </h3>
          <p className="text-[12px] text-muted-foreground">
            Catálogo local actualizado por el sincronizador
          </p>
        </div>
        <select
          className="fantasy-field px-2 py-1 text-[11px] uppercase tracking-[0.12em] text-foreground outline-none"
          value={catalogFormat}
          onChange={(event) => setCatalogFormat(event.target.value)}
          aria-label="Formato del catálogo"
        >
          {catalogFormats.map((formatOption) => <option key={formatOption} value={formatOption}>{formatOption}</option>)}
        </select>
      </div>
      {savedDeckOptions.length ? (
        <label className="grid gap-1 text-[11px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
          Mazos guardados de esta sesión
          <select
            className={fieldClass}
            value={selectedSavedDeckKey}
            onChange={(event) => applySavedDeck(savedDeckOptions.find((option) => option.key === event.target.value))}
            aria-label="Elegir un mazo guardado para el lobby"
          >
            <option value="">Elegir Alice, Bob u otro mazo guardado</option>
            {savedDeckOptions.map((option) => <option key={option.key} value={option.key}>{option.label}</option>)}
          </select>
        </label>
      ) : null}
      <input
        className={fieldClass}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder="Arquetipo, carta o evento"
        aria-label="Buscar decks"
      />
      {loading ? <p className="text-[12px] text-muted-foreground">Cargando catálogo…</p> : null}
      {error ? <p className="text-[12px] text-red-300">{error}</p> : null}
      {!loading && !error && decks.length === 0 ? (
        <p className="text-[12px] text-muted-foreground">No hay decks para esta búsqueda.</p>
      ) : null}
      <div className="grid max-h-[320px] gap-2 overflow-y-auto pr-1" data-lobby-catalog-list="">
        {decks.map((deck) => (
          <article key={deck.id} className="grid gap-1.5 border border-white/10 bg-black/10 p-2.5">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <h4 className="truncate text-[13px] font-semibold text-foreground">{deck.name || deck.archetype}</h4>
                <p className="truncate text-[12px] text-muted-foreground">
                  {deck.archetype || "Sin arquetipo"} · {deck.event || deck.source || "Fuente desconocida"}
                </p>
              </div>
              <button
                type="button"
                className="stone-pill shrink-0 px-2 py-1 text-[11px] font-semibold uppercase tracking-[0.1em] disabled:opacity-60"
                disabled={Boolean(busyId)}
                onClick={() => applyDeck(deck)}
              >
                {busyId === deck.id ? "…" : "Usar"}
              </button>
            </div>
            <p className="text-[11px] leading-5 text-muted-foreground">
              {deck.mainboardCount || "?"} cartas{deck.sideboardCount ? ` · ${deck.sideboardCount} SB` : ""} · {deck.date || "sin fecha"}
              {deck.placement ? ` · puesto #${deck.placement}` : ""}
            </p>
            <div className="flex flex-wrap gap-1">
              {(deck.cardNames || []).slice(0, 5).map((cardName) => (
                <span key={`${deck.id}-${cardName}`} className="rounded border border-white/10 px-1.5 py-0.5 text-[10px] text-muted-foreground">
                  {cardName}
                </span>
              ))}
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}
