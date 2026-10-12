// A persistent shuffled bag deals every card once per cycle. At a cycle
// boundary, recently seen cards go last so adjacent hands remain disjoint.
export function createCustomCardSampler({storage = null, random = Math.random} = {}) {
  const key = "ironsmith-custom-card-draw-bag-v1";
  let memory = null;
  return (cards, count = 7) => {
    if (!Array.isArray(cards) || !cards.length) throw new Error("There are no verified cards available yet");
    if (!Number.isInteger(count) || count < 0) throw new Error("Invalid draw count");
    const ids = cards.map((card, index) => String(card?.id ?? index));
    if (new Set(ids).size !== ids.length) throw new Error("Duplicate custom card IDs");
    const signature = JSON.stringify([...ids].sort());
    let state = memory;
    try { state = JSON.parse(storage?.getItem(key) || "null") || state; } catch { /* memory fallback */ }
    if (state?.signature !== signature || !Array.isArray(state.remaining)
      || !Array.isArray(state.recent) || new Set(state.remaining).size !== state.remaining.length
      || [...state.remaining, ...state.recent].some(id => !ids.includes(id))) {
      state = {signature, remaining: [], recent: []};
    }
    const selected = [];
    while (selected.length < count) {
      if (!state.remaining.length) {
        const cycle = [...ids];
        for (let i = cycle.length - 1; i > 0; i--) {
          const j = Math.floor(random() * (i + 1));
          [cycle[i], cycle[j]] = [cycle[j], cycle[i]];
        }
        const recent = new Set(state.recent);
        state.remaining = [...cycle.filter(id => !recent.has(id)), ...cycle.filter(id => recent.has(id))];
      }
      // Prefer the least recently seen remaining card when a small pool
      // cannot avoid the previous hand entirely.
      let index = state.remaining.findIndex(id => !state.recent.includes(id));
      if (index < 0) index = state.remaining.reduce((best, id, i) =>
        state.recent.indexOf(id) < state.recent.indexOf(state.remaining[best]) ? i : best, 0);
      const [id] = state.remaining.splice(index, 1);
      selected.push(id);
      state.recent = [...state.recent.filter(value => value !== id), id].slice(-count);
    }
    memory = state;
    try { storage?.setItem(key, JSON.stringify(state)); } catch { /* memory fallback */ }
    const byId = new Map(ids.map((id, i) => [id, cards[i]]));
    return selected.map(id => byId.get(id));
  };
}

let browserSampler;
export function sampleCustomCards(cards, count = 7, random) {
  if (random) return createCustomCardSampler({random})(cards, count);
  if (!browserSampler) {
    let storage = null;
    try { storage = globalThis.localStorage; } catch { /* private browsing */ }
    browserSampler = createCustomCardSampler({storage});
  }
  return browserSampler(cards, count);
}

// Runs as one worker request. A failed compilation or insertion restores the
// prior gameplay state and pending decisions. Compiled definitions remain
// cached in the engine's shared registry.
export async function replaceHandWithCustomCards(game, playerIndex, drafts) {
  if (!Array.isArray(drafts) || drafts.length !== 7) {
    throw new Error("Custom cards requires exactly seven card drafts");
  }
  const saved = game.createRuntimeSavepoint();
  try {
    for (const draft of drafts) {
      const preview = await game.previewCustomCard(draft);
      if (!preview?.canCreate) throw new Error("Custom card did not compile completely");
    }
    game.clearPlayerZoneForSetup(playerIndex, "hand");
    const ids = [];
    for (const draft of drafts) {
      ids.push(await game.createCustomCard({
        draft, playerIndex, zoneName: "hand", skipTriggers: true,
      }));
    }
    return ids;
  } catch (error) {
    game.restoreRuntimeSavepoint(saved);
    throw error;
  } finally {
    game.releaseRuntimeSavepoint(saved);
  }
}
