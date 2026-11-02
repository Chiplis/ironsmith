import test from "node:test";
import assert from "node:assert/strict";
import { withinAdditionalTargetBudget } from "../src/lib/additional-target-budget.js";
test("extra target allowance is shared across existing occurrences", () => {
 const requirements=[{max_targets:2,additional_target_budget:1},{max_targets:2,additional_target_budget:1}];
 assert.equal(withinAdditionalTargetBudget(requirements,[[1],[2]]),true);
 assert.equal(withinAdditionalTargetBudget(requirements,[[1,2],[1]]),true);
 assert.equal(withinAdditionalTargetBudget(requirements,[[1],[1,2]]),true);
 assert.equal(withinAdditionalTargetBudget(requirements,[[1,2],[1,2]]),false);
});
test("zero original capacity and unlimited original capacity are distinct", () => {
 assert.equal(withinAdditionalTargetBudget([{max_targets:1,additional_target_budget:1}],[[1]]),true);
 assert.equal(withinAdditionalTargetBudget([{max_targets:null}],[[1,2,3]]),true);
 assert.equal(withinAdditionalTargetBudget([{max_targets:1}],[[1,2]]),false);
});
