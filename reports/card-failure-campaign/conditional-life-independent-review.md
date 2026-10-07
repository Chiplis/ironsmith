# Independent conditional-life complete-body source review

Review target: `d82eca235fcd14347f74466674f0d6ee511849d6`, against `ff30f190c72b80b9c212d30056d402e94d4f2c86`.
Checkout: `ironsmith-conditional-life-bodies`.
Scope: only Tezzeret's Simulacrum (`dd886509-7455-4c8f-976f-0f5fafcb97be`) and Necra Sanctuary (`2586a59d-8501-4c22-9d69-f4bf91de7024`).

## Initial disposition: blocked pending source correction

One concrete source-level blocker prevents accepting the authored Necra coverage at this head.

### B1: Upkeep helper starts from an incoherent phase checkpoint

`crates/ironsmith-compiler-runtime/tests/conditional_life_loss_bodies.rs:51-55` initializes `Phase::FirstMain`. Its `upkeep` helper at lines 107-113 only changes active player before constructing `TurnRunner::from_state_for_sync(TurnState::Upkeep)` and advancing.

`from_state_for_sync` (`crates/ironsmith-engine/src/turn_runner.rs:911-914`) only installs runner state. The Upkeep branch (1144-1168) installs `Step::Upkeep`, but does not install `Phase::Beginning`. `generate_step_trigger_events_for_active_players` (`crates/ironsmith-engine/src/triggers/check.rs:4494-4500`) requires the pair `(Phase::Beginning, Some(Step::Upkeep))` to emit BeginningOfUpkeepEvent. Therefore the helper emits no upkeep event from this fixture's state.

Consequences: positive cases at test lines 204-208 and 218 fail their expected stack size before reaching the intended body. Negative trigger cases are vacuous. Necra branches of the illegal-target and rollback/retry cases attempt to resolve an empty stack. This is a source deduction, not an observed test result.

Required correction: establish a coherent Beginning-phase checkpoint before advancing Upkeep, retaining the real turn runner and trigger-stack dispatch. Add phase/step assertions to make this prerequisite explicit. Parent and author were notified before this report was written. No production repair is indicated by this finding.

## Other inspected evidence

- Diff contains only the architecture report, the exact two-body fixture, and one authored test module. No production, accounting, ledger, measured-result, or residual gate changes.
- `definitions` calls strict direct compilation and strict artifact compilation separately with `allow_unsupported=false`, checks captured loss, validates and JSON-round-trips the artifact, and materializes it. A separate native codec encodes the direct runtime definition, JSON-round-trips its wire definition, then materializes it. The native route is codec recovery from the direct route, not a third independent parser invocation.
- Full expected executable rules text is independently authored per body and rendered from the structured runtime definition through `canonical_compiled_lines`. Mana cost, card types, subtype, P/T, colors, identity, ability count, and one self-replacement are asserted on all three routes.
- Concrete parser owners inspected: LoseLife is in `subject_verb_player_action_player_mut`; explicit-player carry and nested follow-up carry feed `default_effects_for_self_replacement`; `post_rule_future_zone_and_self_replacement` binds the true-arm player and moves default effects into the false arm. The leading conditional attachment/classifier paths are present.
- Concrete execution owners inspected: `execute_resolution_program` selects default effects or a matching self-replacement branch, not both. Representative target assignments are supplied before branch execution. `LoseLifeEffect` resolves its player specification, builds a non-damage life-change proposal, and exposes its targeting spec.
- Real Simulacrum activation uses public priority/decision continuation APIs, inspects stack source/controller/target/payment receipts, actual tap and activation history, and checks immediate reactivation exclusion. Authored scenarios cover the selected third player, controller-relative planeswalker subtype/type/zone/phasing, arrival/departure/control changes, and source departure after payment. Negative admission uses actual legal-action computation.
- Necra scenario expectations correctly distinguish outer OR from inner AND, single multicolored versus separate permanents, controller-relative witnesses, source departure, and live gate changes. Trigger checking verifies intervening-if before publication; stack resolution checks it again with the captured ability controller. These authored scenarios require B1's correction to reach those owners.
- Sole illegal-target resolution is handled before body execution by stack target validation. Native resolution checkpointing restores GameState and stack on errors or `awaiting_choice`. Life-change transaction owners also retain pending/failed replacement execution atomically. Authored retry coverage preserves the already-paid tap and one-shot replacement, checks no partial earlier life gain escapes, and checks exactly one final replacement result.
- The test APIs, relevant imports/reexports, public codec paths, return types, controller changes, token budget, life/event counters and state clone mechanisms were checked against concrete source. No further concrete source blocker was identified in this pass.

## Verification boundaries

All test cases remain **UNRUN / UNVALIDATED**. No build, test, probe, formatter, corpus replay, codegen, or remote write was performed. Frozen fixture text and metadata were reviewed directly; the external compressed corpus was not replayed or independently re-extracted in this review. Native recovery means runtime-definition codec round trip and in-memory GameState/PriorityLoopState cloning, not full-game/network persistence. Any eventual source-proposal clearance is separate from executed gameplay, measured recovery, admission/accounting credit, and unrelated sibling bodies.

## Final delta review: bounded source-proposal clearance

Corrected exact head: `6de7cdbb4f863a67877690b0491a5af92dbe6943`.

The delta from `d82eca235fcd14347f74466674f0d6ee511849d6` changes only the test helper/import and architecture correction note. The helper now sets `Phase::Beginning` and `Step::Untap` before restoring the Upkeep runner, advances the real runner, and asserts `Phase::Beginning`, `Step::Upkeep`, and `TurnState::UpkeepPriority` before calling `put_triggers_on_stack_with_dm`. This supplies the prerequisite of the concrete trigger dispatcher and closes B1. The prior blocked disposition is superseded for this corrected head only.

**No remaining concrete source blocker identified. Bounded source-proposal clearance is granted for these two exact bodies at this head, based on reviewed production source plus authored UNRUN coverage.** This does not assert that the test file compiles or passes, that gameplay has executed successfully, or that measured recovery/admission/accounting credit has been earned. All execution and unrelated bodies remain outside this review. The checkout was clean at the corrected head when inspected. No production changes are present.
