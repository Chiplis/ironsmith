# Bioshift source gate (UNRUN)

Base: ff30f190c72b80b9c212d30056d402e94d4f2c86. Scope is one frozen original-gate identity, Bioshift (`3da604fc-68e9-4749-98f9-6fbcebcab9b6`). This is an unadmitted measured recovery; historical compile success is not whole-body gameplay evidence. No measurement, residual credit, or accounting change is claimed.

The dedicated fixture copies exact metadata and the entire Oracle body from the frozen source JSON. Inspection found existing shared owners for AnyNumber, distinct target roles, the donor-relative controller relation, and native transactional counter mutation. No card-name special case or owner change is justified by inspection so far.

Independent strict direct and validated-artifact source scenarios are being authored. All new scenarios are UNRUN. No build, test, compiler probe, formatter, code generation, corpus execution, remote write, or accounting change is authorized or performed.

## Review-ready source outcome

Evidence-only patch: exact frozen fixture and `crates/ironsmith-compiler-runtime/tests/bioshift_source_gate.rs`. The fixture distinguishes the Oracle ID above from printing ID `6e18f7a9-2af6-467a-8f62-5f7da83a3c92`; includes mana cost `{G/U}`, Instant type, colors/color identity G/U, and the entire unchanged sentence. No trailing ability is omitted.

Each scenario independently invokes direct compilation and artifact compilation under separate parse-loss capture. The artifact path discards its accompanying runtime definition, JSON-roundtrips, validates, and explicitly materializes the retained artifact. Both paths reject unimplemented content. Neither a successful parse nor a retained effect fragment is treated as gameplay evidence.

Authored whole-body scenarios cover:
- Real public cast, both explicit single-target roles, native pending-cast cloning, and hybrid mana payment.
- Zero, partial, and all +1/+1 counter allocations; unrelated charge counters survive on both endpoints; two opposing-player-controlled endpoints are legal together.
- Same-object and different-controller target declarations are refused through the public target submission API.
- Either endpoint departing, either endpoint's controller changing, and both controllers changing together are rechecked at resolution. Only the last case retains a legal matching pair.
- Either endpoint leaving and returning uses the native new ObjectId (with the same stable identity); stale selected target roles must not rebind or offer an amount choice. Both the returned incarnation and untouched endpoint retain their counters.
- Pending amount selection rolls back native stack/counters/one-shot replacement while preserving casting payment. Both the original and a native clone retry through the full stack-resolution owner; placement doubles once while removal stays one.
- Empty donor and zero choice leave the one-shot replacement unused.
- Checked destination overflow errors roll back removal, stack assignments, and replacement consumption; a native zero-choice retry settles without replaying failed mutation.

## Shared-owner inspection

`effect_sentences/verb_handlers/zone_move_verbs.rs::parse_move_counted_counters` retains `CounterMoveAmount::AnyNumber`; `parse_counter_move_destination` binds `PlayerFilter::ControllerOf(ObjectRef::Target)` to the donor rather than the caster. `game_loop/targeting.rs::prior_object_controller_recheck_tests` already documents the current-donor-controller target legality owner. `effects/counters/move_counters.rs` resolves assigned endpoint roles, skips invalid/self endpoints, obtains a bounded number before mutation, and composes removal plus placement transactionally. `game_loop/stack_resolution.rs::resolve_stack_entry_full` checkpoints the native game and restores on error or pending choice. Existing helper scenarios are supporting owner evidence only, never substitutes for the new exact Bioshift paths.

No bounded shared-owner defect was established by static inspection, so production source is unchanged. No schema, protocol, accounting, or generated file changes are needed for this evidence-only patch. Forgotten Ancient, Goldberry, Resourceful Defense, and Slippery Bogbonder remain outside this single-destination body.

## Validation boundary and admission

All seven new test functions and their parameterized scenarios are authored **UNRUN / UNVALIDATED**. Only source reading, exact frozen-row extraction, local text authoring, and Git bookkeeping were performed. No test/build/probe/formatter/corpus runner/codegen/remote write was performed. A future authorized validation pass must compile and execute both paths, then independently review runtime results before any admission decision. This patch itself earns zero new measured recovery or residual credit and does not change the original-gate ledger.
