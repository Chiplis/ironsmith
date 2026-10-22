import useUiText from "@/i18n/useUiText";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import MiniatureCardFrame from "@/components/cards/MiniatureCardFrame";
import { useScryfallImage } from "@/hooks/useScryfallImageUrl";
import { useTranslatedCardName } from "@/i18n/useTranslatedCardName";
import { placeCardPreview } from "@/lib/card-preview-placement";
import { parseCommanderList, parseDeckList, parseSideboardList } from "@/lib/decklists";
import { fetchCardFaceSource } from "@/lib/scryfall";

// A read-only view of the deck a player brings to a lobby seat: one row per
// unique card, grouped into commander / main deck / sideboard. Hovering or
// focusing a row floats the card's frame beside the list.

const PREVIEW_OPEN_DELAY_MS = 120;
const PREVIEW_HEIGHT = 420;
const PREVIEW_GAP = 12;
const VIEWPORT_INSET = 8;

// The parsers expand "4 Name" into four entries; fold them back into one
// counted row per name, in first-seen order.
function countedEntries(names) {
  const counts = new Map();
  for (const name of names) {
    if (!name) continue;
    counts.set(name, (counts.get(name) || 0) + 1);
  }
  return [...counts].map(([name, count]) => ({ name, count }));
}

function problemFor(cardProblems, name) {
  if (!cardProblems || !name) return "";
  if (Object.hasOwn(cardProblems, name)) return String(cardProblems[name] || "");
  const lower = name.toLowerCase();
  const key = Object.keys(cardProblems).find((entry) => entry.toLowerCase() === lower);
  return key ? String(cardProblems[key] || "") : "";
}

// The same name-only frame the inspector uses for a known face-down card:
// printed characteristics are loaded by name, art from Scryfall.
function NamedCardFrame({ name }) {
  const [loaded, setLoaded] = useState({ name: "", card: null });
  useEffect(() => {
    let active = true;
    fetchCardFaceSource(name).then((card) => { if (active) setLoaded({ name, card: card || null }); }).catch(() => {});
    return () => { active = false; };
  }, [name]);
  const image = useScryfallImage(name, "art_crop");
  const source = loaded.name === name ? loaded.card : null;
  return <MiniatureCardFrame card={source || { name }} imageUrl={image.url} />;
}

function previewPlacement(list, row) {
  if (typeof window === "undefined" || !list || !row) return { visibility: "hidden" };
  const listRect = list.getBoundingClientRect();
  const rowRect = row.getBoundingClientRect();
  const bounds = {
    left: VIEWPORT_INSET,
    top: VIEWPORT_INSET,
    right: window.innerWidth - VIEWPORT_INSET,
    bottom: window.innerHeight - VIEWPORT_INSET,
  };
  const width = (PREVIEW_HEIGHT * 63) / 88;
  const roomRight = bounds.right - listRect.right - PREVIEW_GAP;
  const roomLeft = listRect.left - bounds.left - PREVIEW_GAP;
  const sideways = Math.max(roomRight, roomLeft) >= width * 0.6;
  // Beside the hovered row when there is room to the side; otherwise above
  // the list (narrow sheets), falling back to below it.
  const preferred = sideways
    ? {
      left: roomRight >= roomLeft ? listRect.right + PREVIEW_GAP : listRect.left - PREVIEW_GAP - width,
      top: rowRect.top + rowRect.height / 2 - PREVIEW_HEIGHT / 2,
    }
    : { left: listRect.left + listRect.width / 2 - width / 2, top: listRect.top - PREVIEW_GAP - PREVIEW_HEIGHT };
  return placeCardPreview({
    preferred,
    height: PREVIEW_HEIGHT,
    bounds,
    obstacles: [listRect],
    gap: PREVIEW_GAP,
  });
}

function CardRow({ entry, problem, active, onPreview, onLeave }) {
  const ui = useUiText();
  const displayName = useTranslatedCardName(entry.name);
  const reason = problem ? ui(problem) : "";
  return (
    <li>
      <button
        type="button"
        className={`lobby-deck-card-list__row${problem ? " is-problem" : ""}${active ? " is-active" : ""}`}
        data-card-name={entry.name}
        title={reason || undefined}
        onPointerEnter={(event) => { if (event.pointerType !== "touch") onPreview(entry.name, event.currentTarget, true); }}
        onPointerLeave={(event) => { if (event.pointerType !== "touch") onLeave(entry.name); }}
        onFocus={(event) => onPreview(entry.name, event.currentTarget, false)}
        onBlur={() => onLeave(entry.name)}
        onClick={(event) => onPreview(entry.name, event.currentTarget, false, true)}
      >
        <span className="lobby-deck-card-list__count">{entry.count}</span>
        <span className="lobby-deck-card-list__name">{displayName}</span>
        {problem ? <span className="lobby-deck-card-list__problem">{reason}</span> : null}
      </button>
    </li>
  );
}

export default function LobbyDeckCardList({ deckText, commanderText = "", format, cardProblems = {}, title }) {
  const ui = useUiText();
  const listRef = useRef(null);
  const rowRef = useRef(null);
  const timerRef = useRef(0);
  const [previewName, setPreviewName] = useState("");
  const [placement, setPlacement] = useState(null);

  const sections = useMemo(() => [
    { id: "commander", label: "Commander", entries: countedEntries(parseCommanderList(commanderText)) },
    { id: "main", label: "Main deck", entries: countedEntries(parseDeckList(deckText)) },
    { id: "sideboard", label: "Sideboard", entries: countedEntries(parseSideboardList(deckText)) },
  ].filter((section) => section.entries.length > 0), [commanderText, deckText]);

  const problemCount = useMemo(() => sections.reduce((total, section) => total
    + section.entries.filter((entry) => problemFor(cardProblems, entry.name)).length, 0), [cardProblems, sections]);

  const clearTimer = () => {
    if (timerRef.current) window.clearTimeout(timerRef.current);
    timerRef.current = 0;
  };

  const showPreview = useCallback((name, row, delayed, toggle = false) => {
    clearTimer();
    const open = () => {
      rowRef.current = row;
      setPreviewName((current) => (toggle && current === name ? "" : name));
    };
    if (delayed) timerRef.current = window.setTimeout(open, PREVIEW_OPEN_DELAY_MS);
    else open();
  }, []);

  const hidePreview = useCallback((name) => {
    clearTimer();
    setPreviewName((current) => (current === name ? "" : current));
  }, []);

  useEffect(() => () => clearTimer(), []);

  // Hide the preview when the deck changes under it.
  const knownNames = useMemo(() => new Set(sections.flatMap((section) => section.entries.map((entry) => entry.name))), [sections]);
  const visibleName = previewName && knownNames.has(previewName) ? previewName : "";

  useLayoutEffect(() => {
    if (!visibleName) return undefined;
    const update = () => setPlacement(previewPlacement(listRef.current, rowRef.current));
    update();
    const list = listRef.current;
    list?.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, { passive: true, capture: true });
    return () => {
      list?.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, { capture: true });
    };
  }, [visibleName]);

  const totalCards = sections.reduce((total, section) => total + section.entries.reduce((sum, entry) => sum + entry.count, 0), 0);

  return (
    <div className="lobby-deck-card-list" data-format={format || undefined}>
      <div className="lobby-deck-card-list__header">
        <span className="text-[10px] font-bold uppercase tracking-[0.16em] text-[#d8bf7a]">{title ?? ui("Decklist")}</span>
        <span className="text-[12px] text-muted-foreground">
          {ui("{count} cards", { count: totalCards })}
          {problemCount > 0 ? <span className="lobby-deck-card-list__problem-count">{" · "}{ui("{count} not legal", { count: problemCount })}</span> : null}
        </span>
      </div>
      <div ref={listRef} className="lobby-deck-card-list__scroll" role="region" aria-label={title ?? ui("Decklist")}>
        {sections.length === 0 ? (
          <p className="lobby-deck-card-list__empty text-[12px] text-muted-foreground">{ui("No cards yet")}</p>
        ) : sections.map((section) => (
          <section key={section.id} className="lobby-deck-card-list__section" data-section={section.id}>
            <h3 className="lobby-deck-card-list__section-heading">
              <span>{ui(section.label)}</span>
              <span className="lobby-deck-card-list__section-count">
                {section.entries.reduce((sum, entry) => sum + entry.count, 0)}
              </span>
            </h3>
            <ul className="lobby-deck-card-list__rows">
              {section.entries.map((entry) => (
                <CardRow
                  key={entry.name}
                  entry={entry}
                  problem={problemFor(cardProblems, entry.name)}
                  active={visibleName === entry.name}
                  onPreview={showPreview}
                  onLeave={hidePreview}
                />
              ))}
            </ul>
          </section>
        ))}
      </div>
      {visibleName && placement && typeof document !== "undefined" ? createPortal(
        <div className="lobby-deck-card-list__preview" style={placement} data-preview-card={visibleName} aria-hidden="true">
          <NamedCardFrame name={visibleName} />
        </div>,
        document.body,
      ) : null}
    </div>
  );
}
