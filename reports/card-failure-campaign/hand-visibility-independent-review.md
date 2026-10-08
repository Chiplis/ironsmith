# Independent source-only review: public revealed hands

Final reviewed commit: `e7b17f36841df70ab85dfc9a0c5897252aeb705f` against `94e075587`.
Initial review: `2405dc24e8fe8312ede49107ca3d1139d54d88a1`; correction review: `efceebcae526fb68188dbf2ff6cc19490719b9c9`.
Repository: `ironsmith-opponent-hand-library`.
Outcome: **bounded source-clear after corrections**. Both initial test-oracle defects and an intermediate JSON-key mismatch are resolved by source inspection at the final commit. No concrete remaining defect was found in the assigned source scope. All authored scenarios remain UNRUN / UNVALIDATED.

This is sufficient as an independently reviewed, bounded authored-source proposal under the implementation-first campaign rule. Execution is not a prerequisite for proposing source eligibility. The separate accounting owner decides actual proposal-count admission; no count is changed here. Runtime/gameplay recovery remains unverified.

## Resolved findings (initial and intermediate commits)

### 1. Resolved P1: Snapshot oracle reverses the actual hand display contract

`crates/ironsmith-wasm/src/public_revealed_hand_source_tests.rs:39-43` compares `player.hand_cards` to forward `subject.hand.iter()` order. The actual snapshot owner deliberately renders hand cards in reverse zone order: `ui_snapshot.rs:715-719` passes `true` to `IncrementalZoneCards::update`, and `ui_snapshot.rs:324-328` reverses the resulting ordered vector.

This is not only missing future coverage. The first inactive-zone scenario already creates a two-card Alice hand: the private witness at line 60 followed by the source card at line 65 for `Zone::Hand`. Alice can see her own hand, so the first Alice/Alice assertion compares reversed actual IDs with forward expected IDs and fails if execution reaches it. This stops the entire snapshot test before active visibility, phasing, membership changes, ability removal/expiry, World/lethal removal, and rollback assertions execute.

Correct only the `hand_cards` oracle to reflect the reverse display order (or compare exact identity sets if order is intentionally out of scope). Do not reverse `persistent_look_cards`: that owner uses forward hand order. This conclusion is traced from source, not an executed failure.

### 2. Resolved P2: Crypto oracle asserts existence, not the claimed exact public requirement set

`crates/ironsmith-wasm/src/public_revealed_hand_source_tests.rs:112-122` uses `.any(...)` for each expected window/opening. It neither bounds the public requirement count nor compares the full normalized tuples. Consequently, correct expected requirements accompanied by additional incorrect public windows/openings pass. When inactive, the negated predicates reject only exact matching records: a stale opening with a wrong commitment or owner, or a stale hand window with the wrong count, passes. The library check excludes public openings for the four known library IDs, but does not exclude an unwanted library view window.

This undercuts the test's “exact public hand proofs” name and the report's “Exact public view windows and public opening commitments” claim. Compare the actual normalized public-window and public-opening sets against explicitly expected sets, including cardinality and relevant owner, viewer/visibility, zone, object ID, slot, commitment and count fields. Expect no such public requirements while inactive and no library windows/openings. Keep unrelated requirement kinds out of the comparison if necessary. No runtime leak is claimed; this is a false-negative test oracle.

## Correction verification at the final commit

- `hand_cards` expected identities now use `subject.hand.iter().rev()`, matching the actual display owner. `persistent_look_cards` remains forward.
- Crypto now compares the complete sorted serialized requirement vector to an independently authored expected vector: exactly four public hand windows and four public openings while active, no requirements while inactive. Vector equality preserves cardinality and rejects duplicates, extras, wrong owners/zones/commitments, library exposure, and stale records. An explicit inactive baseline was added.
- The intermediate correction used snake_case expected keys despite `CryptoRequirementView` declaring camelCase serialization. Final keys `objectId`, `originSlot`, and `originCommitment` now match that owner. Expected optional-field omission, viewer, visibility, reason, IDs, origin binding, and counts match the source constructors.
- The final diff remains restricted to source tests/registration, dev dependencies/lock edge, fixture, and report. Final working tree was clean when inspected.

## Positive source findings

- The patch adds tests, their `cfg(test)` registration, two dev-dependency entries, one lockfile edge, the frozen fixture, and a report. It contains no production behavior edit.
- Fixture identities match `/workspace/scratch/bc560e8d90ff/unadmitted-measured-recoveries.json`: Revelation `251ff4e4-3be4-424e-8dd7-eb4ede7b415c`; Wandering Eye `59015068-6868-4235-981b-96137123c685`. The inventory labels both `source_coverage_status: unaddressed` and `verified_compile_recovery: false`; it is not changed.
- Frozen bodies are complete for this packet: Revelation `{G}`, World Enchantment, “Players play with their hands revealed.” Wandering Eye `{2}{U}`, Creature — Illusion, 1/3, “Flying” followed by the same global hand clause. No external/corpus refresh was performed to certify a new source version.
- Both helpers separately invoke `compile_to_runtime_definition(..., false)` and `compile_to_artifact(..., false)` under parse-loss capture. The artifact-returned runtime sibling is discarded. Artifact JSON serialization/deserialization and real materialization form the second route. API implementation inspection confirms those are separately invoked compiler paths, rather than passing one compiled definition twice.
- Compiler-runtime assertions pin exact mana costs, colors, no color indicator, supertypes, types, subtypes, power/toughness, ordered static ability IDs, absence of spell effects/unimplemented content, and full compiled text. They validate round-trip equality and reject a corrupted checksum both during validation and materialization.
- World removal and Wandering Eye 1/3, flying versus ground/reach blockers, and lethal-damage removal use actual engine APIs. Hand visibility in compiler-runtime tests alone is only a live-source query, but the web-session tests separately reach actual snapshot and crypto owners using both original bodies.
- The snapshot module is registered under `ui_snapshot` and the web-session library points to the wasm implementation file. Added dev dependencies make the compiler APIs available to that test owner; the hand-authored lock edge is consistent with the new compiler dependency by inspection, not Cargo verification.
- Snapshot scenarios are authored for all 16 viewer/subject pairs, inactive source zones, controller changes, phasing, empty/replaced hand membership, actual remove-all-abilities continuous effects and end-of-turn cleanup, native `PriorityLoopState::save_checkpoint` / `rollback_action`, and source termination. The retained snapshot cache is reused. The initial finding that blocked later scenarios is now corrected by source inspection.
- Crypto scenarios call actual audit capture/update owners over hidden placeholders and invoke native rollback, rather than only inspecting ability metadata. They cover control, phasing, ability removal/expiry, departure, and restoration, with the exactness limitation in finding 2 now corrected.

## Limits and eligibility

This was read-only source inspection with an out-of-repository report write. No build, test, probe, formatter, code generation, corpus operation, source edit, remote write, or accounting edit was performed. Repository status was clean when inspected.

No compilation, gameplay, browser cryptographic verification, or aggregate gate success is asserted. Native requirement-generation assertions do not prove browser-side cryptographic proof acceptance. Cycling all four perspectives through one cache also changes perspective render keys each round; retained-cache use should not be described as exhaustive same-perspective incremental-cache coverage.

These two cards are existing measured-main successes lacking original-source admission evidence under the assigned task context. The corrected packet is **bounded-clear for authored-source proposal eligibility**, with actual proposal-count decisions reserved to the accounting owner. It earns **zero new measured recoveries and zero residual reduction**. No verified runtime/gameplay recovery or executed gate pass is claimed. The implementation-first campaign forbids execution before its source-proposal threshold; this review does not impose execution as a circular prerequisite to source-proposal admission.
