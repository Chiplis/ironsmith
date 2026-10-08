# Prevention evidence bodies: source-only checkpoint

Base: e405f604fb3bfb091765dbbbb08b02c69d87f102. All six identities and complete metadata were read from `cards-20261003.json.xz` and matched against the original unadmitted measured-success inventory. Existing measured compilation is not gameplay admission. All tests are UNRUN; no build, test, formatter, probe, corpus, code generation, remote write, or accounting change is authorized here.

## Original source finding (superseded owner below)

Deep Wood and Heavy Fog have identical full bodies. The shared `decision/mana.rs::player_was_attacked_this_step` incorrectly infers past attacks from current combat attackers and treats attacks against a player's planeswalker or protected battle as attacks against that player. The declaration path already records direct player attacks in `turn_history.players_attacked_in_combat`, scoped by combat phase. Use that owner for the past-event condition; retain the separate present-tense attack condition's existing implementation outside this patch.

The final source candidate recommendation is limited to Deep Wood and Heavy Fog, with complete independent direct/artifact source gates below. The other four remain explicit holds.

## Final bounded disposition

Propose exactly these two source candidates, currently HELD pending re-review of the step-local correction and coordinated digest/replay compatibility (not measured recovery or accounting admission):

- Deep Wood: `3f01f627-9fbd-470b-8001-974784ccf421`
- Heavy Fog: `a2006755-8812-4aad-8567-e8df6e8923da`

The complete fixtures include exact costs, types, Oracle text and IDs for all six. `source_candidate` is local evidence metadata, not central coverage. Strict tools gates use the full metadata-bearing payload with no Oracle-only fallback. Runtime gates independently call strict direct compilation and strict artifact compilation, validate and JSON round-trip the artifact, materialize it, reject unimplemented definitions, and run each complete-body scenario through both definitions. No internal effect is substituted for the printed spell.

The fog tests author full payment and stack resolution, graveyard departure, direct-player versus other-player/planeswalker/battle attack declarations, phase/step timing, persistence after the declared attacker leaves, next-combat exclusion, cloned native state before and after resolution, both combat and noncombat prevention, repeated and varying amounts, all attacking creatures regardless of defender, nonattacking sources, exact player versus permanent recipients, unpreventable damage, current attacker removal, and cleanup expiry. The separate present-tense `CreatureIsAttackingYou` condition continues reading current combat rather than inheriting the new past-history semantics. The legacy hand-built Heavy Fog fixture now records its explicit declaration history.

Shared owners: grammar timing restriction; `decision/mana.rs::this_spell_cast_condition_allows`; actual declaration retention in `game_loop/combat_decisions.rs`; `PreventAllDamageEffect` and `prevention_helpers::register_prevention_shield`; live `DamageFilter` matching and damage processing; `turn::execute_cleanup_step`.

### Holds, with no unsupported-body overclaim

- Oketra's Avenger (`8f064160-3afe-408a-85b4-b335eae8571c`): a concrete source mismatch remains. `effects/permanents/exert.rs::execute` uses `Until::ControllersNextUntapStep`; `effects/restrictions.rs` stamps `untap_step_object = Some(object_id)` for that duration, so the restriction follows the object's changing controller. Exert must instead retain the exerting player's next untap. The existing temporary-prevention fixture explicitly holds this controller/expiry boundary and reflexive recipient closure. A full declaration/exert-choice/trigger-resolution/decline/removal/control-change/next-untap gate is required before admitting the complete body. This patch does not repair or admit it.
- Glittering Lion (`549e6de7-56e9-4f5c-8c88-30e446bc53bb`) and Glittering Lynx (`890ffe31-642f-46e3-9f09-c744351653b5`): evidence hold, not a claimed parser failure. Owners exist for unqualified self prevention, `semantic_line_parsing/activated.rs` any-player permission, `effect_sentences/gain_ability.rs` quoted ability loss, `lower_ability_removal_modifications`, and layer-6 exact-template `static_ability_matches_loss`. Existing compilation alone does not establish that the quoted removal template equals the live prevention payload or that opponent-paid {3}/{2} activation removes only that rule, survives source/control changes correctly, preserves the activated ability, and restores prevention at cleanup. Those complete-body activation/removal/expiry scenarios are not authored here; both remain held.
- Thunderstaff (`1a88a378-051c-42b4-bd0e-9c40ae8efea6`): evidence hold, not a claimed parser failure. Fixed combat prevention has a shared typed matcher, SourceIsUntapped and generic conditional-static composition are the relevant qualifier owners, and ordinary paid activation/PumpAll is the second-sentence owner. The exact qualifier-to-prevention composition is not established by a full-body source gate here. Admission needs a single complete-body gate showing untapped current-controller one-point prevention, creature/combat/player-only domains, {2}+tap payment disabling prevention, all attacking creatures of every controller gaining +1/+0 at resolution, fixed affected-set/source departure behavior, removal/phasing/control changes, and end-of-turn expiry. No fragment-level prevention test can replace this combined gate; no source widening was attempted.

No changes to family ledger, source coverage, accounting, measured-success inventory, publication, or remote state. All authored gates remain UNRUN.

## Original runtime-only compatibility analysis (superseded below)

The changed owner is only `decision/mana.rs::player_was_attacked_this_step`, used by native `ThisSpellCastCondition::YouWereAttackedThisStep`. Before this patch it was also redundantly ORed into `CreatureIsAttackingYou`; that call is removed, leaving the original current-combat branch unchanged. A dedicated source unit gate asserts past-history versus live-set separation, absent combat frame returning false, and later-combat exclusion. The independent spell-cost `CreatureIsAttackingYou` owners in `static_abilities/cost_modifiers.rs` are unchanged. Grammar's generic predicate fallback `you_were_attacked` is also unchanged and is not the exact named cast-restriction route; no credit is claimed for it.

No compiled model, serialized variant, schema, artifact version, lowering, or compiler-output change is authored. The existing core `ThisSpellCastRestrictionKind` named label `during declare attackers step if you were attacked` still maps through `static_abilities/model_interpreter.rs` to the native timing plus `YouWereAttackedThisStep` condition. The shared runtime interpretation changes for artifacts already carrying that label, so no artifact rebake is necessary for this runtime correction, but future deferred verification should include old-materialized-artifact compatibility. Artifact wire round-trip and native game cloning are authored gates, not executed compatibility evidence.


## Step-instance correction and current compatibility handoff

Independent review found that multiple DeclareAttackers steps can occur within one combat via `add_step_after`. The first commit's phase-key lookup was therefore insufficient; its two-card recommendation is held until re-review. Phase-wide `players_attacked_in_combat` remains untouched for melee and is no longer the fog permission owner.

The new native owner is `CombatState.last_attack_declaration_step_players: Option<BTreeSet<PlayerId>>`. `None` means absent/uncommitted evidence, `Some(empty)` means a completed declaration with no direct-player attacks, and nonempty sorted players are exact direct-player defenders. It is not reconstructed from current attackers, phase-wide history, public JSON, or historical checkpoints. `player_was_attacked_this_step` reads this owner only during the exact phase/step and fails closed when absent.

Entry/commit lifecycle:
- `turn_runner.rs::DeclareAttackersDecision` synchronizes combat, clears step evidence in both runner and game copies before querying the new declaration, including added steps in the same combat and isolated added combat steps.
- `turn.rs::legacy_enter_step` clears evidence before granting priority in each actually entered DeclareAttackers step. Skipped steps do not commit a declaration.
- `game_loop/combat_decisions.rs::apply_prepared_attacker_declarations_after_tapping_with_dm` publishes evidence only after all declaration costs/choices complete and surviving declarations are known. It unions direct-player defenders for teammates or multiple same-step successful transactions. Existing attackers and put-onto-battlefield-attacking objects never supply declaration evidence. Empty commits establish Some(empty); rollback and uncompleted choices publish nothing.
- `mark_combat_phase_started` and `end_combat` clear step evidence. Attacker removal and target changes do not erase completed facts. Native combat clone, TurnRunner state, Grand Melee turn lanes and RuntimeSavepoint/ReplayCheckpoint roots carry this field naturally.

Genuine public projection changes are authored, with version constants deliberately left for the coordinated compatibility worker:
- `PublicAuditCheckpoint.last_attack_declaration_step_players: Option<Vec<u8>>`, serialized at top level as `lastAttackDeclarationStepPlayers` for the active normal lane.
- `SyncGrandMeleeCombat.last_attack_declaration_step_players: Option<Vec<u8>>` for every Grand Melee combat lane.
- `sync_last_attack_declaration_step_players` emits null for absent, [] for committed empty, and sorted player IDs for committed evidence. The projection is digest input, not a restore format. No generic gameplay serialization or inferred historical defaults were added.

This supersedes the initial no-public-projection-change claim. Coordinated public digest10/current audit28 handling is required before admission/publication; this worker does not bump constants. Compiled artifact14/definition schemas and Manabrew wire shapes are unchanged, because no compiler definition field or serialized card rule changes.

Additional UNRUN gates cover full-body permissions across runner-added and legacy-added declaration steps in one combat, preserved existing attackers and melee history, absent versus committed empty, shared-team aggregate/union semantics, failed declarations, native Grand Melee lane switching, native savepoint root/exchange/clone and replay checkpoint preservation, and public active/lane projection distinctions. All checks remain authored source only.


Lifetime precision: `last_attack_declaration_step_players` describes the latest begun attack-declaration step in the current combat, not arbitrary current-step history. The record remains available through later combat steps and native recovery until another declaration-step entry or combat reset. Its only gameplay reader checks both Combat phase and DeclareAttackers step before reading; leaving that step closes fog permission even while the record remains populated. Both schedulers erase the old record before a later declaration step can expose a decision or priority. The public field is consequently named `lastAttackDeclarationStepPlayers`. The source gate explicitly retains a populated record at DeclareBlockers and asserts the complete fog remains illegal.

The existing competing-attack-costs transaction regression now also asserts that the new step record remains absent after a first sacrifice is rolled back when the second attacker cannot pay. This is the post-payment rollback gate, distinct from the new duplicate-attacker prevalidation negative. Both remain UNRUN.
