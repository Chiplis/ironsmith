#![allow(dead_code)]

use winnow::ascii::{digit1, multispace0, multispace1};
use winnow::combinator::{alt, cut_err, delimited, opt, preceded, terminated};
use winnow::error::{
    ContextError, ErrMode, ModalResult as WResult, ParserError, StrContext, StrContextValue,
};
use winnow::prelude::*;
use winnow::token::take_while;

use crate::cards::builders::{CardTextError, ChoiceCount};
use crate::color::Color;
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::Effect;
use crate::filter::ObjectFilter;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::target::PlayerFilter;
use crate::types::{CardType, Subtype, Supertype};

use super::effect_sentences::parse_subtype_word;
use super::lexer::{OwnedLexToken, TokenKind, lex_line, render_lexed_tokens};
use super::native_tokens::LowercaseWordView;
use super::object_filters::parse_object_filter_lexed;
use super::token_primitives::{
    parse_mana_cost_inner, parse_mana_symbol, parse_mana_symbol_group, parse_type_line_with,
};

#[derive(Debug, Clone)]
pub(crate) struct TypeLineCst {
    pub(crate) supertypes: Vec<Supertype>,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActivationCostCst {
    pub(crate) raw: String,
    pub(crate) segments: Vec<ActivationCostSegmentCst>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ActivationCostSegmentCst {
    Mana(ManaCost),
    Tap,
    TapChosen {
        count: u32,
        filter_text: String,
        other: bool,
    },
    Untap,
    Life(u32),
    Energy(u32),
    DiscardSource,
    DiscardHand,
    DiscardCard(u32),
    DiscardFiltered {
        count: u32,
        card_types: Vec<CardType>,
        random: bool,
        name: Option<String>,
        other: bool,
    },
    Mill(u32),
    SacrificeSelf,
    SacrificeCreature,
    SacrificeChosen {
        count: u32,
        filter_text: String,
        other: bool,
    },
    ExileSelf,
    ExileSelfFromGraveyard,
    ExileFromHand {
        count: u32,
        color_filter: Option<ColorSet>,
    },
    ExileFromGraveyard {
        count: u32,
        card_type: Option<CardType>,
    },
    ExileChosen {
        choice_count: ChoiceCount,
        filter_text: String,
    },
    ExileTopLibrary {
        count: u32,
    },
    ReturnSelfToHand,
    ReturnChosenToHand {
        count: u32,
        filter_text: String,
    },
    ExertSelf {
        display_text: String,
    },
    PutCounters {
        counter_type: CounterType,
        count: u32,
    },
    PutCountersChosen {
        counter_type: CounterType,
        count: u32,
        filter_text: String,
    },
    RemoveCounters {
        counter_type: CounterType,
        count: u32,
    },
    RemoveCountersAmong {
        counter_type: Option<CounterType>,
        count: u32,
        filter_text: String,
        display_x: bool,
    },
    RemoveCountersDynamic {
        counter_type: Option<CounterType>,
        display_x: bool,
    },
    Behold {
        subtype: Subtype,
        count: u32,
    },
}

fn parse_word<'a>(input: &mut &'a str) -> WResult<&'a str> {
    take_while(1.., |ch: char| {
        ch.is_ascii_alphabetic() || ch == '\'' || ch == '-'
    })
    .context(StrContext::Label("word"))
    .context(StrContext::Expected(StrContextValue::Description("word")))
    .parse_next(input)
}

fn parse_filter_text(text: &str, other: bool) -> Result<ObjectFilter, CardTextError> {
    let tokens = lex_line(text, 0)?;
    parse_object_filter_lexed(&tokens, other)
}

fn spaced<'a, O, P>(parser: P) -> impl Parser<&'a str, O, ErrMode<ContextError>>
where
    P: Parser<&'a str, O, ErrMode<ContextError>>,
{
    delimited(multispace0, parser, multispace0)
}

fn count_word_value(word: &str) -> Option<u32> {
    match word.to_ascii_lowercase().as_str() {
        "a" | "an" | "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        _ => None,
    }
}

fn parse_card_type_word(word: &str) -> Option<CardType> {
    match word.to_ascii_lowercase().as_str() {
        "creature" | "creatures" => Some(CardType::Creature),
        "artifact" | "artifacts" => Some(CardType::Artifact),
        "enchantment" | "enchantments" => Some(CardType::Enchantment),
        "land" | "lands" => Some(CardType::Land),
        "planeswalker" | "planeswalkers" => Some(CardType::Planeswalker),
        "instant" | "instants" => Some(CardType::Instant),
        "sorcery" | "sorceries" => Some(CardType::Sorcery),
        "battle" | "battles" => Some(CardType::Battle),
        "kindred" => Some(CardType::Kindred),
        _ => None,
    }
}

fn parse_color_word(word: &str) -> Option<ColorSet> {
    Color::from_name(word).map(ColorSet::from_color)
}

fn parse_supertype_word_local(word: &str) -> Option<Supertype> {
    match word.to_ascii_lowercase().as_str() {
        "basic" => Some(Supertype::Basic),
        "legendary" => Some(Supertype::Legendary),
        "snow" => Some(Supertype::Snow),
        "world" => Some(Supertype::World),
        _ => None,
    }
}

fn str_starts_with(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len()) == Some(prefix)
}

fn str_ends_with(text: &str, suffix: &str) -> bool {
    if suffix.len() > text.len() {
        return false;
    }
    text.get(text.len() - suffix.len()..) == Some(suffix)
}

fn str_contains(text: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    text.match_indices(needle).next().is_some()
}

fn str_strip_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    str_starts_with(text, prefix).then(|| &text[prefix.len()..])
}

fn str_strip_suffix<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    str_ends_with(text, suffix).then(|| &text[..text.len().saturating_sub(suffix.len())])
}

fn str_find(text: &str, needle: &str) -> Option<usize> {
    text.match_indices(needle).next().map(|(idx, _)| idx)
}

fn str_find_by(text: &str, mut predicate: impl FnMut(char) -> bool) -> Option<usize> {
    for (idx, ch) in text.char_indices() {
        if predicate(ch) {
            return Some(idx);
        }
    }
    None
}

fn str_split_once<'a>(text: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    let (idx, matched) = text.match_indices(needle).next()?;
    Some((&text[..idx], &text[idx + matched.len()..]))
}

fn str_split_once_char(text: &str, needle: char) -> Option<(&str, &str)> {
    for (idx, ch) in text.char_indices() {
        if ch == needle {
            let len = ch.len_utf8();
            return Some((&text[..idx], &text[idx + len..]));
        }
    }
    None
}

fn word_slice_starts_with(words: &[&str], prefix: &[&str]) -> bool {
    if prefix.len() > words.len() {
        return false;
    }
    for (idx, expected) in prefix.iter().enumerate() {
        if words[idx] != *expected {
            return false;
        }
    }
    true
}

fn trim_plural_s(word: &str) -> Option<&str> {
    let bytes = word.as_bytes();
    (bytes.len() > 1 && bytes[bytes.len() - 1] == b's').then(|| &word[..word.len() - 1])
}

fn find_word_index(words: &[&str], mut predicate: impl FnMut(&str) -> bool) -> Option<usize> {
    for (idx, word) in words.iter().enumerate() {
        if predicate(word) {
            return Some(idx);
        }
    }
    None
}

fn push_unique_card_type(card_types: &mut Vec<CardType>, card_type: CardType) {
    for existing in card_types.iter() {
        if *existing == card_type {
            return;
        }
    }
    card_types.push(card_type);
}

fn first_non_comma_token(tokens: &[OwnedLexToken]) -> Option<&OwnedLexToken> {
    for token in tokens {
        if !token.is_comma() {
            return Some(token);
        }
    }
    None
}

fn first_non_comma_token_index(tokens: &[OwnedLexToken]) -> Option<usize> {
    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_comma() {
            return Some(idx);
        }
    }
    None
}

fn trim_activation_cost_segment_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = first_non_comma_token_index(tokens).unwrap_or(tokens.len());
    let mut end = tokens.len();

    if tokens.get(start).is_some_and(|token| token.is_word("and")) {
        start += 1;
        while start < end && tokens[start].is_comma() {
            start += 1;
        }
    }

    if tokens
        .get(start)
        .is_some_and(|token| token.is_word("waterbend"))
    {
        start += 1;
        while start < end && tokens[start].is_comma() {
            start += 1;
        }
    }

    while end > start && (tokens[end - 1].is_period() || tokens[end - 1].is_comma()) {
        end -= 1;
    }

    &tokens[start..end]
}

fn render_trimmed_lexed_tokens(tokens: &[OwnedLexToken]) -> String {
    render_lexed_tokens(tokens).trim().to_string()
}

fn render_lower_lexed_tokens(tokens: &[OwnedLexToken]) -> String {
    render_trimmed_lexed_tokens(tokens).to_ascii_lowercase()
}

fn word_slice_ends_with(words: &[&str], suffix: &[&str]) -> bool {
    words.len() >= suffix.len() && words[words.len() - suffix.len()..] == *suffix
}

fn words_match_any(words: &[&str], patterns: &[&[&str]]) -> bool {
    patterns.iter().any(|pattern| words == *pattern)
}

fn words_start_with_any(words: &[&str], patterns: &[&[&str]]) -> bool {
    patterns
        .iter()
        .any(|pattern| word_slice_starts_with(words, pattern))
}

fn parse_count_prefix_words(words: &[&str]) -> Option<(u32, usize)> {
    let first = words.first().copied()?;
    if let Some(parsed) = count_word_value(first) {
        return Some((parsed, 1));
    }
    first.parse::<u32>().ok().map(|parsed| (parsed, 1))
}

fn skip_articles(words: &[&str], mut idx: usize) -> usize {
    while words
        .get(idx)
        .is_some_and(|word| matches!(*word, "a" | "an" | "the"))
    {
        idx += 1;
    }
    idx
}

fn token_slice_from_word_index<'a>(
    tokens: &'a [OwnedLexToken],
    words: &LowercaseWordView,
    word_idx: usize,
) -> Option<&'a [OwnedLexToken]> {
    let token_start = if word_idx == 0 {
        0
    } else {
        words.token_index_for_word_index(word_idx)?
    };
    Some(&tokens[token_start..])
}

fn token_slice_for_word_range<'a>(
    tokens: &'a [OwnedLexToken],
    words: &LowercaseWordView,
    word_start: usize,
    word_end: usize,
) -> Option<&'a [OwnedLexToken]> {
    let token_start = if word_start == 0 {
        0
    } else {
        words.token_index_for_word_index(word_start)?
    };
    let token_end = if word_end == word_start {
        token_start
    } else {
        words.token_index_after_words(word_end)?
    };
    Some(&tokens[token_start..token_end])
}

fn intern_counter_name(word: &str) -> &'static str {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    static INTERNER: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

    let map = INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().expect("counter name interner lock poisoned");
    if let Some(existing) = map.get(word) {
        return *existing;
    }

    let leaked: &'static str = Box::leak(word.to_string().into_boxed_str());
    map.insert(word.to_string(), leaked);
    leaked
}

fn parse_counter_type_word(word: &str) -> Option<CounterType> {
    match word {
        "+1/+1" => Some(CounterType::PlusOnePlusOne),
        "-1/-1" | "-0/-1" => Some(CounterType::MinusOneMinusOne),
        "+1/+0" => Some(CounterType::PlusOnePlusZero),
        "+0/+1" => Some(CounterType::PlusZeroPlusOne),
        "+1/+2" => Some(CounterType::PlusOnePlusTwo),
        "+2/+2" => Some(CounterType::PlusTwoPlusTwo),
        "-0/-2" => Some(CounterType::MinusZeroMinusTwo),
        "-2/-2" => Some(CounterType::MinusTwoMinusTwo),
        "deathtouch" => Some(CounterType::Deathtouch),
        "flying" => Some(CounterType::Flying),
        "haste" => Some(CounterType::Haste),
        "hexproof" => Some(CounterType::Hexproof),
        "indestructible" => Some(CounterType::Indestructible),
        "lifelink" => Some(CounterType::Lifelink),
        "menace" => Some(CounterType::Menace),
        "reach" => Some(CounterType::Reach),
        "trample" => Some(CounterType::Trample),
        "vigilance" => Some(CounterType::Vigilance),
        "loyalty" => Some(CounterType::Loyalty),
        "charge" => Some(CounterType::Charge),
        "stun" => Some(CounterType::Stun),
        "void" => Some(CounterType::Void),
        "depletion" => Some(CounterType::Depletion),
        "storage" => Some(CounterType::Storage),
        "ki" => Some(CounterType::Ki),
        "energy" => Some(CounterType::Energy),
        "age" => Some(CounterType::Age),
        "finality" => Some(CounterType::Finality),
        "time" => Some(CounterType::Time),
        "brain" => Some(CounterType::Brain),
        "burden" => Some(CounterType::Named(intern_counter_name("burden"))),
        "level" => Some(CounterType::Level),
        "lore" => Some(CounterType::Lore),
        "luck" => Some(CounterType::Luck),
        "oil" => Some(CounterType::Oil),
        _ => None,
    }
}

fn parse_count_inner(input: &mut &str) -> WResult<u32> {
    alt((
        digit1.try_map(str::parse::<u32>),
        parse_word.verify_map(count_word_value),
    ))
    .context(StrContext::Label("count"))
    .context(StrContext::Expected(StrContextValue::Description(
        "numeric or counted quantity",
    )))
    .parse_next(input)
}

fn parse_discard_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    preceded(
        spaced("discard"),
        cut_err(
            alt((
                spaced("this card").value(ActivationCostSegmentCst::DiscardSource),
                (
                    opt(terminated(parse_count_inner, multispace1)),
                    opt(spaced(alt(("a", "an")))),
                    spaced(alt(("card", "cards"))),
                )
                    .map(|(count, _article, _)| {
                        ActivationCostSegmentCst::DiscardCard(count.unwrap_or(1))
                    }),
            ))
            .context(StrContext::Expected(StrContextValue::Description(
                "discard cost",
            ))),
        ),
    )
    .context(StrContext::Label("discard segment"))
    .parse_next(input)
}

fn parse_pay_life_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    "pay".parse_next(input)?;
    multispace1.parse_next(input)?;
    let amount = parse_count_inner.parse_next(input)?;
    multispace1.parse_next(input)?;
    alt(("life", "lives")).parse_next(input)?;
    Ok(ActivationCostSegmentCst::Life(amount))
}

fn parse_sacrifice_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    let original = *input;
    if let Ok(parsed) = preceded(
        spaced("sacrifice"),
        cut_err(
            alt((
                spaced(alt((
                    "this creature",
                    "this artifact",
                    "this aura",
                    "this enchantment",
                    "this equipment",
                    "this fortification",
                    "this land",
                    "this permanent",
                    "this card",
                )))
                .value(ActivationCostSegmentCst::SacrificeSelf),
                spaced("a creature").value(ActivationCostSegmentCst::SacrificeCreature),
            ))
            .context(StrContext::Expected(StrContextValue::Description(
                "sacrifice cost",
            ))),
        ),
    )
    .parse_next(input)
    {
        return Ok(parsed);
    }

    *input = original;
    "sacrifice".parse_next(input)?;
    multispace1.parse_next(input)?;
    let count = parse_count_inner.parse_next(input).unwrap_or(1);
    let mut other = false;
    if str_starts_with(input.trim_start(), "another ") {
        multispace0.parse_next(input)?;
        "another".parse_next(input)?;
        other = true;
    }
    multispace0.parse_next(input)?;
    let filter_text = input.trim().to_string();
    if filter_text.is_empty() {
        let mut err = ContextError::new();
        err.push(StrContext::Label("sacrifice segment"));
        err.push(StrContext::Expected(StrContextValue::Description(
            "sacrifice filter",
        )));
        return Err(ErrMode::Cut(err));
    }
    *input = "";
    Ok(ActivationCostSegmentCst::SacrificeChosen {
        count,
        filter_text,
        other,
    })
}

fn parse_sacrifice_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let rest = if lower == "sacrifice" {
        ""
    } else if let Some(rest) = str_strip_prefix(lower.as_str(), "sacrifice ") {
        rest
    } else {
        return Err(CardTextError::ParseError(
            "rewrite sacrifice parser expected leading 'sacrifice'".to_string(),
        ));
    };

    if matches!(
        rest,
        "it" | "this"
            | "this creature"
            | "this artifact"
            | "this aura"
            | "this enchantment"
            | "this equipment"
            | "this fortification"
            | "this land"
            | "this permanent"
            | "this card"
    ) {
        return Ok(ActivationCostSegmentCst::SacrificeSelf);
    }
    if rest == "a creature" {
        return Ok(ActivationCostSegmentCst::SacrificeCreature);
    }

    let parts = rest.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    let mut other = false;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        } else if matches!(first, "a" | "an") {
            idx = 1;
        }
    }

    if parts.get(idx).is_some_and(|part| *part == "another") {
        other = true;
        idx += 1;
    }

    let filter_text = parts[idx..].join(" ");
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite sacrifice parser missing filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::SacrificeChosen {
        count,
        filter_text,
        other,
    })
}

fn parse_tap_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    alt((spaced("{t}"), spaced("t")))
        .context(StrContext::Label("tap segment"))
        .context(StrContext::Expected(StrContextValue::Description(
            "tap symbol",
        )))
        .value(ActivationCostSegmentCst::Tap)
        .parse_next(input)
}

fn parse_tap_chosen_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = str_strip_prefix(lower.as_str(), "tap ") else {
        return Err(CardTextError::ParseError(
            "rewrite tap-cost parser expected leading 'tap'".to_string(),
        ));
    };

    let parts = rest.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    let mut other = false;

    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        } else if matches!(first, "a" | "an") {
            idx = 1;
        }
    }

    if parts.get(idx).is_some_and(|part| *part == "another") {
        other = true;
        idx += 1;
    }

    if !parts.get(idx).is_some_and(|part| *part == "untapped") {
        return Err(CardTextError::ParseError(format!(
            "rewrite tap-cost parser expected untapped selector in '{raw}'"
        )));
    }
    idx += 1;

    let filter_text = parts[idx..].join(" ");
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite tap-cost parser missing tap filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::TapChosen {
        count,
        filter_text,
        other,
    })
}

fn parse_untap_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    alt((spaced("{q}"), spaced("q")))
        .context(StrContext::Label("untap segment"))
        .context(StrContext::Expected(StrContextValue::Description(
            "untap symbol",
        )))
        .value(ActivationCostSegmentCst::Untap)
        .parse_next(input)
}

fn parse_pay_energy_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    "pay".parse_next(input)?;
    multispace1.parse_next(input)?;

    let mut count = 0u32;
    loop {
        if spaced("{e}").value(()).parse_next(input).is_ok() {
            count += 1;
        } else if spaced("e").value(()).parse_next(input).is_ok() {
            count += 1;
        } else {
            break;
        }
    }

    if count == 0 {
        let mut err = ContextError::new();
        err.push(StrContext::Label("pay energy segment"));
        err.push(StrContext::Expected(StrContextValue::Description(
            "energy symbols",
        )));
        return Err(ErrMode::Backtrack(err));
    }
    Ok(ActivationCostSegmentCst::Energy(count))
}

fn parse_mill_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    preceded(
        ("mill", multispace1),
        cut_err((
            alt((parse_count_inner, alt(("a", "an")).value(1))),
            multispace1,
            alt(("card", "cards")),
        )),
    )
    .context(StrContext::Label("mill segment"))
    .context(StrContext::Expected(StrContextValue::Description(
        "mill cost",
    )))
    .map(|(count, _, _)| ActivationCostSegmentCst::Mill(count))
    .parse_next(input)
}

fn parse_behold_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    preceded(
        ("behold", multispace1),
        cut_err((
            alt((parse_count_inner, alt(("a", "an")).value(1))),
            multispace1,
            parse_word.verify_map(|word| {
                parse_subtype_word(word)
                    .or_else(|| trim_plural_s(word).and_then(parse_subtype_word))
            }),
        )),
    )
    .context(StrContext::Label("behold segment"))
    .context(StrContext::Expected(StrContextValue::Description(
        "behold cost",
    )))
    .map(|(count, _, subtype)| ActivationCostSegmentCst::Behold { subtype, count })
    .parse_next(input)
}

fn parse_counter_type_descriptor(raw: &str) -> Result<CounterType, CardTextError> {
    let words = raw
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| ch == ',' || ch == '.'))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    let counter_idx = find_word_index(&words, |word| matches!(word, "counter" | "counters"));

    let counter_type = counter_idx.and_then(|counter_idx| {
        if counter_idx == 0 {
            return None;
        }

        let prev = words[counter_idx - 1];
        if let Some(counter_type) = parse_counter_type_word(prev) {
            return Some(counter_type);
        }

        if prev == "strike" && counter_idx >= 2 {
            match words[counter_idx - 2] {
                "double" => return Some(CounterType::DoubleStrike),
                "first" => return Some(CounterType::FirstStrike),
                _ => {}
            }
        }

        if matches!(
            prev,
            "a" | "an" | "one" | "two" | "three" | "four" | "five" | "six" | "another"
        ) {
            return None;
        }

        prev.chars()
            .all(|ch| ch.is_ascii_alphabetic())
            .then(|| CounterType::Named(intern_counter_name(prev)))
    });

    counter_type.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite counter parser could not determine counter type from '{raw}'"
        ))
    })
}

fn parse_loyalty_shorthand_activation_cost_rewrite(
    raw: &str,
) -> Option<Vec<ActivationCostSegmentCst>> {
    let normalized = raw.trim().replace('−', "-");
    let prefix = str_split_once_char(normalized.as_str(), ':')
        .map(|(left, _)| left.trim())
        .unwrap_or(normalized.as_str());

    if let Some(rest) = str_strip_prefix(prefix, "+")
        && let Ok(amount) = rest.parse::<u32>()
    {
        return Some(if amount == 0 {
            Vec::new()
        } else {
            vec![ActivationCostSegmentCst::PutCounters {
                counter_type: CounterType::Loyalty,
                count: amount,
            }]
        });
    }

    if let Some(rest) = str_strip_prefix(prefix, "-") {
        if rest.eq_ignore_ascii_case("x") {
            return Some(vec![ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type: Some(CounterType::Loyalty),
                display_x: true,
            }]);
        }
        if let Ok(amount) = rest.parse::<u32>() {
            return Some(vec![ActivationCostSegmentCst::RemoveCounters {
                counter_type: CounterType::Loyalty,
                count: amount,
            }]);
        }
    }

    (prefix == "0").then(Vec::new)
}

fn parse_discard_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = str_strip_prefix(lower.as_str(), "discard ") else {
        return Err(CardTextError::ParseError(
            "rewrite discard parser expected leading 'discard'".to_string(),
        ));
    };

    if rest == "your hand" {
        return Ok(ActivationCostSegmentCst::DiscardHand);
    }

    if rest == "this card" {
        return Ok(ActivationCostSegmentCst::DiscardSource);
    }

    let Some(first_space) = str_find_by(trimmed, char::is_whitespace) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite discard parser expected selector in '{raw}'"
        )));
    };
    let rest_original = trimmed[first_space..].trim();
    let original_parts = rest_original.split_whitespace().collect::<Vec<_>>();
    let parts = rest.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        }
    }

    let mut other = false;
    if parts
        .get(idx)
        .is_some_and(|part| matches!(*part, "another" | "other"))
    {
        other = true;
        idx += 1;
    }

    while parts
        .get(idx)
        .is_some_and(|part| matches!(*part, "a" | "an"))
    {
        idx += 1;
    }

    if parts.get(idx) == Some(&"card") && parts.get(idx + 1) == Some(&"named") {
        let name_words = &original_parts[idx + 2..];
        if name_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser expected card name in '{raw}'"
            )));
        }
        return Ok(ActivationCostSegmentCst::DiscardFiltered {
            count,
            card_types: Vec::new(),
            random: false,
            name: Some(name_words.join(" ")),
            other,
        });
    }

    let mut card_types = Vec::new();
    while let Some(part) = parts.get(idx).copied() {
        if matches!(part, "card" | "cards") {
            break;
        }
        if matches!(part, "and" | "or" | "a" | "an") {
            idx += 1;
            continue;
        }
        let Some(card_type) = parse_card_type_word(part) else {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser does not yet support selector '{raw}'"
            )));
        };
        push_unique_card_type(&mut card_types, card_type);
        idx += 1;
    }

    if !parts
        .get(idx)
        .is_some_and(|part| matches!(*part, "card" | "cards"))
    {
        return Err(CardTextError::ParseError(format!(
            "rewrite discard parser expected card selector in '{raw}'"
        )));
    }
    idx += 1;

    let random = match parts.get(idx..) {
        None | Some([]) => false,
        Some(["at", "random"]) => true,
        _ => {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser does not yet support trailing clause in '{raw}'"
            )));
        }
    };

    if card_types.is_empty() && !random {
        return Ok(ActivationCostSegmentCst::DiscardCard(count));
    }

    Ok(ActivationCostSegmentCst::DiscardFiltered {
        count,
        card_types,
        random,
        name: None,
        other,
    })
}

fn parse_exile_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = str_strip_prefix(lower.as_str(), "exile ") else {
        return Err(CardTextError::ParseError(
            "rewrite exile parser expected leading 'exile'".to_string(),
        ));
    };

    if str_starts_with(rest, "target ") {
        return Err(CardTextError::ParseError(
            "unsupported targeted exile cost segment".to_string(),
        ));
    }

    if matches!(
        rest,
        "this"
            | "this card"
            | "this spell"
            | "this permanent"
            | "this creature"
            | "this artifact"
            | "this enchantment"
            | "this land"
            | "this aura"
            | "this vehicle"
    ) || str_starts_with(rest, "this card from your ")
        || str_starts_with(rest, "this spell from your ")
        || str_starts_with(rest, "this creature from your ")
        || str_starts_with(rest, "this artifact from your ")
        || str_starts_with(rest, "this enchantment from your ")
        || str_starts_with(rest, "this land from your ")
        || str_starts_with(rest, "this aura from your ")
        || str_starts_with(rest, "this vehicle from your ")
    {
        if str_contains(rest, "from your graveyard") {
            return Ok(ActivationCostSegmentCst::ExileSelfFromGraveyard);
        }
        return Ok(ActivationCostSegmentCst::ExileSelf);
    }

    if let Some(top_suffix) = str_strip_prefix(rest, "the top ")
        .and_then(|tail| str_strip_suffix(tail, " cards of your library"))
        .or_else(|| {
            str_strip_prefix(rest, "the top ")
                .and_then(|tail| str_strip_suffix(tail, " card of your library"))
        })
    {
        let count = parse_count_word_rewrite(top_suffix.trim())?;
        return Ok(ActivationCostSegmentCst::ExileTopLibrary { count });
    }

    if let Some(hand_suffix) = str_strip_suffix(rest, " from your hand") {
        let parts = hand_suffix.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            return Err(CardTextError::ParseError(
                "rewrite exile-from-hand parser found empty selector".to_string(),
            ));
        }

        let mut idx = 0usize;
        let mut count = 1u32;
        if let Some(first) = parts.first().copied() {
            if let Some(parsed) = count_word_value(first) {
                count = parsed;
                idx = 1;
            } else if let Ok(parsed) = first.parse::<u32>() {
                count = parsed;
                idx = 1;
            }
        }
        while parts
            .get(idx)
            .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
        {
            idx += 1;
        }

        let mut color_filter = None;
        if let Some(word) = parts.get(idx).copied()
            && let Some(color) = parse_color_word(word)
        {
            color_filter = Some(color);
            idx += 1;
        }

        if !parts
            .get(idx)
            .is_some_and(|part| matches!(*part, "card" | "cards"))
        {
            return Err(CardTextError::ParseError(format!(
                "rewrite exile-from-hand parser expected card selector in '{raw}'"
            )));
        }

        return Ok(ActivationCostSegmentCst::ExileFromHand {
            count,
            color_filter,
        });
    }

    if let Some(graveyard_suffix) = str_strip_suffix(rest, " from your graveyard") {
        if let Some((choice_count, filter_text)) = parse_generic_choice_prefix(graveyard_suffix) {
            return Ok(ActivationCostSegmentCst::ExileChosen {
                choice_count,
                filter_text: format!("{filter_text} from your graveyard"),
            });
        }

        let parts = graveyard_suffix.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            return Err(CardTextError::ParseError(
                "rewrite exile-from-graveyard parser found empty selector".to_string(),
            ));
        }

        let mut idx = 0usize;
        let mut count = 1u32;
        if let Some(first) = parts.first().copied() {
            if let Some(parsed) = count_word_value(first) {
                count = parsed;
                idx = 1;
            } else if let Ok(parsed) = first.parse::<u32>() {
                count = parsed;
                idx = 1;
            }
        }
        while parts
            .get(idx)
            .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
        {
            idx += 1;
        }

        let mut card_type = None;
        if let Some(word) = parts.get(idx).copied()
            && let Some(parsed) = parse_card_type_word(word)
        {
            card_type = Some(parsed);
            idx += 1;
        }

        if !parts
            .get(idx)
            .is_some_and(|part| matches!(*part, "card" | "cards"))
        {
            return Ok(ActivationCostSegmentCst::ExileChosen {
                choice_count: ChoiceCount::exactly(count as usize),
                filter_text: format!("{} from your graveyard", parts[idx..].join(" ")),
            });
        }

        idx += 1;
        if idx < parts.len() {
            return Ok(ActivationCostSegmentCst::ExileChosen {
                choice_count: ChoiceCount::exactly(count as usize),
                filter_text: format!("{} from your graveyard", parts[idx - 1..].join(" ")),
            });
        }

        return Ok(ActivationCostSegmentCst::ExileFromGraveyard { count, card_type });
    }

    let (choice_count, mut filter_text) = parse_generic_choice_prefix(rest).ok_or_else(|| {
        CardTextError::ParseError(format!("rewrite exile parser does not yet support '{raw}'"))
    })?;
    if str_ends_with(filter_text.as_str(), " from a single graveyard") {
        filter_text = filter_text.replace(" from a single graveyard", " from a graveyard");
    }
    Ok(ActivationCostSegmentCst::ExileChosen {
        choice_count,
        filter_text,
    })
}

fn parse_return_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = str_strip_prefix(lower.as_str(), "return ") else {
        return Err(CardTextError::ParseError(
            "rewrite return-cost parser expected leading 'return'".to_string(),
        ));
    };
    let Some(target) = str_strip_suffix(rest, " to its owner's hand")
        .or_else(|| str_strip_suffix(rest, " to their owner's hand"))
    else {
        return Err(CardTextError::ParseError(format!(
            "rewrite return-cost parser expected owner-hand suffix in '{raw}'"
        )));
    };

    if matches!(
        target,
        "it" | "this"
            | "this card"
            | "this permanent"
            | "this creature"
            | "this artifact"
            | "this enchantment"
            | "this land"
    ) {
        return Ok(ActivationCostSegmentCst::ReturnSelfToHand);
    }

    let parts = target.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        }
    }
    while parts
        .get(idx)
        .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
    {
        idx += 1;
    }
    let filter_text = parts[idx..].join(" ");
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite return-cost parser missing target filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::ReturnChosenToHand { count, filter_text })
}

fn parse_put_counter_segment_rewrite(raw: &str) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = str_strip_prefix(lower.as_str(), "put ") else {
        return Err(CardTextError::ParseError(
            "rewrite put-counter parser expected leading 'put'".to_string(),
        ));
    };
    let Some(on_idx) = str_find(rest, " on ") else {
        return Err(CardTextError::ParseError(format!(
            "rewrite put-counter parser missing 'on' in '{raw}'"
        )));
    };
    let descriptor = rest[..on_idx].trim();
    let target = rest[on_idx + 4..].trim();
    if !matches!(
        target,
        "this"
            | "this creature"
            | "this permanent"
            | "this artifact"
            | "this aura"
            | "this card"
            | "this land"
    ) {
        let parts = descriptor.split_whitespace().collect::<Vec<_>>();
        let mut idx = 0usize;
        let mut count = 1u32;
        if let Some(first) = parts.first().copied() {
            if let Some(parsed) = count_word_value(first) {
                count = parsed;
                idx = 1;
            } else if let Ok(parsed) = first.parse::<u32>() {
                count = parsed;
                idx = 1;
            }
        }
        while parts
            .get(idx)
            .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
        {
            idx += 1;
        }

        let counter_descriptor = parts[idx..].join(" ");
        let counter_type = parse_counter_type_descriptor(counter_descriptor.as_str())?;
        return Ok(ActivationCostSegmentCst::PutCountersChosen {
            counter_type,
            count,
            filter_text: target.to_string(),
        });
    }

    let parts = descriptor.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        }
    }
    while parts
        .get(idx)
        .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
    {
        idx += 1;
    }

    let counter_descriptor = parts[idx..].join(" ");
    let counter_type = parse_counter_type_descriptor(counter_descriptor.as_str())?;
    Ok(ActivationCostSegmentCst::PutCounters {
        counter_type,
        count,
    })
}

fn parse_remove_counter_segment_rewrite(
    raw: &str,
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = str_strip_prefix(lower.as_str(), "remove ") else {
        return Err(CardTextError::ParseError(
            "rewrite remove-counter parser expected leading 'remove'".to_string(),
        ));
    };
    let Some(from_idx) = str_find(rest, " from ") else {
        return Err(CardTextError::ParseError(format!(
            "rewrite remove-counter parser missing 'from' in '{raw}'"
        )));
    };
    let descriptor = rest[..from_idx].trim();
    let target = rest[from_idx + 6..].trim();
    let parts = descriptor.split_whitespace().collect::<Vec<_>>();
    if word_slice_starts_with(parts.as_slice(), &["x"]) {
        let counter_descriptor = parts[1..].join(" ");
        let counter_type = (!counter_descriptor.is_empty())
            .then(|| parse_counter_type_descriptor(counter_descriptor.as_str()))
            .transpose()?;
        return if let Some(filter_text) = str_strip_prefix(target, "among ") {
            Ok(ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count: 0,
                filter_text: filter_text.to_string(),
                display_x: true,
            })
        } else {
            Ok(ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x: true,
            })
        };
    }
    if word_slice_starts_with(parts.as_slice(), &["any", "number", "of"]) {
        let counter_descriptor = parts[3..].join(" ");
        let counter_type = (!counter_descriptor.is_empty()
            && counter_descriptor != "counter"
            && counter_descriptor != "counters")
            .then(|| parse_counter_type_descriptor(counter_descriptor.as_str()))
            .transpose()?;
        return if let Some(filter_text) = str_strip_prefix(target, "among ") {
            Ok(ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count: 0,
                filter_text: filter_text.to_string(),
                display_x: false,
            })
        } else {
            Ok(ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x: false,
            })
        };
    }
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        }
    }
    while parts
        .get(idx)
        .is_some_and(|part| matches!(*part, "a" | "an" | "the"))
    {
        idx += 1;
    }

    let counter_descriptor = parts[idx..].join(" ");
    let counter_type = (!counter_descriptor.is_empty()
        && counter_descriptor != "counter"
        && counter_descriptor != "counters")
        .then(|| parse_counter_type_descriptor(counter_descriptor.as_str()))
        .transpose()?;
    if let Some(filter_text) = str_strip_prefix(target, "among ") {
        return Ok(ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type,
            count,
            filter_text: filter_text.to_string(),
            display_x: false,
        });
    }

    if !matches!(
        target,
        "this"
            | "this creature"
            | "this permanent"
            | "this artifact"
            | "this enchantment"
            | "this card"
            | "this land"
            | "it"
    ) {
        return Ok(ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type,
            count,
            filter_text: target.to_string(),
            display_x: false,
        });
    }

    let counter_type = counter_type.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite remove-counter parser missing counter type in '{raw}'"
        ))
    })?;
    Ok(ActivationCostSegmentCst::RemoveCounters {
        counter_type,
        count,
    })
}

fn parse_generic_choice_prefix(raw: &str) -> Option<(ChoiceCount, String)> {
    let trimmed = raw.trim();
    if let Some(rest) = str_strip_prefix(trimmed, "one or more ") {
        return Some((ChoiceCount::at_least(1), rest.trim().to_string()));
    }
    if let Some(rest) = str_strip_prefix(trimmed, "any number of ") {
        return Some((ChoiceCount::any_number(), rest.trim().to_string()));
    }
    if let Some(rest) = str_strip_prefix(trimmed, "x ") {
        return Some((ChoiceCount::dynamic_x(), rest.trim().to_string()));
    }

    let parts = trimmed.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some(first) = parts.first().copied() {
        if let Some(parsed) = count_word_value(first) {
            count = parsed;
            idx = 1;
        } else if let Ok(parsed) = first.parse::<u32>() {
            count = parsed;
            idx = 1;
        } else if matches!(first, "a" | "an" | "the") {
            idx = 1;
        }
    }

    let filter_text = parts[idx..].join(" ");
    (!filter_text.is_empty()).then_some((ChoiceCount::exactly(count as usize), filter_text))
}

fn parse_generic_choice_prefix_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(ChoiceCount, &'a [OwnedLexToken])> {
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    if lowered.is_empty() {
        return None;
    }

    let (choice_count, consumed_words) =
        if word_slice_starts_with(lowered.as_slice(), &["one", "or", "more"]) {
            (ChoiceCount::at_least(1), 3)
        } else if word_slice_starts_with(lowered.as_slice(), &["any", "number", "of"]) {
            (ChoiceCount::any_number(), 3)
        } else if lowered.first() == Some(&"x") {
            (ChoiceCount::dynamic_x(), 1)
        } else if let Some((count, consumed_words)) = parse_count_prefix_words(lowered.as_slice()) {
            (ChoiceCount::exactly(count as usize), consumed_words)
        } else if lowered
            .first()
            .is_some_and(|word| matches!(*word, "a" | "an" | "the"))
        {
            (ChoiceCount::exactly(1), 1)
        } else {
            (ChoiceCount::exactly(1), 0)
        };

    let remainder = if consumed_words == 0 {
        tokens
    } else {
        let token_start = words.token_index_after_words(consumed_words)?;
        &tokens[token_start..]
    };
    Some((choice_count, remainder))
}

fn parse_discard_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    let tail = lowered.get(1..).unwrap_or_default();

    if tail == ["your", "hand"] {
        return Ok(ActivationCostSegmentCst::DiscardHand);
    }
    if tail == ["this", "card"] {
        return Ok(ActivationCostSegmentCst::DiscardSource);
    }
    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite discard parser expected selector in '{raw}'"
        )));
    }

    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some((parsed, consumed_words)) = parse_count_prefix_words(tail) {
        count = parsed;
        idx = consumed_words;
    }

    let mut other = false;
    if tail
        .get(idx)
        .is_some_and(|word| matches!(*word, "another" | "other"))
    {
        other = true;
        idx += 1;
    }

    while tail
        .get(idx)
        .is_some_and(|word| matches!(*word, "a" | "an"))
    {
        idx += 1;
    }

    if tail.get(idx) == Some(&"card") && tail.get(idx + 1) == Some(&"named") {
        let Some(name_tokens) = token_slice_from_word_index(tokens, &words, idx + 3) else {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser expected card name in '{raw}'"
            )));
        };
        let name = render_lower_lexed_tokens(name_tokens);
        if name.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser expected card name in '{raw}'"
            )));
        }
        return Ok(ActivationCostSegmentCst::DiscardFiltered {
            count,
            card_types: Vec::new(),
            random: false,
            name: Some(name),
            other,
        });
    }

    let mut card_types = Vec::new();
    while let Some(word) = tail.get(idx).copied() {
        if matches!(word, "card" | "cards") {
            break;
        }
        if matches!(word, "and" | "or" | "a" | "an") {
            idx += 1;
            continue;
        }
        let Some(card_type) = parse_card_type_word(word) else {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser does not yet support selector '{raw}'"
            )));
        };
        push_unique_card_type(&mut card_types, card_type);
        idx += 1;
    }

    if !tail
        .get(idx)
        .is_some_and(|word| matches!(*word, "card" | "cards"))
    {
        return Err(CardTextError::ParseError(format!(
            "rewrite discard parser expected card selector in '{raw}'"
        )));
    }
    idx += 1;

    let random = match tail.get(idx..) {
        None | Some([]) => false,
        Some(["at", "random"]) => true,
        _ => {
            return Err(CardTextError::ParseError(format!(
                "rewrite discard parser does not yet support trailing clause in '{raw}'"
            )));
        }
    };

    if card_types.is_empty() && !random {
        return Ok(ActivationCostSegmentCst::DiscardCard(count));
    }

    Ok(ActivationCostSegmentCst::DiscardFiltered {
        count,
        card_types,
        random,
        name: None,
        other,
    })
}

fn parse_sacrifice_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    let tail = lowered.get(1..).unwrap_or_default();

    if words_match_any(
        tail,
        &[
            &["it"],
            &["this"],
            &["this", "creature"],
            &["this", "artifact"],
            &["this", "aura"],
            &["this", "enchantment"],
            &["this", "equipment"],
            &["this", "fortification"],
            &["this", "land"],
            &["this", "permanent"],
            &["this", "card"],
        ],
    ) {
        return Ok(ActivationCostSegmentCst::SacrificeSelf);
    }
    if tail == ["a", "creature"] {
        return Ok(ActivationCostSegmentCst::SacrificeCreature);
    }

    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some((parsed, consumed_words)) = parse_count_prefix_words(tail) {
        count = parsed;
        idx = consumed_words;
    } else if tail.first().is_some_and(|word| matches!(*word, "a" | "an")) {
        idx = 1;
    }

    let mut other = false;
    if tail.get(idx) == Some(&"another") {
        other = true;
        idx += 1;
    }

    let Some(filter_tokens) = token_slice_from_word_index(tokens, &words, idx + 1) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite sacrifice parser missing filter in '{raw}'"
        )));
    };
    let filter_text = render_lower_lexed_tokens(filter_tokens);
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite sacrifice parser missing filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::SacrificeChosen {
        count,
        filter_text,
        other,
    })
}

fn parse_tap_chosen_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    let tail = lowered.get(1..).unwrap_or_default();

    let mut idx = 0usize;
    let mut count = 1u32;
    let mut other = false;

    if let Some((parsed, consumed_words)) = parse_count_prefix_words(tail) {
        count = parsed;
        idx = consumed_words;
    } else if tail.first().is_some_and(|word| matches!(*word, "a" | "an")) {
        idx = 1;
    }

    if tail.get(idx) == Some(&"another") {
        other = true;
        idx += 1;
    }

    if tail.get(idx) != Some(&"untapped") {
        return Err(CardTextError::ParseError(format!(
            "rewrite tap-cost parser expected untapped selector in '{raw}'"
        )));
    }
    idx += 1;

    let Some(filter_tokens) = token_slice_from_word_index(tokens, &words, idx + 1) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite tap-cost parser missing tap filter in '{raw}'"
        )));
    };
    let filter_text = render_lower_lexed_tokens(filter_tokens);
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite tap-cost parser missing tap filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::TapChosen {
        count,
        filter_text,
        other,
    })
}

fn parse_exile_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    let tail = lowered.get(1..).unwrap_or_default();

    if word_slice_starts_with(tail, &["target"]) {
        return Err(CardTextError::ParseError(
            "unsupported targeted exile cost segment".to_string(),
        ));
    }

    if words_match_any(
        tail,
        &[
            &["this"],
            &["this", "card"],
            &["this", "spell"],
            &["this", "permanent"],
            &["this", "creature"],
            &["this", "artifact"],
            &["this", "enchantment"],
            &["this", "land"],
            &["this", "aura"],
            &["this", "vehicle"],
        ],
    ) || words_start_with_any(
        tail,
        &[
            &["this", "card", "from", "your"],
            &["this", "spell", "from", "your"],
            &["this", "creature", "from", "your"],
            &["this", "artifact", "from", "your"],
            &["this", "enchantment", "from", "your"],
            &["this", "land", "from", "your"],
            &["this", "aura", "from", "your"],
            &["this", "vehicle", "from", "your"],
        ],
    ) {
        let mut idx = 0usize;
        while idx + 2 < tail.len() {
            if tail[idx] == "from" && tail[idx + 1] == "your" && tail[idx + 2] == "graveyard" {
                return Ok(ActivationCostSegmentCst::ExileSelfFromGraveyard);
            }
            idx += 1;
        }
        return Ok(ActivationCostSegmentCst::ExileSelf);
    }

    if word_slice_starts_with(tail, &["the", "top"])
        && (word_slice_ends_with(tail, &["cards", "of", "your", "library"])
            || word_slice_ends_with(tail, &["card", "of", "your", "library"]))
    {
        let count_words = &tail[2..tail.len().saturating_sub(4)];
        let count = parse_count_word_rewrite(&count_words.join(" "))?;
        return Ok(ActivationCostSegmentCst::ExileTopLibrary { count });
    }

    if word_slice_ends_with(tail, &["from", "your", "hand"]) {
        let subject = &tail[..tail.len() - 3];
        if subject.is_empty() {
            return Err(CardTextError::ParseError(
                "rewrite exile-from-hand parser found empty selector".to_string(),
            ));
        }

        let mut idx = 0usize;
        let mut count = 1u32;
        if let Some((parsed, consumed_words)) = parse_count_prefix_words(subject) {
            count = parsed;
            idx = consumed_words;
        }
        idx = skip_articles(subject, idx);

        let mut color_filter = None;
        if let Some(word) = subject.get(idx).copied()
            && let Some(color) = parse_color_word(word)
        {
            color_filter = Some(color);
            idx += 1;
        }

        if !subject
            .get(idx)
            .is_some_and(|word| matches!(*word, "card" | "cards"))
        {
            return Err(CardTextError::ParseError(format!(
                "rewrite exile-from-hand parser expected card selector in '{raw}'"
            )));
        }

        return Ok(ActivationCostSegmentCst::ExileFromHand {
            count,
            color_filter,
        });
    }

    if word_slice_ends_with(tail, &["from", "your", "graveyard"]) {
        let Some(subject_tokens) =
            token_slice_for_word_range(tokens, &words, 1, lowered.len().saturating_sub(3))
        else {
            return Err(CardTextError::ParseError(
                "rewrite exile-from-graveyard parser found empty selector".to_string(),
            ));
        };
        let (choice_count, filter_tokens) = parse_generic_choice_prefix_tokens(subject_tokens)
            .ok_or_else(|| {
                CardTextError::ParseError(
                    "rewrite exile-from-graveyard parser found empty selector".to_string(),
                )
            })?;
        let filter_text = render_lower_lexed_tokens(filter_tokens);
        return Ok(ActivationCostSegmentCst::ExileChosen {
            choice_count,
            filter_text: format!("{filter_text} from your graveyard"),
        });
    }

    let Some(subject_tokens) = token_slice_from_word_index(tokens, &words, 1) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite exile parser does not yet support '{raw}'"
        )));
    };
    let (choice_count, filter_tokens) = parse_generic_choice_prefix_tokens(subject_tokens)
        .ok_or_else(|| {
            CardTextError::ParseError(format!("rewrite exile parser does not yet support '{raw}'"))
        })?;
    let mut filter_text = render_lower_lexed_tokens(filter_tokens);
    if str_ends_with(filter_text.as_str(), " from a single graveyard") {
        filter_text = filter_text.replace(" from a single graveyard", " from a graveyard");
    }
    Ok(ActivationCostSegmentCst::ExileChosen {
        choice_count,
        filter_text,
    })
}

fn parse_return_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    let suffix_len = if word_slice_ends_with(lowered.as_slice(), &["to", "its", "owners", "hand"]) {
        4
    } else if word_slice_ends_with(lowered.as_slice(), &["to", "their", "owners", "hand"]) {
        4
    } else {
        return Err(CardTextError::ParseError(format!(
            "rewrite return-cost parser expected owner-hand suffix in '{raw}'"
        )));
    };

    let target = &lowered[1..lowered.len() - suffix_len];
    if words_match_any(
        target,
        &[
            &["it"],
            &["this"],
            &["this", "card"],
            &["this", "permanent"],
            &["this", "creature"],
            &["this", "artifact"],
            &["this", "enchantment"],
            &["this", "land"],
        ],
    ) {
        return Ok(ActivationCostSegmentCst::ReturnSelfToHand);
    }

    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some((parsed, consumed_words)) = parse_count_prefix_words(target) {
        count = parsed;
        idx = consumed_words;
    }
    idx = skip_articles(target, idx);

    let Some(filter_tokens) =
        token_slice_for_word_range(tokens, &words, idx + 1, lowered.len() - suffix_len)
    else {
        return Err(CardTextError::ParseError(format!(
            "rewrite return-cost parser missing target filter in '{raw}'"
        )));
    };
    let filter_text = render_lower_lexed_tokens(filter_tokens);
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite return-cost parser missing target filter in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::ReturnChosenToHand { count, filter_text })
}

fn parse_put_counter_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    let Some(on_word_idx) = find_word_index(lowered.as_slice(), |word| word == "on") else {
        return Err(CardTextError::ParseError(format!(
            "rewrite put-counter parser missing 'on' in '{raw}'"
        )));
    };

    let descriptor = &lowered[1..on_word_idx];
    let target = &lowered[on_word_idx + 1..];
    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some((parsed, consumed_words)) = parse_count_prefix_words(descriptor) {
        count = parsed;
        idx = consumed_words;
    }
    idx = skip_articles(descriptor, idx);

    let Some(counter_tokens) = token_slice_for_word_range(tokens, &words, idx + 1, on_word_idx)
    else {
        return Err(CardTextError::ParseError(format!(
            "rewrite put-counter parser missing counter description in '{raw}'"
        )));
    };
    let counter_descriptor = render_lower_lexed_tokens(counter_tokens);
    let counter_type = parse_counter_type_descriptor(counter_descriptor.as_str())?;

    if words_match_any(
        target,
        &[
            &["this"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "artifact"],
            &["this", "aura"],
            &["this", "card"],
            &["this", "land"],
        ],
    ) {
        return Ok(ActivationCostSegmentCst::PutCounters {
            counter_type,
            count,
        });
    }

    let Some(filter_tokens) = token_slice_from_word_index(tokens, &words, on_word_idx + 1) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite put-counter parser missing target filter in '{raw}'"
        )));
    };
    Ok(ActivationCostSegmentCst::PutCountersChosen {
        counter_type,
        count,
        filter_text: render_lower_lexed_tokens(filter_tokens),
    })
}

fn parse_remove_counter_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    let Some(from_word_idx) = find_word_index(lowered.as_slice(), |word| word == "from") else {
        return Err(CardTextError::ParseError(format!(
            "rewrite remove-counter parser missing 'from' in '{raw}'"
        )));
    };

    let descriptor = &lowered[1..from_word_idx];
    let target = &lowered[from_word_idx + 1..];
    let target_among = word_slice_starts_with(target, &["among"]);
    let target_filter_tokens = if target_among {
        token_slice_from_word_index(tokens, &words, from_word_idx + 2)
    } else {
        token_slice_from_word_index(tokens, &words, from_word_idx + 1)
    };
    let target_filter_text = target_filter_tokens
        .map(render_lower_lexed_tokens)
        .unwrap_or_default();

    if word_slice_starts_with(descriptor, &["x"]) {
        let counter_type = if descriptor.len() <= 1 {
            None
        } else {
            let counter_tokens =
                token_slice_for_word_range(tokens, &words, 2, from_word_idx).unwrap_or(&[]);
            let counter_descriptor = render_lower_lexed_tokens(counter_tokens);
            (!counter_descriptor.is_empty())
                .then(|| parse_counter_type_descriptor(counter_descriptor.as_str()))
                .transpose()?
        };
        return if target_among {
            Ok(ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count: 0,
                filter_text: target_filter_text,
                display_x: true,
            })
        } else {
            Ok(ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x: true,
            })
        };
    }

    if word_slice_starts_with(descriptor, &["any", "number", "of"]) {
        let counter_type = if descriptor.len() <= 3 {
            None
        } else {
            let counter_tokens =
                token_slice_for_word_range(tokens, &words, 4, from_word_idx).unwrap_or(&[]);
            let counter_descriptor = render_lower_lexed_tokens(counter_tokens);
            (!counter_descriptor.is_empty()
                && counter_descriptor != "counter"
                && counter_descriptor != "counters")
                .then(|| parse_counter_type_descriptor(counter_descriptor.as_str()))
                .transpose()?
        };
        return if target_among {
            Ok(ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count: 0,
                filter_text: target_filter_text,
                display_x: false,
            })
        } else {
            Ok(ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x: false,
            })
        };
    }

    let mut idx = 0usize;
    let mut count = 1u32;
    if let Some((parsed, consumed_words)) = parse_count_prefix_words(descriptor) {
        count = parsed;
        idx = consumed_words;
    }
    idx = skip_articles(descriptor, idx);

    let counter_type = if idx >= descriptor.len() {
        None
    } else {
        let counter_tokens =
            token_slice_for_word_range(tokens, &words, idx + 1, from_word_idx).unwrap_or(&[]);
        let counter_descriptor = render_lower_lexed_tokens(counter_tokens);
        (!counter_descriptor.is_empty()
            && counter_descriptor != "counter"
            && counter_descriptor != "counters")
            .then(|| parse_counter_type_descriptor(counter_descriptor.as_str()))
            .transpose()?
    };

    if target_among {
        return Ok(ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type,
            count,
            filter_text: target_filter_text,
            display_x: false,
        });
    }

    if !words_match_any(
        target,
        &[
            &["this"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "artifact"],
            &["this", "enchantment"],
            &["this", "card"],
            &["this", "land"],
            &["it"],
        ],
    ) {
        return Ok(ActivationCostSegmentCst::RemoveCountersAmong {
            counter_type,
            count,
            filter_text: target_filter_text,
            display_x: false,
        });
    }

    let counter_type = counter_type.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite remove-counter parser missing counter type in '{raw}'"
        ))
    })?;
    Ok(ActivationCostSegmentCst::RemoveCounters {
        counter_type,
        count,
    })
}

fn parse_activation_cost_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Result<ActivationCostSegmentCst, CardTextError>> {
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    let first = lowered.first().copied()?;

    match first {
        "pay" => Some(parse_pay_segment_tokens(tokens)),
        "discard" => Some(parse_discard_segment_tokens(tokens)),
        "mill" => Some(parse_mill_segment_tokens(tokens)),
        "sacrifice" => Some(parse_sacrifice_segment_tokens(tokens)),
        "tap" if lowered.iter().any(|word| *word == "untapped") => {
            Some(parse_tap_chosen_segment_tokens(tokens))
        }
        "behold" => Some(parse_behold_segment_tokens(tokens)),
        "exile" => Some(parse_exile_segment_tokens(tokens)),
        "return" => Some(parse_return_segment_tokens(tokens)),
        "exert" => Some(parse_exert_segment_tokens(tokens)),
        "put" => Some(parse_put_counter_segment_tokens(tokens)),
        "remove" => Some(parse_remove_counter_segment_tokens(tokens)),
        _ => parse_bare_symbol_segment_tokens(tokens).map(Ok),
    }
}

fn parse_energy_symbol_count_tokens(tokens: &[OwnedLexToken]) -> Option<u32> {
    let mut count = 0u32;
    for token in tokens {
        match token.kind {
            TokenKind::ManaGroup if token.slice.eq_ignore_ascii_case("{e}") => count += 1,
            TokenKind::Word if token.is_word("e") => count += 1,
            _ => return None,
        }
    }

    (count > 0).then_some(count)
}

fn parse_bare_symbol_segment_tokens(tokens: &[OwnedLexToken]) -> Option<ActivationCostSegmentCst> {
    if tokens.is_empty() {
        return None;
    }

    if tokens.len() == 1 {
        let token = &tokens[0];
        if token.is_word("t") || token.slice.eq_ignore_ascii_case("{t}") {
            return Some(ActivationCostSegmentCst::Tap);
        }
        if token.is_word("q") || token.slice.eq_ignore_ascii_case("{q}") {
            return Some(ActivationCostSegmentCst::Untap);
        }
    }

    if let Some(count) = parse_energy_symbol_count_tokens(tokens) {
        return Some(ActivationCostSegmentCst::Energy(count));
    }

    let mut pips = Vec::new();
    for token in tokens {
        match token.kind {
            TokenKind::ManaGroup => {
                let slice = token.slice.as_str();
                if slice.eq_ignore_ascii_case("{e}")
                    || slice.eq_ignore_ascii_case("{t}")
                    || slice.eq_ignore_ascii_case("{q}")
                {
                    return None;
                }
                let group = parse_mana_symbol_group(slice).ok()?;
                pips.push(group);
            }
            TokenKind::Word => {
                let word = token.as_word()?;
                if word.eq_ignore_ascii_case("e")
                    || word.eq_ignore_ascii_case("t")
                    || word.eq_ignore_ascii_case("q")
                {
                    return None;
                }
                if let Ok(group) = parse_mana_symbol_group(word) {
                    pips.push(group);
                    continue;
                }
                let symbol = parse_mana_symbol(word).ok()?;
                pips.push(vec![symbol]);
            }
            _ => return None,
        }
    }

    (!pips.is_empty()).then(|| ActivationCostSegmentCst::Mana(ManaCost::from_pips(pips)))
}

fn parse_pay_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_trimmed_lexed_tokens(tokens);
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    if lowered.first().copied() != Some("pay") {
        return Err(CardTextError::ParseError(
            "rewrite pay-cost parser expected leading 'pay'".to_string(),
        ));
    }

    let Some(rest_tokens) = token_slice_from_word_index(tokens, &words, 1) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite pay-cost parser missing payment in '{raw}'"
        )));
    };
    let rest_words = LowercaseWordView::new(rest_tokens);
    let lowered_rest = rest_words.to_word_refs();
    if lowered_rest.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite pay-cost parser missing payment in '{raw}'"
        )));
    }

    if matches!(lowered_rest.last(), Some(&"life") | Some(&"lives")) {
        let count_words = &lowered_rest[..lowered_rest.len() - 1];
        if let Some((amount, consumed_words)) = parse_count_prefix_words(count_words)
            && consumed_words == count_words.len()
        {
            return Ok(ActivationCostSegmentCst::Life(amount));
        }
    }

    if let Some(count) = parse_energy_symbol_count_tokens(rest_tokens) {
        return Ok(ActivationCostSegmentCst::Energy(count));
    }

    if let Some((count, consumed_words)) = parse_count_prefix_words(lowered_rest.as_slice())
        && let Some(energy_tokens) =
            token_slice_from_word_index(rest_tokens, &rest_words, consumed_words)
        && parse_energy_symbol_count_tokens(energy_tokens) == Some(1)
    {
        return Ok(ActivationCostSegmentCst::Energy(count));
    }

    if let Some(ActivationCostSegmentCst::Mana(cost)) =
        parse_bare_symbol_segment_tokens(rest_tokens)
    {
        return Ok(ActivationCostSegmentCst::Mana(cost));
    }

    Err(CardTextError::ParseError(format!(
        "rewrite pay-cost parser does not yet support '{raw}'"
    )))
}

fn parse_exert_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_trimmed_lexed_tokens(tokens);
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    if lowered.first().copied() != Some("exert") {
        return Err(CardTextError::ParseError(
            "rewrite exert-cost parser expected leading 'exert'".to_string(),
        ));
    }
    let missing_object = match token_slice_from_word_index(tokens, &words, 1) {
        None => true,
        Some(rest) => LowercaseWordView::new(rest).is_empty(),
    };
    if missing_object {
        return Err(CardTextError::ParseError(format!(
            "rewrite exert-cost parser missing exerted object in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::ExertSelf { display_text: raw })
}

fn parse_mill_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    if lowered.first().copied() != Some("mill") {
        return Err(CardTextError::ParseError(
            "rewrite mill parser expected leading 'mill'".to_string(),
        ));
    }

    let tail = lowered.get(1..).unwrap_or_default();
    let (count, consumed_words) =
        if let Some((count, consumed_words)) = parse_count_prefix_words(tail) {
            (count, consumed_words)
        } else if tail.first().is_some_and(|word| matches!(*word, "a" | "an")) {
            (1, 1)
        } else {
            return Err(CardTextError::ParseError(format!(
                "rewrite mill parser expected card count in '{raw}'"
            )));
        };

    let has_card_word = tail
        .get(consumed_words)
        .is_some_and(|word| matches!(*word, "card" | "cards"));
    if !has_card_word || consumed_words + 1 != tail.len() {
        return Err(CardTextError::ParseError(format!(
            "rewrite mill parser expected trailing card selector in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::Mill(count))
}

fn parse_behold_segment_tokens(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostSegmentCst, CardTextError> {
    let raw = render_lower_lexed_tokens(tokens);
    let words = LowercaseWordView::new(tokens);
    let lowered = words.to_word_refs();
    if lowered.first().copied() != Some("behold") {
        return Err(CardTextError::ParseError(
            "rewrite behold parser expected leading 'behold'".to_string(),
        ));
    }

    let tail = lowered.get(1..).unwrap_or_default();
    let (count, consumed_words) =
        if let Some((count, consumed_words)) = parse_count_prefix_words(tail) {
            (count, consumed_words)
        } else if tail.first().is_some_and(|word| matches!(*word, "a" | "an")) {
            (1, 1)
        } else {
            return Err(CardTextError::ParseError(format!(
                "rewrite behold parser expected subtype count in '{raw}'"
            )));
        };

    let Some(subtype_word) = tail.get(consumed_words).copied() else {
        return Err(CardTextError::ParseError(format!(
            "rewrite behold parser expected subtype in '{raw}'"
        )));
    };
    let Some(subtype) = parse_subtype_word(subtype_word)
        .or_else(|| trim_plural_s(subtype_word).and_then(parse_subtype_word))
    else {
        return Err(CardTextError::ParseError(format!(
            "rewrite behold parser expected subtype in '{raw}'"
        )));
    };
    if consumed_words + 1 != tail.len() {
        return Err(CardTextError::ParseError(format!(
            "rewrite behold parser does not yet support trailing clause in '{raw}'"
        )));
    }

    Ok(ActivationCostSegmentCst::Behold { subtype, count })
}

fn parse_activation_cost_segment_inner(input: &mut &str) -> WResult<ActivationCostSegmentCst> {
    alt((
        parse_tap_segment_inner,
        parse_untap_segment_inner,
        parse_pay_life_segment_inner,
        parse_pay_energy_segment_inner,
        parse_discard_segment_inner,
        parse_mill_segment_inner,
        parse_behold_segment_inner,
        parse_sacrifice_segment_inner,
        parse_mana_cost_inner.map(ActivationCostSegmentCst::Mana),
    ))
    .context(StrContext::Label("activation cost segment"))
    .context(StrContext::Expected(StrContextValue::Description(
        "activation cost segment",
    )))
    .parse_next(input)
}

fn finish_parse<'a, O, E>(
    raw: &'a str,
    mut parser: impl Parser<&'a str, O, E>,
    label: &str,
) -> Result<O, CardTextError>
where
    E: std::fmt::Display,
{
    let mut input = raw.trim();
    let parsed = parser
        .parse_next(&mut input)
        .map_err(|err| CardTextError::ParseError(format!("rewrite {label} parse failed: {err}")))?;
    if !input.trim().is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite {label} parser left trailing input: '{}'",
            input.trim()
        )));
    }
    Ok(parsed)
}

pub(crate) fn parse_count_word_rewrite(raw: &str) -> Result<u32, CardTextError> {
    finish_parse(raw, spaced(parse_count_inner), "count-word")
}

#[cfg(test)]
pub(crate) fn parse_mana_symbol_group_rewrite(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    parse_mana_symbol_group(raw)
}

pub(crate) fn parse_mana_cost_rewrite(raw: &str) -> Result<ManaCost, CardTextError> {
    finish_parse(raw, spaced(parse_mana_cost_inner), "mana-cost")
}

fn parse_shard_style_mana_or_tap_cost_rewrite(raw: &str) -> Option<(ManaSymbol, ManaSymbol)> {
    let normalized = raw.trim().to_ascii_lowercase();
    let (left_raw, right_raw) = str_split_once(normalized.as_str(), " or ")?;

    fn parse_branch(branch: &str) -> Option<ManaSymbol> {
        let parts = branch
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.len() != 2 || parts[1] != "{t}" {
            return None;
        }

        let mana_cost = parse_mana_cost_rewrite(parts[0]).ok()?;
        let [pip] = mana_cost.pips() else {
            return None;
        };
        let [symbol] = pip.as_slice() else {
            return None;
        };
        Some(*symbol)
    }

    let left = parse_branch(left_raw)?;
    let right = parse_branch(right_raw)?;
    Some((left, right))
}

pub(crate) fn parse_type_line_rewrite(raw: &str) -> Result<TypeLineCst, CardTextError> {
    let (supertypes, card_types, subtypes) = parse_type_line_with(
        raw,
        parse_supertype_word_local,
        parse_card_type_word,
        parse_subtype_word,
    )?;

    Ok(TypeLineCst {
        supertypes,
        card_types,
        subtypes,
    })
}

fn starts_new_activation_cost_segment_tokens(tokens: &[OwnedLexToken]) -> bool {
    let Some(first) = first_non_comma_token(tokens) else {
        return false;
    };

    match first.kind {
        TokenKind::ManaGroup | TokenKind::Plus | TokenKind::Dash => true,
        TokenKind::Word => matches!(
            first.slice.to_ascii_lowercase().as_str(),
            "tap"
                | "t"
                | "untap"
                | "q"
                | "pay"
                | "discard"
                | "mill"
                | "sacrifice"
                | "exile"
                | "return"
                | "put"
                | "remove"
                | "behold"
                | "exert"
                | "waterbend"
                | "e"
                | "and"
                | "0"
        ),
        _ => false,
    }
}

fn split_activation_cost_segments_tokens(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut inside_named_card = false;
    let mut idx = 0usize;

    while idx < tokens.len() {
        if !inside_named_card
            && tokens[idx].is_word("card")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("named"))
        {
            inside_named_card = true;
        }

        let split_here = if tokens[idx].is_comma() {
            let remainder = &tokens[idx + 1..];
            !inside_named_card || starts_new_activation_cost_segment_tokens(remainder)
        } else if tokens[idx].is_word("and") && idx > start {
            let remainder = &tokens[idx + 1..];
            !inside_named_card && starts_new_activation_cost_segment_tokens(remainder)
        } else {
            false
        };

        if split_here {
            let segment = tokens[start..idx].to_vec();
            if !segment.is_empty() {
                segments.push(segment);
            }
            start = idx + 1;
            inside_named_card = false;
        }

        idx += 1;
    }

    let tail = tokens[start..].to_vec();
    if !tail.is_empty() {
        segments.push(tail);
    }

    segments
}

fn parse_activation_cost_cst_tokens(
    tokens: &[OwnedLexToken],
    raw: &str,
) -> Result<ActivationCostCst, CardTextError> {
    let trimmed_raw = raw.trim();
    if let Some(segments) = parse_loyalty_shorthand_activation_cost_rewrite(trimmed_raw) {
        return Ok(ActivationCostCst {
            raw: trimmed_raw.to_string(),
            segments,
        });
    }

    if let Some((left, right)) = parse_shard_style_mana_or_tap_cost_rewrite(trimmed_raw) {
        return Ok(ActivationCostCst {
            raw: trimmed_raw.to_string(),
            segments: vec![
                ActivationCostSegmentCst::Mana(ManaCost::from_pips(vec![vec![left, right]])),
                ActivationCostSegmentCst::Tap,
            ],
        });
    }

    let mut segments = Vec::new();
    for segment_tokens in split_activation_cost_segments_tokens(tokens) {
        let segment_tokens = trim_activation_cost_segment_tokens(&segment_tokens);
        if segment_tokens.is_empty() {
            continue;
        }

        let segment = render_trimmed_lexed_tokens(segment_tokens);
        let normalized_segment = segment.to_ascii_lowercase();
        let parsed = if let Some(parsed) = parse_activation_cost_segment_tokens(segment_tokens) {
            parsed
        } else {
            finish_parse(
                normalized_segment.as_str(),
                spaced(parse_activation_cost_segment_inner),
                "activation-cost-segment",
            )
            .map_err(|err| {
                CardTextError::ParseError(format!(
                    "unsupported activation cost segment (clause: '{}'): {err}",
                    segment,
                ))
            })
        }?;
        segments.push(parsed);
    }

    if segments.is_empty() {
        return Err(CardTextError::ParseError(
            "rewrite activation-cost parser found no segments".to_string(),
        ));
    }

    Ok(ActivationCostCst {
        raw: trimmed_raw.to_string(),
        segments,
    })
}

pub(crate) fn parse_activation_cost_tokens_rewrite(
    tokens: &[OwnedLexToken],
) -> Result<ActivationCostCst, CardTextError> {
    parse_activation_cost_cst_tokens(tokens, &render_lexed_tokens(tokens))
}

pub(crate) fn parse_activation_cost_rewrite(raw: &str) -> Result<ActivationCostCst, CardTextError> {
    let tokens = lex_line(raw.trim(), 0)?;
    parse_activation_cost_cst_tokens(&tokens, raw)
}

pub(crate) fn lower_activation_cost_cst(
    cst: &ActivationCostCst,
) -> Result<TotalCost, CardTextError> {
    fn flush_pending_mana(costs: &mut Vec<Cost>, pending: &mut Vec<Vec<ManaSymbol>>) {
        if pending.is_empty() {
            return;
        }
        costs.push(Cost::mana(ManaCost::from_pips(std::mem::take(pending))));
    }

    let mut costs = Vec::new();
    let mut pending_mana_pips = Vec::new();
    let mut tap_tag_id = 0usize;
    let mut sacrifice_tag_id = 0usize;
    let mut exile_tag_id = 0usize;
    let mut return_tag_id = 0usize;
    for segment in &cst.segments {
        match segment {
            ActivationCostSegmentCst::Mana(cost) => {
                pending_mana_pips.extend(cost.pips().to_vec());
            }
            ActivationCostSegmentCst::Tap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::tap());
            }
            ActivationCostSegmentCst::TapChosen {
                count,
                filter_text,
                other,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = parse_filter_text(filter_text, *other)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                filter.untapped = true;
                let tag = format!("tap_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::tap(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::Untap => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::untap());
            }
            ActivationCostSegmentCst::Life(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::life(*amount));
            }
            ActivationCostSegmentCst::Energy(amount) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::energy(*amount));
            }
            ActivationCostSegmentCst::DiscardSource => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_source());
            }
            ActivationCostSegmentCst::DiscardHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard_hand());
            }
            ActivationCostSegmentCst::DiscardCard(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::discard(*count, None));
            }
            ActivationCostSegmentCst::DiscardFiltered {
                count,
                card_types,
                random,
                name,
                other,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                if *random || name.is_some() || *other {
                    let card_filter = if card_types.is_empty() && name.is_none() && !*other {
                        None
                    } else {
                        let mut filter = ObjectFilter {
                            zone: Some(crate::zone::Zone::Hand),
                            card_types: card_types.clone(),
                            ..Default::default()
                        };
                        if let Some(name) = name {
                            filter = filter.named(name.clone());
                        }
                        if *other {
                            filter.other = true;
                        }
                        Some(filter)
                    };
                    costs.push(Cost::validated_effect(Effect::discard_player_filtered(
                        *count as i32,
                        PlayerFilter::You,
                        *random,
                        card_filter,
                    )));
                } else if card_types.len() > 1 {
                    costs.push(Cost::discard_types(*count, card_types.clone()));
                } else if let Some(card_type) = card_types.first().copied() {
                    costs.push(Cost::discard(*count, Some(card_type)));
                } else {
                    costs.push(Cost::discard(*count, None));
                }
            }
            ActivationCostSegmentCst::Mill(count) => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::mill(*count));
            }
            ActivationCostSegmentCst::Behold { subtype, count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::behold(*subtype, *count)));
            }
            ActivationCostSegmentCst::SacrificeSelf => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::sacrifice_self());
            }
            ActivationCostSegmentCst::SacrificeCreature => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let tag = format!("sacrifice_cost_{sacrifice_tag_id}");
                sacrifice_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    ObjectFilter::creature().you_control(),
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::sacrifice(
                    ObjectFilter::tagged(tag),
                    1,
                )));
            }
            ActivationCostSegmentCst::SacrificeChosen {
                count,
                filter_text,
                other,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let normalized_filter_text = if *count == 1 {
                    str_strip_prefix(filter_text.trim(), "a ")
                        .or_else(|| str_strip_prefix(filter_text.trim(), "an "))
                        .unwrap_or(filter_text.trim())
                } else {
                    filter_text.trim()
                };
                let mut filter = parse_filter_text(normalized_filter_text, *other)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = format!("sacrifice_cost_{sacrifice_tag_id}");
                sacrifice_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::sacrifice(
                    ObjectFilter::tagged(tag),
                    *count,
                )));
            }
            ActivationCostSegmentCst::ExileSelf => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            ActivationCostSegmentCst::ExileSelfFromGraveyard => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_self());
            }
            ActivationCostSegmentCst::ExileFromHand {
                count,
                color_filter,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::exile_from_hand(*count, *color_filter));
            }
            ActivationCostSegmentCst::ExileFromGraveyard { count, card_type } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = ObjectFilter::default()
                    .owned_by(PlayerFilter::You)
                    .in_zone(crate::zone::Zone::Graveyard);
                if let Some(card_type) = card_type {
                    filter = filter.with_type(*card_type);
                }
                let tag = format!("exile_cost_{exile_tag_id}");
                exile_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::exile(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExileChosen {
                choice_count,
                filter_text,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = parse_filter_text(filter_text, false)?;
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                if filter.zone == Some(crate::zone::Zone::Battlefield)
                    && filter.controller.is_none()
                {
                    filter.controller = Some(PlayerFilter::You);
                }
                let tag = format!("exile_cost_{exile_tag_id}");
                exile_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    *choice_count,
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::exile(
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExileTopLibrary { count } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::validated_effect(Effect::exile_top_of_library_player(
                    *count as i32,
                    PlayerFilter::You,
                )));
            }
            ActivationCostSegmentCst::ReturnSelfToHand => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::return_self_to_hand());
            }
            ActivationCostSegmentCst::ReturnChosenToHand { count, filter_text } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = parse_filter_text(filter_text, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let tag = format!("return_cost_{return_tag_id}");
                return_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(*count as usize),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::return_to_hand(
                    ObjectFilter::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::ExertSelf { display_text } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::effect(crate::effects::ExertCostEffect::new(
                    display_text.clone(),
                )));
            }
            ActivationCostSegmentCst::PutCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::add_counters(*counter_type, *count));
            }
            ActivationCostSegmentCst::PutCountersChosen {
                counter_type,
                count,
                filter_text,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let normalized_filter = filter_text.trim().to_ascii_lowercase();
                if matches!(
                    normalized_filter.as_str(),
                    "a creature you control" | "creature you control"
                ) {
                    costs.push(Cost::add_counters(*counter_type, *count));
                    continue;
                }
                let mut filter = parse_filter_text(filter_text, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let tag = format!("put_counter_cost_{tap_tag_id}");
                tap_tag_id += 1;
                costs.push(Cost::validated_effect(Effect::choose_objects(
                    filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag.clone(),
                )));
                costs.push(Cost::validated_effect(Effect::put_counters(
                    *counter_type,
                    *count as i32,
                    crate::target::ChooseSpec::tagged(tag),
                )));
            }
            ActivationCostSegmentCst::RemoveCounters {
                counter_type,
                count,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::remove_counters(*counter_type, *count));
            }
            ActivationCostSegmentCst::RemoveCountersAmong {
                counter_type,
                count,
                filter_text,
                display_x,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                let mut filter = parse_filter_text(filter_text, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(crate::zone::Zone::Battlefield);
                }
                let max_count = if *display_x { u32::MAX / 4 } else { *count };
                costs.push(Cost::validated_effect(Effect::remove_any_counters_among(
                    max_count,
                    filter,
                    *counter_type,
                )));
            }
            ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type,
                display_x,
            } => {
                flush_pending_mana(&mut costs, &mut pending_mana_pips);
                costs.push(Cost::remove_any_counters_from_source(
                    *counter_type,
                    *display_x,
                ));
            }
        }
    }
    flush_pending_mana(&mut costs, &mut pending_mana_pips);
    Ok(TotalCost::from_costs(costs))
}
