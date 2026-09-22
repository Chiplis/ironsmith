// Each slice is a separate worker request. User input invalidates the generation
// synchronously; an in-flight slice can finish but cannot publish its result.
export async function improvePayment({ game, token, isCurrent, yieldTask = () => new Promise(resolve => setTimeout(resolve, 0)) }) {
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
