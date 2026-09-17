// Wall-clock measurements change scheduling only, never search limits/results.
//
// A slice costs `fixed + units * perUnit`, where the fixed part is the menu
// re-enumeration that the unit budget cannot shrink. Sizing from total elapsed
// time alone inverts the control loop: once the fixed cost by itself reaches
// the frame target, every slice looks over budget, units collapse to 1, and the
// job costs `nodes * fixed` instead of `fixed + nodes * perUnit`. So the two
// costs are estimated separately from observations at different budget sizes,
// and when the fixed part dominates the only way to finish sooner is to put
// MORE units in a slice, not fewer.
export function createAdaptiveWorkBudget({ initial = 8, max = 4096, targetMs = 4,
  now = () => performance.now(), report = () => {} } = {}) {
  let budget = Math.max(1, initial);
  // Best (cheapest) slice time seen at each budget size. Cheapest rather than
  // latest because scheduler noise only ever inflates a sample.
  const samples = new Map();
  let fixedMs = 0;
  let perUnitMs = 0;
  let saturated = false;

  const record = (units, ms) => {
    const previous = samples.get(units);
    if (previous === undefined || ms < previous) samples.set(units, ms);
    if (samples.size > 8) samples.delete(samples.keys().next().value);
  };

  // Two-point fit across the widest spread of budget sizes observed.
  const fit = () => {
    if (samples.size < 2) return false;
    const points = [...samples.entries()].sort((a, b) => a[0] - b[0]);
    const [lowUnits, lowMs] = points[0];
    const [highUnits, highMs] = points[points.length - 1];
    if (highUnits <= lowUnits) return false;
    const slope = Math.max(0, (highMs - lowMs) / (highUnits - lowUnits));
    perUnitMs = slope;
    fixedMs = Math.max(0, Math.min(lowMs, lowMs - lowUnits * slope));
    return true;
  };

  return {
    // `work` receives the unit budget. `consumedUnits`, when supplied, reports
    // how many units the slice actually spent, so a slice that finished early
    // is not mistaken for a cheap one.
    run(work, consumedUnits) {
      const offered = budget;
      const started = now();
      try {
        return work(offered);
      } finally {
        const elapsed = Math.max(0, now() - started);
        const spent = consumedUnits ? Math.max(0, Math.min(offered, consumedUnits())) : offered;
        // Only a slice that used its whole budget says anything about the cost
        // of a unit. One that settled early bounds the fixed cost instead: its
        // elapsed time is overhead plus the few units it did spend.
        if (spent >= offered) {
          record(offered, elapsed);
        } else {
          const overhead = Math.max(0, elapsed - spent * perUnitMs);
          fixedMs = fixedMs === 0 ? overhead : Math.min(fixedMs, overhead);
        }
        const fitted = fit();
        let next;
        if (!fitted) {
          // Nothing to fit yet. Probe upward so a second operating point
          // exists, unless this slice already blew well past the target.
          next = elapsed > targetMs * 4 ? Math.max(1, Math.floor(offered / 2)) : offered * 2;
        } else {
          const headroomMs = targetMs - fixedMs;
          saturated = headroomMs <= 0;
          if (saturated) {
            // Slicing cannot make this responsive and each extra slice re-pays
            // the fixed cost, so finish in as few slices as possible.
            next = offered * 4;
          } else if (perUnitMs > 0) {
            next = Math.floor(headroomMs / perUnitMs);
          } else {
            next = offered * 2;
          }
        }
        budget = Math.max(1, Math.min(max, next));
        report({ budget: offered, elapsedMs: elapsed, spentUnits: spent,
                 fixedMs, perUnitMs, saturated, nextBudget: budget });
      }
    },
    // True when the fixed per-slice cost alone exceeds the frame target, so the
    // caller can stop yielding between slices and just finish the job.
    isSaturated: () => saturated,
    estimatedFixedMs: () => fixedMs,
  };
}
