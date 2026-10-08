# Exert player tenure: source-only checkpoint

Base: 113e2e3acb01aa8df269ec4594d092b235bcddee. No builds, tests, probes, formatters, corpus runs, code generation or remote writes. All authored scenarios UNRUN. No accounting or credit.

## Independently established owner

Wizards Comprehensive Rules 2026-06-19, 701.43a–d: exert binds the exerting player's next untap, repeated exertions expire in that same step, and attacking exert is an optional attack cost with a linked trigger. Source: https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf (page 138).

`ExertCostEffect::execute` currently creates ControllersNextUntapStep; CantEffect stamps `untap_step_object` and `RestrictionEffectInstance::untap_step_player` follows that incarnation's current controller. This violates exert after a control change. Reuse existing PlayersNextUntapStep with Specific(ctx.controller): it retains exact player, lane-local turn_players identity, step identity, and registration cutoff. Do not change controller-following or YourNextUntapStep owners.

Oketra's Avenger's full metadata/body and oracle_id were independently read from the frozen cards-20261003.json.xz (not refreshed remotely). Dedicated tests will preserve its historical temporary-prevention hold until independent review and execution; no central fixture/accounting promotion.

## Consumer/player proof

The actor is ExecutionContext.controller, not inferred from source ownership. Production constructors found by source search are:

- `static_abilities/combat.rs::ExertAttack::pay_optional_attack_cost` receives the declaring creature's prepared controller, creates ExecutionContext(source, controller) with a cost cause, then invokes ExertCostEffect. `game_loop/combat_decisions.rs` freezes that controller in PreparedAttackerDeclaration and passes it to optional-cost payment. This is the attacking player paying the optional exert cost.
- `compiler-lowering/.../cost_materialization.rs` lowers MaterializationCost::ExertSelf to Cost::effect(ExertCostEffect). `costs/payer_trait.rs::CostExecutionBindings::execution_context` constructs ExecutionContext(self.source, self.payer), explicitly supplying the payer, including activated and mana-ability costs. `game_loop/priority_mana.rs` separately constructs resolution contexts with pending.activator. Thus controller in this context denotes the actor, not an ongoing object-controller lookup.
- The artifact decoder, engine materializer and effect-model interpreter reconstruct the same typed ExertCostEffect payload; they do not select a different player. Native supplementary tests invoke that owner with the explicit actor. No additional production native constructor was found.

## Authored coverage and provenance

The frozen Oketra identity remains `partial_not_counted` in prior campaign evidence. It already has partial family evidence, which is not erased or counted anew. This patch supplies a dedicated full-body candidate gate, not admission. The older temporary-prevention fixture's hold remains historical and is not silently promoted.

Independent direct compilation and artifact compilation each receive the complete metadata-bearing frozen body, reject parse loss/unimplemented content, and the artifact validates and JSON-round-trips before materialization. Tools adds a strict full-payload gate without Oracle fallback. Runtime tests use both definitions for actual attack declaration, accepted/declined exert choice and linked trigger resolution. Cases include damage before trigger resolution, exact self recipient, combat/noncombat/unpreventable domains, repeat amounts, outgoing/other-recipient negatives, cleanup, changed control before trigger resolution, opponent and third-player steps, expiry without control, clone equivalence, suspended declaration rollback, repeated exertions in separate combat frames, departure before/after resolution and new incarnation.

Supplementary native tests distinguish registration during an already-started untap from the next same-turn occurrence, retain cutoff through GameState clones, and distinguish an independent active Grand Melee lane from the exerting player's actual step. These use the real native exert owner and are expressly additional owner gates, not substituted fragments for the full-body tests. All are UNRUN. Actual scheduled extra-combat insertion, browser suspend/replay/exact-image restoration and interactions with skip-untap replacements remain future integrated verification, not claims made by authored scenarios.

## Coordinated compatibility review

- Model: no new core/native field, variant, payload, or schema. ExertCostEffect still contains only display_text. Its runtime interpretation now emits an existing PlayersNextUntapStep receipt rather than a ControllersNextUntapStep receipt with untap_step_object. ControllersNextUntapStep, YourNextUntapStep and other restriction semantics are untouched.
- Artifact: no compiler/lowering/materializer or wire-version edit; already-built artifacts containing ExertCostEffect will execute the corrected native owner when run by the new engine. Artifact JSON tests above are authored, not proof of historical artifact compatibility or equal compiled bytes.
- Public digest: no public-audit schema/version edit. `wasm_game_impl/public_audit.rs` is a redacted projection, not a complete serialization of effect_store restrictions or registration receipts. A matching redacted digest alone cannot establish equal latent exert tenure. The shared fixed-player duration already has a source-authored public effect-carrier test in public_audit_boundary_13_9_26_tests.rs, but that does not prove game-state receipt projection. Coordinated review must decide any engine compatibility/version gate; this patch does not assert mixed old/new peers are safe.
- Replay: identical accepted exert actions intentionally diverge from old engines after a controller change or a registration-cutoff boundary. Historical replay must pin the engine implementation; no replay migration/version claim is made here.
- Exact image/native state: no layout field added. GameState clones naturally carry the existing duration, fixed Specific player, timestamp and untap boundary; clone and declaration rollback cases are authored. Old native snapshots already containing controller-following exert restrictions are not rewritten by this patch and cannot be assumed repaired in place. Exact-image compatibility is held for coordinated review and future execution.

No builds, tests, probes, formatters, corpus/codegen or remote writes performed. No accounting, success count, family ledger, central coverage or publication changes. Independent source review and all execution are pending.
