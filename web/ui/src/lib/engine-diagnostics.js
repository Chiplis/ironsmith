// Reporting must remain available even while the engine worker is stuck.
export async function readEngineDiagnostics(game, timeoutMs = 250) {
  const methods = {
    dispatchPerf: 'lastDispatchPerf',
    snapshotPerf: 'lastSnapshotPerf',
    workCounters: 'lastWorkCounters',
    manaPaymentPerf: 'lastManaPaymentPerf',
    advanceUntilDecisionPerf: 'lastAdvanceUntilDecisionPerf',
    // Cards compile on demand, so registry size is a session-growth signal
    // rather than a constant. Read here so a bundle captured mid-stall still
    // carries the final value even if the journal's last samples were skipped.
    registrySize: 'registrySize',
    externalCardRoutes: 'getExternalCardRoutes',
  };
  const result = Object.fromEntries(Object.keys(methods).map(key => [key, null]));
  let expired = false;
  let timer;
  const reads = Promise.all(Object.entries(methods).map(async ([key, method]) => {
    try {
      const value = typeof game?.[method] === 'function' ? await game[method]() : null;
      if (!expired) result[key] = value;
    } catch { /* An older or failed worker may not expose this diagnostic. */ }
  }));
  try {
    await Promise.race([
      reads,
      new Promise(resolve => { timer = setTimeout(() => { expired = true; resolve(); }, timeoutMs); }),
    ]);
    return { ...result, timedOut: expired };
  } finally {
    clearTimeout(timer);
  }
}
