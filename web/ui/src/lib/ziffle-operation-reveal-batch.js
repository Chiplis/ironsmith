// Scope this closure to one immutable ceremony and one hand-reveal operation.
// load must call the cryptographic verifier; unverified slots cannot be supplied.
export function createOperationRevealBatch(positions, deckCount, load) {
  const expected = new Set(positions);
  if (!Number.isSafeInteger(deckCount) || deckCount <= 0
    || [...expected].some(position => !Number.isSafeInteger(position) || position < 0 || position >= deckCount)) {
    throw new Error("Invalid ziffle reveal batch positions");
  }
  let pending;
  return async position => {
    if (!expected.has(position)) throw new Error("Position outside ziffle reveal batch");
    pending ||= Promise.resolve().then(load).then(reveals => {
      const byPosition = new Map();
      if (!Array.isArray(reveals)) throw new Error("Missing ziffle reveal batch");
      for (const reveal of reveals) {
        const key = Number(reveal.cardPosition);
        const slot = Number(reveal.originalSlot);
        if (reveal.cardPosition == null || reveal.originalSlot == null
          || !expected.has(key) || byPosition.has(key)
          || !Number.isSafeInteger(slot) || slot < 0 || slot >= deckCount) {
          throw new Error("Invalid ziffle reveal batch result");
        }
        byPosition.set(key, reveal);
      }
      if (byPosition.size !== expected.size) throw new Error("Incomplete ziffle reveal batch");
      return byPosition;
    });
    return (await pending).get(position);
  };
}
