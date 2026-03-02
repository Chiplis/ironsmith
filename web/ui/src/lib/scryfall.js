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
