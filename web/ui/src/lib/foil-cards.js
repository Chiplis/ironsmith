import { useSyncExternalStore } from "react";

// Names the local player marked foil in the deck they submitted to a lobby.
// Like preferred printings, this is a local presentation choice: it is kept
// in this browser and never sent to the engine or the other players.
const FOIL_CARDS_STORAGE_KEY = "ironsmith.foilCards.v1";

const listeners = new Set();
let cached = null;

function foilKey(name) {
  return String(name || "").trim().toLowerCase();
}

function readFoilCards() {
  if (cached) return cached;
  let names = [];
  try {
    const raw = window.localStorage.getItem(FOIL_CARDS_STORAGE_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    if (Array.isArray(parsed)) names = parsed;
  } catch {
    names = [];
  }
  cached = new Set(names.map(foilKey).filter(Boolean));
  return cached;
}

// Replaces the set: each submitted deck decides which of its cards are foil.
export function setFoilCards(names) {
  const next = new Set((names || []).map(foilKey).filter(Boolean));
  cached = next;
  try {
    if (next.size) window.localStorage.setItem(FOIL_CARDS_STORAGE_KEY, JSON.stringify([...next]));
    else window.localStorage.removeItem(FOIL_CARDS_STORAGE_KEY);
  } catch {
    // Storage may be unavailable (private mode); the set still applies to this session.
  }
  for (const listener of listeners) listener();
}

// Another tab of the same browser may submit a deck: drop the cached set.
function subscribe(listener) {
  listeners.add(listener);
  const sync = (event) => {
    if (event.key !== FOIL_CARDS_STORAGE_KEY && event.key !== null) return;
    cached = null;
    listener();
  };
  window.addEventListener("storage", sync);
  return () => {
    listeners.delete(listener);
    window.removeEventListener("storage", sync);
  };
}

export function useIsFoilCard(name) {
  return useSyncExternalStore(subscribe, () => readFoilCards().has(foilKey(name)), () => false);
}
