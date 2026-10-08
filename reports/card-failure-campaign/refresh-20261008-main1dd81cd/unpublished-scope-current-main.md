# Bounded unpublished-scope source audit

Date: 2026-10-08 UTC. Checkout: `1dd81cd84c62f272479f26e16d74719fff24b97b`.

This is read-only source inspection with one ignored report written. No compiler, tests, probes, corpus, codegen, source changes, remote writes, old-object reconstruction, or coverage measurements were performed. The missing NEXT07 tree/commit was not recovered or replayed. Card-level compilation and runtime behavior remain subject to fresh evidence from the authorized refresh. Line references below are relative to this checkout.

## Findings at a glance

- Halfdane: an exact end-of-next-upkeep duration is still absent from the central duration enum. The existing next-upkeep runtime predicate expires at upkeep beginning. An isolated grammar rejection is present, but that does **not** establish how the complete card body is admitted, rejected, or under-modeled.
- Ilysian Caryatid: the old immediate-versus-paid mana execution asymmetry remains in source. Immediate execution still iterates the default-effect view of a ResolutionProgram. Whether this exact card now encodes its conditional in those defaults or in program segments requires fresh compiled-body evidence.
- Witch's Clinic: the generic native invalid-activation-target cancellation gap remains in the inspected call chain. The appropriate contract remains cancellation plus rollback and fresh announcement, not retaining the old pending activation to retry it.
- Thunderstaff: held/unadmitted fixture evidence remains; reusable prevention and conditional abstractions exist, but full-body untapped/paid-tap/pump interaction is not established by this bounded inspection.
- Adventure Awaits and Inscribed Tablet: the user's compositional viewed-library refactor changes the relevant grammar architecture. There are suitable look/reveal, optional/mandatory selection, and actual-hand-arrival receipt primitives. They need current exact-body and execution evidence; old bespoke recipe repairs must not be replayed blindly.

## 1. Halfdane

Evidence:
- `crates/ironsmith-core/src/effect.rs:223–248`: Until includes YourNextUpkeep, YourNextTurnEnd, etc., but no exact end-of-next-upkeep occurrence/cutoff variant.
- `crates/ironsmith-engine/src/continuous.rs:4995–5007`: YourNextUpkeep becomes inactive when the captured effect controller is active on a later turn and the step is Upkeep or Draw. It does not remain active throughout that upkeep.
- `crates/ironsmith-engine/src/continuous.rs:1640–1663`: permanent removal is turn-history based; its prose about upkeep end must not override the actual earlier cutoff in the predicate above.
- `crates/ironsmith-compiler-grammar/src/effect_sentences/clause_dispatch/become_clause/dynamic_base_values.rs:363–374`: an authored test rejects `equal to 4 until the end of your next upkeep` in an isolated become-clause parse. This test was not run.

Consequences and limits:
The exact temporal requirement is not supplied by substituting YourNextUpkeep. Full Halfdane document routing was not executed, so an earlier compile-success result cannot be explained or treated as semantic success from this isolated rejection. A fresh repair would need evidence for controller capture, next qualifying occurrence, skipped/extra upkeeps, exact end cutoff, and leaving that upkeep through an ending procedure.

Adjacent source concerns still needing fresh validation:
- `crates/ironsmith-compiler-grammar/src/grammar/keyword_static_lines/copy_shapes.rs:369–430`: ordinary optional entry-copy reads its filter through sentence-end/except and emits until_end_of_turn=false; no explicit generic Until rejection is visible in this shape. `:441–489` separately recognizes only the exact as-enters `until end of turn` form. `crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs:6047–6053` turns that boolean into optional EndOfTurn. This is a fail-closed audit target, not proof that the complete parser accepts malformed durations (filter parsing may reject them).
- `crates/ironsmith-engine/src/effects/player/end_turn.rs:67–84`: ending effect discards prior triggers, exiles stack, then sets runner pending state; the runner owns following SBA/cleanup. No ordering claim is made without tracing/testing the runner and exact future duration cutoff.

## 2. Ilysian Caryatid

Evidence:
- `crates/ironsmith-engine/src/game_loop/priority_apply.rs:247–269`: mana_cost.is_none() selects immediate cost payment (this means no mana-payment cost, not no tap/other cost).
- `priority_apply.rs:310–339`: additional effects are executed using `for effect in &effects_to_run` and execute_effect.
- `crates/ironsmith-core/src/resolution_model.rs:413–426`: Deref and IntoIterator for &ResolutionProgram expose flattened_default_effects. Thus the immediate loop does not itself select resolution-program conditional alternatives.
- `crates/ironsmith-engine/src/game_loop/priority_mana.rs:2314–2348`: staged/paid mana execution instead invokes execute_resolution_program with the whole pending.effects.

Classification: source-level asymmetry remains, not superseded by the refactor. Exact Caryatid compilation, mana classification, threshold predicate placement, one-versus-two mana result, and choice/replacement resumability are unverified. If its condition is nested inside a default effect after refactoring, the existing immediate loop could execute that nested condition; therefore the asymmetry alone is not a new card-specific failing run.

## 3. Witch's Clinic

Evidence:
- `crates/ironsmith-engine/src/game_loop/priority_apply.rs:31–56`: invalid stored targeting requirements produce ActionCancelled.
- `:1335–1355`: activation response takes pending_activation, then uses `build_target_assignments(...)?`.
- `:1458–1463`: closure errors roll back only for a counter-removal declaration or IncompleteEvidence; ordinary invalid-target ActionCancelled has neither qualifier.
- `:113–141`: outer transaction wrapper's restore_on_pending covers action starts and specified cost responses, not Targets; only incomplete execution has unconditional restoration.
- `:1477–1483`: spell targeting explicitly rolls back on analogous assignment failure, highlighting the activation asymmetry.
- `crates/ironsmith-engine/src/game_loop/priority_state.rs:1230–1246`: rollback_action consumes the checkpoint and discards suspended cast/activation state.

Classification: the native generic gap still exists in source: an ordinary invalid activation target can drop pending_activation while leaving the native action checkpoint. This is not an executed Clinic reproduction. A correct future regression should assert ActionCancelled, pre-action rollback/checkpoint consumption, then a **fresh announcement**. Do not restore pending merely to let the invalid announcement retry. WASM has additional live-action-chain rollback (`crates/ironsmith-wasm/src/wasm_game_impl/runtime_flow.rs:223–245`), so native and frontend behavior must not be conflated.

## 4. Thunderstaff

Evidence:
- `fixtures/prevention_evidence_bodies.json.fixture:58–67`: exact card body remains source_candidate=false, validation_status=UNRUN.
- `architecture/card-failure-next-series-06-source-admission.md:17`: existing documentation explicitly holds Thunderstaff and other unadmitted prevention bodies.
- `crates/ironsmith-compiler-grammar/src/keyword_static/damage_prevention.rs:42–125`: general prevention parses source, recipient, combat restriction, threshold, amount into PreventMatchingDamageSpec.
- `crates/ironsmith-core/src/static_ability_model.rs:85–94`: that payload has no dedicated prevention-owner-untapped field. This does not prove absence of a conditional wrapper: the static system has other conditional composition routes.

Classification: fresh evidence required; no reason from this audit to newly admit it. Verify the complete compiled body retains the prevention owner's untapped gate (distinct from the damaging creature's filter), each combat damage event's one-point prevention, loss of prevention after paying tap, paid activation costs, and the +1/+0 attacking-creature effect. No source repair or interaction test was attempted.

## 5. Adventure Awaits

The old named recipe approach is superseded as the starting point for analysis:
- `crates/ironsmith-compiler-grammar/src/effect_sentences/looked_procedure.rs:1–17` describes statement-by-statement composition replacing named sequence-registry programs.
- `looked_procedure.rs:353–373` preserves private Look separately from public RevealTop/LookThenRevealTagged.
- `looked_procedure/selections.rs:84–134` retains card filter, reveal_chosen, and choice count; optional wording converts a positive minimum to up_to.
- `selections.rs:507–581` emits the choice, reveals only chosen cards when authored, and moves chosen cards to hand.
- `selections.rs:325–456` gives the alternate put-from-among path equivalent explicit filter/count/tag/move components.

Classification: plausible reusable support, no exact-body success claim. Confirm full Adventure Awaits selection remains optional and private except the chosen revealed creature, unchosen cards go to bottom in authored order, and fallback draw attaches to actual card arrival rather than choosing/revealing. The fallback condition linkage after the remainder statement was not established by this bounded audit. Empty/no-creature/decline/replaced movement and direct-versus-decoded execution remain fresh-evidence needs.

## 6. Inscribed Tablet

- The same viewed-procedure reader distinguishes public reveal (`looked_procedure.rs:353–373`). For revealed groups with a deferred hand choice, `selections.rs:298–315` chooses LookThenRevealTagged, preserving public disclosure before choice.
- `selections.rs:108–110` only relaxes the minimum for optional actor wording; this provides a route for an authored mandatory selection to retain its count. `:551–581` propagates that count/filter into tagged library choice and actual movement.
- `crates/ironsmith-engine/src/effects/outcome_recording.rs:25–76` creates CardsPutIntoHand from committed ZoneChange-to-Hand receipts, grouped by recipient and restricted to cards, separate from draw receipts.
- `crates/ironsmith-engine/src/effects/composition/if_effect.rs:265–298` checks instruction_result execution facts for CardsPutIntoHand and recipient/filter/count. This does not merely count selected or revealed objects.

Classification: generic receipt infrastructure already exists; exact card's binding to the intended move remains unverified. Do not add a duplicate receipt abstraction or substitute choice success for actual hand arrival. Verify full-card mandatory land choice when available, no-land fallback, public three-card reveal, random bottom remainder, movement replacement, interrupted choices, and direct-versus-decoded parity. `crates/ironsmith-compiler-runtime/tests/put_into_hand_results.rs:1,29–58` contains neighboring authored/unvalidated scenarios based on a four-card fixture set that does not include these two requested cards; their existence is neither a pass nor card coverage.

## Recommended evidence boundary

Keep these six items as current-main investigation candidates. The authorized refresh/comparison owns compiled artifacts and numerical status; this report establishes neither new coverage nor runtime correctness. If subsequent implementation is authorized, prioritize the two narrowly demonstrated generic runtime asymmetries and the precise Halfdane model requirement, while deriving library/prevention repairs from fresh exact-body artifacts under the current compositional architecture.
