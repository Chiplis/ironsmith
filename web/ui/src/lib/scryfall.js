export function scryfallImageUrl(cardName, version = "normal") {
  const query = String(cardName || "").trim();
  if (!query) return "";
  const params = new URLSearchParams({
    fuzzy: query,
    format: "image",
    version,
  });
  return `https://api.scryfall.com/cards/named?${params.toString()}`;
}

const namedCardMetaCache = new Map();

export async function fetchScryfallCardMeta(cardName) {
  const query = String(cardName || "").trim();
  if (!query) return { mana_cost: null };

  if (namedCardMetaCache.has(query)) {
    return namedCardMetaCache.get(query);
  }

  const request = (async () => {
    const params = new URLSearchParams({ exact: query });
    const exactResponse = await fetch(`https://api.scryfall.com/cards/named?${params.toString()}`);
    if (exactResponse.ok) {
      const card = await exactResponse.json();
      return { mana_cost: card?.mana_cost || null };
    }

    const fuzzyParams = new URLSearchParams({ fuzzy: query });
    const fuzzyResponse = await fetch(`https://api.scryfall.com/cards/named?${fuzzyParams.toString()}`);
    if (!fuzzyResponse.ok) {
      throw new Error(`Could not resolve Scryfall metadata for ${query}`);
    }
    const fuzzyCard = await fuzzyResponse.json();
    return { mana_cost: fuzzyCard?.mana_cost || null };
  })()
    .catch((error) => {
      namedCardMetaCache.delete(query);
      throw error;
    });

  namedCardMetaCache.set(query, request);
  return request;
}
