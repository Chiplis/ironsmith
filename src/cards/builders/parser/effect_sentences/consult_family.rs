use winnow::Parser as _;
use winnow::combinator::{alt, cut_err, dispatch, fail, opt, peek};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::take_till;

use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::keyword_static::parse_where_x_value_clause;
use super::super::lexer::{LexStream, OwnedLexToken};
use super::super::token_primitives::{
    find_index, find_window_by, parse_turn_duration_prefix, parse_value_comparison_tokens,
    slice_contains, slice_ends_with, slice_starts_with, strip_leading_if_you_do_lexed,
    word_view_has_prefix,
};
use super::super::util::{
    helper_tag_for_tokens, parse_number, parse_subject, token_index_for_word_index, trim_commas,
};
use super::super::value_helpers::parse_value_from_lexed;
use super::dispatch_entry::{
    consult_stop_rule_is_single_match, find_from_among_looked_cards_phrase,
    parse_looked_card_reveal_filter, ConsultCastClause, ConsultCastCost,
    ConsultCastManaValueCondition, ConsultCastTiming, ConsultSentenceParts,
};
use super::{find_verb, parse_effect_chain, parse_effect_sentence_lexed};
use super::search_library::normalize_search_library_filter;
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, SubjectAst, TagKey, TargetAst,
};
use crate::effect::Value;
use crate::zone::Zone;

pub(crate) fn parse_exile_top_library_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let (_, count) = super::dispatch_entry::parse_prefixed_top_of_your_library_count(
        tokens,
        &[
            (&["exile", "the", "top"][..], ()),
            (&["exile", "top"][..], ()),
        ],
    )?;

    Some(vec![EffectAst::ExileTopOfLibrary {
        count: Value::Fixed(count as i32),
        player: PlayerAst::You,
        tags: Vec::new(),
        accumulated_tags: Vec::new(),
    }])
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
    let consult_tokens = if let Some(then_idx) =
        find_index(&sentence_tokens, |token: &OwnedLexToken| token.is_word("then"))
    {
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
    } else {
        sentence_tokens
    };
    if consult_tokens.is_empty() {
        return Ok(None);
    }

    let Some(consult_verb_idx) = find_index(&consult_tokens, |token: &OwnedLexToken| {
        token.is_word("reveal")
            || token.is_word("reveals")
            || token.is_word("exile")
            || token.is_word("exiles")
    }) else {
        return Ok(None);
    };
    let player = if consult_verb_idx == 0 {
        infer_consult_player_from_prefix(&prefix_tokens).unwrap_or(PlayerAst::You)
    } else {
        match parse_subject(&consult_tokens[..consult_verb_idx]) {
            SubjectAst::Player(player) => player,
            _ => return Ok(None),
        }
    };
    let mode = if consult_tokens[consult_verb_idx].is_word("reveal")
        || consult_tokens[consult_verb_idx].is_word("reveals")
    {
        LibraryConsultModeAst::Reveal
    } else {
        LibraryConsultModeAst::Exile
    };

    let Some(until_idx) = find_index(&consult_tokens, |token: &OwnedLexToken| token.is_word("until"))
    else {
        return Ok(None);
    };
    if until_idx <= consult_verb_idx + 1 {
        return Ok(None);
    }

    let consult_prefix_words = TokenWordView::new(&consult_tokens[consult_verb_idx + 1..until_idx]);
    let prefix_words: Vec<&str> = consult_prefix_words
        .word_refs()
        .into_iter()
        .filter(|word| !super::super::util::is_article(word))
        .collect();
    if !slice_starts_with(&prefix_words, &["cards", "from", "top", "of"])
        || !slice_ends_with(&prefix_words, &["library"])
    {
        return Ok(None);
    }

    let until_tokens = trim_commas(&consult_tokens[until_idx + 1..]);
    let Some(match_verb_idx) = find_index(&until_tokens, |token: &OwnedLexToken| {
        token.is_word("reveal")
            || token.is_word("reveals")
            || token.is_word("exile")
            || token.is_word("exiles")
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

    let stop_rule = if let Some((count, used)) = parse_number(&filter_tokens) {
        let remaining = trim_commas(&filter_tokens[used..]).to_vec();
        if remaining.is_empty() {
            return Ok(None);
        }
        filter_tokens = remaining;
        LibraryConsultStopRuleAst::MatchCount(Value::Fixed(count as i32))
    } else {
        LibraryConsultStopRuleAst::FirstMatch
    };

    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        match super::super::object_filters::parse_object_filter(&filter_tokens, false) {
            Ok(filter) => filter,
            Err(_) => return Ok(None),
        }
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let all_tag = helper_tag_for_tokens(
        tokens,
        match mode {
            LibraryConsultModeAst::Reveal => "revealed",
            LibraryConsultModeAst::Exile => "exiled",
        },
    );
    let match_tag = helper_tag_for_tokens(tokens, "chosen");
    let mut effects = prefix_effects;
    effects.push(EffectAst::ConsultTopOfLibrary {
        player,
        mode,
        filter,
        stop_rule,
        all_tag: all_tag.clone(),
        match_tag: match_tag.clone(),
    });

    Ok(Some(ConsultSentenceParts {
        effects,
        player,
        all_tag,
        match_tag,
    }))
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
    if !slice_contains(words, &"bottom") || !slice_contains(words, &"library") {
        return None;
    }
    if super::super::activation_and_restrictions::contains_word_sequence(words, &["random", "order"]) {
        return Some(LibraryBottomOrderAst::Random);
    }
    if super::super::activation_and_restrictions::contains_word_sequence(words, &["any", "order"]) {
        return Some(LibraryBottomOrderAst::ChooserChooses);
    }
    None
}

pub(crate) fn parse_consult_condition_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    let word_refs = word_view.word_refs();
    if matches!(word_refs.as_slice(), ["thiss", "power"] | ["this", "power"]) {
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

fn strip_prefix_phrases<'a>(
    tokens: &'a [OwnedLexToken],
    phrases: &[&'static [&'static str]],
) -> Option<(&'static [&'static str], &'a [OwnedLexToken])> {
    phrases.iter().find_map(|phrase| {
        grammar::parse_prefix(tokens, grammar::phrase(phrase)).map(|(_, rest)| (*phrase, rest))
    })
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

fn parse_if_no_card_into_hand_this_way_remainder_inner<'a>(
    input: &mut LexStream<'a>,
) -> Result<&'a [OwnedLexToken], ErrMode<ContextError>> {
    dispatch! {peek(grammar::word_parser_text);
        "if" => (
            (
                alt((
                    grammar::phrase(&["if", "you", "didnt"]),
                    grammar::phrase(&["if", "you", "didn't"]),
                    grammar::phrase(&["if", "you", "did", "not"]),
                )),
                grammar::kw("put"),
                opt(alt((grammar::kw("a"), grammar::kw("an"), grammar::kw("the")))),
                grammar::kw("card"),
                grammar::phrase(&["into", "your", "hand", "this", "way"]),
            )
                .void()
                .context(StrContext::Label("if-no-card prefix"))
                .context(StrContext::Expected(StrContextValue::Description(
                    "if you didn't put a card into your hand this way",
                ))),
            cut_err(grammar::comma())
                .context(StrContext::Label("if-no-card separator"))
                .context(StrContext::Expected(StrContextValue::Description(
                    "comma after if-no-card clause",
                ))),
            cut_err(take_remaining_clause_tokens),
        )
            .map(|(_, _, remainder)| remainder),
        _ => fail::<_, &'a [OwnedLexToken], _>,
    }
    .parse_next(input)
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

pub(crate) fn parse_consult_mana_value_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConsultCastManaValueCondition> {
    let (.., after_prefix) = strip_prefix_phrases(
        tokens,
        &[
            &["if", "it's", "a", "spell", "with", "mana", "value"][..],
            &["if", "it", "is", "a", "spell", "with", "mana", "value"][..],
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
    if let Some((super::super::token_primitives::TurnDurationPhrase::UntilEndOfTurn, remainder)) =
        parse_turn_duration_prefix(&second_tokens)
    {
        second_tokens = trim_commas(remainder);
        timing = ConsultCastTiming::UntilEndOfTurn;
    }

    let may_idx = find_index(&second_tokens, |token: &OwnedLexToken| token.is_word("may"))?;
    if may_idx == 0 || may_idx + 1 >= second_tokens.len() {
        return None;
    }

    let caster = match parse_subject(&second_tokens[..may_idx]) {
        SubjectAst::Player(player) => player,
        _ => return None,
    };
    let tail_tokens = &second_tokens[may_idx + 1..];
    let (matched_phrase, remainder_tokens) = strip_prefix_phrases(
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
    let allow_land = matches!(matched_phrase, ["play", ..]);
    let remainder_word_view = TokenWordView::new(remainder_tokens);
    let remainder = remainder_word_view.word_refs();
    if remainder == ["this", "turn"] {
        return Some(ConsultCastClause {
            caster,
            allow_land,
            timing: ConsultCastTiming::UntilEndOfTurn,
            cost: ConsultCastCost::Normal,
            mana_value_condition: None,
        });
    }

    if remainder
        == [
            "by", "paying", "life", "equal", "to", "the", "spell's", "mana", "value", "rather",
            "than", "paying", "its", "mana", "cost",
        ]
    {
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
    let mut clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    while clause_words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        clause_words.remove(0);
    }

    let Some(order) = parse_consult_remainder_order(&clause_words) else {
        return None;
    };
    let mode_word = match mode {
        LibraryConsultModeAst::Reveal => "revealed",
        LibraryConsultModeAst::Exile => "exiled",
    };
    if !grammar::contains_word(tokens, mode_word) {
        return None;
    }
    let mentions_cast_window = grammar::words_find_phrase(tokens, &["not", "cast", "this"]).is_some()
        || find_window_by(&clause_words, 4, |window| {
            window == ["werent", "cast", "this", "way"]
                || window == ["weren't", "cast", "this", "way"]
        })
        .is_some()
        || grammar::words_find_phrase(tokens, &["were", "not", "cast", "this", "way"]).is_some();
    let mentions_remainder =
        grammar::contains_word(tokens, "rest") || grammar::contains_word(tokens, "other");

    (mentions_cast_window || mentions_remainder).then_some(order)
}

pub(crate) fn parse_if_declined_put_match_into_hand(
    tokens: &[OwnedLexToken],
    match_tag: TagKey,
) -> Option<Vec<EffectAst>> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let moves_to_hand = clause_words == ["put", "that", "card", "into", "your", "hand"]
        || clause_words == ["put", "the", "exiled", "card", "into", "your", "hand"]
        || clause_words == ["put", "it", "into", "your", "hand"]
        || clause_words
            == [
                "put", "that", "card", "into", "your", "hand", "if", "it", "wasnt", "cast",
                "this", "way",
            ]
        || clause_words
            == [
                "put", "that", "card", "into", "your", "hand", "if", "it", "wasn't", "cast",
                "this", "way",
            ]
        || clause_words
            == [
                "put", "the", "exiled", "card", "into", "your", "hand", "if", "it", "wasnt",
                "cast", "this", "way",
            ]
        || clause_words
            == [
                "put", "the", "exiled", "card", "into", "your", "hand", "if", "it", "wasn't",
                "cast", "this", "way",
            ]
        || clause_words
            == [
                "put", "it", "into", "your", "hand", "if", "it", "wasnt", "cast", "this",
                "way",
            ]
        || clause_words
            == [
                "put", "it", "into", "your", "hand", "if", "it", "wasn't", "cast", "this",
                "way",
            ]
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &["if", "you", "dont", "put", "that", "card", "into", "your", "hand"],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if",
                "you",
                "dont",
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
            &["if", "you", "don\u{2019}t", "put", "it", "into", "your", "hand"],
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
                "if", "you", "do", "not", "put", "the", "exiled", "card", "into", "your",
                "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &["if", "you", "do", "not", "put", "it", "into", "your", "hand"],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "dont", "cast", "that", "card", "this", "way", "put", "it",
                "into", "your", "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "dont", "cast", "the", "exiled", "card", "this", "way", "put",
                "it", "into", "your", "hand",
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
                "if", "you", "do", "not", "cast", "the", "exiled", "card", "this", "way",
                "put", "it", "into", "your", "hand",
            ],
        )
        .is_some()
        || super::super::grammar::primitives::words_match_prefix(
            tokens,
            &[
                "if", "you", "dont", "cast", "it", "this", "way", "put", "it", "into",
                "your", "hand",
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
                "if", "you", "do", "not", "cast", "it", "this", "way", "put", "it", "into",
                "your", "hand",
            ],
        )
        .is_some();
    if !moves_to_hand {
        return None;
    }

    Some(vec![EffectAst::MoveToZone {
        target: TargetAst::Tagged(match_tag, None),
        zone: Zone::Hand,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped: false,
        attached_to: None,
    }])
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
            if clause.allow_land || matches!(clause.timing, ConsultCastTiming::UntilEndOfTurn) {
                vec![EffectAst::GrantPlayTaggedUntilEndOfTurn {
                    tag: match_tag.clone(),
                    player: clause.caster,
                    allow_land: clause.allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast: false,
                }]
            } else {
                vec![EffectAst::May {
                    effects: vec![EffectAst::CastTagged {
                        tag: match_tag.clone(),
                        allow_land: false,
                        as_copy: false,
                        without_paying_mana_cost,
                        cost_reduction: None,
                    }],
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
                EffectAst::GrantPlayTaggedUntilEndOfTurn {
                    tag: match_tag.clone(),
                    player: clause.caster,
                    allow_land: false,
                    without_paying_mana_cost: false,
                    allow_any_color_for_cast: false,
                },
                EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                    tag: match_tag.clone(),
                    player: clause.caster,
                },
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

pub(crate) fn parse_if_no_card_into_hand_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(after) = grammar::parse_all_or_none(
        tokens,
        parse_if_no_card_into_hand_this_way_remainder_inner,
        "if-no-card-into-hand-this-way clause",
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
