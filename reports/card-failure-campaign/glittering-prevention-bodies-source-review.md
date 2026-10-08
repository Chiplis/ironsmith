# Glittering Lion / Glittering Lynx: source-only evidence checkpoint

Base: 113e2e3acb01aa8df269ec4594d092b235bcddee. Both identities were original unadmitted measured successes. Zero residual or new-measurement credit. No accounting admission.

## Early finding

No bounded production defect has yet been established. Exact self prevention already lowers to the native unit `PreventAllDamageToSelf`. Quoted loss uses the shared granted-ability grammar and `lower_ability_removal_modifications`, which produces `RemoveAbility`, not `RemoveAllAbilities` when the concrete template exists. Layer 6 compares the exact static template (with unrelated keyword-family exceptions). The activation parser retains the any-player restriction; native legality chooses the priority player as payer for this permission. These owners must be exercised together through the whole printed body, not replaced by an internal removal effect.

## Holds

All source gates are authored UNRUN. No builds, tests, probes, formatters, corpus runs, code generation, remote writes, or accounting changes. Source evidence alone does not establish gameplay success. Native paid activation, exact template removal, expiry, source/controller changes and rollback are being authored. No production/schema/model change is currently justified.

## Completed source trace and authored gates

No production change was warranted by independent source inspection. In particular, the controlled-battlefield helper is not evidence of an opponent permission defect: `derived_view.rs::simple_battlefield_mana_analysis` scans every battlefield source, filters individual ability indices with `allows_any_player_to_activate`, and supplies relevant source IDs to `decision/legal_actions.rs::add_battlefield_actions`. The activation's payer/controller comes from the priority player in `game_loop/priority_apply.rs`, including the original announced cost and source snapshot. `decision/legal_actions.rs` also uses that actor for affordability. This trace avoids a false-positive repair to the battlefield candidate scan.

Other owners checked:

- `keyword_static/costs_replacements_and_permissions.rs::parse_prevent_all_damage_to_matching_permanents_line`: unqualified `this creature` yields native unit `PreventAllDamageToSelf`.
- `effect_sentences/gain_ability.rs` and `gain_ability/granted_component_readings.rs::read_static_ability_line`: quoted loss templates route through the same static-line reader.
- `semantic_line_parsing/activated.rs`: trailing any-player permission is retained as activation restriction rather than swallowed as an effect.
- `lowering_impl/compile_support/effect_dispatch/subject_verb_middle.rs::lower_ability_removal_modifications`: concrete object abilities become individual removal templates; empty-template fallback is not a substitute for this complete body.
- Engine `continuous.rs::static_ability_matches_loss`, `object_ability_matches_loss` and layer-6 application: exact template comparison preserves the activated ability and unrelated abilities. The model-to-runtime generic removal path is distinct from the older static-only variant; no claim equates their Rust types.
- `static_abilities/misc.rs::PreventAllDamageToSelf`: repeatable native DamageToSelfMatcher replacement, tied to source identity and current controller, not a spent shield.
- `effects/continuous/apply_continuous.rs::resolve_target`, `effects/helpers.rs::resolve_source_object_id`: non-target source resolution retains exact object identity; zone changes create a new incarnation and must not redirect the old activation.
- `turn.rs::execute_cleanup_step`: clears end-of-turn continuous effects and marked damage.

`fixtures/glittering_prevention_bodies.json.fixture` copies the exact metadata-bearing bodies and Oracle IDs from the preceding six-card evidence fixture. Runtime definitions are independently compiled by strict direct and strict artifact routes, reject lossy/unimplemented results, validate the artifact before and after JSON round-trip, materialize separately, and assert full costs/types/subtype/stats plus both printed abilities. Tools also author a strict full-payload gate with no Oracle-only fallback.

The runtime source gates author: repeatable varying combat/noncombat damage; every source controller and battlefield/stack/graveyard source zone; exact self versus other Cat/witness/player recipients; unpreventable bypass and native prevention events; real {3}/{2} payment by controller and both opponents; summoning-sick activation without a tap cost; protection until resolution; loss of only the quoted rule while independent flying and the repeatable activation survive; end-of-turn restoration; payment-controller preservation across source control change; phasing and zone legality; departing/re-entering source before and after resolution; current-controller event ownership; insufficient opponent mana despite funded controller; native cancelled-payment rollback; native game and pending-action cloning; checkpoint restoration; stale unaffordable action rejection; actual marked-damage execution before/after paid removal and cleanup.

No printed effect is replaced by a hand-built removal or permission. The one independent flying grant is an adversarial witness to exact removal. Source-only review may nominate both as evidence candidates; gameplay admission remains held until authorized execution and independent source review. All authored gates are UNRUN.

## Compatibility and explicit holds

No production parser, runtime, core/model, artifact wire, schema, version, compiler-output, generated artifact, audit support classifier, or accounting change is authored. Existing artifact materialization must execute the same two clauses; the independent round-trip route is a future gate, not executed compatibility evidence. No artifact rebake or model migration is proposed. No measured-success inventory, central coverage, family ledger, or recovery count changed. Both remain original unadmitted measured successes with zero residual/new-measurement credit.

Holds: execution (including compile/build/test), aggregate admission, measurement/accounting, publication and all remote writes. No builds, tests, probes, formatters, corpus, codegen or remote actions were run. Reading source and writing/committing source-only evidence are the extent of this checkpoint.

## Independent review correction: native activation receipts

The first checkpoint asserted payment prompts, pool consumption and stack controller but omitted direct assertions on the committed native receipt. This correction reads `StackEntry::mana_spent_on_activation` (`game_state.rs`), which `priority_mana.rs` accumulates from the actual pending activator's payment and `priority_cast.rs::ActivationStage::ReadyToFinalize` copies into the committed stack entry. No test constructs or writes a receipt.

Every successful paid activation now asserts exact native receipt total and colorless amount (Lion 3, Lynx 2), zero spending in the other five colors, source identity, activator controller and source-snapshot controller distinction for opponent activations. The shared resolution helper repeats exact receipt assertions before and after native game/stack cloning, covering repeated activations, changed controllers and departed sources as well. Cancelled payment must leave no committed stack receipt. All remain authored UNRUN, with unchanged execution, admission, accounting and publication holds.
