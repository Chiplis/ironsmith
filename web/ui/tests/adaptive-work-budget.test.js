import test from "node:test";
import assert from "node:assert/strict";
import { createAdaptiveWorkBudget } from "../src/lib/adaptive-work-budget.js";

// Drives a budget against a slice whose cost is `fixed + units * perUnit`, with
// simulated wall clock so the test is deterministic.
function drive({ fixedMs, perUnitMs, totalUnits, targetMs = 4, max = 4096, cap = 200_000 }) {
  let clock = 0;
  const budget = createAdaptiveWorkBudget({ initial: 8, max, targetMs, now: () => clock });
  let done = 0;
  let slices = 0;
  let worstSliceMs = 0;
  while (done < totalUnits && slices < cap) {
    let spent = 0;
    budget.run((units) => {
      spent = Math.min(units, totalUnits - done);
      const elapsed = fixedMs + spent * perUnitMs;
      clock += elapsed;
      worstSliceMs = Math.max(worstSliceMs, elapsed);
      done += spent;
      slices += 1;
    }, () => spent);
  }
  return { slices, wallMs: clock, worstSliceMs, saturated: budget.isSaturated(),
           idealMs: fixedMs + totalUnits * perUnitMs };
}

test("slices stay near the frame target when per-unit cost dominates", () => {
  const run = drive({ fixedMs: 0.2, perUnitMs: 0.002, totalUnits: 32768 });
  assert.ok(run.worstSliceMs <= 8, `worst slice ${run.worstSliceMs}ms should stay near target`);
  assert.ok(run.wallMs < run.idealMs * 2, `wall ${run.wallMs}ms vs ideal ${run.idealMs}ms`);
  assert.equal(run.saturated, false);
});

test("a fixed cost above the frame target grows the budget instead of collapsing it", () => {
  // The regression this guards: sizing from total elapsed time alone drove the
  // budget to 1 unit per slice, turning the job into `units * fixed` — for
  // these inputs roughly five minutes of work instead of a tenth of a second.
  const run = drive({ fixedMs: 10, perUnitMs: 0.002, totalUnits: 32768 });
  assert.ok(run.saturated, "a fixed cost above target must be reported as saturated");
  assert.ok(run.slices < 40, `expected few slices, got ${run.slices}`);
  assert.ok(run.wallMs < run.idealMs * 4, `wall ${run.wallMs}ms vs ideal ${run.idealMs}ms`);
  assert.ok(run.wallMs < 1000, `wall ${run.wallMs}ms should not blow up`);
});

test("work that needs one slice does not pay for extra ones", () => {
  const run = drive({ fixedMs: 1, perUnitMs: 0.002, totalUnits: 8 });
  assert.equal(run.slices, 1);
});

test("consumed units below the offered budget report the fixed cost, not a unit cost", () => {
  let clock = 0;
  const seen = [];
  const budget = createAdaptiveWorkBudget({
    initial: 64, now: () => clock, report: (row) => seen.push(row),
  });
  // A slice that settles after 2 units although 64 were offered: its whole
  // elapsed time is fixed cost and must not be attributed to those 2 units.
  budget.run(() => { clock += 3; }, () => 2);
  assert.equal(seen[0].spentUnits, 2);
  assert.ok(seen[0].fixedMs >= 3 - 1e-9, `fixed estimate ${seen[0].fixedMs} should absorb the slice`);
});
