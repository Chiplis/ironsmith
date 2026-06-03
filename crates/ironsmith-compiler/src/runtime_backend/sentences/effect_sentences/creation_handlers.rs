use crate::cards::builders::{
    CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, KeywordAction, ObjectRefAst,
    OwnedLexToken, PlayerAst, PredicateAst, SubjectAst, SubjectVerbActionAst, SubjectVerbRoleAst,
    TagKey, TargetAst,
};
use crate::color::ColorSet;
use crate::effect::{EventValueSpec, Value};
use crate::static_abilities::{Anthem, AnthemCountExpression, AnthemValue, StaticAbility};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
use ironsmith_core::ValueSurfaceHint;

use super::super::grammar::primitives as grammar;
use super::super::grammar::structure::parse_who_player_predicate_lexed;
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::{
    LexedClause, render_token_slice, token_slice_at_is, token_slice_first_is, token_word_refs,
};
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::{
    find_index as find_token_index, str_split_once_char, str_starts_with_char,
};
use super::super::util::{
    is_article, parse_card_type, parse_color, parse_counter_type_word, parse_number,
    parse_subtype_flexible, parse_target_phrase, parse_value, source_choose_spec_for_surface,
    source_reference_surface_for_words, token_index_for_word_index, trim_commas,
    value_contains_unbound_x,
};
use super::clause_pattern_helpers::{ClauseShape, clause_shape, extract_subject_player};
use super::conditionals::parse_subtype_word;
use super::dispatch_entry::target_references_it;
use super::lex_chain_helpers::starts_with_inline_token_rules_tail;

fn push_unique_card_type(card_types: &mut Vec<CardType>, card_type: CardType) {
    crate::slice_primitives::push_unique(card_types, card_type);
}

fn push_unique_subtype(subtypes: &mut Vec<Subtype>, subtype: Subtype) {
    crate::slice_primitives::push_unique(subtypes, subtype);
}

const CREATE_CARD_TYPES_AMONG_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases & [&[&["card", "type", "among"], &["card", "types", "among"]]]
);
const CREATE_EXCEPT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["except"]);
const CREATE_LOSE_OR_LOSES_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["lose", "loses"]]);
const CREATE_SOULBOND_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["soulbond"]);
const CREATE_NOT_LEGENDARY_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["isnt", "legendary"],
            &["isn't", "legendary"],
            &["is", "not", "legendary"],
        ]]
);
const CREATE_GRANT_VERB_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["has", "have", "gain", "gains"]]);
const CREATE_BEGINNING_NEXT_END_STEP_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["beginning", "of", "the", "next", "end", "step"],
            &["beginning", "of", "next", "end", "step"],
            &["beginning", "of", "the", "end", "step"],
            &["beginning", "of", "end", "step"],
        ]]
);
const CREATE_SACRIFICE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["sacrifice"]);
const CREATE_EXILE_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["exile"]);
const CREATE_DELAY_REFERENCE_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_words & [&["token", "tokens", "permanent", "permanents", "it", "them"]]
);
const CREATE_TOKEN_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["token"]);
const CREATE_LEGENDARY_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["legendary"]);
const CREATE_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const CREATE_AND_OR_OR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["or"]]);
const CREATE_ARTICLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"]]);
const CREATE_ARTICLE_OR_THE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["the"]]);
const CREATE_OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const CREATE_INLINE_REFERENCE_START_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["that"], &["it"], &["those"], &["thats"], &["its"]]);
const CREATE_TAPPED_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["tapped"]);
const CREATE_ATTACKING_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["attacking"]);
const CREATE_DECAYED_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["decayed"]);
const CREATE_INLINE_MODIFIER_START_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["thats"], &["that", "is"], &["that", "are"]]);
const CREATE_IDENTITY_CLAUSE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["its"],
            &["it", "is"],
            &["it", "s"],
            &["it's"],
            &["it’s"],
            &["theyre"],
            &["they", "re"],
            &["they're"],
            &["they’re"],
            &["they", "are"],
        ]
);
const CREATE_ATTACK_TARGET_PLAYER_OR_PLANESWALKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &[
                "that",
                "player",
                "or",
                "a",
                "planeswalker",
                "they",
                "control"
            ],
            &["that", "player", "or", "planeswalker", "they", "control"],
            &[
                "that",
                "player",
                "or",
                "a",
                "planeswalker",
                "they",
                "controls"
            ],
            &["that", "player", "or", "planeswalker", "they", "controls"],
            &[
                "that",
                "player",
                "or",
                "a",
                "planeswalker",
                "their",
                "control"
            ],
            &["that", "player", "or", "planeswalker", "their", "control"],
        ]]
);
const CREATE_TOKEN_RULES_TEXT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["it", "has"],
            &["it", "gains"],
            &["it", "gets"],
            &["this", "token"],
            &["that", "token"],
        ]]
);
const CREATE_EQUIPMENT_RULES_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["equipped", "creature", "has"],
            &["equipped", "creature", "gets"],
        ]]
);
const CREATE_UNBLOCKABLE_RULES_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["this", "token", "cant", "be", "blocked"],
            &["this", "creature", "cant", "be", "blocked"],
            &["cant", "be", "blocked"],
        ]]
);
const CREATE_ATTACKING_THAT_PLAYER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["attacking", "that", "player"]]);
const CREATE_ATTACHED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["attached"]);
const CREATE_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["to"]);
const CREATE_WITH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["with"]);
const CREATE_NAMED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["named"]);
const CREATE_TOKEN_OR_TOKENS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["token"], &["tokens"]]);
const CREATE_FOR_EACH_WORDS: &[&str] = &["for", "each"];
const CREATE_FOR_EACH_PATTERN: ClauseShape<'static> = clause_shape!(exact & CREATE_FOR_EACH_WORDS);
const CREATE_COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const CREATE_SOURCE_COUNTER_LEADING_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["one"], &["another"]]);
const CREATE_ON_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["on"]);
const CREATE_SOURCE_COUNTER_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it"],
            &["this"],
            &["this", "card"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "source"],
            &["this", "artifact"],
            &["this", "land"],
            &["this", "enchantment"],
        ]
);
const CREATE_TOKEN_GETS_FOR_EACH_WORDS: &[&str] =
    &["this", "token", "gets", "+1/+1", "for", "each"];
const CREATE_TOKEN_GETS_FOR_EACH_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & CREATE_TOKEN_GETS_FOR_EACH_WORDS);
const CREATE_CREATURE_GETS_FOR_EACH_WORDS: &[&str] =
    &["this", "creature", "gets", "+1/+1", "for", "each"];
const CREATE_CREATURE_GETS_FOR_EACH_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & CREATE_CREATURE_GETS_FOR_EACH_WORDS);
const CREATE_YOU_CONTROL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you", "control"]);
const CREATE_IN_ADDITION_TO_ITS_TYPES_WORDS: &[&str] =
    &["in", "addition", "to", "its", "other", "types"];
const CREATE_IN_ADDITION_TO_ITS_TYPES_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & CREATE_IN_ADDITION_TO_ITS_TYPES_WORDS);
const CREATE_IN_ADDITION_TO_THEIR_TYPES_WORDS: &[&str] =
    &["in", "addition", "to", "their", "other", "types"];
const CREATE_IN_ADDITION_TO_THEIR_TYPES_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & CREATE_IN_ADDITION_TO_THEIR_TYPES_WORDS);
const CREATE_DESCRIPTOR_END_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["with"], &["has"], &["have"], &["gain"], &["gains"]]);
const CREATE_NAME_END_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["with"], &["that"], &["which"], &["thats"]]);
const CREATE_RULES_TEXT_START_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["when"],
            &["whenever"],
            &["if"],
            &["t"],
            &["this"],
            &["that"],
            &["it"],
            &["those"],
            &["sacrifice"],
            &["add"],
            &["draw"],
            &["deals"],
            &["deal"],
        ]
);
const CREATE_PRESERVE_RULES_TAIL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["when"],
            &["whenever"],
            &["at"],
            &["sacrifice"],
            &["return"],
            &["counter"],
            &["draw"],
            &["add"],
            &["deals"],
            &["deal"],
            &["gets"],
            &["gain"],
            &["gains"],
            &["power"],
            &["toughness"],
            &["cant"],
            &["can"],
            &["block"],
        ]
);
const CREATE_PT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["x/x"]]);
const CREATE_WHERE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["where"]);
const CREATE_EXILED_THIS_WAY_WORDS: &[&str] = &["exiled", "this", "way"];
const CREATE_EXILED_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & CREATE_EXILED_THIS_WAY_WORDS);
const CREATE_THIS_WAY_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["this", "way"]]);
const CREATE_HASTE_GRANT_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases & [&[&["has", "haste"], &["gain", "haste"], &["gains", "haste"]]]
);
const CREATE_OTHER_THAN_FIRST_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["other", "than", "the", "first"]]);
const CREATE_TIME_OR_TIMES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["time"], &["times"]]);
const CREATE_ONCE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["once"]);
const CREATE_TWICE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["twice"]);
const CREATE_COPY_OR_COPIES_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["copy", "copies"]]);
const CREATE_COPY_OR_COPIES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["copy"], &["copies"]]);
const CREATE_WHEN_OR_WHENEVER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["when"], &["whenever"]]);
const CREATE_YOU_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["you"], &["your"], &["youve"]]);
const CREATE_OPPONENT_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["opponent"], &["opponents"]]);
const CREATE_SPELL_OR_SPELLS_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["spell", "spells"]]);
const CREATE_CAST_OR_CASTS_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["cast", "casts"]]);
const CREATE_TURN_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["turn"]);
const CREATE_INVESTIGATE_TRAILING_TIME_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["time"], &["times"]]);
const CREATE_EQUAL_TO_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["equal", "to"]);

fn create_find_phrase_shape(
    words: &[&str],
    phrase_len: usize,
    shape: ClauseShape<'static>,
) -> Option<usize> {
    words
        .windows(phrase_len)
        .position(|window| shape.matches_words(window))
}

fn reject_lossy_for_each_fallback(
    tokens: &[OwnedLexToken],
    full_clause_words: &[&str],
) -> Result<(), CardTextError> {
    let words = token_word_refs(tokens);
    if CREATE_CARD_TYPES_AMONG_PATTERN.matches_words(&words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported card-types-among create count (clause: '{}')",
            full_clause_words.join(" ")
        )));
    }
    if CREATE_THIS_WAY_MARKER_PATTERN.matches_words(&words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported this-way create count (clause: '{}')",
            full_clause_words.join(" ")
        )));
    }
    Ok(())
}

pub(crate) fn looks_like_pt_word(word: &str) -> bool {
    let Some((power, toughness)) = str_split_once_char(word, '/') else {
        return false;
    };
    let is_component = |part: &str| {
        let part = part.trim_matches(|ch| matches!(ch, '+' | '-'));
        matches!(part, "x" | "*") || part.parse::<i32>().is_ok()
    };
    is_component(power) && is_component(toughness)
}

pub(crate) fn parse_unsigned_pt_word(word: &str) -> Option<(i32, i32)> {
    let (power, toughness) = str_split_once_char(word, '/')?;
    if str_starts_with_char(power, '+')
        || str_starts_with_char(toughness, '+')
        || str_starts_with_char(power, '-')
        || str_starts_with_char(toughness, '-')
    {
        return None;
    }
    let power = power.parse::<i32>().ok()?;
    let toughness = toughness.parse::<i32>().ok()?;
    Some((power, toughness))
}

pub(crate) fn is_probable_token_name_word(word: &str) -> bool {
    if !word
        .chars()
        .all(|ch| ch.is_ascii_alphabetic() || ch == '\'' || ch == '-')
    {
        return false;
    }
    !matches!(
        word,
        "legendary"
            | "artifact"
            | "enchantment"
            | "creature"
            | "token"
            | "tokens"
            | "white"
            | "blue"
            | "black"
            | "red"
            | "green"
            | "colorless"
    )
}

pub(crate) fn parse_copy_modifiers_from_tail(
    tail_words: &[&str],
) -> Result<
    (
        Option<ColorSet>,
        Option<Vec<CardType>>,
        Option<Vec<Subtype>>,
        Vec<CardType>,
        Vec<Subtype>,
        Vec<Supertype>,
        Option<(i32, i32)>,
        Vec<StaticAbility>,
    ),
    CardTextError,
> {
    let mut set_colors = None;
    let mut set_card_types = None;
    let mut set_subtypes = None;
    let mut added_card_types = Vec::new();
    let mut added_subtypes = Vec::new();
    let mut removed_supertypes = Vec::new();
    let mut set_base_power_toughness = None;
    let mut granted_abilities = Vec::new();

    let except_idx = CREATE_EXCEPT_WORD_PATTERN.rfind_word(tail_words);
    let modifier_words = except_idx
        .map(|idx| &tail_words[idx + 1..])
        .unwrap_or_default();
    if modifier_words.is_empty() {
        return Ok((
            set_colors,
            set_card_types,
            set_subtypes,
            added_card_types,
            added_subtypes,
            removed_supertypes,
            set_base_power_toughness,
            granted_abilities,
        ));
    }

    if CREATE_LOSE_OR_LOSES_MARKER_PATTERN.matches_words(modifier_words)
        && CREATE_SOULBOND_MARKER_PATTERN.matches_words(modifier_words)
    {
        return Err(CardTextError::ParseError(
            "removing soulbond requires non-marker semantics".to_string(),
        ));
    }

    if CREATE_NOT_LEGENDARY_PATTERN.matches_words(modifier_words) {
        removed_supertypes.push(Supertype::Legendary);
    }

    if let Some((power, toughness)) = modifier_words
        .iter()
        .find_map(|word| parse_unsigned_pt_word(word))
    {
        set_base_power_toughness = Some((power, toughness));
    }

    let has_grant_verb = CREATE_GRANT_VERB_MARKER_PATTERN.matches_words(modifier_words);
    let has_modifier_keyword = |keyword: &str| {
        let with_keyword = ["with", keyword];
        let with_keyword_phrases: &[&[&str]] = &[&with_keyword];
        let keyword_words = [keyword];
        ClauseShape::new()
            .contains_phrases(with_keyword_phrases)
            .matches_words(modifier_words)
            || (has_grant_verb
                && ClauseShape::new()
                    .contains_words(&keyword_words)
                    .matches_words(modifier_words))
    };
    if has_modifier_keyword("flying") {
        granted_abilities.push(StaticAbility::flying());
    }
    if has_modifier_keyword("trample") {
        granted_abilities.push(StaticAbility::trample());
    }
    if let Some(idx) = create_find_phrase_shape(
        modifier_words,
        CREATE_TOKEN_GETS_FOR_EACH_WORDS.len(),
        CREATE_TOKEN_GETS_FOR_EACH_PATTERN,
    )
    .or_else(|| {
        create_find_phrase_shape(
            modifier_words,
            CREATE_CREATURE_GETS_FOR_EACH_WORDS.len(),
            CREATE_CREATURE_GETS_FOR_EACH_PATTERN,
        )
    }) {
        let mut tail = modifier_words.get(idx + 6..).unwrap_or_default();
        while tail.first().is_some_and(|word| {
            is_article(word) || CREATE_ARTICLE_OR_THE_WORD_PATTERN.matches_word(word)
        }) {
            tail = &tail[1..];
        }
        if let Some(subtype_word) = tail.first().copied() {
            let subtype = parse_subtype_flexible(subtype_word);
            let you_control = tail
                .windows(2)
                .any(|window| CREATE_YOU_CONTROL_PATTERN.matches_words(window));
            if let Some(subtype) = subtype
                && you_control
            {
                let mut filter = ObjectFilter::default();
                filter.zone = Some(Zone::Battlefield);
                filter.controller = Some(PlayerFilter::You);
                filter.subtypes = vec![subtype];
                let count = AnthemCountExpression::MatchingFilter(filter);
                let anthem = Anthem::for_source(0, 0).with_values(
                    AnthemValue::scaled(1, count.clone()),
                    AnthemValue::scaled(1, count),
                );
                granted_abilities.push(StaticAbility::new(anthem));
            }
        }
    }

    let addition_idx = create_find_phrase_shape(
        modifier_words,
        CREATE_IN_ADDITION_TO_ITS_TYPES_WORDS.len(),
        CREATE_IN_ADDITION_TO_ITS_TYPES_PATTERN,
    )
    .or_else(|| {
        create_find_phrase_shape(
            modifier_words,
            CREATE_IN_ADDITION_TO_THEIR_TYPES_WORDS.len(),
            CREATE_IN_ADDITION_TO_THEIR_TYPES_PATTERN,
        )
    });
    if let Some(addition_idx) = addition_idx {
        let descriptor_words = &modifier_words[..addition_idx];
        for word in descriptor_words {
            if let Some(card_type) = parse_card_type(word) {
                push_unique_card_type(&mut added_card_types, card_type);
            }
            if let Some(subtype) = parse_subtype_flexible(word) {
                push_unique_subtype(&mut added_subtypes, subtype);
            }
        }
    } else {
        if CREATE_IDENTITY_CLAUSE_PREFIX_PATTERN.matches_words(modifier_words) {
            let descriptor_end = CREATE_DESCRIPTOR_END_WORD_PATTERN
                .find_word(modifier_words)
                .unwrap_or(modifier_words.len());
            let descriptor_words = &modifier_words[..descriptor_end];
            let mut colors = ColorSet::new();
            let mut card_types = Vec::new();
            let mut subtypes = Vec::new();
            for word in descriptor_words {
                if is_article(word)
                    || matches!(
                        *word,
                        "its"
                            | "it"
                            | "is"
                            | "s"
                            | "it's"
                            | "it’s"
                            | "they"
                            | "are"
                            | "re"
                            | "theyre"
                            | "they're"
                            | "they’re"
                    )
                    || looks_like_pt_word(word)
                {
                    continue;
                }
                if let Some(color) = parse_color(word) {
                    colors = colors.union(color);
                }
                if let Some(card_type) = parse_card_type(word) {
                    push_unique_card_type(&mut card_types, card_type);
                }
                if let Some(subtype) = parse_subtype_flexible(word) {
                    push_unique_subtype(&mut subtypes, subtype);
                }
            }
            if !colors.is_empty() {
                set_colors = Some(colors);
            }
            if !card_types.is_empty() {
                set_card_types = Some(card_types);
            }
            if !subtypes.is_empty() {
                set_subtypes = Some(subtypes);
            }
        }
    }

    Ok((
        set_colors,
        set_card_types,
        set_subtypes,
        added_card_types,
        added_subtypes,
        removed_supertypes,
        set_base_power_toughness,
        granted_abilities,
    ))
}

pub(crate) fn parse_next_end_step_token_delay_flags(tail_words: &[&str]) -> (bool, bool) {
    if !CREATE_BEGINNING_NEXT_END_STEP_PATTERN.matches_words(tail_words) {
        return (false, false);
    }

    let has_sacrifice_reference = CREATE_SACRIFICE_MARKER_PATTERN.matches_words(tail_words)
        && CREATE_DELAY_REFERENCE_MARKER_PATTERN.matches_words(tail_words);
    let has_exile_reference = CREATE_EXILE_MARKER_PATTERN.matches_words(tail_words)
        && CREATE_DELAY_REFERENCE_MARKER_PATTERN.matches_words(tail_words);

    (has_sacrifice_reference, has_exile_reference)
}

pub(crate) fn trailing_create_at_next_end_step_clause(
    tail_words: &[&str],
) -> Option<(usize, PlayerFilter)> {
    let suffixes: &[(&[&str], PlayerFilter)] = &[
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "end",
                "step",
            ],
            PlayerFilter::You,
        ),
        (
            &["at", "the", "beginning", "of", "the", "next", "end", "step"],
            PlayerFilter::Any,
        ),
        (
            &["at", "the", "beginning", "of", "next", "end", "step"],
            PlayerFilter::Any,
        ),
        (
            &["at", "the", "beginning", "of", "the", "end", "step"],
            PlayerFilter::Any,
        ),
        (
            &["at", "the", "beginning", "of", "end", "step"],
            PlayerFilter::Any,
        ),
    ];

    for (suffix, player) in suffixes {
        if tail_words.len() < suffix.len() {
            continue;
        }
        let start = tail_words.len() - suffix.len();
        if tail_words[start..] != **suffix {
            continue;
        }
        if tail_words[..start]
            .iter()
            .any(|word| CREATE_WHEN_OR_WHENEVER_WORD_PATTERN.matches_word(word))
        {
            continue;
        }
        return Some((start, player.clone()));
    }

    None
}

pub(crate) fn split_copy_source_tail_modifiers(
    source_tokens: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, bool, bool) {
    let mut split_idx: Option<usize> = None;
    for idx in 0..source_tokens.len() {
        if !CREATE_AND_WORD_PATTERN.matches_token(&source_tokens[idx]) {
            continue;
        }
        let tail_tokens = trim_commas(&source_tokens[idx + 1..]);
        let tail_words = token_word_refs(&tail_tokens);
        if tail_words.is_empty() {
            continue;
        }
        let starts_reference = tail_words
            .first()
            .is_some_and(|word| CREATE_INLINE_REFERENCE_START_PATTERN.matches_word(word));
        if !starts_reference {
            continue;
        }
        if !CREATE_TAPPED_MARKER_PATTERN.matches_words(&tail_words)
            && !CREATE_ATTACKING_MARKER_PATTERN.matches_words(&tail_words)
        {
            continue;
        }
        split_idx = Some(idx);
        break;
    }

    let Some(split_idx) = split_idx else {
        return (source_tokens.to_vec(), false, false);
    };

    let modifier_tokens = trim_commas(&source_tokens[split_idx + 1..]);
    let modifier_words = token_word_refs(&modifier_tokens);
    let enters_tapped = CREATE_TAPPED_MARKER_PATTERN.matches_words(&modifier_words);
    let enters_attacking = CREATE_ATTACKING_MARKER_PATTERN.matches_words(&modifier_words);
    let source_tokens = trim_commas(&source_tokens[..split_idx]).to_vec();
    (source_tokens, enters_tapped, enters_attacking)
}

pub(crate) fn split_copy_source_inline_combat_modifiers(
    source_tokens: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, bool, bool, Option<PlayerAst>) {
    let source_words = token_word_refs(source_tokens);
    let modifier_start_word_idx = source_words.iter().enumerate().find_map(|(idx, _)| {
        CREATE_INLINE_MODIFIER_START_PATTERN
            .matches_words(&source_words[idx..])
            .then_some(idx)
    });

    let Some(modifier_start_word_idx) = modifier_start_word_idx else {
        return (source_tokens.to_vec(), false, false, None);
    };

    let modifier_words = &source_words[modifier_start_word_idx..];
    let enters_tapped = CREATE_TAPPED_MARKER_PATTERN.matches_words(modifier_words);
    let enters_attacking = CREATE_ATTACKING_MARKER_PATTERN.matches_words(modifier_words);
    if !enters_tapped && !enters_attacking {
        return (source_tokens.to_vec(), false, false, None);
    }

    let attack_target_player_or_planeswalker_controlled_by =
        CREATE_ATTACK_TARGET_PLAYER_OR_PLANESWALKER_PATTERN
            .matches_words(modifier_words)
            .then_some(PlayerAst::That);

    let Some(modifier_start_token_idx) =
        token_index_for_word_index(source_tokens, modifier_start_word_idx)
    else {
        return (
            source_tokens.to_vec(),
            enters_tapped,
            enters_attacking,
            attack_target_player_or_planeswalker_controlled_by,
        );
    };
    let source_tokens = trim_commas(&source_tokens[..modifier_start_token_idx]).to_vec();
    (
        source_tokens,
        enters_tapped,
        enters_attacking,
        attack_target_player_or_planeswalker_controlled_by,
    )
}

fn parse_create_equal_to_dynamic_count(
    tail_tokens: &[OwnedLexToken],
) -> Result<Option<(Value, usize)>, CardTextError> {
    let tail_words = token_word_refs(tail_tokens);
    let Some(equal_word_idx) = create_find_phrase_shape(&tail_words, 2, CREATE_EQUAL_TO_PATTERN)
    else {
        return Ok(None);
    };
    let Some(equal_token_idx) = token_index_for_word_index(tail_tokens, equal_word_idx) else {
        return Ok(None);
    };
    let Some(value_token_idx) = token_index_for_word_index(tail_tokens, equal_word_idx + 2) else {
        return Ok(None);
    };
    let value_tokens = trim_commas(&tail_tokens[value_token_idx..]);
    if value_tokens.is_empty() {
        return Ok(None);
    }

    let synthetic = format!("where x is {}", render_token_slice(&value_tokens));
    let synthetic_tokens = super::super::lexer::lex_line(synthetic.as_str(), 0)?;
    Ok(parse_value_binding_clause(&synthetic_tokens).map(|value| {
        (
            value.with_surface_hint(ValueSurfaceHint::EqualTo),
            equal_token_idx,
        )
    }))
}

pub(crate) fn parse_create(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let mut player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let clause_words = token_word_refs(tokens);
    let mut idx = 0;
    let mut count_value = Value::Fixed(1);
    let mut needs_equal_to_dynamic_count = false;
    if token_slice_first_is(tokens, "that") && token_slice_at_is(tokens, 1, "many") {
        count_value = Value::EventValue(EventValueSpec::Amount);
        idx = 2;
    } else if grammar::words_match_any_prefix(
        tokens,
        &[&["a", "number", "of"], &["the", "number", "of"]],
    )
    .is_some()
    {
        needs_equal_to_dynamic_count = true;
        idx = 3;
    } else if token_slice_first_is(tokens, "x") {
        count_value = Value::X;
        idx = 1;
    } else if let Some((parsed_count, used)) = parse_number(tokens) {
        count_value = Value::Fixed(parsed_count as i32);
        idx = used;
    }

    if tokens
        .get(idx)
        .is_some_and(|token| CREATE_ARTICLE_WORD_PATTERN.matches_token(token))
    {
        idx += 1;
    }

    let remaining_words = token_word_refs(&tokens[idx..]);
    let token_idx = CREATE_TOKEN_OR_TOKENS_WORD_PATTERN
        .find_word(&remaining_words)
        .ok_or_else(|| CardTextError::ParseError("create clause missing token".to_string()))?;

    let mut name_words =
        crate::runtime_backend::util::non_article_word_refs(&remaining_words[..token_idx]);
    let mut tail_tokens = tokens[idx + token_idx + 1..].to_vec();
    if needs_equal_to_dynamic_count {
        let Some((dynamic_count, equal_token_idx)) =
            parse_create_equal_to_dynamic_count(&tail_tokens)?
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic token count in create clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        count_value = dynamic_count;
        tail_tokens.truncate(equal_token_idx);
    }
    let mut delayed_create_player = None;
    let initial_tail_words = token_word_refs(&tail_tokens);
    if let Some((clause_start, player)) =
        trailing_create_at_next_end_step_clause(&initial_tail_words)
    {
        delayed_create_player = Some(player);
        if let Some(cut_idx) = token_index_for_word_index(&tail_tokens, clause_start) {
            tail_tokens.truncate(cut_idx);
        }
    }
    let mut attached_to_target: Option<TargetAst> = None;
    let pre_attach_tail_words = token_word_refs(&tail_tokens);
    let pre_attach_for_each_idx = create_find_phrase_shape(
        &pre_attach_tail_words,
        CREATE_FOR_EACH_WORDS.len(),
        CREATE_FOR_EACH_PATTERN,
    );
    if let Some(attached_word_idx) = CREATE_ATTACHED_WORD_PATTERN.find_word(&pre_attach_tail_words)
        && CREATE_TO_PREFIX_PATTERN.matches_words(&pre_attach_tail_words[attached_word_idx + 1..])
        && (pre_attach_for_each_idx.is_none()
            || pre_attach_for_each_idx.is_some_and(|for_each_idx| attached_word_idx < for_each_idx))
        && let Some(attached_token_idx) =
            token_index_for_word_index(&tail_tokens, attached_word_idx)
    {
        let target_tokens = trim_commas(&tail_tokens[attached_token_idx + 2..]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing attachment target in create clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        attached_to_target = Some(parse_target_phrase(&target_tokens)?);
        tail_tokens.truncate(attached_token_idx);
    }
    let tail_words = token_word_refs(&tail_tokens);
    if attached_to_target.is_some()
        && CREATE_COPY_OR_COPIES_MARKER_PATTERN.matches_words(&tail_words)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported aura-copy attachment fanout clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let with_idx = CREATE_WITH_WORD_PATTERN.find_word(&tail_words);
    let raw_for_each_idx = create_find_phrase_shape(
        &tail_words,
        CREATE_FOR_EACH_WORDS.len(),
        CREATE_FOR_EACH_PATTERN,
    );
    let for_each_idx = raw_for_each_idx.filter(|idx| {
        let prefix_words = &tail_words[..*idx];
        let looks_like_token_rules_text = CREATE_TOKEN_RULES_TEXT_PREFIX_PATTERN
            .matches_words(prefix_words)
            || (CREATE_TOKEN_MARKER_PATTERN.matches_words(prefix_words)
                && CREATE_GRANT_VERB_MARKER_PATTERN.matches_words(prefix_words));
        if looks_like_token_rules_text {
            return false;
        }

        let Some(with_idx) = with_idx else {
            return true;
        };
        if with_idx >= *idx {
            return true;
        }
        let between_with_and_for_each = &tail_words[with_idx + 1..*idx];
        let has_rules_text_hint = between_with_and_for_each.iter().any(|word| {
            matches!(
                *word,
                "this"
                    | "that"
                    | "it"
                    | "token"
                    | "tokens"
                    | "gets"
                    | "get"
                    | "gains"
                    | "gain"
                    | "has"
                    | "have"
                    | "when"
                    | "whenever"
                    | "at"
                    | "sacrifice"
                    | "draw"
                    | "add"
                    | "deals"
                    | "deal"
                    | "counter"
                    | "counters"
            )
        });
        !has_rules_text_hint
    });
    let mut for_each_dynamic_count: Option<Value> = None;
    let mut for_each_object_filter: Option<ObjectFilter> = None;
    let mut for_each_player_condition: Option<(PlayerFilter, PredicateAst)> = None;
    if let Some(for_each_idx) = for_each_idx {
        let filter_tokens = &tail_tokens[for_each_idx + 2..];
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'for each' in create clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if let Some(parsed) = parse_create_for_each_player_condition(filter_tokens, &clause_words)?
        {
            for_each_player_condition = Some(parsed);
            if player == PlayerAst::Implicit {
                player = PlayerAst::You;
            }
        } else if let Some(dynamic) = parse_create_for_each_dynamic_count(filter_tokens) {
            for_each_dynamic_count = Some(dynamic.with_surface_hint(ValueSurfaceHint::ForEach));
        } else {
            reject_lossy_for_each_fallback(filter_tokens, &clause_words)?;
            let filter = parse_object_filter(filter_tokens, false)?;
            for_each_object_filter = Some(filter);
        }
    }
    let resolve_create_count = |references_iterated_object: bool| {
        if let Some(dynamic) = for_each_dynamic_count.clone() {
            return dynamic;
        }
        if let Some(filter) = for_each_object_filter.clone() {
            if references_iterated_object {
                return count_value.clone();
            }
            return Value::Count(filter);
        }
        count_value.clone()
    };
    let wrap_for_each_when_needed = |effect: EffectAst, references_iterated_object: bool| {
        if references_iterated_object && let Some(filter) = for_each_object_filter.clone() {
            EffectAst::ForEachObject {
                filter,
                effects: vec![effect],
            }
        } else {
            effect
        }
    };
    let wrap_for_each_player_condition = |effect: EffectAst| {
        if let Some((filter, predicate)) = &for_each_player_condition {
            let effects = vec![EffectAst::Conditional {
                predicate: predicate.clone(),
                if_true: vec![effect],
                if_false: Vec::new(),
            }];
            match filter {
                PlayerFilter::Opponent => EffectAst::ForEachOpponent { effects },
                PlayerFilter::Any => EffectAst::ForEachPlayer { effects },
                other => EffectAst::ForEachPlayersFiltered {
                    filter: other.clone(),
                    effects,
                },
            }
        } else {
            effect
        }
    };
    let wrap_delayed_create = |effect: EffectAst| {
        if let Some(player) = delayed_create_player {
            EffectAst::DelayedUntilNextEndStep {
                player,
                effects: vec![effect],
            }
        } else {
            effect
        }
    };
    let mut tapped = false;
    let mut attacking = false;
    let mut modifier_tail_words = tail_words.clone();
    let mut raw_name_override: Option<String> = None;
    let mut rules_text_range: Option<(usize, usize)> = None;
    if let Some(named_idx) = CREATE_NAMED_WORD_PATTERN.find_word(&tail_words) {
        let range_end = for_each_idx.unwrap_or(tail_words.len());
        if named_idx + 1 < range_end {
            let after_named = &tail_words[named_idx + 1..range_end];
            let name_end = CREATE_NAME_END_WORD_PATTERN
                .find_word(after_named)
                .map(|offset| named_idx + 1 + offset)
                .unwrap_or(range_end);
            if named_idx + 1 < name_end {
                name_words.push("named");
                name_words.extend(tail_words[named_idx + 1..name_end].iter().copied());
            }
        }
    }
    name_words.retain(|word| {
        if CREATE_TAPPED_MARKER_PATTERN.matches_word(word) {
            tapped = true;
            return false;
        }
        if CREATE_ATTACKING_MARKER_PATTERN.matches_word(word) {
            attacking = true;
            return false;
        }
        true
    });
    name_words.retain(|word| !CREATE_AND_OR_OR_WORD_PATTERN.matches_word(word));
    let name_words_primary_len = name_words.len();
    if name_words.is_empty() {
        if tail_words
            .iter()
            .any(|word| CREATE_COPY_OR_COPIES_WORD_PATTERN.matches_word(word))
        {
            let (
                set_colors,
                set_card_types,
                set_subtypes,
                added_card_types,
                added_subtypes,
                removed_supertypes,
                set_base_power_toughness,
                granted_abilities,
            ) = parse_copy_modifiers_from_tail(&tail_words)?;
            let half_pt = grammar::contains_word(&tail_tokens, "half")
                && grammar::contains_word(&tail_tokens, "power")
                && grammar::contains_word(&tail_tokens, "toughness");
            let has_haste = CREATE_HASTE_GRANT_MARKER_PATTERN.matches_words(&tail_words)
                || grammar::contains_word(&tail_tokens, "haste");
            let token_modifier_words = tail_words
                .iter()
                .position(|word| *word == "token" || *word == "tokens")
                .map(|idx| &tail_words[..idx])
                .unwrap_or(&[]);
            let copy_modifier_words = tail_words
                .iter()
                .position(|word| CREATE_COPY_OR_COPIES_WORD_PATTERN.matches_word(word))
                .map(|idx| &tail_words[..idx])
                .unwrap_or(&[]);
            let mut enters_tapped = tapped
                || CREATE_TAPPED_MARKER_PATTERN.matches_words(token_modifier_words)
                || CREATE_TAPPED_MARKER_PATTERN.matches_words(copy_modifier_words);
            let mut enters_attacking = attacking
                || CREATE_ATTACKING_MARKER_PATTERN.matches_words(token_modifier_words)
                || CREATE_ATTACKING_MARKER_PATTERN.matches_words(copy_modifier_words);
            let mut attack_target_player_or_planeswalker_controlled_by = None;
            if player == PlayerAst::Implicit {
                player = PlayerAst::You;
            }
            let (sacrifice_at_next_end_step, exile_at_next_end_step) =
                parse_next_end_step_token_delay_flags(&tail_words);
            if let Some(of_idx) = find_token_index(&tail_tokens, |token| {
                CREATE_OF_WORD_PATTERN.matches_token(token)
            }) {
                let source_tokens = &tail_tokens[of_idx + 1..];
                let source_end = find_token_index(source_tokens, |token| {
                    token.is_comma() || CREATE_EXCEPT_WORD_PATTERN.matches_token(token)
                })
                .unwrap_or(source_tokens.len());
                let mut source_end = source_end;
                for idx in 1..source_end {
                    if starts_with_inline_token_rules_tail(&source_tokens[idx..])
                        || (CREATE_AND_WORD_PATTERN.matches_token(&source_tokens[idx])
                            && starts_with_inline_token_rules_tail(&source_tokens[idx + 1..]))
                    {
                        source_end = idx;
                        break;
                    }
                }
                let source_tokens = &source_tokens[..source_end];
                let (source_tokens, tail_tapped, tail_attacking) =
                    split_copy_source_tail_modifiers(source_tokens);
                let (source_tokens, inline_tapped, inline_attacking, inline_attack_target_player) =
                    split_copy_source_inline_combat_modifiers(&source_tokens);
                enters_tapped = tail_tapped || inline_tapped;
                enters_attacking = tail_attacking || inline_attacking;
                attack_target_player_or_planeswalker_controlled_by = inline_attack_target_player;
                if !source_tokens.is_empty() {
                    if let Some(token_word_idx) = clause_words
                        .iter()
                        .position(|word| *word == "token" || *word == "tokens")
                    {
                        let token_prefix = &clause_words[..token_word_idx];
                        enters_tapped |= CREATE_TAPPED_MARKER_PATTERN.matches_words(token_prefix);
                        enters_attacking |=
                            CREATE_ATTACKING_MARKER_PATTERN.matches_words(token_prefix);
                    }
                    let source = parse_target_phrase(&source_tokens)?;
                    let references_iterated_object = target_references_it(&source);
                    let create = EffectAst::subject_verb(
                        SubjectVerbRoleAst::Actor,
                        player,
                        SubjectVerbActionAst::CreateTokenCopyFromSource {
                            source,
                            count: resolve_create_count(references_iterated_object),
                            player,
                            enters_tapped,
                            enters_attacking,
                            attack_target_player_or_planeswalker_controlled_by,
                            half_power_toughness_round_up: half_pt,
                            has_haste,
                            exile_at_end_of_combat: false,
                            sacrifice_at_next_end_step,
                            exile_at_next_end_step,
                            set_colors,
                            set_card_types,
                            set_subtypes,
                            added_card_types,
                            added_subtypes,
                            removed_supertypes,
                            set_base_power_toughness,
                            granted_abilities,
                        },
                    );
                    return Ok(wrap_for_each_player_condition(wrap_delayed_create(
                        wrap_for_each_when_needed(create, references_iterated_object),
                    )));
                }
            }
            let references_iterated_object = true;
            let create = EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                player,
                SubjectVerbActionAst::CreateTokenCopy {
                    object: ObjectRefAst::Tagged(TagKey::from(IT_TAG)),
                    count: resolve_create_count(references_iterated_object),
                    player,
                    enters_tapped,
                    enters_attacking,
                    attack_target_player_or_planeswalker_controlled_by,
                    half_power_toughness_round_up: half_pt,
                    has_haste,
                    exile_at_end_of_combat: false,
                    sacrifice_at_next_end_step,
                    exile_at_next_end_step,
                    set_colors,
                    set_card_types,
                    set_subtypes,
                    added_card_types,
                    added_subtypes,
                    removed_supertypes,
                    set_base_power_toughness,
                    granted_abilities,
                },
            );
            return Ok(wrap_for_each_player_condition(wrap_delayed_create(
                wrap_for_each_when_needed(create, references_iterated_object),
            )));
        }
        return Err(CardTextError::ParseError(
            "create clause missing token name".to_string(),
        ));
    }
    if let Some(with_idx) = CREATE_WITH_WORD_PATTERN.find_word(&tail_words) {
        let with_tail_end = for_each_idx.unwrap_or(tail_words.len());
        if with_idx + 1 < with_tail_end {
            let with_words = &tail_words[with_idx + 1..with_tail_end];
            let has_equipment_rules_subject =
                CREATE_EQUIPMENT_RULES_SUBJECT_PATTERN.matches_words(with_words);
            let rules_text_start = CREATE_RULES_TEXT_START_WORD_PATTERN.find_word(with_words);
            let mut include_end = rules_text_start.unwrap_or(with_words.len());
            if include_end > 0
                && let Some(named_pos) =
                    CREATE_NAMED_WORD_PATTERN.find_word(&with_words[..include_end])
            {
                include_end = named_pos;
            }
            let preserve_rules_tail = rules_text_start
                .is_some_and(|start| start < with_words.len())
                && with_words[include_end..]
                    .iter()
                    .any(|word| CREATE_PRESERVE_RULES_TAIL_WORD_PATTERN.matches_word(word));
            let preserve_rules_tail = preserve_rules_tail || has_equipment_rules_subject;
            if preserve_rules_tail {
                let start = with_idx + 1 + include_end;
                if start < with_tail_end {
                    rules_text_range = Some((start, with_tail_end));
                }
                let raw_tail_start = find_token_index(&tail_tokens, |token| {
                    CREATE_WITH_WORD_PATTERN.matches_token(token)
                })
                .unwrap_or(with_idx.min(tail_tokens.len()));
                let raw_tail_end = if let Some(for_each_idx) = for_each_idx {
                    token_index_for_word_index(&tail_tokens, for_each_idx)
                        .unwrap_or(tail_tokens.len())
                } else {
                    tail_tokens.len()
                };
                let raw_tail = render_token_slice(&tail_tokens[raw_tail_start..raw_tail_end])
                    .trim()
                    .to_string();
                let prefix = normalize_token_name(&name_words);
                raw_name_override = Some(if prefix.is_empty() {
                    raw_tail
                } else {
                    format!("{prefix} {raw_tail}")
                });
            }
            if include_end > 0 {
                name_words.extend(with_words[..include_end].iter().copied());
                if preserve_rules_tail {
                    // Keep quoted token rules text tails so token lowering can
                    // reconstruct granted abilities instead of dropping them.
                    name_words.extend(with_words[include_end..].iter().copied());
                }
            } else {
                // Preserve quoted token rules text so token compilation can
                // attach the ability to the created token definition.
                name_words.extend(with_words.iter().copied());
            }
        }
    }
    let mut dynamic_power_toughness = None;
    if let Some(pt_idx) = CREATE_PT_WORD_PATTERN
        .find_word(&name_words)
        .or_else(|| find_token_index(&name_words, |word| looks_like_pt_word(word)))
        && pt_idx < name_words_primary_len
    {
        if CREATE_PT_WORD_PATTERN.matches_word(name_words[pt_idx]) {
            dynamic_power_toughness = Some((Value::X, Value::X));
            name_words[pt_idx] = "0/0";
        }
        let prefix_words = &name_words[..pt_idx];
        let keep_prefix = CREATE_NOT_LEGENDARY_PATTERN.matches_words(prefix_words)
            || CREATE_LEGENDARY_MARKER_PATTERN.matches_words(prefix_words)
            || prefix_words
                .first()
                .is_some_and(|word| is_probable_token_name_word(word));
        if !keep_prefix {
            name_words = name_words[pt_idx..].to_vec();
        }
    }
    let name = raw_name_override.unwrap_or_else(|| normalize_token_name(&name_words));

    let grants_unblockable = CREATE_UNBLOCKABLE_RULES_PATTERN.matches_words(&tail_words);

    if let Some((start, end)) = rules_text_range {
        if start < end && end <= modifier_tail_words.len() {
            modifier_tail_words = modifier_tail_words[..start]
                .iter()
                .chain(modifier_tail_words[end..].iter())
                .copied()
                .collect();
        }
    }

    if let Some(where_word_idx) = CREATE_WHERE_WORD_PATTERN.find_word(&tail_words)
        && let Some(where_token_idx) = token_index_for_word_index(&tail_tokens, where_word_idx)
    {
        let where_value =
            parse_value_binding_clause(&tail_tokens[where_token_idx..]).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported where-x clause in create clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let where_value = where_value.with_surface_hint(ValueSurfaceHint::WhereXIs);
        if let Some((power, toughness)) = dynamic_power_toughness.as_mut() {
            if value_contains_unbound_x(power) {
                *power = where_value.clone();
            }
            if value_contains_unbound_x(toughness) {
                *toughness = where_value.clone();
            }
        }
        modifier_tail_words.truncate(where_word_idx);
    }

    tapped |= CREATE_TAPPED_MARKER_PATTERN.matches_words(&modifier_tail_words);
    attacking |= CREATE_ATTACKING_MARKER_PATTERN.matches_words(&modifier_tail_words);
    if attacking
        && matches!(player, PlayerAst::That)
        && CREATE_ATTACKING_THAT_PLAYER_PATTERN.matches_words(&modifier_tail_words)
    {
        player = PlayerAst::You;
    }
    let (sacrifice_at_next_end_step, exile_at_next_end_step) =
        parse_next_end_step_token_delay_flags(&modifier_tail_words);
    let mut granted_abilities = Vec::new();
    if CREATE_DECAYED_MARKER_PATTERN.matches_words(&modifier_tail_words) {
        granted_abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Decayed));
    }
    if grants_unblockable {
        granted_abilities.push(GrantedAbilityAst::KeywordAction(KeywordAction::Unblockable));
    }
    let references_iterated_object = attached_to_target
        .as_ref()
        .is_some_and(target_references_it);
    let create = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        player,
        SubjectVerbActionAst::CreateTokenWithMods {
            name,
            count: resolve_create_count(references_iterated_object),
            dynamic_power_toughness,
            player,
            attached_to: attached_to_target,
            tapped,
            attacking,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            granted_abilities,
        },
    );
    Ok(wrap_for_each_player_condition(wrap_delayed_create(
        wrap_for_each_when_needed(create, references_iterated_object),
    )))
}

fn parse_create_for_each_player_condition(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<(PlayerFilter, PredicateAst)>, CardTextError> {
    let (filter, who_tokens) = if let Some(rest) =
        grammar::words_match_prefix(tokens, &["opponent", "who"])
            .or_else(|| grammar::words_match_prefix(tokens, &["opponents", "who"]))
    {
        (
            PlayerFilter::Opponent,
            &tokens[tokens.len() - rest.len() - 1..],
        )
    } else if let Some(rest) = grammar::words_match_prefix(tokens, &["player", "who"])
        .or_else(|| grammar::words_match_prefix(tokens, &["players", "who"]))
    {
        (PlayerFilter::Any, &tokens[tokens.len() - rest.len() - 1..])
    } else {
        return Ok(None);
    };

    let predicate = parse_who_player_predicate_lexed(who_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported player predicate after create for-each clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    Ok(Some((filter, predicate)))
}

fn parse_create_for_each_counter_count(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = token_word_refs(tokens);
    let mut idx = 0usize;
    if words
        .get(idx)
        .is_some_and(|word| CREATE_SOURCE_COUNTER_LEADING_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }

    let counter_type = words
        .get(idx)
        .and_then(|word| parse_counter_type_word(word));
    if counter_type.is_some() {
        idx += 1;
    }

    if !words
        .get(idx)
        .is_some_and(|word| CREATE_COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word))
        || !words
            .get(idx + 1)
            .is_some_and(|word| CREATE_ON_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let reference = &words[idx + 2..];
    if CREATE_SOURCE_COUNTER_REFERENCE_PATTERN.matches_words(reference) {
        return Some(match counter_type {
            Some(counter_type) => Value::CountersOnSource(counter_type),
            None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
        });
    }

    source_reference_surface_for_words(reference).map(|surface| {
        Value::CountersOn(
            Box::new(source_choose_spec_for_surface(surface)),
            counter_type,
        )
    })
}

pub(crate) fn parse_create_for_each_dynamic_count(tokens: &[OwnedLexToken]) -> Option<Value> {
    if let Some(value) = parse_create_for_each_counter_count(tokens) {
        return Some(value.with_surface_hint(ValueSurfaceHint::ForEach));
    }

    if grammar::words_match_any_prefix(
        tokens,
        &[
            &["card", "put", "into", "a", "graveyard", "this", "way"],
            &["cards", "put", "into", "a", "graveyard", "this", "way"],
            &["object", "put", "into", "a", "graveyard", "this", "way"],
            &["objects", "put", "into", "a", "graveyard", "this", "way"],
            &["permanent", "put", "into", "a", "graveyard", "this", "way"],
            &["permanents", "put", "into", "a", "graveyard", "this", "way"],
            &["card", "put", "into", "graveyard", "this", "way"],
            &["cards", "put", "into", "graveyard", "this", "way"],
            &["object", "put", "into", "graveyard", "this", "way"],
            &["objects", "put", "into", "graveyard", "this", "way"],
            &["permanent", "put", "into", "graveyard", "this", "way"],
            &["permanents", "put", "into", "graveyard", "this", "way"],
            &["card", "exiled", "from", "their", "hand", "this", "way"],
            &["cards", "exiled", "from", "their", "hand", "this", "way"],
            &[
                "card", "exiled", "from", "his", "or", "her", "hand", "this", "way",
            ],
            &[
                "cards", "exiled", "from", "his", "or", "her", "hand", "this", "way",
            ],
        ],
    )
    .is_some()
    {
        let words = token_word_refs(tokens);
        if words.iter().any(|word| *word == "put")
            && words.windows(2).any(|window| window == ["this", "way"])
        {
            return Some(
                Value::PendingEffectMetric {
                    source: ironsmith_core::EffectMetricSource::AffectedObjects,
                    metric: ironsmith_core::EffectMetric::Count,
                }
                .with_surface_hint(ValueSurfaceHint::ForEach),
            );
        }

        let mut filter = ObjectFilter::default().in_zone(Zone::Hand);
        filter.owner = Some(PlayerFilter::IteratedPlayer);
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        return Some(Value::Count(filter).with_surface_hint(ValueSurfaceHint::ForEach));
    }
    if grammar::words_match_any_prefix(
        tokens,
        &[
            &["creature", "that", "died", "this", "turn"],
            &["creatures", "that", "died", "this", "turn"],
        ],
    )
    .is_some()
    {
        return Some(Value::CreaturesDiedThisTurn.with_surface_hint(ValueSurfaceHint::ForEach));
    }
    if grammar::words_match_prefix(tokens, &["time", "it", "regenerated", "this", "turn"]).is_some()
        || grammar::words_match_prefix(tokens, &["times", "it", "regenerated", "this", "turn"])
            .is_some()
    {
        return Some(
            Value::SourceRegeneratedThisTurnCount.with_surface_hint(ValueSurfaceHint::ForEach),
        );
    }
    let clause_words = token_word_refs(tokens);
    if CREATE_SPELL_OR_SPELLS_MARKER_PATTERN.matches_words(&clause_words)
        && CREATE_CAST_OR_CASTS_MARKER_PATTERN.matches_words(&clause_words)
        && CREATE_TURN_MARKER_PATTERN.matches_words(&clause_words)
    {
        let player = if clause_words
            .iter()
            .any(|word| CREATE_YOU_REFERENCE_WORD_PATTERN.matches_word(word))
        {
            PlayerFilter::You
        } else if clause_words
            .iter()
            .any(|word| CREATE_OPPONENT_REFERENCE_WORD_PATTERN.matches_word(word))
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::Any
        };

        let token_words = token_word_refs(tokens);
        let other_than_first = CREATE_OTHER_THAN_FIRST_MARKER_PATTERN.matches_words(&token_words);
        if other_than_first {
            return Some(
                Value::Add(
                    Box::new(Value::SpellsCastThisTurn(player)),
                    Box::new(Value::Fixed(-1)),
                )
                .with_surface_hint(ValueSurfaceHint::ForEach),
            );
        }
        if grammar::contains_word(tokens, "this") && grammar::contains_word(tokens, "turn") {
            return Some(
                Value::SpellsCastThisTurn(player).with_surface_hint(ValueSurfaceHint::ForEach),
            );
        }
    }
    if grammar::words_match_prefix(
        tokens,
        &[
            "color", "of", "mana", "spent", "to", "cast", "this", "spell",
        ],
    )
    .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["color", "of", "mana", "used", "to", "cast", "this", "spell"],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "colors", "of", "mana", "used", "to", "cast", "this", "spell",
            ],
        )
        .is_some()
    {
        return Some(
            Value::ColorsOfManaSpentToCastThisSpell.with_surface_hint(ValueSurfaceHint::ForEach),
        );
    }
    if grammar::words_match_prefix(
        tokens,
        &["basic", "land", "type", "among", "lands", "you", "control"],
    )
    .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["basic", "land", "types", "among", "lands", "you", "control"],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "basic", "land", "type", "among", "the", "lands", "you", "control",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "basic", "land", "types", "among", "the", "lands", "you", "control",
            ],
        )
        .is_some()
    {
        return Some(
            Value::BasicLandTypesAmong(ObjectFilter::land().you_control())
                .with_surface_hint(ValueSurfaceHint::ForEach),
        );
    }
    if grammar::words_match_prefix(tokens, &["card", "type", "among"]).is_some()
        || grammar::words_match_prefix(tokens, &["card", "types", "among"]).is_some()
    {
        let scope_tokens = trim_commas(LexedClause::new(tokens).from_word(3)?.tokens());
        if let Ok(filter) = parse_object_filter(&scope_tokens, false) {
            return Some(
                Value::CardTypesAmong(filter).with_surface_hint(ValueSurfaceHint::ForEach),
            );
        }
    }
    None
}

pub(crate) fn normalize_token_name(words: &[&str]) -> String {
    words.join(" ")
}

fn parse_investigate_for_each_count(tokens: &[OwnedLexToken]) -> Result<Value, CardTextError> {
    let words = token_word_refs(tokens);
    if let Some(exiled_idx) = create_find_phrase_shape(
        &words,
        CREATE_EXILED_THIS_WAY_WORDS.len(),
        CREATE_EXILED_THIS_WAY_PATTERN,
    ) {
        let clause = LexedClause::new(tokens);
        let filter_tokens = trim_commas(
            clause
                .before_word(exiled_idx)
                .unwrap_or_else(|| clause.before(tokens.len()))
                .tokens(),
        );
        let mut filter = if filter_tokens.is_empty() {
            ObjectFilter::default()
        } else {
            parse_object_filter(&filter_tokens, false)?
        };
        filter.zone = Some(Zone::Exile);
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(Value::Count(filter).with_surface_hint(ValueSurfaceHint::ForEach));
    }

    if CREATE_THIS_WAY_MARKER_PATTERN.matches_words(&words) {
        return Ok(
            Value::EventValue(EventValueSpec::Amount).with_surface_hint(ValueSurfaceHint::ForEach)
        );
    }

    if let Some(dynamic) = parse_create_for_each_dynamic_count(tokens) {
        return Ok(dynamic.with_surface_hint(ValueSurfaceHint::ForEach));
    }

    reject_lossy_for_each_fallback(tokens, &words)?;
    Ok(Value::Count(parse_object_filter(tokens, false)?)
        .with_surface_hint(ValueSurfaceHint::ForEach))
}

pub(crate) fn parse_investigate(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    if tokens.is_empty() {
        return Ok(EffectAst::subject_verb_investigate(player, Value::Fixed(1)));
    }

    if token_slice_first_is(tokens, "for") && token_slice_at_is(tokens, 1, "each") {
        let filter_tokens = &tokens[2..];
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'for each' in investigate clause (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )));
        }

        let count = parse_investigate_for_each_count(filter_tokens)?;

        return Ok(EffectAst::subject_verb_investigate(player, count));
    }

    let (mut count, used) = if let Some(first) = tokens.first().and_then(OwnedLexToken::as_word) {
        match first {
            "once" => (Value::Fixed(1), 1),
            "twice" => (Value::Fixed(2), 1),
            _ => parse_value(tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing investigate count (clause: '{}')",
                    token_word_refs(tokens).join(" ")
                ))
            })?,
        }
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing investigate count (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    };

    let trailing = trim_commas(&tokens[used..]);
    let trailing_words = token_word_refs(&trailing);
    if token_slice_first_is(&trailing, "for") && token_slice_at_is(&trailing, 1, "each") {
        let filter_tokens = &trailing[2..];
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'for each' in investigate clause (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )));
        }

        let each_count = parse_investigate_for_each_count(filter_tokens)?.into_unhinted();
        count = match (count, each_count) {
            (Value::Fixed(1), Value::Count(filter)) => {
                Value::CountScaled(filter, 1).with_surface_hint(ValueSurfaceHint::ForEach)
            }
            (Value::Fixed(1), each_count) => each_count,
            (Value::Fixed(multiplier), Value::Count(filter)) => {
                Value::CountScaled(filter, multiplier).with_surface_hint(ValueSurfaceHint::ForEach)
            }
            (multiplier, each_count) => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported scaled investigate for-each clause (count: '{multiplier:?}', each: '{each_count:?}')"
                )));
            }
        };
        return Ok(EffectAst::subject_verb_investigate(player, count));
    }

    if matches!(count, Value::X)
        && CREATE_TIME_OR_TIMES_WORD_PATTERN.matches_word_at(&trailing_words, 0)
        && let Some(where_idx) = CREATE_WHERE_WORD_PATTERN.find_word(&trailing_words)
    {
        let where_token_idx = token_index_for_word_index(&trailing, where_idx).unwrap_or(0);
        if let Some(where_count) = parse_value_binding_clause(&trailing[where_token_idx..]) {
            count = where_count;
            return Ok(EffectAst::subject_verb_investigate(player, count));
        }
    }
    let trailing_ok = trailing_words.is_empty()
        || CREATE_INVESTIGATE_TRAILING_TIME_PATTERN.matches_words(&trailing_words);
    if !trailing_ok {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing investigate clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_investigate(player, count))
}

pub(crate) fn parse_incubate(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let (mut amount, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing incubate amount (clause: '{}')",
            token_word_refs(tokens).join(" ")
        ))
    })?;
    let mut count = Value::Fixed(1);

    let mut trailing = trim_commas(&tokens[used..]).to_vec();
    let mut trailing_words = token_word_refs(&trailing);
    if CREATE_ONCE_WORD_PATTERN.matches_word_at(&trailing_words, 0) {
        count = Value::Fixed(1);
        trailing = trim_commas(&trailing[1..]).to_vec();
        trailing_words = token_word_refs(&trailing);
    } else if CREATE_TWICE_WORD_PATTERN.matches_word_at(&trailing_words, 0) {
        count = Value::Fixed(2);
        trailing = trim_commas(&trailing[1..]).to_vec();
        trailing_words = token_word_refs(&trailing);
    } else if let Some((parsed_count, count_used)) = parse_value(&trailing) {
        let count_tail = trim_commas(&trailing[count_used..]).to_vec();
        let count_tail_words = token_word_refs(&count_tail);
        if CREATE_TIME_OR_TIMES_WORD_PATTERN.matches_word_at(&count_tail_words, 0) {
            count = parsed_count;
            trailing = trim_commas(&count_tail[1..]).to_vec();
            trailing_words = token_word_refs(&trailing);
        }
    } else if CREATE_TIME_OR_TIMES_WORD_PATTERN.matches_word_at(&trailing_words, 0) {
        trailing = trim_commas(&trailing[1..]).to_vec();
        trailing_words = token_word_refs(&trailing);
    }

    if let Some(where_word_idx) = CREATE_WHERE_WORD_PATTERN.find_word(&trailing_words) {
        let where_token_idx = token_index_for_word_index(&trailing, where_word_idx).unwrap_or(0);
        let Some(where_value) = parse_value_binding_clause(&trailing[where_token_idx..]) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing incubate where clause (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )));
        };
        let where_value = where_value.with_surface_hint(ValueSurfaceHint::WhereXIs);
        if value_contains_unbound_x(&amount) {
            amount = where_value;
        } else if value_contains_unbound_x(&count) {
            count = where_value;
        } else {
            return Err(CardTextError::ParseError(format!(
                "incubate where clause did not bind X (clause: '{}')",
                token_word_refs(tokens).join(" ")
            )));
        }
        trailing = trim_commas(&trailing[..where_token_idx]).to_vec();
        trailing_words = token_word_refs(&trailing);
    }

    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing incubate clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }

    Ok(EffectAst::subject_verb_incubate(player, amount, count))
}
