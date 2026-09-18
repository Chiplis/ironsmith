import { memo, useCallback, useDeferredValue, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { importDeckCatalogEntry } from "@/lib/deck-catalog-import";
import { loadCatalogDeckDetail, loadCatalogIndex, searchCatalogEntries } from "@/lib/catalog-client";

const fieldClass = "w-full border border-[rgba(154,126,82,0.46)] bg-[#0b0d0e] px-3 py-2 text-[13px] text-[#e7d9bc] outline-none";
const labelClass = "grid gap-1 text-[11px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]";

const CatalogDeckRow = memo(function CatalogDeckRow({ entry, busyId, onSelect }) {
  return (
    <div className="flex items-center justify-between gap-3 border border-white/10 px-2 py-2">
      <div className="min-w-0">
        <div className="truncate text-[13px] font-bold text-[#e7d9bc]">{entry.name || entry.archetype || "Deck sin nombre"}</div>
        <div className="truncate text-[11px] text-[#b8aa8e]">{entry.event || "Evento desconocido"} · {entry.date || "sin fecha"}{entry.placement ? ` · #${entry.placement}` : ""}</div>
        <div className="truncate text-[10px] text-[#8b806b]">
          {(entry.colors || []).join(" ")} {(entry.mechanics || []).join(" · ")}
          {(entry.cardNames || []).slice(0, 4).length ? ` · ${(entry.cardNames || []).slice(0, 4).join(", ")}` : ""}
        </div>
      </div>
      <Button type="button" variant="ghost" size="sm" className="h-8 shrink-0 border border-[#9a7e52]/55 px-3 text-[11px] font-bold uppercase tracking-wide text-[#d8bf7a]" disabled={Boolean(busyId)} onClick={() => onSelect(entry)}>
        {busyId === entry.id ? "Cargando…" : "Usar"}
      </Button>
    </div>
  );
});

export default function CompetitiveDeckBrowser({ players, targetIndex, onTargetChange, onSelect }) {
  const [catalog, setCatalog] = useState(null);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState("");
  const [error, setError] = useState("");
  const deferredQuery = useDeferredValue(query);

  useEffect(() => {
    let active = true;
    loadCatalogIndex()
      .then((nextCatalog) => {
        if (active) setCatalog(nextCatalog);
      })
      .catch((loadError) => {
        if (active) setError(loadError.message || "Could not load the catalog");
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);

  const results = useMemo(
    () => searchCatalogEntries(catalog?.decks, deferredQuery, { searchIndex: catalog?.searchIndex }),
    [catalog, deferredQuery],
  );

  const handleSelect = useCallback(async (entry) => {
    if (busyId) return;
    setBusyId(entry.id);
    setError("");
    try {
      const detail = await loadCatalogDeckDetail(entry);
      onSelect(importDeckCatalogEntry(detail));
    } catch (selectError) {
      setError(selectError.message || "Could not import this deck");
    } finally {
      setBusyId("");
    }
  }, [busyId, onSelect]);

  return (
    <section className="grid gap-2 border border-[rgba(154,126,82,0.42)] bg-[rgba(8,9,9,0.7)] p-3" aria-label="Competitive deck catalog">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <h2 className="text-[13px] font-bold uppercase tracking-[0.16em] text-[#f2d9a3]">Catálogo competitivo</h2>
          <p className="text-[11px] text-[#b8aa8e]">Buscá por arquetipo, carta, evento o color. El detalle se carga sólo al elegir.</p>
        </div>
        <label className={labelClass}>Jugador destino<select className={fieldClass} value={targetIndex} onChange={(event) => onTargetChange(Number(event.target.value))}>
          {players.map((player, index) => <option key={player.id || index} value={index}>{player.name}</option>)}
        </select></label>
      </div>
      <input className={fieldClass} value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Broodscale Bloodchief, Dimir Control, Counterspell..." aria-label="Buscar en catálogo" />
      {loading ? <p className="text-[12px] text-[#b8aa8e]">Cargando índice…</p> : null}
      {error ? <p className="text-[12px] text-red-300">{error}</p> : null}
      {!loading && !error && results.length === 0 ? <p className="text-[12px] text-[#b8aa8e]">No hay resultados para esta búsqueda.</p> : null}
      <div className="grid max-h-[220px] gap-2 overflow-y-auto pr-1">
        {results.map((entry) => <CatalogDeckRow key={entry.id} entry={entry} busyId={busyId} onSelect={handleSelect} />)}
      </div>
    </section>
  );
}
