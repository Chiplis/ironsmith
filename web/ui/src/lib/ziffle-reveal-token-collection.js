// A signer's proof for one position is independent of the other signers and
// positions. Reuse those proofs and start all remaining signer jobs together;
// cryptographic proof verification still belongs to the reveal backend.
export async function collectZiffleRevealTokenGroups({
  keys,
  positions,
  readCachedTokens,
  collectTokens,
  rememberTokens,
  onCacheHit,
}) {
  return Promise.all(keys.map(async (key) => {
    const cached = [];
    const missing = [];
    for (const position of positions) {
      const token = readCachedTokens(key, [position])?.[0];
      if (token) cached.push(token);
      else missing.push(position);
    }
    if (cached.length > 0) onCacheHit?.(key, cached);
    if (missing.length === 0) return cached;

    const response = await collectTokens(key, missing);
    // Older peers answer a single-position request with `token`, not `tokens`.
    const tokens = Array.isArray(response) ? response
      : missing.length === 1 && response && typeof response === "object" ? [response] : null;
    const expectedPositions = new Set(missing);
    const receivedPositions = new Set();
    if (!Array.isArray(tokens)) throw new Error("Missing ziffle reveal tokens");
    for (const token of tokens) {
      const position = Number(token?.cardPosition ?? (missing.length === 1 ? missing[0] : NaN));
      if (
        Number(token?.player) !== Number(key.player)
        || !expectedPositions.has(position)
        || receivedPositions.has(position)
      ) {
        throw new Error("Ziffle reveal tokens do not match the requested player and positions");
      }
      receivedPositions.add(position);
    }
    if (receivedPositions.size !== missing.length) {
      throw new Error("Missing ziffle reveal tokens for requested positions");
    }
    rememberTokens(tokens, missing);
    const complete = readCachedTokens(key, positions);
    if (!complete || complete.length !== positions.length) {
      throw new Error("Missing ziffle reveal tokens for requested positions");
    }
    return complete;
  }));
}
