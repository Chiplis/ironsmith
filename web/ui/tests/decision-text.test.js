import test from "node:test";
import assert from "node:assert/strict";

import { normalizeDecisionText } from "../src/components/decisions/decisionText.js";

test("normalizes alternative casting labels into action-first language", () => {
  assert.equal(normalizeDecisionText("Normal: {2}{U}"), "Pay mana cost · {2}{U}");
  assert.equal(
    normalizeDecisionText("Cast without paying mana cost: Free"),
    "Cast for free"
  );
});

test("repairs generated article duplication without changing non-string values", () => {
  assert.equal(normalizeDecisionText("Sacrifice a another creature"), "Sacrifice another creature");
  assert.equal(normalizeDecisionText(null), null);
});
