# Sinister Concierge: source-only HOLD

## Scope and disposition

- Exact baseline: `1dd81cd84c62f272479f26e16d74719fff24b97b`.
- Oracle ID: `9b14c20f-c4ee-42ea-99cd-099b4eb25883`.
- Frozen content hash supplied by the fresh inventory: `456f35c2c05e5f792826a766597ed5b023faa7371af917e2f099d4da3b95b8fe`.
- Source: baseline `reports/current-refresh-20261008/analysis/current-unresolved-entries.json`, exact ID entry. This packet copies both complete reminder-bearing raw Oracle text and normalized Oracle text into `fixtures/sinister_concierge_hold.json.fixture`.
- HOLD, not a repaired or supported card. No production parser, lowering, runtime, registry, status database, generated artifact, or admission data changed.
- Newly authored grammar and direct/artifact rejection tests are **UNRUN**. No builds, tests, probes, corpus runs, formatting tools, or code generation were run. No remote write was performed.

## Frozen complete body

> When this creature dies, you may exile it and put three time counters on it. If you do, exile up to one target creature and put three time counters on it. Each card exiled this way that doesn't have suspend gains suspend. (For each card with suspend, its owner removes a time counter from it at the beginning of their upkeep. When the last is removed, they may cast it without paying its mana cost. Those creature spells have haste.)

The fresh inventory reports strict parse failure at the last non-reminder sentence, with `unsupported complete negated restriction clause (clause: 'each card exiled this way that doesnt have suspend gains suspend')`. Oracle-only fallback failed with the same diagnostic. That observed baseline diagnostic is retained unchanged in the fixture; no post-change execution is claimed.

## Source diagnosis

1. `crates/ironsmith-compiler-grammar/src/grammar/activation_restrictions.rs`, `parse_negation_candidate`, excludes `if you` result conditions and control/own predicates. It does not recognize the relative predicate `that doesn't have suspend`.
2. `grammar/effects/typed_clause_heads.rs`, `classify_typed_clause_head`, calls that scanner before action dispatch. There is no earlier classified action in the final sentence, so it selects `Restriction` rather than the affirmative `gains suspend` head.
3. `effect_sentences/clause_dispatch/clause_dispatch_core.rs` rejects an unowned complete restriction instead of reaching generic affirmative fallback. This currently provides the strict failure boundary.
4. A scanner-only exemption is insufficient. The head classifier also scans the prefix for any action word, and `have` is classified as `Gain`. If a relative negation were skipped, that relative `have` could then hide a later genuine main negation, for example `Each card exiled this way that doesn't have suspend can't be cast.` A correct repair must separate relative predicates from main-action ownership, not globally ignore `doesn't`/`have`.
5. Existing grant machinery is only a building block. `lowering_impl/runtime_static_ability_helpers.rs::suspend_exile_triggered_abilities` produces exile-zone upkeep and last-time-counter triggers; the upkeep uses `PlayerFilter::You`. Proving which player `You` denotes for granted abilities on cards owned by different players is required before claiming owner-upkeep semantics. The helper alone does not establish that proof.
6. `semantic_line_parsing/lines/lines_trigger.rs` has a distinct `SecondSpellSuspend` program with one triggering-object tag, four counters and a conditional suspend grant. Its single-object provenance cannot simply be reused for Concierge's optional graveyard self-exile plus a separately targeted exile.
7. `effect_sentences/zone_counter_helpers.rs` documents prior-action `It` rebinding to one concrete snapshot tag. That is not evidence that the two successful exile receipts in this complete body are unioned. Neither a shared exile-zone filter nor the latest `It` tag is sufficient.

## Authored evidence, all UNRUN

- Grammar rejection of the complete final clause keeps the unsupported owner fail-closed without asserting that its current mistaken head classification is semantically correct.
- Genuine main restrictions retain `Restriction` and disallow affirmative fallback: plural non-untap, main `does not have suspend`, and exiled-card `can't be cast`.
- Runtime integration admission guard compiles both complete frozen raw and normalized bodies through `compile_to_runtime_definition` and `compile_to_artifact`, requiring rejection on each route.
- No positive lowering/runtime/artifact-materialization evidence is claimed or fabricated: there is no accepted complete program to materialize. The strict artifact rejection is the current boundary.

Tests are in `grammar/activation_restrictions/tests.rs` and `crates/ironsmith-compiler-runtime/tests/sinister_concierge_hold.rs`. The HOLD guard is deliberately temporary: replacing rejection with success is appropriate only together with the complete evidence below.

## Minimum bounded implementation and release evidence

1. Add a complete grammar owner for the affirmative gain-suspend clause with a typed relative ability-absence predicate. Scope relative predicates before classifying any main action or negation. Preserve genuine restrictions both alone and following that predicate. Reject unsupported/trailing syntax without partial consumption.
2. Bind the death source's graveyard object, optional self-exile receipt, target declaration (zero or one), target-exile receipt, and counter placements independently. Each counter placement follows its own successful moved object identity. The conditional branch follows the rules-correct result of the optional self action, not the later counter count or a stale reference.
3. Build the grant's exact successful exile set from both receipts, including self when zero targets were chosen. Exclude unrelated cards in exile, earlier resolutions' receipts, failed/replaced moves and stale object incarnations. Do not use all cards in exile or only the most recent exile tag.
4. Materialize suspend for each still-applicable card lacking it, with per-card owner upkeep, last-counter optional free casting, creature haste, and proper exile-zone lifetime. Retain existing suspend without duplicate triggers. Establish how keyword membership is represented for printed and granted suspend rather than equating it blindly with a payable alternative cast.
5. Author and execute fullbody parser/AST, lowering, admission, and direct-versus-serialized-artifact runtime scenarios. Include: decline self exile; no chosen target; legal target owned by another player; chosen target made illegal before resolution; self object moved before resolution; exile/counter replacement; existing suspend; two distinct owners' upkeeps; last-counter accept/decline; creature haste; unrelated exiled cards; multiple resolutions; and leave/reenter identity changes. Assert separate counters and receipts after each step. Respect the all-targets-illegal resolution rule instead of resolving the self branch unconditionally.
6. Negative grammar witnesses must include `Each card exiled this way that doesn't have suspend can't be cast.`, normalized/contraction variants, malformed relative predicates and unsupported suffixes. The positive relative exemption must not turn their main negated actions affirmative.
7. Only after those scenarios pass may the strict HOLD guard be replaced, admission changed, and any corpus improvement count claimed. Execution and publication remain separately gated by the coordinator's permissions.
