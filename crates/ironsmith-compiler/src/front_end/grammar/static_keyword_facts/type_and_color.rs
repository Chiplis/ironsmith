use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::color::{Color, ColorSet};
use crate::effect::Value;
use crate::types::Subtype;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipYourUpkeepTail<'a> {
    None,
    Condition(&'a [OwnedLexToken]),
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkipYourUpkeepFact<'a> {
    pub tail: SkipYourUpkeepTail<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubjectTypeAdditionFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub descriptor_tokens: &'a [OwnedLexToken],
    pub chosen_type: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubjectCardTypeIdentityFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub descriptor_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChosenColorAdditionFact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerToughnessTypeAdditionFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub descriptor_tokens: &'a [OwnedLexToken],
    pub power: i32,
    pub toughness: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorTypeAdditionFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub descriptor_tokens: &'a [OwnedLexToken],
    pub color: ColorSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubjectsAreBasicFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubjectColorFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub color: ColorSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasicLandSubtypeFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub subtype: Subtype,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandTypeAdditionFact<'a> {
    EveryBasic {
        subject_tokens: &'a [OwnedLexToken],
    },
    One {
        subject_tokens: &'a [OwnedLexToken],
        subtype: Subtype,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandAnimationFact<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub power: i32,
    pub toughness: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OtherTypeAdditionTailFact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LosesOtherCreatureTypesFact {
    pub marker_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasePowerToughnessGrantFact<'a> {
    pub power: i32,
    pub toughness: i32,
    pub ability_tokens: &'a [OwnedLexToken],
}

pub fn parse_skip_your_upkeep_tokens(tokens: &[OwnedLexToken]) -> Option<SkipYourUpkeepFact<'_>> {
    let (_, rest) =
        primitives::parse_prefix(tokens, semantic_phrase(&["skip", "your", "upkeep", "step"]))?;
    if semantic_tokens_are_empty(rest) {
        return Some(SkipYourUpkeepFact {
            tail: SkipYourUpkeepTail::None,
        });
    }
    let Some((_, condition_tokens)) = primitives::parse_prefix(rest, semantic_kw("if")) else {
        return Some(SkipYourUpkeepFact {
            tail: SkipYourUpkeepTail::Unsupported,
        });
    };
    let condition_tokens = trim_sentence_edges(condition_tokens);
    Some(SkipYourUpkeepFact {
        tail: if condition_tokens.is_empty() {
            SkipYourUpkeepTail::Unsupported
        } else {
            SkipYourUpkeepTail::Condition(condition_tokens)
        },
    })
}

pub fn parse_subject_type_addition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SubjectTypeAdditionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_subject_type_addition,
        "static subject type addition",
    )
    .ok()
}

pub fn parse_subject_card_type_identity_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SubjectCardTypeIdentityFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_subject_card_type_identity,
        "static subject card-type identity",
    )
    .ok()
}

pub fn parse_all_cards_chosen_color_addition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ChosenColorAdditionFact> {
    primitives::parse_all(
        tokens,
        (
            semantic_phrase(&[
                "all",
                "cards",
                "that",
                "arent",
                "on",
                "the",
                "battlefield",
                "spells",
                "and",
                "permanents",
                "are",
                "the",
                "chosen",
                "color",
                "in",
                "addition",
                "to",
                "their",
                "other",
                "colors",
            ]),
            semantic_finish,
        )
            .value(ChosenColorAdditionFact),
        "all cards chosen-color addition",
    )
    .ok()
}

pub fn parse_power_toughness_type_addition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PowerToughnessTypeAdditionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_power_toughness_type_addition,
        "static power-toughness type addition",
    )
    .ok()
}

pub fn parse_color_type_addition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ColorTypeAdditionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_color_type_addition,
        "static color type addition",
    )
    .ok()
}

pub fn parse_subjects_are_basic_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SubjectsAreBasicFact<'_>> {
    primitives::parse_all(tokens, parse_subjects_are_basic, "static basic supertype").ok()
}

pub fn parse_subject_color_tokens(tokens: &[OwnedLexToken]) -> Option<SubjectColorFact<'_>> {
    let fact = primitives::parse_all(tokens, parse_subject_color, "static subject color").ok()?;
    let trailing_subject_word = fact
        .subject_tokens
        .iter()
        .rev()
        .find_map(OwnedLexToken::as_word)?;
    // The atomic color shape must own a nominal subject, not the first half of
    // a compound predicate. For example, in "enchanted creature gets +3/+1
    // and is black", the copular splitter otherwise treats the dangling
    // "and" as part of the subject and hides the preceding anthem clause.
    if matches!(trailing_subject_word, "and" | "or" | "and/or") {
        return None;
    }
    Some(fact)
}

pub fn parse_basic_land_subtype_tokens(
    tokens: &[OwnedLexToken],
) -> Option<BasicLandSubtypeFact<'_>> {
    let fact = primitives::parse_all(
        tokens,
        parse_basic_land_subtype,
        "static basic-land subtype",
    )
    .ok()?;
    primitives::find_prefix(fact.subject_tokens, || {
        alt((semantic_kw("land"), semantic_kw("lands")))
    })?;
    Some(fact)
}

pub fn parse_land_type_addition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LandTypeAdditionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_land_type_addition,
        "static land-type addition",
    )
    .ok()
}

pub fn parse_land_animation_tokens(tokens: &[OwnedLexToken]) -> Option<LandAnimationFact<'_>> {
    primitives::parse_all(tokens, parse_land_animation, "static land animation").ok()
}

pub fn parse_other_type_addition_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<OtherTypeAdditionTailFact> {
    primitives::parse_all(
        tokens,
        (other_type_addition_tail, semantic_finish).value(OtherTypeAdditionTailFact),
        "other-type addition tail",
    )
    .ok()
}

pub fn find_loses_other_creature_types_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LosesOtherCreatureTypesFact> {
    primitives::find_prefix(tokens, || {
        alt((
            semantic_phrase(&["it", "loses", "all", "other", "creature", "types"]),
            semantic_phrase(&["this", "loses", "all", "other", "creature", "types"]),
        ))
    })
    .map(|(marker_token, _, _)| LosesOtherCreatureTypesFact { marker_token })
}

pub fn parse_base_power_toughness_grant_tokens(
    tokens: &[OwnedLexToken],
) -> Option<BasePowerToughnessGrantFact<'_>> {
    let ((power, toughness), rest) = primitives::parse_prefix(
        tokens,
        (
            semantic_phrase(&["base", "power", "and", "toughness"]),
            fixed_power_toughness,
        )
            .map(|(_, power_toughness)| power_toughness),
    )?;
    let ability_tokens = trim_sentence_edges(rest);
    (!ability_tokens.is_empty()).then_some(BasePowerToughnessGrantFact {
        power,
        toughness,
        ability_tokens,
    })
}

fn parse_subject_type_addition<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SubjectTypeAdditionFact<'a>> {
    let subject_tokens = take_until(input, 1, is_or_are)?;
    // The broad "subject is/are TYPE in addition" shape must not absorb an
    // earlier completed predicate.  Compound static lines such as
    // "equipped creature gets +1/+1 and is an artifact ..." are owned by the
    // anthem/type-addition grammar; treating everything before "is" as an
    // object filter silently turns the P/T bonus into a filter constraint.
    if subject_tokens.iter().any(|token| {
        token
            .as_word()
            .is_some_and(|word| matches!(word, "get" | "gets" | "has" | "have"))
    }) {
        return Err(primitives::backtrack_err(
            "static subject type addition",
            "subject without an earlier predicate",
        ));
    }
    is_or_are().parse_next(input)?;
    let descriptor_tokens = take_until(input, 1, || other_type_addition_tail)?;
    other_type_addition_tail.parse_next(input)?;
    semantic_finish(input)?;
    let subject_tokens = trim_sentence_edges(subject_tokens);
    let descriptor_tokens = trim_sentence_edges(descriptor_tokens);
    Ok(SubjectTypeAdditionFact {
        subject_tokens,
        descriptor_tokens,
        chosen_type: is_chosen_type(descriptor_tokens),
    })
}

fn parse_subject_card_type_identity<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SubjectCardTypeIdentityFact<'a>> {
    let subject_tokens = take_until(input, 1, is_or_are)?;
    is_or_are().parse_next(input)?;
    let descriptor_tokens = take_until(input, 1, || semantic_finish)?;
    semantic_finish(input)?;
    Ok(SubjectCardTypeIdentityFact {
        subject_tokens: trim_sentence_edges(subject_tokens),
        descriptor_tokens: trim_sentence_edges(descriptor_tokens),
    })
}

fn parse_power_toughness_type_addition<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PowerToughnessTypeAdditionFact<'a>> {
    let subject_tokens = take_until(input, 1, is_or_are)?;
    is_or_are().parse_next(input)?;
    let (power, toughness) = fixed_power_toughness(input)?;
    let descriptor_tokens = take_until(input, 1, || their_other_type_tail)?;
    their_other_type_tail.parse_next(input)?;
    semantic_finish(input)?;
    Ok(PowerToughnessTypeAdditionFact {
        subject_tokens: trim_sentence_edges(subject_tokens),
        descriptor_tokens: trim_sentence_edges(descriptor_tokens),
        power,
        toughness,
    })
}

fn parse_color_type_addition<'a>(input: &mut LexStream<'a>) -> WResult<ColorTypeAdditionFact<'a>> {
    let subject_tokens = take_until(input, 1, || semantic_kw("are"))?;
    semantic_kw("are").parse_next(input)?;
    let color = color_token(input)?;
    semantic_phrase(&["and", "are"]).parse_next(input)?;
    let descriptor_tokens = take_until(input, 1, || their_other_creature_type_tail)?;
    their_other_creature_type_tail.parse_next(input)?;
    semantic_finish(input)?;
    Ok(ColorTypeAdditionFact {
        subject_tokens: trim_sentence_edges(subject_tokens),
        descriptor_tokens: trim_sentence_edges(descriptor_tokens),
        color,
    })
}

fn parse_subjects_are_basic<'a>(input: &mut LexStream<'a>) -> WResult<SubjectsAreBasicFact<'a>> {
    let subject_tokens = take_until(input, 1, is_or_are)?;
    is_or_are().parse_next(input)?;
    semantic_kw("basic").parse_next(input)?;
    semantic_finish(input)?;
    Ok(SubjectsAreBasicFact {
        subject_tokens: trim_sentence_edges(subject_tokens),
    })
}

fn parse_subject_color<'a>(input: &mut LexStream<'a>) -> WResult<SubjectColorFact<'a>> {
    let subject_tokens = take_until(input, 1, is_or_are)?;
    is_or_are().parse_next(input)?;
    let color = alt((
        semantic_phrase(&["all", "colors"]).value(Color::ALL.into_iter().collect::<ColorSet>()),
        color_token,
    ))
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(SubjectColorFact {
        subject_tokens: trim_sentence_edges(subject_tokens),
        color,
    })
}

fn parse_basic_land_subtype<'a>(input: &mut LexStream<'a>) -> WResult<BasicLandSubtypeFact<'a>> {
    let subject_tokens = take_until(input, 1, is_or_are)?;
    is_or_are().parse_next(input)?;
    let subtype = land_subtype_token(input)?;
    semantic_finish(input)?;
    Ok(BasicLandSubtypeFact {
        subject_tokens: trim_sentence_edges(subject_tokens),
        subtype,
    })
}

fn parse_land_type_addition<'a>(input: &mut LexStream<'a>) -> WResult<LandTypeAdditionFact<'a>> {
    let subject_tokens = take_until(input, 1, is_or_are)?;
    is_or_are().parse_next(input)?;
    let subject_tokens = trim_sentence_edges(subject_tokens);
    alt((
        (
            semantic_phrase(&["every", "basic", "land"]),
            alt((semantic_kw("type"), semantic_kw("types"))),
            semantic_phrase(&["in", "addition", "to"]),
            alt((semantic_kw("its"), semantic_kw("their"))),
            semantic_kw("other"),
            alt((semantic_kw("type"), semantic_kw("types"))),
            semantic_finish,
        )
            .value(LandTypeAdditionFact::EveryBasic { subject_tokens }),
        (
            land_subtype_token,
            semantic_phrase(&["in", "addition", "to"]),
            alt((semantic_kw("its"), semantic_kw("their"))),
            semantic_kw("other"),
            semantic_kw("land"),
            alt((semantic_kw("type"), semantic_kw("types"))),
            semantic_finish,
        )
            .map(|(subtype, _, _, _, _, _, _)| LandTypeAdditionFact::One {
                subject_tokens,
                subtype,
            }),
    ))
    .parse_next(input)
}

fn parse_land_animation<'a>(input: &mut LexStream<'a>) -> WResult<LandAnimationFact<'a>> {
    let subject_tokens = take_until(input, 1, is_or_are)?;
    is_or_are().parse_next(input)?;
    let (power, toughness) = fixed_power_toughness(input)?;
    alt((semantic_kw("creature"), semantic_kw("creatures"))).parse_next(input)?;
    semantic_kw("that").parse_next(input)?;
    is_or_are().parse_next(input)?;
    semantic_kw("still").parse_next(input)?;
    alt((semantic_kw("land"), semantic_kw("lands"))).parse_next(input)?;
    semantic_finish(input)?;
    Ok(LandAnimationFact {
        subject_tokens: trim_sentence_edges(subject_tokens),
        power,
        toughness,
    })
}

fn take_until<'a, P, F>(
    input: &mut LexStream<'a>,
    minimum: usize,
    make_end: F,
) -> WResult<&'a [OwnedLexToken]>
where
    F: Fn() -> P + Copy,
    P: Parser<LexStream<'a>, (), ErrMode<ContextError>>,
{
    repeat_till::<_, _, (), _, _, _, _>(minimum.., any.void(), peek(make_end()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)
}

fn is_or_are<'a>() -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    alt((semantic_kw("is"), semantic_kw("are")))
}

fn other_type_addition_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    semantic_phrase(&["in", "addition", "to"]).parse_next(input)?;
    alt((semantic_kw("its"), semantic_kw("their"))).parse_next(input)?;
    semantic_kw("other").parse_next(input)?;
    // Oracle sometimes names the subtype family explicitly ("their other
    // creature types"). The qualifier changes only the authored surface, not
    // the additive type-layer semantics captured by this fact.
    opt(semantic_kw("creature")).parse_next(input)?;
    alt((semantic_kw("type"), semantic_kw("types")))
        .void()
        .parse_next(input)
}

fn their_other_type_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    semantic_phrase(&["in", "addition", "to", "their", "other"]).parse_next(input)?;
    alt((semantic_kw("type"), semantic_kw("types")))
        .void()
        .parse_next(input)
}

fn their_other_creature_type_tail<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    semantic_phrase(&["in", "addition", "to", "their", "other", "creature"]).parse_next(input)?;
    alt((semantic_kw("type"), semantic_kw("types")))
        .void()
        .parse_next(input)
}

fn fixed_power_toughness<'a>(input: &mut LexStream<'a>) -> WResult<(i32, i32)> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    any.verify_map(|token: &OwnedLexToken| {
        let (power, toughness) =
            leaf::parse_leaf_pt_modifier_values_complete(token.parser_text()).ok()?;
        match (power, toughness) {
            (Value::Fixed(power), Value::Fixed(toughness)) => Some((power, toughness)),
            _ => None,
        }
    })
    .parse_next(input)
}

fn color_token<'a>(input: &mut LexStream<'a>) -> WResult<ColorSet> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    any.verify_map(|token: &OwnedLexToken| {
        leaf::parse_leaf_color_complete(token.parser_text()).ok()
    })
    .parse_next(input)
}

fn land_subtype_token<'a>(input: &mut LexStream<'a>) -> WResult<Subtype> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    any.verify_map(|token: &OwnedLexToken| {
        leaf::parse_leaf_subtype_flexible_complete(token.parser_text())
            .ok()
            .filter(Subtype::is_land_subtype)
    })
    .parse_next(input)
}

fn is_chosen_type(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (semantic_phrase(&["chosen", "type"]), semantic_finish).void(),
        "chosen type descriptor",
    )
    .is_ok()
}

fn semantic_tokens_are_empty(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(tokens, semantic_finish, "semantic empty tail").is_ok()
}

fn trim_sentence_edges(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    tokens = trim_lexed_commas(tokens);
    while tokens
        .last()
        .is_some_and(|token| matches!(token.kind, TokenKind::Period | TokenKind::Semicolon))
    {
        tokens = &tokens[..tokens.len() - 1];
    }
    trim_lexed_commas(tokens)
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        while input
            .peek_token()
            .is_some_and(|token| is_semantic_noise(token) && !token_matches(token, expected))
        {
            any.parse_next(input)?;
        }
        any.verify(|token: &&OwnedLexToken| token_matches(token, expected))
            .void()
            .parse_next(input)
    }
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn semantic_noise<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| is_semantic_noise(token))
        .void()
        .parse_next(input)
}

fn is_semantic_noise(token: &OwnedLexToken) -> bool {
    token.parser_word_pieces().is_empty()
        || token.is_word("a")
        || token.is_word("an")
        || token.is_word("the")
}

fn token_matches(token: &OwnedLexToken, expected: &str) -> bool {
    token.is_word(expected)
        || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
}

fn semantic_finish<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    eof.void().parse_next(input)
}

#[cfg(test)]
#[path = "type_and_color_inline_tests.rs"]
mod tests;
