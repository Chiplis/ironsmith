import { memo, useCallback, useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { deckCatalogEntryToMtgoText, importDeckCatalogEntry } from "@/lib/deck-catalog-import";
import { loadCatalogDeckDetail, loadCatalogIndex, loadLocalCardArt, searchCatalogEntries } from "@/lib/catalog-client";
import { ManaSymbol } from "@/lib/mana-symbols";

const fieldClass = "w-full border border-[rgba(154,126,82,0.46)] bg-[#0b0d0e] px-3 py-2 text-[13px] text-[#e7d9bc] outline-none";
const labelClass = "grid gap-1 text-[11px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]";
const CAROUSEL_SIZE = 3;
const CAROUSEL_STEP = 2;
const manaOptions = ["W", "U", "B", "R", "G", "C"];
const catalogFormats = [
  { id: "modern", label: "Modern" },
  { id: "pioneer", label: "Pioneer" },
  { id: "standard", label: "Standard" },
  { id: "legacy", label: "Legacy" },
  { id: "pauper", label: "Pauper" },
];

function ActionSpinner() {
  return <svg viewBox="0 0 20 20" className="h-3.5 w-3.5 animate-spin" aria-hidden="true"><circle cx="10" cy="10" r="7" fill="none" stroke="currentColor" strokeOpacity="0.25" strokeWidth="2" /><path d="M17 10a7 7 0 0 0-7-7" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="2" /></svg>;
}

function isMajorEntry(entry) {
  const event = String(entry?.event || "").toLocaleLowerCase("en-US");
  const tags = (entry?.tags || []).join(" ").toLocaleLowerCase("en-US");
  return /(pro tour|grand prix|regional|championship|scg|open|showcase|spotlight|qualifier)/.test(`${event} ${tags}`);
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

function completeManaProfile(entry) {
  const profile = entry?.manaProfile;
  return profile?.metadataCoverage?.complete === true ? profile : null;
}

function isRecentEntry(entry, recentIds) {
  return entry?.collections?.includes("last-20-events") || recentIds.has(entry?.id);
}

function isMajorCollectionEntry(entry) {
  return entry?.collections?.includes("last-major-events") || isMajorEntry(entry);
}

function sortDeckEntries(entries, sortMode, usageCounts) {
  return [...entries].sort((left, right) => {
    if (sortMode === "usage") {
      const leftKey = String(left?.archetype || left?.name || "").trim().toLocaleLowerCase("en-US");
      const rightKey = String(right?.archetype || right?.name || "").trim().toLocaleLowerCase("en-US");
      return (usageCounts.get(rightKey) || 0) - (usageCounts.get(leftKey) || 0)
        || String(right.date || "").localeCompare(String(left.date || ""))
        || String(left.id || "").localeCompare(String(right.id || ""));
    }
    if (sortMode === "placement") {
      return (Number(left.placement) || 9999) - (Number(right.placement) || 9999)
        || String(right.date || "").localeCompare(String(left.date || ""));
    }
    return String(right.date || "").localeCompare(String(left.date || ""))
      || (Number(left.placement) || 9999) - (Number(right.placement) || 9999)
      || String(left.id || "").localeCompare(String(right.id || ""));
  });
}

function matchesManaFilters(entry, activeMana, manaMatchMode) {
  const colors = completeManaProfile(entry)?.colors || [];
  if (manaMatchMode === "exact") {
    const coloredColors = colors.filter((color) => color !== "C");
    const requestedColored = activeMana.filter((color) => color !== "C");
    if (coloredColors.length !== requestedColored.length || !requestedColored.every((color) => coloredColors.includes(color))) return false;
  }
  return activeMana.every((color) => colors.includes(color));
}

const CatalogDeckRow = memo(function CatalogDeckRow({ entry, actionKey, targetPlayerName, isBusy, isCopying, isCopied, onSelect, onCopy }) {
  const [artUrl, setArtUrl] = useState("");
  const manaProfile = completeManaProfile(entry);
  const colors = (manaProfile?.colors || []).filter((color) => /^[WUBRGC]$/.test(color));
  const predominantColors = manaProfile?.predominantColors || [];
  const predominantLands = (manaProfile?.predominantLands || []).slice(0, 2);

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
    <article className="flex min-w-0 gap-3 rounded-sm bg-transparent p-3">
      <div className="h-[104px] w-[74px] shrink-0 overflow-hidden rounded-sm bg-[#17130e]" aria-hidden="true">
        {artUrl ? <img className="h-full w-full object-cover" src={artUrl} alt="" loading="lazy" onError={(event) => { event.currentTarget.style.display = "none"; }} /> : null}
      </div>
      <div className="min-w-0">
        <div className="truncate text-[13px] font-bold text-[#e7d9bc]">{entry.name || entry.archetype || "Deck sin nombre"}</div>
        <div className="truncate text-[11px] text-[#b8aa8e]">{entry.event || "Evento desconocido"} · {entry.date || "sin fecha"}{entry.placement ? ` · #${entry.placement}` : ""} · {entry.mainboardCount || "?"} cartas{entry.sideboardCount ? ` + ${entry.sideboardCount} SB` : ""}</div>
        <div className="flex min-w-0 items-center gap-1.5 truncate text-[10px] text-[#8b806b]">
          {colors.length ? <span className="inline-flex shrink-0 items-center gap-0.5" aria-label={`Colores: ${colors.join(", ")}`} title={`Mana predominante: ${predominantColors.join(", ") || "sin predominio"}`}>
            {colors.map((color) => <ManaSymbol key={color} sym={color} size={13} />)}
          </span> : null}
          <span className="shrink-0 uppercase">{entry.format || "modern"}</span>
          {manaProfile ? <span className="shrink-0">· {manaProfile.landCount} tierras</span> : null}
          {predominantLands.length ? <span className="truncate" title={`Tierras predominantes: ${predominantLands.map(({ name, count }) => `${name} (${count})`).join(", ")}`}>· {predominantLands.map(({ name, count }) => `${name} ${count}`).join(", ")}</span> : null}
          {(entry.mechanics || []).length ? <span className="truncate">· {(entry.mechanics || []).join(" · ")}</span> : null}
          {(entry.cardNames || []).slice(0, 3).length ? <span className="truncate">· {(entry.cardNames || []).slice(0, 3).join(", ")}</span> : null}
        </div>
      </div>
      <div className="ml-auto flex shrink-0 flex-col justify-center gap-1">
        <Button type="button" variant="ghost" size="sm" className="h-8 w-[112px] max-w-[112px] truncate border border-[#9a7e52]/55 px-2 text-[10px] font-bold uppercase tracking-wide text-[#d8bf7a]" disabled={isBusy || isCopying} onClick={() => onSelect(entry, actionKey)} title={`Usar en ${targetPlayerName}`}>
          {isBusy ? <ActionSpinner /> : `Usar en ${targetPlayerName}`}
        </Button>
        <Button type="button" variant="ghost" size="sm" className="h-7 w-[88px] max-w-[88px] truncate border border-white/15 px-2 text-[10px] font-semibold text-[#b8aa8e]" disabled={isBusy || isCopying} onClick={() => onCopy(entry, actionKey)}>
          {isCopying ? <ActionSpinner /> : isCopied ? "Copiado" : "Copiar MTGO"}
        </Button>
      </div>
    </article>
  );
});

const DeckCarousel = memo(function DeckCarousel({ title, entries, targetPlayerName, resetKey, busyId, copyingId, copiedId, onSelect, onCopy }) {
  const [offset, setOffset] = useState(0);
  const [animating, setAnimating] = useState(false);
  const animatingRef = useRef(false);
  const centerOffset = entries.length * 2;
  const trackEntries = useMemo(() => {
    if (!entries.length) return [];
    const trackLength = Math.max(CAROUSEL_SIZE, entries.length * 5 + CAROUSEL_SIZE);
    return Array.from({ length: trackLength }, (_, index) => ({
      entry: entries[index % entries.length],
      key: `${entries[index % entries.length].id}-${index}`,
      actionKey: `${title}-${entries[index % entries.length].id}-${index}`,
    }));
  }, [entries, title]);

  useEffect(() => {
    animatingRef.current = false;
    const timer = window.setTimeout(() => {
      setAnimating(false);
      setOffset(centerOffset);
    }, 0);
    return () => window.clearTimeout(timer);
  }, [centerOffset, resetKey]);

  useEffect(() => {
    if (!animating || !entries.length) return undefined;
    const timer = window.setTimeout(() => {
      const count = entries.length;
      setOffset((current) => {
        const relative = ((current - (count * 2)) % count + count) % count;
        return (count * 2) + relative;
      });
      animatingRef.current = false;
      setAnimating(false);
    }, 380);
    return () => window.clearTimeout(timer);
  }, [animating, entries.length]);

  const move = useCallback((direction) => {
    if (entries.length <= CAROUSEL_SIZE || animatingRef.current) return;
    animatingRef.current = true;
    setAnimating(true);
    setOffset((current) => current + direction * CAROUSEL_STEP);
  }, [entries.length]);

  if (!entries.length) return null;
  return (
    <section className="grid gap-1 border-t border-white/10 pt-2" aria-label={title}>
      <h3 className="px-1 text-[10px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">{title}</h3>
      <div className="relative">
        {entries.length > CAROUSEL_SIZE ? <Button type="button" variant="ghost" size="sm" className="absolute left-0 top-1/2 z-10 h-8 w-8 -translate-y-1/2 rounded-full bg-[#11110f] p-0 text-[#d8bf7a] shadow-lg hover:bg-[#28231b]" aria-label={`${title}: decks anteriores`} title="Decks anteriores" onClick={() => move(-1)}>
          <svg viewBox="0 0 20 20" className="h-4 w-4" aria-hidden="true"><path d="M12.5 4.5 7 10l5.5 5.5" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" /></svg>
        </Button> : null}
        <div className="mx-9 overflow-hidden" style={{ containerType: "inline-size" }}>
          <div className="flex gap-2" style={{ transform: `translateX(calc(-${offset} * (33.333cqw + 0.1667rem)))`, transition: animating ? "transform 380ms cubic-bezier(0.22, 0.61, 0.36, 1)" : "none" }}>
            {trackEntries.map(({ entry, key, actionKey }) => <div key={key} className="min-w-0" style={{ flex: "0 0 calc(33.333cqw - 0.333rem)" }}><CatalogDeckRow entry={entry} actionKey={actionKey} targetPlayerName={targetPlayerName} isBusy={busyId === actionKey} isCopying={copyingId === actionKey} isCopied={copiedId === actionKey} onSelect={onSelect} onCopy={onCopy} /></div>)}
          </div>
        </div>
        {entries.length > CAROUSEL_SIZE ? <Button type="button" variant="ghost" size="sm" className="absolute right-0 top-1/2 z-10 h-8 w-8 -translate-y-1/2 rounded-full bg-[#11110f] p-0 text-[#d8bf7a] shadow-lg hover:bg-[#28231b]" aria-label={`${title}: decks siguientes`} title="Decks siguientes" onClick={() => move(1)}>
          <svg viewBox="0 0 20 20" className="h-4 w-4" aria-hidden="true"><path d="m7.5 4.5 5.5 5.5-5.5 5.5" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" /></svg>
        </Button> : null}
      </div>
    </section>
  );
});

export default function CompetitiveDeckBrowser({ players, targetIndex, onSelect }) {
  const [catalog, setCatalog] = useState(null);
  const [catalogFormat, setCatalogFormat] = useState("modern");
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState("");
  const [copyingId, setCopyingId] = useState("");
  const [copiedId, setCopiedId] = useState("");
  const [error, setError] = useState("");
  const [activeMana, setActiveMana] = useState([]);
  const [manaMatchMode, setManaMatchMode] = useState("include");
  const [sortMode, setSortMode] = useState("recent");
  const [carouselResetKey, setCarouselResetKey] = useState(0);
  const busyRef = useRef("");
  const copyingRef = useRef("");
  const deferredQuery = useDeferredValue(query);
  const targetPlayerName = players[targetIndex]?.name || "jugador";

  const resetCarousel = useCallback(() => {
    setCarouselResetKey((current) => current + 1);
  }, []);

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
        if (active) setError(loadError.message || "Could not load the catalog");
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [catalogFormat]);

  const searchResults = useMemo(
    () => searchCatalogEntries(catalog?.decks, deferredQuery, { limit: 240, searchIndex: catalog?.searchIndex }),
    [catalog, deferredQuery],
  );

  const recentIds = useMemo(() => buildRecentIds(catalog?.decks || []), [catalog]);

  const usageCounts = useMemo(() => {
    const counts = new Map();
    for (const entry of catalog?.decks || []) {
      const key = String(entry?.archetype || entry?.name || "").trim().toLocaleLowerCase("en-US");
      if (key) counts.set(key, (counts.get(key) || 0) + 1);
    }
    return counts;
  }, [catalog]);

  const manaFilteredResults = useMemo(
    () => searchResults.filter((entry) => matchesManaFilters(entry, activeMana, manaMatchMode)),
    [activeMana, manaMatchMode, searchResults],
  );

  const recentResults = useMemo(
    () => sortDeckEntries(manaFilteredResults.filter((entry) => isRecentEntry(entry, recentIds)), sortMode, usageCounts),
    [manaFilteredResults, recentIds, sortMode, usageCounts],
  );

  const majorResults = useMemo(
    () => sortDeckEntries(manaFilteredResults.filter(isMajorCollectionEntry), sortMode, usageCounts),
    [manaFilteredResults, sortMode, usageCounts],
  );

  const monoResults = useMemo(
    () => sortDeckEntries(manaFilteredResults.filter((entry) => entry?.collections?.includes("mono-color")), sortMode, usageCounts),
    [manaFilteredResults, sortMode, usageCounts],
  );

  const availableMana = useMemo(
    () => manaOptions.filter((color) => searchResults.some((entry) => (completeManaProfile(entry)?.colors || []).includes(color))),
    [searchResults],
  );

  const clearFilters = useCallback(() => {
    setActiveMana([]);
    setManaMatchMode("include");
    resetCarousel();
  }, [resetCarousel]);

  const toggleMana = useCallback((color) => {
    setActiveMana((current) => {
      if (!current.includes(color)) return [...current, color];
      const next = current.filter((active) => active !== color);
      if (!next.length) setManaMatchMode("include");
      return next;
    });
    resetCarousel();
  }, [resetCarousel]);

  const handleSelect = useCallback(async (entry, actionKey) => {
    if (busyRef.current) return;
    busyRef.current = actionKey;
    setBusyId(actionKey);
    setError("");
    try {
      const detail = await loadCatalogDeckDetail(entry, { format: catalogFormat });
      onSelect(importDeckCatalogEntry(detail));
    } catch (selectError) {
      setError(selectError.message || "Could not import this deck");
    } finally {
      busyRef.current = "";
      setBusyId("");
    }
  }, [catalogFormat, onSelect]);

  const handleCopy = useCallback(async (entry, actionKey) => {
    if (copyingRef.current || busyRef.current) return;
    copyingRef.current = actionKey;
    setCopyingId(actionKey);
    setError("");
    try {
      const detail = await loadCatalogDeckDetail(entry, { format: catalogFormat });
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
      setCopiedId(actionKey);
      window.setTimeout(() => setCopiedId((current) => current === actionKey ? "" : current), 900);
    } catch (copyError) {
      setError(copyError.message || "No se pudo copiar el deck");
    } finally {
      copyingRef.current = "";
      setCopyingId("");
    }
  }, [catalogFormat]);

  return (
    <section className="grid gap-2 border-b border-[rgba(154,126,82,0.32)] bg-transparent pb-3" aria-label="Decks">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <h2 className="text-[13px] font-bold uppercase tracking-[0.16em] text-[#f2d9a3]">Decks</h2>
          <p className="text-[11px] text-[#b8aa8e]">Buscá por arquetipo, carta, evento o color. El detalle se carga sólo al elegir.</p>
        </div>
        <div className="flex flex-wrap items-end gap-2">
          <label className={labelClass}>Formato<select className={fieldClass} value={catalogFormat} onChange={(event) => { setCatalogFormat(event.target.value); resetCarousel(); }}>
            {catalogFormats.map((formatOption) => <option key={formatOption.id} value={formatOption.id}>{formatOption.label}</option>)}
          </select></label>
          <span className="text-[10px] text-[#8b806b]" aria-live="polite">El deck elegido se pondrá en {targetPlayerName}.</span>
        </div>
      </div>
      <input className={fieldClass} value={query} onChange={(event) => { setQuery(event.target.value); resetCarousel(); }} placeholder="Broodscale Bloodchief, Dimir Control, Counterspell..." aria-label="Buscar en catálogo" />
      <div className="flex flex-wrap items-center gap-1.5" aria-label="Filtros del catálogo">
        {availableMana.map((color) => {
          const isActive = activeMana.includes(color);
          return <Button key={color} type="button" variant="ghost" size="sm" className={`h-7 w-8 max-w-8 rounded-full px-1 text-[10px] font-bold ${isActive ? "bg-[#342817] ring-1 ring-[#d8bf7a]/55" : "text-[#b8aa8e] hover:bg-white/5"}`} aria-label={`Filtrar por mana ${color}`} aria-pressed={isActive} onClick={() => toggleMana(color)}><ManaSymbol sym={color} size={15} /></Button>;
        })}
        {activeMana.length ? <div className="flex items-center gap-0.5 rounded-full border border-white/10 p-0.5" aria-label="Modo de coincidencia de mana">
          <Button type="button" variant="ghost" size="sm" className={`h-6 rounded-full px-2 text-[9px] font-bold uppercase tracking-wide ${manaMatchMode === "include" ? "bg-[#342817] text-[#f2d9a3] ring-1 ring-[#d8bf7a]/55 shadow-[0_0_9px_rgba(216,191,122,0.28)]" : "text-[#8b806b] hover:text-[#e7d9bc]"}`} aria-pressed={manaMatchMode === "include"} title="Incluye estos colores, aunque el deck use otros" onClick={() => { setManaMatchMode("include"); resetCarousel(); }}>Incluye</Button>
          <Button type="button" variant="ghost" size="sm" className={`h-6 rounded-full px-2 text-[9px] font-bold uppercase tracking-wide ${manaMatchMode === "exact" ? "bg-[#342817] text-[#f2d9a3] ring-1 ring-[#d8bf7a]/55 shadow-[0_0_9px_rgba(216,191,122,0.28)]" : "text-[#8b806b] hover:text-[#e7d9bc]"}`} aria-pressed={manaMatchMode === "exact"} title="Sólo estos colores; el maná C puede ser auxiliar" onClick={() => { setManaMatchMode("exact"); resetCarousel(); }}>Sólo estos</Button>
        </div> : null}
        {activeMana.length ? <Button type="button" variant="ghost" size="sm" className="h-7 px-1.5 text-[10px] font-semibold text-[#8b806b] hover:text-[#e7d9bc]" onClick={clearFilters}>Limpiar</Button> : null}
        <label className="ml-auto flex items-center gap-1 text-[10px] font-bold uppercase tracking-wide text-[#b8aa8e]">Ordenar
          <select className="bg-transparent px-1 py-1 text-[10px] text-[#e7d9bc]" value={sortMode} onChange={(event) => { setSortMode(event.target.value); resetCarousel(); }}>
            <option value="recent">Más recientes</option>
            <option value="placement">Mejor puesto</option>
            <option value="usage">Más usados</option>
          </select>
        </label>
      </div>
      {loading ? <p className="text-[12px] text-[#b8aa8e]">Cargando índice…</p> : null}
      {error ? <p className="text-[12px] text-red-300">{error}</p> : null}
      {!loading && !error && !manaFilteredResults.length ? <p className="text-[12px] text-[#b8aa8e]">No hay resultados para esta búsqueda.</p> : null}
      <DeckCarousel title="Mono-color" entries={monoResults} targetPlayerName={targetPlayerName} resetKey={carouselResetKey} busyId={busyId} copyingId={copyingId} copiedId={copiedId} onSelect={handleSelect} onCopy={handleCopy} />
      <DeckCarousel title="Last major events" entries={majorResults} targetPlayerName={targetPlayerName} resetKey={carouselResetKey} busyId={busyId} copyingId={copyingId} copiedId={copiedId} onSelect={handleSelect} onCopy={handleCopy} />
      <DeckCarousel title="Last 20 events" entries={recentResults} targetPlayerName={targetPlayerName} resetKey={carouselResetKey} busyId={busyId} copyingId={copyingId} copiedId={copiedId} onSelect={handleSelect} onCopy={handleCopy} />
    </section>
  );
}
