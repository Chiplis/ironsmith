# Independent exert-player-tenure source review

## Disposition

Bounded source-proposal clearance for `fa189ce7e82df26ea30f1ff8af01b1f3aeeac1f8` against `113e2e3acb01aa8df269ec4594d092b235bcddee` in `ironsmith-exert-player-tenure`. No blocking native implementation defect found in the reviewed delta. This means reviewed source plus authored UNRUN coverage only. It does not mean compiled, executed, corpus-admitted, release-compatible, or publication-ready.

Only source/file inspection and reading the cited official rule document were performed. No builds, tests, probes, formatters, corpus execution, code generation, remote writes, accounting edits, or code changes were performed.

## Rules and exact input

Independently read Wizards' June 19, 2026 Comprehensive Rules, 701.43a–d, PDF page 138: https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf . The exerting player's next untap owns the restriction; repeated exertions before that step expire together; exert while untapped is allowed; attacking exert is an optional attack cost with a linked trigger.

Independently inspected the Oketra's Avenger row in the local frozen `fixtures/card-failure-campaign/cards-20261003.json.xz`. The new fixture matches name, oracle_id `8f064160-3afe-408a-85b4-b335eae8571c`, mana cost, type line, power/toughness, and the complete Oracle body including reminder text. No remote Oracle refresh was used. The prior partial/not-counted and temporary-prevention hold are not promoted by this patch.

## Native owner and actor trace

- The sole production behavioral edit is in `crates/ironsmith-engine/src/effects/permanents/exert.rs`: the exact-source untap prohibition now uses existing `PlayersNextUntapStep { Specific(ctx.controller) }` rather than controller-following `ControllersNextUntapStep`.
- `game_loop/combat_decisions.rs` captures `creature_controller` in `PreparedAttackerDeclaration` and forwards that prepared actor to `ExertAttack::pay_optional_attack_cost`. `static_abilities/combat.rs` constructs the exert execution context and cost cause from that actor. The choice and keyword action use the same actor.
- The lowered activated-cost path uses `MaterializationCost::ExertSelf` and the same native effect. `CostExecutionBindings::execution_context` binds `self.payer`. `PaymentExecutionInputs` explicitly classifies controller as local, so restoration of inherited inputs does not overwrite the payer. Artifact decoder/materializer and effect-model interpreter reconstruct the same typed effect, without alternate actor selection.
- `CantEffect` resolves the already-specific player and retains an exact object filter. It does not stamp an object-controller owner for this duration. `RestrictionEffectInstance::untap_step_player` reads the fixed player directly.
- `is_active` requires an actual recorded untap occurrence, the creation timestamp at or before the boundary, Beginning/Untap, and membership in lane-local `turn_players()`. Thus control changes cannot retarget the player, a different Grand Melee lane cannot consume the receipt, and registration after the step boundary waits for the next occurrence even in the same turn.
- `execute_untap_step_inner` freezes consumed receipt timestamps and expires all applicable restrictions together. Consumption does not depend on still controlling the source. The object filter keeps the old incarnation distinct from a returned permanent.
- Other next-untap owner implementations and other restrictions are unchanged by this delta. Existing departure handling already recognizes fixed-player receipts. No new layout or schema field was added.

## Authored coverage and oracle review

The runtime test constructs direct and artifact definitions through separate strict compilation calls using the full metadata/body. Both reject parse loss and unimplemented runtime content. The artifact is validated and JSON-round-tripped before independent materialization. These are meaningful separate transport routes, not a materialized direct definition relabeled as an artifact.

The full-body scenarios exercise actual declaration, accepted/declined choice, linked trigger resolution, no premature prevention, exact recipient, combat/noncombat/unpreventable distinctions, repeat amounts, outgoing/other-recipient negatives, cleanup, control change before resolution, third/opponent untaps, expiry without control, repeated exertions, suspended declaration rollback, and departure before/after resolution with fresh incarnation. The inspected API signatures and return shapes match their uses. The damage helper observes processed assignments and prevented events; it does not claim combat-damage assignment, applied lethal damage, or state-based-action integration.

Supplementary native tests exercise a receipt registered by the real exert executor during an already-started untap and an independent Grand Melee lane. The first occurrence goes through TurnRunner; subsequent same-turn occurrences call the native untap kernel directly. The added-step scheduling call is not itself verified by consuming the schedule through TurnRunner. Likewise, repeated combats are separately established combat frames, not a scheduled extra-combat integration test. These distinctions agree with the author's stated integration limits.

Clone coverage checks resulting behavior and/or latent receipt properties. Suspended declaration checks restored tapping, restriction receipts and stack output, while the production declaration transaction checkpoints GameState, CombatState and TriggerQueue. It does not substitute for browser suspended-choice restoration or serialized exact-image replay.

Nonblocking hardening opportunity: the tools-only strict test checks `StrictCompiled` and `parse_lossy` but not `snapshot.has_unimplemented`. `compile_strict_snapshot_from_payload` does not apply the separate authoritative helper's unimplemented-content rejection. Add that explicit predicate if this tools test is intended to be an independently sufficient admission gate. The two runtime routes already explicitly reject unimplemented content, so this omission does not remove the combined proposal's authored coverage.

## Model, artifact, digest, replay and exact-image boundary

- Core/native ExertCostEffect payload remains display text only. No model, artifact envelope, materializer schema or wire-version edit is made. Existing artifacts containing this payload will receive changed semantics when interpreted by the new engine. JSON round-trip does not establish historical compiled-byte equivalence.
- `crates/ironsmith-wasm/src/wasm_game_impl/public_audit.rs` builds a redacted checkpoint. It has no complete restriction receipt or `untap_step_started_at` projection. Equal public digest values therefore cannot prove equal future exert tenure. The existing fixed-player effect-carrier test verifies a projected effect payload, not latent GameState receipt completeness.
- Accepted actions can replay differently under old and new engines after a control change or registration-boundary case. Mixed-version peer safety and historical replay need an implementation/version boundary; no compatibility clearance follows from an unchanged payload or digest schema.
- GameState clones carry existing duration/player/timestamp/boundary fields. Older native snapshots containing a controller-following exert receipt are not rewritten by this patch. Exact-image restoration and migration remain held, as does mixed old/new execution.
- The wider NEXT06/digest10/audit28 fog step-history work is outside this delta and is being coordinated separately. This review neither supersedes it nor treats its outcome as evidence already obtained.

## Remaining gates

All newly authored tests remain UNRUN. Future authorized execution must establish compilation and both strict/runtime routes, then the agreed aggregate gates. Scheduled extra-combat/extra-untap integration, skipped-untap interactions, browser suspension/replay, exact-image restoration, and release compatibility remain distinct integration work. No ledger or coverage credit is warranted from this review alone.
