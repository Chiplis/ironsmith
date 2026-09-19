import { memo, useCallback, useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import useUiText from "@/i18n/useUiText";
import { Button } from "@/components/ui/button";
import { deckCatalogEntryToMtgoText, importDeckCatalogEntry } from "@/lib/deck-catalog-import";
import { loadCatalogDeckDetail, loadCatalogIndex, loadLocalCardArt, searchCatalogEntries } from "@/lib/catalog-client";
import { ManaSymbol } from "@/lib/mana-symbols";

const fieldClass = "w-full border border-[rgba(154,126,82,0.46)] bg-[#0b0d0e] px-3 py-2 text-[13px] text-[#e7d9bc] outline-none";
const selectClass = `${fieldClass} pr-12`;
const selectStyle = {
  appearance: "none",
  backgroundImage: "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 20 20' fill='none' stroke='%23b8aa8e' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.8'%3E%3Cpath d='m5 7 5 5 5-5'/%3E%3C/svg%3E\")",
  backgroundPosition: "right 1.35rem center",
  backgroundRepeat: "no-repeat",
  backgroundSize: "0.9rem",
};
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

const CatalogDeckRow = memo(function CatalogDeckRow({ entry, actionKey, isBusy, isCopying, isCopied, targetName, onSelect, onCopy }) {
  const ui = useUiText();
  const [artUrl, setArtUrl] = useState("");
  const manaProfile = completeManaProfile(entry);
  const colors = (manaProfile?.colors || []).filter((color) => /^[WUBRGC]$/.test(color));
  const predominantColors = manaProfile?.predominantColors || [];
  const predominantLands = (manaProfile?.predominantLands || []).slice(0, 2);
  const cardCounts = entry.sideboardCount
    ? ui("{0} cards + {1} SB", { 0: entry.mainboardCount || "?", 1: entry.sideboardCount })
    : ui("{0} cards", { 0: entry.mainboardCount || "?" });
  const summary = [
    entry.event || ui("Unknown event"),
    entry.date || ui("no date"),
    ...(entry.placement ? [`#${entry.placement}`] : []),
    cardCounts,
  ].join(" · ");

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
    <article className="flex min-w-0 items-center gap-2 rounded-sm border border-transparent bg-transparent p-2 transition-colors hover:border-[#9a7e52]/35 hover:bg-white/[0.03]">
      <div className="h-[72px] w-[52px] shrink-0 overflow-hidden rounded-sm bg-[#17130e]" aria-hidden="true">
        {artUrl ? <img className="h-full w-full object-cover" src={artUrl} alt="" loading="lazy" onError={(event) => { event.currentTarget.style.display = "none"; }} /> : null}
      </div>
      <div className="min-w-0 flex-1">
        <div className="truncate text-[12px] font-bold text-[#e7d9bc]">{entry.name || entry.archetype || ui("Unnamed deck")}</div>
        <div className="truncate text-[10px] text-[#b8aa8e]">{summary}</div>
        <div className="flex min-w-0 items-center gap-1 truncate text-[10px] text-[#8b806b]">
          {colors.length ? <span className="inline-flex shrink-0 items-center gap-0.5" aria-label={ui("Colors: {0}", { 0: colors.join(", ") })} title={ui("Predominant mana: {0}", { 0: predominantColors.join(", ") || ui("none") })}>
            {colors.map((color) => <ManaSymbol key={color} sym={color} size={12} />)}
          </span> : null}
          <span className="shrink-0 uppercase">{entry.format || "modern"}</span>
          {manaProfile ? <span className="shrink-0">{`· ${ui("{0} lands", { 0: manaProfile.landCount })}`}</span> : null}
          {predominantLands.length ? <span className="truncate" title={ui("Predominant lands: {0}", { 0: predominantLands.map(({ name, count }) => `${name} (${count})`).join(", ") })}>{`· ${predominantLands.map(({ name, count }) => `${name} ${count}`).join(", ")}`}</span> : null}
          {(entry.mechanics || []).length ? <span className="truncate">· {(entry.mechanics || []).join(" · ")}</span> : null}
        </div>
      </div>
      <div className="flex shrink-0 flex-col gap-1">
        <Button type="button" variant="ghost" size="sm" className="h-7 w-[86px] max-w-[86px] truncate border border-[#9a7e52]/55 px-1 text-[10px] font-bold uppercase tracking-wide text-[#d8bf7a]" disabled={isBusy || isCopying} onClick={() => onSelect(entry, actionKey)} title={targetName ? ui("Use in {0}", { 0: targetName }) : ui("Use deck")}>
          {isBusy ? <ActionSpinner /> : ui("Use")}
        </Button>
        <Button type="button" variant="ghost" size="sm" className="h-7 w-[86px] max-w-[86px] truncate border border-white/15 px-1 text-[10px] font-semibold text-[#b8aa8e]" disabled={isBusy || isCopying} onClick={() => onCopy(entry, actionKey)} title={ui("Copy the list in MTGO format")}>
          {isCopying ? <ActionSpinner /> : isCopied ? ui("Copied") : "MTGO"}
        </Button>
      </div>
    </article>
  );
});

// The catalog scrolls vertically next to the player editors, so every
// collection is a titled run of rows in one scroll container rather than its
// own horizontal carousel.
const DeckGroup = memo(function DeckGroup({ title, entries, busyId, copyingId, copiedId, targetName, onSelect, onCopy }) {
  if (!entries.length) return null;
  return (
    <section className="grid gap-px" aria-label={title} data-deck-group={title}>
      <h3 className="sticky top-0 z-10 flex items-baseline gap-1.5 bg-[#0d0f10] px-1 py-1 text-[10px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">
        {title}
        <span className="text-[9px] font-semibold text-[#8b806b]">{entries.length}</span>
      </h3>
      {entries.map((entry) => {
        const actionKey = `${title}-${entry.id}`;
        return (
          <CatalogDeckRow
            key={actionKey}
            entry={entry}
            actionKey={actionKey}
            isBusy={busyId === actionKey}
            isCopying={copyingId === actionKey}
            isCopied={copiedId === actionKey}
            targetName={targetName}
            onSelect={onSelect}
            onCopy={onCopy}
          />
        );
      })}
    </section>
  );
});

export default function CompetitiveDeckBrowser({ onSelect, targetName = "" }) {
  const ui = useUiText();
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
  const busyRef = useRef("");
  const copyingRef = useRef("");
  const listRef = useRef(null);
  const deferredQuery = useDeferredValue(query);
  // Narrowing the results while scrolled halfway down a long list leaves the
  // reader looking at whatever happens to be under the viewport, so every
  // filter change returns to the top of the list.
  const resetScroll = useCallback(() => {
    listRef.current?.scrollTo?.({ top: 0 });
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
        if (active) setError(loadError.message || ui("Could not load the catalog"));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [catalogFormat, ui]);

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

  const queryResults = useMemo(
    () => sortDeckEntries(manaFilteredResults, sortMode, usageCounts),
    [manaFilteredResults, sortMode, usageCounts],
  );

  const availableMana = useMemo(
    () => manaOptions.filter((color) => searchResults.some((entry) => (completeManaProfile(entry)?.colors || []).includes(color))),
    [searchResults],
  );

  const clearFilters = useCallback(() => {
    setActiveMana([]);
    setManaMatchMode("include");
    resetScroll();
  }, [resetScroll]);

  const toggleMana = useCallback((color) => {
    setActiveMana((current) => {
      if (!current.includes(color)) return [...current, color];
      const next = current.filter((active) => active !== color);
      if (!next.length) setManaMatchMode("include");
      return next;
    });
    resetScroll();
  }, [resetScroll]);

  const handleSelect = useCallback(async (entry, actionKey) => {
    if (busyRef.current) return;
    busyRef.current = actionKey;
    setBusyId(actionKey);
    setError("");
    try {
      const detail = await loadCatalogDeckDetail(entry, { format: catalogFormat });
      onSelect(importDeckCatalogEntry(detail));
    } catch (selectError) {
      setError(selectError.message || ui("Could not import this deck"));
    } finally {
      busyRef.current = "";
      setBusyId("");
    }
  }, [catalogFormat, onSelect, ui]);

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
      setError(copyError.message || ui("Could not copy the deck."));
    } finally {
      copyingRef.current = "";
      setCopyingId("");
    }
  }, [catalogFormat, ui]);

  return (
    <section className="flex min-h-0 flex-1 flex-col gap-2 bg-transparent" aria-label={ui("Decks")} data-deck-catalog="">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <h2 className="text-[13px] font-bold uppercase tracking-[0.16em] text-[#f2d9a3]">{ui("Decks")}</h2>
          <p className="text-[11px] text-[#b8aa8e]">{targetName ? ui("Search and use a deck for {0}.", { 0: targetName }) : ui("Search by archetype, card, event or color.")}</p>
        </div>
        <label className="grid gap-1 text-[10px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">{ui("Format")}
          <select className={selectClass.replace("w-full", "w-auto min-w-[136px]")} style={selectStyle} value={catalogFormat} onChange={(event) => { setCatalogFormat(event.target.value); resetScroll(); }}>
            {catalogFormats.map((formatOption) => <option key={formatOption.id} value={formatOption.id}>{formatOption.label}</option>)}
          </select>
        </label>
      </div>
      <input className={fieldClass} value={query} onChange={(event) => { setQuery(event.target.value); resetScroll(); }} placeholder={ui("Archetype, card or event")} aria-label={ui("Search the catalog")} />
      <div className="flex flex-wrap items-center gap-1.5" aria-label={ui("Catalog filters")}>
        {availableMana.map((color) => {
          const isActive = activeMana.includes(color);
          return <Button key={color} type="button" variant="ghost" size="sm" className={`h-7 w-8 max-w-8 rounded-full px-1 text-[10px] font-bold ${isActive ? "bg-[#342817] ring-1 ring-[#d8bf7a]/55" : "text-[#b8aa8e] hover:bg-white/5"}`} aria-label={ui("Filter by {0} mana", { 0: color })} aria-pressed={isActive} onClick={() => toggleMana(color)}><ManaSymbol sym={color} size={15} /></Button>;
        })}
        {activeMana.length ? <div className="flex items-center gap-0.5 rounded-full border border-white/10 p-0.5" aria-label={ui("Mana match mode")}>
          <Button type="button" variant="ghost" size="sm" className={`h-6 rounded-full px-2 text-[9px] font-bold uppercase tracking-wide ${manaMatchMode === "include" ? "bg-[#342817] text-[#f2d9a3] ring-1 ring-[#d8bf7a]/55 shadow-[0_0_9px_rgba(216,191,122,0.28)]" : "text-[#8b806b] hover:text-[#e7d9bc]"}`} aria-pressed={manaMatchMode === "include"} title={ui("Includes these colors, even if the deck uses others")} onClick={() => { setManaMatchMode("include"); resetScroll(); }}>{ui("Includes")}</Button>
          <Button type="button" variant="ghost" size="sm" className={`h-6 rounded-full px-2 text-[9px] font-bold uppercase tracking-wide ${manaMatchMode === "exact" ? "bg-[#342817] text-[#f2d9a3] ring-1 ring-[#d8bf7a]/55 shadow-[0_0_9px_rgba(216,191,122,0.28)]" : "text-[#8b806b] hover:text-[#e7d9bc]"}`} aria-pressed={manaMatchMode === "exact"} title={ui("Only these colors; C mana may be auxiliary")} onClick={() => { setManaMatchMode("exact"); resetScroll(); }}>{ui("Only these")}</Button>
        </div> : null}
        {activeMana.length ? <Button type="button" variant="ghost" size="sm" className="h-7 px-1.5 text-[10px] font-semibold text-[#8b806b] hover:text-[#e7d9bc]" onClick={clearFilters}>{ui("Clear")}</Button> : null}
        <label className="ml-auto flex items-center gap-1 text-[10px] font-bold uppercase tracking-wide text-[#b8aa8e]">{ui("Sort")}
          <select className="bg-transparent px-1 py-1 pr-8 text-[10px] text-[#e7d9bc]" style={{ ...selectStyle, backgroundPosition: "right 0.75rem center", backgroundSize: "0.75rem" }} value={sortMode} onChange={(event) => { setSortMode(event.target.value); resetScroll(); }}>
            <option value="recent">{ui("Most recent")}</option>
            <option value="placement">{ui("Best placement")}</option>
            <option value="usage">{ui("Most played")}</option>
          </select>
        </label>
      </div>
      {loading ? <p className="text-[12px] text-[#b8aa8e]">{ui("Loading index…")}</p> : null}
      {error ? <p className="text-[12px] text-red-300">{error}</p> : null}
      {!loading && !error && !manaFilteredResults.length ? <p className="text-[12px] text-[#b8aa8e]">{ui("No results for this search.")}</p> : null}
      <div ref={listRef} className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden pr-1" data-deck-catalog-list="">
        {deferredQuery.trim() ? (
          <DeckGroup title={ui("Results")} entries={queryResults} busyId={busyId} copyingId={copyingId} copiedId={copiedId} targetName={targetName} onSelect={handleSelect} onCopy={handleCopy} />
        ) : <>
          <DeckGroup title={ui("Mono-color")} entries={monoResults} busyId={busyId} copyingId={copyingId} copiedId={copiedId} targetName={targetName} onSelect={handleSelect} onCopy={handleCopy} />
          <DeckGroup title={ui("Last major events")} entries={majorResults} busyId={busyId} copyingId={copyingId} copiedId={copiedId} targetName={targetName} onSelect={handleSelect} onCopy={handleCopy} />
          <DeckGroup title={ui("Last 20 events")} entries={recentResults} busyId={busyId} copyingId={copyingId} copiedId={copiedId} targetName={targetName} onSelect={handleSelect} onCopy={handleCopy} />
        </>}
      </div>
    </section>
  );
}
