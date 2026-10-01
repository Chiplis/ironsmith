import test from "node:test";
import assert from "node:assert/strict";
import { paymentPreferences, selectPaymentSource, excludePaymentSource, removePaymentStep, paymentDraftRows, paymentTransactionKey } from "../src/lib/payment-draft.js";
const source = { source_id: "10", source_name: "Prism", ability_index: 1, color_restriction: ["blue"], payment_kind: "mana_ability" };
test("exact source choices preserve ability and output and replace exclusions", () => {
  const draft = selectPaymentSource(paymentPreferences({ excluded_source_ids: [10] }), source);
  assert.deepEqual(draft.required_activations, [{ source_id: "10", ability_index: 1, color_restriction: ["blue"] }]);
  assert.deepEqual(draft.excluded_source_ids, []);
  const changed = selectPaymentSource(draft, { ...source, color_restriction: ["red"] }, { replace: true });
  assert.equal(changed.required_activations.length, 1);
  assert.deepEqual(changed.required_activations[0].color_restriction, ["red"]);
  assert.deepEqual(excludePaymentSource(changed, 10).required_activations, []);
});
test("repeatable activations are distinct steps, and removing one preserves the others", () => {
  const repeatable = { ...source, repeatable: true };
  let draft = selectPaymentSource(paymentPreferences(), repeatable);
  draft = selectPaymentSource(draft, repeatable);
  const rows = paymentDraftRows({ planned_sources: [source, source] }, draft);
  assert.equal(rows.length, 2);
  assert.deepEqual(rows.map(row => row.occurrence), [0, 1]);
  draft = removePaymentStep(draft, rows[1], 1);
  assert.equal(draft.required_activations.length, 1);
  assert.deepEqual(draft.excluded_source_ids, []);
  assert.deepEqual(removePaymentStep(draft, source).excluded_source_ids, ["10"]);
});
test("keyword selections replace incompatible mana choices and remain visible when unfunded", () => {
  let draft = selectPaymentSource(paymentPreferences(), source);
  draft = selectPaymentSource(draft, { ...source, payment_kind: "convoke" });
  assert.deepEqual(draft.required_activations, []);
  assert.deepEqual(draft.required_alternatives, [{ source_id: "10", payment_kind: "convoke" }]);
  const rows = paymentDraftRows({ planned_sources: [], available_sources: [{ source_id: "10", source_name: "Helper", payment_kinds: ["convoke"] }] }, draft);
  assert.equal(rows[0].source_name, "Helper");
  assert.equal(rows[0].pinned, true);
  assert.equal(rows[0].pending, true);
});
test("removing an automatic suggestion excludes the source and does not resurrect it", () => {
  const draft = removePaymentStep(paymentPreferences(), source);
  assert.deepEqual(draft.excluded_source_ids, ["10"]);
  assert.deepEqual(paymentDraftRows({ planned_sources: [source] }, draft), []);
});
test("preference edits do not change payment transaction identity", () => {
  assert.equal(paymentTransactionKey({ transaction_id: "casting", request_hash: "before" }), paymentTransactionKey({ transaction_id: "casting", request_hash: "after" }));
});

test("changing a repeatable row replaces only its exact activation", () => {
  const blue = { ...source, repeatable: true };
  let draft = selectPaymentSource(selectPaymentSource(paymentPreferences(), blue), blue);
  const step = paymentDraftRows({ planned_sources: [blue, blue] }, draft)[1];
  assert.deepEqual(selectPaymentSource(draft, blue, { step }), draft);
  draft = selectPaymentSource(draft, { ...blue, color_restriction: ["red"] }, { step });
  assert.equal(draft.required_activations.length, 2);
  assert.deepEqual(draft.required_activations.map(value => value.color_restriction).sort(), [["blue"], ["red"]]);
  assert.equal(selectPaymentSource(draft, blue).required_activations.length, 3);
});
