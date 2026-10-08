# Complete suspended-target union integration

Status: **source-authored, UNVALIDATED; all executable gates UNRUN**.
Base: local `94e075587a07b4ba659bcca84ded1d98605ccb82`, the retained source tree
corresponding to published recovery packet 868. This repair preserves that
source and does not change campaign ledgers or compatibility declarations.

## Exact original candidates

The retained `fixtures/suspended_counter_bodies.json.fixture` contains complete
original bodies, including printed mana/type/P/T metadata, all modal arms,
Shivan's fixed Suspend and Timebender's Morph. No fixture was shortened or
rewritten. All four remain recorded as `reheld_current_main_failure` in
`fixtures/card-failure-campaign/current-residual-source-coverage.json`:

- Fury Charm: `b86b1878-65a9-48df-9727-1e80683d7e86`
- Shivan Sand-Mage: `3afe854a-7daa-4745-a5d9-04879e5d940a`
- Timebender: `efa3e524-1591-4036-81ec-74199cee25aa`
- Timecrafting: `57939644-6f9a-4653-920b-f2d28559a9ca`

The retained refresh record is
`reports/card-failure-campaign/refresh-20261007-main5cc46c1/prior-proposals-55-unresolved.json.gz`.
Each ledger entry records `unsupported complete object filter: permanent or
suspended card`, including failure of the Oracle-only fallback. These are four
full-body source candidates, not measured recoveries. Their recorded holds are
unchanged pending the full compile/render/native-runtime and corpus gates.

## Root and shared owner

`spell_filters.rs` classifies a complete filter after target extraction. Bare
`permanent or suspended card` has no relational keyword and reaches strict
characteristic-only rejection before the existing typed two-arm owner. Adding
`suspended` to the tolerant relational vocabulary would risk admitting unknown
tails. Separately, an earlier public branch-shape registry can consume a bogus
zone suffix and ignore `suspended` before reaching this classifier.

The correction commits the complete phrase to the existing union owner before
both public object-filter registries and the strict grammar classification.
Its result also replaces the old optional relational-owner call. Known union
heads are parsed completely or rejected, including unknown tails, unsupported
counter quantities/types, extra arms, internal punctuation and wrong-zone
suffixes. Leading `all` retains its surface quantifier. Enclosing target
relations retain their own ownership; this packet does not claim newly
supported nested spell-target unions.

The existing `any_of` arms remain unchanged:

- Permanent: Battlefield, no controller restriction unless explicitly written.
  Removal does not require an existing Time counter; placement does.
- Suspended card: Exile, Suspend capability and a Time counter. This never means
  any exiled card or any card carrying Time counters. Owner qualifiers remain
  arm-local; caller `other` remains an outer exclusion.

The current capability/filter owner, native counter changes, real last-counter
Suspend trigger, free casting, Morph, modes and legality revalidation remain
the baseline implementations. No card-name-specific runtime path is added.

The structured renderer now recognizes only the exact unqualified two-arm
semantic predicate and spells `suspended card`. It retains the permanent-arm
Time qualification for put, and declines compaction when any extra owner,
controller, zone, counter, capability or outer constraint would be hidden.

## Independent authored gates

`suspended_counter_bodies.rs` registers sixteen scenario bodies separately in
`direct` and `artifact` test modules. Each scenario compiles only its own route.
Direct calls `compile_to_runtime_definition` on each original fixture, with its
own parse-loss capture. Artifact separately captures `compile_to_artifact`,
validates, serializes, decodes, revalidates and materializes. Its returned
companion definition is intentionally unused: that companion is already
artifact-materialized and does not establish an independent direct route.

Literal expectations pin UUID/name, mana symbols, types, subtypes, P/T, fixed
Suspend cost/count, absence of unimplemented content, all modal arms, exact
Time quantities and the complete target predicates. The executable renderer is
reparsed with independently specified metadata and checked against those same
literal semantic expectations, not merely compared with another compiler route.
Malformed original full bodies are independently rejected by both routes.

The retained native scenarios cover every mode, exact one-target selection,
own and opposing permanents and suspended cards, counterless/Charge-only
permanents, exile without Suspend or without Time, face-down exile, other-zone
objects, paid X including zero, Fury destruction/pump/trample cleanup, normal
Shivan casting and paid Suspend/upkeeps/free cast/ETB/haste, paid Morph and its
face-up trigger, last-counter response invalidation, and actual last-counter
removal casting only once. Stack exclusion is a zone-mismatch object witness,
not a separately announced StackEntry.

New response cases remove genuinely granted Suspend or conceal its recipient
after target declaration, for either owner and every counter mode. They assert
that live abilities actually disappear, the target becomes illegal, counters
remain untouched and no cast is offered. An ungranted sibling confirms the
grant is identity-locked.

## Independent review correction: Exile grant setup

The historical fixture cast a generic single-target `gains suspend` witness in
Exile. Source review showed `with_spec` collapses one target to `Specific`,
whose resolution scope is Battlefield/Stack. Its RemoveAllAbilities negative
had the same issue. Those scenarios did not establish an effective Exile grant.

The corrected witness uses the established explicit Exile `EffectTarget::Filter`
with `Resolution { locked_targets }`, a real end-of-turn continuous effect, and
the two typed Exile abilities from the route's complete Shivan definition.
Recipients have no alternative-cast record. Live-ability assertions establish
the intended prestate in the authored scenario; the granting player remains A
even for a B-owned card. Loss uses the same supported zone/identity owner.
No engine-zone expansion was made. Generic single-target continuous granting
in Exile remains a separate shared-owner limitation, outside these four cards.
This supersedes the historical packet's claim that its generic witness proved
working `gains suspend` lowering in Exile.

## Compatibility and admission

Compiled artifact 13, public digest 9, signed audit 26, the existing schema hash,
Manabrew 3 and local-image format 1 are untouched. No serialized field, enum,
codec, canonical object representation, checkpoint or hash preimage is added.
This is nevertheless a compiler-admission and canonical-rendering semantic
change: newly compiled accepted unions and their presentation can differ from
prior output. Any eventual publication must include it in the parent's reviewed
semantic compatibility decision; unchanged constants are not a claim of old
artifact, signed-byte or replay equivalence. No prior payload was relabeled and
no generated fixture, card asset or recovery count was produced.

Independent source review checked the parser ownership boundaries, target
predicate, test API signatures, runtime scope and renderer. It found no remaining
concrete blocker in these four paths after the stated corrections. This is not
validation. Builds, tests, probes, formatters, corpus runs, analysis reruns,
generation and remote writes were not performed.

## Subsequent review correction: relation-tail ownership

A subsequent independent source review found one remaining blocker in the
preceding packet, superseding its final no-blocker statement: the complete union
owner exempted every phrase containing `that target(s)`. For `permanent or
suspended card that targets a creature`, the relation reader extracted
`target_object`, then returned a new union from the shortened `base_tokens`
before attaching that relation. The resulting filter silently lost the tail.

The correction determines ownership from the noun prefix before the first
`that target(s)` relation. A prefix containing both union domains is committed
to complete parsing of the original tokens, including any following relation;
unsupported tails are rejected. A prefix without the union still delegates to
the enclosing relation reader. No target-relation grammar or global fallback
is broadened.

The relational reader's union dispatch now occurs at the start of
`parse_object_filter_inner`, before vote, arity, source, target, attachment, or
exclusion preprocessing. The former dispatch on shortened `base_tokens` is
removed. Thus this owner cannot return a fresh union after those passes have
accumulated semantic state. The existing relation attachment through
`ObjectFilter::targeting`, `targeting_only`, and `with_target_count` remains
unchanged for enclosing relations.

Additional UNRUN source gates exercise all four public filter readers and the
strict/permissive relational readers, with both caller `other` values. They
reject target-object/player, only/single/two-target, plural/reversed/coordinated,
attachment, exclusion, and target-count tails. Delegation assertions retain
outer targeting filters; supported outer relation witnesses with explicitly
counter-qualified suspended operands assert the exact nested two-arm union,
Stack scope, `other`, and absence of invented arity. Bare nested suspended
fragments remain subject to their existing reader's admission limits.

Whole-body negatives separately call `compile_to_runtime_definition` and
`compile_to_artifact` through the existing independent route registrations for
all four original fixtures. Each replacement now changes one counter arm at a
time while retaining every other original body line, so a removal-arm rejection
cannot conceal a put-arm tail loss. The checked APIs and field names are the
existing compiler routes and `ObjectFilter` target-relation fields/builders;
no fixture, runtime semantic owner, renderer, ledger, or version is changed.

This correction is source-authored, UNVALIDATED, and UNRUN. No build, test,
probe, formatter, corpus/analysis rerun, generation, or remote write was made.
It remains pending independent rereview and all authorized executable gates.
