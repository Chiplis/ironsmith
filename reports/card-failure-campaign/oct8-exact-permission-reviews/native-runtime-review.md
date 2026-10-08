# Re-review: a646cc65 source corrections

Exact SHA: `a646cc65f957ee616d6ed98f1f3e4cf82d331dbb`, atop preserved
`d0c214f6de9d8256627804e10812dd698cff7d4f`. Reviewed 2026-10-08 UTC.

**Current disposition: R1 and R2 are closed by source inspection. No additional
required native/runtime source correction identified in this bounded review.**
All execution, including type checking, remains **UNRUN**. This is not artifact16
admission, cache/public/signed-boundary approval, a recovered-ID count, or runtime
clearance. The historical review below remains an account of the earlier SHA;
its open R1/R2 findings and missing-witness list are superseded by this addendum.
Only this review report was edited by this reviewer; no source edits, tests,
builds, probes, generators or remote writes were performed.

## R1 closure: forced-zero X legality and rollback

- `crates/ironsmith-engine/src/decision/mana.rs:2101-2120` now resolves each
  stack-functional minimum using the proposed spell's ID/controller and X=0,
  returns false for a positive minimum, and propagates calculation failure.
- `decision/mana.rs:3747-3768` applies that check to the selected face only for
  printed-X, empty Exile FromZone alternatives. The existing payment proposal
  at `:3353-3431` constructs the chosen spell face in Stack, stages the casting
  controller and refreshes continuous state. It does not silently inspect an
  unchosen physical face for a source-relative minimum.
- `game_loop/priority_cast.rs:3254-3277` performs the same check in the
  authoritative forced-zero phase, before assigning X=0 or finalizing. Both
  contradiction and calculation error call `rollback_action`; that method
  (`game_loop/priority_state.rs:1232-1248`) restores the game checkpoint and
  clears pending transaction state. The older effect-waived route reaches the
  same corrected branch. No positive X is substituted for the free price.
- Actual assertions in `game_loop/priced_exile_permission_tests.rs:269-342`
  include an unaffordable-X menu, a forged priority action, and a bypassed-menu
  direct proposal which first retires the exile ID and consumes a shared grant.
  The rejection asserts restoration of object/debug state, IDs, exile zone,
  resources, grant state/budget, turn casting history, triggers and pending
  state/checkpoint. This is materially stronger than asserting only an error.
- `:345-398` checks source-relative minimum values on opposite proposed faces
  and actually casts the allowed face; `:401-424` checks stale-action rejection;
  `:427-454` checks the existing resolution-effect waiver; `:247-265` and
  `:457-482` retain minimum-zero, additional-only-X and paid/X-price controls.

## R2 closure: complete singleton target vocabulary

`crates/ironsmith-core/src/effect.rs:1710-1725` now requires equality to the
complete canonical ObjectFilter::spell, or a literal SpecificObject after
permitted presentation/Target wrapping. This rejects source flags, nested tagged
filters and other contextual predicates without a partial recursive blacklist.
The three printed programs use the permitted unrestricted spell target; the
permanent gate still lives on the counter replacement, not target legality.

Inspected new assertions:
- decoder/core/graph/legacy checks:
  `ironsmith-artifact-effect-decoder/src/counter_exile_permission_tests.rs:53-80`
- direct compiler-model interpretation, legacy None and presentation hint:
  `ironsmith-compiler-runtime/tests/counter_exile_permission_artifacts.rs:408-437`
- native execution with a genuinely live source spell and a populated stale
  tag, asserting both original stack entries and no grants:
  `ironsmith-engine/src/effects/stack/counter_exile_permission_tests.rs:289-316`
- full-card serialized malformed-target cases:
  `ironsmith-compiler-runtime/tests/counter_exile_complete_frozen_cards.rs:398-436`

## Receipt-edge evidence now authored

The native test file now supplies the previously requested missing cases:
- `counter_exile_permission_tests.rs:332-356`: a nonpermanent independently
  redirected to Exile asserts an actual SpellCounteredEvent but no gated price.
- `:359-391`: a synthetic Instead program separately exiles both an unrelated
  object and the original target, checks both real destinations and absence of
  any permission, and distinguishes this lack of original counter event from
  a successful original counter receipt. This is a provenance witness for its
  synthetic instruction sequence, not a general external rules ruling about
  all possible replacement-owned exiles.
- `:394-443`: checked discovery failure uses a nonconverging source fixture,
  first establishes the typed capture failure, and then asserts no counter
  transaction changes to stack, zone lists, IDs, provenance, replacement or
  continuous state, grants, life or pending trigger/decision state.

These are authored controls, not claimed passing evidence. The original receipt
owner itself is unchanged in this follow-up and retains the positive source
properties documented in the historical review.

## Real frozen-card integration versus synthetic prerequisites

`compiler-runtime/tests/counter_exile_complete_frozen_cards.rs:59-92`
independently calls compile_builder_to_runtime_definition and
compile_builder_to_artifact, then serializes/deserializes/materializes the
artifact. It compares the exact counter model and first-print metadata. This
corrects the earlier risk of treating compile_to_artifact's already materialized
second return value as an independent direct route. Kheru's corresponding
fixture is corrected at `kheru_complete_morph_counter_permission.rs:36-57`.

The new full-card instant witnesses actually cast Spelljack/Thranduil's Decree
and their targets, pay the counter card's cost, select its target, resolve its
counter and let its source naturally depart (`:178-202`). Assertions cover:
- Permanent and nonpermanent destinations, exact-ID grant binding, opponent
  ownership, captured casting controller, later turns and an actual zero-mana
  recast (`:205-234`).
- Ordinary timing, mandatory red mana and life, actual resource consumption,
  and final battlefield controller/owner (`:237-268`).
- Spelljack's permitted modal land face, ordinary land timing and actual land
  entry under the grantee's control; Decree omits that action, rejects the
  forged land action without moving the card, and still casts its spell face
  (`:288-317`). These occur after the real counter source has already departed.
- No free morph, intrinsic competing alternative, or ordinary-price origin,
  despite enough mana to pay morph/the intrinsic alternative. Forged attempts
  preserve exile/resources/grants and the valid face-up free cast actually
  completes (`:320-375`).

The separate price-prerequisite tests still intentionally use another producer
of the same FromZone shape; they are synthetic shared-path evidence. Kheru's
full frozen morph/trigger witness and these complete frozen instant witnesses
supply the distinct actual-card integration layer. None has been run.

## Source departure and exact lifetime across faces

No face-specific stable-card fallback was added. The counter grant still stores
only target_id, Zone::Exile and the captured controller, with an Effect lifetime
that does not require the source to remain present (`effects/stack/counter.rs:303-335`,
`grant_registry.rs:961-985`, `:231-241`). A proposed other/land face preserves the
same ObjectId while querying permission; announcement/land entry leaves Exile
and retires that identity. Later return has a different ObjectId and cannot
inherit the price or Spelljack's land domain. The Adventure exception can retain
its separately authored stable permission but cannot turn an exact grant into a
stable grant. Existing authored departure/reentry, genuine Adventure return,
source-departure, selected-face and new full-card land controls support these
separate parts. I found no need to broaden this repair into unrelated legacy
permission proofs or require their Cartesian product as a new blocker.

Full-card mutations at `counter_exile_complete_frozen_cards.rs:439-464` compare
well-typed rechecksummed gate/play/rider changes against independent source
semantics. They correctly do not claim that envelope checks authenticate source
meaning. Semantic/cache/public/signed admission remains a separate gate.

---

# Historical review of d0c214f6 (superseded as noted above)

# Independent native/runtime source review

Reviewed SHA: `d0c214f6de9d8256627804e10812dd698cff7d4f`.
Base: `041e1d0b1af01ad82612359d55e8614a97756926`.
Date: 2026-10-08 UTC.

Disposition: **HOLD for source corrections and missing edge-case evidence below.**
This is an independent static source review, not executed evidence. Every build,
test, compiler probe, formatter, corpus measurement and generator remains
**UNRUN**. No source file was edited, and no remote write was performed. This
report is the only file written by this reviewer. SOURCE.md and
ARTIFACT-IMPACT.md were read. No artifact16 admission, digest10 compatibility,
recovery count or release readiness is claimed; that boundary is separately
pending.

## Required corrections

### R1 — Free printed-X casts bypass an authored positive minimum X

Production anchors:
- `crates/ironsmith-engine/src/game_loop/priority_cast.rs:3241-3264`
- Same file `:853-877`, `:947-954`, `:3300-3349`
- `crates/ironsmith-engine/src/decision/mana.rs:2081-2097`,
  `:3618-3997` (`can_cast_spell_with_context`)

The newly extended fast path recognizes an empty Exile FromZone price and a
printed mana cost containing X, sets X to zero, and immediately advances to
targeting/finalization. It does not call `min_x_from_static_abilities` before
that return. The new bounds branch at lines 952-954 records `(true, min_x, 0)`,
but this fast path never reaches it. Source search finds the actual
`this_spell_x_minimum_value` enforcement only in that bounds machinery;
`spell_cast_restrictions_allow` handles a different capability.

Consequently a supported native spell with printed `{X}` and a stack-functional
`StaticAbility::this_spell_x_minimum(Value::Fixed(1), ...)` has no legal X for
this free price, yet the new branch proceeds with X=0. This is a static
control-flow finding, not a run result. The older waived-base branch shares
this omission, but this patch extends the affected route to the new exile
price and cannot claim complete printed-X legality without addressing it.

Required repair: reject this selected price when its forced zero violates an
authored minimum; preserve rollback of the pre-cast exile object, resources,
permissions and continuation state. The menu should omit the impossible route,
and forged/stale actions must fail safely at authoritative announcement. Do
not solve it by permitting a nonzero printed X or by silently clamping a
contradictory bound upward.

Required authored controls: printed-X/minimum-positive free exile has no menu
route and cannot commit a forged action; minimum-zero free exile still casts
at zero; independently additional-only X and paid/X-bearing alternatives retain
their announced values. Existing tests at
`game_loop/priced_exile_permission_tests.rs:241-286` omit the minimum-positive
case.

### R2 — Shared target-contract validation is shallow

Production anchors:
- `crates/ironsmith-core/src/effect.rs:1707-1722`
- `crates/ironsmith-engine/src/effects/helpers.rs:3065-3074`, `:4040-4051`
- `crates/ironsmith-engine/src/filter/matching.rs:122-129`
- `crates/ironsmith-engine/src/effect_model_interpreter.rs:445`
- `crates/ironsmith-artifact-effect-decoder/src/stack_event.rs:12`
- `crates/ironsmith-engine/src/effects/stack/counter.rs:468-490`

`exile_permission_target_is_supported` accepts any Object filter whose top
level says Stack/Spell and has an empty top-level tagged_constraints vector.
For example, ObjectFilter::spell() with `source=true` is accepted. The helper
then expressly resolves that selector as the context source, bypassing an
explicit target. If the source is a live spell stack entry, the native
one-spell runtime check does not reject it. This contradicts the carrier's
stated explicit-target/no-source-selector contract.

Also, placing a tagged constraint in an `any_of` child passes the admission
check, while filter matching evaluates that child. Therefore the common
predicate does not actually establish its advertised tag-independent target
vocabulary across compiler interpretation, decode, graph walk and execution.
It does not undermine the new grant's exact object ID, but it weakens the
semantic fail-closed boundary that all those routes rely on.

Required repair: explicitly constrain the admitted filter vocabulary, or reject
source/context-collection and tag dependencies recursively, retaining legitimate
presentation-only hints and the intended single spell/SpecificObject forms.
Do not rely only on the number of objects returned at execution.

Required authored controls: source-qualified stack filter and nested-tag filter
must fail core validation, direct interpretation, typed decoding/card-graph
walking, and native execution before any counter or grant mutation. Preserve
plain legacy counter behavior. Current decoder controls
`counter_exile_permission_tests.rs:52-72` and native controls
`effects/stack/counter_exile_permission_tests.rs:289-304` cover only top-level
Tagged, All, and/or nonspell domains.

## Positive source findings, conditional on the corrections

- Counter success is bound to the exact original committed receipt:
  `effects/stack/counter.rs:168-240` obtains a checked pre-move snapshot, installs
  only an ephemeral self-replacement, and grants only on Proceed with final
  Exile and Some(new_object_id). Prevented, NotApplicable, Replaced and other
  destinations do not produce this grant. Nonpermanent eligibility is None even
  if another replacement later chooses Exile.
- The permanent-spell gate checks calculated pre-move stack card types, rather
  than narrowing the initial target to a battlefield permanent. The five
  spell-permanent types are used; Land is appropriately excluded.
- `counter.rs:303-335` uses exact target_id, no target_stable_id, Exile, and
  ctx.controller. The price is a complete empty FromZone total cost. Spell
  filtering excludes lands. Only play adds the distinct land-filtered PlayFrom
  grant. There is no ordinary-price spell PlayFrom grant from this producer.
- The Effect lifetime is source-independent (`grant_registry.rs:231-241`);
  exact target identity prevents a price from being revived by a later
  incarnation, including the existing Adventure stable-permission exception.
- `counter.rs:463-519` owns both installation and finishing deferred receipt
  additions inside the same transaction. `effects/composition/compound.rs:343-407`
  restores whole game/context on error or pending choice. An addition that
  subsequently moves the exiled object cannot transfer the old exact-ID price.
  This is a source argument, not executed continuation proof.
- The narrowly changed affordability branch at `decision/legal_actions.rs:641-669`
  uses full selected-method calculation, which adds mandatory additional mana
  in `decision/mana.rs:2633-2641` and checks ordinary timing, targets, nonmana
  costs and cost adjustments. Paid/nonzero alternatives retain their older
  branch. Other zones are outside the special condition.
- The other-face helper at `decision/legal_actions.rs:679-735` uses the existing
  castable-face decision (including transforming/aftermath restrictions), a
  checked proposed-face query, a face-local validation index, and the original
  combined published index. The old ordinary-reader loop skips the same empty
  Exile price to avoid duplicate routes. It does not invent a normal-price
  permission for the new producer.

## Authored test assessment and remaining evidence

The authored tests contain substantive state assertions, not just display or
parse-success assertions:
- Native receipt tests check exact new IDs, opponent ownership, recipient
  isolation, stale tags/source-linked exiles, source departure, later turns,
  ordinary timing, all five permanent types versus instants/sorceries,
  uncounterability, stale targets, prevented/redirected movements, reentry,
  Adventure stable-grant separation, deferred additions, and pending/error
  rollback followed by replay (`counter_exile_permission_tests.rs:52-318`).
- Price tests actually drive priority continuations and inspect consumed mana,
  life, chosen X, ownership/controller, other-face identity, land entry,
  normal Adventure resolution and subsequent ordinary front-face payment
  (`priced_exile_permission_tests.rs:46-339`). They use an existing tagged-grant
  producer of the same empty FromZone shape, rather than the new counter owner.
  That is useful shared-prerequisite coverage, not a complete counter integration
  substitute.
- Synthetic direct/materialized counter tests combine actual counter receipt,
  source departure, free casting, mandatory life payment, cloning and reentry
  (`compiler-runtime/tests/counter_exile_permission_artifacts.rs:267-399`).
- The complete frozen Kheru witness strictly compiles full metadata/text, compares
  artifact envelope roundtrip, and drives face-down casting, full morph payment,
  real face-up trigger targeting, counter resolution, source departure before
  and after resolution, opponent ownership, timing, free casting and final
  controller. The negative morph cases check missing total/pip payment and
  reject a forged action (`kheru_complete_morph_counter_permission.rs:23-204`).
  Spot checks found the used compile, special-action, priority, trigger and
  stack-resolution APIs in current source; this is not type-check evidence.

Additional required negative witnesses for the receipt guarantees are still
missing from the new native suite: a replacement-owned Replaced body that moves
an object to Exile must not grant; an ineligible nonpermanent independently
redirected to Exile must not grant; checked characteristic discovery failure
must restore the complete pre-counter state. Their production branches look
appropriately guarded, but the named evidence is not supplied by the current
prevent/redirect or face-down tests.

Full runtime witnesses for Spelljack and Thranduil's Decree are not present in
the new compiler-runtime full-card file, which specifically covers Kheru. The
other two have frozen lowering evidence and generic/synthetic runtime coverage;
do not describe that as executed full-card runtime validation.

All three original IDs remain gated and measured recoveries remain zero. After
source repair, the later authorized gates must validate the integrated final
source and all replay/artifact/runtime paths; this report does not authorize
execution or admit the source under the inherited artifact boundary.
