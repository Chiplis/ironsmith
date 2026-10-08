# Independent bounded static-head reconciliation review

Verdict: **CLEAR for source-only integration.** No correction requested. This is not an executed test pass.

Reviewed isolated checkout `ironsmith-static-head-reconciliation`, exact HEAD `6c6dc496423c0a3be41ff848d0bb61017ce30a86`, against `e22c96108`. The complete delta changes only the repeated-static-head test matrix and its source-only report: two insertions and one deletion in the test, plus the 23-line report. No production, fixture, model, serialization, version, or accounting files change. Working tree was clean when inspected.

## Positive assertion

The exact supported Kormus Bell line now appears in `repeated_static_heads_preserve_land_animation_filters_and_pt`:

“All Swamps are 1/1 black creatures that are still lands.”

The existing `type_and_color.rs::parse_land_animation` consumes the whole subject, fixed 1/1, `black creatures` descriptor, exact still-land suffix, and sentence completion. `costs_replacements_and_permissions.rs::complete_characteristic_subject` accepts the complete Swamps subject after its optional All quantifier and supplies battlefield scope. `parse_lands_are_pt_creatures_still_lands_line` recognizes black and creature and emits the existing creature addition, black setter, and base 1/1 setter.

Registry semantics justify the new assertion: `keyword_static/mod.rs` keeps unrestricted nominal-head eligibility for this specialist, registers its multi-static adapter, and preserves the specialist AST in that adapter. `assert_reachable` checks index eligibility, direct Match, registry Match, equality of complete AST payloads, and whole-static-parser acceptance. It is stronger than a mere no-error assertion and exercises the exact routing contract intended here. Adjacent generic identity/type-addition owners require their own complete descriptor/tail shapes; they are not a reason to preserve an obsolete rejection for the already-supported line. Existing copular tests independently author exact Kormus registry acceptance.

## Negative assertion

The replacement appends “and have flying.” after the otherwise valid still-land suffix. This is a correct negative for this bounded specialist. `parse_land_animation` reaches `semantic_finish` immediately after lands; `semantic_finish` permits semantic noise and then requires EOF. Its noise set cannot consume “and”, “have”, or “flying”. Therefore the full-shape fact fails, the specialist returns `Ok(None)`, and `static_multi_rule_outcome` maps that to `ParseOutcome::NoMatch`, exactly as the changed negative matrix expects. The existing copular grammar negatives already author this same unsupported-tail boundary.

The separate “until end of turn” refusal is unchanged and remains justified for the same complete-tail reason. This review does not broaden the negative into a claim that every possible whole-card route must reject a granted-ability sentence; the matrix intentionally checks this specific specialist.

## Scope and evidence limits

The accompanying report accurately states UNRUN source status, zero new source/residual/measured credit, and no production changes. Ambush Commander and Kormus Bell already have source proposals; this repair reconciles the authored test oracle only. No builds, tests, probes, formatters, corpus runs, code generation, or remote writes were performed for this independent review. Future authorized execution remains necessary. Integration belongs to the later packet as requested, not a claim about already-published PRs.
