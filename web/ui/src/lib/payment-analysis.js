// Browser ranking owns an isolated worker. Native callers can use slices.
// User input invalidates the generation before any suggestion can be applied.
export async function improvePayment({ game, token, isCurrent, yieldTask = () => new Promise(resolve => setTimeout(resolve, 0)) }) {
  if (game.analyzePayment) {
    if (!isCurrent()) return null;
    const result = await game.analyzePayment(token);
    return isCurrent() ? result || null : null;
  }
  if (!isCurrent() || !await game.beginPaymentAnalysis(token)) return null;
  while (isCurrent()) {
    await yieldTask();
    if (!isCurrent()) break;
    const result = await game.stepPaymentAnalysis(token, 1);
    if (!isCurrent()) break;
    if (result !== null) return result || null;
  }
  return null;
}
