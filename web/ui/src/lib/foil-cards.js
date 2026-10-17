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
// `persist: false` applies the set to this tab only (the startup demo board),
// leaving the deck's saved choice for the next lobby.
export function setFoilCards(names, { persist = true } = {}) {
  const next = new Set((names || []).map(foilKey).filter(Boolean));
  cached = next;
  if (!persist) {
    for (const listener of listeners) listener();
    return;
  }
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

function randomNames(cards, count) {
  const names = [...new Set((cards || []).map((card) => card?.name).filter(Boolean))];
  for (let index = names.length - 1; index > 0; index -= 1) {
    const swap = Math.floor(Math.random() * (index + 1));
    [names[index], names[swap]] = [names[swap], names[index]];
  }
  return names.slice(0, count);
}

// The startup demo board shows the foil finish on a few of the viewer's
// permanents and hand cards, picked at random, without touching the saved
// lobby choice.
export async function markStartupFoils(game, { battlefield = 3, hand = 2 } = {}) {
  if (typeof game?.uiState !== "function") return;
  const state = await game.uiState();
  const viewer = (state?.players || []).find((player) => Number(player?.id) === Number(state?.perspective));
  setFoilCards([
    ...randomNames(viewer?.battlefield, battlefield),
    ...randomNames(viewer?.hand_cards, hand),
  ], { persist: false });
}

export function useIsFoilCard(name) {
  return useSyncExternalStore(subscribe, () => readFoilCards().has(foilKey(name)), () => false);
}
