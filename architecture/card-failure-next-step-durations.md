# Exact next-step owners: reconstructed source packet

Status: UNVALIDATED, source-only reconstruction on
`a1dc47c4c0176fe429a5f7bd752b0338b1cd0aab`. The old `123eaf27` object and worktree
were absent from the restored repository. This packet reconstructs the intended
owners and scenarios; it does not claim historical byte identity.

No build, cargo command, test, compiler/parser/engine/browser/replay probe,
formatter, or code generation was run. No ledger, version, compatibility gate,
measured recovery count, or publication was changed. The inherited
artifact12/digest8/audit25 tuple is not a compatibility approval for these new
model variants.

## Frozen cards and disposition

`fixtures/next_step_durations.json.fixture` retains exact Oracle IDs, complete
bodies, mana/type/PT metadata and the original per-card diagnostic records from
`cards-20261003.json.xz` and `baseline-e8740178.snapshot.json.gz`:

- Fatigue, `0a88dcb6-a391-408f-8bfc-7b5b2cc34267`: proposed complete source body.
- Misstep, `4ee994a1-fd3b-43cf-be3c-9c705e60754a`: proposed complete source body.
- Orcish Farmer, `c3039d19-8c98-4953-8943-9922b6ab45ef`: complete body authored,
  but the controller-change duration edge remains explicitly held for independent
  rules review below. Its earlier basic-land cohort remains partial; no earlier
  cohort count or fixture was rewritten.
- Savor the Moment, `d5b9b75d-4bf0-4328-9bb3-7b260a0c46af`: wholly held. Existing
  extra-turn queue indices do not survive Grand Melee deferred/retained turn
  ownership. No new grammar acceptance, runtime placeholder or full-body claim
  is introduced for Savor.

## Reconstructed source owners

Fatigue reuses the existing independently counted `ScheduledSkipKind::DrawStep`.
The complete draw shortcut now declines `draw step`/`draw steps` nouns so the
skip reader owns the entire clause. Two pending effects consume two distinct
future draw-step occurrences. Off-step draws, other players' steps and skipped
turns do not consume those occurrences.

Misstep creates a rule over a live controlled-creature filter. Its selected
player is retained in both that filter and
`Until::PlayersNextUntapStep { player }`, materialized to `Specific` on resolution.
Later entrants, later creatures and control changes remain relevant; this is
not a locked resolving object set and does not tap anything. Lowering requires
the explicit player antecedent instead of inventing a target for an orphan
`that player`. Named-step activity uses the currently focused lane's
`turn_players()`, not Grand Melee's wider `is_active_player()` set. Inactive
lanes retain the rule without applying or consuming it.

Orcish Farmer reuses the reviewed CR305.7 basic-land owner:
`SetSubtypes` plus `RemoveLandRulesTextAbilities`. Independent grants, card types,
supertypes and unrelated subtype families retain that existing implementation.
The proposed `Until::UntilControllersNextUntapStep { object }` materializes the
exact affected incarnation and removes all layer parts at the next qualifying
beginning, before phasing. The currently authored player test is dynamic; the
rules evidence and hold are recorded separately below. Source departure does
not end this duration; a target's later incarnation is not substituted.

Literal `ThisLeavesTheBattlefield` remains a leaves event duration and renders
`until this source leaves the battlefield`. It is not rendered as the distinct
`ForAsLongAs` predicate duration, whose phasing and permanent-latch behavior
remains separate.

## Actual occurrence and native continuation

`TurnStore::untap_step_started_at: Option<(u32, u64)>` retains a lane-local turn
and registration-timestamp receipt. A named-player rule registered inside an
already-begun untap waits for another actual occurrence, including an added
untap in the same turn. Skipped steps never create a receipt.

The runner's native `pending_untap_boundary` retains an `UntapStepBoundary`
containing the original receipt and exact beginning-expiration effect IDs.
An optional-choice retry does not recalculate that boundary after another lane
registers effects or changes controllers. A resumed untap also cannot consume
a newly scheduled skip of a future step. The game, runner, timestamps, exact
IDs and pending answers are retained by native savepoint owners. Pending/error
attempts do not publish partial characteristic changes or consumption.

An error after an accepted optional untap answer restores the runner's complete
pending choice prefix, submitted response, and original boundary together with
the game and trigger queue. The host's ordinary Boolean command path also owns
a scoped RuntimeSavepoint for this suspended untap, restoring the last prompt
on error without dropping earlier answers. Native retry reuses the submitted
answer; a host retry can resubmit the last answer without replaying prior prompts.

Named-step restrictions expire after the untap actions complete. They do not
need to survive the following mana-emptying operation: the earlier suspicion
about `UntapEndMana` was withdrawn after checking CR500.3/500.5.

## Rules evidence and explicit inference

Primary rules actually read are Wizards' [June 19, 2026 Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf),
including CR500.3–500.5, 502, 608.2h, 611.2c, 614.10a and 702.26f. They distinguish
beginning expiration, end expiration before mana emptying, rule-modifying live
sets, skipped occurrences and expiry while phased out. The campaign's later
September25 URLs were unavailable in the prior pass; no September read is
claimed here.

Misstep's live creature set is an inference from CR611.2c, corroborated by its
Wizards-attributed 2008-08-01 ruling reproduced by third parties. It is not an
assumption carried over from the original locked-set handoff.

For Farmer, Gatherer was inaccessible. The official [Commander Masters release
notes](https://media.wizards.com/2023/downloads/CMM_Release_Notes/EN_MTGCMM_ReleaseNotes_20230616.pdf)
state that Lorthos's controller-relative next-untap restriction follows later
control changes to the new controller. Applying that player-reference reading
to Farmer's `until` wording is an explicit analogy, not a Farmer-specific
ruling. CR500.4 establishes the beginning boundary but does not by itself settle
whether the duration's player is captured. Independent rules review must
reconcile this analogy with CR608.2h before Farmer is admitted as complete.

## Authored, unrun scenarios and review gate

`ironsmith-compiler-runtime/tests/next_step_durations.rs` exercises complete
frozen direct/artifact bodies, real casts and Farmer's tap-cost activation,
counted Fatigue skips, live Misstep filters, target/control/phasing/departure
boundaries, CR305.7 preservation, malformed clauses, exact duration rendering,
mid-step registration and added untaps. Strict leaf grammar cases distinguish
beginning and during-step vocabulary.

`ironsmith-wasm/src/wasm_game_impl/next_step_duration_savepoint_tests.rs` authors
native game/runner capture, clone, exchange, restore, errors, inactive host
carriers and pending-choice coverage, including optional-answer then replacement
failure, repeated native retry and ordinary Boolean host retry. Whole-body
negative cases cover orphan player references and malformed step/timing tails;
conditional named-step rejection is additionally authored in the owner-boundary correction below. ReplayCheckpoint retains native game
state; RuntimeSavepoint also owns the host runner. Native retention is not
public-audit wire reconstruction: the current audit projection omits the
continuous-effect and restriction-instance registries.

The two appended `Until` variants change serialized model vocabulary through
the existing CantEffect/ApplyContinuousEffect codecs. Native additions are the
TurnStore receipt and runner's retained beginning-boundary structure. These
need the coordinated next-series compatibility review before any admission
count; this packet deliberately makes no version or gate edits.

Fresh review should cover full-body routing, exact player versus object
ownership, beginning snapshots across pending lane switches, registration
cutoffs, independent counted skips, artifact codecs and native savepoints.
Farmer's player-binding question and Savor's exact extra-turn owner remain
explicitly reviewable holds. All executable validation remains UNRUN.


## Source-review correction: duration owner boundaries

The fresh source review found that generic duration readers could emit
`PlayersNextUntapStep` for characteristic changes or permissions whose runtime
owners do not implement it. Those readers now decline the variant; only the
complete Cant sentence reader introduces it, followed by the lowerer's existing
explicit-player, untap-rule, and unconditionality checks. The reverse pairing,
`UntilControllersNextUntapStep` on a Cant restriction, is rejected in lowering
because only the continuous-effect registry implements that beginning expiry.
This does not expand either runtime owner or resolve Farmer's rules hold.

Independent complete-body negatives cover base power, pumps, grants, leading
carry, play permission, beginning-duration restrictions, and conditional named
untap rules through both direct and artifact routes. Focused generic-reader
rejection scenarios are also authored. All remain UNRUN. Since the reviewer
authored this correction, a separate independent source review is required.

### Complete generic-owner inventory and prevention follow-up

Independent review additionally found that a carried controller-beginning
phrase could reach a prevention shield, which has no such expiration owner.
Both new variants are now refused by all four generic constructors: token
prefix/suffix, search/permission shape, base-characteristic shape, and chain
carry prefix. The only grammar constructors are the checked Cant sentence
(named-player occurrence) and a typed basic-land conversion suffix
(controller-beginning). The latter does not make Farmer complete.

A shared lowering validator runs before every subject-verb action dispatch,
including fast paths. It inventories every direct `Until` field: Cant,
control, characteristics, stat changes, grants, all twelve duration-bearing
damage-prevention actions, spell cost changes, goad, additional land plays,
exchanges, permanent-state changes, and mana-spend permission. Only Cant may
carry the named-player variant; only the two basic-land conversion AST actions
may carry controller-beginning. Thus a duration copied from an accepted AST
by gain-followup, shared-trailing, or chain-carry logic is checked again at its
receiving owner. Distinct duration enums do not contain these variants. The sole additional
direct `Until` owner outside subject-verb actions, `DelayedTriggerForDuration`,
already rejects both through the exhaustive supported mapping in
`effect_handlers.rs::apply_delayed_trigger_duration`.
No runtime lifetime was broadened.

Authored UNRUN checks cover the independent reviewer's exact prevention
witness, both variants across prevention/permission/control/grant forms,
generic-reader boundaries, and typed lowering rejection independent of the
text parser. Existing frozen positive full-body scenarios remain unchanged.
A fresh independent source rereview is still required.
