// Wall-clock measurements change scheduling only, never search limits/results.
export function createAdaptiveWorkBudget({ initial = 8, max = 128, targetMs = 4,
  now = () => performance.now(), report = () => {} } = {}) {
  let budget = Math.max(1, initial);
  return {
    run(work) {
      const used = budget;
      const started = now();
      try { return work(used); }
      finally {
        const elapsed = Math.max(0, now() - started);
        const scaled = elapsed > 0 ? Math.floor(used * targetMs / elapsed) : used * 2;
        budget = Math.max(1, Math.min(max, used * 2, scaled));
        report({ budget: used, elapsedMs: elapsed, nextBudget: budget });
      }
    },
  };
}
