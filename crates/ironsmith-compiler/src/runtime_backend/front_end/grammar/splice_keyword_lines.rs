use winnow::combinator::{alt, opt, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::CardTextError;
use crate::mana::ManaCost;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpliceSubject {
    Arcane,
    InstantOrSorcery,
}

impl SpliceSubject {
    pub(crate) fn oracle_surface(self) -> &'static str {
        match self {
            Self::Arcane => "Arcane",
            Self::InstantOrSorcery => "instant or sorcery",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpliceKeywordLineShape {
    pub(crate) subject: SpliceSubject,
    pub(crate) cost: ManaCost,
}

pub(crate) fn parse_splice_keyword_line_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<SpliceKeywordLineShape>, CardTextError> {
    primitives::parse_all_or_none(
        tokens,
        parse_splice_keyword_line_lexed,
        "splice-keyword-line",
    )
}

fn parse_splice_keyword_line_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SpliceKeywordLineShape> {
    primitives::phrase(&["splice", "onto"]).parse_next(input)?;
    let subject = alt((
        primitives::kw("arcane").value(SpliceSubject::Arcane),
        primitives::phrase(&["instant", "or", "sorcery"]).value(SpliceSubject::InstantOrSorcery),
    ))
    .parse_next(input)?;
    let cost = leaf::parse_leaf_mana_cost_lexed.parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    opt(parse_parenthetical_reminder_lexed)
        .void()
        .parse_next(input)?;
    Ok(SpliceKeywordLineShape { subject, cost })
}

fn parse_parenthetical_reminder_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::token_kind(TokenKind::LParen)
        .void()
        .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::token_kind(TokenKind::RParen).void(),
    )
    .void()
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_arcane_and_instant_or_sorcery_subjects_with_typed_costs() {
        for (text, expected_subject, expected_cost) in [
            ("Splice onto Arcane {1}{R}", SpliceSubject::Arcane, "{1}{R}"),
            (
                "Splice onto instant or sorcery {2}{U} (As you cast an instant or sorcery spell, you may reveal this card.)",
                SpliceSubject::InstantOrSorcery,
                "{2}{U}",
            ),
        ] {
            let tokens = lex_line(text, 0).expect("lex splice line");
            let parsed = parse_splice_keyword_line_tokens(&tokens)
                .expect("parse splice line")
                .expect("recognize splice line");
            assert_eq!(parsed.subject, expected_subject);
            assert_eq!(parsed.cost.to_oracle(), expected_cost);
        }
    }

    #[test]
    fn rejects_unknown_subjects_and_non_mana_cost_surfaces() {
        for text in [
            "Splice onto creature {1}{G}",
            "Splice onto Arcane—Sacrifice two Mountains.",
        ] {
            let tokens = lex_line(text, 0).expect("lex unsupported splice line");
            assert!(
                parse_splice_keyword_line_tokens(&tokens)
                    .expect("unsupported surface should backtrack")
                    .is_none(),
                "unexpectedly recognized {text}"
            );
        }
    }
}
