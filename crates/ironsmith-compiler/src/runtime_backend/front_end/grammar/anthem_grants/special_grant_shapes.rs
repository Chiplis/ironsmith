use winnow::combinator::{alt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::KeywordAction;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommanderCreatureSubject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnthemGoadedShape {
    pub(crate) get_token: usize,
    pub(crate) and_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ColoredSpellProtectionShape;

pub(crate) fn parse_commander_creature_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CommanderCreatureSubject> {
    primitives::parse_all(
        tokens,
        alt((
            primitives::phrase(&["commander", "creature", "you", "own"]),
            primitives::phrase(&["commander", "creatures", "you", "own"]),
            primitives::phrase(&["commander", "creature", "cards", "you", "own"]),
            primitives::phrase(&["commander", "creature", "card", "you", "own"]),
        )),
        "commander creature anthem subject",
    )
    .ok()
    .map(|()| CommanderCreatureSubject)
}

pub(crate) fn parse_anthem_goaded_shape(tokens: &[OwnedLexToken]) -> Option<AnthemGoadedShape> {
    primitives::parse_all(tokens, parse_anthem_goaded_lexed, "anthem and goaded line").ok()
}

pub(crate) fn parse_colored_spell_protection_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ColoredSpellProtectionShape> {
    primitives::parse_all(
        tokens,
        (
            primitives::phrase(&[
                "protection",
                "from",
                "spells",
                "that",
                "are",
                "one",
                "or",
                "more",
                "colors",
            ]),
            primitives::sentence_end(),
        ),
        "protection from colored spells",
    )
    .ok()
    .map(|_| ColoredSpellProtectionShape)
}

pub(crate) fn parse_unblockable_keyword_fragment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordAction> {
    primitives::parse_all(
        tokens,
        (
            alt((
                primitives::phrase(&["can't", "be", "blocked"]),
                primitives::phrase(&["cant", "be", "blocked"]),
                primitives::phrase(&["cannot", "be", "blocked"]),
            )),
            primitives::sentence_end(),
        ),
        "unblockable granted keyword fragment",
    )
    .ok()
    .map(|_| KeywordAction::Unblockable)
}

fn parse_anthem_goaded_lexed<'a>(input: &mut LexStream<'a>) -> WResult<AnthemGoadedShape> {
    let initial_len = input.len();
    repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::kw("get"), primitives::kw("gets")))),
    )
    .void()
    .parse_next(input)?;
    let get_token = initial_len.saturating_sub(input.len());
    alt((primitives::kw("get"), primitives::kw("gets"))).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("and")))
        .void()
        .parse_next(input)?;
    let and_token = initial_len.saturating_sub(input.len());
    primitives::kw("and").parse_next(input)?;
    alt((
        primitives::phrase(&["is", "goaded"]),
        primitives::phrase(&["are", "goaded"]),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(AnthemGoadedShape {
        get_token,
        and_token,
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_special_anthem_and_grant_shapes() {
        let tokens = lex_line("Commander creature cards you own", 0).unwrap();
        assert!(parse_commander_creature_subject_tokens(&tokens).is_some());

        let tokens = lex_line("Enchanted creature gets +2/+2 and is goaded.", 0).unwrap();
        let shape = parse_anthem_goaded_shape(&tokens).unwrap();
        assert!(shape.and_token > shape.get_token);

        let tokens = lex_line("Protection from spells that are one or more colors.", 0).unwrap();
        assert!(parse_colored_spell_protection_tokens(&tokens).is_some());

        let tokens = lex_line("can't be blocked", 0).unwrap();
        assert_eq!(
            parse_unblockable_keyword_fragment_tokens(&tokens),
            Some(KeywordAction::Unblockable)
        );
    }
}
