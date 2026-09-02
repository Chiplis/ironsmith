#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_02::*;
use super::*;

#[test]
pub(super) fn keyword_static_play_from_haste_followup_uses_clause_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_play_from_permission_with_haste_this_way_line",
        "pub(crate) fn parse_you_may_look_top_card_any_time_line",
    );

    assert!(
        parser.contains("late_static_facts::parse_play_permission_with_haste_followup(tokens)")
            && parser.contains("parse_permission_clause_spec(permission_sentence)?"),
        "{relative} should consume a typed permission sentence after the grammar validates the haste follow-up"
    );
    for forbidden in [
        "let haste_words = parser_token_word_refs(haste_sentence)",
        "\"if\", \"you\", \"cast\", \"a\", \"creature\", \"spell\", \"this\", \"way\", \"it\", \"gains\", \"haste\"",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse cast-this-way haste follow-up through raw word arrays: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_count_as_card_named_uses_token_clause_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn parse_count_as_card_named_for_spell_effect_line",
        "fn parse_static_ability_ast_line_early_lexed",
    );
    let early_parser = function_source(
        &content,
        "fn parse_static_ability_ast_line_early_lexed",
        "pub(crate) fn parse_static_ability_ast_line_lexed",
    );

    for required in [
        "fn parse_count_as_card_named_for_spell_effect_line(tokens: &[OwnedLexToken])",
        "let words = LexedClause::new(tokens).word_refs()",
        "early_static_facts::parse_count_as_card_named_shape_words(&words)",
        "words.get(shape.spell_name_words)?",
        "words.get(shape.counted_name_words)?",
        "parse_count_as_card_named_for_spell_effect_line(tokens)",
    ] {
        assert!(
            helper.contains(required) || early_parser.contains(required),
            "{relative} should parse count-as-card-named static lines through token clause shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_count_as_card_named_for_spell_effect_line(words: &[&str])",
        "COUNT_AS_CARD_NAMED_GRAVEYARD_PREFIX_PATTERN.matches_words(words)",
        "COUNT_IT_AS_A_CARD_NAMED_PATTERN.matches_words(tail)",
        "parse_count_as_card_named_for_spell_effect_line(&words)",
    ] {
        assert!(
            !helper.contains(forbidden) && !early_parser.contains(forbidden),
            "{relative} should not parse count-as-card-named static lines through raw word slices: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_ward_wrapper_uses_token_shapes() {
    let root = workspace_root();
    let grammar_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/keyword_static_lines/nearby_primitives.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    let marker = function_source(
        &grammar,
        "pub(crate) fn parse_ward_abilities_dont_trigger_marker_tokens",
        "pub(crate) fn parse_dont_untap_during_controllers_step_tokens",
    );
    for required in [
        "semantic_all(",
        "semantic_phrase(&[\"ward\", \"abilities\", \"of\", \"those\", \"creatures\"])",
        "alt((semantic_kw(\"dont\"), semantic_kw(\"don't\")))",
        "semantic_kw(\"trigger\")",
    ] {
        assert!(
            marker.contains(required),
            "{grammar_relative} should own the multiword ward-suppression marker with typed winnow grammar: missing `{required}`"
        );
    }

    let family_relative = "crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs";
    let family = read_repo_file(&root, family_relative);
    let consumer = function_source(
        &family,
        "pub(crate) fn parse_ward_static_ability_line",
        "pub(crate) fn parse_ward_discard_card_type_cost",
    );

    for required in [
        "tokens.first().is_some_and(|token| token.is_word(\"ward\"))",
        "keyword_static_lines::parse_ward_abilities_dont_trigger_marker_tokens(tokens)",
        "keyword_static_lines::parse_ward_cost_tokens(tokens)",
        "let cost_tokens = trim_commas(ward.cost_tokens)",
        "parse_payment_clause_as_total_cost(&cost_tokens)",
        "render_token_slice(tokens)",
    ] {
        assert!(
            consumer.contains(required),
            "{family_relative} should consume typed ward tokens and lower only their cost/result: missing `{required}`"
        );
    }
    for forbidden in [
        "token_word_refs",
        "parser_token_word_refs",
        "word_slice_",
        ".matches_words(",
        "raw_line",
        "normalized_text",
        "split_once",
        "str_contains",
        "str_starts_with",
        ".join(\" \")",
    ] {
        assert!(
            !consumer.contains(forbidden),
            "{family_relative} should not rediscover ward structure through raw text or detached word vectors: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_source_attack_control_gate_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "parse_source_did_not_attack_or_enter_control_this_turn_shape(predicate_tokens)",
        "fn is_source_did_not_attack_subject_clause(clause: LexedClause<'_>) -> bool",
        "strip_leading_article_tokens(clause.trimmed().tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route the source attack/control predicate gate through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "let source_state_tokens = crate::lexer::synthetic_word_tokens(&filtered)",
        "parse_source_did_not_attack_or_enter_control_this_turn_shape(&source_state_tokens)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild source state predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_spell_lifecycle_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_spell_lifecycle_predicate",
        "fn parse_you_cast_source_shape",
    );

    for required in [
        "fn parse_spell_lifecycle_predicate(tokens: &[OwnedLexToken])",
        "parse_you_cast_source_shape(tokens)",
        "parse_tagged_was_cast_shape(tokens)",
        "parse_this_spell_was_cast_from_shape(tokens)",
        "parse_no_spells_cast_last_turn_shape(tokens)",
        "parse_this_spell_paid_named_label_shape(tokens)",
        "parse_target_was_kicked_shape(tokens)",
        "parse_spell_lifecycle_predicate(predicate_tokens)",
        "strip_leading_article_tokens(clause.trimmed().tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route spell lifecycle predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_spell_lifecycle_predicate(words: &[&str])",
        "parse_spell_lifecycle_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild spell lifecycle predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild spell lifecycle predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_paid_cost_label_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_paid_cost_label_predicate",
        "fn paid_cost_tail_is_negated",
    );

    for required in [
        "fn parse_paid_cost_label_predicate(tokens: &[OwnedLexToken])",
        "let clause = LexedClause::new(tokens)",
        "parse_paid_cost_label_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route paid-cost label predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_paid_cost_label_predicate(words: &[&str])",
        "parse_paid_cost_label_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild paid-cost label predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild paid-cost label predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_attached_tagged_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_attached_tagged_predicate",
        "fn parse_this_permanent_attached_to_shape",
    );

    for required in [
        "fn parse_attached_tagged_predicate(tokens: &[OwnedLexToken])",
        "parse_this_permanent_attached_to_shape(tokens)",
        "parse_attached_tagged_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route attached-tagged predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_attached_tagged_predicate(words: &[&str])",
        "parse_attached_tagged_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild attached-tagged predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild attached-tagged predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_tagged_state_and_exile_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_tagged_exiled_predicate",
        "fn parse_tagged_controlled_permanent_shape",
    );

    for required in [
        "fn parse_tagged_exiled_predicate(tokens: &[OwnedLexToken])",
        "fn parse_tagged_state_predicate(tokens: &[OwnedLexToken])",
        "parse_tagged_state_predicate(predicate_tokens)",
        "parse_tagged_exiled_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route tagged state/exile predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_tagged_exiled_predicate(words: &[&str])",
        "fn parse_tagged_state_predicate(words: &[&str])",
        "parse_tagged_state_predicate(&filtered)",
        "parse_tagged_exiled_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild tagged state/exile predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild tagged state/exile predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_revealed_or_controlled_subtype_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_revealed_or_controlled_subtype_predicate",
        "fn is_card_graveyard_existential_clause",
    );

    for required in [
        "fn parse_revealed_or_controlled_subtype_predicate(\n    tokens: &[OwnedLexToken]",
        "let revealed_token = revealed_subtype.token(0)?",
        "let controlled_token = controlled_subtype.token(0)?",
        "revealed_token.parser_text() != controlled_token.parser_text()",
        "parse_subtype_word(revealed_token.parser_text())",
        "parse_revealed_or_controlled_subtype_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route revealed/control subtype predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_revealed_or_controlled_subtype_predicate(words: &[&str])",
        "parse_revealed_or_controlled_subtype_predicate(&filtered)",
        "let revealed_words = revealed_subtype.word_refs()",
        "let controlled_words = controlled_subtype.word_refs()",
        "parse_subtype_word(revealed_words.first().copied()?)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild revealed/control subtype predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild revealed/control subtype predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_vote_results_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_vote_result_predicate",
        "fn parse_spell_context_predicate",
    );

    for required in [
        "fn parse_vote_result_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_vote_option_result_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_no_vote_objects_matched_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_vote_result_predicate(predicate_tokens, true)",
        "parse_vote_result_predicate(predicate_tokens, false)",
        "option.tokens().is_empty()",
        "render_token_slice(option.tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route vote-result predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_vote_result_predicate(\n    words: &[&str]",
        "fn parse_vote_option_result_predicate(words: &[&str]",
        "fn parse_no_vote_objects_matched_predicate(\n    words: &[&str]",
        "parse_vote_result_predicate(&filtered, true)",
        "parse_vote_result_predicate(&filtered, false)",
        "option.word_refs().is_empty()",
        "option.word_refs().join(\" \")",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild vote-result predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild vote-result predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_x_value_comparison_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_x_value_comparison_predicate",
        "fn parse_paid_cost_label_predicate",
    );

    for required in [
        "fn parse_x_value_comparison_predicate(tokens: &[OwnedLexToken])",
        "let words = clause.word_refs()",
        "parse_x_value_comparison_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route X-value comparison predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_x_value_comparison_predicate(words: &[&str])",
        "parse_x_value_comparison_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild X-value comparison predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild X-value comparison predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_mana_spent_helpers_use_tokens() {
    let root = workspace_root();
    let helper_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/meld_and_special_subjects.rs";
    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let etb_relative = "crates/ironsmith-compiler-grammar/src/keyword_static/etb_static_lines.rs";
    let helper_content = read_repo_file(&root, helper_relative);
    let predicate_content = read_repo_file(&root, predicate_relative);
    let etb_content = read_repo_file(&root, etb_relative);
    let helper = function_source(
        &helper_content,
        "pub(super) fn parse_mana_spent_to_cast_predicate",
        "pub(super) fn parse_mana_symbol_word",
    );
    let predicate = function_source(
        &predicate_content,
        "fn parse_mana_spent_capture_predicate",
        "fn parse_mana_symbol_spent_to_cast_shape",
    );

    for required in [
        "pub(super) fn parse_mana_spent_to_cast_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_mana_symbol_word(token.parser_text())",
        "fn parse_same_color_mana_spent_to_cast_predicate(tokens: &[OwnedLexToken]",
        "fn parse_mana_spent_capture_predicate(tokens: &[OwnedLexToken])",
        "let validation_words = mana_spent_symbol_clause_words(symbol_clause)",
        "word_is_any(word, MANA_SYMBOL_WORDS)",
        "parse_mana_symbol(token.parser_text())",
        "parse_mana_spent_capture_predicate(predicate_tokens)",
        "parse_same_color_mana_spent_to_cast_predicate(tokens)",
        "parse_mana_spent_to_cast_predicate(tokens)",
        "parse_same_color_mana_spent_to_cast_predicate(&condition_tokens)",
    ] {
        assert!(
            helper_content.contains(required)
                || predicate_content.contains(required)
                || etb_content.contains(required),
            "mana-spent parsing should use token slices end-to-end: missing `{required}`"
        );
    }
    for forbidden in [
        "parse_mana_spent_capture_predicate(&filtered)",
        "fn parse_mana_spent_capture_predicate(words: &[&str])",
        "parse_same_color_mana_spent_to_cast_predicate(\n            &condition_words",
        "LexedClause::new(token).word_refs()",
        "let symbol_words = tokens",
        "let symbol_words = symbol_clause.word_refs()",
        "MANA_SYMBOL_WORD_PATTERN.matches_word(word)",
        "parse_mana_symbol(word).ok()",
    ] {
        assert!(
            !predicate_content.contains(forbidden) && !etb_content.contains(forbidden),
            "mana-spent parsing should not route through filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !helper.contains(forbidden) && !predicate.contains(forbidden),
            "{helper_relative} and {predicate_relative} should not rebuild mana-spent parser tokens from raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_card_in_your_graveyard_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_card_in_your_graveyard_predicate",
        "fn parse_object_on_battlefield_predicate",
    );
    let subtype_helper = function_source(
        &content,
        "fn parse_subtype_card_descriptor_clause",
        "fn parse_card_in_your_graveyard_predicate",
    );
    let named_helper = function_source(
        &content,
        "fn parse_named_object_filter_name_tail",
        "fn graveyard_card_types_subject",
    );

    // Formatting of a call is not part of this invariant: compare against
    // whitespace-collapsed sources so a rustfmt line break inside an argument
    // list cannot read as a missing route.
    let collapse = |source: &str| source.split_whitespace().collect::<Vec<_>>().join(" ");
    let collapsed_content = collapse(&content);
    let collapsed_named_helper = collapse(&named_helper);
    let collapsed_subtype_helper = collapse(&subtype_helper);
    for required in [
        "fn parse_card_in_your_graveyard_predicate( tokens: &[OwnedLexToken]",
        "let clause = LexedClause::new(tokens)",
        "parse_card_in_your_graveyard_predicate(predicate_tokens)",
        "descriptor.tokens().is_empty()",
        "object.tokens().is_empty()",
        "parse_object_filter( trimmed_tokens, false, )",
        "parse_subtype_card_descriptor_clause(descriptor)",
        "let descriptor_tokens = strip_leading_article_tokens(clause.trimmed().tokens())",
        "token_word_is_any(&descriptor_tokens[1], CARD_OR_CARDS_WORDS)",
    ] {
        let required = collapse(required);
        assert!(
            collapsed_content.contains(&required)
                || collapsed_named_helper.contains(&required)
                || collapsed_subtype_helper.contains(&required),
            "{relative} should route graveyard-card predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_card_in_your_graveyard_predicate(words: &[&str])",
        "parse_card_in_your_graveyard_predicate(&filtered)",
        "let tokens = crate::lexer::synthetic_word_tokens(words)",
        "let trimmed_tokens = crate::lexer::synthetic_word_tokens(trimmed)",
        "descriptor.word_refs().is_empty()",
        "object.word_refs().is_empty()",
        "let descriptor_words = descriptor.word_refs()",
        "descriptor_words.strip_prefix(&[\"an\"])",
        "descriptor_words.strip_prefix(&[\"a\"])",
    ] {
        assert!(
            !parser.contains(forbidden)
                && !named_helper.contains(forbidden)
                && !subtype_helper.contains(forbidden),
            "{relative} should not rebuild graveyard-card predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_half_starting_life_threshold_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_half_starting_life_total_threshold_predicate",
        "fn parse_life_total_subject_clause",
    );

    for required in [
        "fn parse_half_starting_life_total_threshold_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_half_starting_life_total_threshold_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route half-starting-life predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_half_starting_life_total_threshold_predicate(words: &[&str])",
        "parse_half_starting_life_total_threshold_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild half-starting-life predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild half-starting-life predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_life_total_static_thresholds_use_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_life_total_at_least_starting_predicate",
        "fn parse_counted_objects_have_counter_predicate",
    );

    for required in [
        "const LIFE_TOTAL_AT_LEAST_LAST_NOTED_PHRASES: &[&[&str]]",
        "fn parse_life_total_at_least_starting_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_life_total_at_least_last_noted_predicate(\n    tokens: &[OwnedLexToken]",
        "non_article_token_words_eq_phrase(tokens, LIFE_TOTAL_AT_LEAST_STARTING_PHRASE)",
        "non_article_token_words_eq_any(tokens, LIFE_TOTAL_AT_LEAST_LAST_NOTED_PHRASES)",
        "parse_life_total_at_least_starting_predicate(&cleaned_tokens[life_idx + 1..])",
        "parse_life_total_at_least_starting_predicate(predicate_tokens)",
        "parse_life_total_at_least_last_noted_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route life-total static thresholds through token shapes: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_life_total_at_least_starting_predicate(words: &[&str])",
        "fn parse_life_total_at_least_last_noted_predicate(words: &[&str])",
        "LIFE_TOTAL_AT_LEAST_STARTING_PATTERN.matches_non_article_tokens(tokens)",
        "LIFE_TOTAL_AT_LEAST_LAST_NOTED_PATTERN.matches_non_article_tokens(tokens)",
        "parse_life_total_at_least_starting_predicate(&words[life_idx + 1..])",
        "parse_life_total_at_least_starting_predicate(&filtered)",
        "parse_life_total_at_least_last_noted_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route life-total static thresholds through filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["matches!(\n        words"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not keep life-total static thresholds as raw word slice matches: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_you_life_total_at_most_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_you_life_total_at_most_predicate",
        "fn life_total_at_most_from_amount_tokens",
    );

    for required in [
        "fn parse_you_life_total_at_most_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_you_life_total_at_most_predicate(predicate_tokens)",
        "let clause = LexedClause::new(tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route life-total-at-most predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_you_life_total_at_most_predicate(\n    words: &[&str]",
        "parse_you_life_total_at_most_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild life-total-at-most predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild life-total-at-most predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_player_object_keyword_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_graveyard_escape_keyword_predicate",
        "fn parse_keyword_subject_object_in_zone_filter",
    );
    let helper = function_source(
        &content,
        "fn parse_keyword_subject_object_filter_tokens",
        "fn parse_graveyard_escape_keyword_predicate",
    );

    for required in [
        "fn parse_keyword_subject_object_filter_tokens(\n    object_tokens: &[OwnedLexToken]",
        "let object_tokens = strip_leading_article_tokens(object_tokens)",
        "non_article_token_words_eq_any(object_tokens, NONLAND_CARD_OBJECT_PHRASES)",
        "*last = OwnedLexToken::synthetic_word(\"card\")",
        "parse_keyword_subject_object_filter_tokens(object.tokens())",
        "fn parse_graveyard_escape_keyword_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_player_object_keyword_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_filter_keyword_constraint_tokens(keyword.tokens())",
        "consumed != keyword.tokens().len()",
        "token_word_is(token, CONTROL_WORD)",
        "token_word_is_any(token, ZONE_WORDS)",
        "parse_graveyard_escape_keyword_predicate(tokens)",
        "parse_player_object_keyword_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required) || helper.contains(required),
            "{relative} should route player-object keyword predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_graveyard_escape_keyword_predicate(\n    words: &[&str]",
        "fn parse_player_object_keyword_predicate(\n    words: &[&str]",
        "fn parse_keyword_subject_object_filter_words",
        "parse_player_object_keyword_predicate(&filtered)",
        "parse_keyword_subject_object_filter_words(object_words.as_slice())",
        "parse_keyword_subject_object_filter_words(&object_words)",
        "let keyword_words = keyword.word_refs()",
        "parse_filter_keyword_constraint_words(&keyword_words)",
        "consumed != keyword_words.len()",
        "let subject_words = subject.word_refs()",
        "CONTROL_WORD_PATTERN.matches_word(word)",
        "ZONE_WORD_PATTERN.matches_word(word)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route player-object keyword predicates through filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden) && !helper.contains(forbidden),
            "{relative} should not rebuild player-object keyword predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_source_state_identity_keyword_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_source_identity_predicate",
        "fn parse_source_crewed_by_exactly_predicate",
    );

    for required in [
        "fn parse_source_identity_predicate(tokens: &[OwnedLexToken])",
        "parse_identity_descriptor_filter_tokens(descriptor_clause.tokens())",
        "fn parse_identity_descriptor_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter>",
        "parse_card_type(token.parser_text())",
        "parse_subtype_flexible(token.parser_text())",
        "fn parse_source_keyword_predicate(tokens: &[OwnedLexToken])",
        "fn parse_filter_keyword_constraint_tokens(\n    tokens: &[OwnedLexToken],\n) -> Option<(FilterKeywordConstraint, usize)>",
        "let consumed_tokens = words.token_index_after_words(consumed_words)?",
        "parse_filter_keyword_constraint_tokens(keyword.tokens())",
        "consumed != keyword.tokens().len()",
        "fn parse_source_simple_state_predicate(tokens: &[OwnedLexToken])",
        "fn parse_source_power_threshold_predicate(tokens: &[OwnedLexToken])",
        "parse_source_simple_state_predicate(predicate_tokens)",
        "parse_source_identity_predicate(predicate_tokens)",
        "parse_source_keyword_predicate(predicate_tokens)",
        "parse_source_power_threshold_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route source state/identity/keyword predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_source_identity_predicate(words: &[&str])",
        "fn parse_source_keyword_predicate(words: &[&str])",
        "fn parse_source_simple_state_predicate(words: &[&str])",
        "fn parse_source_power_threshold_predicate(words: &[&str])",
        "parse_source_simple_state_predicate(&filtered)",
        "parse_source_identity_predicate(&filtered)",
        "parse_source_keyword_predicate(&filtered)",
        "parse_source_power_threshold_predicate(&filtered)",
        "let descriptor_words = descriptor_clause.word_refs()",
        "parse_identity_descriptor_filter_words(&descriptor_words)",
        "let keyword_words = keyword.word_refs()",
        "parse_filter_keyword_constraint_words(&keyword_words)",
        "consumed != keyword_words.len()",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild source state/identity/keyword predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild source state/identity/keyword predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_source_attachment_count_uses_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_source_attachment_count_predicate",
        "fn parse_attachment_count_filter_tokens",
    );

    for required in [
        "fn parse_source_attachment_count_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_source_attachment_count_predicate(predicate_tokens)",
        "render_token_slice(tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route source attachment-count predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_source_attachment_count_predicate(\n    words: &[&str]",
        "parse_source_attachment_count_predicate(&filtered)",
        "words.join(\" \")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild source attachment-count predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild source attachment-count predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_basic_land_and_combat_shapes_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_basic_land_types_among_lands_predicate",
        "fn parse_you_attacked_this_turn_shape",
    );

    for required in [
        "fn parse_basic_land_types_among_lands_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_combat_turn_predicate(tokens: &[OwnedLexToken])",
        "parse_basic_land_types_among_lands_predicate(predicate_tokens)",
        "parse_combat_turn_predicate(predicate_tokens)",
        "render_token_slice(tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route basic-land/combat predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_basic_land_types_among_lands_predicate(\n    words: &[&str]",
        "fn parse_combat_turn_predicate(words: &[&str])",
        "parse_basic_land_types_among_lands_predicate(&filtered)",
        "parse_combat_turn_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild basic-land/combat predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild basic-land/combat predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_simple_capture_wrappers_use_predicate_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_turn_timing_predicate",
        "fn parse_vote_result_predicate",
    );

    for required in [
        "fn parse_turn_timing_predicate(tokens: &[OwnedLexToken])",
        "fn parse_opponent_controls_tagged_object_predicate(\n    tokens: &[OwnedLexToken]",
        "fn parse_secret_choices_match_predicate(\n    tokens: &[OwnedLexToken]",
        "parse_turn_timing_predicate(predicate_tokens)",
        "parse_opponent_controls_tagged_object_predicate(predicate_tokens)",
        "parse_secret_choices_match_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route simple capture-wrapper predicates through captured predicate tokens: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_turn_timing_predicate(words: &[&str])",
        "fn parse_opponent_controls_tagged_object_predicate(words: &[&str])",
        "fn parse_secret_choices_match_predicate(words: &[&str])",
        "parse_turn_timing_predicate(&filtered)",
        "parse_opponent_controls_tagged_object_predicate(&filtered)",
        "parse_secret_choices_match_predicate(&filtered)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild simple capture-wrapper predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
    for forbidden in ["let tokens = crate::lexer::synthetic_word_tokens(words)"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild simple capture-wrapper predicate tokens from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn predicate_or_parser_uses_token_slices_for_split_and_prefix_fallback() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "fn parse_or_predicate",
        "fn parse_attacking_you_own_control_predicate",
    );

    for required in [
        "fn parse_or_predicate(tokens: &[OwnedLexToken])",
        "token_word_is(token, OR_WORD)",
        "token_word_is_any(token, OR_COMPARISON_TAIL_WORDS)",
        "let left_tokens = &tokens[..or_idx]",
        "let right_tokens = &tokens[or_idx + 1..]",
        "parse_predicate(left_tokens)",
        "parse_predicate(right_tokens)",
        "predicate_reference_prefix_tokens(left_tokens)",
        "predicate_tokens_start_with_reference(right_tokens)",
        "token_word_is_any(token, PREDICATE_REFERENCE_START_WORDS)",
        "prefixed_tokens.extend_from_slice(right_tokens)",
        "parse_or_predicate(predicate_tokens)",
    ] {
        assert!(
            content.contains(required) || parser.contains(required),
            "{relative} should split OR predicates with token slices and preserve reference-prefix fallback: missing `{required}`"
        );
    }
    for forbidden in [
        "fn parse_or_predicate(filtered: &[&str])",
        "parse_or_predicate(&filtered)",
        "predicate_reference_prefix(left_words)",
        "predicate_words_start_with_reference(right_words)",
        "let first_word = LexedClause::new(tokens).word_refs().first().copied()",
        "predicate_tokens_from_words(left_words)",
        "predicate_tokens_from_words(right_words)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rebuild OR predicate branches from filtered raw words: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn combat_restriction_control_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/grammar/activation_costs/cant_shapes/attack_unless.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn parse_controller_control_requirement_inner",
        "fn parse_minimum_count_lexed",
    );

    assert!(
        helper.contains("conditions::parse_control_condition")
            && helper.contains("conditions::ControlConditionOptions")
            && helper.contains("parsed.has_explicit_quantity()")
            && helper.contains(".at_least_count()")
            && helper.contains("PredicateAst::PlayerHasAtLeast"),
        "{relative} should parse combat-restriction control tails into a typed condition through the shared capture parser"
    );
    for forbidden in [
        "parse_greater_than_or_equal_count_prefix_from_words(tail.get(2..)",
        "let filter_words = tail.get(2 + used..)",
        "parse_object_filter(&filter_tokens, false)",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not rebuild control-condition count/filter tails by hand with `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn static_control_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/keyword_static/anthem_grant_lines.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_static_condition_clause",
        "fn parse_devotion_static_condition",
    );

    assert!(
        parser.contains("grammar::conditions::parse_control_condition")
            && parser.contains("allow_opponent_players: true")
            && parser.contains("bind_filter_controller_to_subject: true")
            && parser.contains("comparison: control_condition.comparison"),
        "{relative} should parse static control conditions through the shared captured control-condition parser"
    );
    for forbidden in [
        "ANTHEM_CONTROL_CONDITION_TWO_WORD_PREFIX_PATTERN",
        "ANTHEM_CONTROL_CONDITION_THREE_WORD_PREFIX_PATTERN",
        "parse_counted_object_condition_after_prefix(\n                &count_condition_tokens[..control_prefix_token_len]",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep static control-condition prefix slicing through `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn static_ownership_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/keyword_static/anthem_grant_lines.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_static_condition_clause",
        "fn parse_devotion_static_condition",
    );

    assert!(
        parser.contains("grammar::conditions::parse_ownership_condition")
            && parser.contains("OwnershipConditionOptions")
            && parser.contains("bind_filter_owner_to_subject: true")
            && parser.contains("comparison: ownership_condition.comparison"),
        "{relative} should parse static ownership conditions through the shared captured ownership-condition parser"
    );
    for forbidden in [
        "ANTHEM_OWN_CONDITION_TWO_WORD_PREFIX_PATTERN",
        "ANTHEM_OWN_CONDITION_THREE_WORD_PREFIX_PATTERN",
        "parse_counted_object_condition_after_prefix(\n                &count_condition_tokens[..own_prefix_token_len]",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not keep static ownership-condition prefix slicing through `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn subject_status_conditions_use_shared_capture_parser() {
    let root = workspace_root();
    let static_relative =
        "crates/ironsmith-compiler-grammar/src/keyword_static/anthem_grant_lines.rs";
    let static_content = read_repo_file(&root, static_relative);
    let static_parser = function_source(
        &static_content,
        "pub(crate) fn parse_static_condition_clause",
        "fn parse_devotion_static_condition",
    );
    assert!(
        static_parser.contains("grammar::conditions::parse_subject_status_condition")
            && static_parser.contains(".and_then(|condition| condition.condition_expr())"),
        "{static_relative} should parse source/equipped-creature status clauses through the shared captured subject-status parser"
    );
    for forbidden in [
        "SOURCE_IS_EQUIPPED_CONDITION_PATTERN",
        "SOURCE_IS_ENCHANTED_CONDITION_PATTERN",
        "SOURCE_IS_UNTAPPED_CONDITION_PATTERN",
        "SOURCE_IS_TAPPED_CONDITION_PATTERN",
        "SOURCE_IS_MONSTROUS_CONDITION_PATTERN",
        "SOURCE_IS_ATTACKING_CONDITION_PATTERN",
        "EQUIPPED_CREATURE_IS_TAPPED_CONDITION_PATTERN",
        "EQUIPPED_CREATURE_IS_UNTAPPED_CONDITION_PATTERN",
        "EQUIPPED_CREATURE_IS_ATTACKING_CONDITION_PATTERN",
    ] {
        assert!(
            !static_content.contains(forbidden),
            "{static_relative} should not keep exact subject-status ClauseShape `{forbidden}`"
        );
    }

    let grammar_relative = "crates/ironsmith-compiler-grammar/src/grammar/abilities.rs";
    let grammar_content = read_repo_file(&root, grammar_relative);
    let tap_status_parser = function_source(
        &grammar_content,
        "pub(crate) fn parse_source_tap_status_condition_lexed",
        "pub(crate) fn is_enchanted_land_is_chosen_type_line_lexed",
    );
    assert!(
        tap_status_parser.contains("super::conditions::parse_subject_status_condition")
            && !tap_status_parser.contains("&[\"this\", \"creature\", \"is\", \"tapped\"]"),
        "{grammar_relative} should reuse the captured subject-status parser instead of exact tap-status phrase arrays"
    );
}

#[test]
pub(super) fn world_state_timing_predicates_use_token_shapes() {
    let root = workspace_root();
    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_world_state_or_timing_predicate",
        "fn parse_empty_battlefield_predicate",
    );
    assert!(
        predicate_parser.contains("parse_world_state_or_timing_predicate(predicate_tokens)")
            && predicate_helper.contains(
                "fn parse_world_state_or_timing_predicate(\n    tokens: &[OwnedLexToken]"
            )
            && predicate_helper.contains("parse_initiative_choice_predicate_shape(tokens)")
            && predicate_helper.contains("parse_night_state_predicate_shape(tokens)")
            && predicate_helper.contains("parse_first_combat_phase_predicate_shape(tokens)")
            && predicate_helper.contains("parse_cast_this_spell_during_main_phase_shape(tokens)")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_world_state_or_timing_predicate(&filtered)"),
        "{predicate_relative} should route world-state/timing predicates through lexed token shape parsers"
    );
}

#[test]
pub(super) fn empty_battlefield_predicates_use_token_shapes() {
    let root = workspace_root();
    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_empty_battlefield_predicate",
        "fn is_battlefield_zone_clause",
    );
    assert!(
        predicate_parser.contains("parse_empty_battlefield_predicate(predicate_tokens)")
            && predicate_helper
                .contains("fn parse_empty_battlefield_predicate(tokens: &[OwnedLexToken])")
            && predicate_helper.contains("let clause = LexedClause::new(tokens)")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_empty_battlefield_predicate(&filtered)"),
        "{predicate_relative} should route empty-battlefield predicates through lexed token shape parsers"
    );
}

#[test]
pub(super) fn player_turn_event_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_turn_event_predicate",
        "fn parse_turn_timing_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_turn_event_predicate(predicate_tokens)")
            && predicate_content.contains("grammar::conditions::parse_player_turn_event_condition")
            && predicate_helper
                .contains("fn parse_player_turn_event_predicate(tokens: &[OwnedLexToken])")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_player_turn_event_predicate(&filtered)"),
        "{predicate_relative} should route turn-event count predicates through the shared captured parser"
    );
    for forbidden in [
        "DREW_WORD_PATTERN",
        "DRAWN_WORD_PATTERN",
        "LAND_OR_LANDS_WORD_PATTERN",
        "ENTER_OR_ENTERED_WORD_PATTERN",
        "BATTLEFIELD_WORD_PATTERN",
        "CONTROL_POSSESSIVE_WORD_PATTERN",
        "Value::MaxCardsDrawnThisTurn(player_filter)",
        "Value::LandsEnteredBattlefieldThisTurn(player_filter)",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual turn-event predicate parsing through `{forbidden}`"
        );
    }

    let conditions_relative = "crates/ironsmith-compiler-grammar/src/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub fn parse_player_turn_event_condition")
            && conditions_content.contains("PlayerTurnEventConditionAst")
            && conditions_content.contains("PlayerTurnEventAst"),
        "{conditions_relative} should expose a captured turn-event condition AST parser"
    );
}

#[test]
pub(super) fn spell_context_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_spell_context_predicate",
        "fn parse_player_spell_cast_this_turn_predicate",
    );
    assert!(
        predicate_parser.contains("parse_spell_context_predicate(predicate_tokens)")
            && predicate_content.contains("grammar::conditions::parse_spell_context_condition")
            && predicate_helper
                .contains("fn parse_spell_context_predicate(tokens: &[OwnedLexToken])")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_spell_context_predicate(&filtered)"),
        "{predicate_relative} should route target-spell context predicates through the shared captured parser"
    );
    for forbidden in [
        "TARGET_SPELL_CONTROLLER_POISONED_PATTERN",
        "TARGET_SPELL_NO_MANA_SPENT_TO_CAST_PATTERN",
        "YOU_CONTROL_MORE_CREATURES_THAN_TARGET_SPELL_CONTROLLER_PATTERN",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep exact target-spell context predicate parsing through `{forbidden}`"
        );
    }

    let conditions_relative = "crates/ironsmith-compiler-grammar/src/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub fn parse_spell_context_condition")
            && conditions_content.contains("SpellContextConditionAst")
            && conditions_content.contains("SpellContextReferenceAst"),
        "{conditions_relative} should expose a captured target-spell context condition AST parser"
    );
}

#[test]
pub(super) fn player_spell_cast_this_turn_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_spell_cast_this_turn_predicate",
        "fn parse_player_life_change_this_turn_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_spell_cast_this_turn_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_player_spell_cast_this_turn_condition")
            && predicate_helper.contains(
                "fn parse_player_spell_cast_this_turn_predicate(\n    tokens: &[OwnedLexToken]"
            )
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_player_spell_cast_this_turn_predicate(&filtered)"),
        "{predicate_relative} should route player spell-cast-this-turn predicates through the shared captured parser"
    );
    for forbidden in [
        "YOU_CAST_ANOTHER_SPELL_THIS_TURN_PATTERN",
        "OPPONENT_HAS_CAST_PREFIX_PATTERN",
        "OPPONENTS_HAVE_CAST_PREFIX_PATTERN",
        "YOUVE_CAST_PREFIX_PATTERN",
        "YOU_HAVE_CAST_PREFIX_PATTERN",
        "YOU_CAST_PREFIX_PATTERN",
        "THAT_PLAYER_DIDNT_CAST_PREFIX_PATTERN",
        "THAT_PLAYER_DID_NOT_CAST_PREFIX_PATTERN",
        "YOU_DIDNT_CAST_PREFIX_PATTERN",
        "YOU_DID_NOT_CAST_PREFIX_PATTERN",
        "spell_cast_matching_predicate(",
        "parse_both_spell_cast_predicate(",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual spell-cast-this-turn parsing through `{forbidden}`"
        );
    }

    let conditions_relative = "crates/ironsmith-compiler-grammar/src/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub fn parse_player_spell_cast_this_turn_condition")
            && conditions_content.contains("PlayerSpellCastThisTurnConditionAst")
            && conditions_content.contains("MatchingFilters")
            && conditions_content.contains("CountAtLeast"),
        "{conditions_relative} should expose a captured spell-cast-this-turn condition AST parser"
    );
}

#[test]
pub(super) fn player_life_change_this_turn_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_life_change_this_turn_predicate",
        "fn parse_object_death_this_turn_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_life_change_this_turn_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_player_life_change_this_turn_condition")
            && predicate_helper.contains(
                "fn parse_player_life_change_this_turn_predicate(\n    tokens: &[OwnedLexToken]"
            )
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser
                .contains("parse_player_life_change_this_turn_predicate(&filtered)"),
        "{predicate_relative} should route player life-change-this-turn predicates through the shared captured parser"
    );
    for forbidden in [
        "OPPONENT_LOST_LIFE_THIS_TURN_PATTERN",
        "YOU_GAINED_PREFIX_PATTERN",
        "YOU_LOST_PREFIX_PATTERN",
        "LIFE_THIS_TURN_TAIL_PATTERN",
        "YOU_GAINED_LIFE_THIS_TURN_PATTERN",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual life-change-this-turn parsing through `{forbidden}`"
        );
    }

    let conditions_relative = "crates/ironsmith-compiler-grammar/src/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub fn parse_player_life_change_this_turn_condition")
            && conditions_content.contains("PlayerLifeChangeThisTurnConditionAst")
            && conditions_content.contains("PlayerLifeChangeDirectionAst"),
        "{conditions_relative} should expose a captured life-change-this-turn condition AST parser"
    );
}

#[test]
pub(super) fn this_spell_cost_conditions_use_clause_shapes_and_life_change_capture_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_this_spell_cost_condition",
        "fn parse_conjoined_this_spell_cost_condition",
    );

    for required in [
        "parse_player_life_change_this_turn_condition(tokens)",
        "this_spell_cost_condition_from_life_change_this_turn",
        "static_mid_facts::parse_known_spell_cost_condition(tokens)",
        "Fact::LifeTotalLessThanStarting",
        "Fact::AttackedThisTurn",
        "Fact::Target(target)",
        "Fact::OpponentControlsLandsOrMore(count)",
        "Fact::AssassinOrCommanderDealtCombatDamage",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should parse this-spell cost condition gates through token clauses and shared captures: missing `{required}`"
        );
    }
    for forbidden in [
        "IT_TARGETS_PREFIX_PATTERN.matches_words(&w)",
        "THIS_SPELL_TARGETS_PREFIX_PATTERN.matches_words(&w)",
        "YOU_GAINED_LIFE_THIS_TURN_PATTERN.matches_words(&w)",
        "YOU_GAINED_PREFIX_PATTERN.matches_words(&w)",
        "LIFE_THIS_TURN_SUFFIX_PATTERN.matches_words(&w)",
        "LIFE_THIS_TURN_SUFFIX_PATTERN.matches_words(&w[rest_start..])",
        "OPPONENT_CONTROLS_PREFIX_PATTERN.matches_words(&w)",
        "ASSASSIN_OR_COMMANDER_COMBAT_DAMAGE_PATTERN.matches_words(&w)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not parse this-spell cost condition gates through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn player_would_action_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_would_action_predicate",
        "fn parse_battlefield_entry_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_would_action_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_player_would_action_condition")
            && predicate_helper
                .contains("fn parse_player_would_action_predicate(\n    tokens: &[OwnedLexToken]")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_player_would_action_predicate(&filtered)"),
        "{predicate_relative} should route player-would-action predicates through the shared captured parser"
    );
    for forbidden in [
        "PLAYER_WOULD_DRAW_CARD_PATTERN",
        "PLAYER_WOULD_PROLIFERATE_PATTERN",
        "OPPONENT_WOULD_BEGIN_EXTRA_TURN_PATTERN",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual player-would-action parsing through `{forbidden}`"
        );
    }

    let conditions_relative = "crates/ironsmith-compiler-grammar/src/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub fn parse_player_would_action_condition")
            && conditions_content.contains("PlayerWouldActionConditionAst")
            && conditions_content.contains("PlayerWouldActionAst"),
        "{conditions_relative} should expose a captured player-would-action condition AST parser"
    );
}

#[test]
pub(super) fn battlefield_change_this_turn_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_battlefield_change_this_turn_predicate",
        "fn parse_combat_damage_this_turn_predicate",
    );
    assert!(
        predicate_parser.contains("parse_battlefield_change_this_turn_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_battlefield_change_this_turn_condition")
            && predicate_helper.contains(
                "fn parse_battlefield_change_this_turn_predicate(\n    tokens: &[OwnedLexToken]"
            )
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser
                .contains("parse_battlefield_change_this_turn_predicate(&filtered)"),
        "{predicate_relative} should route battlefield-change-this-turn predicates through the shared captured parser"
    );
    for forbidden in [
        "NO_PERMANENT_LEFT_BATTLEFIELD_THIS_TURN_PATTERN",
        "PERMANENT_LEFT_BATTLEFIELD_THIS_TURN_PATTERN",
        "LAND_YOU_CONTROLLED_PUT_INTO_GRAVEYARD_THIS_TURN_PATTERN",
        "PERMANENT_LEFT_UNDER_YOUR_CONTROL_THIS_TURN_PATTERN",
        "NONLAND_PERMANENT_LEFT_OR_SPELL_WARPED_PATTERN",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual battlefield-change parsing through `{forbidden}`"
        );
    }

    let conditions_relative = "crates/ironsmith-compiler-grammar/src/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub fn parse_battlefield_change_this_turn_condition")
            && conditions_content.contains("BattlefieldChangeThisTurnConditionAst"),
        "{conditions_relative} should expose a captured battlefield-change-this-turn condition AST parser"
    );
}

#[test]
pub(super) fn object_death_this_turn_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_object_death_this_turn_predicate",
        "fn parse_player_would_action_predicate",
    );
    assert!(
        predicate_parser.contains("parse_object_death_this_turn_predicate(predicate_tokens)")
            && predicate_content
                .contains("grammar::conditions::parse_object_death_this_turn_condition")
            && predicate_helper.contains(
                "fn parse_object_death_this_turn_predicate(\n    tokens: &[OwnedLexToken]"
            )
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_object_death_this_turn_predicate(&filtered)"),
        "{predicate_relative} should route object-death-this-turn predicates through the shared captured parser"
    );
    for forbidden in [
        "CREATURE_DIED_COUNT_TAIL_PATTERN",
        "CREATURE_CARD_PUT_INTO_YOUR_GRAVEYARD_THIS_TURN_PATTERN",
    ] {
        assert!(
            !predicate_parser.contains(forbidden),
            "{predicate_relative} should not keep manual object-death parsing through `{forbidden}`"
        );
    }

    let conditions_relative = "crates/ironsmith-compiler-grammar/src/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub fn parse_object_death_this_turn_condition")
            && conditions_content.contains("ObjectDeathThisTurnConditionAst")
            && conditions_content.contains("ObjectDeathThisTurnEventAst"),
        "{conditions_relative} should expose a captured object-death-this-turn condition AST parser"
    );
}

#[test]
pub(super) fn combat_damage_this_turn_predicates_use_token_shapes() {
    let root = workspace_root();

    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_combat_damage_this_turn_predicate",
        "fn is_player_object_clause",
    );
    assert!(
        predicate_parser.contains("parse_combat_damage_this_turn_predicate(predicate_tokens)")
            && predicate_helper.contains(
                "fn parse_combat_damage_this_turn_predicate(\n    tokens: &[OwnedLexToken]"
            )
            && predicate_helper
                .contains("parse_source_dealt_combat_damage_this_turn_shape(tokens)")
            && predicate_helper
                .contains("parse_player_dealt_combat_damage_by_subtype_this_turn_shape(tokens)")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_combat_damage_this_turn_predicate(&filtered)"),
        "{predicate_relative} should route combat-damage-this-turn predicates through lexed token shape parsers"
    );
}

#[test]
pub(super) fn player_life_total_conditions_use_shared_capture_parser() {
    let root = workspace_root();

    let predicate_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs";
    let predicate_content = read_repo_file(&root, predicate_relative);
    let predicate_parser = function_source(
        &predicate_content,
        "pub(crate) fn parse_predicate",
        "#[cfg(test)]",
    );
    let predicate_helper = function_source(
        &predicate_content,
        "fn parse_player_life_total_predicate",
        "fn parse_player_life_relation_predicate",
    );
    assert!(
        predicate_parser.contains("parse_player_life_total_predicate(predicate_tokens)")
            && predicate_content.contains("grammar::conditions::parse_player_life_total_condition")
            && predicate_helper
                .contains("fn parse_player_life_total_predicate(tokens: &[OwnedLexToken])")
            && !predicate_helper.contains("synthetic_word_tokens")
            && !predicate_parser.contains("parse_player_life_total_predicate(&filtered)"),
        "{predicate_relative} should route player life-total numeric predicates through the shared captured parser"
    );
    assert!(
        !predicate_content.contains("LIFE_TAIL_PATTERN"),
        "{predicate_relative} should not keep an exact life-tail predicate ClauseShape for numeric life totals"
    );

    let anthem_relative =
        "crates/ironsmith-compiler-grammar/src/keyword_static/anthem_grant_lines.rs";
    let anthem_content = read_repo_file(&root, anthem_relative);
    let life_total_condition_parser = function_source(
        &anthem_content,
        "fn parse_life_total_static_condition",
        "pub(crate) fn parse_anthem_for_each_expression",
    );
    assert!(
        life_total_condition_parser
            .contains("grammar::conditions::parse_player_life_total_condition")
            && life_total_condition_parser.contains("condition_expr()"),
        "{anthem_relative} should parse static life-total numeric conditions through the shared captured parser"
    );
    assert!(
        !anthem_content.contains("ANTHEM_LIFE_TAIL_PATTERN"),
        "{anthem_relative} should not keep an exact life-tail static ClauseShape"
    );

    let conditions_relative = "crates/ironsmith-compiler-grammar/src/grammar/conditions.rs";
    let conditions_content = read_repo_file(&root, conditions_relative);
    assert!(
        conditions_content.contains("pub fn parse_player_life_total_condition")
            && conditions_content.contains("PlayerLifeTotalConditionAst")
            && conditions_content.contains("Value::LifeTotal"),
        "{conditions_relative} should expose a captured life-total condition AST parser"
    );
}

#[test]
pub(super) fn jump_start_parser_uses_tokens_not_raw_oracle_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/util.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_jump_start_line",
        "pub(crate) fn parse_jump_start_line_lexed",
    );
    let actual = non_test_raw_text_check_literals(parser)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "jump-start recognition should use token phrase helpers, not rendered oracle-text searches"
    );
}

#[test]
pub(super) fn keyword_static_marker_support_uses_token_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let marker_support = function_source(
        &content,
        "fn supported_keyword_marker_tokens",
        "fn trim_outer_quotes",
    );

    for forbidden in [
        "TOUGHNESS_CREWS_VEHICLES_MARKER_TEXTS",
        "POWER_GREATER_MARKER_PREFIXES",
        "POWER_GREATER_MARKER_SUFFIX",
        "LOYALTY_COUNTER_CREW_COST_PREFIX",
        "LOYALTY_COUNTER_CREW_COST_SUFFIX",
        ".starts_with(prefix)",
        ".ends_with(POWER_GREATER_MARKER_SUFFIX)",
        ".starts_with(LOYALTY_COUNTER_CREW_COST_PREFIX)",
        ".ends_with(LOYALTY_COUNTER_CREW_COST_SUFFIX)",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should classify supported keyword-static crew markers through token shapes, not raw fragment `{forbidden}`"
        );
    }

    for expected in [
        "is_core_keyword_marker_text(&text)",
        "early_static_facts::parse_early_keyword_marker_tokens(tokens).is_some()",
    ] {
        assert!(
            marker_support.contains(expected),
            "{relative} should keep supported keyword-static marker routing on ClauseShape `{expected}`"
        );
    }
    for forbidden in [
        "let words = parser_token_word_refs(tokens)",
        "TOUGHNESS_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)",
        "POWER_GREATER_CREWS_VEHICLES_MARKER_PATTERN.matches_words(&words)",
        "LOYALTY_COUNTER_INSTEAD_OF_CREW_COST_MARKER_PATTERN.matches_words(&words)",
    ] {
        assert!(
            !marker_support.contains(forbidden),
            "{relative} should not route supported keyword-static markers through raw word vectors: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_damage_doubling_marker_uses_lexed_clause_shape() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_damage_doubling_mana_value_marker_line",
        "pub(crate) fn parse_static_ability_ast_line_lexed",
    );

    for required in [
        "early_static_facts::parse_damage_doubling_mana_value_marker_tokens(tokens)",
        "keyword_static_marker(tokens)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should route damage-doubling marker through LexedClause shape matching: missing `{required}`"
        );
    }
    for forbidden in [
        "let clause_words = crate::token_word_refs(tokens)",
        "DAMAGE_DOUBLING_MANA_VALUE_MARKER_PATTERN.matches_words(&clause_words)",
        "DAMAGE_DOUBLING_TO_TARGET_PATTERN.matches_words(&clause_words)",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild raw words for damage-doubling marker: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_static_pt_modifier_parsers_use_char_helpers() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs";
    let content = read_repo_file(&root, relative);
    let parser = function_source(
        &content,
        "pub(crate) fn parse_pt_modifier",
        "pub(crate) fn parse_no_maximum_hand_size_line",
    );

    for forbidden in [
        "raw.split('/')",
        "trim_start_matches('+')",
        "str_strip_prefix(trimmed, \"+\")",
        "str_strip_prefix(trimmed, \"-\")",
    ] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should parse keyword-static P/T modifiers through char helpers, not raw fragment `{forbidden}`"
        );
    }

    for expected in [
        "split_pt_modifier_components(raw)",
        "strip_leading_plus_char(power_raw)",
        "split_signed_pt_component(trimmed)",
    ] {
        assert!(
            parser.contains(expected),
            "{relative} should keep keyword-static P/T modifier parsing on helper `{expected}`"
        );
    }
}

#[test]
pub(super) fn anthem_attached_object_grants_use_subject_tags_not_rendered_text() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler-grammar/src/keyword_static/anthem_grant_conditionals.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn grant_object_ability_for_anthem_subject",
        "fn parse_granted_object_ability_segment",
    );

    assert!(
        helper.contains("attached_object_anthem_subject_filter(&clause.subject)")
            && helper.contains("TaggedOpbjectRelation::IsTaggedObject"),
        "{relative} should classify attached object ability grants from typed subject tags"
    );
    for forbidden in [
        "subject\n            .split_whitespace()",
        ".next()\n            .is_some_and(|word|",
        "ANTHEM_ENCHANTED_OR_EQUIPPED_WORD_PATTERN.matches_word(word)",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not classify attached object grants by inspecting rendered subject text `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn anthem_landwalk_override_uses_keyword_action_parser() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/keyword_static/anthem_grant_lines.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn is_landwalk_ability_word",
        "pub(crate) fn parse_subject_cant_be_blocked_as_long_as_condition_line",
    );

    assert!(
        helper.contains("parse_single_word_keyword_action(word)")
            && helper.contains("KeywordAction::Landwalk"),
        "{relative} should classify landwalk override tails through the keyword action parser"
    );
    for forbidden in ["LANDWALK_ABILITY_SUFFIX", ".ends_with("] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not classify landwalk override tails by raw suffix `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn cst_lowering_loyalty_detection_uses_cst_flag() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/semantic_assembly.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn activation_cost_cst_is_loyalty",
        "pub fn assemble_non_metadata_line",
    );

    for forbidden in [
        "raw_activation_cost_is_loyalty_shorthand",
        "cost.raw.as_str()",
        "raw.trim()",
        ".replace('−', \"-\")",
        ".starts_with('+')",
        ".starts_with('-')",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should classify loyalty shorthand costs through sign chars, not raw fragment `{forbidden}`"
        );
    }

    assert!(
        helper.contains("cost.is_loyalty_shorthand"),
        "{relative} should consume the token-parser loyalty shorthand flag instead of reclassifying raw cost text"
    );
}

#[test]
pub(super) fn sacrifice_filter_article_normalization_is_typed_grammar_owned() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler-grammar/src/grammar/activation_costs/object_segments.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(crate) fn parse_sacrifice_segment_tokens",
        "pub(crate) fn parse_discard_segment_tokens",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "single-choice sacrifice filter normalization should strip articles through tokens, not raw text prefixes"
    );
    assert!(
        helper.contains("parse_sacrifice_cost_shape_lexed")
            && helper.contains("SacrificeCostShape::Chosen")
            && helper.contains("parse_object_filter_with_grammar_entrypoint_lexed"),
        "{relative} should lower a typed winnow sacrifice shape directly into the cost CST"
    );
}

#[test]
pub(super) fn self_reference_name_word_shape_uses_word_counts_not_raw_spaces() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn is_single_word_keyword_verb",
        "fn preceded_by_named_keyword",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "self-reference name/keyword word-shape checks should use word counts, not raw space searches"
    );
}

#[test]
pub(super) fn vote_count_followup_preprocess_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/preprocess.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn rewrite_vote_count_followups_line",
        "fn resized_char_map_for_rewrite",
    );
    let actual = non_test_raw_text_check_literals(helper)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "vote-count followup preprocessing should use token words and spans, not raw oracle-text searches"
    );
}

#[test]
pub(super) fn future_zone_replacement_recognizer_uses_tokens_not_raw_text() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_entry.rs";
    let content = read_repo_file(&root, relative);
    let recognizer = function_source(
        &content,
        "fn future_zone_replacement_from_sentence_tokens",
        "fn maybe_rewrite_future_zone_replacement_sentence",
    );
    let actual = non_test_raw_text_check_literals(recognizer)
        .into_iter()
        .map(|literal| format!("{relative} -> {literal}"))
        .collect::<BTreeSet<_>>();

    let expected = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "future zone replacement recognition should use token phrase helpers, not raw oracle-text searches"
    );
}

#[test]
pub(super) fn where_x_effect_sentence_uses_token_rendered_clause_surface() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_inner/sentence_shape_predicates.rs";
    let content = read_repo_file(&root, relative);
    let start_marker = "fn parse_effect_sentence_with_where_x_lexed";
    let start = content
        .find(start_marker)
        .unwrap_or_else(|| panic!("missing function start marker: {start_marker}"));
    let parser = &content[start..];

    for required in [
        "let clause_display = render_token_slice(tokens).trim().to_string()",
        "replace_unbound_x_in_effects_anywhere(&mut effects, &where_value, &clause_display)",
    ] {
        assert!(
            parser.contains(required),
            "{relative} should preserve where-X clause surfaces from parse tokens: missing `{required}`"
        );
    }
    for forbidden in ["clause_words.join(\" \")"] {
        assert!(
            !parser.contains(forbidden),
            "{relative} should not rebuild where-X clause surfaces by joining word refs `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn sentence_shape_predicates_route_direct_sentence_gates_through_typed_grammar() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_inner/sentence_shape_predicates.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "sentence_shapes::parses_cant_gain_life_replacement_tokens(tokens)",
        "sentence_shapes::parse_delayed_sentence_tokens(tokens)",
        "sentence_shapes::parse_quoted_ability_sentence_tokens(tokens)",
        "sentence_shapes::parse_immediate_sacrifice_sentence_tokens(tokens)",
        "sentence_shapes::parse_leading_if_sentence_tokens(tokens)",
        "fn parse_it_is_aura_enchantment_sentence_lexed(",
        "sentence_shapes::parse_aura_enchantment_tokens(tokens)",
        "parse_it_is_aura_enchantment_sentence_lexed(tokens)",
        "sentence_shapes::DelayedSentenceShape::EndOfCombat",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route direct sentence-shape gates through typed grammar: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "fn sentence_find_phrase_start_lexed(",
        "fn sentence_shape_matches_words(",
        "_PATTERN",
        "SENTENCE_SACRIFICE_COUNTED_PREFIXES",
        "SENTENCE_DELAYED_LIFECYCLE_PHRASES",
        "SENTENCE_AURA_ENCHANT_CREATURE_PREFIX",
        "contains_token_kind(tokens, TokenKind::Quote)",
        "word_slice_find_phrase_start(&sentence_words",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route direct sentence-shape gates through raw word refs: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn replacement_and_prevention_routes_shape_recognition_through_typed_grammar() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_inner/replacement_and_prevention_shapes.rs";
    let content = read_repo_file(&root, relative);
    let parser_body = function_source(
        &content,
        "pub(crate) fn parse_monstrosity_sentence",
        "#[cfg(test)]",
    );

    for required in [
        "replacement_grammar::parse_monstrosity_shape(tokens)",
        "replacement_grammar::parse_counter_removed_pump_shape(tokens)",
        "replacement_grammar::parse_token_end_combat_action_shape(tokens)",
        "replacement_grammar::parse_extra_turn_shape(tokens)",
        "replacement_grammar::parse_additional_phases_shape(tokens)",
        "replacement_grammar::parse_split_all_shape(tokens)",
        "replacement_grammar::parse_exile_return_same_shape(tokens)",
        "replacement_grammar::parse_exile_each_target_type_shape(tokens)",
        "replacement_grammar::parse_look_hand_shape(tokens)",
        "replacement_grammar::parse_look_top_exile_one_shape(tokens)",
        "replacement_grammar::parse_voted_with_you_scry_shape(tokens)",
    ] {
        assert!(
            parser_body.contains(required),
            "{relative} should delegate parser-owned recognition to typed grammar: missing `{required}`"
        );
    }

    for forbidden in [
        "LexPattern",
        "LexCapture",
        ".match_clause(",
        ".matches_clause(",
        ".matches_prefix(",
        ".word_refs(",
        "word_slice_",
        "token_slice_",
        "replace_up_to_one_target",
        "strip_lexed_suffix",
        "REPLACE_",
    ] {
        assert!(
            !parser_body.contains(forbidden) && !content.contains(forbidden),
            "{relative} should not rediscover Oracle shapes in the sentence caller via `{forbidden}`"
        );
    }

    for (grammar_relative, required_type) in [
        (
            "crates/ironsmith-compiler-grammar/src/grammar/effects/replacement_prevention_shapes/actions.rs",
            "pub struct AdditionalPhasesShape",
        ),
        (
            "crates/ironsmith-compiler-grammar/src/grammar/effects/replacement_prevention_shapes/zones.rs",
            "pub struct ExileReturnSameShape",
        ),
        (
            "crates/ironsmith-compiler-grammar/src/grammar/effects/replacement_prevention_shapes/look.rs",
            "pub struct LookTopExileOneShape",
        ),
    ] {
        let grammar = read_repo_file(&root, grammar_relative);
        assert!(
            grammar.contains("winnow::") && grammar.contains(required_type),
            "{grammar_relative} should expose typed winnow grammar output `{required_type}`"
        );
    }
}
#[test]
pub(super) fn compile_support_tag_prefix_checks_use_named_helpers() {
    let root = workspace_root();
    let checked_files = [
        "crates/ironsmith-compiler-lowering/src/lowering_impl/compile_support/effect_dispatch.rs",
        "crates/ironsmith-compiler-lowering/src/lowering_impl/compile_support/effect_visibility_object_handlers.rs",
        "crates/ironsmith-compiler-resolve/src/tag_support.rs",
    ];
    let forbidden_fragments = [
        ".starts_with(\"revealed",
        ".starts_with(\"searched",
        ".starts_with(\"exile_cost_",
        ".starts_with(\"exiled_",
        ".starts_with(\"__sentence_helper_exiled",
        "str_starts_with(tag, \"exiled_",
        "str_starts_with(tag, \"__sentence_helper_exiled",
        "str_starts_with(last_tag, \"exile_cost_",
    ];

    for relative in checked_files {
        let content = read_repo_file(&root, relative);
        for forbidden in forbidden_fragments {
            assert!(
                !content.contains(forbidden),
                "{relative} should use named tag-family helpers instead of raw prefix check `{forbidden}`"
            );
        }
    }
}

#[test]
pub(super) fn clause_pattern_helpers_delegate_migrated_families_to_typed_grammar() {
    let root = workspace_root();
    let caller_relative =
        "crates/ironsmith-compiler-grammar/src/effect_sentences/clause_pattern_helpers.rs";
    let caller = read_repo_file(&root, caller_relative);
    let grammar_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/effects/clause_pattern_shapes.rs";
    let grammar = read_repo_file(&root, grammar_relative);

    for required in [
        "clause_shapes::parse_prevent_next_damage_tokens(tokens)",
        "clause_shapes::parse_prevent_next_time_damage_tokens(tokens)",
        "clause_shapes::parse_redirect_next_damage_tokens(tokens)",
        "clause_shapes::parse_counter_ability_target_tokens(tokens)",
        "clause_shapes::parse_keyword_mechanic_tokens(tokens)",
    ] {
        assert!(
            caller.contains(required),
            "{caller_relative} should consume the typed clause grammar result `{required}`"
        );
    }
    for required in [
        "mod counter_ability;",
        "mod damage;",
        "mod keywords;",
        "pub use counter_ability::*;",
        "pub use damage::*;",
        "pub use keywords::*;",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should expose migrated typed clause grammar: missing `{required}`"
        );
    }
    let redirect = function_source(
        &caller,
        "pub(crate) fn parse_redirect_next_damage_sentence",
        "pub(crate) fn parse_can_block_additional_creature_this_turn_clause",
    );
    for forbidden in [
        ".starts_with(&[",
        "find_phrase_start",
        "CLAUSE_REDIRECT_DAMAGE_PREFIX_PATTERN",
        "CLAUSE_THAT_DAMAGE_IS_DEALT_TO_PREFIX_PATTERN",
    ] {
        assert!(
            !redirect.contains(forbidden),
            "{caller_relative} should not rediscover redirect Oracle shape with `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn prevent_all_damage_clause_parser_uses_clause_shapes() {
    let root = workspace_root();
    let grammar_relative = "crates/ironsmith-compiler-grammar/src/grammar/effects/clause_pattern_shapes/typed_clauses.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    for required in [
        "pub enum PreventAllDamageSourceShape<'a>",
        "pub enum PreventAllDamageShape<'a>",
        "FromSource",
        "ToTarget",
        "ToTargetFromSource",
        "fn parse_duration_first_source",
        "fn parse_duration_first_target",
        "fn parse_target_first_source",
        "fn parse_target_first",
        "pub fn parse_prevent_all_damage_shape_tokens",
        "repeat_till",
        "primitives::sentence_end()",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should own typed prevent-all-damage variants with all-consuming winnow parsers: missing `{required}`"
        );
    }

    let caller_relative =
        "crates/ironsmith-compiler-grammar/src/effect_sentences/clause_pattern_helpers.rs";
    let caller = read_repo_file(&root, caller_relative);
    let consumer = function_source(
        &caller,
        "pub(crate) fn parse_prevent_all_damage_clause",
        "pub(crate) fn parse_can_attack_as_though_no_defender_clause",
    );
    for required in [
        "clause_shapes::parse_prevent_all_damage_shape_tokens(tokens)",
        "clause_shapes::PreventAllDamageShape::FromSource",
        "clause_shapes::PreventAllDamageShape::ToTarget",
        "clause_shapes::PreventAllDamageShape::ToTargetFromSource",
        "clause_shapes::PreventAllDamageSourceShape::Choice",
        "clause_shapes::PreventAllDamageSourceShape::Filter",
    ] {
        assert!(
            consumer.contains(required),
            "{caller_relative} should lower typed prevent-all-damage variants: missing `{required}`"
        );
    }
    for forbidden in [
        "classify_prevent_all_damage_clause",
        "CLAUSE_PREVENT_ALL_DAMAGE",
        "CLAUSE_SOURCES_SUFFIX",
        "CLAUSE_THIS_TURN_PATTERN",
        "token_word_refs",
        "word_slice_",
        ".starts_with(",
        ".ends_with(",
        ".matches_words(",
        "split_once",
    ] {
        assert!(
            !consumer.contains(forbidden),
            "{caller_relative} should not rediscover prevent-all-damage Oracle shapes after typed parsing: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn keyword_payload_additional_cost_recognition_uses_lexed_tail_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/keyword_payloads.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(super) fn parse_additional_cost(",
        "pub(super) fn parse_alternative_cast(",
    );

    assert!(
        helper.contains("additional_cost_tail_tokens_lexed(tokens)"),
        "{relative} should recognize additional-cost effects from parse tokens"
    );
    for forbidden in [
        "additional_cost_tail_tokens_from_text",
        "text.split_once(',')",
        "line.text.as_str()",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not recover additional-cost tails by splitting rendered text with `{forbidden}`"
        );
    }

    let registry = read_repo_file(
        &root,
        "crates/ironsmith-compiler-grammar/src/keyword_registry.rs",
    );
    assert!(
        registry.contains("(rule.parse)(line, &tokens, &full_parse_tokens)")
            && registry.contains("payload.to_line_ast()"),
        "keyword recognition must carry its typed payload through CST instead of pairing a boolean match with a lowering reparse"
    );
    assert!(
        !registry.contains("__split_kicker_label:"),
        "split kicker labels must use the typed keyword payload"
    );
}

#[test]
pub(super) fn triggered_label_source_selection_uses_lexed_dash_tokens() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/document_parser/mod.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "fn trigger_presentation_from_line_tokens",
        "fn is_nonkeyword_choice_labeled_line",
    );

    assert!(
        helper.contains("split_label_prefix_lexed")
            && helper.contains("line_starts_with_trigger_intro_tokens"),
        "{relative} should derive trigger presentation labels from document CST tokens"
    );
    for forbidden in [
        ".split_once('—')",
        "split_once(\" - \")",
        "label.contains('.')",
        "label.contains(':')",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not split or validate trigger labels with raw string branch `{forbidden}`"
        );
    }

    let legacy_lowering_relative =
        "crates/ironsmith-compiler-lowering/src/lowering_impl/lower/parser_semantic_lowering.rs";
    assert!(
        !root.join(legacy_lowering_relative).exists(),
        "trigger presentation recognition must not return to deleted parser-owned lowering module {legacy_lowering_relative}"
    );

    let semantic_relative =
        "crates/ironsmith-compiler-grammar/src/semantic_line_parsing/lines/lines_object_action.rs";
    let semantic = read_repo_file(&root, semantic_relative);
    assert!(
        !semantic.contains("presentation_label_from_raw_trigger_line")
            && !semantic.contains(".or_else(|| presentation_label_from_raw"),
        "{semantic_relative} should consume trigger presentation facts from CST/IR, not re-read raw Oracle text"
    );
}

#[test]
pub(super) fn chosen_option_context_flow_uses_typed_cst_ir_fact() {
    let root = workspace_root();
    let ir_relative = "crates/ironsmith-compiler-grammar/src/ir.rs";
    let ir = read_repo_file(&root, ir_relative);
    assert!(
        ir.contains("enum ChosenOptionContext")
            && ir.contains("StationThreshold(i32)")
            && ir.contains("ControlsSubtypePermanent(Subtype)")
            && ir.contains("ControlsEitherColorPermanent"),
        "{ir_relative} should carry typed chosen-option and threshold facts"
    );

    let relative = "crates/ironsmith-compiler-grammar/src/semantic_line_parsing/chosen_options.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(crate) fn condition_for_chosen_option",
        "pub(crate) fn wrap_chosen_option_static_chunk",
    );

    assert!(
        helper.contains("ChosenOptionContext::")
            && !helper.contains("strip_prefix")
            && !helper.contains("split_once")
            && !helper.contains("raw_line"),
        "{relative} should consume typed chosen-option contexts without decoding label strings"
    );

    for lowering_relative in [
        "crates/ironsmith-compiler-grammar/src/semantic_line_parsing/lines.rs",
        "crates/ironsmith-compiler-grammar/src/semantic_line_parsing/activated.rs",
        "crates/ironsmith-compiler-lowering/src/lowering_impl/lower/line_ast_helpers.rs",
    ] {
        let lowering = read_repo_file(&root, lowering_relative);
        assert!(
            !lowering.contains("__max_speed_condition")
                && !lowering.contains("__station_threshold_")
                && !lowering.contains("__control_subtype_permanent_")
                && !lowering.contains("__control_color_pair_permanent_"),
            "{lowering_relative} should not decode chosen-option semantics from magic strings"
        );
    }
}

#[test]
pub(super) fn partner_parenthetical_trims_are_typed_grammar_owned() {
    let root = workspace_root();
    let grammar_relative = "crates/ironsmith-compiler-grammar/src/grammar/keyword_special_lines.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    let entry = function_source(
        &grammar,
        "pub(crate) fn parse_partner_with_name_shape_tokens",
        "pub(crate) fn parse_partner_visible_label_tokens",
    );
    let recognizer = function_source(
        &grammar,
        "fn parse_partner_with_name_shape_lexed",
        "fn parse_partner_visible_label_lexed",
    );

    for required in [
        "pub struct PartnerWithNameShape<'a>",
        "pub name_tokens: &'a [OwnedLexToken]",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should define a typed partner-name capture: missing `{required}`"
        );
    }
    for required in [
        "primitives::parse_prefix(tokens, parse_partner_with_name_shape_lexed)",
        "Some(PartnerWithNameShape { name_tokens })",
        "render_token_slice(shape.name_tokens)",
    ] {
        assert!(
            entry.contains(required),
            "{grammar_relative} should expose the captured partner-name token range as a typed grammar result: missing `{required}`"
        );
    }
    for required in [
        "WResult<&'a [OwnedLexToken]>",
        "primitives::phrase(&[\"partner\", \"with\"])",
        "repeat_till",
        "primitives::token_kind(TokenKind::LParen)",
        "primitives::token_kind(TokenKind::Period)",
        "eof.value(())",
    ] {
        assert!(
            recognizer.contains(required),
            "{grammar_relative} should own partner parenthetical/terminal boundaries with winnow token kinds: missing `{required}`"
        );
    }
    for forbidden in [
        "try_lower_partner_with_text",
        "raw_line: &str",
        "normalized_text: &str",
        "lex_line(raw_line",
        "lex_line(normalized_text",
        "partner_with_name_from_text",
        "split_once('(')",
        "str_split_once_char",
        "trim_end_matches('.')",
        "\"partner with \".len()",
    ] {
        assert!(
            !entry.contains(forbidden) && !recognizer.contains(forbidden),
            "{grammar_relative} should not trim partner parentheticals with raw string branch `{forbidden}`"
        );
    }

    let semantic_relative =
        "crates/ironsmith-compiler-grammar/src/semantic_line_parsing/lines/lines_object_action.rs";
    let semantic = read_repo_file(&root, semantic_relative);
    let adapter = function_source(
        &semantic,
        "pub(super) fn try_lower_partner_with_tokens",
        "pub(super) fn partner_with_name_from_tokens",
    );
    let typed_name = function_source(
        &semantic,
        "pub(super) fn partner_with_name_from_tokens",
        "#[test]",
    );
    assert!(
        adapter.contains("partner_with_name_from_tokens(parse_tokens)")
            && typed_name
                .contains("keyword_special_grammar::parse_partner_with_name_tokens(tokens)"),
        "{semantic_relative} should consume the typed partner-name grammar result"
    );
    for forbidden in [
        "TokenKind::LParen",
        "split_once",
        "str_split_once_char",
        "trim_end_matches",
        "raw_line",
        "normalized_text",
    ] {
        assert!(
            !adapter.contains(forbidden) && !typed_name.contains(forbidden),
            "{semantic_relative} should not rediscover partner-name boundaries after typed parsing: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn semantic_line_hideaway_special_case_uses_token_words() {
    let root = workspace_root();
    let relative =
        "crates/ironsmith-compiler-grammar/src/semantic_line_parsing/lines/lines_object_action.rs";
    let content = read_repo_file(&root, relative);
    let helper = function_source(
        &content,
        "pub(super) fn try_lower_hideaway_tokens",
        "#[test]",
    );

    assert!(
        helper.contains("semantic_grammar::parse_hideaway_keyword_tokens(parse_tokens)?")
            && helper.contains("hideaway_line_ast(shape.count)"),
        "{relative} should lower hideaway from the typed grammar capture"
    );
    for forbidden in [
        "try_lower_hideaway_tokens(parse_tokens, line.info.raw_line.as_str())",
        "raw_line: &str",
        "try_lower_hideaway_text",
        "split_whitespace()",
        "trim_matches",
        "eq_ignore_ascii_case",
    ] {
        assert!(
            !helper.contains(forbidden),
            "{relative} should not lower hideaway by normalizing rendered text with `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn verb_handlers_do_not_use_raw_clause_shape_word_matching() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/effect_sentences/verb_handlers";
    let dir = root.join(relative);
    let mut files = Vec::new();
    collect_rust_files(&dir, &mut files);

    for file in files {
        let content = fs::read_to_string(&file)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", file.display()));
        assert!(
            !content.contains(".matches_words("),
            "{} should route verb-handler shape gates through LexedClause/token matching",
            repo_relative(&root, &file)
        );
    }
}

#[test]
pub(super) fn clause_dispatch_routes_shape_recognition_through_typed_grammar() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/effect_sentences/clause_dispatch.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "grammar::effects::clause_dispatch_shapes as clause_grammar",
        "clause_grammar::parse_clause_subject_verb_shape(tokens)",
        "clause_grammar::parse_direct_clause_shape(tokens)",
        "clause_grammar::parse_pump_subject_shape(subject_tokens)",
        "clause_grammar::parse_cast_any_tagged_shape(tokens)",
        "clause_grammar::parse_passive_sacrifice_shape(tokens)",
        "parse_hexproof_targeting_override_clause(tokens)?",
    ] {
        assert!(
            content.contains(required),
            "{relative} should lower typed clause grammar results: missing `{required}`"
        );
    }

    for forbidden in [
        "clause_shape!",
        "LexPattern",
        ".matches_word(",
        ".matches_token(",
        "strip_leading_pump_subject_duration",
        "dispatch_words_eq",
        "rest_starts_all_abilities_shared_gain",
        "is_tagged_object_reference",
        "CAST_ANY_NUMBER_OF_SPELLS_PREFIX",
        "RING_TEMPTS_YOU_WORDS",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not rediscover migrated clause shapes: found `{forbidden}`"
        );
    }

    for grammar_relative in [
        "crates/ironsmith-compiler-grammar/src/grammar/effects/clause_dispatch_shapes/core.rs",
        "crates/ironsmith-compiler-grammar/src/grammar/effects/clause_dispatch_shapes/direct.rs",
        "crates/ironsmith-compiler-grammar/src/grammar/effects/clause_dispatch_shapes/permissions.rs",
        "crates/ironsmith-compiler-grammar/src/grammar/effects/clause_dispatch_shapes/relational.rs",
    ] {
        let grammar_content = read_repo_file(&root, grammar_relative);
        assert!(
            grammar_content.contains("winnow::"),
            "{grammar_relative} should recognize clause shapes with winnow"
        );
        assert!(
            !grammar_content.contains("nom::"),
            "{grammar_relative} must not introduce nom"
        );
    }
}

#[test]
pub(super) fn return_exchange_routes_shape_gates_through_lexed_clauses() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/effect_sentences/return_exchange.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "grammar::effects::parse_return_clause_shape(tokens)",
        "grammar::effects::ReturnTargetShape::All",
        "grammar::effects::ReturnTargetShape::Singular",
        "grammar::effects::parse_exchange_clause_shape(tokens)",
        "ExchangeClauseShape::LifeTotalsWith",
        "ExchangeClauseShape::Values",
        "grammar::effects::parse_exchange_value_operands",
        "grammar::effects::parse_return_timing_words_shape(words)",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route return/exchange gates through typed grammar shapes: missing `{required}`"
        );
    }

    for forbidden in [
        "fn return_shape_matches_words(",
        "return_find_phrase_start",
        "return_find_prefix_start",
        "return_word_is_any",
        "word_slice_eq(",
        "word_slice_eq_any(",
        "words_start_with(",
        "words_start_with_any(",
        "locate_index(",
        "synthetic_word_tokens",
        "LexedClause",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "_PATTERN",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route return/exchange gates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn counter_marker_family_routes_shape_gates_through_typed_grammar() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler-grammar/src/effect_sentences/subject_verb_primitives/counter_marker_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "grammar::effects::counter_marker_shapes as counter_shapes",
        "counter_shapes::parse_return_with_counters_tokens(clause.tokens())",
        "counter_shapes::parse_put_onto_battlefield_with_counters_tokens(clause.tokens())",
        "counter_shapes::parse_if_enters_additional_tokens(clause.tokens())",
        "counter_shapes::parse_tagged_enters_additional_tokens(clause.tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route counter-marker shape gates through typed grammar: missing `{required}`"
        );
    }

    for forbidden in [
        "LexPattern",
        "LexCaptureRole",
        "counter_marker_control_tail_controller",
        "counter_marker_matches_accepted_target",
        "word_slice_eq_any",
        ".matches_words(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not retain parser-shape helpers after typed grammar migration: found `{forbidden}`"
        );
    }

    let grammar_relative =
        "crates/ironsmith-compiler-grammar/src/grammar/effects/counter_marker_shapes.rs";
    let grammar = read_repo_file(&root, grammar_relative);
    for required in [
        "struct CounterDescriptorShape",
        "struct MoveWithCountersShape",
        "enum CounterMarkerTimingShape",
        "fn parse_return_with_counters_lexed",
        "fn parse_if_enters_additional_lexed",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should own typed counter-marker recognition: missing `{required}`"
        );
    }
}
