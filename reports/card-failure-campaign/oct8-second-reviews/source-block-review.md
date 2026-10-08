# Source-only review of 7e4bc0c9470de16d5778c9a37b5437cf5bf64338

Base: ad0b0056cb8f9fe92ee9bb336b8184e1e9d101a8.
Scope: three frozen full-body source-owned must-be-blocked candidates.
Verdict: changes required for complete raw source-token ownership. No build, test, compiler probe, corpus run, code generation, source edit, or remote write was performed. Hashing and JSON equality checks authenticated preserved data only; all authored compiler/runtime tests remain UNRUN.

## Required production and evidence correction

The new source guard at `crates/ironsmith-compiler-grammar/src/effect_sentences/clause_primitives.rs:904–920` validates an already-trimmed subject. `grammar/effects/clause_primitive_shapes/combat_and_duration.rs:135–145` scans arbitrary subject tokens and calls `trim_shape_edges(subject_tokens)` before returning the must-be-blocked shape. `grammar/effects/clause_primitive_shapes.rs:88–107` discards edge comma, period, semicolon and quote tokens. The sentence-boundary check in `combat_and_duration.rs:159–163` then checks only that trimmed subject. `LexedClause::trimmed` also strips commas (`ironsmith-grammar-common/src/grammar/lexical.rs:475–477`).

Consequently, source inspection establishes a loss path for `This creature, must be blocked this turn if able.` and the corresponding period/semicolon/quote forms: punctuation immediately before `must` disappears before the complete-source guard. A subject-final period additionally disappears before the purported cross-sentence rejection. These are newly reachable through the added `this` registry head, not merely an unrelated inherited runtime edge. Existing malformed-source tests use internal mana, colon, or a qualification; those tokens survive trimming and therefore do not discriminate this defect.

Required fix: validate the complete raw source subject before any punctuation trimming, or retain its original token slice and validate that. Preserve existing valid `this creature`/`this permanent` handling, complete suffix ownership and the narrow must-be-blocked registry change. Do not widen attack-this-turn or attacks-or-blocks heads and do not globally change unrelated grammars merely to address this source-specific ownership defect. Documentation alone is insufficient.

Required authored regressions: raw source subjects containing comma, period, semicolon, and quote immediately before the requirement verb must not produce a successful requirement through the specialist or primitive registry. Retain valid exact creature/permanent subjects across ordinary if-able, this-turn, each-combat-this-turn, and this-combat forms, with exact source binding and turn/combat durations. Keep internal mana/colon/unsupported-qualification negatives and unowned suffix negatives. Add strict whole-body malformed-source rejection evidence where the public compiler exposes the same raw form; do not let unrelated unsupported siblings be the sole reason a negative fails. All remain UNRUN until separately authorized execution.

## Authenticated fixture and scope

Preserved baseline `reports/current-refresh-20261008/data/cards-current.json` SHA-256 is exactly `bae465b9d536fffa24c656daff5577a87f5a963dcb2160be0c1dc9ba8e225750`. Each fixture row, including every metadata field and full Oracle body, exactly equals its unique baseline row selected by print ID. Fixture SHA-256 is exactly `03f15def6d0a6449cb12aa5078745ceccf4324dcd03fd2d33054b108f06e4806`.

- Anzrag: Oracle ID `4adcd967-9ff7-4940-b8b9-0c4215bbcb75`.
- Glorfindel: Oracle ID `93842030-2017-4233-a7ac-7112361c019f`.
- Loathsome Catoblepas: Oracle ID `c6f68e4b-af2b-43d5-8052-104defe7f3ec`.

Only one production file changes, plus its inline tests, two new integration-test files, fixture, and report. Attack registry and inherited compatibility descriptors are untouched.

## Full-body evidence and API review

`source_must_be_blocked_lowering.rs:13–32` compiles complete metadata-bearing bodies under strict policy with parse-loss capture and inspects typed source restriction/duration. Lines 35–102 inspect ability counts, modal common pump/source/+1/+1, exact activation pips and absence of invented choices/costs, and additional-combat carrier. Lines 105–122 demand whole-card failure for unsupported extra siblings and malformed requirements. Some sibling lowering assertions are intentionally broad (e.g. maximum-blocker mode nonempty), but runtime assertions below discriminate their actual behavior rather than treating mere presence as sufficient.

`source_must_be_blocked.rs:35–64` performs independent direct and artifact compilation, loss rejection, JSON serialization/decode, validation, decoded materialization, and unimplemented-content rejection. Behavioral scenarios run on both materializations. Inspected referenced API definitions for compile/artifact conversion, decision callbacks, priority/activation handling, declarations, trigger stacking, continuous effect fields, cost introspection, and phase/cleanup calls; no obvious API mismatch was identified by source inspection. This is not proof the tests compile or pass.

- Lines 231–275: exact colored mana requirement, payment before resolution, no tap cost, source-only versus ally declarations, one sufficient versus optional second blocker, third-party blocker rejection, ability removal after resolution, cleanup.
- Lines 278–314: tapped/cannot-block inability, defender separation, source not attacking, blink/new ObjectId with old obligation not migrating.
- Lines 317–373: another attacker being blocked is a nonmatch; one and two blockers produce one source trigger; every controlled creature untaps while enemy stays tapped; one additional combat is queued and consumed; obligation persists and trigger repeats in added combat.
- Lines 376–411: actual opponent/self scry, scry 2 as one event, pump delayed until resolution, one +1/+1 source-only pump, separately discriminated minimum-one and maximum-one modes, cleanup of pump and both restrictions.
- Lines 414–440: two actual scry triggers select different modes, common pump twice, zero/one/two-blocker discrimination, inability waiver, pump expiry.
- Lines 443–479: unrelated death and source exile are nonmatches; source death uses departed identity, opponent-only creature target pools include both opponents and exclude own creature/noncreature; exact -3/-3 only on selected victim; cleanup restores stats.

These authored gates cover the mandatory printed sibling bodies. None may be claimed supported if its complete strict/loss-free/materialization/runtime path fails later.

## Advisory inherited interaction edge

No new test explicitly combines a minimum above one (such as menace) with source must-be-blocked and/or Glorfindel's maximum-one mode. This is an inherited interaction, not a discovered new production defect. Current engine `combat_state.rs:985–1008` validates min/max counts and `:1439–1490` searches for genuinely legal improved requirement satisfaction, including maximum capacity. A focused extra authored test could distinguish: one available blocker plus menace permits zero; two available blockers require two; menace plus maximum one makes blocking impossible and therefore permits zero. Do not broaden this source patch into unrelated combat-engine changes without evidence.

## Compatibility and accounting

This reachability/acceptance change requires a later compiler semantic boundary before baking/publishing artifacts or fresh comparable audits. Preserve inherited artifact15/audit29 descriptors unchanged. Three candidates, zero measured fixes, zero executed tests. The production raw-token defect above is the sole identified required correction; actual test execution and full-card credit remain later gated work.

## Follow-up re-review: e46e0124a8ba6264179b2a5ba1de6f10750263fe

Reviewed exact follow-up against preserved `7e4bc0c9470de16d5778c9a37b5437cf5bf64338`. Worktree HEAD matches follow-up and status is clean. Verdict: the identified raw captured source-subject ownership blocker is corrected in production and the requested discriminating evidence is authored. No additional required correction identified by source inspection. This supersedes the initial changes-required verdict for this two-commit packet; all execution gates remain UNRUN.

- `crates/ironsmith-compiler-grammar/src/grammar/effects/clause_primitive_shapes/combat_and_duration.rs:135–154` now retains the captured raw subject slice only when the must-be-blocked subject starts with `this`. Comma, period, semicolon, and quote immediately before `must` are no longer removed by subject-edge trimming. `:168–172` therefore sees and rejects the retained period. Other retained nonword punctuation reaches the primitive guard.
- `crates/ironsmith-compiler-grammar/src/effect_sentences/clause_primitives.rs:904–922` constructs the subject clause without the former secondary comma trim, then rejects any nonword token or unsupported complete source-reference surface. This closes the second erasure path as well as the shape-level path.
- Other shapes are preserved: attacks-or-blocks and attacks still use their prior normalization at `combat_and_duration.rs:113–132`; non-source must-be-blocked retains `trim_shape_edges` at `:146–147`. That existing trim already removes edge commas, making removal of the primitive's subsequent comma-only trim redundant for those non-source subjects. The complete suffix parser, source-target lowering, and registry head boundary are unchanged in the follow-up. No unintended other-shape regression identified.
- `clause_primitives.rs:1992–2011` authors the requested 2 subjects × 4 punctuation kinds × 4 duration surfaces matrix through both direct specialist and primitive registry. Assertions reject any successful effect, correctly permitting either clean nonmatch (period boundary) or error (other nonword source tokens). The original valid source/duration/source-binding matrix is unchanged.
- `crates/ironsmith-compiler-runtime/tests/source_must_be_blocked_lowering.rs:124–149` mutates only the activation's source punctuation in complete frozen Loathsome Catoblepas, retaining metadata and printed death trigger. Each of the four punctuation variants must fail strict compiler lowering, direct runtime compilation, and artifact compilation independently. Existing valid whole-body controls remain unchanged, so an unrelated deliberately unsupported sibling no longer explains these negatives.
- The follow-up changes two production grammar files plus authored tests and report only. Fixtures, runtime full-body behavior scenarios, attack registry, inherited compatibility descriptors, and all measured accounting remain untouched. Inherited whole-input edge normalization is unchanged; this correction specifically preserves the raw captured `this`-headed subject and fixes the subject-final token loss identified by review.

No compiler/test/build/probe/corpus/codegen execution or source mutation performed during re-review. Original authenticated-fixture findings and mandatory sibling-body assessment remain applicable. Later semantic boundary still required before artifact publication/comparable audit; inherited artifact15 must remain untouched. Minimum-above-one interaction remains advisory coverage, not a new blocker.
