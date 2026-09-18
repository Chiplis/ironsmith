import { createHash } from "node:crypto";

export function text(value) {
  return String(value ?? "").trim();
}

export function slugify(value) {
  return text(value)
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLocaleLowerCase("en-US")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 80);
}

export function normalizeCardList(entries) {
  if (!Array.isArray(entries)) return [];

  const counts = new Map();
  for (const entry of entries) {
    const name = text(typeof entry === "string" ? entry : entry?.name);
    const count = Number(typeof entry === "string" ? 1 : entry?.count);
    if (!name || !Number.isInteger(count) || count < 1) continue;
    counts.set(name, (counts.get(name) || 0) + count);
  }

  return [...counts.entries()]
    .sort(([left], [right]) => left.localeCompare(right, "en"))
    .map(([name, count]) => ({ name, count }));
}

export function stableStringify(value) {
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

export function shortHash(value) {
  return createHash("sha256").update(stableStringify(value)).digest("hex").slice(0, 16);
}

export function deckTokenValues(deck) {
  return [
    deck.id,
    deck.format,
    deck.archetype,
    deck.event,
    deck.source,
    ...deck.tags,
    ...deck.mainboard.flatMap((card) => [card.name, card.name.split(/\s+/)]).flat(),
    ...deck.sideboard.flatMap((card) => [card.name, card.name.split(/\s+/)]).flat(),
  ]
    .map(text)
    .filter(Boolean);
}
