use super::*;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use crate::runtime_backend::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern};

const ONE_OR_MORE_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["one", "or", "more"]);
const CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const AND_OR_CONNECTOR_WORDS: &[&str] = &["and", "or"];
const OTHER_OR_ANOTHER_WORDS: &[&str] = &["another", "other"];
const RELATIVE_PRONOUN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that"], &["which"], &["who"], &["whom"]]);
const EACH_WITH_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["each", "with"]);
const YOU_TRIGGER_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const ANOTHER_PLAYER_TRIGGER_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["another", "player"],
            &["a", "player", "other", "than", "you"],
            &["a", "player", "other", "than", "yourself"],
        ]
);
const CHOSEN_PLAYER_TRIGGER_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["the", "chosen", "player"], &["chosen", "player"]]);
const ENCHANTED_PLAYER_TRIGGER_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enchanted", "player"], &["the", "enchanted", "player"]]);
const EFFECT_CONTROLLER_TRIGGER_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "player", "who", "cast"],
            &["player", "who", "cast"]
        ]
);
const ANY_PLAYER_TRIGGER_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a", "player"],
            &["any", "player"],
            &["player"],
            &["one", "or", "more", "players"],
        ]
);
const OPPONENT_TRIGGER_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["an", "opponent"],
            &["opponent"],
            &["opponents"],
            &["your", "opponents"],
            &["one", "of", "your", "opponents"],
            &["one", "or", "more", "of", "your", "opponents"],
            &["one", "of", "the", "opponents"],
            &["one", "or", "more", "opponents"],
            &["each", "opponent"],
        ]
);
const OPPONENT_WORDS: &[&str] = &["opponent", "opponents"];
const ON_YOUR_TEAM_TRIGGER_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["on", "your", "team"]; contains_any_words & [&["player", "players"]]);
const ENCHANTED_PLAYER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["enchanted", "player"], &["enchanted", "players"]]]);
const CHOSEN_PLAYER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["chosen", "player"], &["chosen", "players"]]]);
const EACH_PLAYER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["each", "player"]]);
const YOUR_TEAM_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["your", "team"], &["on", "your", "team"]]]);
const YOU_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["you"]);
const SHUFFLE_CAUSED_BY_SPELL_OR_ABILITY_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["a", "spell", "or", "ability", "causes"]; suffix & ["to"]);
const ITS_CONTROLLER_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["its", "controller"]);
const SPELL_OR_ABILITY_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["a", "spell", "or", "ability"],
            &["spell", "or", "ability"]
        ]
);
const ANY_SOURCE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "source"], &["source"], &["any", "source"]]);
const TRIGGER_CONTROL_ACTION_PHRASES: &[&[&str]] = &[&["control"], &["controls"]];
const TRIGGER_CONTROL_ACTION_WORDS: &[&str] = &["control", "controls"];
const POWER_GREATER_THAN_BASE_POWER_PATTERN: ClauseShape<'static> = clause_shape!(contains_phrases & [&["power", "greater", "than", "its", "base", "power"]]; contains_any_words & [&["creature", "creatures"]]);
const SPELL_NOUN_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["spell", "spells"]]);
const SPELL_NOUN_EXACT_WORDS: &[&str] = &["spell", "spells"];
const FIRST_WORD: &str = "first";
const DURING_THEIR_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["during", "their", "turn"],
            &["during", "that", "players", "turn"]
        ]]
);
const DURING_YOUR_TURN_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["during", "your", "turn"]]);
const DURING_OPPONENT_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["during", "an", "opponents", "turn"],
            &["during", "an", "opponent's", "turn"],
            &["during", "an", "opponent", "s", "turn"],
            &["during", "opponents", "turn"],
            &["during", "opponent's", "turn"],
            &["during", "opponent", "s", "turn"],
            &["during", "each", "opponents", "turn"],
            &["during", "each", "opponent's", "turn"],
            &["during", "each", "opponent", "s", "turn"],
        ]]
);
const FIRST_SPELL_TURN_CONTEXT_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["each", "turn"],
            &["this", "turn"],
            &["of", "a", "turn"],
            &["during", "your", "turn"],
            &["during", "their", "turn"],
            &["during", "an", "opponents", "turn"],
            &["during", "an", "opponent's", "turn"],
            &["during", "an", "opponent", "s", "turn"],
            &["during", "opponents", "turn"],
            &["during", "opponent's", "turn"],
            &["during", "opponent", "s", "turn"],
            &["during", "each", "opponents", "turn"],
            &["during", "each", "opponent's", "turn"],
            &["during", "each", "opponent", "s", "turn"],
        ]]
);
const SECOND_SPELL_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["second", "spell", "cast", "this", "turn"],
            &["second", "spell", "this", "turn"],
            &["your", "second", "spell", "each", "turn"],
            &["their", "second", "spell", "each", "turn"],
            &["your", "second", "spell", "this", "turn"],
            &["their", "second", "spell", "this", "turn"],
            &["second", "spell", "each", "turn"],
            &["second", "spell", "during", "your", "turn"],
            &["second", "spell", "during", "their", "turn"],
            &["second", "spell", "during", "an", "opponents", "turn"],
            &["second", "spell", "during", "opponents", "turn"],
            &["second", "spell", "during", "each", "opponents", "turn"],
        ]]
);
const OTHER_THAN_FIRST_SPELL_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["other", "than", "your", "first", "spell"],
            &["other", "than", "the", "first", "spell"],
        ]]
);
const OTHER_THAN_FIRST_CASTS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(contains_phrases & [&["other", "than", "the", "first"]]; contains_words & ["spell", "casts", "turn"]);
const FROM_ANYWHERE_NOT_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["from", "anywhere", "other", "than", "your", "hand"],
            &["from", "anywhere", "other", "than", "their", "hand"],
            &["from", "anywhere", "other", "than", "hand"],
        ]]
);
const FROM_ANYWHERE_OTHER_THAN_WORDS: &[&str] = &["from", "anywhere", "other", "than"];
const FROM_ANYWHERE_OTHER_THAN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & FROM_ANYWHERE_OTHER_THAN_WORDS);
const UNQUALIFIED_SPELL_WORDS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "spell"], &["spells"], &["spell"]]);
const SPELL_OR_SPELLS_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["spell"], &["spells"]]);
const SPELL_AUXILIARY_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["is"], &["are"], &["was"], &["were"], &["be"], &["been"]]);
const FILTER_TRUNCATION_WORDS: &[&str] = &["during", "other"];
const FROM_ANYWHERE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["from", "anywhere"]);
const CHOSEN_COLOR_SPELL_QUALIFIER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["of", "the", "chosen", "color"],
            &["of", "chosen", "color"]
        ]
);
const SPELL_ORIGIN_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["graveyard"]);
const SPELL_ORIGIN_EXILE_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["exile"]);
const SPELL_ORIGIN_HAND_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["hand"]);
const YOUR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["your"]);
const OPPONENT_OR_THEIR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["opponent", "their"]]);
const CAST_OR_COPY_SEPARATOR_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const CAST_OR_CASTS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["cast"], &["casts"]]);
const COPY_OR_COPIES_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["copy"], &["copies"]]);
const HAND_EXACT_WORD: &str = "hand";
const FROM_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["from"]);
const ROUND_UP_EACH_TIME_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["round", "up", "each", "time"]);
const IF_YOU_DO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["if", "you", "do"]);
const EXILED_CARDS_OWNER_MAY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "exiled", "cards", "owner", "may"]);
const IT_REFERENCE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["it"]);
const THAT_CARD_REFERENCE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "card"]);
const EXILED_CARD_REFERENCE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "exiled", "card"]);
const REVEALED_CARD_REFERENCE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["the", "revealed", "card"], &["that", "revealed", "card"]]);
const COPY_REFERENCE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["the", "copy"], &["that", "copy"], &["a", "copy"]]);
const WITHOUT_PAYING_MANA_COST_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["without", "paying", "its", "mana", "cost"]);
const WITHOUT_PAYING_MANA_COST_MV_LTE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "without", "paying", "its", "mana", "cost", "if", "its", "a", "spell", "with", "mana",
            "value", "less", "than", "or", "equal", "to",
        ]
);
const COPY_COSTS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["that", "copy", "costs"],
            &["the", "copy", "costs"],
            &["a", "copy", "costs"]
        ]
);
const LESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["less"]);
const COSTS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["costs"]);
const LESS_TO_CAST_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["less", "to", "cast"]);
const YOU_MAY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you", "may"]);
const TOKEN_MANA_REMINDER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["they", "have"],
            &["it", "has"],
            &["this", "token", "has"],
            &["those", "tokens", "have"],
        ]
);
const TOKEN_MANA_REMINDER_WORDS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["sacrifice", "add", "c"]);
const TOKEN_HAS_ABILITY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["it", "has"], &["they", "have"]]);
const TOKEN_PRONOUN_TRIGGER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["when", "it"],
            &["whenever", "it"],
            &["when", "they"],
            &["whenever", "they"],
        ]
);
const TOKEN_PT_REMINDER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["its", "power"],
            &["its", "power", "and", "toughness"],
            &["its", "toughness"],
        ]
);
const TOKEN_DELAYED_LIFECYCLE_ACTION_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["exile"], &["sacrifice"]]);
const TOKEN_REMINDER_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["token", "tokens", "it", "them"]]);
const TOKEN_REMINDER_REFERENCE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["when", "this", "token"],
            &["whenever", "this", "token"],
            &["this", "token"],
            &["those", "tokens"],
        ]
);
const HASTE_REMINDER_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["haste"]);
const TOKEN_REMINDER_EXILE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["exile"]);
const TOKEN_REMINDER_SACRIFICE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["sacrifice"]);
const EXILE_TOKEN_LIFECYCLE_ACTION_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["exile"], &["exiles"]]);
const SACRIFICE_TOKEN_LIFECYCLE_ACTION_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["sacrifice"], &["sacrifices"]]);
const LEAVES_BATTLEFIELD_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["leaves", "the", "battlefield"]);
const CREATED_TOKEN_REFERENCE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that", "token"], &["those", "tokens"], &["them"], &["it"]]);
const THAT_TOKEN_LEAVES_BATTLEFIELD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["that", "token", "leaves", "the", "battlefield"]);
const WHEN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["when"]);
const CREATE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["create"]);
const TOKEN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["token"]);
const WITH_WORD: &str = "with";
const T_WORD: &str = "t";
const AND_WORD: &str = "and";
const TOKEN_PT_EQUAL_PAIR_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "its",
            "power",
            "and",
            "toughness",
            "are",
            "each",
            "equal",
            "to",
        ]
);
const ATTACHED_CONTROLLER_OBJECT_WORDS: &[&str] = &[
    "creature",
    "creatures",
    "permanent",
    "permanents",
    "artifact",
    "artifacts",
    "enchantment",
    "enchantments",
    "land",
    "lands",
];
const CONTROLLER_WORD: &str = "controller";

fn trigger_subject_shape_matches_words<'a>(words: &[&str], shape: ClauseShape<'a>) -> bool {
    shape.matches_word_slice(words)
}

fn trigger_subject_word_is_any(word: &str, expected: &[&str]) -> bool {
    expected.contains(&word)
}

fn trigger_subject_word_is(word: &str, expected: &str) -> bool {
    trigger_subject_word_is_any(word, &[expected])
}

fn trigger_subject_token_word_is(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|word| trigger_subject_word_is(word, expected))
}

fn find_words_matching_shape(
    words: &[&str],
    phrase_len: usize,
    shape: ClauseShape<'static>,
) -> Option<usize> {
    words
        .windows(phrase_len)
        .position(|window| trigger_subject_shape_matches_words(window, shape))
}

fn trigger_control_tail_words(words: &[&str]) -> Option<PlayerFilter> {
    let atoms = [
        LexPattern::subject(
            "controller",
            LexCaptureKind::UntilAnyPhrase(TRIGGER_CONTROL_ACTION_PHRASES),
        ),
        LexPattern::action(
            "control_action",
            LexCaptureKind::OneOf(TRIGGER_CONTROL_ACTION_WORDS),
        ),
    ];
    let matched = LexPattern::new(&atoms).match_word_refs(words)?;
    let controller_range = matched
        .capture_by_role(LexCaptureRole::Subject)?
        .word_range
        .clone();
    trigger_control_controller_words(&words[controller_range])
}

fn trigger_control_controller_words(words: &[&str]) -> Option<PlayerFilter> {
    parse_trigger_subject_player_filter(words)
}

fn trigger_subject_control_suffix(words: &[&str]) -> Option<(PlayerFilter, usize)> {
    for suffix_len in (2..=3).rev() {
        if words.len() < suffix_len {
            continue;
        }
        let tail = &words[words.len() - suffix_len..];
        if let Some(controller) = trigger_control_tail_words(tail) {
            return Some((controller, suffix_len));
        }
    }
    None
}

fn trigger_subject_control_phrase(words: &[&str]) -> Option<(usize, usize, PlayerFilter)> {
    for idx in 0..words.len() {
        for phrase_len in (2..=3).rev() {
            if idx + phrase_len > words.len() {
                continue;
            }
            if let Some(controller) = trigger_control_tail_words(&words[idx..idx + phrase_len]) {
                return Some((idx, phrase_len, controller));
            }
        }
    }
    None
}

fn trigger_source_words(words: &[&str]) -> bool {
    matches!(words, ["a", "source"] | ["source"])
}

fn find_trigger_subject_token_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    find_index(tokens, |token| {
        trigger_subject_token_word_is(token, expected)
    })
}

fn find_token_shape(tokens: &[OwnedLexToken], shape: &ClauseShape<'static>) -> Option<usize> {
    find_index(tokens, |token| {
        shape.matches(crate::runtime_backend::lexer::LexedClause::new(
            std::slice::from_ref(token),
        ))
    })
}

pub(crate) fn parse_discard_trigger_card_filter(
    after_discard_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let remainder = trim_commas(after_discard_tokens);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let remainder_words = crate::runtime_backend::token_word_refs(&remainder);
    let Some(card_word_idx) = find_index(&remainder_words, |word| {
        trigger_subject_word_is_any(word, CARD_OR_CARDS_WORDS)
    }) else {
        return Err(CardTextError::ParseError(format!(
            "missing discard trigger card keyword (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    let qualifier_end =
        token_index_for_word_index(&remainder, card_word_idx).unwrap_or(remainder.len());
    let qualifier_tokens = trim_commas(&remainder[..qualifier_end]);
    let mut qualifier_tokens = strip_leading_articles(&qualifier_tokens);
    if qualifier_tokens.len() >= 2
        && qualifier_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .and_then(parse_cardinal_u32)
            .is_some()
        && qualifier_tokens
            .get(1)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| trigger_subject_word_is_any(word, AND_OR_CONNECTOR_WORDS))
    {
        qualifier_tokens = qualifier_tokens[2..].to_vec();
    } else if qualifier_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .and_then(parse_cardinal_u32)
        .is_some()
    {
        qualifier_tokens = qualifier_tokens[1..].to_vec();
    }

    let trailing_tokens = if card_word_idx + 1 < remainder_words.len() {
        let trailing_start =
            token_index_for_word_index(&remainder, card_word_idx + 1).unwrap_or(remainder.len());
        trim_commas(&remainder[trailing_start..])
    } else {
        Vec::new()
    };
    if !trailing_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing discard trigger clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if qualifier_tokens.is_empty() {
        return Ok(None);
    }

    let qualifier_words = crate::runtime_backend::token_word_refs(&qualifier_tokens);
    if trigger_subject_shape_matches_words(&qualifier_words, ONE_OR_MORE_PATTERN) {
        return Ok(None);
    }

    if let Ok(filter) = parse_object_filter(&qualifier_tokens, false) {
        return Ok(Some(filter));
    }

    let mut fallback = ObjectFilter::default();
    let mut parsed_any = false;
    for word in qualifier_words {
        if trigger_subject_word_is_any(word, AND_OR_CONNECTOR_WORDS) {
            continue;
        }
        if let Some(non_type) = parse_non_type(word) {
            if !slice_contains(&fallback.excluded_card_types, &non_type) {
                fallback.excluded_card_types.push(non_type);
            }
            parsed_any = true;
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            if !slice_contains(&fallback.card_types, &card_type) {
                fallback.card_types.push(card_type);
            }
            parsed_any = true;
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if parsed_any {
        Ok(Some(fallback))
    } else {
        Err(CardTextError::ParseError(format!(
            "unsupported discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )))
    }
}

pub(crate) fn parse_subtype_list_enters_trigger_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.is_empty() {
        return None;
    }

    let (controller, subject_end) = subtype_list_controller_suffix(&words);

    let mut subtypes = Vec::new();
    for word in &words[..subject_end] {
        if trigger_subject_word_is_any(word, AND_OR_CONNECTOR_WORDS) {
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            if !slice_contains(&subtypes, &subtype) {
                subtypes.push(subtype);
            }
        }
    }
    if subtypes.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.subtypes = subtypes;
    filter.controller = controller;
    filter.other = other;
    Some(filter)
}

fn subtype_list_controller_suffix(words: &[&str]) -> (Option<PlayerFilter>, usize) {
    if let Some((controller, suffix_len)) = trigger_subject_control_suffix(words) {
        (Some(controller), words.len().saturating_sub(suffix_len))
    } else {
        (None, words.len())
    }
}

pub(crate) fn parse_possessive_clause_player_filter(words: &[&str]) -> PlayerFilter {
    let attached_controller_filter =
        |tag: &str| PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(tag)));
    let normalized_words = words
        .iter()
        .map(|word| {
            str_strip_suffix(word, "'s")
                .or_else(|| str_strip_suffix(word, "’s"))
                .or_else(|| str_strip_suffix(word, "s'"))
                .or_else(|| str_strip_suffix(word, "s’"))
                .unwrap_or(word)
        })
        .collect::<Vec<_>>();
    let has_attached_controller = |subject: &str| {
        find_window_by(&normalized_words, 3, |window| {
            window[0] == subject
                && trigger_subject_word_is_any(window[1], ATTACHED_CONTROLLER_OBJECT_WORDS)
                && trigger_subject_word_is(window[2], CONTROLLER_WORD)
        })
        .is_some()
    };

    if trigger_subject_shape_matches_words(&normalized_words, ENCHANTED_PLAYER_WORD_PATTERN) {
        return PlayerFilter::TaggedPlayer(TagKey::from("enchanted"));
    }
    if has_attached_controller("enchanted") {
        return attached_controller_filter("enchanted");
    }
    if has_attached_controller("equipped") {
        return attached_controller_filter("equipped");
    }

    // "each player" / "a player" / "that player" should resolve to Any,
    // even if "opponent" appears elsewhere in the clause text.  Check for
    // explicit "each/a/that player" before falling through to the opponent
    // keyword scan.
    if trigger_subject_shape_matches_words(&normalized_words, EACH_PLAYER_WORD_PATTERN) {
        PlayerFilter::Any
    } else if contains_your_team_words(words)
        || trigger_subject_shape_matches_words(words, YOUR_WORD_PATTERN)
    {
        PlayerFilter::You
    } else if contains_opponent_word(words) {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

pub(crate) fn parse_subject_clause_player_filter(words: &[&str]) -> PlayerFilter {
    if contains_your_team_words(words)
        || trigger_subject_shape_matches_words(words, YOU_WORD_PATTERN)
    {
        PlayerFilter::You
    } else if trigger_subject_shape_matches_words(words, ENCHANTED_PLAYER_WORD_PATTERN) {
        PlayerFilter::TaggedPlayer(TagKey::from("enchanted"))
    } else if trigger_subject_shape_matches_words(words, CHOSEN_PLAYER_WORD_PATTERN) {
        PlayerFilter::ChosenPlayer
    } else if contains_opponent_word(words) {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

pub(crate) fn contains_opponent_word(words: &[&str]) -> bool {
    words
        .iter()
        .any(|word| trigger_subject_word_is_any(word, OPPONENT_WORDS))
}

pub(crate) fn contains_your_team_words(words: &[&str]) -> bool {
    trigger_subject_shape_matches_words(words, YOUR_TEAM_WORD_PATTERN)
}

pub(crate) fn parse_trigger_subject_player_filter(subject: &[&str]) -> Option<PlayerFilter> {
    if trigger_subject_shape_matches_words(subject, YOU_TRIGGER_SUBJECT_PATTERN) {
        return Some(PlayerFilter::You);
    }
    if trigger_subject_shape_matches_words(subject, ANOTHER_PLAYER_TRIGGER_SUBJECT_PATTERN) {
        return Some(PlayerFilter::NotYou);
    }
    if trigger_subject_shape_matches_words(subject, CHOSEN_PLAYER_TRIGGER_SUBJECT_PATTERN) {
        return Some(PlayerFilter::ChosenPlayer);
    }
    if trigger_subject_shape_matches_words(subject, ENCHANTED_PLAYER_TRIGGER_SUBJECT_PATTERN) {
        return Some(PlayerFilter::TaggedPlayer(crate::tag::TagKey::from(
            "enchanted",
        )));
    }
    if trigger_subject_shape_matches_words(subject, EFFECT_CONTROLLER_TRIGGER_SUBJECT_PATTERN) {
        return Some(PlayerFilter::EffectController);
    }
    if trigger_subject_shape_matches_words(subject, ANY_PLAYER_TRIGGER_SUBJECT_PATTERN) {
        return Some(PlayerFilter::Any);
    }
    if trigger_subject_shape_matches_words(subject, OPPONENT_TRIGGER_SUBJECT_PATTERN) {
        return Some(PlayerFilter::Opponent);
    }
    if trigger_subject_shape_matches_words(subject, ON_YOUR_TEAM_TRIGGER_SUBJECT_PATTERN) {
        return Some(PlayerFilter::You);
    }
    None
}

pub(crate) fn split_target_clause_before_comma(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let tokens = trim_commas(tokens);
    if let Some(comma_idx) = find_index(&tokens, |token| token.is_comma()) {
        trim_commas(&tokens[..comma_idx])
    } else {
        tokens
    }
}

pub(crate) fn parse_shuffle_trigger_subject(
    subject: &[&str],
) -> Option<(PlayerFilter, bool, bool)> {
    if let Some(player) = parse_trigger_subject_player_filter(subject) {
        return Some((player, false, false));
    }

    if !(trigger_subject_shape_matches_words(subject, SHUFFLE_CAUSED_BY_SPELL_OR_ABILITY_PATTERN)
        && subject.len() > 6)
    {
        return None;
    }

    let caused_player_words = &subject[5..subject.len() - 1];
    if trigger_subject_shape_matches_words(caused_player_words, ITS_CONTROLLER_PATTERN) {
        return Some((PlayerFilter::Any, true, true));
    }

    parse_trigger_subject_player_filter(caused_player_words).map(|player| (player, true, false))
}

pub(crate) fn parse_spell_or_ability_controller_tail(words: &[&str]) -> Option<PlayerFilter> {
    let prefix_len = if trigger_subject_shape_matches_words(
        words,
        ClauseShape::new().prefix(&["a", "spell", "or", "ability"]),
    ) {
        4usize
    } else if trigger_subject_shape_matches_words(
        words,
        ClauseShape::new().prefix(&["spell", "or", "ability"]),
    ) {
        3usize
    } else {
        return None;
    };
    trigger_control_tail_words(&words[prefix_len..])
}

pub(crate) fn parse_trigger_subject_filter(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut subject_tokens = strip_leading_one_or_more(subject_tokens);
    let mut other = false;
    if subject_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| trigger_subject_word_is_any(word, OTHER_OR_ANOTHER_WORDS))
    {
        other = true;
        subject_tokens = &subject_tokens[1..];
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = crate::runtime_backend::token_word_refs(subject_tokens);
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if let Some((controller, suffix_len)) = trigger_subject_control_suffix(&subject_words)
        && trigger_source_words(&subject_words[..subject_words.len().saturating_sub(suffix_len)])
    {
        let mut filter = ObjectFilter::default();
        filter.controller = Some(controller);
        return Ok(Some(filter));
    }
    if trigger_subject_shape_matches_words(&subject_words, ANY_SOURCE_PATTERN) {
        return Ok(Some(ObjectFilter::default()));
    }
    if subject_words
        .iter()
        .any(|word| trigger_subject_shape_matches_words(&[*word], RELATIVE_PRONOUN_PATTERN))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trigger subject filter (clause: '{}')",
            subject_words.join(" ")
        )));
    }

    parse_object_filter(subject_tokens, other)
        .map(Some)
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported trigger subject filter (clause: '{}')",
                crate::runtime_backend::token_word_refs(subject_tokens).join(" ")
            ))
        })
}

pub(crate) fn trigger_subject_player_selector(
    subject_tokens: &[OwnedLexToken],
) -> Option<PlayerFilter> {
    let subject_tokens = strip_leading_one_or_more(subject_tokens);
    let subject_words = crate::runtime_backend::token_word_refs(subject_tokens);
    parse_trigger_subject_player_filter(&subject_words)
}

pub(crate) fn attacking_filter_for_player(player: PlayerFilter) -> ObjectFilter {
    let mut filter = ObjectFilter::creature();
    if !matches!(player, PlayerFilter::Any) {
        filter.controller = Some(player);
    }
    filter
}

pub(crate) fn parse_attack_trigger_subject_filter(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if let Some(player) = trigger_subject_player_selector(subject_tokens) {
        return Ok(Some(attacking_filter_for_player(player)));
    }
    let Some(mut filter) = parse_trigger_subject_filter(subject_tokens)? else {
        return Ok(None);
    };

    // Attack/combat-trigger subjects are creatures by default even when
    // expressed only as a subtype ("a Sliver", "one or more Goblins", etc.).
    if filter.card_types.is_empty() {
        filter.card_types.push(crate::types::CardType::Creature);
    } else if filter.card_types.len() > 1 && filter.all_card_types.is_empty() {
        filter.all_card_types = std::mem::take(&mut filter.card_types);
    }

    Ok(Some(filter))
}

pub(crate) fn strip_leading_one_or_more_lexed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if let Some(used) = leading_one_or_more_prefix_len(tokens) {
        &tokens[used..]
    } else {
        tokens
    }
}

pub(crate) fn parse_subtype_list_enters_trigger_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let words = ActivationRestrictionCompatWords::new(tokens);
    let words = words.to_word_refs();
    if words.is_empty() {
        return None;
    }

    let (controller, subject_end) = subtype_list_controller_suffix(&words);

    let mut subtypes = Vec::new();
    for word in &words[..subject_end] {
        if trigger_subject_word_is_any(word, AND_OR_CONNECTOR_WORDS) {
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            if !slice_contains(&subtypes, &subtype) {
                subtypes.push(subtype);
            }
        }
    }
    if subtypes.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.subtypes = subtypes;
    filter.controller = controller;
    filter.other = other;
    Some(filter)
}

pub(crate) fn parse_trigger_subject_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
    let mut other = false;
    if subject_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| trigger_subject_word_is_any(word, OTHER_OR_ANOTHER_WORDS))
    {
        other = true;
        subject_tokens = &subject_tokens[1..];
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if subject_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "target")
    {
        subject_tokens = &subject_tokens[1..];
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = subject_words.to_word_refs();
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if let Some((controller, suffix_len)) = trigger_subject_control_suffix(&subject_words)
        && trigger_source_words(&subject_words[..subject_words.len().saturating_sub(suffix_len)])
    {
        let mut filter = ObjectFilter::default();
        filter.controller = Some(controller);
        return Ok(Some(filter));
    }
    if trigger_subject_shape_matches_words(&subject_words, ANY_SOURCE_PATTERN) {
        return Ok(Some(ObjectFilter::default()));
    }
    if subject_words
        .iter()
        .any(|word| trigger_subject_shape_matches_words(&[*word], RELATIVE_PRONOUN_PATTERN))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trigger subject filter (clause: '{}')",
            subject_words.join(" ")
        )));
    }

    if trigger_subject_shape_matches_words(&subject_words, POWER_GREATER_THAN_BASE_POWER_PATTERN) {
        let mut filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
        filter.power_greater_than_base_power = true;
        if other {
            filter.other = true;
        }
        if let Some((_, _, controller)) = trigger_subject_control_phrase(&subject_words) {
            filter.controller = Some(controller);
        }
        return Ok(Some(filter));
    }

    let mut normalized_subject_tokens = subject_tokens.to_vec();
    if find_window_by(&normalized_subject_tokens, 2, |window| {
        let words = ActivationRestrictionCompatWords::new(window).to_word_refs();
        trigger_subject_shape_matches_words(&words, EACH_WITH_PATTERN)
    })
    .is_some()
    {
        let mut normalized = Vec::with_capacity(normalized_subject_tokens.len());
        let mut idx = 0usize;
        while idx < normalized_subject_tokens.len() {
            let next_two_words = normalized_subject_tokens
                .get(idx..idx + 2)
                .map(ActivationRestrictionCompatWords::new)
                .map(|view| view.to_word_refs())
                .unwrap_or_default();
            if trigger_subject_shape_matches_words(&next_two_words, EACH_WITH_PATTERN) {
                idx += 1;
                continue;
            }
            normalized.push(normalized_subject_tokens[idx].clone());
            idx += 1;
        }
        normalized_subject_tokens = normalized;
    }

    let mut controller_override = None;
    let word_view = ActivationRestrictionCompatWords::new(&normalized_subject_tokens);
    let normalized_words = word_view.to_word_refs();
    let controller_phrase = if let Some((idx, len, controller)) =
        trigger_subject_control_phrase(&normalized_words)
            .filter(|(idx, len, _)| idx.saturating_add(*len) < normalized_words.len())
    {
        controller_override = Some(controller);
        Some((idx, len))
    } else {
        None
    };

    if let Some((word_idx, len)) = controller_phrase
        && let Some(start) = token_index_for_word_index(&normalized_subject_tokens, word_idx)
        && let Some(end) = token_index_for_word_index(&normalized_subject_tokens, word_idx + len)
    {
        normalized_subject_tokens.drain(start..end);
    }

    parse_object_filter_lexed(&normalized_subject_tokens, other)
        .map(|mut filter| {
            if filter.zone.is_none()
                && filter.tagged_constraints.is_empty()
                && filter.specific.is_none()
                && !filter.source
            {
                filter.zone = Some(Zone::Battlefield);
            }
            if let Some(controller) = controller_override {
                filter.controller = Some(controller);
                filter.zone.get_or_insert(Zone::Battlefield);
            }
            Some(filter)
        })
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported trigger subject filter (clause: '{}')",
                subject_words.join(" ")
            ))
        })
}

pub(crate) fn trigger_subject_player_selector_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Option<PlayerFilter> {
    let subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
    let subject_words = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = subject_words.to_word_refs();
    parse_trigger_subject_player_filter(&subject_words)
}

pub(crate) fn parse_attack_trigger_subject_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if let Some(player) = trigger_subject_player_selector_lexed(subject_tokens) {
        return Ok(Some(attacking_filter_for_player(player)));
    }
    let Some(mut filter) = parse_trigger_subject_filter_lexed(subject_tokens)? else {
        return Ok(None);
    };

    if filter.card_types.is_empty() {
        filter.card_types.push(crate::types::CardType::Creature);
    } else if filter.card_types.len() > 1 && filter.all_card_types.is_empty() {
        filter.all_card_types = std::mem::take(&mut filter.card_types);
    }

    Ok(Some(filter))
}

pub(crate) fn parse_exact_spell_count_each_turn(words: &[&str]) -> Option<u32> {
    for (ordinal, count) in [
        ("third", 3u32),
        ("fourth", 4u32),
        ("fifth", 5u32),
        ("sixth", 6u32),
        ("seventh", 7u32),
        ("eighth", 8u32),
        ("ninth", 9u32),
        ("tenth", 10u32),
    ] {
        let patterns: &[&[&str]] = &[
            &[ordinal, "spell", "cast", "this", "turn"],
            &[ordinal, "spell", "this", "turn"],
            &["your", ordinal, "spell", "each", "turn"],
            &["their", ordinal, "spell", "each", "turn"],
            &["your", ordinal, "spell", "this", "turn"],
            &["their", ordinal, "spell", "this", "turn"],
            &[ordinal, "spell", "each", "turn"],
        ];
        if trigger_subject_shape_matches_words(
            words,
            ClauseShape::new().contains_any_phrases(&[patterns]),
        ) {
            return Some(count);
        }
    }
    None
}

pub(crate) fn parse_exact_draw_count_each_turn(words: &[&str]) -> Option<u32> {
    for pattern in [
        &[
            "a", "card", "except", "the", "first", "one", "they", "draw", "in", "each", "of",
            "their", "draw", "steps",
        ][..],
        &[
            "a", "card", "except", "the", "first", "card", "they", "draw", "in", "each", "of",
            "their", "draw", "steps",
        ],
        &[
            "a", "card", "except", "the", "first", "one", "you", "draw", "in", "each", "of",
            "your", "draw", "steps",
        ],
        &[
            "a", "card", "except", "the", "first", "card", "you", "draw", "in", "each", "of",
            "your", "draw", "steps",
        ],
        &[
            "a", "card", "except", "the", "first", "one", "they", "draw", "in", "their", "draw",
            "step",
        ],
        &[
            "a", "card", "except", "the", "first", "card", "they", "draw", "in", "their", "draw",
            "step",
        ],
        &[
            "a", "card", "except", "the", "first", "one", "you", "draw", "in", "your", "draw",
            "step",
        ],
        &[
            "a", "card", "except", "the", "first", "card", "you", "draw", "in", "your", "draw",
            "step",
        ],
    ] {
        if trigger_subject_shape_matches_words(
            words,
            ClauseShape::new().contains_phrases(&[pattern]),
        ) {
            return Some(2);
        }
    }

    for (ordinal, count) in [
        ("second", 2u32),
        ("third", 3u32),
        ("fourth", 4u32),
        ("fifth", 5u32),
        ("sixth", 6u32),
        ("seventh", 7u32),
        ("eighth", 8u32),
        ("ninth", 9u32),
        ("tenth", 10u32),
    ] {
        let patterns: &[&[&str]] = &[
            &[ordinal, "card", "each", "turn"],
            &[ordinal, "cards", "each", "turn"],
            &["your", ordinal, "card", "each", "turn"],
            &["your", ordinal, "cards", "each", "turn"],
            &["their", ordinal, "card", "each", "turn"],
            &["their", ordinal, "cards", "each", "turn"],
            &[ordinal, "card", "this", "turn"],
            &[ordinal, "cards", "this", "turn"],
            &["your", ordinal, "card", "this", "turn"],
            &["your", ordinal, "cards", "this", "turn"],
            &["their", ordinal, "card", "this", "turn"],
            &["their", ordinal, "cards", "this", "turn"],
        ];
        if trigger_subject_shape_matches_words(
            words,
            ClauseShape::new().contains_any_phrases(&[patterns]),
        ) {
            return Some(count);
        }
    }
    None
}

pub(crate) fn has_draw_except_first_in_draw_step_pattern(words: &[&str]) -> bool {
    let patterns: &[&[&str]] = &[
        &[
            "a", "card", "except", "the", "first", "one", "they", "draw", "in", "each", "of",
            "their", "draw", "steps",
        ],
        &[
            "a", "card", "except", "the", "first", "card", "they", "draw", "in", "each", "of",
            "their", "draw", "steps",
        ],
        &[
            "a", "card", "except", "the", "first", "one", "you", "draw", "in", "each", "of",
            "your", "draw", "steps",
        ],
        &[
            "a", "card", "except", "the", "first", "card", "you", "draw", "in", "each", "of",
            "your", "draw", "steps",
        ],
        &[
            "a", "card", "except", "the", "first", "one", "they", "draw", "in", "their", "draw",
            "step",
        ],
        &[
            "a", "card", "except", "the", "first", "card", "they", "draw", "in", "their", "draw",
            "step",
        ],
        &[
            "a", "card", "except", "the", "first", "one", "you", "draw", "in", "your", "draw",
            "step",
        ],
        &[
            "a", "card", "except", "the", "first", "card", "you", "draw", "in", "your", "draw",
            "step",
        ],
    ];
    trigger_subject_shape_matches_words(words, ClauseShape::new().contains_any_phrases(&[patterns]))
}

pub(crate) fn has_first_spell_each_turn_pattern(words: &[&str]) -> bool {
    if !trigger_subject_shape_matches_words(words, FIRST_SPELL_TURN_CONTEXT_PATTERN) {
        return false;
    }

    for (idx, word) in words.iter().enumerate() {
        if !trigger_subject_word_is(word, FIRST_WORD) {
            continue;
        }
        let window_end = (idx + 5).min(words.len());
        if words[idx + 1..window_end]
            .iter()
            .any(|candidate| trigger_subject_word_is_any(candidate, SPELL_NOUN_EXACT_WORDS))
        {
            return true;
        }
    }
    false
}

fn trim_trailing_spell_auxiliary_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut prefix_tokens = tokens;
    while let Some(last_word) = prefix_tokens.last().and_then(OwnedLexToken::as_word) {
        if trigger_subject_shape_matches_words(&[last_word], SPELL_AUXILIARY_WORD_PATTERN) {
            prefix_tokens = &prefix_tokens[..prefix_tokens.len() - 1];
        } else {
            break;
        }
    }
    prefix_tokens
}

fn token_slice_has_spell_noun(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    trigger_subject_shape_matches_words(&words, SPELL_NOUN_PATTERN)
}

pub(crate) fn has_second_spell_turn_pattern(words: &[&str]) -> bool {
    trigger_subject_shape_matches_words(words, SECOND_SPELL_TURN_PATTERN)
}

pub(crate) fn parse_spell_activity_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !trigger_subject_shape_matches_words(&clause_words, SPELL_NOUN_PATTERN) {
        return Ok(None);
    }

    let cast_idx = find_token_shape(tokens, &CAST_OR_CASTS_PATTERN);
    let copy_idx = find_token_shape(tokens, &COPY_OR_COPIES_PATTERN);
    if cast_idx.is_none() && copy_idx.is_none() {
        return Ok(None);
    }

    let mut actor = parse_subject_clause_player_filter(&clause_words);
    let during_their_turn =
        trigger_subject_shape_matches_words(&clause_words, DURING_THEIR_TURN_PATTERN);
    let mut during_turn =
        if trigger_subject_shape_matches_words(&clause_words, DURING_YOUR_TURN_PATTERN) {
            Some(PlayerFilter::You)
        } else if trigger_subject_shape_matches_words(&clause_words, DURING_OPPONENT_TURN_PATTERN) {
            Some(PlayerFilter::Opponent)
        } else {
            None
        };
    if during_their_turn {
        if matches!(actor, PlayerFilter::Any) {
            actor = PlayerFilter::Active;
            during_turn = None;
        } else if during_turn.is_none() {
            during_turn = Some(actor.clone());
        }
    }
    let has_other_than_first_spell_pattern =
        trigger_subject_shape_matches_words(&clause_words, OTHER_THAN_FIRST_SPELL_PATTERN)
            || trigger_subject_shape_matches_words(
                &clause_words,
                OTHER_THAN_FIRST_CASTS_TURN_PATTERN,
            );
    let second_spell_turn_pattern = has_second_spell_turn_pattern(&clause_words);
    let first_spell_each_turn =
        !has_other_than_first_spell_pattern && has_first_spell_each_turn_pattern(&clause_words);
    let exact_spells_this_turn = parse_exact_spell_count_each_turn(&clause_words)
        .or_else(|| first_spell_each_turn.then_some(1))
        .or_else(|| {
            (!has_other_than_first_spell_pattern && second_spell_turn_pattern).then_some(2)
        });
    let min_spells_this_turn = if exact_spells_this_turn.is_some() {
        None
    } else if has_other_than_first_spell_pattern {
        Some(2)
    } else {
        None
    };
    let from_not_hand =
        trigger_subject_shape_matches_words(&clause_words, FROM_ANYWHERE_NOT_HAND_PATTERN)
            || find_words_matching_shape(
                &clause_words,
                FROM_ANYWHERE_OTHER_THAN_WORDS.len(),
                FROM_ANYWHERE_OTHER_THAN_PATTERN,
            )
            .is_some_and(|idx| {
                clause_words[idx + 4..]
                    .iter()
                    .take(4)
                    .any(|word| trigger_subject_word_is(word, HAND_EXACT_WORD))
            });

    let parse_filter =
        |filter_tokens: &[OwnedLexToken]| -> Result<Option<ObjectFilter>, CardTextError> {
            let filter_tokens = if let Some(idx) =
                find_index(filter_tokens, |token| token.is_comma() || token.is_period())
            {
                &filter_tokens[..idx]
            } else {
                filter_tokens
            };
            let filter_tokens = if let Some(idx) = find_index(filter_tokens, |token| {
                token
                    .as_word()
                    .is_some_and(|word| trigger_subject_word_is_any(word, FILTER_TRUNCATION_WORDS))
            }) {
                &filter_tokens[..idx]
            } else {
                filter_tokens
            };
            let filter_tokens = if let Some(idx) =
                find_token_shape(filter_tokens, &FROM_WORD_PATTERN).filter(|idx| {
                    let tail_words =
                        crate::runtime_backend::token_word_refs(&filter_tokens[*idx..]);
                    trigger_subject_shape_matches_words(&tail_words, FROM_ANYWHERE_PREFIX_PATTERN)
                }) {
                &filter_tokens[..idx]
            } else {
                filter_tokens
            };
            let filter_words: Vec<&str> = filter_tokens
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .collect();
            let is_unqualified_spell =
                trigger_subject_shape_matches_words(&filter_words, UNQUALIFIED_SPELL_WORDS_PATTERN);
            if filter_tokens.is_empty() || is_unqualified_spell {
                Ok(None)
            } else {
                let parse_spell_origin_zone_filter = || -> Option<ObjectFilter> {
                    let zone = if trigger_subject_shape_matches_words(
                        &filter_words,
                        SPELL_ORIGIN_GRAVEYARD_PATTERN,
                    ) {
                        Some(Zone::Graveyard)
                    } else if trigger_subject_shape_matches_words(
                        &filter_words,
                        SPELL_ORIGIN_EXILE_PATTERN,
                    ) {
                        Some(Zone::Exile)
                    } else if trigger_subject_shape_matches_words(
                        &filter_words,
                        SPELL_ORIGIN_HAND_PATTERN,
                    ) {
                        Some(Zone::Hand)
                    } else {
                        None
                    }?;
                    if !trigger_subject_shape_matches_words(&filter_words, SPELL_NOUN_PATTERN) {
                        return None;
                    }
                    let mut filter = ObjectFilter::spell().in_zone(zone);
                    if trigger_subject_shape_matches_words(&filter_words, YOUR_WORD_PATTERN) {
                        filter.owner = Some(actor.clone());
                    } else if trigger_subject_shape_matches_words(
                        &filter_words,
                        OPPONENT_OR_THEIR_WORD_PATTERN,
                    ) {
                        filter.owner = Some(PlayerFilter::Opponent);
                    }
                    Some(filter)
                };
                let compact_words = non_article_word_refs(&filter_words);
                if trigger_subject_shape_matches_words(
                    &compact_words,
                    SPELL_OR_SPELLS_SUFFIX_PATTERN,
                ) {
                    let mut qualifier_words = compact_words.clone();
                    qualifier_words.pop();
                    let qualifier_words = word_refs_except(&qualifier_words, &["or", "and"]);
                    if trigger_subject_shape_matches_words(
                        &qualifier_words,
                        CHOSEN_COLOR_SPELL_QUALIFIER_PATTERN,
                    ) {
                        return Ok(Some(ObjectFilter::spell().of_chosen_color()));
                    }
                }
                match parse_object_filter(filter_tokens, false) {
                    Ok(filter) => Ok(Some(filter)),
                    Err(err) => {
                        let mut compact_words = compact_words;
                        if trigger_subject_shape_matches_words(
                            &compact_words,
                            SPELL_OR_SPELLS_SUFFIX_PATTERN,
                        ) {
                            compact_words.pop();
                            let color_words = word_refs_except(&compact_words, &["or", "and"]);
                            if !color_words.is_empty()
                                && color_words.iter().all(|word| parse_color(word).is_some())
                            {
                                let mut colors = ColorSet::new();
                                for word in color_words {
                                    colors = colors
                                        .union(parse_color(word).expect("validated color word"));
                                }
                                let mut filter = ObjectFilter::spell();
                                filter.colors = Some(colors);
                                return Ok(Some(filter));
                            }
                            if trigger_subject_shape_matches_words(
                                &color_words,
                                CHOSEN_COLOR_SPELL_QUALIFIER_PATTERN,
                            ) {
                                return Ok(Some(ObjectFilter::spell().of_chosen_color()));
                            }
                        }
                        if let Some(origin_filter) = parse_spell_origin_zone_filter() {
                            Ok(Some(origin_filter))
                        } else {
                            Err(err)
                        }
                    }
                }
            }
        };

    if let (Some(cast), Some(copy)) = (cast_idx, copy_idx) {
        let (first, second, first_is_cast) = if cast < copy {
            (cast, copy, true)
        } else {
            (copy, cast, false)
        };
        let between_words = crate::runtime_backend::token_word_refs(&tokens[first + 1..second]);
        if trigger_subject_shape_matches_words(&between_words, CAST_OR_COPY_SEPARATOR_PATTERN) {
            let filter = parse_filter(tokens.get(second + 1..).unwrap_or_default())?;
            let cast_trigger = TriggerSpec::SpellCast {
                filter: filter.clone(),
                caster: actor.clone(),
                during_turn: during_turn.clone(),
                min_spells_this_turn,
                exact_spells_this_turn,
                from_not_hand,
            };
            let copied_trigger = TriggerSpec::SpellCopied {
                filter,
                copier: actor,
            };
            return Ok(Some(if first_is_cast {
                TriggerSpec::Either(Box::new(cast_trigger), Box::new(copied_trigger))
            } else {
                TriggerSpec::Either(Box::new(copied_trigger), Box::new(cast_trigger))
            }));
        }
    }

    if let Some(cast) = cast_idx {
        let mut filter_tokens = tokens.get(cast + 1..).unwrap_or_default();
        if filter_tokens.is_empty() {
            let prefix_tokens = trim_trailing_spell_auxiliary_tokens(&tokens[..cast]);
            if token_slice_has_spell_noun(prefix_tokens) {
                filter_tokens = prefix_tokens;
            }
        }
        let filter = parse_filter(filter_tokens)?;
        return Ok(Some(TriggerSpec::SpellCast {
            filter,
            caster: actor,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        }));
    }

    if let Some(copy) = copy_idx {
        let filter = parse_filter(tokens.get(copy + 1..).unwrap_or_default())?;
        return Ok(Some(TriggerSpec::SpellCopied {
            filter,
            copier: actor,
        }));
    }

    Ok(None)
}

pub(crate) fn is_spawn_scion_token_mana_reminder(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    trigger_subject_shape_matches_words(&words, TOKEN_MANA_REMINDER_PREFIX_PATTERN)
        && trigger_subject_shape_matches_words(&words, TOKEN_MANA_REMINDER_WORDS_PATTERN)
}

pub(crate) fn is_round_up_each_time_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    trigger_subject_shape_matches_words(&words, ROUND_UP_EACH_TIME_PREFIX_PATTERN)
}

pub(crate) enum MayCastItVerb {
    Cast,
    Play,
}

pub(crate) struct MayCastTaggedSpec {
    pub(crate) tag: TagKey,
    pub(crate) player: PlayerAst,
    pub(crate) verb: MayCastItVerb,
    pub(crate) as_copy: bool,
    pub(crate) without_paying_mana_cost: bool,
    pub(crate) predicate: Option<PredicateAst>,
    pub(crate) cost_reduction: Option<ManaCost>,
}

pub(crate) fn parse_may_cast_it_sentence(tokens: &[OwnedLexToken]) -> Option<MayCastTaggedSpec> {
    let clause_words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    let mut clause_words =
        crate::runtime_backend::util::strip_leading_word_refs_any(&clause_words, &["then", "and"])
            .to_vec();

    if trigger_subject_shape_matches_words(&clause_words, IF_YOU_DO_PREFIX_PATTERN) {
        clause_words = crate::runtime_backend::util::strip_leading_word_refs_any(
            &clause_words[3..],
            &["then", "and"],
        )
        .to_vec();
    }

    let (player, subject_tag, verb_idx) = if clause_words.len() >= 4
        && trigger_subject_shape_matches_words(&clause_words, YOU_MAY_PREFIX_PATTERN)
    {
        (PlayerAst::Implicit, None, 2usize)
    } else if clause_words.len() >= 7
        && trigger_subject_shape_matches_words(&clause_words, EXILED_CARDS_OWNER_MAY_PREFIX_PATTERN)
    {
        (
            PlayerAst::ItsOwner,
            Some(TagKey::from(crate::tag::SOURCE_EXILED_TAG)),
            5usize,
        )
    } else {
        return None;
    };

    if clause_words.len() <= verb_idx + 1 {
        return None;
    }

    let verb = match clause_words[verb_idx] {
        "cast" => MayCastItVerb::Cast,
        "play" => MayCastItVerb::Play,
        _ => return None,
    };

    let rest = &clause_words[verb_idx + 1..];
    let (tag, as_copy, consumed) =
        if trigger_subject_shape_matches_words(rest, IT_REFERENCE_PREFIX_PATTERN) {
            (TagKey::from(IT_TAG), false, 1usize)
        } else if trigger_subject_shape_matches_words(rest, THAT_CARD_REFERENCE_PREFIX_PATTERN) {
            (
                subject_tag.unwrap_or_else(|| TagKey::from(IT_TAG)),
                false,
                2usize,
            )
        } else if trigger_subject_shape_matches_words(rest, EXILED_CARD_REFERENCE_PREFIX_PATTERN) {
            (TagKey::from(crate::tag::SOURCE_EXILED_TAG), false, 3usize)
        } else if trigger_subject_shape_matches_words(rest, REVEALED_CARD_REFERENCE_PREFIX_PATTERN)
        {
            (TagKey::from("__last_revealed__"), false, 3usize)
        } else if trigger_subject_shape_matches_words(rest, COPY_REFERENCE_PREFIX_PATTERN) {
            (TagKey::from(IT_TAG), true, 2usize)
        } else {
            return None;
        };

    let tail = &rest[consumed..];
    if tail.is_empty() {
        return Some(MayCastTaggedSpec {
            tag,
            player,
            verb,
            as_copy,
            without_paying_mana_cost: false,
            predicate: None,
            cost_reduction: None,
        });
    }
    if trigger_subject_shape_matches_words(tail, WITHOUT_PAYING_MANA_COST_TAIL_PATTERN) {
        return Some(MayCastTaggedSpec {
            tag,
            player,
            verb,
            as_copy,
            without_paying_mana_cost: true,
            predicate: None,
            cost_reduction: None,
        });
    }
    if tail.len() >= 13
        && trigger_subject_shape_matches_words(tail, WITHOUT_PAYING_MANA_COST_MV_LTE_PREFIX_PATTERN)
        && let Some((value, used)) = parse_value_expr_words(&tail[17..])
        && used == tail.len().saturating_sub(17)
    {
        return Some(MayCastTaggedSpec {
            tag,
            player,
            verb,
            as_copy,
            without_paying_mana_cost: true,
            predicate: Some(PredicateAst::ItMatches(
                ObjectFilter::default().with_mana_value(
                    crate::filter::Comparison::LessThanOrEqualExpr(Box::new(value)),
                ),
            )),
            cost_reduction: None,
        });
    }
    if let [
        "without",
        "paying",
        "its",
        "mana",
        "cost",
        "if",
        "its",
        "mana",
        "value",
        "is",
        parity,
    ] = tail
    {
        let parity = match *parity {
            "odd" => crate::filter::ParityRequirement::Odd,
            "even" => crate::filter::ParityRequirement::Even,
            _ => return None,
        };
        return Some(MayCastTaggedSpec {
            tag,
            player,
            verb,
            as_copy,
            without_paying_mana_cost: true,
            predicate: Some(PredicateAst::ItMatches(
                ObjectFilter::default().with_mana_value_parity(parity),
            )),
            cost_reduction: None,
        });
    }
    None
}

pub(crate) fn parse_copy_reference_cost_reduction_sentence(
    tokens: &[OwnedLexToken],
) -> Option<ManaCost> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 6 {
        return None;
    }
    if !trigger_subject_shape_matches_words(&clause_words, COPY_COSTS_PREFIX_PATTERN) {
        return None;
    }

    let less_idx = LESS_WORD_PATTERN.find_word(&clause_words)?;
    if !trigger_subject_shape_matches_words(&clause_words[less_idx..], LESS_TO_CAST_TAIL_PATTERN) {
        return None;
    }

    let costs_token_idx = find_token_shape(tokens, &COSTS_WORD_PATTERN)?;
    let less_token_idx = find_token_shape(tokens, &LESS_WORD_PATTERN)?;
    if less_token_idx <= costs_token_idx + 1 {
        return None;
    }
    let reduction_tokens = trim_commas(&tokens[costs_token_idx + 1..less_token_idx]).to_vec();
    let (reduction, consumed) = parse_cost_modifier_mana_cost(&reduction_tokens)?;
    if consumed != reduction_tokens.len() {
        return None;
    }
    Some(reduction)
}

pub(crate) fn build_may_cast_tagged_effect(spec: &MayCastTaggedSpec) -> EffectAst {
    let cast = EffectAst::subject_verb_cast_tagged(
        spec.tag.clone(),
        spec.player,
        matches!(spec.verb, MayCastItVerb::Play),
        spec.as_copy,
        spec.without_paying_mana_cost,
        spec.cost_reduction.clone(),
    );
    let may = if matches!(spec.player, PlayerAst::Implicit | PlayerAst::You) {
        EffectAst::May {
            effects: vec![cast],
        }
    } else {
        EffectAst::MayByPlayer {
            player: spec.player,
            effects: vec![cast],
        }
    };
    if let Some(predicate) = &spec.predicate {
        EffectAst::Conditional {
            predicate: predicate.clone(),
            if_true: vec![may],
            if_false: Vec::new(),
        }
    } else {
        may
    }
}

pub(crate) fn is_simple_copy_reference_sentence(tokens: &[OwnedLexToken]) -> bool {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    matches!(
        clause_words.as_slice(),
        ["copy", "it"]
            | ["copy", "this"]
            | ["copy", "that"]
            | ["copy", "that", "card"]
            | ["copy", "the", "exiled", "card"]
    )
}

pub(crate) fn token_name_mentions_eldrazi_spawn_or_scion(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (lower.matches("eldrazi").next().is_some() && lower.matches("spawn").next().is_some())
        || (lower.matches("eldrazi").next().is_some() && lower.matches("scion").next().is_some())
}

pub(crate) fn effect_creates_eldrazi_spawn_or_scion(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb)
            if matches!(
                &subject_verb.action,
                crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenWithMods {
                    name,
                    ..
                } if token_name_mentions_eldrazi_spawn_or_scion(name)
            ) =>
        {
            true
        }
        _ => {
            let mut found = false;
            for_each_nested_effects(effect, false, |nested| {
                if !found && nested.iter().any(effect_creates_eldrazi_spawn_or_scion) {
                    found = true;
                }
            });
            found
        }
    }
}

pub(crate) fn effect_creates_any_token(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb)
            if matches!(
                &subject_verb.action,
                crate::runtime_backend::ast::SubjectVerbActionAst::Populate { .. }
                    | crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenWithMods {
                        ..
                    }
                    | crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenCopy { .. }
                    | crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenCopyFromSource {
                        ..
                    }
            ) =>
        {
            true
        }
        _ => {
            let mut found = false;
            for_each_nested_effects(effect, false, |nested| {
                if !found && nested.iter().any(effect_creates_any_token) {
                    found = true;
                }
            });
            found
        }
    }
}

pub(crate) fn last_created_token_info(effects: &[EffectAst]) -> Option<(String, PlayerAst)> {
    for effect in effects.iter().rev() {
        if let Some(info) = created_token_info_from_effect(effect) {
            return Some(info);
        }
    }
    None
}

pub(crate) fn created_token_info_from_effect(effect: &EffectAst) -> Option<(String, PlayerAst)> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenWithMods {
                name,
                player,
                ..
            } => Some((name.clone(), *player)),
            _ => {
                let mut found = None;
                for_each_nested_effects(effect, true, |nested| {
                    if found.is_none() {
                        found = last_created_token_info(nested);
                    }
                });
                found
            }
        },
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, true, |nested| {
                if found.is_none() {
                    found = last_created_token_info(nested);
                }
            });
            found
        }
    }
}

pub(crate) fn title_case_token_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut out = first.to_uppercase().to_string();
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}

pub(crate) fn controller_filter_for_token_player(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        _ => None,
    }
}

pub(crate) fn parse_sentence_exile_that_token_when_source_leaves(
    tokens: &[OwnedLexToken],
    prior_effects: &[EffectAst],
) -> Option<EffectAst> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 6
        || !trigger_subject_shape_matches_words(&clause_words, EXILE_TOKEN_LIFECYCLE_ACTION_PATTERN)
    {
        return None;
    }
    let when_idx = WHEN_WORD_PATTERN.find_word(&clause_words)?;
    if when_idx < 2 || when_idx + 3 >= clause_words.len() {
        return None;
    }
    if !trigger_subject_shape_matches_words(&clause_words, LEAVES_BATTLEFIELD_SUFFIX_PATTERN) {
        return None;
    }
    let object_words = &clause_words[1..when_idx];
    if !trigger_subject_shape_matches_words(object_words, CREATED_TOKEN_REFERENCE_PATTERN) {
        return None;
    }
    let subject_words = &clause_words[when_idx + 1..clause_words.len() - 3];
    if !is_source_reference_words(subject_words) {
        return None;
    }

    let _ = last_created_token_info(prior_effects)?;

    Some(EffectAst::subject_verb_exile_when_source_leaves(
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
    ))
}

pub(crate) fn parse_sentence_sacrifice_source_when_that_token_leaves(
    tokens: &[OwnedLexToken],
    prior_effects: &[EffectAst],
) -> Option<EffectAst> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() < 8
        || !trigger_subject_shape_matches_words(
            &clause_words,
            SACRIFICE_TOKEN_LIFECYCLE_ACTION_PATTERN,
        )
    {
        return None;
    }
    let when_idx = WHEN_WORD_PATTERN.find_word(&clause_words)?;
    if when_idx < 2 || when_idx + 4 > clause_words.len() {
        return None;
    }
    let subject_words = &clause_words[1..when_idx];
    if !is_source_reference_words(subject_words) {
        return None;
    }
    if !trigger_subject_shape_matches_words(
        &clause_words[when_idx + 1..],
        THAT_TOKEN_LEAVES_BATTLEFIELD_PATTERN,
    ) {
        return None;
    }

    let _ = last_created_token_info(prior_effects)?;

    Some(EffectAst::subject_verb_sacrifice_source_when_leaves(
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
    ))
}

pub(crate) fn is_generic_token_reminder_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }
    if trigger_subject_shape_matches_words(&words, TOKEN_HAS_ABILITY_PREFIX_PATTERN) {
        return true;
    }
    if trigger_subject_shape_matches_words(&words, TOKEN_PRONOUN_TRIGGER_PREFIX_PATTERN) {
        return true;
    }
    if trigger_subject_shape_matches_words(&words, TOKEN_PT_REMINDER_PREFIX_PATTERN) {
        return true;
    }
    let delayed_lifecycle_reference =
        trigger_subject_shape_matches_words(&words, TOKEN_DELAYED_LIFECYCLE_ACTION_PATTERN)
            && (is_beginning_of_end_step_words(&words) || is_end_of_combat_words(&words))
            && trigger_subject_shape_matches_words(&words, TOKEN_REMINDER_REFERENCE_WORD_PATTERN);
    if delayed_lifecycle_reference {
        return true;
    }
    trigger_subject_shape_matches_words(&words, TOKEN_REMINDER_REFERENCE_PREFIX_PATTERN)
}

pub(crate) fn strip_embedded_token_rules_text(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    if !trigger_subject_shape_matches_words(&words_all, CREATE_WORD_PATTERN)
        || !trigger_subject_shape_matches_words(&words_all, TOKEN_WORD_PATTERN)
    {
        return tokens.to_vec();
    }
    let Some(with_idx) = find_trigger_subject_token_word(tokens, WITH_WORD) else {
        return tokens.to_vec();
    };
    let next_word = tokens.get(with_idx + 1).and_then(OwnedLexToken::as_word);
    if next_word.is_some_and(|word| trigger_subject_word_is(word, T_WORD)) {
        return tokens[..with_idx].to_vec();
    }
    tokens.to_vec()
}

fn rewrite_token_pronoun_trigger_reminder<'a>(reminder_words: &[&'a str]) -> Option<Vec<&'a str>> {
    if !trigger_subject_shape_matches_words(reminder_words, TOKEN_PRONOUN_TRIGGER_PREFIX_PATTERN) {
        return None;
    }
    let trigger_word = reminder_words.first().copied()?;
    let mut rewritten = vec![trigger_word, "this", "token"];
    rewritten.extend_from_slice(&reminder_words[2..]);
    Some(rewritten)
}

pub(crate) fn append_token_reminder_to_last_create_effect(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) -> bool {
    let reminder_word_storage = tokens
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::ManaGroup => {
                let inner = token.mana_group_inner()?;
                (!inner.is_empty()).then(|| inner.to_ascii_lowercase())
            }
            _ => token.as_word().map(|word| word.to_ascii_lowercase()),
        })
        .collect::<Vec<_>>();
    let mut reminder_words = reminder_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut prepend_with = false;
    if trigger_subject_shape_matches_words(&reminder_words, TOKEN_HAS_ABILITY_PREFIX_PATTERN) {
        reminder_words = reminder_words[2..].to_vec();
        prepend_with = true;
    }
    if let Some(rewritten) = rewrite_token_pronoun_trigger_reminder(&reminder_words) {
        reminder_words = rewritten;
    }
    if reminder_words.is_empty() {
        return false;
    }
    let reminder = if prepend_with {
        format!("with {}", reminder_words.join(" "))
    } else {
        reminder_words.join(" ")
    };
    for effect in effects.iter_mut().rev() {
        if append_token_reminder_to_effect(Some(effect), &reminder, &reminder_words) {
            return true;
        }
    }
    false
}

pub(crate) fn append_token_reminder_to_effect(
    effect: Option<&mut EffectAst>,
    reminder: &str,
    reminder_words: &[&str],
) -> bool {
    fn token_reminder_has_haste(reminder_words: &[&str]) -> bool {
        trigger_subject_shape_matches_words(reminder_words, HASTE_REMINDER_PATTERN)
    }

    fn token_reminder_exiles_at_end_of_combat(reminder_words: &[&str]) -> bool {
        trigger_subject_shape_matches_words(reminder_words, TOKEN_REMINDER_EXILE_WORD_PATTERN)
            && is_end_of_combat_words(reminder_words)
    }

    fn token_reminder_sacrifices_at_end_of_combat(reminder_words: &[&str]) -> bool {
        trigger_subject_shape_matches_words(reminder_words, TOKEN_REMINDER_SACRIFICE_WORD_PATTERN)
            && is_end_of_combat_words(reminder_words)
    }

    fn parse_dynamic_token_pt_reminder(reminder_words: &[&str]) -> Option<(Value, Value)> {
        use super::super::util::parse_value_expr_words;

        fn parse_possessive_stat_rhs(words: &[&str], is_power: bool) -> Option<Value> {
            let stat_word = if is_power { "power" } else { "toughness" };
            if !words.last().is_some_and(|word| *word == stat_word) {
                return None;
            }
            let owner_words = &words[..words.len() - 1];
            let value_for_source = || {
                if is_power {
                    Value::SourcePower
                } else {
                    Value::SourceToughness
                }
            };
            let value_for_it = || {
                let spec = Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG)));
                if is_power {
                    Value::PowerOf(spec)
                } else {
                    Value::ToughnessOf(spec)
                }
            };

            match owner_words {
                ["this"] | ["thiss"] | ["this", "creature"] | ["thiss", "creature"] => {
                    Some(value_for_source())
                }
                ["this", owner] | ["thiss", owner]
                    if matches!(*owner, "creatures" | "creature's") =>
                {
                    Some(value_for_source())
                }
                ["that", "card"] | ["that", "creature"] | ["that", "object"] => {
                    Some(value_for_it())
                }
                ["that", owner]
                    if matches!(
                        *owner,
                        "cards" | "card's" | "creatures" | "creature's" | "objects" | "object's"
                    ) =>
                {
                    Some(value_for_it())
                }
                _ => None,
            }
        }

        let parse_rhs = |words: &[&str]| {
            if let Some(value) = parse_possessive_stat_rhs(words, true)
                .or_else(|| parse_possessive_stat_rhs(words, false))
            {
                return Some(value);
            }
            let (value, used) = parse_value_expr_words(words)?;
            (used == words.len()).then_some(value)
        };

        if trigger_subject_shape_matches_words(reminder_words, TOKEN_PT_EQUAL_PAIR_PATTERN) {
            let rhs_words = &reminder_words[8..];
            let value = parse_rhs(rhs_words)?;
            return Some((value.clone(), value));
        }
        let mut and_idx = None;
        let mut idx = 0usize;
        while idx < reminder_words.len() {
            if trigger_subject_word_is(reminder_words[idx], AND_WORD) {
                and_idx = Some(idx);
                break;
            }
            idx += 1;
        }
        if let Some(and_idx) = and_idx {
            let left = &reminder_words[..and_idx];
            let right = &reminder_words[and_idx + 1..];
            let power_words = slice_strip_prefix(left, &["its", "power", "is", "equal", "to"])?;
            let toughness_words =
                slice_strip_prefix(right, &["its", "toughness", "is", "equal", "to"])?;
            return Some((parse_rhs(power_words)?, parse_rhs(toughness_words)?));
        }

        None
    }

    let Some(effect) = effect else {
        return false;
    };
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            crate::runtime_backend::ast::SubjectVerbActionAst::Populate {
                has_haste,
                exile_at_end_of_combat,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                ..
            } => {
                if token_reminder_has_haste(reminder_words) {
                    *has_haste = true;
                    return true;
                }
                let (sacrifice_next_end_step, exile_next_end_step) =
                    parse_next_end_step_token_delay_flags(reminder_words);
                if sacrifice_next_end_step {
                    *sacrifice_at_next_end_step = true;
                    return true;
                }
                if exile_next_end_step {
                    *exile_at_next_end_step = true;
                    return true;
                }
                let exile_end_of_combat = token_reminder_exiles_at_end_of_combat(reminder_words);
                if exile_end_of_combat {
                    *exile_at_end_of_combat = true;
                    return true;
                }
                false
            }
            crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenCopy {
                has_haste,
                exile_at_end_of_combat,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                ..
            }
            | crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenCopyFromSource {
                has_haste,
                exile_at_end_of_combat,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                ..
            } => {
                if token_reminder_has_haste(reminder_words) {
                    *has_haste = true;
                    return true;
                }
                let (sacrifice_next_end_step, exile_next_end_step) =
                    parse_next_end_step_token_delay_flags(reminder_words);
                if sacrifice_next_end_step {
                    *sacrifice_at_next_end_step = true;
                }
                if exile_next_end_step {
                    *exile_at_next_end_step = true;
                }
                let exile_end_of_combat = token_reminder_exiles_at_end_of_combat(reminder_words);
                if exile_end_of_combat {
                    *exile_at_end_of_combat = true;
                }
                *has_haste
                    || *sacrifice_at_next_end_step
                    || *exile_at_next_end_step
                    || *exile_at_end_of_combat
            }
            crate::runtime_backend::ast::SubjectVerbActionAst::CreateTokenWithMods {
                name,
                dynamic_power_toughness,
                exile_at_end_of_combat,
                sacrifice_at_end_of_combat,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                ..
            } => {
                if let Some((power, toughness)) = parse_dynamic_token_pt_reminder(reminder_words) {
                    *dynamic_power_toughness = Some((power, toughness));
                    return true;
                }
                if !name.chars().last().is_some_and(|ch| ch == ' ') {
                    name.push(' ');
                }
                name.push_str(reminder);
                let (sacrifice_next_end_step, exile_next_end_step) =
                    parse_next_end_step_token_delay_flags(reminder_words);
                if sacrifice_next_end_step {
                    *sacrifice_at_next_end_step = true;
                }
                if exile_next_end_step {
                    *exile_at_next_end_step = true;
                }
                let exile_end_of_combat = token_reminder_exiles_at_end_of_combat(reminder_words);
                if exile_end_of_combat {
                    *exile_at_end_of_combat = true;
                }
                let sacrifice_end_of_combat =
                    token_reminder_sacrifices_at_end_of_combat(reminder_words);
                if sacrifice_end_of_combat {
                    *sacrifice_at_end_of_combat = true;
                }
                true
            }
            _ => false,
        },
        _ => {
            let mut applied = false;
            for_each_nested_effects_mut(effect, false, |nested| {
                if !applied {
                    applied = append_token_reminder_to_effect(
                        nested.last_mut(),
                        reminder,
                        reminder_words,
                    );
                }
            });
            applied
        }
    }
}
