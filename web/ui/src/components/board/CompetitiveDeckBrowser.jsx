import { memo, useCallback, useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import useUiText from "@/i18n/useUiText";
import { Button } from "@/components/ui/button";
import { deckCatalogEntryToMtgoText, importDeckCatalogEntry } from "@/lib/deck-catalog-import";
import { loadCatalogDeckDetail, loadCatalogIndex, loadLocalCardArt, searchCatalogEntries } from "@/lib/catalog-client";
import { parseDeckList } from "@/lib/decklists";
import { ManaSymbol } from "@/lib/mana-symbols";

const fieldClass = "w-full bg-[#050607] px-3 py-2 text-[13px] text-[#e7d9bc] outline-none transition-colors focus:bg-[#101114] focus-visible:ring-1 focus-visible:ring-[#d8bf7a]/35";
const selectClass = `${fieldClass} pr-12`;
const selectStyle = {
  appearance: "none",
  backgroundImage: "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 20 20' fill='none' stroke='%23b8aa8e' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.8'%3E%3Cpath d='m5 7 5 5 5-5'/%3E%3C/svg%3E\")",
  backgroundPosition: "right 1.35rem center",
  backgroundRepeat: "no-repeat",
  backgroundSize: "0.9rem",
};
const FEATURED_COLLECTION = "last-major-events";
const FEATURED_SIZE = 3;
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

function entryColors(entry) {
  return (completeManaProfile(entry)?.colors || []).filter((color) => /^[WUBRGC]$/.test(color));
}

function isRecentEntry(entry, recentIds) {
  return entry?.collections?.includes("last-20-events") || recentIds.has(entry?.id);
}

function isMajorCollectionEntry(entry) {
  return entry?.collections?.includes(FEATURED_COLLECTION) || isMajorEntry(entry);
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
  const colors = entryColors(entry);
  if (manaMatchMode === "exact") {
    const coloredColors = colors.filter((color) => color !== "C");
    const requestedColored = activeMana.filter((color) => color !== "C");
    if (coloredColors.length !== requestedColored.length || !requestedColored.every((color) => coloredColors.includes(color))) return false;
  }
  return activeMana.every((color) => colors.includes(color));
}

// The synchronizer records the deck's most expensive nonland as `artCard`;
// a catalog written before that falls back to whatever name sorted first.
function deckArtName(entry) {
  return entry?.artCard || entry?.cardNames?.[0] || "";
}

function useCardArt(cardName) {
  const [artUrl, setArtUrl] = useState("");
  useEffect(() => {
    let active = true;
    loadLocalCardArt(cardName).then((url) => {
      if (active) setArtUrl(url);
    });
    return () => {
      active = false;
    };
  }, [cardName]);
  return artUrl;
}

// Not every card in the local corpus has art, so the tile is a card back
// rather than an empty hole.
const DeckArt = memo(function DeckArt({ entry, className }) {
  const artUrl = useCardArt(deckArtName(entry));
  return (
    <div className={`shrink-0 overflow-hidden bg-[#131418] ${className}`} aria-hidden="true" data-deck-art={artUrl ? "loaded" : "placeholder"}>
      {artUrl
        ? <img className="h-full w-full object-cover" src={artUrl} alt="" loading="lazy" onError={(event) => { event.currentTarget.style.display = "none"; }} />
        : (
          <svg viewBox="0 0 24 24" className="h-full w-full text-[#3a3226]" fill="none" stroke="currentColor" strokeWidth="1.2">
            <rect x="7" y="4.5" width="10" height="15" rx="1.5" />
            <path d="M9.5 8.5h5M9.5 12h5M9.5 15.5h3" strokeLinecap="round" />
          </svg>
        )}
    </div>
  );
});

const ManaPips = memo(function ManaPips({ colors, size = 13 }) {
  const ui = useUiText();
  if (!colors.length) return null;
  return (
    <span className="inline-flex shrink-0 items-center gap-0.5" aria-label={ui("Colors: {0}", { 0: colors.join(", ") })}>
      {colors.map((color) => <ManaSymbol key={color} sym={color} size={size} />)}
    </span>
  );
});

// The featured strip is the catalog's `last-major-events` collection: the decks
// that placed at the most recent majors, with their art rather than a row.
const FeaturedDeck = memo(function FeaturedDeck({ entry, isBusy, isCopying, isCopied, targetName, onSelect, onCopy }) {
  const ui = useUiText();
  const actionKey = `featured-${entry.id}`;
  return (
    <article className="flex min-w-0 flex-col overflow-hidden rounded-sm bg-[#131418]" data-featured-deck={entry.id}>
      <DeckArt entry={entry} className="h-[72px] w-full" />
      <div className="grid gap-1 p-1.5">
        <div className="truncate text-[13px] font-bold text-[#e7d9bc]" title={entry.name || entry.archetype}>{entry.name || entry.archetype || ui("Unnamed deck")}</div>
        <div className="truncate text-[10px] text-[#8b806b]">{entry.event || ui("Unknown event")}</div>
        <ManaPips colors={entryColors(entry)} />
        <div className="mt-1 flex gap-1">
          <Button type="button" variant="ghost" size="sm" className="h-7 flex-1 px-1 text-[10px] font-bold uppercase tracking-wide flat-button-gold" disabled={isBusy || isCopying} onClick={() => onSelect(entry, actionKey)} title={targetName ? ui("Use in {0}", { 0: targetName }) : ui("Use deck")}>
            {isBusy ? <ActionSpinner /> : ui("Use")}
          </Button>
          <Button type="button" variant="ghost" size="sm" className="h-7 flex-1 px-1 text-[10px] font-semibold flat-button" disabled={isBusy || isCopying} onClick={() => onCopy(entry, actionKey)} title={ui("Copy the list in MTGO format")}>
            {isCopying ? <ActionSpinner /> : isCopied ? ui("Copied") : ui("Copy MTGO")}
          </Button>
        </div>
      </div>
    </article>
  );
});

const CatalogDeckRow = memo(function CatalogDeckRow({ entry, actionKey, isBusy, isCopying, isCopied, targetName, onSelect, onCopy }) {
  const ui = useUiText();
  const manaProfile = completeManaProfile(entry);
  const predominantLands = (manaProfile?.predominantLands || []).slice(0, 2);
  const details = [
    entry.event || ui("Unknown event"),
    String(entry.format || "").toUpperCase(),
    entry.date || ui("no date"),
    ...(entry.placement ? [ui("place #{0}", { 0: entry.placement })] : []),
    entry.sideboardCount
      ? ui("{0} cards + {1} SB", { 0: entry.mainboardCount || "?", 1: entry.sideboardCount })
      : ui("{0} cards", { 0: entry.mainboardCount || "?" }),
    ...(manaProfile ? [ui("{0} lands", { 0: manaProfile.landCount })] : []),
    ...(predominantLands.length ? [predominantLands.map(({ name, count }) => `${name} ${count}`).join(", ")] : []),
  ].filter(Boolean).join(" · ");

  return (
    <article className="flex min-w-0 items-center gap-2 rounded-sm p-1.5 transition-colors hover:bg-[#131418]" data-deck-row={entry.id}>
      <DeckArt entry={entry} className="h-[40px] w-[56px] rounded-sm" />
      <div className="min-w-0 flex-1">
        <div className="truncate text-[12px] font-bold text-[#e7d9bc]">{entry.name || entry.archetype || ui("Unnamed deck")}</div>
        <div className="flex min-w-0 items-center gap-1.5">
          <ManaPips colors={entryColors(entry)} size={11} />
          <span className="truncate text-[10px] text-[#8b806b]" title={details}>{details}</span>
        </div>
      </div>
      <div className="flex shrink-0 gap-1">
        <Button type="button" variant="ghost" size="sm" className="h-7 w-[72px] max-w-[72px] truncate px-1 text-[10px] font-bold uppercase tracking-wide flat-button-gold" disabled={isBusy || isCopying} onClick={() => onSelect(entry, actionKey)} title={targetName ? ui("Use in {0}", { 0: targetName }) : ui("Use deck")}>
          {isBusy ? <ActionSpinner /> : ui("Use")}
        </Button>
        <Button type="button" variant="ghost" size="sm" className="h-7 w-[94px] max-w-[94px] truncate px-1 text-[10px] font-semibold flat-button" disabled={isBusy || isCopying} onClick={() => onCopy(entry, actionKey)} title={ui("Copy the list in MTGO format")}>
          {isCopying ? <ActionSpinner /> : isCopied ? ui("Copied") : ui("Copy MTGO")}
        </Button>
      </div>
    </article>
  );
});

const SavedDeckRow = memo(function SavedDeckRow({ preset, isBusy, onSelect, targetName }) {
  const ui = useUiText();
  return (
    <article className="flex min-w-0 items-center gap-2 rounded-sm p-1.5 transition-colors hover:bg-[#131418]" data-saved-deck={preset.key}>
      <div className="flex h-[40px] w-[56px] shrink-0 items-center justify-center rounded-sm bg-[#131418] text-[10px] font-bold uppercase tracking-wide text-[#6f6759]" aria-hidden="true">
        {ui("Session")}
      </div>
      <div className="min-w-0 flex-1">
        <div className="truncate text-[12px] font-bold text-[#e7d9bc]">{preset.name}</div>
        <div className="truncate text-[10px] text-[#8b806b]">{ui("{0} cards", { 0: preset.cardCount })} · {preset.playerName}</div>
      </div>
      <Button type="button" variant="ghost" size="sm" className="h-7 w-[72px] max-w-[72px] shrink-0 truncate px-1 text-[10px] font-bold uppercase tracking-wide flat-button-gold" disabled={isBusy} onClick={() => onSelect(preset)} title={targetName ? ui("Use in {0}", { 0: targetName }) : ui("Use deck")}>
        {ui("Use")}
      </Button>
    </article>
  );
});

export default function CompetitiveDeckBrowser({ onSelect, targetName = "", savedDecks = [] }) {
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
  const [activeTab, setActiveTab] = useState("catalog");
  const [collection, setCollection] = useState("all");
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

  // The workspace owns the session's saved decks, so saving or deleting one
  // there refreshes this tab without the browser re-reading storage.
  const savedPresets = useMemo(
    () => savedDecks.flatMap((preset) => (preset.texts || [])
      .map((text, playerIndex) => ({
        key: `${preset.name}:${playerIndex}`,
        name: preset.name,
        playerName: preset.playerNames?.[playerIndex] || ui("Player {0}", { 0: playerIndex + 1 }),
        cardCount: parseDeckList(text).length,
        text,
      }))
      .filter((preset) => preset.cardCount > 0)),
    [savedDecks, ui],
  );

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

  const collectionResults = useMemo(() => {
    if (collection === FEATURED_COLLECTION) return manaFilteredResults.filter(isMajorCollectionEntry);
    if (collection === "last-20-events") return manaFilteredResults.filter((entry) => isRecentEntry(entry, recentIds));
    if (collection === "mono-color") return manaFilteredResults.filter((entry) => entry?.collections?.includes("mono-color"));
    return manaFilteredResults;
  }, [collection, manaFilteredResults, recentIds]);

  const listedResults = useMemo(
    () => sortDeckEntries(collectionResults, sortMode, usageCounts),
    [collectionResults, sortMode, usageCounts],
  );

  const featuredResults = useMemo(
    () => sortDeckEntries(manaFilteredResults.filter(isMajorCollectionEntry), sortMode, usageCounts).slice(0, FEATURED_SIZE),
    [manaFilteredResults, sortMode, usageCounts],
  );

  const availableMana = useMemo(
    () => manaOptions.filter((color) => searchResults.some((entry) => entryColors(entry).includes(color))),
    [searchResults],
  );

  const collections = useMemo(() => [
    { id: "all", label: ui("All decks") },
    { id: FEATURED_COLLECTION, label: ui("Last major events") },
    { id: "last-20-events", label: ui("Last 20 events") },
    { id: "mono-color", label: ui("Mono-color") },
  ], [ui]);

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

  const handleSelectSaved = useCallback((preset) => {
    onSelect({ deckText: preset.text, deckName: preset.name });
  }, [onSelect]);

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

  const showingSaved = activeTab === "saved";
  const visibleCount = showingSaved ? savedPresets.length : listedResults.length;

  return (
    <section className="flex min-h-0 flex-1 flex-col gap-1.5 bg-transparent" aria-label={ui("Browse decks")} data-deck-catalog="">
      <div className="flex items-center gap-3">
        <h2 className="shrink-0 text-[13px] font-bold uppercase tracking-[0.16em] text-[#f2d9a3]">{ui("Browse decks")}</h2>
        <label className="relative min-w-0 flex-1">
          <span className="sr-only">{ui("Search the catalog")}</span>
          <svg viewBox="0 0 20 20" className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[#8b806b]" aria-hidden="true"><circle cx="9" cy="9" r="5.5" fill="none" stroke="currentColor" strokeWidth="1.6" /><path d="m13.5 13.5 3 3" fill="none" stroke="currentColor" strokeLinecap="round" strokeWidth="1.6" /></svg>
          <input className={`${fieldClass} pl-8`} value={query} onChange={(event) => { setQuery(event.target.value); resetScroll(); }} placeholder={ui("Search by name, archetype, card, event…")} aria-label={ui("Search the catalog")} />
        </label>
      </div>
      <div className="flex items-end gap-3">
        <label className="grid gap-1 text-[10px] font-bold uppercase tracking-[0.16em] text-[#8b806b]">{ui("Format")}
          <select className={selectClass.replace("w-full", "w-auto min-w-[136px]")} style={selectStyle} value={catalogFormat} onChange={(event) => { setCatalogFormat(event.target.value); resetScroll(); }}>
            {catalogFormats.map((formatOption) => <option key={formatOption.id} value={formatOption.id}>{formatOption.label}</option>)}
          </select>
        </label>
        {availableMana.length ? <div className="grid min-w-0 gap-1 text-[10px] font-bold uppercase tracking-[0.16em] text-[#8b806b]">{ui("Colors")}
          <div className="flex flex-wrap items-center gap-1" aria-label={ui("Catalog filters")}>
            {availableMana.map((color) => {
              const isActive = activeMana.includes(color);
              return <Button key={color} type="button" variant="ghost" size="sm" className={`h-8 w-8 max-w-8 rounded-full px-1 transition-colors ${isActive ? "bg-[#3d2f1c]" : "hover:bg-[#17181b]"}`} aria-label={ui("Filter by {0} mana", { 0: color })} aria-pressed={isActive} onClick={() => toggleMana(color)}><ManaSymbol sym={color} size={17} /></Button>;
            })}
            {activeMana.length ? <Button type="button" variant="ghost" size="sm" className="h-7 px-1.5 text-[10px] font-semibold text-[#6f6759] transition-colors hover:text-[#e7d9bc]" onClick={clearFilters}>{ui("Clear")}</Button> : null}
          </div>
        </div> : null}
        <label className="grid shrink-0 gap-1 text-[10px] font-bold uppercase tracking-[0.16em] text-[#8b806b]">{ui("Sort")}
          <select className={selectClass.replace("w-full", "w-auto min-w-[140px]")} style={selectStyle} value={sortMode} onChange={(event) => { setSortMode(event.target.value); resetScroll(); }}>
            <option value="recent">{ui("Most recent")}</option>
            <option value="placement">{ui("Best placement")}</option>
            <option value="usage">{ui("Most played")}</option>
          </select>
        </label>
      </div>
      {activeMana.length ? <div className="flex items-center gap-0.5 self-start rounded-full bg-[#0e0f11] p-0.5" aria-label={ui("Mana match mode")}>
        <Button type="button" variant="ghost" size="sm" className={`h-6 rounded-full px-2 text-[9px] font-bold uppercase tracking-wide ${manaMatchMode === "include" ? "bg-[#3d2f1c] text-[#f2d9a3]" : "text-[#6f6759] hover:text-[#e7d9bc]"}`} aria-pressed={manaMatchMode === "include"} title={ui("Includes these colors, even if the deck uses others")} onClick={() => { setManaMatchMode("include"); resetScroll(); }}>{ui("Includes")}</Button>
        <Button type="button" variant="ghost" size="sm" className={`h-6 rounded-full px-2 text-[9px] font-bold uppercase tracking-wide ${manaMatchMode === "exact" ? "bg-[#3d2f1c] text-[#f2d9a3]" : "text-[#6f6759] hover:text-[#e7d9bc]"}`} aria-pressed={manaMatchMode === "exact"} title={ui("Only these colors; C mana may be auxiliary")} onClick={() => { setManaMatchMode("exact"); resetScroll(); }}>{ui("Only these")}</Button>
      </div> : null}

      {!showingSaved && featuredResults.length ? (
        <section className="grid gap-1" aria-label={ui("Featured decks")} data-featured-decks="">
          <div className="flex items-baseline gap-2">
            <h3 className="shrink-0 text-[11px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">{ui("Featured decks")}</h3>
            <p className="truncate text-[10px] text-[#8b806b]">{ui("Decks from the last major events")}</p>
          </div>
          <div className="grid grid-cols-3 gap-2">
            {featuredResults.map((entry) => (
              <FeaturedDeck
                key={entry.id}
                entry={entry}
                isBusy={busyId === `featured-${entry.id}`}
                isCopying={copyingId === `featured-${entry.id}`}
                isCopied={copiedId === `featured-${entry.id}`}
                targetName={targetName}
                onSelect={handleSelect}
                onCopy={handleCopy}
              />
            ))}
          </div>
        </section>
      ) : null}

      <div className="flex flex-wrap items-center gap-1">
        {[{ id: "catalog", label: ui("All decks") }, { id: "saved", label: ui("My decks") }].map((tab) => (
          <button
            key={tab.id}
            type="button"
            className={`rounded-sm px-2 py-1 text-[11px] font-bold uppercase tracking-[0.14em] transition-colors ${activeTab === tab.id ? "bg-[#2e2416] text-[#f2d9a3]" : "text-[#6f6759] hover:bg-[#17181b] hover:text-[#e7d9bc]"}`}
            aria-pressed={activeTab === tab.id}
            data-catalog-tab={tab.id}
            onClick={() => { setActiveTab(tab.id); resetScroll(); }}
          >{tab.label}</button>
        ))}
        <span className="ml-auto text-[10px] uppercase tracking-wide text-[#6f6759]">{ui("{0} decks", { 0: visibleCount })}</span>
      </div>
      {!showingSaved ? (
        <div className="flex flex-wrap gap-1" aria-label={ui("Collections")}>
          {collections.map((option) => (
            <button
              key={option.id}
              type="button"
              className={`rounded-full px-2.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide transition-colors ${collection === option.id ? "bg-[#3d2f1c] text-[#f2d9a3]" : "bg-[#131418] text-[#6f6759] hover:bg-[#1d1e22] hover:text-[#e7d9bc]"}`}
              aria-pressed={collection === option.id}
              data-collection={option.id}
              onClick={() => { setCollection(option.id); resetScroll(); }}
            >{option.label}</button>
          ))}
        </div>
      ) : null}

      {loading && !showingSaved ? <p className="text-[12px] text-[#b8aa8e]">{ui("Loading index…")}</p> : null}
      {error && !showingSaved ? <p className="text-[12px] text-red-300">{error}</p> : null}
      {!loading && !error && !visibleCount ? (
        <p className="text-[12px] text-[#b8aa8e]">{showingSaved ? ui("You have no saved decks in this session.") : ui("No results for this search.")}</p>
      ) : null}
      <div ref={listRef} className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden pr-1" data-deck-catalog-list="">
        {showingSaved
          ? savedPresets.map((preset) => (
            <SavedDeckRow key={preset.key} preset={preset} isBusy={Boolean(busyId)} targetName={targetName} onSelect={handleSelectSaved} />
          ))
          : listedResults.map((entry) => (
            <CatalogDeckRow
              key={entry.id}
              entry={entry}
              actionKey={entry.id}
              isBusy={busyId === entry.id}
              isCopying={copyingId === entry.id}
              isCopied={copiedId === entry.id}
              targetName={targetName}
              onSelect={handleSelect}
              onCopy={handleCopy}
            />
          ))}
      </div>
    </section>
  );
}
