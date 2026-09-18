import { normalizeCardList, text } from "../catalog-utils.mjs";

const EVENT_LINK_RE = /href=["']?\/?event\?e=(\d+)&f=([A-Za-z0-9]+)/gi;
const DECK_LINK_RE = /href=["']?\/?(?:event)?\?e=(\d+)&d=(\d+)&f=([A-Za-z0-9]+)/gi;
const CARD_LINE_RE = /<div\s+id=((?:md|sb)[^\s>]*)\s+class=["']deck_line[^"']*["'][^>]*>\s*(\d+)\s+<span[^>]*>([\s\S]*?)<\/span>/gi;
const ARCHETYPE_RE = /href=["']?\/?archetype\?[^"'>]*["'][^>]*>([^<]*?)\s+decks<\/a>/gi;
const EVENT_TITLE_RE = /<div\s+class=event_title[^>]*>\s*([\s\S]*?)<\/div>/gi;
const PLACEMENT_TITLE_RE = /^#(\d+)(?:-\d+)?\s+/;
const FORMAT_CODES = {
  standard: "ST",
  pioneer: "PI",
  modern: "MO",
  legacy: "LE",
  vintage: "VI",
  pauper: "PA",
  historic: "HI",
  commander: "EDH",
  edh: "EDH",
};

function decodeHtml(value) {
  return text(value)
    .replace(/&amp;/g, "&")
    .replace(/&#39;|&apos;/g, "'")
    .replace(/&quot;/g, '"')
    .replace(/&nbsp;/g, " ")
    .replace(/<[^>]+>/g, "")
    .replace(/\\'/g, "'");
}

function sourceDate(value) {
  const match = String(value || "").match(/^(\d{2})\/(\d{2})\/(\d{2})$/);
  return match ? `20${match[3]}-${match[2]}-${match[1]}` : "";
}

function sectionHtml(html, title) {
  const source = String(html || "");
  const start = source.search(new RegExp(`<div class=w_title[^>]*>\\s*${title}\\b`, "i"));
  if (start < 0) return "";
  const body = source.slice(start);
  const titleEnd = body.indexOf(">");
  if (titleEnd < 0) return "";
  const sectionBody = body.slice(titleEnd + 1);
  const nextSection = sectionBody.search(/<div class=w_title[^>]*>/i);
  return nextSection > 0 ? sectionBody.slice(0, nextSection) : sectionBody;
}

function extractSectionEvents(html, title, { limit = 25 } = {}) {
  const events = [];
  const seen = new Set();
  const section = sectionHtml(html, title);
  for (const rowMatch of section.matchAll(/<tr[^>]*class=hover_tr[^>]*>[\s\S]*?<\/tr>/gi)) {
    const row = rowMatch[0];
    const link = new RegExp(EVENT_LINK_RE.source, "i").exec(row);
    if (!link) continue;
    const id = link[1];
    const format = link[2].toLowerCase();
    if (seen.has(`${format}:${id}`)) continue;
    seen.add(`${format}:${id}`);
    const titleMatch = row.match(new RegExp(`href=["']?/?event\\?e=${id}&f=${format}[^>]*>([\\s\\S]*?)<\\/a>`, "i"));
    const dateMatch = row.match(/class=S12[^>]*>\s*(\d{2}\/\d{2}\/\d{2})\s*</i);
    events.push({
      id,
      format,
      name: decodeHtml(titleMatch?.[1] || ""),
      date: sourceDate(dateMatch?.[1]),
      url: `https://mtgtop8.com/event?e=${id}&f=${format.toUpperCase()}`,
    });
    if (events.length >= limit) break;
  }
  return events;
}

export function modernFormatUrl({ meta = 54, page = 0 } = {}) {
  return formatUrl({ format: "MO", meta, page });
}

export function normalizeFormat(value = "modern") {
  const token = text(value).toLocaleLowerCase("en-US");
  const code = FORMAT_CODES[token] || token.toUpperCase();
  const slug = Object.entries(FORMAT_CODES).find(([, formatCode]) => formatCode === code)?.[0] || token || "modern";
  return { slug, code };
}

export function formatUrl({ format = "modern", meta = 54, page = 0 } = {}) {
  const { code } = normalizeFormat(format);
  const params = new URLSearchParams({ a: "", f: code, meta: String(meta) });
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

export function extractEventCollections(html, { recentLimit = 20, majorLimit = 5 } = {}) {
  return {
    last20Events: extractSectionEvents(html, "LAST 20 EVENTS", { limit: recentLimit }),
    lastMajorEvents: extractSectionEvents(html, "LAST MAJOR EVENTS", { limit: majorLimit }),
  };
}

export function extractArchetypeLinks(html, { namePattern = /mono/i, limit = 12 } = {}) {
  const links = [];
  const seen = new Set();
  const pattern = /href=["']?\/?archetype\?([^"'>]+)["']?[^>]*>([^<]+)<\/a>/gi;
  for (const match of String(html || "").matchAll(pattern)) {
    const name = decodeHtml(match[2]).trim();
    if (!name || (namePattern && !namePattern.test(name))) continue;
    const query = match[1];
    const key = `${query}:${name.toLocaleLowerCase("en-US")}`;
    if (seen.has(key)) continue;
    seen.add(key);
    links.push({ name, url: `https://mtgtop8.com/archetype?${query}` });
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
  collections = [],
  format = "modern",
} = {}) {
  const mainboard = [];
  const sideboard = [];
  for (const match of String(html || "").matchAll(CARD_LINE_RE)) {
    const section = match[1].startsWith("sb") ? sideboard : mainboard;
    section.push({ name: decodeHtml(match[3]), count: Number(match[2]) });
  }

  const titles = [...String(html || "").matchAll(EVENT_TITLE_RE)].map((match) => decodeHtml(match[1]));
  const deckTitle = titles.find((title) => PLACEMENT_TITLE_RE.test(title)) || "";
  const placementMatch = deckTitle.match(PLACEMENT_TITLE_RE);
  const archetypeFromTitle = deckTitle
    .replace(PLACEMENT_TITLE_RE, "")
    .replace(/\s+-\s+.*$/, "")
    .trim();
  const archetypeLinks = [...String(html || "").matchAll(ARCHETYPE_RE)]
    .map((match) => decodeHtml(match[1]))
    .filter(Boolean);
  const archetype = archetypeFromTitle || archetypeLinks.at(-1) || "";
  const event = titles.find((title) => !PLACEMENT_TITLE_RE.test(title)) || "";

  return {
    id: deckId ? `mtgtop8-${eventId}-${deckId}` : "",
    format: normalizeFormat(format).slug,
    name: archetype,
    archetype,
    event,
    date,
    placement: placementMatch ? Number(placementMatch[1]) : null,
    source: "mtgtop8",
    sourceUrl,
    collections,
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
