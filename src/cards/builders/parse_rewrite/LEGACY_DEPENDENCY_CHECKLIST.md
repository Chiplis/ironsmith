# Rewrite Parser Legacy Dependency Checklist

This file originally tracked the remaining places where the rewrite runtime
still depended on legacy parser/lowering helpers from `src/cards/builders/*`.

That migration is now complete. The checklist is kept as an implementation
record of what was removed or ported into the rewrite/shared support modules.

## Runtime Status

- [x] Runtime entrypoint uses rewrite parsing/lowering.
- [x] Rewrite has its own preprocessing, lexer, CST, and semantic IR.
- [x] Rewrite runtime is fully independent of legacy parser helpers.
- [x] Legacy parser code has been removed from the repository.

## 1. Legacy Text/Token Utilities Still Used

These are foundational helpers imported from the legacy parser stack and used
throughout rewrite parsing/lowering.

- [x] Replace `tokenize_line(...)` usage in rewrite modules.
- [x] Replace `words(...)` usage in rewrite modules.
- [x] Replace `trim_commas(...)` usage in rewrite modules.
- [x] Replace `split_on_period(...)` usage in rewrite modules.
- [x] Replace `parse_scryfall_mana_cost(...)` usage in rewrite modules.
- [x] Replace `parse_mana_symbol(...)` usage in rewrite modules.
- [x] Replace `parse_number_or_x_value(...)` usage in rewrite modules.

Primary usage sites:
- [lower.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/lower.rs)
- [parse.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/parse.rs)
- [leaf.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/leaf.rs)

Completed note:
- `leaf.rs` is also rewrite-owned for card-type/color/supertype/counter-type helper parsing.

## 2. Legacy Clause/Effect Parsing Still Used

The rewrite parser used to delegate its top-level sentence/trigger/static entry
points back into the legacy clause parser tree. Those entry points are now
rewrite-owned and live under `parse_rewrite`, even though the copied subtree
still shares some lower-level helpers that are tracked in later sections.

- [x] Replace `parse_effect_sentences(...)` with rewrite-owned statement/effect parsing.
- [x] Replace `parse_trigger_clause(...)` with rewrite-owned trigger clause parsing.
- [x] Replace `parse_triggered_line(...)` fallback usage with rewrite-owned triggered lowering.
- [x] Replace `parse_static_ability_ast_line(...)` with rewrite-owned static clause parsing.
- [x] Replace `parse_ability_line(...)` with rewrite-owned keyword-action/static parsing.

Primary usage sites:
- [lower.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/lower.rs)
- [parse.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/parse.rs)

Completed note:
- Rewrite call sites now route through
  [clause_support.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/clause_support.rs)
  into rewrite-owned modules:
  [ported_effects_sentences](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/ported_effects_sentences/mod.rs),
  [ported_activation_and_restrictions.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/ported_activation_and_restrictions.rs),
  and [ported_keyword_static.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/ported_keyword_static.rs).
- Section 2 is complete once rewrite runtime no longer calls the legacy clause
  entry points directly. That boundary is now rewrite-owned; deeper shared
  helpers remain tracked under sections 3-5 and 7.

## 3. Legacy Keyword/Mechanic Parsers Still Used

Many keyword lines are only classified by rewrite structure and then parsed by
legacy keyword parsers during lowering.

- [x] Replace `parse_bestow_line(...)`
- [x] Replace `parse_buyback_line(...)`
- [x] Replace `parse_channel_line(...)`
- [x] Replace `parse_cycling_line(...)`
- [x] Replace `parse_entwine_line(...)`
- [x] Replace `parse_equip_line(...)`
- [x] Replace `parse_escape_line(...)`
- [x] Replace `parse_flashback_line(...)`
- [x] Replace `parse_kicker_line(...)`
- [x] Replace `parse_madness_line(...)`
- [x] Replace `parse_morph_keyword_line(...)`
- [x] Replace `parse_multikicker_line(...)`
- [x] Replace `parse_offspring_line(...)`
- [x] Replace `parse_reinforce_line(...)`
- [x] Replace `parse_squad_line(...)`
- [x] Replace `parse_transmute_line(...)`
- [x] Replace `parse_warp_line(...)`
- [x] Replace `parse_cast_this_spell_only_line(...)`
- [x] Replace `parse_you_may_rather_than_spell_cost_line(...)`
- [x] Replace `parse_if_conditional_alternative_cost_line(...)`
- [x] Replace `parse_self_free_cast_alternative_cost_line(...)`
- [x] Replace `parse_additional_cost_choice_options(...)`
- [x] Replace `parse_if_this_spell_costs_less_to_cast_line(...)`
- [x] Replace `preserve_keyword_prefix_for_parse(...)`

Shared helpers still backing part of this keyword/cost bucket:
- [x] Replace `parse_this_spell_cost_condition(...)`
- [x] Replace `parse_this_spell_target_condition(...)`

Primary usage sites:
- [parse.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/parse.rs)
- [lower.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/lower.rs)

Completed note:
- Rewrite now owns the finite mana/cost-driven keyword parsers for bestow, buyback,
  entwine, escape, flashback, kicker, madness, morph/megamorph, multikicker,
  offspring, reinforce, squad, transmute, and warp. These still sit on top of
  the shared activation-cost parser until Section 4 is complete.
- Rewrite runtime now also owns the channel, cycling, equip, additional-cost-choice,
  and spell-cost-condition entry points through rewrite-local modules rather than
  the shared builder surface. Section 3 is complete.

## 4. Legacy Activation-Cost Parsing Still Used

Rewrite has real `winnow` coverage for many cost segments, but several paths
still fall back to legacy cost parsers.

- [x] Replace `parse_activation_cost(...)` fallback in rewrite leaf parsing.
- [x] Replace `parse_loyalty_shorthand_activation_cost(...)` fallback in rewrite leaf parsing.
- [x] Replace `parse_object_filter(...)` fallback-driven activation-cost object filters.

Primary usage site:
- [leaf.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/leaf.rs)

Notes:
- Existing rewrite-owned coverage already handles many common segments.
- The remaining work is mostly loyalty shorthand, richer object-filter-driven
  costs, and the tail of targeted/qualified exile-return-sacrifice patterns.
- Rewrite activation-cost parsing now routes only through rewrite-owned modules:
  [leaf.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/leaf.rs)
  uses rewrite-local `parse_activation_cost(...)` from
  [ported_activation_and_restrictions.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/ported_activation_and_restrictions.rs)
  and rewrite-local `parse_object_filter(...)` from
  [ported_object_filters.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/ported_object_filters.rs).

## 5. Legacy Modal/Level/Header Parsing Still Used

Rewrite owns document structure discovery, but some header/item parsing still
leans on legacy helpers.

- [x] Replace `parse_modal_header(...)`
- [x] Replace `replace_modal_header_x_in_effects_ast(...)`
- [x] Replace `parse_level_up_line(...)`
- [x] Replace `parse_level_header(...)`
- [x] Replace `parse_saga_chapter_prefix(...)`
- [x] Replace `parse_power_toughness(...)`

Primary usage sites:
- [lower.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/lower.rs)
- [parse.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/parse.rs)

Completed note:
- Rewrite modal lowering now uses rewrite-local helpers in
  [modal_support.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/modal_support.rs)
  for modal-header parsing, prefix-effect parsing, modal gate detection, and
  modal-header X replacement instead of calling back into the deleted legacy
  parser adapter.

## 6. Legacy Card-Model Lowering Still Used

This is the biggest remaining migration gap. Rewrite IR still lowers by feeding
legacy AST/lowering infrastructure instead of lowering directly into runtime
`CardDefinition` structures.

- [x] Replace `normalize_card_ast(...)`
- [x] Replace `lower_card_ast(...)`
- [x] Remove legacy `LineAst`/`ParsedCardAst` reconstruction from rewrite lowering.
- [x] Lower rewrite IR directly into `CardDefinition` / `Ability` / `ResolutionProgram`.

Primary usage site:
- [lower.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/lower.rs)

Completed note:
- Rewrite runtime now lowers semantic items directly into `NormalizedCardAst`
  without rebuilding a legacy `ParsedCardAst` or routing through
  `normalize_card_ast(...)`.
- Rewrite runtime now owns the normalized-card lowering loop, the per-line
  application/lowering loop, and the finalization pass in
  [lower.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/lower.rs)
  instead of calling `lower_card_ast(...)`, `lower_line_ast(...)`,
  `apply_line_ast(...)`, or `finalize_lowered_card(...)`.
- Rewrite also now owns the rewrite-facing prepared-effect/ability lowering,
  static-ability lowering, effect preparation, and iterated-player validation
  helpers in
  [lowering_support.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/lowering_support.rs)
  instead of importing those layers from legacy lowering modules.
- Section 6 is complete once rewrite lowering no longer depends on the legacy
  card-level lowering stack. That boundary is now rewrite-owned.
- Remaining shared dependencies are lower-level effect-compiler and clause-parser
  utilities, which belong to other migration sections rather than the
  card-model lowering section itself.

## 7. Legacy Restriction / Post-Parse Helpers Still Used

Rewrite still reuses some legacy post-parse glue when constructing activated
abilities and restrictions.

- [x] Replace `infer_activated_functional_zones(...)`
- [x] Replace `is_any_player_may_activate_sentence(...)`
- [x] Replace `apply_pending_mana_restriction(...)`
- [x] Replace `apply_pending_activation_restriction(...)`
- [x] Replace `parse_mana_usage_restriction_sentence(...)`
- [x] Replace `parse_spell_filter(...)`

Primary usage site:
- [lower.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/lower.rs)

Completed note:
- Rewrite runtime now uses rewrite-local restriction application helpers in
  [restriction_support.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/restriction_support.rs)
  instead of calling the shared post-parse glue from
  [effect_pipeline.rs](/Users/chiplis/ironsmith/src/cards/builders/effect_pipeline.rs).
- Activated functional-zone inference, mana-usage sentence parsing, and
  any-player activation sentence detection are now sourced from
  [ported_activation_and_restrictions.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/ported_activation_and_restrictions.rs),
  and spell-filter parsing is sourced from
  [ported_object_filters.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/ported_object_filters.rs).

## 8. Legacy Differential / Tooling Surface Still Present

- [x] Legacy differential tooling is isolated under test/tooling cfgs.
- [x] Remove any remaining runtime-facing legacy parser reachability.
- [x] Keep only audit/test adapters for legacy comparison.

Primary usage sites:
- [tests.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/tests.rs)

Completed note:
- Rewrite runtime no longer imports helpers from the deleted legacy parser
  module. The remaining sentence-splitting and spell-followup helpers were moved into
  [parser_support.rs](/Users/chiplis/ironsmith/src/cards/builders/parse_rewrite/parser_support.rs),
  and the old audit-only parser adapters were deleted after the shared support
  pieces were moved into `parse_support` and `parse_rewrite`.

## 9. Exit Criteria For “Fully Migrated”

We can honestly say the legacy parser has been fully migrated into
`winnow + logos` when all of these are true:

- [x] Rewrite parsing does not call legacy parse helpers for statements, triggers,
  statics, keywords, activation costs, or object filters.
- [x] Rewrite lowering does not rebuild legacy parser ASTs.
- [x] Runtime parsing/lowering does not call `normalize_card_ast(...)` or `lower_card_ast(...)`.
- [x] Legacy parser code is no longer used and has been deleted.
- [x] `cargo test --lib` remains green.
- [x] Corpus audit remains at or better than current parity.

Completed note:
- Final verification after deletion cleanup:
  - `cargo test --lib -q`
  - `rg -n "parse_parsing::|mod parse_parsing|parse_text_with_annotations_legacy_for_tooling|parse_card_ast_with_annotations\\(|audit_rewrite_parse_diff|inspect_rewrite_card|src/cards/builders/parser.rs|mod parser;" src Cargo.toml`
- Final cleanup state:
  - `cargo test --lib -q` is green
  - `src/cards/builders/parse_parsing` has been removed
  - `src/cards/builders/parser.rs` has been removed
  - the old audit binaries have been removed
  - remaining references in this checklist are historical notes only
