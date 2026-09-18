import { resolveScryfallCanonicalCardName } from "./scryfall.js";

function compactNames(names) {
  return [...new Set((names || []).map((name) => String(name || "").trim()).filter(Boolean))];
}

// The engine registry is the source of truth for mutations. Scryfall is only a
// multilingual identity resolver; it must never cause an unembedded card to be
// injected or replace a card with a similarly named face.
export async function resolveCardNameForGame({ game, cardName, locale, resolveExternal = resolveScryfallCanonicalCardName }) {
  const requestedName = String(cardName || "").trim();
  if (!requestedName) return { status: "empty" };
  if (!game || typeof game.filterKnownCardNames !== "function") {
    return { status: "registry-unavailable", requestedName };
  }

  const direct = compactNames(await game.filterKnownCardNames([requestedName]));
  if (direct.length > 0) {
    return { status: "available", requestedName, canonicalName: direct[0], source: "registry" };
  }

  const external = await resolveExternal(requestedName, locale);
  if (!external?.canonicalName) return { status: "not-found", requestedName };

  // A face name is a lookup/display alias only. It must never make a distinct
  // whole card addable (for example Cheerful Osteomancer // Raise Dead must
  // not silently become the unrelated single-card sorcery Raise Dead).
  const known = compactNames(await game.filterKnownCardNames([external.canonicalName]));
  if (known.length > 0) {
    return {
      status: "available",
      requestedName,
      canonicalName: known[0],
      oracleId: external.oracleId || "",
      source: "localized",
    };
  }
  return {
    status: "not-embedded",
    requestedName,
    canonicalName: external.canonicalName,
    oracleId: external.oracleId || "",
  };
}
