import { normalizeCardList, text } from "../catalog-utils.mjs";

const EVENT_LINK_RE = /href=["']?\/?event\?e=(\d+)&f=([A-Za-z0-9]+)/gi;
const DECK_LINK_RE = /href=["']?\/?\?e=(\d+)&d=(\d+)&f=([A-Za-z0-9]+)/gi;
const CARD_LINE_RE = /<div\s+id=((?:md|sb)[^\s>]*)\s+class=["']deck_line[^"']*["'][^>]*>\s*(\d+)\s+<span[^>]*>([\s\S]*?)<\/span>/gi;
const ARCHETYPE_RE = /href=["']?\/?archetype\?[^"'>]*["'][^>]*>([^<]*?)\s+decks<\/a>/gi;
const EVENT_TITLE_RE = /<div\s+class=event_title[^>]*>\s*([\s\S]*?)<\/div>/gi;

function decodeHtml(value) {
  return text(value)
    .replace(/&amp;/g, "&")
    .replace(/&#39;|&apos;/g, "'")
    .replace(/&quot;/g, '"')
    .replace(/&nbsp;/g, " ")
    .replace(/<[^>]+>/g, "")
    .replace(/\\'/g, "'");
}

export function modernFormatUrl({ meta = 54, page = 0 } = {}) {
  const params = new URLSearchParams({ a: "", f: "MO", meta: String(meta) });
  if (Number(page) > 0) params.set("cp", String(page));
  return `https://mtgtop8.com/format?${params.toString()}`;
}

export function extractEventLinks(html, { limit = 25 } = {}) {
  const links = [];
  const seen = new Set();
  for (const match of String(html || "").matchAll(EVENT_LINK_RE)) {
    const id = match[1];
    const format = match[2].toLowerCase();
    const key = `${format}:${id}`;
    if (seen.has(key)) continue;
    seen.add(key);
    links.push({ id, format, url: `https://mtgtop8.com/event?e=${id}&f=${format.toUpperCase()}` });
    if (links.length >= limit) break;
  }
  return links;
}

export function extractDeckLinks(html, { eventId = "", limit = 25 } = {}) {
  const links = [];
  const seen = new Set();
  for (const match of String(html || "").matchAll(DECK_LINK_RE)) {
    if (eventId && match[1] !== String(eventId)) continue;
    const key = `${match[1]}:${match[2]}`;
    if (seen.has(key)) continue;
    seen.add(key);
    links.push({
      eventId: match[1],
      deckId: match[2],
      format: match[3].toLowerCase(),
      url: `https://mtgtop8.com/event?e=${match[1]}&d=${match[2]}&f=${match[3].toUpperCase()}`,
    });
    if (links.length >= limit) break;
  }
  return links;
}

export function parseDeckPage(html, {
  eventId = "",
  deckId = "",
  sourceUrl = "",
  date = "",
} = {}) {
  const mainboard = [];
  const sideboard = [];
  for (const match of String(html || "").matchAll(CARD_LINE_RE)) {
    const section = match[1].startsWith("sb") ? sideboard : mainboard;
    section.push({ name: decodeHtml(match[3]), count: Number(match[2]) });
  }

  const titles = [...String(html || "").matchAll(EVENT_TITLE_RE)].map((match) => decodeHtml(match[1]));
  const deckTitle = titles.find((title) => /^#\d+\s+/.test(title)) || "";
  const placementMatch = deckTitle.match(/^#(\d+)\s+/);
  const archetypeFromTitle = deckTitle
    .replace(/^#\d+\s+/, "")
    .replace(/\s+-\s+.*$/, "")
    .trim();
  const archetypeLinks = [...String(html || "").matchAll(ARCHETYPE_RE)]
    .map((match) => decodeHtml(match[1]))
    .filter(Boolean);
  const archetype = archetypeFromTitle || archetypeLinks.at(-1) || "";
  const event = titles.find((title) => !/^#\d+\s+/.test(title)) || "";

  return {
    id: deckId ? `mtgtop8-${eventId}-${deckId}` : "",
    format: "modern",
    name: archetype,
    archetype,
    event,
    date,
    placement: placementMatch ? Number(placementMatch[1]) : null,
    source: "mtgtop8",
    sourceUrl,
    mainboard: normalizeCardList(mainboard),
    sideboard: normalizeCardList(sideboard),
    commander: [],
    tags: placementMatch && Number(placementMatch[1]) <= 8 ? ["Top 8"] : [],
  };
}

export async function fetchText(url, {
  fetchImpl = globalThis.fetch,
  signal,
  minDelayMs = 750,
  now = () => Date.now(),
  sleep = (duration) => new Promise((resolve) => setTimeout(resolve, duration)),
  lastRequestAt = 0,
} = {}) {
  if (typeof fetchImpl !== "function") throw new Error("A fetch implementation is required");
  const remaining = Math.max(0, Number(minDelayMs) - (now() - Number(lastRequestAt || 0)));
  if (remaining) await sleep(remaining);
  const response = await fetchImpl(url, { signal, headers: { "user-agent": "IronSmith deck catalog sync" } });
  if (!response?.ok) throw new Error(`MTGTop8 request failed (${response?.status || "unknown"})`);
  return response.text();
}
