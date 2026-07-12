use winnow::combinator::{alt, eof, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::color::{Color, ColorSet};
use crate::effect::Value;
use crate::types::Subtype;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkipYourUpkeepTail<'a> {
    None,
    Condition(&'a [OwnedLexToken]),
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkipYourUpkeepFact<'a> {
    pub(crate) tail: SkipYourUpkeepTail<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SubjectTypeAdditionFact<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) descriptor_tokens: &'a [OwnedLexToken],
    pub(crate) chosen_type: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChosenColorAdditionFact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PowerToughnessTypeAdditionFact<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) descriptor_tokens: &'a [OwnedLexToken],
    pub(crate) power: i32,
    pub(crate) toughness: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ColorTypeAdditionFact<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) descriptor_tokens: &'a [OwnedLexToken],
    pub(crate) color: ColorSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SubjectsAreBasicFact<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SubjectColorFact<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) color: ColorSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BasicLandSubtypeFact<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) subtype: Subtype,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LandTypeAdditionFact<'a> {
    EveryBasic {
        subject_tokens: &'a [OwnedLexToken],
    },
    One {
        subject_tokens: &'a [OwnedLexToken],
        subtype: Subtype,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LandAnimationFact<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) power: i32,
    pub(crate) toughness: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OtherTypeAdditionTailFact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LosesOtherCreatureTypesFact {
    pub(crate) marker_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BasePowerToughnessGrantFact<'a> {
    pub(crate) power: i32,
    pub(crate) toughness: i32,
    pub(crate) ability_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_skip_your_upkeep_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SkipYourUpkeepFact<'_>> {
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

pub(crate) fn parse_subject_type_addition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SubjectTypeAdditionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_subject_type_addition,
        "static subject type addition",
    )
    .ok()
}

pub(crate) fn parse_all_cards_chosen_color_addition_tokens(
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

pub(crate) fn parse_power_toughness_type_addition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PowerToughnessTypeAdditionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_power_toughness_type_addition,
        "static power-toughness type addition",
    )
    .ok()
}

pub(crate) fn parse_color_type_addition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ColorTypeAdditionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_color_type_addition,
        "static color type addition",
    )
    .ok()
}

pub(crate) fn parse_subjects_are_basic_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SubjectsAreBasicFact<'_>> {
    primitives::parse_all(tokens, parse_subjects_are_basic, "static basic supertype").ok()
}

pub(crate) fn parse_subject_color_tokens(tokens: &[OwnedLexToken]) -> Option<SubjectColorFact<'_>> {
    primitives::parse_all(tokens, parse_subject_color, "static subject color").ok()
}

pub(crate) fn parse_basic_land_subtype_tokens(
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

pub(crate) fn parse_land_type_addition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LandTypeAdditionFact<'_>> {
    primitives::parse_all(
        tokens,
        parse_land_type_addition,
        "static land-type addition",
    )
    .ok()
}

pub(crate) fn parse_land_animation_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LandAnimationFact<'_>> {
    primitives::parse_all(tokens, parse_land_animation, "static land animation").ok()
}

pub(crate) fn parse_other_type_addition_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<OtherTypeAdditionTailFact> {
    primitives::parse_all(
        tokens,
        (other_type_addition_tail, semantic_finish).value(OtherTypeAdditionTailFact),
        "other-type addition tail",
    )
    .ok()
}

pub(crate) fn find_loses_other_creature_types_tokens(
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

pub(crate) fn parse_base_power_toughness_grant_tokens(
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
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::{lex_line, render_token_slice};

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("static type-and-color fixture should lex")
    }

    #[test]
    fn typed_static_grant_migration_parses_type_and_color_facts() {
        let skip_tokens = lex("Skip your upkeep step if you control no creatures.");
        let skip = parse_skip_your_upkeep_tokens(&skip_tokens).unwrap();
        let SkipYourUpkeepTail::Condition(condition) = skip.tail else {
            panic!("expected a typed skip-upkeep condition")
        };
        assert_eq!(render_token_slice(condition), "you control no creatures");

        let addition_tokens =
            lex("Lands you control are the chosen type in addition to their other types.");
        let addition = parse_subject_type_addition_tokens(&addition_tokens).unwrap();
        assert!(addition.chosen_type);
        assert_eq!(
            render_token_slice(addition.subject_tokens),
            "Lands you control"
        );

        let animation_tokens = lex("Lands you control are 3/3 creatures that are still lands.");
        let animation = parse_land_animation_tokens(&animation_tokens).unwrap();
        assert_eq!((animation.power, animation.toughness), (3, 3));

        let base_tokens = lex("base power and toughness 4/4, flying and vigilance");
        let base = parse_base_power_toughness_grant_tokens(&base_tokens).unwrap();
        assert_eq!((base.power, base.toughness), (4, 4));
        assert_eq!(
            render_token_slice(base.ability_tokens),
            "flying and vigilance"
        );

        let color_tokens = lex("All creatures are all colors.");
        let color = parse_subject_color_tokens(&color_tokens).unwrap();
        assert_eq!(color.color, Color::ALL.into_iter().collect::<ColorSet>());

        let subtype_tokens = lex("Nonbasic lands are Mountains.");
        let subtype = parse_basic_land_subtype_tokens(&subtype_tokens).unwrap();
        assert_eq!(subtype.subtype, Subtype::Mountain);

        let pt_addition_tokens =
            lex("All lands are 2/2 blue creatures in addition to their other types.");
        let pt_addition = parse_power_toughness_type_addition_tokens(&pt_addition_tokens).unwrap();
        assert_eq!((pt_addition.power, pt_addition.toughness), (2, 2));
        assert_eq!(
            render_token_slice(pt_addition.descriptor_tokens),
            "blue creatures"
        );

        let color_addition_tokens =
            lex("All creatures are blue and are Frogs in addition to their other creature types.");
        let color_addition = parse_color_type_addition_tokens(&color_addition_tokens).unwrap();
        assert_eq!(
            render_token_slice(color_addition.descriptor_tokens),
            "Frogs"
        );

        assert!(
            parse_all_cards_chosen_color_addition_tokens(&lex(
                "All cards that aren't on the battlefield, spells, and permanents are the chosen color in addition to their other colors."
            ))
            .is_some()
        );

        assert!(matches!(
            parse_land_type_addition_tokens(&lex(
                "Lands you control are every basic land type in addition to their other types."
            )),
            Some(LandTypeAdditionFact::EveryBasic { .. })
        ));
    }
}
