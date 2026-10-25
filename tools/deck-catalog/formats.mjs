import { text } from "./catalog-utils.mjs";
import { ARCHIDEKT_FORMATS } from "./sources/archidekt.mjs";
import { normalizeFormat } from "./sources/mtgtop8.mjs";

const ALIASES = { pdh: "paupercommander", "pauper-commander": "paupercommander", "pauper-edh": "paupercommander" };

// Resolves a catalog format to its directory slug and the source that supplies
// its decks: MTGTop8 for tournament formats, Archidekt for the rest.
export function catalogFormat(value = "modern") {
  const token = text(value).toLocaleLowerCase("en-US");
  const slug = ALIASES[token] || token;
  if (ARCHIDEKT_FORMATS[slug]) return { slug, source: "archidekt" };
  return { ...normalizeFormat(value), source: "mtgtop8" };
}
