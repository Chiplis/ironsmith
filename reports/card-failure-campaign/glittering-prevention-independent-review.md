# Glittering Lion / Glittering Lynx independent source review

Reviewed checkpoint: `e33834ff9c7cbbfe9dd2d0e7638bad545a5e4bf0` against `113e2e3acb01aa8df269ec4594d092b235bcddee` in `ironsmith-glittering-prevention-bodies`. Also independently reviewed corrective delta through final checkpoint `8909757e430bdc80b744565d2b359eaaf558734e`.

## Disposition

Bounded source-only clearance at `8909757e430bdc80b744565d2b359eaaf558734e`: the sole requested evidence correction is resolved. No production defect established and no remaining concrete source blocker found within the reviewed scope. The proposal has meaningful end-to-end authored gates rather than ability-ID-only assertions. This is not build, test, gameplay, compatibility, recovery, measurement, or accounting approval.

## Finding G1: native payment receipt is not asserted

At the reviewed checkpoint, `crates/ironsmith-compiler-runtime/tests/glittering_prevention_bodies.rs:100-130` records payment prompt actors and checks stack controller, but never inspects `StackEntry::mana_spent_on_activation`. Exact initially funded pool exhaustion and the insufficient-payer test are good payment evidence; they do not detect loss or corruption of the authoritative receipt when finalizing or cloning the stack entry.

The real public owner exists at `crates/ironsmith-engine/src/game_state.rs:3741`. Native commit accumulates the spent-pool delta in `game_loop/priority_mana.rs:883`; finalization copies it into the stack entry in `game_loop/priority_cast.rs:7279`. Add exact total and colorless receipt assertions for Lion {3} / Lynx {2}, covering each successful activation and preservation across the existing stack clone. All test funding is colorless, so total plus colorless amount pins every other color to zero. The author has received this finding and is preparing a source-only correction.

This is a test-evidence gap, not an established production payment bug.

### G1 resolution at 8909757e430bdc80b744565d2b359eaaf558734e

Independently inspected the correction. The shared successful-activation helper now reads the real committed receipt, checks exact total/colorless amount and zero in each other color, validates original source ID, native ability flag, no tap/X cost, source snapshot controller and activator distinction. The shared resolution helper repeats exact receipt checks before and after cloning the game/stack. Every successful activation call reaches these checks, including repeated activations, source-controller changes and departed source resolution. Cancellation requires an empty stack. The APIs and field types exist in the reviewed owner source (`controller_of` returns `PlayerId`, snapshot controller is `PlayerId`, `StackEntry::is_ability` and the receipt are public). No receipt is synthesized or mutated by the tests. G1 is resolved on source inspection; the assertions remain UNRUN.

## Independently checked source evidence

- Scope is exactly four added files: runtime tests, tools tests, two-row fixture, and source report. No production/schema/model/artifact format/accounting modifications.
- Fixture Oracle IDs, mana costs, creature Cat metadata, stats and full two-clause texts match the preceding six-card frozen fixture in `ironsmith-prevention-evidence-bodies/fixtures/prevention_evidence_bodies.json.fixture`. Runtime tests pin these exact values and compile the full metadata-bearing input.
- Strict direct and artifact routes invoke separate compiler entry points with `allow_unsupported=false`, separate loss capture, artifact validation before/after JSON round-trip, and independent materialization. Runtime definitions reject unimplemented content. Tools call `compile_strict_snapshot_from_payload`; its owner calls `parse_card_payload(payload, false)`, not the Oracle-only fallback owner.
- `derived_view.rs:1028` scans every battlefield source, filters each ability with controller permission or `allows_any_player_to_activate`, and supplies relevant source IDs to `decision/legal_actions.rs:1364`. The controller-only helper is not proof of an opponent-activation bug.
- Activation execution rechecks legality in `game_loop/priority_apply.rs:829`, takes the actor from priority, captures source snapshot and original announced ability, and follows the actual pending payment pipeline. The tests use native `PriorityResponse::PriorityAction`, `ManaPaymentContext`, plan ID/request hash confirmation, and `apply_decision_context_with_dm`; they do not replace the printed cost or effect.
- Both route definitions are exercised with controller A and opponents B/C, exact cost funding, a second paid activation, and insufficient opponent mana despite funded controller. Pool assertions are present after successful and cancelled payment. Existing stack controller assertions distinguish activator from permanent controller.
- `keyword_static/costs_replacements_and_permissions.rs:3685` maps the unqualified self sentence to `PreventAllDamageToSelf`; quoted loss uses `gain_ability/granted_component_readings.rs:346` and the shared static reader. `semantic_line_parsing/activated.rs:336` preserves any-player permission separately from effects.
- `subject_verb_middle.rs:4396` produces individual concrete removal templates; runtime `continuous.rs:5879-5908,6343` compares the exact object/static ability, subject only to unrelated banding/hexproof families. Printed activation executes through native stack resolution. Independent flying and successful repeated activation are adversarial witnesses that all abilities were not erased.
- Native `PreventAllDamageToSelf` at `static_abilities/misc.rs:3329` emits a repeatable `DamageToSelfMatcher` replacement tied to source and controller. Authored tests assert remaining damage, native prevention event amount/source/controller, no one-shot shield, unaffected player/witness recipients, other protected Cat identity, varying damage/combat flags, source owners/zones and unpreventable bypass.
- Damage-assignment inspection is supplemented by actual `Effect::deal_damage` execution and marked-damage assertions before removal, after paid removal and after cleanup. Those targeted gates intentionally do not run an SBA pass; they are not a complete turn simulation.
- Current-controller mutation, phasing legality, before/after-resolution leave/reentry and distinct incarnation IDs are covered. `effects/helpers.rs:467` resolves exact source object identity and does not generically follow stable card identity. `apply_continuous.rs:76` uses native resolution; stack resolution ignores invalid-target execution rather than redirecting to a returned incarnation.
- `turn.rs:1332` clears marked damage and end-of-turn effects, then refreshes continuous state. Authored restoration checks exercise this owner while preserving independent forever-flying.
- Cancel uses native `priority_mana.rs:1188` rollback. Stale unaffordable activation uses native recheck/error rollback. Tests clone game and pending priority state at payment boundaries and clone the stack before resolution. Assigning `game = before` is separately a clone/checkpoint restoration assertion, not claimed as a second engine cancellation API.

## Verification limits and holds

Source reads only. No builds, tests, probes, formatters, corpus, code generation, remote writes or accounting changes were performed. All authored gates remain UNRUN, including compile/type correctness and runtime/artifact compatibility. Both cards remain original unadmitted measured successes with zero residual/new-measurement/recovery credit. Source-proposal eligibility and any future execution/admission decision remain separate.
