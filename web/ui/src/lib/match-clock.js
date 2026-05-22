function finiteFloor(value, fallback = 0) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? Math.floor(numeric) : fallback;
}

function remainingAt(values, index, fallback = 0) {
  if (!Array.isArray(values)) return Math.max(0, finiteFloor(fallback, 0));
  return Math.max(0, finiteFloor(values[index], fallback));
}

export function isDisadvantageousActivePlayerClockAdvance({
  actorIndex,
  activePlayerIndex,
  elapsedMs,
  observedElapsedMs,
  previousRemainingMsByPlayer,
  submittedRemainingMsByPlayer,
  isTimeoutForfeit = false,
  skewMs = 0,
}) {
  const actor = Number(actorIndex);
  const activePlayer = Number(activePlayerIndex);
  if (
    !Number.isInteger(actor)
    || !Number.isInteger(activePlayer)
    || actor !== activePlayer
    || isTimeoutForfeit
  ) {
    return false;
  }

  const elapsed = Math.max(0, finiteFloor(elapsedMs, 0));
  const observed = Math.max(0, finiteFloor(observedElapsedMs, 0));
  const allowedSkew = Math.max(0, finiteFloor(skewMs, 0));
  if (elapsed <= observed + allowedSkew) return false;

  const previousRemaining = remainingAt(previousRemainingMsByPlayer, activePlayer, 0);
  const submittedRemaining = remainingAt(submittedRemainingMsByPlayer, activePlayer, previousRemaining);
  const expectedSubmittedRemaining = Math.max(0, previousRemaining - elapsed);
  const locallyObservedRemaining = Math.max(0, previousRemaining - observed);

  return (
    submittedRemaining === expectedSubmittedRemaining
    && submittedRemaining <= locallyObservedRemaining
  );
}
