# Independent source review: Lesson graveyard predicate repair

Reviewed commit: b38192bf05ed62bfdd84a8a3251c658dfaca1f91
Base: 8ac862bff90d9ffbde89195187369d5ccf283a9e
Checkout: /workspace/scratch/bc560e8d90ff/ironsmith-lesson-graveyard
Review date: 2026-10-08 UTC

## Verdict

One blocking regression-test defect. The narrow production parser repair is consistent with the inspected lexer and word-view contract, and no introduced production semantic defect was identified. Do not label this patch gate-ready until the test baseline is repaired and the authorized successor boundary executes its gates. ALL TESTS UNRUN. This review ran only read-only source/data inspection and saved this review; no builds, tests, probes, corpus/compiler runs, code generation, source edits, or remote writes.

## Blocking finding: pre-existing hand decoy invalidates exact reward assertion

P1, crates/ironsmith-compiler-runtime/tests/intervening_predicate_cohort.rs:560 and :583; helper :120–127 (hand assertion :122–123).

The newly added complete_lesson_bodies_route_contractions_and_recheck_exact_owned_graveyard creates a Lesson in B's hand as a wrong-zone decoy. It then calls assert_death_reward, whose invariant is that every hand starts empty. Consequently every new case expects B's hand count to be zero or one while the actual count starts at one; a successful Dragonfly draw produces two. The first false-at-death/true-later case already violates the assertion for either card. This is a source-demonstrable harness failure, not evidence of incorrect game behavior. Preserve the hand decoy and compare exact deltas against captured baseline hand counts (or explicitly parameterize the helper's baseline); retain exact life and other-player invariants. Do not weaken to an inequality or remove the wrong-zone control merely to satisfy the assertion.

## Parser and token contract

- crates/ironsmith-compiler-syntax/src/lexer.rs:20–30 normalizes curly apostrophes to straight apostrophes in parser spelling; Word regex around :98 retains contractions as word tokens.
- lexer.rs:428–436 matches token words against normalized parser spelling. Both straight and curly input therefore satisfy the token guard's "there's" spelling.
- lexer.rs:475–550, especially :539–544, strips apostrophes while constructing word pieces. TokenWordView sees "theres" for either spelling; the unpunctuated input also yields "theres".
- predicate_phrases/capture_shapes.rs:121–169 performs WinnowSequence parse_full on clause.word_refs() and requires word-input end; predicate_phrases/surface.rs:9–15 and :34–49 likewise operate on TokenWordView with EOF.
- advanced.rs:5125–5133 correctly centralizes [there,is], [there,are], and [theres] for these word-view consumers. The old apostrophe spelling cannot match the normalized word view. Both readers now use the same normalized alternatives at :5175 and :5280.
- Contracted token guards at :5169 and :5264 now also cover already-normalized "theres", rejecting non-word/non-comma tails before the word-only captures. Curly apostrophes are covered by is_word normalization. Location capture is complete and constrained; unsupported trailing words cannot simply be dropped by the changed capture.
- Expanded "there is" remains the existing two-word route. The special non-word guard remains contraction-specific, as before. Therefore this patch does not establish global token-complete strictness for all historical expanded forms; in particular it adds no expanded-head standalone-operator regression. This is a coverage/scope caveat rather than an introduced change.
- Conjoined independently articled objects still lower as two requirements (And), with each existing object-filter parser preserved. No card-name special case, global rewrite, permissive fallback, or runtime schema was added.

## Exact semantics and runtime wiring

advanced.rs:5325–5335 retains filter.zone = Graveyard and filter.owner = You, returned as PlayerControls(You, filter). The subtype-specific fallback remains an existing strict two-token subtype/card reader at :5149–5161; Lesson is represented by the exact Lesson subtype, without a spurious creature/type requirement. The grammar assertions compare the whole expected filter for contractions and preserve expanded-form coverage.

crates/ironsmith-compiler-resolve/src/predicate_conditions.rs:191–198 converts PlayerPredicateAst::PlayerControls through the existing shared Condition::PlayerControls path, resolving its player and filter. crates/ironsmith-engine/src/condition_eval.rs:3612–3617 iterates the requested zone; :3658–3670 selects owner for nonbattlefield zones, controller only for battlefield/default; :4009–4029 then applies the exact object filter. The predicate's legacy name does not change graveyard ownership semantics.

The departed-source trigger route in crates/ironsmith-engine/src/triggers/check.rs:2909–2922 checks intervening_if using source_snapshot.controller, retaining the last controller for a stolen source. crates/ironsmith-engine/src/game_loop/stack_resolution.rs:1551–1571 rechecks against live state using entry.controller and suppresses the effect when false. This patch does not change either path.

## Authored evidence inspected, not passed gates

Runtime definitions_with_oracle (:33–55) retains actual metadata, captures parse-loss independently for direct and artifact compilation, validates the artifact, serializes/deserializes JSON, checks equality, and materializes the restored artifact. Its two results are direct and artifact-restored definitions, not two direct conversions.

New tests (:535–595) cover both full bodies with straight/curly contractions and expanded "there is", exact lowered Condition equality, false-at-death/true-later suppression, true-at-death/false-at-resolution suppression, replacement Lesson success, and a source owned by A but controlled by B. Decoys cover A's graveyard, B's hand, exile, and a non-Lesson graveyard card. Invalid full-body tails must fail direct and artifact compilation. The reward helper intends exact one draw/two life and unaffected other players, but the baseline blocker above prevents this new matrix from serving as passing evidence.

Inherited full-body tests retain actual printed mana cost/types/P-T and no-unimplemented assertions (:130 onward), Lesson owner/zone/teammate exclusions and stolen-source reward recipient (:365 onward), live replacement/cloned-game resolution checks (:399 onward), Dragonfly flying/ward and noncreature/nonland power exclusions plus controller switching (:424 onward), Sentries reach/deathtouch, and actual ward payment/nonpayment with teammate exclusion (:470 onward). These are source-authored tests, all UNRUN for this review.

Tools regression checks both exact oracle IDs, actual full metadata, raw bodies and reminder-stripped bodies, three existential surfaces, StrictCompiled, !parse_lossy, and !has_unimplemented. It does not grant oracle-only fallback permission. compile_strict_snapshot_from_payload calls parse_card_payload(payload, false); the regression's strict/lossless assertions reject any lossy fallback outcome.

## Preserved measured dataset authentication

Read preserved files under ../ironsmith-refresh-20261008-1dd81cd/reports/current-refresh-20261008. Recomputed SHA-256 by read-only hashing:

- data/cards-current.json: bae465b9d536fffa24c656daff5577a87f5a963dcb2160be0c1dc9ba8e225750, matching analysis/dataset-summary.json.
- analysis/current-unresolved-entries.json: 3e89d38bd63c3d906ce6cd46835c0aeed3c4e57a6af3cdf0368cb60fd0ff1e9f, matching analysis/manifest.json.
- analysis/input-index.json: 2e0d1e4e8ec63258515595dcce73cedc7b472c9568bef0cd22f9fb5d24a7ed9d, matching analysis/manifest.json.

Extracted the two retained data rows read-only and compared all fixture fields: name, oracle_id, mana_cost, type_line, power, toughness, and full raw oracle_text match exactly.

Dragonfly Swarm: 83abf1d0-04e4-49e5-bf63-59f77829ffd1; {1}{U}{R}; Creature — Dragon Insect; */3; full flying/ward {1} reminder, noncreature/nonland graveyard power, and Lesson death draw. Current unresolved row at :15366 records parse_failed/parser_failure and content hash e96329e842ac2a47b2b567e2f5dafad5cf5e545f41e86c6eb9eba1b84604f7cf.

Walltop Sentries: fb3a0910-a582-41ef-b5b9-3cda1f1cd5ce; {2}{G}; Creature — Human Soldier Ally; 2/3; reach/deathtouch and Lesson death gain 2 life. Current unresolved row at :67831 records parse_failed/parser_failure and content hash 5c0d2b7823811c5804d29526de64d7683e1d07b9c0a301bffd1934ba345088a1.

Both retained diagnostics fail the contracted intervening-if on the metadata-bearing parse_input route and on historical oracle_only retry. Dragonfly's measured normalized text strips only its ward reminder, matching the tools regression's normalized body. analysis/audit-historical-source-proposals.md:27 and :41 identify both as historical intervening-predicate-cohort proposals.

## Boundary and accounting

This is historical revalidation, not two new original-majority completions. No support count or gate result has changed on this source-only evidence. Existing artifact16/audit30 outputs remain immutable and cannot certify the new compiler provenance. Repair the test baseline, integrate the final reviewed source commit, then execute authorized focused and aggregate gates and generate/audit a fresh exact-head successor boundary. Until then, runtime correctness and measured support remain unestablished.

## Follow-up review: blocker resolved at 26c0d38fc2e1d1b49e1ce18ce26f0d6ad7377b92

Independently inspected the bounded diff against b38192bf05ed62bfdd84a8a3251c658dfaca1f91 on 2026-10-08. This section supersedes the earlier blocking verdict for the corrected head; the earlier finding remains accurate for b38192bf. No additional borrow, API, or assertion defect was identified by source review. ALL TESTS STILL UNRUN; no build, test, probe, corpus/compiler run, code generation, or source edit was performed.

Exact corrected anchors: crates/ironsmith-compiler-runtime/tests/intervening_predicate_cohort.rs:564–569 capture baselines and expected draw identity; :589–603 enforce the exact reward matrix. The hand decoy at :560 remains. All four players' initial hand and library collections are cloned and life totals copied before the death, so these snapshots do not retain a game borrow. In the final loop, state and game.object both borrow game immutably; only the owned local library snapshot is mutated. assert_eq! borrows its operands, so state.library is not moved from a borrowed Player.

API verification: player.rs:773 and :806–807 define i32 life and ZoneSequence hand/library. zone_sequence.rs:30–45 provides independent persistent clones and equality by the ordered ID sequence; :67–80 supplies iter/last/contains; :157–165 pops the last element of the owned snapshot. ids.rs:20–35 defines Copy player/object/stable identifiers. Thus the iterator, copied values, vector indexing, local pop, equality, and life arithmetic match the inspected API.

The initial game seeds three Artifact cards into each library at cohort :68–69, making B's initial last-card unwrap valid in every scenario. game_state.rs:8597–8614 (draw_cards) and :8622–8648 (draw_cards_with_dm) use library.last() as the top and perform the zone move. zones_and_characteristics.rs:1386–1390 carries the old object's stable identity while allocating its new zone-change ObjectId; the special stable-ID reset at :1563–1566 is for combined permanents, irrelevant to these plain Artifact cards. Comparing the newly added hand card's stable ID against the old library-top identity is therefore the appropriate assertion.

The corrected checks preserve every pre-existing hand ID and enforce both exact length delta and exact newly added ID count; added[0] is accessed only after asserting one added card in a successful Dragonfly case. That card must match B's prior top stable identity. The expected library removes exactly its last element only for that same draw case, and the complete ordered library identity sequence is compared for every player. Life changes by exactly two only for successful Walltop/B and by zero otherwise. False-at-death, false-at-resolution, all other players, and Walltop's no-draw cases retain their full baselines. Existing zero-baseline scenarios keep their separate original helper unchanged.

Current source-review verdict at 26c0d38fc2e1d1b49e1ce18ce26f0d6ad7377b92: the reported harness blocker is resolved; no further blocking source finding identified within the reviewed scope. Execution, fresh exact-head successor-boundary validation, and historical-only accounting remain required exactly as above. This is not a passing-gate or measured-support claim.
