use winnow::combinator::{alt, opt, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, take_till};

use crate::cards::builders::CardTextError;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceSubject {
    Arcane,
    InstantOrSorcery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpliceKeywordLineShape<'a> {
    pub subject: SpliceSubject,
    pub cost_tokens: &'a [OwnedLexToken],
}

pub fn parse_splice_keyword_line_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Result<Option<SpliceKeywordLineShape<'a>>, CardTextError> {
    primitives::parse_all_or_none(
        tokens,
        parse_splice_keyword_line_lexed,
        "splice-keyword-line",
    )
}

fn parse_splice_keyword_line_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<SpliceKeywordLineShape<'a>> {
    primitives::phrase(&["splice", "onto"]).parse_next(input)?;
    let subject = alt((
        primitives::kw("arcane").value(SpliceSubject::Arcane),
        primitives::phrase(&["instant", "or", "sorcery"]).value(SpliceSubject::InstantOrSorcery),
    ))
    .parse_next(input)?;
    let cost_tokens = take_till(1.., |token: &OwnedLexToken| token.kind == TokenKind::LParen)
        .parse_next(input)?;
    opt(parse_parenthetical_reminder_lexed)
        .void()
        .parse_next(input)?;
    Ok(SpliceKeywordLineShape {
        subject,
        cost_tokens,
    })
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
    use super::super::super::lexer::{lex_line, render_token_slice};
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
            assert_eq!(render_token_slice(parsed.cost_tokens).trim(), expected_cost);
        }
    }

    #[test]
    fn rejects_unknown_subjects_but_preserves_non_mana_cost_tokens() {
        {
            let text = "Splice onto creature {1}{G}";
            let tokens = lex_line(text, 0).expect("lex unsupported splice line");
            assert!(
                parse_splice_keyword_line_tokens(&tokens)
                    .expect("unsupported surface should backtrack")
                    .is_none(),
                "unexpectedly recognized {text}"
            );
        }

        let tokens = lex_line("Splice onto Arcane—Sacrifice two Mountains.", 0)
            .expect("lex nonmana splice line");
        let parsed = parse_splice_keyword_line_tokens(&tokens)
            .expect("parse nonmana splice line")
            .expect("recognize nonmana splice line");
        assert_eq!(
            render_token_slice(parsed.cost_tokens).trim(),
            "—Sacrifice two Mountains."
        );
    }
}
