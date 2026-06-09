use super::*;
use crate::runtime_backend::lexer::{
    word_slice_contains_all_words, word_slice_contains_any_word, word_slice_contains_phrase,
    word_slice_starts_with_any,
};

const EACH_PLAYER_PREFIXES: &[&[&str]] = &[&["each", "player"]];
const EACH_PLAYER_EXILES_ALL_PREFIXES: &[&[&str]] = &[&["each", "player", "exiles", "all"]];
const EXILE_PREFIXES: &[&[&str]] = &[&["exile"]];
const RETURN_EACH_CREATURE_ISNT_PREFIXES: &[&[&str]] =
    &[&["return", "each", "creature", "that", "isnt"]];
const SPLIT_NEGATION_PHRASES: &[&[&str]] = &[&["does", "not"], &["do", "not"], &["can", "not"]];
const NEGATED_UNTAP_DURING_PHRASES: &[&[&str]] =
    &[&["dont", "untap", "during"], &["doesnt", "untap", "during"]];
const CONTROLLERS_UNTAP_STEP_PHRASES: &[&[&str]] = &[
    &["controllers", "untap", "step"],
    &["controllers", "untap", "steps"],
];
const AS_COPY_PREFIXES: &[&[&str]] =
    &[&["as", "a", "copy"], &["as", "an", "copy"], &["as", "copy"]];
const MOST_COMMON_COLOR_EXCLUDED_SHARES_PHRASE: &[&str] = &[
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
];
const PUT_INTO_GRAVEYARDS_FROM_BATTLEFIELD_THIS_TURN_PHRASE: &[&str] = &[
    "put",
    "into",
    "graveyards",
    "from",
    "the",
    "battlefield",
    "this",
    "turn",
];
const DEFENDING_PLAYER_CHOICE_PHRASES: &[&[&str]] = &[
    &["defending", "player's", "choice"],
    &["defending", "player", "choice"],
    &["player's", "choice", "target"],
    &["defending", "player", "s", "choice"],
];

fn unsupported_words_contain_phrase(words: &[&str], phrase: &[&str]) -> bool {
    word_slice_contains_phrase(words, phrase)
}

fn unsupported_words_contain_all(words: &[&str], required: &[&str]) -> bool {
    word_slice_contains_all_words(words, required)
}

fn unsupported_words_contain_any(words: &[&str], candidates: &[&str]) -> bool {
    word_slice_contains_any_word(words, candidates)
}

fn unsupported_words_contain_any_phrase(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| unsupported_words_contain_phrase(words, phrase))
}

fn unsupported_prefix_start(words: &[&str], prefixes: &[&[&str]]) -> Option<usize> {
    (0..words.len()).find(|idx| word_slice_starts_with_any(&words[*idx..], prefixes))
}

pub(crate) fn is_enters_as_copy_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let as_copy_idx = unsupported_prefix_start(&words, AS_COPY_PREFIXES);
    match as_copy_idx {
        Some(idx) => tokens[..idx].iter().any(|token| {
            token
                .as_word()
                .is_some_and(|word| matches!(word, "enter" | "enters"))
        }),
        None => false,
    }
}

pub(crate) fn is_negated_untap_clause_words(words: &[&str]) -> bool {
    if words.len() < 3 {
        return false;
    }
    let has_untap = unsupported_words_contain_any(words, &["untap", "untaps"]);
    let has_negation = unsupported_words_contain_any(words, &["doesnt", "dont", "cant"])
        || unsupported_words_contain_any_phrase(words, SPLIT_NEGATION_PHRASES);
    has_untap && has_negation
}

pub(crate) fn is_negated_untap_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let has_untap = unsupported_words_contain_any(&words, &["untap", "untaps"]);
    let has_negation = unsupported_words_contain_any(&words, &["doesnt", "dont", "cant"])
        || unsupported_words_contain_any_phrase(&words, SPLIT_NEGATION_PHRASES);
    has_untap && has_negation
}

pub(crate) fn looks_like_supported_negated_untap_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    let has_negated_untap =
        unsupported_words_contain_any_phrase(words.as_slice(), NEGATED_UNTAP_DURING_PHRASES);
    let has_controllers_untap_step =
        unsupported_words_contain_any_phrase(words.as_slice(), CONTROLLERS_UNTAP_STEP_PHRASES);
    let has_tapped_duration =
        unsupported_words_contain_phrase(words.as_slice(), &["for", "as", "long", "as"])
            && unsupported_words_contain_all(words.as_slice(), &["remains", "tapped"]);
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
    unsupported_words_contain_phrase(&words, &["one", "of", "them", "into", "your"])
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
        && unsupported_words_contain_phrase(&words, &["all", "abilities"]);
    has_loses_all_abilities && primitives::contains_word(tokens, "becomes")
}

pub(crate) fn has_spent_to_cast_this_spell_without_condition_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    unsupported_words_contain_phrase(&words, &["was", "spent", "to", "cast", "this", "spell"])
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
    unsupported_words_contain_phrase(
        &crate::runtime_backend::token_word_refs(tokens),
        &["different", "mana", "value"],
    )
}

pub(crate) fn has_most_common_color_constraint_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if unsupported_words_contain_phrase(&words, MOST_COMMON_COLOR_EXCLUDED_SHARES_PHRASE) {
        return false;
    }
    unsupported_words_contain_phrase(&words, &["most", "common", "color", "among", "all"])
        && primitives::contains_word(tokens, "permanents")
}

pub(crate) fn has_power_vs_count_constraint_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    primitives::contains_word(tokens, "power")
        && unsupported_words_contain_phrase(
            &words,
            &["less", "than", "or", "equal", "to", "the", "number", "of"],
        )
}

pub(crate) fn has_put_into_graveyards_from_battlefield_this_turn_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    unsupported_words_contain_phrase(
        &crate::runtime_backend::token_word_refs(tokens),
        PUT_INTO_GRAVEYARDS_FROM_BATTLEFIELD_THIS_TURN_PHRASE,
    )
}

pub(crate) fn has_phase_out_until_leaves_clause_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    (primitives::contains_word(tokens, "phase")
        || primitives::contains_word(tokens, "phases")
        || primitives::contains_word(tokens, "phased"))
        && primitives::contains_word(tokens, "until")
        && unsupported_words_contain_phrase(&words, &["leaves", "the", "battlefield"])
}

pub(crate) fn has_same_name_as_another_in_hand_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    unsupported_words_contain_phrase(
        &crate::runtime_backend::token_word_refs(tokens),
        &["same", "name", "as", "another", "card", "in"],
    ) && primitives::contains_word(tokens, "hand")
}

pub(crate) fn has_for_each_mana_from_spent_to_cast_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    unsupported_words_contain_phrase(&words, &["for", "each", "mana", "from"])
        && primitives::contains_word(tokens, "spent")
        && unsupported_words_contain_phrase(&words, &["cast", "this", "spell", "create"])
}

pub(crate) fn has_when_you_sacrifice_this_way_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    unsupported_words_contain_phrase(&words, &["when", "you", "sacrifice"])
        && unsupported_words_contain_phrase(&words, &["this", "way"])
}

pub(crate) fn has_greatest_mana_value_clause_sentence_lexed(words: &[&str]) -> bool {
    unsupported_words_contain_phrase(words, &["greatest", "mana", "value"])
}

pub(crate) fn has_least_power_among_creatures_clause_sentence_lexed(words: &[&str]) -> bool {
    unsupported_words_contain_phrase(words, &["least", "power", "among", "creatures"])
}

pub(crate) fn has_villainous_choice_clause_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "villainous") && primitives::contains_word(tokens, "choice")
}

pub(crate) fn has_divided_evenly_clause_sentence_lexed(words: &[&str]) -> bool {
    unsupported_words_contain_phrase(words, &["divided", "evenly"])
}

pub(crate) fn has_different_names_clause_sentence_lexed(words: &[&str]) -> bool {
    if words.first().is_some_and(|word| *word == "choose") {
        return false;
    }
    unsupported_words_contain_phrase(words, &["different", "names"])
}

pub(crate) fn has_chosen_at_random_clause_sentence_lexed(words: &[&str]) -> bool {
    unsupported_words_contain_phrase(words, &["chosen", "at", "random"])
}

pub(crate) fn has_defending_players_choice_clause_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    unsupported_words_contain_any_phrase(
        &crate::runtime_backend::token_word_refs(tokens),
        DEFENDING_PLAYER_CHOICE_PHRASES,
    )
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
    unsupported_words_contain_phrase(words, &["if", "you", "sacrifice", "an", "island"])
        && unsupported_words_contain_phrase(words, &["this", "way"])
}

pub(crate) fn has_spent_to_cast_clause_sentence_lexed(words: &[&str]) -> bool {
    unsupported_words_contain_phrase(words, &["spent", "to", "cast"])
}

pub(crate) fn has_face_down_clause_sentence_lexed(
    words: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    let has_face_down = unsupported_words_contain_phrase(words, &["face", "down"])
        || words
            .iter()
            .any(|word| matches!(*word, "face-down" | "facedown"));
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
        unsupported_words_contain_phrase(&words, &["for", "as", "long", "as", "you", "control"]);
    let has_supported_source_tapped_duration =
        unsupported_words_contain_phrase(&words, &["for", "as", "long", "as"])
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
