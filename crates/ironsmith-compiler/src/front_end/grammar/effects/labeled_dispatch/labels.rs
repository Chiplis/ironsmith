use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::rest;

use crate::lexer::{LexStream, OwnedLexToken, TokenKind};

use super::super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeadingEffectLabelKind {
    Conditional,
    SupportedProcedure,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeadingEffectLabelShape<'a> {
    pub(crate) kind: LeadingEffectLabelKind,
    pub(crate) label_tokens: &'a [OwnedLexToken],
    pub(crate) body_tokens: &'a [OwnedLexToken],
}

fn label_delimiter(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::token_kind(TokenKind::Dash),
        primitives::token_kind(TokenKind::EmDash),
    ))
    .void()
    .parse_next(input)
}

fn short_label_atom(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::word_parser_text.void(),
        primitives::token_kind(TokenKind::Bang).void(),
    ))
    .parse_next(input)
}

fn leading_effect_label<'a>(input: &mut LexStream<'a>) -> WResult<LeadingEffectLabelShape<'a>> {
    let label_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1..=5, short_label_atom, peek(label_delimiter))
            .map(|((), ())| ())
            .take()
            .parse_next(input)?;
    label_delimiter.parse_next(input)?;
    let body_tokens = rest.parse_next(input)?;
    if body_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "leading effect label",
            "non-empty effect body",
        ));
    }

    let kind = if primitives::parse_all(
        label_tokens,
        super::super::conditional_label_phrase,
        "conditional effect label",
    )
    .is_ok()
    {
        LeadingEffectLabelKind::Conditional
    } else if primitives::parse_all(
        label_tokens,
        supported_procedure_label,
        "supported procedure label",
    )
    .is_ok()
    {
        LeadingEffectLabelKind::SupportedProcedure
    } else {
        LeadingEffectLabelKind::Unknown
    };

    Ok(LeadingEffectLabelShape {
        kind,
        label_tokens,
        body_tokens,
    })
}

fn supported_procedure_label(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::phrase(&["tempting", "offer"]),
        primitives::phrase(&["will", "of", "the", "council"]),
        primitives::phrase(&["council's", "dilemma"]),
        primitives::phrase(&["parley"]),
        primitives::phrase(&["join", "forces"]),
    ))
    .void()
    .parse_next(input)?;
    eof.void().parse_next(input)
}

pub(crate) fn parse_leading_effect_label_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LeadingEffectLabelShape<'_>> {
    primitives::parse_all(tokens, leading_effect_label, "leading effect label").ok()
}

#[cfg(test)]
mod tests {
    use crate::lexer::{lex_line, render_token_slice};

    use super::*;

    #[test]
    fn classifies_known_and_unknown_short_leading_labels() {
        let conditional = lex_line("Spell mastery — Draw a card.", 0).unwrap();
        let shape = parse_leading_effect_label_tokens(&conditional).unwrap();
        assert_eq!(shape.kind, LeadingEffectLabelKind::Conditional);
        assert_eq!(render_token_slice(shape.body_tokens), "Draw a card.");

        let unknown = lex_line("Mystery — Draw a card.", 0).unwrap();
        let shape = parse_leading_effect_label_tokens(&unknown).unwrap();
        assert_eq!(shape.kind, LeadingEffectLabelKind::Unknown);

        let embedded = lex_line(
            "Create a token with \"Landfall — Whenever a land enters, draw a card.\"",
            0,
        )
        .unwrap();
        assert!(parse_leading_effect_label_tokens(&embedded).is_none());
    }
}
