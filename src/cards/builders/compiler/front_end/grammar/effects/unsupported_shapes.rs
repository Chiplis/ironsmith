use super::*;

const EACH_PLAYER_PREFIXES: &[&[&str]] = &[&["each", "player"]];
const EACH_PLAYER_EXILES_ALL_PREFIXES: &[&[&str]] = &[&["each", "player", "exiles", "all"]];
const EXILE_PREFIXES: &[&[&str]] = &[&["exile"]];
const RETURN_EACH_CREATURE_ISNT_PREFIXES: &[&[&str]] =
    &[&["return", "each", "creature", "that", "isnt"]];

fn contains_word_window(words: &[&str], pattern: &[&str]) -> bool {
    word_slice_contains_sequence(words, pattern)
}

fn contains_any_word_window(words: &[&str], patterns: &[&[&str]]) -> bool {
    patterns.iter().any(|pattern| contains_word_window(words, pattern))
}

fn slice_contains_any(words: &[&str], expected: &[&str]) -> bool {
    expected.iter().any(|word| words.iter().any(|candidate| candidate == word))
}

pub(crate) fn is_enters_as_copy_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let as_copy_idx = primitives::words_find_phrase(tokens, &["as", "a", "copy"])
        .or_else(|| primitives::words_find_phrase(tokens, &["as", "an", "copy"]))
        .or_else(|| primitives::words_find_phrase(tokens, &["as", "copy"]));
    match as_copy_idx {
        Some(idx) => tokens[..idx]
            .iter()
            .any(|t| t.is_word("enter") || t.is_word("enters")),
        None => false,
    }
}

pub(crate) fn is_negated_untap_clause_words(words: &[&str]) -> bool {
    if words.len() < 3 {
        return false;
    }
    let has_untap = slice_contains_any(words, &["untap", "untaps"]);
    let has_negation = slice_contains_any(words, &["doesnt", "dont", "cant"])
        || contains_any_word_window(words, &[&["does", "not"], &["do", "not"], &["can", "not"]]);
    has_untap && has_negation
}

pub(crate) fn is_negated_untap_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let has_untap =
        primitives::contains_word(tokens, "untap") || primitives::contains_word(tokens, "untaps");
    let has_negation = primitives::contains_word(tokens, "doesnt")
        || primitives::contains_word(tokens, "dont")
        || primitives::contains_word(tokens, "cant")
        || primitives::words_find_phrase(tokens, &["does", "not"]).is_some()
        || primitives::words_find_phrase(tokens, &["do", "not"]).is_some()
        || primitives::words_find_phrase(tokens, &["can", "not"]).is_some();
    has_untap && has_negation
}

pub(crate) fn looks_like_supported_negated_untap_clause_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    let has_negated_untap = contains_any_word_window(
        words.as_slice(),
        &[&["dont", "untap", "during"], &["doesnt", "untap", "during"]],
    );
    let has_controllers_untap_step = contains_any_word_window(
        words.as_slice(),
        &[&["controllers", "untap", "step"], &["controllers", "untap", "steps"]],
    );
    let has_tapped_duration = contains_word_window(words.as_slice(), &["for", "as", "long", "as"])
        && word_slice_contains(words.as_slice(), "remains")
        && word_slice_contains(words.as_slice(), "tapped");
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
    primitives::words_find_phrase(tokens, &["one", "of", "them", "into", "your"]).is_some()
        && primitives::contains_word(tokens, "rest")
        && (primitives::contains_word(tokens, "graveyard")
            || primitives::contains_word(tokens, "graveyards"))
}

pub(crate) fn has_loses_all_abilities_with_becomes_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let has_loses_all_abilities = (primitives::contains_word(tokens, "lose")
        || primitives::contains_word(tokens, "loses"))
        && primitives::words_find_phrase(tokens, &["all", "abilities"]).is_some();
    has_loses_all_abilities && primitives::contains_word(tokens, "becomes")
}

pub(crate) fn has_spent_to_cast_this_spell_without_condition_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    let has_spent_to_cast_this_spell =
        primitives::words_find_phrase(tokens, &["was", "spent", "to", "cast", "this", "spell"])
            .is_some();
    has_spent_to_cast_this_spell
        && !primitives::contains_word(tokens, "if")
        && !primitives::contains_word(tokens, "unless")
}

pub(crate) fn has_would_enter_instead_replacement_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::contains_word(tokens, "would")
        && (primitives::contains_word(tokens, "enter") || primitives::contains_word(tokens, "enters"))
        && primitives::contains_word(tokens, "instead")
}

pub(crate) fn has_different_mana_value_constraint_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::words_find_phrase(tokens, &["different", "mana", "value"]).is_some()
}

pub(crate) fn has_most_common_color_constraint_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::words_find_phrase(tokens, &["most", "common", "color", "among", "all"]).is_some()
        && primitives::contains_word(tokens, "permanents")
}

pub(crate) fn has_power_vs_count_constraint_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "power")
        && primitives::words_find_phrase(
            tokens,
            &["less", "than", "or", "equal", "to", "the", "number", "of"],
        )
        .is_some()
}

pub(crate) fn has_put_into_graveyards_from_battlefield_this_turn_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::words_find_phrase(
        tokens,
        &[
            "put",
            "into",
            "graveyards",
            "from",
            "the",
            "battlefield",
            "this",
            "turn",
        ],
    )
    .is_some()
}

pub(crate) fn has_phase_out_until_leaves_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    (primitives::contains_word(tokens, "phase")
        || primitives::contains_word(tokens, "phases")
        || primitives::contains_word(tokens, "phased"))
        && primitives::contains_word(tokens, "until")
        && primitives::words_find_phrase(tokens, &["leaves", "the", "battlefield"]).is_some()
}

pub(crate) fn has_same_name_as_another_in_hand_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::words_find_phrase(tokens, &["same", "name", "as", "another", "card", "in"]).is_some()
        && primitives::contains_word(tokens, "hand")
}

pub(crate) fn has_for_each_mana_from_spent_to_cast_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::words_find_phrase(tokens, &["for", "each", "mana", "from"]).is_some()
        && primitives::contains_word(tokens, "spent")
        && primitives::words_find_phrase(tokens, &["cast", "this", "spell", "create"]).is_some()
}

pub(crate) fn has_when_you_sacrifice_this_way_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::words_find_phrase(tokens, &["when", "you", "sacrifice"]).is_some()
        && primitives::words_find_phrase(tokens, &["this", "way"]).is_some()
}

pub(crate) fn has_sacrifice_any_number_then_draw_that_many_clause_sentence_lexed(
    words: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    (primitives::contains_word(tokens, "sacrifice") || primitives::contains_word(tokens, "sacrifices"))
        && contains_word_window(words, &["any", "number", "of"])
        && (primitives::contains_word(tokens, "draw") || primitives::contains_word(tokens, "draws"))
        && contains_word_window(words, &["that", "many"])
}

pub(crate) fn has_greatest_mana_value_clause_sentence_lexed(words: &[&str]) -> bool {
    contains_word_window(words, &["greatest", "mana", "value"])
}

pub(crate) fn has_least_power_among_creatures_clause_sentence_lexed(words: &[&str]) -> bool {
    contains_word_window(words, &["least", "power", "among", "creatures"])
}

pub(crate) fn has_villainous_choice_clause_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    primitives::contains_word(tokens, "villainous") && primitives::contains_word(tokens, "choice")
}

pub(crate) fn has_divided_evenly_clause_sentence_lexed(words: &[&str]) -> bool {
    contains_word_window(words, &["divided", "evenly"])
}

pub(crate) fn has_different_names_clause_sentence_lexed(words: &[&str]) -> bool {
    contains_word_window(words, &["different", "names"])
}

pub(crate) fn has_chosen_at_random_clause_sentence_lexed(words: &[&str]) -> bool {
    contains_word_window(words, &["chosen", "at", "random"])
}

pub(crate) fn has_defending_players_choice_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::words_find_phrase(tokens, &["defending", "player's", "choice"]).is_some()
        || primitives::words_find_phrase(tokens, &["defending", "player", "choice"]).is_some()
        || primitives::words_find_phrase(tokens, &["player's", "choice", "target"]).is_some()
        || primitives::words_find_phrase(tokens, &["defending", "player", "s", "choice"]).is_some()
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
    contains_word_window(words, &["if", "you", "sacrifice", "an", "island"])
        && contains_word_window(words, &["this", "way"])
}

pub(crate) fn has_spent_to_cast_clause_sentence_lexed(words: &[&str]) -> bool {
    contains_word_window(words, &["spent", "to", "cast"])
}

pub(crate) fn has_face_down_clause_sentence_lexed(words: &[&str], tokens: &[OwnedLexToken]) -> bool {
    let has_face_down = contains_word_window(words, &["face", "down"])
        || words
            .iter()
            .any(|word| matches!(*word, "face-down" | "facedown"));
    if !has_face_down {
        return false;
    }

    let simple_exile_face_down = primitives::words_match_any_prefix(tokens, EXILE_PREFIXES).is_some()
        && !primitives::contains_word(tokens, "then")
        && !primitives::contains_word(tokens, "manifest")
        && !primitives::contains_word(tokens, "pile");
    !simple_exile_face_down
}

pub(crate) fn has_copy_spell_legendary_exception_clause_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> bool {
    primitives::contains_word(tokens, "copy")
        && primitives::contains_word(tokens, "spell")
        && primitives::contains_word(tokens, "legendary")
        && (primitives::contains_word(tokens, "except") || primitives::contains_word(tokens, "isnt"))
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
    let has_supported_control_duration =
        primitives::words_find_phrase(tokens, &["for", "as", "long", "as", "you", "control"])
            .is_some();
    let has_supported_source_tapped_duration =
        primitives::words_find_phrase(tokens, &["for", "as", "long", "as"]).is_some()
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
