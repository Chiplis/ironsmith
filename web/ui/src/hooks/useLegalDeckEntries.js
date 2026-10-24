import { useEffect, useState } from "react";
import { loadCatalogDeckDetail } from "../lib/catalog-client.js";
import { deckCatalogEntryToLobbyTexts } from "../lib/deck-catalog-import.js";
import { splitSavedLobbyDeck } from "../lib/decklists.js";
import { prepareFormatDeck } from "../lib/prepare-format-deck.js";
import { loadFormatCatalog } from "../lib/relay/format-legality.js";

const EMPTY = new Map();

// Check full lists, not historical format labels or index card-name summaries.
export default function useLegalDeckEntries(entries, format, allowUnreleased) {
  const [result, setResult] = useState(null);
  useEffect(() => {
    if (!format) return;
    let active = true;
    async function check() {
      const legal = new Map();
      let error = "";
      let published = 0;
      const completed = new Map();
      const publish = () => {
        if (!active) return;
        const previous = published;
        while (completed.has(published)) {
          const { entry, prepared, failure } = completed.get(published);
          if (prepared) legal.set(entry, { ...entry, ...prepared });
          if (failure) error = failure;
          completed.delete(published++);
        }
        if (published !== previous) setResult({ entries, format, allowUnreleased, legal: new Map(legal), error, checking: published < entries.length });
      };
      try {
        const data = await loadFormatCatalog();
        let cursor = 0;
        await Promise.all(Array.from({ length: Math.min(8, entries.length) }, async () => {
          while (active && cursor < entries.length) {
            const index = cursor++;
            const entry = entries[index];
            try {
              const texts = entry.deckText != null || entry.text != null
                ? { ...splitSavedLobbyDeck(entry.deckText ?? entry.text), ...(entry.commanderText ? { commanderText: entry.commanderText } : {}) }
                : deckCatalogEntryToLobbyTexts(await loadCatalogDeckDetail(entry, { format }));
              // Always derive from the source entry; never feed the previously
              // cleaned list back in, so rule changes can restore removed cards.
              const prepared = prepareFormatDeck(format, texts, data, { allowUnreleased });
              completed.set(index, { entry, prepared });
            } catch (cause) {
              // Unavailable lists must not be offered as verified legal decks.
              completed.set(index, { entry, failure: cause.message });
            }
            publish();
          }
        }));
      } catch (cause) {
        error = cause.message;
      }
      if (active) setResult({ entries, format, allowUnreleased, legal: new Map(legal), error, checking: false });
    }
    check();
    return () => { active = false; };
  }, [entries, format, allowUnreleased]);
  if (!format) return { legal: null, checking: false, error: "" };
  // Gate during render: effect cleanup alone is too late to prevent a commit
  // containing results for the previous format or unreleased-card setting.
  const current = result?.entries === entries && result.format === format && result.allowUnreleased === allowUnreleased;
  return { legal: current ? result.legal : EMPTY, checking: !current || result.checking, error: current ? result.error : "" };
}
