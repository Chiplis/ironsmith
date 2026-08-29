use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use super::super::lexer::{LexStream, OwnedLexToken, render_token_slice};
use super::primitives;

const ACTIVATE_ONLY_ONCE_EACH_TURN: &[&str] = &["activate", "only", "once", "each", "turn"];
const ACTIVATE_ONLY_ONCE_EACH_TURN_AND: &[&str] =
    &["activate", "only", "once", "each", "turn", "and"];
const AND_ONLY_ONCE_EACH_TURN: &[&str] = &["and", "only", "once", "each", "turn"];
const ACTIVATE_ONLY_IF: &[&str] = &["activate", "only", "if"];
const ONLY_IF: &[&str] = &["only", "if"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationRestrictionNormalization {
    Redundant,
    Residual(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOnlyActivationRestriction {
    SourceDidNotAttackThisTurn,
    SourceAttackedThisTurn,
}

pub fn parse_once_per_turn_activation_restriction_tokens(
    tokens: &[OwnedLexToken],
) -> ActivationRestrictionNormalization {
    let mut input = LexStream::new(tokens);
    alt((parse_redundant_restriction, parse_residual_restriction))
        .parse_next(&mut input)
        .unwrap_or(ActivationRestrictionNormalization::Redundant)
}

pub fn parse_text_only_activation_restriction_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TextOnlyActivationRestriction> {
    primitives::parse_all(
        tokens,
        parse_text_only_activation_restriction_lexed,
        "text-only activation restriction",
    )
    .ok()
}

fn parse_text_only_activation_restriction_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TextOnlyActivationRestriction> {
    opt(primitives::phrase(ACTIVATE_ONLY_ONCE_EACH_TURN_AND)).parse_next(input)?;
    opt(alt((
        primitives::phrase(ACTIVATE_ONLY_IF),
        primitives::phrase(ONLY_IF),
    )))
    .parse_next(input)?;

    let restriction = alt((
        parse_did_not_attack_this_turn
            .value(TextOnlyActivationRestriction::SourceDidNotAttackThisTurn),
        (parse_source_subject, parse_did_not_attack_this_turn)
            .value(TextOnlyActivationRestriction::SourceDidNotAttackThisTurn),
        (
            parse_source_subject,
            primitives::phrase(&["attacked", "this", "turn"]),
        )
            .value(TextOnlyActivationRestriction::SourceAttackedThisTurn),
    ))
    .parse_next(input)?;

    opt(primitives::phrase(AND_ONLY_ONCE_EACH_TURN)).parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(restriction)
}

fn parse_did_not_attack_this_turn<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["didn't", "attack", "this", "turn"]),
        primitives::phrase(&["didnt", "attack", "this", "turn"]),
        primitives::phrase(&["did", "not", "attack", "this", "turn"]),
        primitives::phrase(&["has", "not", "attacked", "this", "turn"]),
        primitives::phrase(&["hasnt", "attacked", "this", "turn"]),
        primitives::phrase(&["hasn't", "attacked", "this", "turn"]),
    ))
    .parse_next(input)
}

fn parse_source_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["this", "creature"]),
        primitives::kw("it").void(),
        primitives::phrase(&["that", "creature"]),
    ))
    .parse_next(input)
}

fn parse_redundant_restriction<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivationRestrictionNormalization> {
    primitives::phrase(ACTIVATE_ONLY_ONCE_EACH_TURN).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ActivationRestrictionNormalization::Redundant)
}

fn parse_residual_restriction<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivationRestrictionNormalization> {
    opt(primitives::phrase(ACTIVATE_ONLY_ONCE_EACH_TURN_AND)).parse_next(input)?;
    let residual = repeat_till(0.., any.void(), peek(primitives::sentence_end()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    let residual = residual_without_once_per_turn_suffix(residual);
    let rendered = render_token_slice(residual);
    let normalized = lowercase_first_ascii(rendered.trim());
    if normalized.is_empty() {
        Ok(ActivationRestrictionNormalization::Redundant)
    } else {
        Ok(ActivationRestrictionNormalization::Residual(normalized))
    }
}

fn residual_without_once_per_turn_suffix(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut input = LexStream::new(tokens);
    alt((parse_residual_before_once_per_turn_suffix, rest))
        .parse_next(&mut input)
        .unwrap_or(tokens)
}

fn parse_residual_before_once_per_turn_suffix<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
    let residual = repeat_till(
        0..,
        any.void(),
        peek((primitives::phrase(AND_ONLY_ONCE_EACH_TURN), eof)),
    )
    .map(|((), ((), _))| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(AND_ONLY_ONCE_EACH_TURN).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(residual)
}

fn lowercase_first_ascii(text: &str) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut lowered = first.to_ascii_lowercase().to_string();
    lowered.push_str(chars.as_str());
    lowered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn parse(raw: &str) -> ActivationRestrictionNormalization {
        let tokens = lex_line(raw, 0).expect("restriction should lex");
        parse_once_per_turn_activation_restriction_tokens(&tokens)
    }

    #[test]
    fn recognizes_a_redundant_once_per_turn_restriction() {
        assert_eq!(
            parse("Activate only once each turn."),
            ActivationRestrictionNormalization::Redundant
        );
    }

    #[test]
    fn removes_the_leading_once_per_turn_envelope() {
        assert_eq!(
            parse("Activate only once each turn and Only if this creature attacked this turn."),
            ActivationRestrictionNormalization::Residual(
                "only if this creature attacked this turn".to_string()
            )
        );
    }

    #[test]
    fn removes_the_trailing_once_per_turn_envelope() {
        assert_eq!(
            parse("Activate only if it didn't attack this turn and only once each turn."),
            ActivationRestrictionNormalization::Residual(
                "activate only if it didn't attack this turn".to_string()
            )
        );
    }

    #[test]
    fn preserves_punctuation_within_the_residual_clause() {
        assert_eq!(
            parse(
                "Activate only once each turn and Only if you control an Elf, a Dwarf, or a Human."
            ),
            ActivationRestrictionNormalization::Residual(
                "only if you control an Elf, a Dwarf, or a Human".to_string()
            )
        );
    }

    #[test]
    fn parses_typed_attack_state_restrictions_with_optional_framing() {
        let did_not = lex_line(
            "Activate only once each turn and only if this creature didn't attack this turn.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_text_only_activation_restriction_tokens(&did_not),
            Some(TextOnlyActivationRestriction::SourceDidNotAttackThisTurn)
        );

        let attacked = lex_line(
            "Activate only if that creature attacked this turn and only once each turn.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_text_only_activation_restriction_tokens(&attacked),
            Some(TextOnlyActivationRestriction::SourceAttackedThisTurn)
        );
    }
}
