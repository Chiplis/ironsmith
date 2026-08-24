#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::*;

#[test]
pub(super) fn sentence_shape_predicates_route_shape_gates_through_typed_grammar() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/front_end/grammar/effect_clauses/effect_sentences/dispatch_inner/sentence_shape_predicates.rs";
    let content = read_repo_file(&root, relative);
    let grammar_relative =
        "crates/ironsmith-compiler/src/front_end/grammar/effects/sentence_predicate_shapes.rs";
    let grammar = read_repo_file(&root, grammar_relative);

    for required in [
        "sentence_shapes::parse_trailing_counter_constraint_tokens(tokens)",
        "sentence_shapes::parse_power_damage_self_tokens(tokens)",
        "sentence_shapes::parse_tapped_this_way_binding_tokens",
        "sentence_shapes::parse_where_x_sentence_tokens(tokens)",
        "sentence_shapes::parse_where_x_value_shape_tokens",
    ] {
        assert!(
            content.contains(required),
            "{relative} should route sentence-shape predicates through typed grammar: missing `{required}`"
        );
    }

    for required in [
        "enum WhereXValueShape",
        "struct WhereXSentenceShape",
        "struct AuraEnchantmentShape",
        "enum DelayedSentenceShape",
        "fn parse_where_x_value_shape_tokens",
        "use winnow::",
    ] {
        assert!(
            grammar.contains(required),
            "{grammar_relative} should own typed sentence-shape recognition: missing `{required}`"
        );
    }

    for forbidden in [
        "ClauseShape",
        "clause_shape",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
        "sentence_shape_matches_words",
        "sentence_find_phrase_start",
        "_PATTERN",
        "SENTENCE_WHERE_X_IS_PREFIX",
        "sentence_removed_counters_this_way",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not route sentence-shape predicates through ClauseShape adapters: found `{forbidden}`"
        );
    }
}

#[test]
pub(super) fn token_copy_control_uses_typed_grammar_shapes() {
    let root = workspace_root();
    let relative = "crates/ironsmith-compiler/src/front_end/grammar/effect_clauses/effect_sentences/subject_verb_primitives/token_copy_control_family.rs";
    let content = read_repo_file(&root, relative);

    for required in [
        "effect_grammar::parse_each_player_reveal_permanents_shape(clause.tokens())",
        "effect_grammar::parse_return_same_subtypes_shape(clause.tokens())",
        "effect_grammar::parse_choose_same_filter_shape(clause.tokens())",
        "effect_grammar::parse_choose_sequence_shape(clause.tokens())",
        "effect_grammar::parse_sacrifice_choice_shape(clause.tokens())",
        "effect_grammar::parse_then_sequence_shape(clause.tokens())",
        "effect_grammar::parse_return_create_shape(clause.tokens())",
        "effect_grammar::parse_exile_may_put_shape(clause.tokens())",
        "effect_grammar::parse_exile_shuffle_shape(clause.tokens())",
        "effect_grammar::parse_exile_source_counter_shape(clause.tokens())",
        "effect_grammar::parse_comma_then_special_shape(clause.tokens())",
        "effect_grammar::parse_destroy_land_damage_shape(clause.tokens())",
        "effect_grammar::parse_destroy_attached_shape(clause.tokens())",
    ] {
        assert!(
            content.contains(required),
            "{relative} should lower typed token/copy/control grammar facts: missing `{required}`"
        );
    }

    for forbidden in [
        "LexPattern",
        "LexCapture",
        ".match_pattern(",
        ".word_refs()",
        "word_slice_",
        "words_start_with(",
        "find_phrase_start(",
        "rfind_token_word(",
        ".matches_word(",
        ".matches_token(",
        ".matches_words(",
    ] {
        assert!(
            !content.contains(forbidden),
            "{relative} should not retain parser-shape probe `{forbidden}`"
        );
    }
}
