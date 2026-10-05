# Current-or-last defending-player references

RECOVERED SOURCE / UNRUN. This recovers the reviewed defender chain through the lost `be13e472` using retained visible edit history, adapted to `bb4452a6a` (new main d51 plus IncompleteEvidence and recovered Foretell). No old file was rolled back wholesale. No build, compilation, test, formatter, runtime probe or corpus execution was performed. The nine-card fixture and all scenario contracts remain unvalidated; central count disposition requires fresh independent source review.

## Rules and exact scope

The September 25, 2026 Comprehensive Rules, CR 508.5/508.5a, 508.7 and 805.10e, distinguish a related attacker's current destination from its last destination after removal. Its damage recipient is a different role. Portal Manipulator's release-note ruling expressly makes defending-player-relative targets illegal when reselection changes the relevant defender. Primary sources: https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.pdf and https://media.wizards.com/2024/downloads/MKM_Release_Notes_82nnDTWVBdD/EN_MTGMKM_ReleaseNotes_20240125.pdf .

The blocking-Memory multiplayer case is explicitly a rules inference from CR 802.2/802.2a: a blocking source naming no attacker is outside the attacker-relative exception, so its ability's controller chooses a specific eligible defending player. The captured combat's active opponents and range membership provide that choice. Neither the redirected damage recipient nor the blocking source's controller is an automatic substitute. This interpretation remains part of the deferred review/scenario gate.

`fixtures/combat_defending_actor_references.json.fixture` contains the exact nine frozen bodies/Oracle IDs from `fixtures/card-failure-campaign/cards-20261003.json.xz`: Falkenrath Perforator, Generous Plunderer, Simian Sling, Tormentor's Helm, Memory Vampire, Auton Soldier, Barret Wallace, Blue Mage's Cane, and Lord Xander, the Collector. The first four are new proposals; the last five were re-held for this shared defect. No fixture marks them as executed or validated.

## Shared owners and transfer inventory

The existing copy-on-write combat-transient owner retains attacking-tenure IDs and blocker defender sets. Exact ObjectId and tenure identify the event participant; a later incarnation or a new attacking tenure cannot take over an earlier reference. Live references query their own destination across focused and suspended Grand Melee lanes. Removal, control/type/phasing changes and combat end retain the last defender. Attacked-permanent recovery precedes retiring attackers in a simultaneous control transition. TurnRunner refreshes its local combat copy before end capture.

Actual attack/becomes-blocked/unblocked producers and both ordinary and generic combat-marked damage producers stamp the exact role before replacement programs or damage-result programs run. RawEvent payload replacement, simultaneous receipts and native clones retain the role. Equipment-granted damage still uses the existing exact source/snapshot attribution rather than replacing the source with the Equipment.

Fresh StackEntry/RawEvent constructors start without inherited evidence. Unstamped legacy direct-player events can use their explicit player only when no tracked or current attacking tenure exists for that exact object; otherwise they fail with IncompleteEvidence. Matching a current ObjectId alone cannot prove the event belongs to that tenure. Trigger admission transfers the event reference. Target announcement, retargeting and resolution legality requery it. An exact attacker reference stays singular in shared-team turns; neither its target nor a supplied exact player scalar expands to teammates. Reselection to another player, including within the same team, changes legality dynamically. Only the explicitly captured no-attacker candidate set invokes a native player choice. New d51 target count/iteration/outcome contexts retain their existing data and now additionally carry this role.

ExecutionContext and its checkpoints retain the combat context. Typed selector/effect traversal binds a role at the instruction that uses it, replacing the defending-player Debug-string scan. Optional branches do not ask for unused actor choices. Missing required evidence is IncompleteEvidence, distinct from a known empty recipient. The existing checked-query/resource latch carries query errors; failed admission restores the queue and stack.

Reflexive registration binds its governing actor only after its antecedent succeeds. Pending reflexive state and its later stack entry carry Selected/KnownAbsent. Delayed config, template, registration and event transfers retain a selected inherited actor when that delayed body uses one; a trigger observing a new attack/block/damage event instead starts from that new event's reference. Both copy constructors, including the currently resolving source path, preserve the typed field alongside the legacy scalar. Legacy raw source constructors explicitly start without one.

Native GameState and RuntimeSavepoint capture/restore/exchange preserve the owner, stack, pending decisions, live continuations and runner together. Grand Melee stack/focus transfer and suspended parent-game frames keep native clones. The current public_audit.rs is a consensus projection and explicitly has no GameState importer: recovery uses native savepoints or signed action replay. No partial wire recovery path or weakened refusal boundary was added. A required reference whose retained owner is absent fails explicitly.

## Adaptation and deferred validation

The bare/article defending-player parser prerequisite is already present in d51 and is preserved. Changes are targeted additions to current owners. The d51 numeric/announced-X, instruction-result, exact cost/entry-receipt, announcement-reference fallback and existing `verify_intervening_if_at_resolution` / `triggering_object_current` behavior are not replaced by an older file. In particular, Sigil's normal-resolution correction remains in main.

Deferred tests:

- `crates/ironsmith-compiler-runtime/tests/combat_defending_actor_references.rs`: strict direct/artifact whole-body round trips; actual attack/block/damage and native activation paths; recipient redirection, reselection and dynamic targets; removal/control/type/phasing/new incarnations; Plunderer's optional/reflexive Treasures; Sling/Reconfigure and Helm source attribution; Memory accepted/declined evidence, blocking multiplayer choice and pending rollback; Cane, Xander, Barret and Auton actor/lifecycle behavior; missing-evidence admission and known absence.
- `game_state/defending_player.rs`: tenure reuse, legacy evidence, singular shared-team attackers, exact native clones and suspended Grand Melee lanes.
- `effects/stack/copy_spell.rs` and `effects/delayed/schedule_delayed_trigger.rs`: copied dynamic/selected/absent/missing roles and inherited versus fresh delayed observations.
- `crates/ironsmith-wasm/src/wasm_game_impl/combat_defending_actor_savepoint_tests.rs`: native branch restore, pending blocking-Memory options and invalid answers, copied Cane revalidation after source removal/reselection and missing-evidence retries.

The existing full-body suites remain required: `effect_granted_casting_prices.rs` for Cane's job select and paid copy, `relative_hand_quantities.rs` for Xander's entry/death actions, `aggregate_damage_quantities.rs` for Barret's full count/reach body, `enter_copy_exceptions.rs` for Auton's copy/myriad lifecycle, and `collect_evidence.rs` for the evidence owner. All new scenarios are UNRUN. Source inspection is not a compiler or runtime pass.

Fresh-review follow-ups use the independent strict direct compiler API in addition to artifact decoding, inspect completed source attribution through the public per-player action-history accessor, and capture effect-created blocked roles before a continuous refresh can remove their attacker. The same-ObjectId legacy second-tenure ambiguity has an explicit unrun failure contract.

Fresh September-rules review also narrows Grand Melee no-attacker candidates through the existing attack-direction owner (CR 803.1a and 807.2b). Its attack-left profile does not turn the other in-range opponent into a defending player.
