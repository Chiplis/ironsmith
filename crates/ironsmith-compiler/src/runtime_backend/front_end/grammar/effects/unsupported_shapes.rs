use super::*;
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};

const EACH_PLAYER_PREFIXES: &[&[&str]] = &[&["each", "player"]];
const EACH_PLAYER_EXILES_ALL_PREFIXES: &[&[&str]] = &[&["each", "player", "exiles", "all"]];
const EXILE_PREFIXES: &[&[&str]] = &[&["exile"]];
const RETURN_EACH_CREATURE_ISNT_PREFIXES: &[&[&str]] =
    &[&["return", "each", "creature", "that", "isnt"]];
const UNTAP_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["untap", "untaps"]]);
const CONTRACTION_NEGATION_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["doesnt", "dont", "cant"]]);
const SPLIT_NEGATION_MARKER_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[&["does", "not"], &["do", "not"], &["can", "not"]]]);
const NEGATED_UNTAP_DURING_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[&["dont", "untap", "during"], &["doesnt", "untap", "during"]]]);
const CONTROLLERS_UNTAP_STEP_PATTERN: ClauseShape<'static> = ClauseShape::new()
    .contains_any_phrases(&[&[
        &["controllers", "untap", "step"],
        &["controllers", "untap", "steps"],
    ]]);
const FOR_AS_LONG_AS_REMAINS_TAPPED_PATTERN: ClauseShape<'static> = clause_shape!(contains_phrases & [&["for", "as", "long", "as"]]; contains_words & ["remains", "tapped"]);
const GREATEST_MANA_VALUE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["greatest", "mana", "value"]]);
const LEAST_POWER_AMONG_CREATURES_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["least", "power", "among", "creatures"]]);
const DIVIDED_EVENLY_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["divided", "evenly"]]);
const DIFFERENT_NAMES_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["different", "names"]]);
const CHOSEN_AT_RANDOM_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["chosen", "at", "random"]]);
const IF_SACRIFICE_ISLAND_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [
            &["if", "you", "sacrifice", "an", "island"],
            &["this", "way"]
        ]
);
const SPENT_TO_CAST_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["spent", "to", "cast"]]);
const FACE_DOWN_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["face", "down"]]);
const FACE_DOWN_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["face-down"], &["facedown"]]);
const ENTER_OR_ENTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enter"], &["enters"]]);
const AS_COPY_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["as", "a", "copy"], &["as", "an", "copy"], &["as", "copy"]]);
const PUT_ONE_OF_THEM_INTO_YOUR_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["one", "of", "them", "into", "your"]]);
const ALL_ABILITIES_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["all", "abilities"]]);
const WAS_SPENT_TO_CAST_THIS_SPELL_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["was", "spent", "to", "cast", "this", "spell"]]);
const DIFFERENT_MANA_VALUE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["different", "mana", "value"]]);
const MOST_COMMON_COLOR_EXCLUDED_SHARES_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [&[
            "shares",
            "a",
            "color",
            "with",
            "the",
            "most",
            "common",
            "color",
            "among",
            "all",
            "permanents",
            "or",
            "a",
            "color",
            "tied",
            "for",
            "most",
            "common",
        ]]
);
const MOST_COMMON_COLOR_AMONG_ALL_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["most", "common", "color", "among", "all"]]);
const POWER_LESS_THAN_OR_EQUAL_COUNT_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases & [&["less", "than", "or", "equal", "to", "the", "number", "of",]]
);
const PUT_INTO_GRAVEYARDS_FROM_BATTLEFIELD_THIS_TURN_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [&[
            "put",
            "into",
            "graveyards",
            "from",
            "the",
            "battlefield",
            "this",
            "turn",
        ]]
);
const LEAVES_THE_BATTLEFIELD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["leaves", "the", "battlefield"]]);
const SAME_NAME_AS_ANOTHER_CARD_IN_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["same", "name", "as", "another", "card", "in"]]);
const FOR_EACH_MANA_FROM_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["for", "each", "mana", "from"]]);
const CAST_THIS_SPELL_CREATE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["cast", "this", "spell", "create"]]);
const WHEN_YOU_SACRIFICE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["when", "you", "sacrifice"]]);
const THIS_WAY_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["this", "way"]]);
const DEFENDING_PLAYER_CHOICE_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["defending", "player's", "choice"],
            &["defending", "player", "choice"],
            &["player's", "choice", "target"],
            &["defending", "player", "s", "choice"],
        ]]
);
const FOR_AS_LONG_AS_YOU_CONTROL_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["for", "as", "long", "as", "you", "control"]]);
const FOR_AS_LONG_AS_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["for", "as", "long", "as"]]);

fn unsupported_phrase_start(words: &[&str], shape: ClauseShape<'static>) -> Option<usize> {
    (0..words.len()).find(|idx| shape.matches_words(&words[*idx..]))
}

pub(crate) fn is_enters_as_copy_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let as_copy_idx = unsupported_phrase_start(&words, AS_COPY_TAIL_PATTERN);
    match as_copy_idx {
        Some(idx) => tokens[..idx]
            .iter()
            .any(|token| ENTER_OR_ENTERS_WORD_PATTERN.matches_token(token)),
        None => false,
    }
}

pub(crate) fn is_negated_untap_clause_words(words: &[&str]) -> bool {
    if words.len() < 3 {
        return false;
    }
    let has_untap = UNTAP_MARKER_PATTERN.matches_words(words);
    let has_negation = CONTRACTION_NEGATION_MARKER_PATTERN.matches_words(words)
        || SPLIT_NEGATION_MARKER_PATTERN.matches_words(words);
    has_untap && has_negation
}

pub(crate) fn is_negated_untap_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let has_untap = UNTAP_MARKER_PATTERN.matches_words(&words);
    let has_negation = CONTRACTION_NEGATION_MARKER_PATTERN.matches_words(&words)
        || SPLIT_NEGATION_MARKER_PATTERN.matches_words(&words);
    has_untap && has_negation
}

pub(crate) fn looks_like_supported_negated_untap_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    let has_negated_untap = NEGATED_UNTAP_DURING_PATTERN.matches_words(words.as_slice());
    let has_controllers_untap_step = CONTROLLERS_UNTAP_STEP_PATTERN.matches_words(words.as_slice());
    let has_tapped_duration = FOR_AS_LONG_AS_REMAINS_TAPPED_PATTERN.matches_words(words.as_slice());
    has_negated_untap && has_controllers_untap_step && has_tapped_duration
}

pub(crate) fn has_each_player_lose_discard_sacrifice_chain_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::words_match_any_prefix(tokens, EACH_PLAYER_PREFIXES).is_some()
        && primitives::contains_word(tokens, "then")
        && (primitives::contains_word(tokens, "lose") || primitives::contains_word(tokens, "loses"))
        && (primitives::contains_word(tokens, "discard")
            || primitives::contains_word(tokens, "discards"))
        && (primitives::contains_word(tokens, "sacrifice")
            || primitives::contains_word(tokens, "sacrifices"))
}

pub(crate) fn has_each_player_exile_sacrifice_return_exiled_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::words_match_any_prefix(tokens, EACH_PLAYER_EXILES_ALL_PREFIXES).is_some()
        && primitives::contains_word(tokens, "sacrifices")
        && primitives::contains_word(tokens, "puts")
        && primitives::contains_word(tokens, "exiled")
        && primitives::contains_word(tokens, "this")
        && primitives::contains_word(tokens, "way")
}

pub(crate) fn has_put_one_of_them_into_hand_rest_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    PUT_ONE_OF_THEM_INTO_YOUR_MARKER_PATTERN.matches_words(&words)
        && primitives::contains_word(tokens, "rest")
        && (primitives::contains_word(tokens, "graveyard")
            || primitives::contains_word(tokens, "graveyards"))
}

pub(crate) fn has_loses_all_abilities_with_becomes_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let has_loses_all_abilities = (primitives::contains_word(tokens, "lose")
        || primitives::contains_word(tokens, "loses"))
        && ALL_ABILITIES_MARKER_PATTERN.matches_words(&words);
    has_loses_all_abilities && primitives::contains_word(tokens, "becomes")
}

pub(crate) fn has_spent_to_cast_this_spell_without_condition_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    WAS_SPENT_TO_CAST_THIS_SPELL_MARKER_PATTERN.matches_words(&words)
        && !primitives::contains_word(tokens, "if")
        && !primitives::contains_word(tokens, "unless")
}

pub(crate) fn has_would_enter_instead_replacement_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::contains_word(tokens, "would")
        && (primitives::contains_word(tokens, "enter")
            || primitives::contains_word(tokens, "enters"))
        && primitives::contains_word(tokens, "instead")
}

pub(crate) fn has_different_mana_value_constraint_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    DIFFERENT_MANA_VALUE_MARKER_PATTERN
        .matches_words(&crate::runtime_backend::token_word_refs(tokens))
}

pub(crate) fn has_most_common_color_constraint_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if MOST_COMMON_COLOR_EXCLUDED_SHARES_PATTERN.matches_words(&words) {
        return false;
    }
    MOST_COMMON_COLOR_AMONG_ALL_MARKER_PATTERN.matches_words(&words)
        && primitives::contains_word(tokens, "permanents")
}

pub(crate) fn has_power_vs_count_constraint_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    primitives::contains_word(tokens, "power")
        && POWER_LESS_THAN_OR_EQUAL_COUNT_MARKER_PATTERN.matches_words(&words)
}

pub(crate) fn has_put_into_graveyards_from_battlefield_this_turn_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    PUT_INTO_GRAVEYARDS_FROM_BATTLEFIELD_THIS_TURN_MARKER_PATTERN
        .matches_words(&crate::runtime_backend::token_word_refs(tokens))
}

pub(crate) fn has_phase_out_until_leaves_clause_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    (primitives::contains_word(tokens, "phase")
        || primitives::contains_word(tokens, "phases")
        || primitives::contains_word(tokens, "phased"))
        && primitives::contains_word(tokens, "until")
        && LEAVES_THE_BATTLEFIELD_MARKER_PATTERN.matches_words(&words)
}

pub(crate) fn has_same_name_as_another_in_hand_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    SAME_NAME_AS_ANOTHER_CARD_IN_MARKER_PATTERN
        .matches_words(&crate::runtime_backend::token_word_refs(tokens))
        && primitives::contains_word(tokens, "hand")
}

pub(crate) fn has_for_each_mana_from_spent_to_cast_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    FOR_EACH_MANA_FROM_MARKER_PATTERN.matches_words(&words)
        && primitives::contains_word(tokens, "spent")
        && CAST_THIS_SPELL_CREATE_MARKER_PATTERN.matches_words(&words)
}

pub(crate) fn has_when_you_sacrifice_this_way_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    WHEN_YOU_SACRIFICE_MARKER_PATTERN.matches_words(&words)
        && THIS_WAY_MARKER_PATTERN.matches_words(&words)
}

pub(crate) fn has_greatest_mana_value_clause_sentence_lexed(words: &[&str]) -> bool {
    GREATEST_MANA_VALUE_PATTERN.matches_words(words)
}

pub(crate) fn has_least_power_among_creatures_clause_sentence_lexed(words: &[&str]) -> bool {
    LEAST_POWER_AMONG_CREATURES_PATTERN.matches_words(words)
}

pub(crate) fn has_villainous_choice_clause_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "villainous") && primitives::contains_word(tokens, "choice")
}

pub(crate) fn has_divided_evenly_clause_sentence_lexed(words: &[&str]) -> bool {
    DIVIDED_EVENLY_PATTERN.matches_words(words)
}

pub(crate) fn has_different_names_clause_sentence_lexed(words: &[&str]) -> bool {
    if words.first().is_some_and(|word| *word == "choose") {
        return false;
    }
    DIFFERENT_NAMES_PATTERN.matches_words(words)
}

pub(crate) fn has_chosen_at_random_clause_sentence_lexed(words: &[&str]) -> bool {
    CHOSEN_AT_RANDOM_PATTERN.matches_words(words)
}

pub(crate) fn has_defending_players_choice_clause_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    DEFENDING_PLAYER_CHOICE_MARKER_PATTERN
        .matches_words(&crate::runtime_backend::token_word_refs(tokens))
}

pub(crate) fn has_target_creature_token_player_planeswalker_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::contains_word(tokens, "target")
        && primitives::contains_word(tokens, "creature")
        && primitives::contains_word(tokens, "token")
        && primitives::contains_word(tokens, "player")
        && primitives::contains_word(tokens, "planeswalker")
}

pub(crate) fn has_if_you_sacrifice_an_island_this_way_clause_sentence_lexed(
    words: &[&str],
) -> bool {
    IF_SACRIFICE_ISLAND_THIS_WAY_PATTERN.matches_words(words)
}

pub(crate) fn has_spent_to_cast_clause_sentence_lexed(words: &[&str]) -> bool {
    SPENT_TO_CAST_PATTERN.matches_words(words)
}

pub(crate) fn has_face_down_clause_sentence_lexed(
    words: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    let has_face_down = FACE_DOWN_PATTERN.matches_words(words)
        || words
            .iter()
            .any(|word| FACE_DOWN_WORD_PATTERN.matches_word(word));
    if !has_face_down {
        return false;
    }

    if matches!(
        words,
        ["look", "at", "target", "face", "down", "creature"]
            | ["look", "at", "target", "face", "down", "creatures"]
            | ["look", "at", "target", "face", "down", "permanent"]
            | ["look", "at", "target", "face", "down", "permanents"]
    ) {
        return false;
    }

    let simple_exile_face_down = primitives::words_match_any_prefix(tokens, EXILE_PREFIXES)
        .is_some()
        && !primitives::contains_word(tokens, "then")
        && !primitives::contains_word(tokens, "manifest")
        && !primitives::contains_word(tokens, "pile");
    !simple_exile_face_down
}

pub(crate) fn has_return_each_creature_that_isnt_list_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::words_match_any_prefix(tokens, RETURN_EACH_CREATURE_ISNT_PREFIXES).is_some()
        && primitives::contains_word(tokens, "or")
}

pub(crate) fn has_unsupported_negated_untap_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let has_supported_control_duration =
        FOR_AS_LONG_AS_YOU_CONTROL_MARKER_PATTERN.matches_words(&words);
    let has_supported_source_tapped_duration = FOR_AS_LONG_AS_MARKER_PATTERN.matches_words(&words)
        && primitives::contains_word(tokens, "remains")
        && primitives::contains_word(tokens, "tapped")
        && (primitives::contains_word(tokens, "this")
            || primitives::contains_word(tokens, "thiss")
            || primitives::contains_word(tokens, "source")
            || primitives::contains_word(tokens, "artifact")
            || primitives::contains_word(tokens, "creature")
            || primitives::contains_word(tokens, "permanent"));
    is_negated_untap_clause_lexed(tokens)
        && !primitives::contains_word(tokens, "and")
        && !primitives::contains_word(tokens, "next")
        && !has_supported_control_duration
        && !has_supported_source_tapped_duration
        && primitives::contains_word(tokens, "during")
        && (primitives::contains_word(tokens, "step") || primitives::contains_word(tokens, "steps"))
}
