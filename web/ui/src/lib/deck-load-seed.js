// The engine derives a deck load's shuffle seed from the player names, life
// and deck lists when none is given, so reloading the same deck would deal the
// same opening hand every time. Pick a fresh seed before the command is
// journaled: replicas replaying the journal still see the same seed.
export function withFreshDeckLoadSeed(payload, random = () => globalThis.crypto.getRandomValues(new Uint32Array(1))[0]) {
  const structured = Array.isArray(payload) ? { decks: payload } : payload;
  if (!structured || typeof structured !== "object" || structured.seed != null) return payload;
  return { ...structured, seed: random() };
}
