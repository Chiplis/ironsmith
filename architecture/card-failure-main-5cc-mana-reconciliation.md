# Current-main payment and local-analysis reconciliation

Status: source-only reconstruction on actual current main. All authored
scenarios are UNRUN. No build, test, compiler/engine probe, formatter, codegen,
browser/replay execution, publication, coverage-ledger or version-gate changes.
Commit titles in the user's history are not independent test results.

## Inputs and retained payload

The restored checkout was clean at main
`5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1`, tree
`2ef96adada1e40b7f39922f24a1e4788de1a7386`. It already includes published865
`489e6561d8b90ca088f2ba2ee481fec875cc3a46`, main's `355199dc` mana-payment
changes, their merge `8b991102`, and the user's three later commits:

- `ba39596a3b90af0412ac794a6b3d516f39c201a1`: Fix build.
- `eb9789141f24284493c2ca0cfe1bfdd9a454041c`: Fix engine unit tests.
- `5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1`: Fix integration tests.

The three user commits change 99 unique paths (+297/-217). The complete
published865-to-current-main delta changes 135 paths (+2,128/-749).
Reconstruction starts directly at current main. It does not replay the old
unpublished merge union or duplicate the user's changes.

The old unpublished `ff9e8357` objects were lost when the workspace was
replaced. Its five correction descriptions and authored source payloads were
retained in conversation and reconciled against actual current files:

1. **Already fixed by the user:** CostEffect passes the authoritative
   `outputs.outcome` to original-sacrifice binding. No production edit.
2. **Already fixed by the user:** BranchScope `run` and `project` borrow `&self`.
   This solves the non-Copy scope issue without the old clone edits. These
   improved APIs are retained unchanged.
3. **Still needed, restored:** bounded deferred planning propagates typed
   execution/resource/evidence failures. Only incomplete search and ordinary
   unpayability/preferences create an unfunded offer.
4. **Still needed, restored:** UnlessPaysProgram finalizes a successful Stop
   through its real consequence cursor and retains actual output packets,
   declarations, payment ownership and completion facts.
5. **Still needed, restored:** seeded local analysis keeps the exact image when
   all session-owned native savepoint slots are occupied. It drops the image
   only after capturing the canonical native reset branch successfully.

The user's merge also retains typed errors over pending choices and reserved
tap sources in the TotalCost gateway. The user's build fix already adapts
Waterbend's unit result into the rich receipt vector. Those owners were read
and left unchanged. Main's CostPaymentReceipt/pay_with_outputs architecture,
Published child ownership and exact captureLocal/restoreLocal APIs are kept.

Fresh source checkpoint:
`c66d9b585de3233949bf606d66ea31b6cdebdbb4`, tree
`18d3443d1d8222affce3e8893b3ef343cb3a1eed`.
Authored-scenario checkpoint:
`201adead87f0b7d2caa6fe850e7a2ed8b86173cd`, tree
`168447ae7777661265e7145de19248141cda791e`.

## Restored UNRUN scenarios

- Deferred foreground resource limits and overflowing Waterbend X remain typed
  errors without changing source tap, mana pool, object IDs or history.
- Direct and dispatched UnlessPays consequences retain a real life-gain prefix
  before Stop, skip the later effect, retain the same child event identity and
  record one history event.
- A seeded replica with full native branch capacity repeatedly resets its exact
  image plus later journal operations, retaining every session-owned handle.
- The existing original-sacrifice regression now uses pay_with_outputs and
  checks the retained original fact, exact source snapshot and one-time history
  for prevent, redirect, instead and additional replacement paths. The user's
  current Cost::try_effect and qualified ObjectFilter fixes are preserved.

All four authored payloads are reconstructed. There was no separate authored
regression for the old BranchScope clone repair; the current borrowed API makes
that repair unnecessary. Nothing here is a new card identity proposal.

## Current-main changes requiring evidence review

Published 1,256 is historical source evidence on its recorded base. Shared
owners changed, so old admission cannot be assumed automatically on current
main. This map names review boundaries, not a claim that all changes are defects
or that a particular number of identities has failed:

- **Prepared object iteration and TotalCost:** the user adds the new native
  TotalCost arm to IterationContinuation. Review nested payment phases,
  inherited payer/source/reason, one-time Published receipts, Stop and suspended
  replacement draws in combination with the new unless-payment program.
- **Text-changing programs and statics:** ChangeText now participates in result
  traversal and binds unresolved `it`; compiled core Landwalk receives its own
  subtype-word rewriter. These change live reference/rewrite owners for typed
  text changes, landwalk/blocking restrictions and related token/program state.
  The module/function visibility changes expose existing rewriting behavior.
- **Waterbend parsing and payment:** recognition moves from the effects reader
  into parse_payment_clause_as_total_cost in keyword_action_costs.rs. The
  interactive payment's unit result adapts to an empty rich-output vector; its
  existing owner must still publish actual consequences and scoped completion
  receipts exactly once. lines_resource.rs also supplies explicit None
  provenance. These are payment-owner review points, not added coverage.
- **Retained receipts and context:** the authoritative sacrifice receipt fix
  and new ExecutionContext::with_effect_outcomes setter restore the intended
  captured result route. Activation and follow-on amount evidence should be
  reviewed through these current owners, rather than an old scalar adapter.
- **Draw/prevention continuation ownership:** move/borrow repairs retain the
  actual draw resume value and prevention suffix before closures, and ZoneTail
  restores its checkpoint by reference. These appear to preserve the intended
  chronology, but changed continuation owners require combined source review.
- **Retained snapshot decoding:** explicit serde deserialize bounds remove the
  accidental E: Default requirement. The existing spell-effect default remains
  Unavailable; no new field/default or ordinal is introduced. Review supported
  historical decode behavior rather than interpreting a trait-bound repair as
  evidence of restored gameplay state.
- **Face-down presentation:** the permission label now reads a source object
  name or uses a fallback. Review the hidden-information boundary for blind
  exile permissions; formatting repair alone does not establish disclosure
  safety.

Additional production adapters route conditional flashback through the current
condition-parser API, pass the current StaticAbility type in token lifecycle,
and read current spell-source counters through counts() while retaining the
source-snapshot fallback. Compare those owners before carrying forward their
prior source conclusions. Native draw algorithms and public-audit production
encoding have no changes in the three user commits.

Most other user changes are visibility/import/type-path fixes, repaired test
constructors or fixtures. The numeric-pair token codec regression moved from
continuous/text_change_token_tests.rs into artifact_text_program_codec.rs; it
was not deleted. The typed_text_changes integration test removed the direct
`ctx.created_continuous_effects.is_empty()` assertion while switching to the
public EffectContext. Its game rollback assertions remain, but context receipt
cleanup no longer has that particular explicit assertion. Keep this evidence
reduction visible in the next review.

## Compatibility and remaining limits

No source edit here changes serialized fields, enum ordinals, the artifact
format, the public audit encoding or protocol numbers. The user's public_audit.rs
change is in a test type annotation; it is not a public encoding change.
Artifact11/digest7/audit24 stay untouched. Current payment/Stop behavior,
reference rewriting and changed continuation owners still require the
coordinator's compatibility and combined admission decision. The measured
40/3,193 baseline remains unchanged.

Trusted local analysis images remain distinct from native local handle recovery,
private persisted exact-build recovery and authenticated remote transcript
recovery. The image restores actual memory/globals/references under existing
build/layout/root checks; no generic gameplay serializer or foreign-peer seed
route was added. Generated exact-snapshot glue/build identity remains a deferred
validation prerequisite, as in the prior source review. No missing glue was
invented or generated.

Fresh independent combined source review follows this reconstruction. Runtime
validation, exact native/root/inactive-lane retention, pending/error retries,
current replay, artifact regeneration and new-base source admission remain
separate gates. No runtime correctness follows from this source inspection.
