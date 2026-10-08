# Final re-review amendment — source-only clearance

Final reviewed head: `33f8ba78cd99b466c85806c33cdef4ce88a03636`
Base remains: `ff30f190c72b80b9c212d30056d402e94d4f2c86`
Re-reviewed delta from: `afd864562b0e9f8dbb5f93b31f368ce811e3f658`
Date: 2026-10-07

**Bounded decision: clear for source-proposal eligibility. Both previous findings below are resolved.** No remaining source defect established within this packet's reviewed scope. This clearance is solely for reviewed source and complete authored UNRUN scenarios, never compilation, runtime validation, measured recovery, or admission.

The seventh test, `frozen_bioshift_returned_endpoint_is_not_rebound_to_its_new_incarnation`, independently obtains both strict definitions and iterates donor-return and recipient-return cases. It performs native battlefield → graveyard → battlefield moves, retains both returned ObjectIds, proves all incarnation IDs differ and stable identity remains, confirms the old object is absent, and retains the original declared stack target IDs. The `stable_id` field is public, and the native move implementation preserves it while creating a new ObjectId, so the new assertions match the actual API.

Each current returned incarnation receives seven +1/+1 and nine charge counters. Both original/native-cloned game branches then resolve through the native full-stack owner. Assertions require an empty stack, no amount decision, unchanged returned-incarnation counters, and unchanged +1/+1/charge counters on the other live endpoint. These checks make accidental rebinding observably wrong. The architecture lifecycle coverage statement now matches these authored cases. There are exactly seven test functions.

The delta changes only the evidence note and integration-test source; the worktree is clean. No build, test, probe, formatter, code generation, corpus execution, remote write, or production edit was performed during either review. Earlier positive source findings remain applicable.

---

# Historical first review (findings resolved by final head above)

# Bioshift independent source review

Reviewed commit: `afd864562b0e9f8dbb5f93b31f368ce811e3f658`
Base: `ff30f190c72b80b9c212d30056d402e94d4f2c86`
Date: 2026-10-07

## Decision

Source proposal needs one bounded coverage revision before complete source-only clearance: explicitly author new-incarnation non-rebinding scenarios for both endpoint roles. No production defect was established. This is not runtime validation, measured recovery, or admission; all scenarios remain UNRUN / UNVALIDATED.

## Findings

1. **Missing new-incarnation scenarios.** `crates/ironsmith-compiler-runtime/tests/bioshift_source_gate.rs`, `frozen_bioshift_rechecks_departure_and_current_controllers_at_resolution` (lines 137–164), moves each endpoint to the graveyard but discards the returned ID and never returns that card to the battlefield. Consequently these cases cover absent old targets, not reappearing physical cards with new object identities. The native zone owner (`crates/ironsmith-engine/src/game_state/zones_and_characteristics.rs:1290–1323`) creates a new ID on each move and clears counters. Author a leave-and-return scenario for each role; retain and assert new IDs, seed distinct current-incarnation counters, resolve original stack assignments, and assert no number prompt or movement and unchanged counters on both live endpoints. Repeat independently through both compilation paths and native clones. This is an authored-coverage gap, not an established runtime bug.
2. **Incorrect scenario-function count.** `architecture/bioshift-source-gate.md` says seven new test functions; this commit contains six. Fix the count or add the missing seventh function as part of the above revision.

## Bounded positive findings

- Diff is evidence-only: architecture note, dedicated fixture, and one integration-test source. No production, schema, protocol, ledger, or generated file edits.
- Read the exact frozen row from `fixtures/card-failure-campaign/cards-20261003.json.xz`. The fixture matches its name, Oracle ID, printing ID, `{G/U}` cost, Instant type, complete Oracle sentence, colors, and color identity. The body has no omitted trailing ability.
- Direct compilation and artifact compilation are distinct calls with `allow_unsupported=false` and independent parse-loss captures. The accompanying artifact-runtime value is discarded. The retained artifact is JSON-roundtripped, validated, compared, and explicitly materialized; both definitions reject unimplemented content.
- Public cast announcement, explicit two-role target submission, native pending state clones, and real payment are authored. Inspected current payment APIs: `apply_decision_context_with_dm` handles `ManaPayment`; the default `decide_mana_payment` offers confirmation at index 1 first, and `SelectFirstDecisionMaker` selects that option and returns the current plan ID/request hash. Thus this does not suffer a default-cancel payment mismatch. Green payment for the hybrid cost is authored; blue-only payment is not claimed or separately covered.
- Target grouping batches these same-chooser roles. The controller relation builds allowed pairs with distinctness and binds to the donor controller. Existing source supports the same-target and different-controller rejection expectations.
- Zero, partial, and all allocations, opposing-player endpoint control, and untouched charge counters are authored. Current source chooses an amount bounded by available donor counters before mutation and skips empty donors.
- Resolution source revalidates assigned endpoint roles using the current donor controller; changed donor, changed recipient, both changed, and departures have coherent authored expectations. Both controllers changing together remains a matching legal pair.
- Pending amount-choice scenario enters the native stack-resolution owner; that owner checkpoints the game and restores on pending choice or error. The authored checks retain stack assignments, counters, one-shot replacement, and already-paid casting mana. Both native branches retry through full stack resolution. Placement-only matcher supports removal of one and placement of two.
- Empty donor and zero amount produce no placement proposal, consistent with retaining the one-shot replacement.
- Overflow expectation is grounded in checked object-counter addition (`effects/counters/object_counter_placement.rs:203`). The movement owner wraps removal plus placement transactionally, and the full stack owner restores on error. A zero-choice native retry has coherent no-mutation expectations.

## Method and limits

Only source and documentation reading, frozen-row text extraction with xz/jq, Git inspection, and authoring this review were performed. No build, test, compiler probe, formatter, code generation, corpus execution, remote write, or production edit was performed. Source consistency cannot establish compilation or runtime success. Reported coverage revision to the parent and packet author before completing this report.
