import assets from "./custom-card-assets.generated.json" with { type: "json" };
import { resolveAssetUrl } from "./card-art-url.js";

// Callers restrict this catalog to names registered as custom cards, because
// a few designs intentionally reuse names of official cards.
export function customCardAssets(name) {
  return assets[String(name || "").trim().toLowerCase()] || null;
}

export function customCardImageUrl(name, version = "normal") {
  const entry = customCardAssets(name);
  return entry ? resolveAssetUrl(version === "art_crop" ? entry.art : entry.full) : "";
}
