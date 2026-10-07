# Complete permanent-or-suspended counter bodies

## Current NEXT03 disposition

Independent final source review cleared `92168f50060c362c88d5c1ecf6477dbbffa11839`. The [NEXT03 admission](card-failure-next-series-03-source-admission.md) records only the bounded source proposals: predicate 5, copy 4 IDs / 5 entries, suspended 4, numeric 4, plural untap 5. All held neighbors remain excluded. All executable scenarios are **UNRUN**, the source remains **UNVALIDATED**, and no new measured recovery is claimed. The historical scoped-work notes below describe their original stages; coordinated compatibility is now artifact 14 / digest 9 / audit 27.


Source-only closure proposal for Fury Charm, Shivan Sand-Mage, Timebender, and
Timecrafting. The four rows in `fixtures/suspended_counter_bodies.json.fixture`
retain the exact names, Oracle identities, complete Oracle text, printed mana
costs, types, power/toughness where applicable, and source URLs from
`fixtures/card-failure-campaign/cards-20261003.json.xz`. The uncompressed frozen
dataset SHA-256 is
`9915ac0e2ed2c6fa7e6351842666dc024499e6e4f42812f548036f967bec374c`.
All four have category `parser_failure` in the frozen
`baseline-e8740178.snapshot.json.gz`; their recorded failure is the complete
`permanent or suspended card` filter.

This packet builds on source baseline
`c2d21f514164174fa2deb12c1bd1785754411242`. It adds full-body regression scenarios
and repairs granted-Suspend capability discovery in the shared filter owner.
No build, compiler probe,
formatter, test, engine scenario, or corpus pass has been run. The scenarios
remain unvalidated and do not establish measured coverage or successful recovery.

## Existing shared owners

- `grammar/filters/reference_tag_stage/reference_tag_stage_library.rs` retains
  the permanent and suspended-card alternatives as separate `ObjectFilter`
  `any_of` arms. The permanent arm carries Battlefield. The suspended arm
  carries Exile, the Suspend alternative-casting capability, and a Time-counter
  constraint. Each arm owns its qualifiers; the put instruction's Time-counter
  requirement must not leak to the unqualified removal permanent arm.
- `filter/descriptions.rs::object_has_alternative_cast_kind_in_view` checks
  printed and granted alternatives plus the current executable Suspend ability
  set, and rejects current face-down objects before these checks. Layered
  subjects supply calculated abilities so real native grants and their expiry
  or removal are visible. Suspend permission grants are checked independently
  of the targeting player's identity; casting permission itself is unchanged.
  An exile card with counters but no Suspend, a Suspend card with no Time
  counters, and a card in another zone cannot use the suspended arm.
- Existing modal lowering, `ChooseModeEffect`, spell announcement, and
  `sba_triggers::choose_trigger_modes` choose the mode before one target is
  declared. Target extraction and stack resolution retain/recheck that selected
  mode's target requirement.
- Existing put/remove-counter executors own counter changes and actual removal
  notifications. Suspend's own upkeep and last-counter triggers consume those
  notifications. The body never manually casts a card or synthesizes a second
  Suspend trigger.
- Existing priority actions and special actions own printed mana, X, Suspend,
  face-down casting, and paid Morph. Existing continuous effects own Fury's
  temporary power/toughness and trample, and Suspend's continuous-control haste.

Independent source review found that the original registry-only fixture masked
two gaps: actual `gains suspend` grants contain executable abilities instead of
an alternative-casting record, and a grant to an opponent was filtered out by
the spell controller's identity. Both corrections are in the shared capability
query; no card-name branch or text rewrite is introduced.

Under [CR 702.62b](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf#page=164),
suspended status depends on the card's exile zone, Suspend ability, and Time
counter, not on who targets it. The implementation uses the already serialized
`CastSourceEffect.cast_as_suspend` gameplay flag in the source's last-Time-counter
exile trigger. This flag is set by the printed and granted Suspend builders and
already controls Suspend casting/haste; generic free casting or presentation
labels cannot establish the capability. An executor capability hook opts in only
the actual source cast and its source-preserving `MayEffect` wrapper. It does
not traverse granted/copied definitions, delayed programs, or an
`ExecuteWithSource` boundary; those payloads belong to another execution scope.

No serialized field, enum, canonical payload, public checkpoint representation,
or hash preimage changes. Artifact format 6, checkpoint format 3, and audit 19
remain unchanged. Prior admitted grant programs retain the same typed cast flag
and are recognized after restoration. Native trigger objects are inspected
through their actual typed matcher; existing codec rejection of native triggers
without canonical models is unchanged. The authored retained-body case removes
presentation labels and alternative methods, then checks a direct native body
and a previously representable body through the existing ability codec. A
mislabelled ordinary free-cast lookalike remains ineligible.
Nested grants, delayed casts, and source-rebound casts likewise cannot establish
Suspend for the outer source.

## Authored direct, artifact, and native scenarios

`crates/ironsmith-compiler-runtime/tests/suspended_counter_bodies.rs` separately
compiles each full body directly and through an artifact. The latter is validated,
serialized, decoded, compared, and materialized by the authored test code. Both
definitions then run the same independently specified game expectations through
native priority actions, special actions, trigger queues, and stack resolution.
Expected behavior is never inferred by comparing the two implementations.

The target matrix selects every positive candidate in an independent game and
checks all negative candidates at real target declaration. It includes your and
an opponent's battlefield permanents, your and an opponent's suspended cards,
both A-owned/A-granted and B-owned/B-granted registry permissions, real
`gains suspend` spells on either player's card, counterless and Charge-only
permanents, exile without Suspend,
exile without Time, face-down exile, and Hand/Graveyard/Library/Command. Every
mode has exactly one target. Removal accepts the counterless permanent while
put does not. Mode menus preserve this distinction even at X=0.

The complete bodies have the following additional expectations:

- Fury Charm pays `{1}{R}` for each of its three distinct modes. Destruction
  moves the artifact to its owner's graveyard. The pump affects the selected
  creature by exactly +1/+1 and grants trample; both disappear during cleanup.
- Shivan Sand-Mage pays `{2}{R}{R}` for an ordinary cast and announces either
  real ETB mode. Its independent Suspend scenario pays `{R}`, places four Time
  counters, ignores opposing upkeeps, removes one on each own upkeep, and offers
  exactly one free cast at zero. The resulting 3/2 creature has its full ETB
  modes and haste, which ends on control loss.
- Timebender pays `{3}` for a real face-down 2/2 cast, then `{U}` for the native
  face-up special action, restoring 1/1 characteristics and queuing either modal
  trigger. A separate normal `{U}` cast confirms that ETB alone does not produce
  the turned-face-up trigger.
- Timecrafting independently announces and pays X=0, 1, 4, and 9 in both modes,
  plus the fixed `{R}`. X=0 still requires a legal target. Removal is bounded by
  the number present, while put adds exactly X.
- For every counter mode, a response removing the target's last Time counter
  makes the suspended arm illegal. The real intervening Suspend trigger is
  resolved and declined before the original mode resolves; put cannot recreate
  suspended status. On the battlefield the corresponding Time-counter
  qualification only invalidates put, leaving removal legal.
- Every removal body removes the last one or two counters from an opponent's
  suspended full Shivan card. Exactly one cast trigger and one real creature
  spell follow; Shivan's subsequent independent ETB mode and Suspend haste are
  retained. The opponent spends no mana on that free cast.
- Actual granted Suspend is separately removed to zero for both owners, then
  produces one free cast and haste. Expiration, ability loss, and face-down exile
  remove the granted capability from target candidacy.

The deferred full-body, strict round-trip, native runtime, linked-face, full
frozen-corpus, and regression gates remain necessary before any recovery credit.
