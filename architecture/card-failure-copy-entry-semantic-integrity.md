# Entry-copy semantic integrity: source proposal, UNVALIDATED / UNRUN

## Current NEXT03 disposition

Independent final source review cleared `92168f50060c362c88d5c1ecf6477dbbffa11839`. The [NEXT03 admission](card-failure-next-series-03-source-admission.md) records only the bounded source proposals: predicate 5, copy 4 IDs / 5 entries, suspended 4, numeric 4, plural untap 5. All held neighbors remain excluded. All executable scenarios are **UNRUN**, the source remains **UNVALIDATED**, and no new measured recovery is claimed. The historical scoped-work notes below describe their original stages; coordinated compatibility is now artifact 14 / digest 9 / audit 27.


Base: local `94e075587a07b4ba659bcca84ded1d98605ccb82`, exact tree
`d76dc1fb283514aeccb4e4d068598ded0ba11cf8` (published PR868 tree).

No build, test, parser probe, formatter, corpus execution, analysis rerun, code
generation, or remote write was performed. The retained current audit remains
an observation of `5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1`, not this proposal.
No measured totals, campaign ledgers, compatibility constants, native savepoints,
or exact retained local images are changed here.

## Shared root and narrow repair

`document_parser::normalize_named_source_tokens_for_builder` calls
`normalize_named_source_enter_agreement_tokens` even when no source-name alias
was replaced. The latter previously rewrote every matching `this creature enter`
to `this creature enters`, including the already grammatical causative
`you may have this creature enter as a copy`.

The contextual named-source dispatch and the labeled-static body route use this
helper. That explains both the named-body regressions and the generic Raid body:
a source-name rewrite is not a prerequisite for the unwanted verb inflection.
The entry-copy grammar accepts either `enter` or `enters`, so it retains an
executable copy payload while its stored display carries the altered verb.
Lowering, core mapping, native model interpretation, and artifact materialization
preserve that display. The existing semantic gate correctly notices that
`enter as a copy` is absent. No change to that gate or renderer is needed.

The repair leaves the infinitive unchanged immediately after causative `have`.
Finite source-subject agreement remains in place. It preserves source tokens and
spans, rather than reconstructing a claimed copy instruction from output text.

A separate narrow fail-closed correction makes the name-exception grammar retain
its remaining tokens. Previously it consumed and discarded that suffix, and the
consumer only opportunistically recovered an `and it's ...` characteristic
exception. An unsupported suffix can no longer disappear behind a correct name
exception. Only the already supported complete `and <characteristics>` suffix is
admitted. This does not add a model for Sakashima the Impostor's compound body.

## Bounded candidates, source-admitted; all execution deferred

Four Oracle identities / five compile entries:

- Chameleon, Master of Disguise: `1ce239f2-79ad-4a30-8e8f-f985045f78dc`
- Moritte of the Frost: `78699161-fc14-4a44-8a15-0f7c08be0343`
- Protean Raider: `b2f6a75e-5637-41a8-a32b-56666716492d`
- Sakashima of a Thousand Faces: `8ecdaf4b-4442-42da-9714-4257a83faf50`, ordinary
  entry and identical-face reversible alias

Chameleon and Moritte were strict-compiled in the original frozen snapshot and
failed in the retained current snapshot. Protean Raider and Sakashima were
original failures. These classifications are retained-record facts, not new
measurements. The existing source proposals for Protean/Sakashima must not be
counted a second time merely because this shared blocker is repaired.

Existing executable ownership reviewed for these candidates:

- `keyword_static::parse_enter_as_copy_as_enters_line` retains the copy filter,
  optionality, name, other source abilities, supertypes and conditional additions.
- `ironsmith-core/src/static_ability_model.rs` maps the complete spec, including
  embedded abilities; `lowering_support.rs` lowers embedded abilities in their
  own trigger context.
- `static_abilities/model_interpreter.rs` carries the same spec through direct
  runtime construction, including conditional wrappers.
- `events/processing/mod.rs` selects nontargeted candidates using the entry
  controller, excludes the entering object, checks Raid's current-turn history,
  retains only other copiable source abilities by occurrence identity, and tests
  conditional additions against copiable characteristics rather than layer-four
  animation.
- `events/processing/application.rs` and `zones_and_characteristics.rs` apply
  copy exceptions during entry and place entry counters separately from the
  copied characteristics.
- Costed Mayhem is an existing graveyard alternative-cast method conditioned on
  the current incarnation's discard/cycle receipt, with real mana payment,
  ordinary timing, and no automatic exile-on-resolution rider.

## Authored independent gates

`crates/ironsmith-compiler-runtime/tests/copy_entry_semantic_integrity.rs` supplies
full frozen bodies and printed metadata. It separately calls
`compile_to_runtime_definition` and `compile_to_artifact`, captures strict parse
loss for each, validates and JSON-round-trips the artifact, and independently
round-trips the direct native definition through its wire codec. The artifact
API's companion definition is deliberately not described as a direct route.

Authored checks cover whole rendered rules, copy marker preservation, exact
colored mana costs, colors, mana value,
printed types/subtypes/supertypes/P/T, exact name exception, typed optionality and
controller filters, retained own abilities and conditional counters. Both
Sakashima alias faces are taken from the frozen fixture and required to share the
same Oracle identity. A missing existing required own-ability-retention field is
rejected rather than silently defaulted during decoding.

The separate `ironsmith-tools/tests/copy_entry_alias_catalog.rs` gate uses the
real canonical loader on the unchanged frozen fixture, then strictly compiles
both the ordinary and combined-name catalog entries with their actual parse-name
selection. It rejects fallback loss and unimplemented content. This is distinct
from the individual-face tests and remains UNRUN.

Runtime scenarios use each independent route: accept/decline copy; own versus
opponent and graveyard candidates; hexproof donors demonstrating nontargeted
selection; Chameleon's name-only exception; Moritte's copiable-type condition and
noncopiable donor counters/animation; Sakashima's Partner and legend exception,
exclusion of the applying occurrence and layer-six grants (asserted active before
and throughout entry, then removed before testing that they were not copied);
actual Raid attack
declarations by the right/wrong controller; actual Mayhem discard history,
colored/insufficient mana, timing, turn expiration, incarnation expiration, real
2U payment and creature resolution. Grammar tests separately cover token/span
preservation, finite agreement, named source binding and unsupported exception
tails. These are authored scenarios, not passing results.

## Explicit full-body holds

Five further identities stay outside source-complete admission even if the shared
infinitive repair changes their future diagnostic or rendered marker:

- Sakashima the Impostor: `a7243d25-22a2-4df5-adaf-1f40f5330ec1`.
  The complete name + Legendary + quoted activation exception needs one complete
  grammar owner. Its `{2}{U}{U}` activation must schedule the exact permanent's
  return at the next end step, surviving ability loss without following a later
  incarnation. The old name reader's discarded suffix was not that ownership.
- Superior Spider-Man: `636cc915-9d1b-4ffe-9e74-795b78663911`.
  Each chosen entry-copy occurrence needs its own donor/incarnation and reflexive
  exile obligation. `EnterBattlefieldEvent::with_copy_of` and
  `with_copy_followups` replace one donor/vector; a later acquired copy replacement
  can erase or rebind the earlier choice's obligation. Merely appending enum
  values does not bind the correct source, controller, and copied card.
- The Fourteenth Doctor: `0afcf1eb-ac2c-4bb0-8823-9fe45968c2d9`.
  Its cast-body reveal partition currently lowers matching Doctors through
  ordinary `ForEachTagged` zone moves rather than one simultaneous move batch.
  Its haste consequence also shares the single-follow-up-slot problem. The
  existing copy eligibility filter does correctly use the current graveyard
  incarnation's Library-origin/current-turn history; this must not be narrowed
  to only cards revealed by this particular cast trigger.
- The Master, Formed Anew: `e7828284-543d-4f8b-9c5e-a8f082010398`.
  Its optional exile producer and subsequent takeover counter need a proven
  current-resolution zone-change-successor binding, including redirected exile.
  Generic untargeted exile lowering can assign `SourceExiled`, but ExileEffect
  publishes that tag only for final Exile destinations. This is an unresolved
  source-visible full-body route, not an executed failure. Entry eligibility must
  remain any creature card in exile bearing a takeover counter, regardless of
  owner or which Master exiled it.
- Vesuvan Doppelganger: `aeaccab9-3e2c-4a40-a483-52c4972b2014`.
  Additive copy colors do not represent excluding the donor's color, and keeping
  every source ability is not exact retention of the recursive upkeep ability.
  Both entry and resolving copy models need preserved-color semantics and an
  exact retained ability occurrence, including optional resolution and targets.

Detailed source anchors for these holds: `grammar/keyword_static_lines/copy_shapes.rs`
(name exceptions); `events/zones/enter_battlefield.rs:168-180`,
`events/processing/application.rs` (`apply_trait_enter_as_copy`), and
`game_state/zones_and_characteristics.rs` (`apply_enter_as_copy_followups`);
`reference_linked_programs/reference_linked_library.rs:489-516`,
`compile_support/effect_flow_search_handlers.rs` (`ForEachTagged`),
`effects/composition/for_each_tagged.rs` and `iteration_program.rs`;
`compile_support/effect_dispatch/subject_verb_late.rs:1568-1584` and
`effects/zones/exile.rs:333-406`; shared `static_ability_model/grants.rs` and
`effects/continuous/apply_continuous.rs` copy policies.

## Compatibility impact

The new remainder slice is transient grammar data only. No persistent core,
artifact, native savepoint, runtime event, or protocol field/variant changes.
Existing enum ordinals and the 13 / 9 / 26 boundary remain untouched. Source
acceptance becomes stricter for unowned compound name exceptions; correctly
owned source references preserve a different display spelling. Regenerating
source-derived cached definitions may therefore produce different bytes, but old
artifacts and signed savepoints must never be relabeled or rewritten as a
migration. Any coordinated definition/cache-version decision belongs to later
admission, not an unreviewed compatibility constant edit here.
