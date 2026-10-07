# Temporary prevention bindings and damage filters

Status: source-only, UNVALIDATED. Every executable scenario below is authored
and UNRUN. No build, compiler/parser invocation, test, engine replay, browser
probe, formatter or code generation was performed. Measured recoveries and
campaign publication/version/ledger files are unchanged.

The bounded source inventory is `fixtures/temporary_prevention_bindings.json.fixture`.
It preserves fifteen exact frozen identities, whole Oracle bodies, metadata,
baseline content hashes and original diagnostics from dataset SHA-256
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.
The original source checkpoint was published864
`f6876fd284afbdef37d16db37d5c73b182194426`.

## Reconstruction after the executor replacement

This source was reconstructed on published865
`489e6561d8b90ca088f2ba2ee481fec875cc3a46` after the original worktree disappeared.
The retained conversation includes the main implementation and scenario patch
payloads, plus the original worker's complete Powder/secondary test payloads and
final corrections. The frozen fixture was transferred again from the verified
corpus and baseline JSON. Existing source was compared before editing; the six
newer Class-related lines in `artifact_materializer.rs` were preserved.

The previous final local checkpoint was `d1dbdb7e24472cbb6a9fc9d19135fe7d95d4bfa7`,
including the reviewed iterated/attacked recipient corrections after `014b1c72c`.
Those corrections are included here. Original full Git blobs are unavailable,
so reconstructed byte identity is not asserted. This is a new source checkpoint
with a fresh source review, not a resumed or validated executable artifact.

### Restoration onto latest main

After a later filesystem replacement, the cohort corrected through
`dcf73cc903c915723d4b95961d2612cf90c8bdab` was replayed onto main
`5cc46c1fa41edb235aacd8e7567ad4ab2f12b7a1`. Retained per-file Git hashes now
allow byte comparison of the reconstructed payloads. The corrected main runtime
scenario file matches `b6abbf7cd1c5b2e51bd3c3ce0a109b48c5e4b376` exactly.
The three overlapping source owners preserve main's new ChangeText resolver
arms, `PreparedEventOutcome::pure` visibility, and `restore_ref` correction.
Removing only those main-specific hunks in memory reproduces each retained
prevention blob; no main hunk was removed from the working files.

This replay remains source-only. The eleven candidates and four partial bodies
are unchanged, no ledger or compatibility versions are changed here, and no
compiler/test result or independent review is inferred from matching bytes.

## Proposed complete bodies

The eleven source candidates are Chameleon Blur, Ethersworn Shieldmage,
Gossamer Chains, Snag, Lithomancer's Focus, Loyal Unicorn, Blinding Powder,
Boros Fury-Shield, Samite Alchemist, Avacyn, Guardian Angel, and Decorated Griffin.
These are full-body source proposals, not measured compiler or runtime successes.

The shared finite grammar retains `combat_only` and accepts both complete
recipient/duration orders. The lowered finite shield retains its real budget,
damage filter and child program; no combat qualifier is discarded by a counter
or reflection rewrite. Samite's ordinary tap and `YourNextUntapStep` instructions
remain in sequence, with the activating controller fixed by their existing owner.

Filtered unlimited prevention now distinguishes live recipient sets from
declared targets and previously bound references. Live artifact-creature sets
remain conjunctive filters; Chameleon protects players against creature sources.
An explicitly targeted or referenced recipient is captured when the shield is
created, so the shield keeps that incarnation through control changes and does
not follow a blink. The source filter remains live at damage time, using the
shared damage owner's current characteristics or exact departed-source snapshot.

Avacyn declares the creature/player/planeswalker target during announcement.
Its controller chooses one of the five colors on resolution, once per shield
instruction. This neither declares a source target nor changes a stored chosen
color on Avacyn. The color further restricts the source filter. A suspended or
invalid color decision publishes no shield. Control-transition discovery and
registration use the existing resource transaction, propagating typed failures.
Chosen-color live recipient sets are still rejected until their shared decision
owner is implemented; they are not included in these eleven identities.

Blinding Powder's complete normalized `granting permanent` unattach operand
selects the exact granting Equipment. It does not impose an Equipment-controller
restriction. The equipped creature owns the granted ability and is the protected
recipient; the Equipment is paid as a cost before resolution. The existing
granting-reference materializer captures that Equipment, including duplicate
Equipments and later detachment, reattachment, or source departure.

No new prevention store, replacement layer, damage-processing loop or player
payment owner was added. Existing shields continue to account for actual
prevented damage, unpreventability, replacement ordering, per-event follow-ups,
turn expiry and restoration of failed root transactions. Completed originals
and additional programs still belong to the shared receipt owner.

## Serialized/public compatibility boundary

### Authoritative damage-source correction after fresh review

The shared shield matcher previously accepted a live source-filter match OR
a match against a carried snapshot. An older colorless/creature snapshot could
therefore incorrectly protect against damage from the same currently blue or
noncreature source. The matcher now chooses one frame: the exact live incarnation
when present and not phased out; otherwise its exact departure/phasing history,
then the existing exact carried snapshot when history is unavailable. No stable
card identity lookup or blink following is added. A failed current match cannot
fall back to LKI. Range, source filter, color list and card-type list all use that
same frame inside the existing checked `PreparedEventContext` owner; incomplete
continuous discovery remains a typed failure before matching.

Additional UNRUN full-body Focus/Chameleon scenarios explicitly supply stale
snapshots through `DealDamageEffect`, test both gaining and losing the qualifying
property, phase out/in, departure LKI, and old versus fresh incarnations after a
blink. Native/restored finite shields cover sibling color/type fields, missing
source evidence with a different snapshot ID, and unchanged finite budgets.
A checked-query scalar failure case requires no partial damage, history or
shield mutation. This correction adds no serialized fields and no coverage credit.

The later central gate must explicitly cover these new fields:

- `ironsmith_core::PreventDamageEffect<E>.damage_filter: DamageFilter`
- `ironsmith_core::PreventAllDamageToTargetEffect<E>.damage_filter: DamageFilter`
- `ironsmith_core::PreventAllDamageToTargetEffect<E>.source_color_of_your_choice: bool`

Both filters default to all damage and the color-choice flag defaults to false
when deserializing legacy structural payloads. The existing `combat_only` field
on unlimited target prevention is retained and conjoined during interpretation.
Defaults are structural compatibility, not authorization to bypass the compiled
artifact admission/version gate. This family is later than pending99's 11/7/24
boundary, so that gate does not automatically admit it.

The semantic action gains finite `combat_only` and filtered unlimited
`of_chosen_color` fields. Target/reference visitors include the previously
untargeted filtered action. The ordinary typed materializer, registered decoder
and card-graph mapper use the core models. New native encoders preserve filters,
duration, choices, shared-recipient flags and complete child programs without a
retained serialized model. Text rendering keeps source restrictions after the
recipient/duration and retains the resolution-time color choice. Special text
folds cannot erase the new choice flag or finite damage filter.

## Authored executable scenarios, all UNRUN

- `ironsmith-tools/tests/temporary_prevention_bindings.rs`: all eleven exact
  metadata-bearing bodies through strict admission, with no Oracle-only fallback.
- `ironsmith-compiler-runtime/tests/temporary_prevention_bindings.rs`: complete
  definitions through direct compilation, serialized artifact restoration and
  native definition transport; compiled-text parsing/stability; real cast or
  activation and actual damage for Chameleon, Ethersworn, Focus, Avacyn and Griffin.
  Tests distinguish combat/noncombat, unpreventable damage, player/object scopes,
  artifact-creature conjunction, creatures entering after registration, target
  controller changes, live source colors, source departure, all Avacyn target
  kinds, another-target exclusion, one resolution-time color decision, stored
  color independence, consumed finite budgets, and public turn expiry.
- The same runtime file exercises fresh native payload encoding with child
  programs, pending-choice rollback/retry, invalid decisions/targets, exhausted
  instruction resources, and restoration of a finite budget, replacement,
  history and life after a deferred numeric failure. Native recipient regressions
  preserve object iteration over an enclosing player binding and distinguish an
  attacked player from an attacked planeswalker/battle without also protecting
  the latter object's controller.
- `ironsmith-compiler-runtime/tests/temporary_prevention_secondary_bodies.rs`:
  exact Snag Forest-discard payment and source filtering; Gossamer owner-return
  cost and unblocked target; Boros red-payment predicate, attacking/blocking
  source, live power and exact controller; Loyal trigger-time/resolution-time
  ownership/control of commander plus vigilance; Samite mana/tap costs, already
  tapped recipient, four-damage pool and fixed activating-player untap timing.
  Illegal targets and controller changes are explicit negative cases.
- `ironsmith-compiler-runtime/tests/blinding_powder_body.rs`: direct/artifact
  full body, paid equip and sorcery timing, duplicate Equipment, another player's
  Equipment, cost-time unattachment, source/recipient changes, combat scope and
  expiry. `granting_unattach_tests.rs` adds complete-operand grammar negatives.
- Grammar cases retain finite amount/kind/duration order and targeted chosen
  color, and reject unowned duration/source tails and second instructions.

Additional central-gate scenarios are explicitly pending execution: choose a
source color after replacing source characteristics, resolve departed-source
damage from its exact LKI, then blink that source and require a fresh identity;
checkpoint between color prompt and completion, reload/retry exactly once;
combine two shields with a mandatory multiplier and let the affected player
choose replacement/prevention order; compare both simultaneous original damage
results before running any additional follow-up program. No new implementation
claim is made for a different transaction boundary.

## Partial bodies, no coverage credit

- Oketra's Avenger: reflexive `to it` needs its source binding, and exert's
  `ControllersNextUntapStep` currently follows a later controller instead of the
  player who exerted the creature. Its complete body is not proposed.
- You Look Upon the Tarrasque: the second mode needs opponent-filtered forced
  blockers, rather than the existing unrestricted literal form.
- Shieldmage Advocate: a protected-target source-choice bridge and the complete
  CR 609.7a source-choice domain remain distinct open work.
- Barbed Wire: finite outgoing prevention needs its exact source selector and
  one shared shield budget across recipients.

Remedy, Embolden, Angel of Salvation, Pollen Remedy and Serra's Hymn are excluded.
Their divided shield allocations (and Pollen Remedy's kicked replacement) are
not implemented by a per-recipient fixed-budget fallback.
