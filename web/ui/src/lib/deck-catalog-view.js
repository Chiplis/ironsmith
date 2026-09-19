import { useEffect, useState } from "react";
import { loadLocalCardArt } from "./catalog-client.js";

// Shared between the deck workspace and the lobby's picker so a catalog deck
// reads the same wherever it is offered.
export function completeManaProfile(entry) {
  const profile = entry?.manaProfile;
  return profile?.metadataCoverage?.complete === true ? profile : null;
}

export function entryColors(entry) {
  return (completeManaProfile(entry)?.colors || []).filter((color) => /^[WUBRGC]$/.test(color));
}

// The synchronizer records the deck's most expensive nonland as `artCard`;
// a catalog written before that falls back to whatever name sorted first.
export function deckArtName(entry) {
  return entry?.artCard || entry?.cardNames?.[0] || "";
}

export function useCardArt(cardName) {
  const [artUrl, setArtUrl] = useState("");
  useEffect(() => {
    let active = true;
    loadLocalCardArt(cardName).then((url) => {
      if (active) setArtUrl(url);
    });
    return () => {
      active = false;
    };
  }, [cardName]);
  return artUrl;
}
