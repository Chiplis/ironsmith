# Independent source review: attacked-step prevention

Reviewed commit: `03eb8738ced0badf53ea3586d91b112f29bd1e0f` against `e405f604fb3bfb091765dbbbb08b02c69d87f102` in `ironsmith-prevention-evidence-bodies`.

Scope: exactly Deep Wood and Heavy Fog. Oketra's Avenger, Glittering Lion, Glittering Lynx, and Thunderstaff remain held. Read the engineering skill and the candidate source-review report, then independently inspected the diff and runtime/compiler consumers. No builds, tests, probes, formatters, corpus work, code generation, or remote writes were performed. All authored gates are UNRUN.

## Disposition: hold for one source defect

### P2: combat-phase history is not declare-attackers-step history

`crates/ironsmith-engine/src/decision/mana.rs:1858–1861` implements “you've been attacked this step” by matching the current combat phase number in `players_attacked_in_combat`. This is sufficient for the usual single declare-attackers step but leaks a previous declaration into an added declare-attackers step within that phase.

Concrete source-supported sequence:

1. In combat phase N, B declares a creature attacking A, recording A under `(N, B)`.
2. That attacker leaves combat/the battlefield.
3. An additional DeclareAttackers step follows the first one, scheduled with the existing `add_step_after(Step::DeclareAttackers, Step::DeclareAttackers)` API.
4. In that second step B declares another creature attacking only C. A has not been attacked in this step.
5. A nevertheless satisfies the new predicate because the old `(N, B)` entry still contains A and a combat frame exists.

Source trace:

- `game_state/turns_and_tracking.rs:163–170,223–233`: arbitrary before/after step scheduling is accepted and recorded.
- `turn_runner.rs:1588–1597,3802–3824`: finishing DeclareAttackers consumes and schedules added steps.
- `turn_runner.rs:3687–3714,3650–3660`: activation sets the scheduled phase/step and dispatches directly to DeclareAttackersDecision.
- `turn_runner.rs:1363–1383`: entry clears decision state but does not reset declaration evidence or advance combat identity. `reset_priority_for_new_window` only sets the priority recipient (`turns_and_tracking.rs:516–518`).
- `game_loop/combat_decisions.rs:1081–1094`: direct declarations accumulate under the combat-phase key.
- `turn_history.rs:230–234`: that map intentionally represents a whole combat phase, including for melee; clearing it per step would break its separate consumer.

The isolated added-step path also dispatches directly to its step, bypassing BeginCombat's phase-counter increment. A repair should use a coherent step-local declaration owner rather than repurpose or erase melee's phase history. Exact field/boundary choice is left to the author; this review implements nothing.

The existing unit gate changes `combat_phases_started_this_turn`, and the complete-body gate does the same. Both prove next-combat exclusion only; neither catches two declare-attackers steps with an unchanged combat-phase number. Add a real scheduled-transition gate, independent direct/artifact complete-body checks, and stale-history negatives for later step instances. If runtime state ownership changes, review native clone/restore and lane handling explicitly.

## Otherwise bounded source findings

- The declaration producer records only `AttackTarget::Player`, after costs and survival checks, and records the attacking controller at declaration. Removing/changing control of the attacker does not erase this retained direct-player event. Planeswalker/battle attacks and creatures merely put onto the battlefield attacking do not enter this map.
- Ignoring the attacking-player component when asking whether a particular defender was attacked is appropriate for multiple attacking players; the defender membership remains exact. Another attacked player does not open A's permission.
- Ordinary combat entry increments the phase counter (`turn_runner.rs:1339`, legacy `turn.rs:305`). The counter and history are carried together in TurnStore; Grand Melee lane load/save copies TurnState, TurnStore, and combat together (`game_state/grand_melee.rs:153–174`). That avoids combining the focused lane's step with another lane's retained declarations. It does not solve the multiple-step defect above.
- The predicate retains fail-closed behavior when no combat frame exists and rejects phases/steps other than Combat/DeclareAttackers.
- `player_was_attacked_this_step` now has only the native `YouWereAttackedThisStep` production caller. The redundant OR was removed from `CreatureIsAttackingYou`, leaving that condition's old live-combat branch unchanged. Its existing planeswalker/battle interpretation is not corrected by this patch and is not credited as new correctness. The independent spell-cost condition remains unchanged and direct-player-only. The generic grammar `you_were_attacked` predicate is not the named cast-restriction path and is not covered by this review's admission.
- Runtime effect ownership remains `PreventAllDamageEffect` → `register_prevention_shield`, with unlimited amount, retained controller/recipient, damage-source filtering at the damage event, and end-of-turn cleanup. The inspected matcher supports exact player recipients and live attacking predicates, with source LKI fallback where applicable. No prevention production code changes are made here.

## Gate quality and limits

Both bodies use complete metadata-bearing fixture text. Runtime `definitions()` separately invokes strict direct compilation and strict artifact compilation, independently rejects parse loss, validates the artifact, round-trips it through JSON, materializes the restored artifact, and rejects unimplemented definitions. Every authored full-body scenario iterates both resulting definitions. This is genuinely two entry calls, not reuse of one direct definition as the artifact result.

The paid path selects the normal legal cast action, confirms a payment plan, drives pending choices, asserts an empty {1}{G} pool, resolves the actual complete spell, and checks departure to the graveyard. Native cloning is exercised with a pending stack and with a resolved shield. Tests cover direct versus other-player/planeswalker/battle declarations, removed declared attackers, next-combat exclusion, repeated combat/noncombat amounts, multiple defending players, exact recipient, nonattacking sources, unpreventable damage, live removal from combat, and cleanup expiry. The tools gate uses full metadata payloads with no Oracle-only fallback.

These are authored assertions, not executed proof. There is no actual old-artifact replay gate, no real added-step transition gate, and no direct/artifact test distinguishing a newly put-onto-battlefield-attacking creature from a declaration. Present-tense separation and absent-frame rejection are helper-level gates. The fixture count/candidate flags do not establish externally measured coverage or catalog admission. No credit is granted to the four held bodies.

## Compatibility and old artifacts

The diff changes no compiler output, core model, serialized variant, artifact schema/version, or lowering. The existing core named restriction label “during declare attackers step if you were attacked” still maps in `static_abilities/model_interpreter.rs:912–914` to the same native timing plus `YouWereAttackedThisStep` condition.

For an existing artifact that already carries that named restriction and complete prevention body, this patch does not require regeneration: materialization reaches the corrected runtime owner. Rebuilding the artifact cannot fix the step-history defect. Runtime behavior nevertheless changes for old artifacts: planeswalker/battle-only attacks cease granting permission; a valid direct declaration can keep permission after attacker departure; hand-built/restored states containing attackers but no retained declaration evidence now fail closed. Prior compilation success is not corrected gameplay/replay admission.

Future verification must separately replay a genuinely old materialized artifact against corrected native state, covering direct-player identity, departure, absent history/frame, multiple step instances and combat/lane boundaries. Fresh artifact JSON round-trip and native clone gates are useful but do not demonstrate historical-artifact or persisted-state migration compatibility. No blanket invalidation, artifact-version bump, migration, or compatibility boundary is implemented or authorized by this review.

---

## Corrected native re-review: bounded source clearance

Final reviewed native head: `e86b5870f73147a3c4af7934453b3041ad2c9ae2`.
Sequence inspected: `03eb8738ced0badf53ea3586d91b112f29bd1e0f` → `14b3cf727` → `6c0fa844b397a82acb0e4e76d6095d9749cabff0` → `e86b5870f73147a3c4af7934453b3041ad2c9ae2`.

This section supersedes the original native hold above. The original finding and its source trace are retained as review history. No additional native source defect was found in the correction. This is source clearance for exactly Deep Wood and Heavy Fog, not executed verification, measured recovery, catalog admission, or publication approval. The four other bodies remain held. All gates remain UNRUN.

### Resolved owner and lifetime

- `CombatState.last_attack_declaration_step_players: Option<BTreeSet<PlayerId>>` separates the last begun declaration-step evidence from melee's combat-phase history. Default None means absent/uncommitted; Some(empty) means a completed declaration with no direct-player defenders. The sole gameplay reader in `decision/mana.rs:1844–1855` requires both Combat and DeclareAttackers before accessing this evidence and fails closed without a frame/record.
- Runner entry (`turn_runner.rs:1363–1381`) synchronizes game combat, clears the field in the runner copy, and republishes that copy before exposing the new declaration decision. The legacy scheduled-step path enters `legacy_enter_step`, which resets before priority for an actually entered DeclareAttackers step (`turn.rs:259–287`). Added steps within the same phase therefore cannot borrow old evidence. Skipped steps do not create a committed record or grant their own priority.
- `mark_combat_phase_started` clears the game record; `end_combat` also clears it. Retention through DeclareBlockers and other later steps is deliberate: it is a last-declaration record, not arbitrary-current-step evidence. The exact guard closes permission on exit even when the record remains populated. The revised name and public camelCase name reflect this lifetime.
- `combat_decisions.rs:1067–1080` populates only after payment/choices and survivor filtering, using only exact AttackTarget::Player declarations. Existing attackers, including put-onto-battlefield-attacking objects, do not synthesize facts. A successful empty transaction materializes Some(empty); subsequent successful same-step transactions union their defenders rather than erase prior teammates' entries.
- The existing transaction wrapper clones game/combat/trigger state and restores them on error or awaiting-choice (`settle_combat_declaration`, around line 1769). Evidence is written after the potentially failing costs; interrupted and failed payment do not publish it. The no-cost path has no further fallible declaration stage after publication.
- Phase-wide `players_attacked_in_combat` remains unchanged and continues serving melee. Removing an attacker or changing its present target cannot erase committed step evidence. The separate live CreatureIsAttackingYou branch remains independent.

### Re-reviewed authored gates

- Both complete bodies still compile independently through strict direct and artifact entry calls, and all full-body runtime scenarios loop over both definitions.
- A real TurnRunner scheduled second DeclareAttackers step retains the old live attacker and phase number while asserting fresh None, denied fog permission, committed empty evidence after no declarations, and preserved phase-wide melee history.
- A corresponding legacy `advance_step` schedule asserts the same reset before priority and empty-commit behavior.
- The shared-team test declares two different controllers against different players, verifies aggregation, verifies that a later empty same-step transaction does not erase it, and verifies that clearing evidence followed by an empty transaction does not import the still-live attackers.
- Duplicate-attacker prevalidation rejects without publishing evidence. The final `e86b5870f` follow-up additionally asserts evidence remains absent in the existing competing-attack-cost test where a first sacrifice is rolled back after the second attacker cannot pay. This is a real post-payment failure negative, distinct from prevalidation.
- Grand Melee lane switching preserves distinct populated/empty records, including after game clone and restore-snapshot extraction. Native `RuntimeSavepoint` capture/clone/exchange/restore and nested pending-decision/replay-checkpoint roots are covered by authored tests. The existing implementations clone/swap whole native GameState/CombatState roots, so this field is carried without inference from wire JSON.
- Projection tests distinguish null, empty list, and sorted populated list at the active top level and the Grand Melee combat projection. BTreeSet provides deterministic PlayerId ordering. The final follow-up changes a TurnState equality assertion to `matches!`, consistent with that enum not implementing PartialEq.

### Compatibility scope after correction

The initial claim of no public projection change no longer applies. The corrected native patch adds `lastAttackDeclarationStepPlayers` to active `PublicAuditCheckpoint` and `SyncGrandMeleeCombat` canonical projections. These fields affect digest inputs and distinguish absent versus committed-empty versus exact defenders. They are projections, not a generic persisted-game restore contract. No old public JSON is used to infer declaration evidence.

Compiler definitions, card artifact schemas, and the named cast-restriction label remain unchanged. Correct existing compiled card artifacts still need runtime replay verification rather than card rebaking. Conversely, public digest/replay compatibility must account for the new projection and native behavior. The parent assigned coordinated digest/replay handling (10/28) to the separate `ironsmith-step-history-compatibility` worktree. That boundary is outside this native re-review; it is neither implemented nor independently cleared here. Final integration must include its separately reviewed boundary before admission/publication.

No builds, tests, probes, formatting, corpus work, code generation, or remote writes were performed for this re-review.
