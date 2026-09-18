import { memo, useCallback, useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { deckCatalogEntryToMtgoText, importDeckCatalogEntry } from "@/lib/deck-catalog-import";
import { loadCatalogDeckDetail, loadCatalogIndex, loadLocalCardArt, searchCatalogEntries } from "@/lib/catalog-client";

const fieldClass = "w-full border border-[rgba(154,126,82,0.46)] bg-[#0b0d0e] px-3 py-2 text-[13px] text-[#e7d9bc] outline-none";
const labelClass = "grid gap-1 text-[11px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]";
const CAROUSEL_SIZE = 3;
const collectionOptions = [
  { id: "all", label: "Todos" },
  { id: "recent", label: "Recientes" },
  { id: "major", label: "Eventos grandes" },
  { id: "top8", label: "Top 8" },
];

function isMajorEntry(entry) {
  const event = String(entry?.event || "").toLocaleLowerCase("en-US");
  return /(pro tour|grand prix|regional|championship|scg|open)/.test(event);
}

function isTop8Entry(entry) {
  return Number(entry?.placement) > 0 && Number(entry.placement) <= 8;
}

function buildRecentIds(entries) {
  const datedEntries = entries
    .map((entry) => ({ id: entry?.id, timestamp: Date.parse(entry?.date || "") }))
    .filter(({ id, timestamp }) => id && Number.isFinite(timestamp));
  if (!datedEntries.length) return new Set(entries.map((entry) => entry?.id).filter(Boolean));
  const newestTimestamp = Math.max(...datedEntries.map(({ timestamp }) => timestamp));
  const recentCutoff = newestTimestamp - (1000 * 60 * 60 * 24 * 30);
  return new Set(datedEntries.filter(({ timestamp }) => timestamp >= recentCutoff).map(({ id }) => id));
}

function matchesCollectionFilters(entry, activeCollections, recentIds) {
  return activeCollections.every((collection) => {
    if (collection === "recent") return recentIds.has(entry?.id);
    if (collection === "major") return isMajorEntry(entry);
    if (collection === "top8") return isTop8Entry(entry);
    return true;
  });
}

const CatalogDeckRow = memo(function CatalogDeckRow({ entry, isBusy, isCopying, isCopied, onSelect, onCopy }) {
  const [artUrl, setArtUrl] = useState("");

  useEffect(() => {
    let active = true;
    loadLocalCardArt(entry.cardNames?.[0]).then((url) => {
      if (active) setArtUrl(url);
    });
    return () => {
      active = false;
    };
  }, [entry]);

  return (
    <article className="flex min-w-0 gap-3 rounded-sm bg-black/15 p-3 shadow-[inset_0_-1px_rgba(255,255,255,0.08)]">
      <div className="h-[104px] w-[74px] shrink-0 overflow-hidden rounded-sm bg-[#17130e]" aria-hidden="true">
        {artUrl ? <img className="h-full w-full object-cover" src={artUrl} alt="" loading="lazy" onError={(event) => { event.currentTarget.style.display = "none"; }} /> : null}
      </div>
      <div className="min-w-0">
        <div className="truncate text-[13px] font-bold text-[#e7d9bc]">{entry.name || entry.archetype || "Deck sin nombre"}</div>
        <div className="truncate text-[11px] text-[#b8aa8e]">{entry.event || "Evento desconocido"} · {entry.date || "sin fecha"}{entry.placement ? ` · #${entry.placement}` : ""} · {entry.mainboardCount || "?"} cartas{entry.sideboardCount ? ` + ${entry.sideboardCount} SB` : ""}</div>
        <div className="truncate text-[10px] text-[#8b806b]">
          {(entry.colors || []).join(" ")} {(entry.mechanics || []).join(" · ")}
          {(entry.cardNames || []).slice(0, 4).length ? ` · ${(entry.cardNames || []).slice(0, 4).join(", ")}` : ""}
        </div>
      </div>
      <div className="ml-auto flex shrink-0 flex-col justify-center gap-1">
        <Button type="button" variant="ghost" size="sm" className="h-8 border border-[#9a7e52]/55 px-2 text-[10px] font-bold uppercase tracking-wide text-[#d8bf7a]" disabled={isBusy || isCopying} onClick={() => onSelect(entry)}>
          {isBusy ? "Cargando…" : "Usar"}
        </Button>
        <Button type="button" variant="ghost" size="sm" className="h-7 border border-white/15 px-2 text-[10px] font-semibold text-[#b8aa8e]" disabled={isBusy || isCopying} onClick={() => onCopy(entry)}>
          {isCopying ? "Copiando…" : isCopied ? "Copiado" : "Copiar MTGO"}
        </Button>
      </div>
    </article>
  );
});

export default function CompetitiveDeckBrowser({ players, targetIndex, onTargetChange, onSelect }) {
  const [catalog, setCatalog] = useState(null);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState("");
  const [copyingId, setCopyingId] = useState("");
  const [copiedId, setCopiedId] = useState("");
  const [error, setError] = useState("");
  const [activeCollections, setActiveCollections] = useState([]);
  const [sortMode, setSortMode] = useState("recent");
  const [carouselPage, setCarouselPage] = useState(0);
  const busyRef = useRef("");
  const copyingRef = useRef("");
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

  const searchResults = useMemo(
    () => searchCatalogEntries(catalog?.decks, deferredQuery, { limit: 240, searchIndex: catalog?.searchIndex }),
    [catalog, deferredQuery],
  );

  const recentIds = useMemo(() => buildRecentIds(catalog?.decks || []), [catalog]);

  const filteredResults = useMemo(() => {
    const filtered = searchResults.filter((entry) => matchesCollectionFilters(entry, activeCollections, recentIds));
    return [...filtered].sort((left, right) => {
      if (sortMode === "placement") {
        return (Number(left.placement) || 9999) - (Number(right.placement) || 9999)
          || String(right.date || "").localeCompare(String(left.date || ""));
      }
      return String(right.date || "").localeCompare(String(left.date || ""))
        || (Number(left.placement) || 9999) - (Number(right.placement) || 9999);
    });
  }, [activeCollections, recentIds, searchResults, sortMode]);

  const pageCount = Math.max(1, Math.ceil(filteredResults.length / CAROUSEL_SIZE));
  const visiblePage = Math.min(carouselPage, pageCount - 1);
  const carouselEntries = filteredResults.slice(visiblePage * CAROUSEL_SIZE, (visiblePage + 1) * CAROUSEL_SIZE);

  const collectionCounts = useMemo(() => {
    const counts = Object.fromEntries(collectionOptions.map(({ id }) => [id, 0]));
    counts.all = searchResults.length;
    counts.recent = searchResults.filter((entry) => recentIds.has(entry?.id)).length;
    for (const entry of searchResults) {
      if (isMajorEntry(entry)) counts.major += 1;
      if (isTop8Entry(entry)) counts.top8 += 1;
    }
    return counts;
  }, [recentIds, searchResults]);

  const toggleCollection = useCallback((collection) => {
    setActiveCollections((current) => current.includes(collection)
      ? current.filter((active) => active !== collection)
      : [...current, collection]);
    setCarouselPage(0);
  }, []);

  const clearCollections = useCallback(() => {
    setActiveCollections([]);
    setCarouselPage(0);
  }, []);

  const handleSelect = useCallback(async (entry) => {
    if (busyRef.current) return;
    busyRef.current = entry.id;
    setBusyId(entry.id);
    setError("");
    try {
      const detail = await loadCatalogDeckDetail(entry);
      onSelect(importDeckCatalogEntry(detail));
    } catch (selectError) {
      setError(selectError.message || "Could not import this deck");
    } finally {
      busyRef.current = "";
      setBusyId("");
    }
  }, [onSelect]);

  const handleCopy = useCallback(async (entry) => {
    if (copyingRef.current || busyRef.current) return;
    copyingRef.current = entry.id;
    setCopyingId(entry.id);
    setError("");
    try {
      const detail = await loadCatalogDeckDetail(entry);
      const text = deckCatalogEntryToMtgoText(detail);
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
      setCopiedId(entry.id);
    } catch (copyError) {
      setError(copyError.message || "No se pudo copiar el deck");
    } finally {
      copyingRef.current = "";
      setCopyingId("");
    }
  }, []);

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
      <input className={fieldClass} value={query} onChange={(event) => { setQuery(event.target.value); setCarouselPage(0); }} placeholder="Broodscale Bloodchief, Dimir Control, Counterspell..." aria-label="Buscar en catálogo" />
      <div className="flex flex-wrap items-center gap-1.5" aria-label="Filtros del catálogo">
        <Button type="button" variant="ghost" size="sm" className={`h-7 rounded-full px-2 text-[10px] font-bold uppercase tracking-wide ${activeCollections.length === 0 ? "bg-[#342817] text-[#f2d9a3]" : "text-[#b8aa8e] hover:bg-white/5"}`} aria-pressed={activeCollections.length === 0} onClick={clearCollections}>
          {collectionOptions[0].label} ({collectionCounts.all})
        </Button>
        {collectionOptions.slice(1).map((option) => {
          const isActive = activeCollections.includes(option.id);
          return (
            <Button key={option.id} type="button" variant="ghost" size="sm" className={`h-7 rounded-full px-2 text-[10px] font-bold uppercase tracking-wide ${isActive ? "bg-[#342817] text-[#f2d9a3] ring-1 ring-[#d8bf7a]/55" : "text-[#b8aa8e] hover:bg-white/5"}`} aria-pressed={isActive} onClick={() => toggleCollection(option.id)}>
              {isActive ? "✓ " : ""}{option.label} ({collectionCounts[option.id]})
            </Button>
          );
        })}
        {activeCollections.length ? <Button type="button" variant="ghost" size="sm" className="h-7 px-1.5 text-[10px] font-semibold text-[#8b806b] hover:text-[#e7d9bc]" onClick={clearCollections}>Limpiar</Button> : null}
        <label className="ml-auto flex items-center gap-1 text-[10px] font-bold uppercase tracking-wide text-[#b8aa8e]">Ordenar
          <select className="bg-transparent px-1 py-1 text-[10px] text-[#e7d9bc]" value={sortMode} onChange={(event) => { setSortMode(event.target.value); setCarouselPage(0); }}>
            <option value="recent">Más recientes</option>
            <option value="placement">Mejor puesto</option>
          </select>
        </label>
      </div>
      {loading ? <p className="text-[12px] text-[#b8aa8e]">Cargando índice…</p> : null}
      {error ? <p className="text-[12px] text-red-300">{error}</p> : null}
      {!loading && !error && carouselEntries.length === 0 ? <p className="text-[12px] text-[#b8aa8e]">No hay resultados para esta búsqueda.</p> : null}
      <div className="relative px-0 sm:px-9">
        {filteredResults.length > CAROUSEL_SIZE ? (
          <Button type="button" variant="ghost" size="sm" className="absolute left-0 top-1/2 z-10 h-9 w-9 -translate-y-1/2 rounded-full bg-[#17130e]/85 p-0 text-[#d8bf7a] shadow-lg hover:bg-[#342817]" aria-label="Decks anteriores" title="Decks anteriores" onClick={() => setCarouselPage((current) => (current - 1 + pageCount) % pageCount)}>
            <svg viewBox="0 0 20 20" className="h-4 w-4" aria-hidden="true"><path d="M12.5 4.5 7 10l5.5 5.5" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" /></svg>
          </Button>
        ) : null}
        <div className="grid items-stretch gap-2 sm:grid-cols-2 lg:grid-cols-3">
          {carouselEntries.map((entry) => <CatalogDeckRow key={entry.id} entry={entry} isBusy={busyId === entry.id} isCopying={copyingId === entry.id} isCopied={copiedId === entry.id} onSelect={handleSelect} onCopy={handleCopy} />)}
        </div>
        {filteredResults.length > CAROUSEL_SIZE ? (
          <Button type="button" variant="ghost" size="sm" className="absolute right-0 top-1/2 z-10 h-9 w-9 -translate-y-1/2 rounded-full bg-[#17130e]/85 p-0 text-[#d8bf7a] shadow-lg hover:bg-[#342817]" aria-label="Decks siguientes" title="Decks siguientes" onClick={() => setCarouselPage((current) => (current + 1) % pageCount)}>
            <svg viewBox="0 0 20 20" className="h-4 w-4" aria-hidden="true"><path d="m7.5 4.5 5.5 5.5-5.5 5.5" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" /></svg>
          </Button>
        ) : null}
      </div>
      {filteredResults.length > CAROUSEL_SIZE ? (
        <div className="pt-1 text-center text-[10px] uppercase tracking-wide text-[#8b806b]">{visiblePage + 1} / {pageCount} · {filteredResults.length} decks · carrusel circular</div>
      ) : null}
    </section>
  );
}
