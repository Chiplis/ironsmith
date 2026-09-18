import { memo, useCallback, useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { deckCatalogEntryToMtgoText, importDeckCatalogEntry } from "@/lib/deck-catalog-import";
import { loadCatalogDeckDetail, loadCatalogIndex, loadLocalCardArt, searchCatalogEntries } from "@/lib/catalog-client";

const fieldClass = "w-full border border-[rgba(154,126,82,0.46)] bg-[#0b0d0e] px-3 py-2 text-[13px] text-[#e7d9bc] outline-none";
const labelClass = "grid gap-1 text-[11px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]";
const CAROUSEL_SIZE = 3;
const CAROUSEL_BUFFER = 3;
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

function matchesCollection(entry, collection) {
  if (collection === "major") return isMajorEntry(entry);
  if (collection === "top8") return isTop8Entry(entry);
  return true;
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
  const [collection, setCollection] = useState("all");
  const [sortMode, setSortMode] = useState("recent");
  const busyRef = useRef("");
  const copyingRef = useRef("");
  const carouselRef = useRef(null);
  const dragRef = useRef(null);
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

  const filteredResults = useMemo(() => {
    const filtered = searchResults.filter((entry) => matchesCollection(entry, collection));
    return [...filtered].sort((left, right) => {
      if (sortMode === "placement") {
        return (Number(left.placement) || 9999) - (Number(right.placement) || 9999)
          || String(right.date || "").localeCompare(String(left.date || ""));
      }
      return String(right.date || "").localeCompare(String(left.date || ""))
        || (Number(left.placement) || 9999) - (Number(right.placement) || 9999);
    });
  }, [collection, searchResults, sortMode]);

  const carouselEntries = useMemo(() => {
    if (filteredResults.length <= CAROUSEL_SIZE) return filteredResults;
    return [
      ...filteredResults.slice(-CAROUSEL_BUFFER),
      ...filteredResults,
      ...filteredResults.slice(0, CAROUSEL_BUFFER),
    ];
  }, [filteredResults]);

  const carouselStep = useCallback(() => {
    const container = carouselRef.current;
    const first = container?.firstElementChild;
    if (!container || !first) return 0;
    const gap = Number.parseFloat(getComputedStyle(container).columnGap || "0");
    return first.getBoundingClientRect().width + gap;
  }, []);

  const resetCarousel = useCallback(() => {
    const container = carouselRef.current;
    const step = carouselStep();
    if (!container) return;
    if (filteredResults.length <= CAROUSEL_SIZE) {
      container.scrollTo({ left: 0, behavior: "auto" });
      return;
    }
    if (!step) return;
    container.scrollTo({ left: step * CAROUSEL_BUFFER, behavior: "auto" });
  }, [carouselStep, filteredResults.length]);

  useEffect(() => {
    const frame = requestAnimationFrame(resetCarousel);
    return () => cancelAnimationFrame(frame);
  }, [resetCarousel]);

  const handleCarouselScroll = useCallback(() => {
    const container = carouselRef.current;
    const step = carouselStep();
    if (!container || !step || filteredResults.length <= CAROUSEL_SIZE) return;
    const originalWidth = step * filteredResults.length;
    const firstOriginal = step * CAROUSEL_BUFFER;
    const lastOriginal = firstOriginal + originalWidth;
    if (container.scrollLeft < firstOriginal - step * 0.5) container.scrollLeft += originalWidth;
    if (container.scrollLeft > lastOriginal - step * 0.5) container.scrollLeft -= originalWidth;
  }, [carouselStep, filteredResults.length]);

  const moveCarousel = useCallback((direction) => {
    const container = carouselRef.current;
    const step = carouselStep();
    if (container && step) container.scrollBy({ left: direction * step, behavior: "smooth" });
  }, [carouselStep]);

  const handlePointerDown = useCallback((event) => {
    if (event.target.closest("button")) return;
    const container = carouselRef.current;
    if (!container) return;
    dragRef.current = { pointerId: event.pointerId, startX: event.clientX, startScrollLeft: container.scrollLeft };
    container.setPointerCapture?.(event.pointerId);
  }, []);

  const handlePointerMove = useCallback((event) => {
    const drag = dragRef.current;
    const container = carouselRef.current;
    if (!drag || !container) return;
    container.scrollLeft = drag.startScrollLeft - (event.clientX - drag.startX);
  }, []);

  const handlePointerUp = useCallback((event) => {
    const container = carouselRef.current;
    if (container && dragRef.current?.pointerId === event.pointerId) {
      container.releasePointerCapture?.(event.pointerId);
    }
    dragRef.current = null;
  }, []);

  const collectionCounts = useMemo(() => {
    const counts = Object.fromEntries(collectionOptions.map(({ id }) => [id, 0]));
    counts.all = searchResults.length;
    counts.recent = searchResults.length;
    for (const entry of searchResults) {
      if (isMajorEntry(entry)) counts.major += 1;
      if (isTop8Entry(entry)) counts.top8 += 1;
    }
    return counts;
  }, [searchResults]);

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
      <input className={fieldClass} value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Broodscale Bloodchief, Dimir Control, Counterspell..." aria-label="Buscar en catálogo" />
      <div className="flex flex-wrap items-center gap-1.5" aria-label="Filtros del catálogo">
        {collectionOptions.map((option) => (
          <Button key={option.id} type="button" variant="ghost" size="sm" className={`h-7 rounded-full px-2 text-[10px] font-bold uppercase tracking-wide ${collection === option.id ? "bg-[#342817] text-[#f2d9a3]" : "text-[#b8aa8e] hover:bg-white/5"}`} onClick={() => setCollection(option.id)}>
            {option.label} ({collectionCounts[option.id]})
          </Button>
        ))}
        <label className="ml-auto flex items-center gap-1 text-[10px] font-bold uppercase tracking-wide text-[#b8aa8e]">Ordenar
          <select className="bg-transparent px-1 py-1 text-[10px] text-[#e7d9bc]" value={sortMode} onChange={(event) => setSortMode(event.target.value)}>
            <option value="recent">Más recientes</option>
            <option value="placement">Mejor puesto</option>
          </select>
        </label>
      </div>
      {loading ? <p className="text-[12px] text-[#b8aa8e]">Cargando índice…</p> : null}
      {error ? <p className="text-[12px] text-red-300">{error}</p> : null}
      {!loading && !error && carouselEntries.length === 0 ? <p className="text-[12px] text-[#b8aa8e]">No hay resultados para esta búsqueda.</p> : null}
      <div className="flex items-stretch gap-2 overflow-x-auto overflow-y-hidden scroll-smooth snap-x snap-mandatory pb-1 pr-1 touch-pan-y select-none" ref={carouselRef} onScroll={handleCarouselScroll} onPointerDown={handlePointerDown} onPointerMove={handlePointerMove} onPointerUp={handlePointerUp} onPointerCancel={handlePointerUp}>
        {carouselEntries.map((entry, index) => <div key={`${entry.id}-${index}`} className="min-w-[min(86vw,360px)] snap-start lg:min-w-0 lg:flex-[0_0_32%]"><CatalogDeckRow entry={entry} isBusy={busyId === entry.id} isCopying={copyingId === entry.id} isCopied={copiedId === entry.id} onSelect={handleSelect} onCopy={handleCopy} /></div>)}
      </div>
      {filteredResults.length > CAROUSEL_SIZE ? (
        <div className="flex items-center justify-between gap-2 pt-1">
          <Button type="button" variant="ghost" size="sm" className="h-8 w-8 rounded-full p-0 text-[#d8bf7a] hover:bg-white/10" aria-label="Decks anteriores" title="Decks anteriores" onClick={() => moveCarousel(-1)}>
            <svg viewBox="0 0 20 20" className="h-4 w-4" aria-hidden="true"><path d="M12.5 4.5 7 10l5.5 5.5" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" /></svg>
          </Button>
          <span className="text-[10px] uppercase tracking-wide text-[#8b806b]">Arrastrá para explorar · {filteredResults.length} decks</span>
          <Button type="button" variant="ghost" size="sm" className="h-8 w-8 rounded-full p-0 text-[#d8bf7a] hover:bg-white/10" aria-label="Decks siguientes" title="Decks siguientes" onClick={() => moveCarousel(1)}>
            <svg viewBox="0 0 20 20" className="h-4 w-4" aria-hidden="true"><path d="m7.5 4.5 5.5 5.5-5.5 5.5" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" /></svg>
          </Button>
        </div>
      ) : null}
    </section>
  );
}
