import assets from "./custom-card-assets.generated.json" with { type: "json" };
import { resolveAssetUrl } from "./card-art-url.js";

const imported = new Map();
export function registerCustomCardAssets(cards) {
  for (const card of cards || []) for (const face of card.draft.faces) {
    if (!face.imageUrl && !face.attribution && !card.draft.attribution) continue;
    const meta = face.attribution || card.draft.attribution || {};
    imported.set(face.name.trim().toLowerCase(), {
      full: face.imageUrl || face.artUrl || '', art: face.artUrl || '',
      typeLine: [...(face.supertypes || []), ...(face.cardTypes || []), ...(face.subtypes?.length ? ['—', ...face.subtypes] : [])].join(' '),
      manaCost: face.manaCost, flavorText: face.flavorText, power: face.power, toughness: face.toughness,
      loyalty: face.loyalty, defense: face.defense, colorIndicator: face.colorIndicator,
      artist: meta.artist, set: meta.setCode, setName: meta.setName, collectorNumber: meta.collectorNumber,
      sourceUrl: meta.sourceUrl, customMechanics: card.draft.customMechanics || [],
    });
  }
}

// Callers restrict this catalog to names registered as custom cards, because
// a few designs intentionally reuse names of official cards.
export function customCardAssets(name) {
  const key = String(name || "").trim().toLowerCase();
  return imported.get(key) || assets[key] || null;
}

export function customCardImageUrl(name, version = "normal") {
  const entry = customCardAssets(name);
  return entry ? resolveAssetUrl(version === "art_crop" ? entry.art : entry.full) : "";
}
