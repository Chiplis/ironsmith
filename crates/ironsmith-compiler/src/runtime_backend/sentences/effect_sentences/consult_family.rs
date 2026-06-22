use winnow::Parser as _;
use winnow::combinator::{alt, cut_err, dispatch, fail, opt, peek};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::take_till;

use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::{
    LexStream, LexedClause, OwnedLexToken, TokenKind, contains_token_any_word, contains_token_word,
    find_token_kind, find_token_word, token_slice_strip_any_word_prefix,
    word_slice_contains_any_phrase, word_slice_contains_phrase, word_slice_contains_word,
    word_slice_ends_with, word_slice_eq, word_slice_eq_any, word_slice_starts_with,
};
use super::super::token_primitives::{
    find_index, find_window_by, parse_turn_duration_prefix, parse_value_comparison_tokens,
    strip_leading_if_you_do_lexed, word_view_has_prefix,
};
use super::super::util::{
    helper_tag_for_tokens, parse_number, parse_subject, strip_leading_word_refs_any,
    token_index_for_word_index, trim_commas,
};
use super::super::value_helpers::parse_value_from_lexed;
use super::dispatch_entry::{
    ConsultCastClause, ConsultCastCost, ConsultCastManaValueCondition, ConsultCastTiming,
    ConsultSentenceParts, consult_stop_rule_is_single_match, find_from_among_looked_cards_phrase,
    parse_looked_card_reveal_filter,
};
use super::search_library::normalize_search_library_filter;
use super::{find_verb, parse_effect_chain, parse_effect_sentence_lexed};
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, ObjectFilter, PlayerAst, PredicateAst, SubjectAst, TagKey,
    TargetAst,
};
use crate::effect::{EventValueSpec, Value};
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;

const CONSULT_REVEAL_OR_EXILE_WORDS: &[&str] = &["reveal", "reveals", "exile", "exiles"];
const CONSULT_REVEAL_WORDS: &[&str] = &["reveal", "reveals"];
const CONSULT_TOP_LIBRARY_PREFIX: &[&str] = &["cards", "from", "top", "of"];
const CONSULT_TOP_LIBRARY_SUFFIX: &[&str] = &["library"];
const CONSULT_THAT_MANY_PREFIX: &[&str] = &["that", "many"];
const CONSULT_THIS_POWER_CLAUSES: &[&[&str]] = &[&["thiss", "power"], &["this", "power"]];
const CONSULT_THIS_TURN_CLAUSE: &[&str] = &["this", "turn"];
const CONSULT_PAY_LIFE_MANA_VALUE_CLAUSE: &[&str] = &[
    "by", "paying", "life", "equal", "to", "the", "spell's", "mana", "value", "rather", "than",
    "paying", "its", "mana", "cost",
];
const CONSULT_NOT_CAST_THIS_MARKER_PHRASES: &[&[&str]] = &[
    &["not", "cast", "this"],
    &["were", "not", "cast", "this", "way"],
    &["werent", "cast", "this", "way"],
    &["weren't", "cast", "this", "way"],
];
const CONSULT_PUT_MATCH_INTO_HAND_CLAUSES: &[&[&str]] = &[
    &["put", "that", "card", "into", "your", "hand"],
    &["put", "the", "exiled", "card", "into", "your", "hand"],
    &["put", "it", "into", "your", "hand"],
    &[
        "put", "that", "card", "into", "your", "hand", "if", "it", "wasnt", "cast", "this", "way",
    ],
    &[
        "put", "that", "card", "into", "your", "hand", "if", "it", "wasn't", "cast", "this", "way",
    ],
    &[
        "put", "the", "exiled", "card", "into", "your", "hand", "if", "it", "wasnt", "cast",
        "this", "way",
    ],
    &[
        "put", "the", "exiled", "card", "into", "your", "hand", "if", "it", "wasn't", "cast",
        "this", "way",
    ],
    &[
        "put", "it", "into", "your", "hand", "if", "it", "wasnt", "cast", "this", "way",
    ],
    &[
        "put", "it", "into", "your", "hand", "if", "it", "wasn't", "cast", "this", "way",
    ],
];

fn is_consult_reveal_or_exile_word(word: &str) -> bool {
    CONSULT_REVEAL_OR_EXILE_WORDS
        .iter()
        .any(|expected| word.eq_ignore_ascii_case(expected))
}

fn is_consult_reveal_word(word: &str) -> bool {
    CONSULT_REVEAL_WORDS
        .iter()
        .any(|expected| word.eq_ignore_ascii_case(expected))
}

pub(crate) fn parse_exile_top_library_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let (_, count) = super::dispatch_entry::parse_prefixed_top_of_your_library_count(
        tokens,
        &[
            (&["exile", "the", "top"][..], ()),
            (&["exile", "top"][..], ()),
        ],
    )?;

    Some(vec![EffectAst::subject_verb_exile_top_of_library(
        PlayerAst::You,
        Value::Fixed(count as i32),
        Vec::new(),
        Vec::new(),
    )])
}

pub(crate) fn parse_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<ConsultSentenceParts>, CardTextError> {
    let mut sentence_tokens = trim_commas(tokens);
    sentence_tokens = trim_commas(strip_leading_if_you_do_lexed(&sentence_tokens));
    if sentence_tokens.is_empty() {
        return Ok(None);
    }

    let mut prefix_effects = Vec::new();
    let mut prefix_tokens: Vec<OwnedLexToken> = Vec::new();
    let consult_tokens = if let Some(then_idx) = find_index(&sentence_tokens, |token| {
        token
            .as_word()
            .is_some_and(|word| word.eq_ignore_ascii_case("then"))
    }) {
        if then_idx == 0 {
            trim_commas(&sentence_tokens[1..])
        } else {
            prefix_tokens = trim_commas(&sentence_tokens[..then_idx]);
            if prefix_tokens.is_empty() {
                return Ok(None);
            }
            prefix_effects = parse_exile_top_library_prefix(&prefix_tokens)
                .or_else(|| parse_effect_sentence_lexed(&prefix_tokens).ok())
                .or_else(|| parse_effect_chain(&prefix_tokens).ok())
                .unwrap_or_default();
            if prefix_effects.is_empty() {
                return Ok(None);
            }
            trim_commas(&sentence_tokens[then_idx + 1..])
        }
    } else {
        sentence_tokens
    };
    if consult_tokens.is_empty() {
        return Ok(None);
    }

    let Some(consult_verb_idx) = find_index(&consult_tokens, |token| {
        token.as_word().is_some_and(is_consult_reveal_or_exile_word)
    }) else {
        return Ok(None);
    };
    let player = if consult_verb_idx == 0 {
        infer_consult_player_from_prefix(&prefix_tokens).unwrap_or(PlayerAst::You)
    } else {
        if word_slice_eq(
            &crate::runtime_backend::token_word_refs(&consult_tokens[..consult_verb_idx]),
            &["they"],
        ) {
            PlayerAst::That
        } else {
            match parse_subject(&consult_tokens[..consult_verb_idx]) {
                SubjectAst::Player(player) => player,
                _ => return Ok(None),
            }
        }
    };
    let mode = if consult_tokens[consult_verb_idx]
        .as_word()
        .is_some_and(is_consult_reveal_word)
    {
        LibraryConsultModeAst::Reveal
    } else {
        LibraryConsultModeAst::Exile
    };

    let Some(until_idx) = find_index(&consult_tokens, |token| {
        token
            .as_word()
            .is_some_and(|word| word.eq_ignore_ascii_case("until"))
    }) else {
        return Ok(None);
    };
    if until_idx <= consult_verb_idx + 1 {
        return Ok(None);
    }

    let consult_prefix_tokens = &consult_tokens[consult_verb_idx + 1..until_idx];
    let consult_prefix_words =
        crate::runtime_backend::util::non_article_token_word_refs(consult_prefix_tokens);
    if !word_slice_starts_with(&consult_prefix_words, CONSULT_TOP_LIBRARY_PREFIX)
        || !word_slice_ends_with(&consult_prefix_words, CONSULT_TOP_LIBRARY_SUFFIX)
    {
        return Ok(None);
    }

    let mut until_tokens = trim_commas(&consult_tokens[until_idx + 1..]);
    if let Some(comma_idx) = find_token_kind(&until_tokens, TokenKind::Comma) {
        until_tokens = trim_commas(&until_tokens[..comma_idx]);
    }
    let (stop_rule, filter) = if let Some((stop_rule, filter)) =
        parse_passive_consult_stop_rule_and_filter(&until_tokens, mode)?
    {
        (stop_rule, filter)
    } else {
        let Some(match_verb_idx) = find_index(&until_tokens, |token| {
            token.as_word().is_some_and(is_consult_reveal_or_exile_word)
        }) else {
            return Ok(None);
        };
        if match_verb_idx == 0 || match_verb_idx + 1 >= until_tokens.len() {
            return Ok(None);
        }

        let mut filter_tokens = trim_commas(&until_tokens[match_verb_idx + 1..]).to_vec();
        if filter_tokens.is_empty() {
            return Ok(None);
        }
        let stop_rule = if word_slice_starts_with(
            &crate::runtime_backend::token_word_refs(&filter_tokens),
            CONSULT_THAT_MANY_PREFIX,
        ) {
            let remaining_start = TokenWordView::new(&filter_tokens)
                .token_index_after_words(2)
                .unwrap_or(2);
            let remaining = trim_commas(&filter_tokens[remaining_start..]).to_vec();
            if remaining.is_empty() {
                return Ok(None);
            }
            filter_tokens = remaining;
            LibraryConsultStopRuleAst::MatchCount(Value::EventValue(EventValueSpec::Amount))
        } else if let Some((count, used)) = parse_number(&filter_tokens) {
            let remaining = trim_commas(&filter_tokens[used..]).to_vec();
            if remaining.is_empty() {
                return Ok(None);
            }
            filter_tokens = remaining;
            LibraryConsultStopRuleAst::MatchCount(Value::Fixed(count as i32))
        } else if let Some((value, used)) = parse_value_from_lexed(&filter_tokens) {
            let remaining = trim_commas(&filter_tokens[used..]).to_vec();
            if remaining.is_empty() {
                return Ok(None);
            }
            filter_tokens = remaining;
            LibraryConsultStopRuleAst::MatchCount(value)
        } else {
            LibraryConsultStopRuleAst::FirstMatch
        };

        let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
            filter
        } else {
            match super::super::object_filters::parse_object_filter(&filter_tokens, false) {
                Ok(filter) => filter,
                Err(_) => {
                    let Some(stripped_filter_tokens) =
                        without_consult_relative_mana_value_clause(&filter_tokens)
                    else {
                        return Ok(None);
                    };
                    match super::super::object_filters::parse_object_filter(
                        &stripped_filter_tokens,
                        false,
                    ) {
                        Ok(filter) => filter,
                        Err(_) => return Ok(None),
                    }
                }
            }
        };
        apply_consult_relative_mana_value_filter(&filter_tokens, &mut filter);
        normalize_search_library_filter(&mut filter);
        filter.zone = None;
        (stop_rule, filter)
    };

    let all_tag = helper_tag_for_tokens(
        tokens,
        match mode {
            LibraryConsultModeAst::Reveal => "revealed",
            LibraryConsultModeAst::Exile => "exiled",
        },
    );
    let match_tag = helper_tag_for_tokens(tokens, "chosen");
    let mut effects = prefix_effects;
    effects.push(EffectAst::subject_verb_consult_top_of_library(
        player,
        mode,
        filter,
        stop_rule,
        all_tag.clone(),
        match_tag.clone(),
    ));

    Ok(Some(ConsultSentenceParts {
        effects,
        player,
        all_tag,
        match_tag,
    }))
}

fn apply_consult_relative_mana_value_filter(tokens: &[OwnedLexToken], filter: &mut ObjectFilter) {
    let has_lesser_mana_value = contains_token_any_word(tokens, &["lesser", "less"])
        && contains_token_word(tokens, "mana")
        && contains_token_word(tokens, "value");
    if !has_lesser_mana_value {
        return;
    }
    if filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            TaggedOpbjectRelation::ManaValueLtTagged
        )
    }) {
        return;
    }

    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("sacrificed_0"),
        relation: TaggedOpbjectRelation::ManaValueLtTagged,
    });
}

fn without_consult_relative_mana_value_clause(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let start = find_token_word(tokens, "with")?;
    let tail = &tokens[start..];
    let has_lesser_mana_value = contains_token_any_word(tail, &["lesser", "less"])
        && contains_token_word(tail, "mana")
        && contains_token_word(tail, "value");
    if !has_lesser_mana_value {
        return None;
    }
    Some(trim_commas(&tokens[..start]).to_vec())
}

fn parse_passive_consult_stop_rule_and_filter(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Result<Option<(LibraryConsultStopRuleAst, ObjectFilter)>, CardTextError> {
    let tokens = trim_commas(tokens);
    let Some((count, used)) = parse_number(&tokens)
        .map(|(count, used)| (Value::Fixed(count as i32), used))
        .or_else(|| parse_value_from_lexed(&tokens))
        .or_else(|| {
            TokenWordView::new(&tokens)
                .word_refs()
                .first()
                .is_some_and(|word| matches!(*word, "a" | "an"))
                .then_some((Value::Fixed(1), 1))
        })
    else {
        return Ok(None);
    };

    let tail_tokens = trim_commas(&tokens[used..]);
    let tail_words = TokenWordView::new(&tail_tokens);
    let tail_word_refs = tail_words.word_refs();
    let passive_suffix = match mode {
        LibraryConsultModeAst::Reveal => ["cards", "are", "revealed"],
        LibraryConsultModeAst::Exile => ["cards", "are", "exiled"],
    };
    let singular_suffix = match mode {
        LibraryConsultModeAst::Reveal => ["card", "is", "revealed"],
        LibraryConsultModeAst::Exile => ["card", "is", "exiled"],
    };
    let bare_singular_suffix = match mode {
        LibraryConsultModeAst::Reveal => ["is", "revealed"],
        LibraryConsultModeAst::Exile => ["is", "exiled"],
    };
    let suffix_len = if tail_word_refs.as_slice().ends_with(&passive_suffix) {
        passive_suffix.len()
    } else if tail_word_refs.as_slice().ends_with(&singular_suffix) {
        singular_suffix.len()
    } else if tail_word_refs.as_slice().ends_with(&bare_singular_suffix) {
        bare_singular_suffix.len()
    } else {
        return Ok(None);
    };

    let filter_word_count = tail_words.len().saturating_sub(suffix_len);
    let filter_end = tail_words
        .token_index_after_words(filter_word_count)
        .unwrap_or(tail_tokens.len());
    let filter_tokens = trim_commas(&tail_tokens[..filter_end]).to_vec();
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else if filter_tokens.is_empty() {
        ObjectFilter::default()
    } else {
        match super::super::object_filters::parse_object_filter(&filter_tokens, false) {
            Ok(filter) => filter,
            Err(_) => return Ok(None),
        }
    };
    normalize_search_library_filter(&mut filter);
    apply_consult_relative_mana_value_filter(&filter_tokens, &mut filter);
    filter.zone = None;

    Ok(Some((LibraryConsultStopRuleAst::MatchCount(count), filter)))
}

fn infer_consult_player_from_prefix(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let prefix_tokens = trim_commas(tokens);
    let (_, verb_idx) = find_verb(&prefix_tokens)?;
    match parse_subject(&prefix_tokens[..verb_idx]) {
        SubjectAst::Player(player) => Some(player),
        _ => None,
    }
}

pub(crate) fn parse_consult_remainder_order(words: &[&str]) -> Option<LibraryBottomOrderAst> {
    if !word_slice_contains_word(words, "bottom") || !word_slice_contains_word(words, "library") {
        return None;
    }
    if word_slice_contains_phrase(words, &["random", "order"]) {
        return Some(LibraryBottomOrderAst::Random);
    }
    if word_slice_contains_phrase(words, &["any", "order"]) {
        return Some(LibraryBottomOrderAst::ChooserChooses);
    }
    None
}

pub(crate) fn parse_consult_condition_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    if word_slice_eq_any(
        &crate::runtime_backend::token_word_refs(tokens),
        CONSULT_THIS_POWER_CLAUSES,
    ) {
        return Some(Value::SourcePower);
    }

    if let Some((value, used)) = parse_value_from_lexed(tokens)
        && TokenWordView::new(&tokens[used..]).is_empty()
    {
        return Some(value);
    }

    let filter_start_word_idx = if word_view_has_prefix(&word_view, &["the", "number", "of"]) {
        Some(3usize)
    } else if word_view_has_prefix(&word_view, &["number", "of"]) {
        Some(2usize)
    } else {
        None
    }?;
    if filter_start_word_idx >= word_view.len() {
        return None;
    }

    let filter_start_token_idx = word_view.token_index_for_word_index(filter_start_word_idx)?;
    let filter_tokens = &tokens[filter_start_token_idx..];
    let filter = super::super::object_filters::parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

fn take_remaining_clause_tokens<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [OwnedLexToken], ErrMode<ContextError>> {
    take_till(0.., |_token: &OwnedLexToken| false).parse_next(input)
}

fn parse_face_down_search_cast_mana_value_gate_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(crate::effect::ValueComparisonOperator, Value), ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "you" => (
            alt((
                grammar::phrase(&["you", "may", "cast", "the", "exiled", "card"]),
                grammar::phrase(&["you", "may", "cast", "that", "card"]),
                grammar::phrase(&["you", "may", "cast", "it"]),
            )),
            cut_err(grammar::phrase(&["without", "paying", "its", "mana", "cost"])),
            cut_err(|input: &mut LexStream<'a>| {
                let condition_tokens = take_remaining_clause_tokens(input)?;
                let condition = parse_consult_mana_value_condition_tokens(condition_tokens)
                    .ok_or_else(|| {
                        grammar::cut_err_ctx(
                            "mana value condition",
                            "supported mana value condition",
                        )
                    })?;
                Ok((condition.operator, condition.right))
            }),
        )
            .map(|(_, _, parsed)| parsed),
        _ => fail::<_, (crate::effect::ValueComparisonOperator, Value), _>,
    }
    .parse_next(input)
}

fn parse_bargained_face_down_cast_mana_value_gate_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<(crate::effect::ValueComparisonOperator, Value), ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "if" => (
            grammar::phrase(&["if", "this", "spell", "was", "bargained"]),
            opt(grammar::comma()),
            cut_err(parse_face_down_search_cast_mana_value_gate_inner),
        )
            .map(|(_, _, parsed)| parsed),
        _ => fail::<_, (crate::effect::ValueComparisonOperator, Value), _>,
    }
    .parse_next(input)
}

pub(crate) fn parse_bargained_face_down_cast_mana_value_gate(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::ValueComparisonOperator, Value)>, CardTextError> {
    grammar::parse_all_or_none(
        tokens,
        parse_bargained_face_down_cast_mana_value_gate_inner,
        "bargained face-down cast clause",
    )
}

fn parse_if_you_dont_remainder_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [OwnedLexToken], ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "if" => (
            alt((
                grammar::phrase(&["if", "you", "dont"]),
                grammar::phrase(&["if", "you", "don't"]),
                grammar::phrase(&["if", "you", "do", "not"]),
            ))
            .context(StrContext::Label("if-you-don't prefix"))
            .context(StrContext::Expected(StrContextValue::Description(
                "if you don't",
            ))),
            cut_err(grammar::comma())
                .context(StrContext::Label("if-you-don't separator"))
                .context(StrContext::Expected(StrContextValue::Description(
                    "comma after if-you-don't clause",
                ))),
            cut_err(take_remaining_clause_tokens),
        )
            .map(|(_, _, remainder)| remainder),
        _ => fail::<_, &'a [OwnedLexToken], _>,
    }
    .parse_next(input)
}

fn parse_if_you_cant_remainder_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [OwnedLexToken], ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "if" => (
            alt((
                grammar::phrase(&["if", "you", "cant"]),
                grammar::phrase(&["if", "you", "can't"]),
                grammar::phrase(&["if", "you", "cannot"]),
            ))
            .context(StrContext::Label("if-you-can't prefix"))
            .context(StrContext::Expected(StrContextValue::Description(
                "if you can't",
            ))),
            cut_err(grammar::comma())
                .context(StrContext::Label("if-you-can't separator"))
                .context(StrContext::Expected(StrContextValue::Description(
                    "comma after if-you-can't clause",
                ))),
            cut_err(take_remaining_clause_tokens),
        )
            .map(|(_, _, remainder)| remainder),
        _ => fail::<_, &'a [OwnedLexToken], _>,
    }
    .parse_next(input)
}

pub(crate) fn parse_consult_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConsultCastManaValueCondition> {
    let (.., after_prefix) = token_slice_strip_any_word_prefix(
        tokens,
        &[
            &["if", "it's", "a", "spell", "with", "mana", "value"][..],
            &[
                "if", "it's", "an", "instant", "spell", "with", "mana", "value",
            ][..],
            &["if", "its", "a", "spell", "with", "mana", "value"][..],
            &[
                "if", "its", "an", "instant", "spell", "with", "mana", "value",
            ][..],
            &["if", "it", "is", "a", "spell", "with", "mana", "value"][..],
            &[
                "if", "it", "is", "an", "instant", "spell", "with", "mana", "value",
            ][..],
            &["if", "the", "spell's", "mana", "value"][..],
            &["if", "the", "spells", "mana", "value"][..],
            &["if", "that", "spell's", "mana", "value"][..],
            &["if", "that", "spells", "mana", "value"][..],
            &["if", "its", "mana", "value"][..],
        ],
    )?;

    let (operator, right_tokens) = parse_value_comparison_tokens(after_prefix)?;
    let right = parse_consult_condition_value(right_tokens)?;
    Some(ConsultCastManaValueCondition { operator, right })
}

pub(crate) fn parse_consult_cast_clause(tokens: &[OwnedLexToken]) -> Option<ConsultCastClause> {
    let mut second_tokens = trim_commas(tokens);
    let mut timing = ConsultCastTiming::Immediate;
    if let Some((duration, remainder)) = parse_turn_duration_prefix(&second_tokens) {
        match duration {
            super::super::token_primitives::TurnDurationPhrase::UntilEndOfTurn => {
                second_tokens = trim_commas(remainder);
                timing = ConsultCastTiming::UntilEndOfTurn;
            }
            super::super::token_primitives::TurnDurationPhrase::UntilYourNextTurnEnd => {
                second_tokens = trim_commas(remainder);
                timing = ConsultCastTiming::UntilYourNextTurnEnd;
            }
            _ => {}
        }
    }

    let may_idx = find_index(&second_tokens, |token| {
        token.as_word().is_some_and(|word| word == "may")
    })?;
    if may_idx == 0 || may_idx + 1 >= second_tokens.len() {
        return None;
    }

    let caster = match parse_subject(&second_tokens[..may_idx]) {
        SubjectAst::Player(player) => player,
        _ => return None,
    };
    let tail_tokens = &second_tokens[may_idx + 1..];
    let (matched_phrase, remainder_tokens) = token_slice_strip_any_word_prefix(
        tail_tokens,
        &[
            &["cast", "that", "card"],
            &["cast", "it"],
            &["cast", "that", "exiled", "card"],
            &["cast", "the", "exiled", "card"],
            &["play", "that", "card"],
            &["play", "it"],
        ],
    )?;
    let allow_land = matched_phrase.first().is_some_and(|word| *word == "play");
    let remainder_word_view = TokenWordView::new(remainder_tokens);
    let remainder = remainder_word_view.word_refs();
    if word_slice_eq(&remainder, CONSULT_THIS_TURN_CLAUSE) {
        return Some(ConsultCastClause {
            caster,
            allow_land,
            timing: ConsultCastTiming::UntilEndOfTurn,
            cost: ConsultCastCost::Normal,
            mana_value_condition: None,
        });
    }

    if word_slice_eq(&remainder, CONSULT_PAY_LIFE_MANA_VALUE_CLAUSE) {
        return Some(ConsultCastClause {
            caster,
            allow_land,
            timing,
            cost: ConsultCastCost::PayLifeEqualToManaValue,
            mana_value_condition: None,
        });
    }

    if grammar::words_match_prefix(
        remainder_tokens,
        &["without", "paying", "its", "mana", "cost"],
    )
    .is_none()
    {
        return None;
    }

    let mana_value_condition = if remainder.len() == 5 {
        None
    } else {
        let condition_start = token_index_for_word_index(remainder_tokens, 5)?;
        let condition_tokens = &remainder_tokens[condition_start..];
        Some(parse_consult_mana_value_condition_tokens(condition_tokens)?)
    };

    Some(ConsultCastClause {
        caster,
        allow_land,
        timing,
        cost: ConsultCastCost::WithoutPayingManaCost,
        mana_value_condition,
    })
}

pub(crate) fn parse_consult_bottom_remainder_clause(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Option<LibraryBottomOrderAst> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let clause_words = strip_leading_word_refs_any(&clause_words, &["then", "and"]);

    let Some(order) = parse_consult_remainder_order(clause_words) else {
        return None;
    };
    let mode_word = match mode {
        LibraryConsultModeAst::Reveal => "revealed",
        LibraryConsultModeAst::Exile => "exiled",
    };
    if !grammar::contains_word(tokens, mode_word) {
        return None;
    }
    let mentions_cast_window = word_slice_contains_any_phrase(
        &crate::runtime_backend::token_word_refs(tokens),
        CONSULT_NOT_CAST_THIS_MARKER_PHRASES,
    );
    let mentions_remainder =
        grammar::contains_word(tokens, "rest") || grammar::contains_word(tokens, "other");

    (mentions_cast_window || mentions_remainder).then_some(order)
}

pub(crate) fn parse_if_declined_put_match_into_hand(
    tokens: &[OwnedLexToken],
    match_tag: TagKey,
) -> Option<Vec<EffectAst>> {
    let moves_to_hand = word_slice_eq_any(
        &crate::runtime_backend::token_word_refs(tokens),
        CONSULT_PUT_MATCH_INTO_HAND_CLAUSES,
    ) || super::super::grammar::primitives::words_match_prefix(
        tokens,
        &[
            "if", "you", "dont", "put", "that", "card", "into", "your", "hand",
        ],
    )
    .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "dont", "put", "the", "exiled", "card", "into", "your", "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &["if", "you", "dont", "put", "it", "into", "your", "hand"],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if",
                "you",
                "don\u{2019}t",
                "put",
                "that",
                "card",
                "into",
                "your",
                "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if",
                "you",
                "don\u{2019}t",
                "put",
                "the",
                "exiled",
                "card",
                "into",
                "your",
                "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if",
                "you",
                "don\u{2019}t",
                "put",
                "it",
                "into",
                "your",
                "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "do", "not", "put", "that", "card", "into", "your", "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "do", "not", "put", "the", "exiled", "card", "into", "your", "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "do", "not", "put", "it", "into", "your", "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "dont", "cast", "that", "card", "this", "way", "put", "it", "into",
                "your", "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "dont", "cast", "the", "exiled", "card", "this", "way", "put", "it",
                "into", "your", "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if",
                "you",
                "don\u{2019}t",
                "cast",
                "that",
                "card",
                "this",
                "way",
                "put",
                "it",
                "into",
                "your",
                "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if",
                "you",
                "don\u{2019}t",
                "cast",
                "the",
                "exiled",
                "card",
                "this",
                "way",
                "put",
                "it",
                "into",
                "your",
                "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "do", "not", "cast", "that", "card", "this", "way", "put", "it",
                "into", "your", "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "do", "not", "cast", "the", "exiled", "card", "this", "way", "put",
                "it", "into", "your", "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "dont", "cast", "it", "this", "way", "put", "it", "into", "your",
                "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if",
                "you",
                "don\u{2019}t",
                "cast",
                "it",
                "this",
                "way",
                "put",
                "it",
                "into",
                "your",
                "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "do", "not", "cast", "it", "this", "way", "put", "it", "into", "your",
                "hand",
            ],
        )
        .is_some();
    if !moves_to_hand {
        return None;
    }

    Some(vec![EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(match_tag, None),
        Zone::Hand,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    )])
}

pub(crate) fn consult_cast_effects(
    clause: &ConsultCastClause,
    match_tag: TagKey,
) -> Result<Vec<EffectAst>, CardTextError> {
    if clause.allow_land && !matches!(clause.cost, ConsultCastCost::Normal) {
        return Err(CardTextError::ParseError(
            "playing a land without paying its mana cost is unsupported".to_string(),
        ));
    }

    let mut cast_effects = match clause.cost {
        ConsultCastCost::Normal | ConsultCastCost::WithoutPayingManaCost => {
            let without_paying_mana_cost =
                matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost);
            if clause.allow_land
                || matches!(
                    clause.timing,
                    ConsultCastTiming::UntilEndOfTurn | ConsultCastTiming::UntilYourNextTurnEnd
                )
            {
                let grant = if matches!(clause.timing, ConsultCastTiming::UntilYourNextTurnEnd) {
                    EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                        match_tag.clone(),
                        clause.caster,
                        clause.allow_land,
                        false,
                    )
                } else {
                    EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                        match_tag.clone(),
                        clause.caster,
                        clause.allow_land,
                        without_paying_mana_cost,
                        false,
                    )
                };
                vec![grant]
            } else {
                vec![EffectAst::May {
                    effects: vec![EffectAst::subject_verb_cast_tagged(
                        match_tag.clone(),
                        clause.caster,
                        false,
                        false,
                        without_paying_mana_cost,
                        None,
                    )],
                }]
            }
        }
        ConsultCastCost::PayLifeEqualToManaValue => {
            if clause.allow_land {
                return Err(CardTextError::ParseError(
                    "pay-life consult cast clauses cannot allow lands".to_string(),
                ));
            }
            vec![
                EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(match_tag.clone(), clause.caster, false, false, false),
                EffectAst::subject_verb_grant_tagged_spell_alternative_cost_pay_life_by_mana_value_until_end_of_turn(match_tag.clone(), clause.caster),
            ]
        }
    };

    if let Some(condition) = &clause.mana_value_condition {
        cast_effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::ValueComparison {
                left: Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(match_tag))),
                operator: condition.operator,
                right: condition.right.clone(),
            },
            if_true: cast_effects,
            if_false: Vec::new(),
        }]
    }

    Ok(cast_effects)
}

pub(crate) fn parse_if_you_dont_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(after) = grammar::parse_all_or_none(
        tokens,
        parse_if_you_dont_remainder_inner,
        "if-you-don't clause",
    )?
    else {
        return Ok(None);
    };

    let effects = parse_effect_chain(after)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}

pub(crate) fn parse_if_you_cant_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(after) = grammar::parse_all_or_none(
        tokens,
        parse_if_you_cant_remainder_inner,
        "if-you-can't clause",
    )?
    else {
        return Ok(None);
    };

    let effects = parse_effect_chain(after)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}
