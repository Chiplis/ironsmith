use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::KeywordAction;
use crate::color::ColorSet;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommanderCreatureSubject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnthemGoadedShape {
    pub get_token: usize,
    pub and_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColoredSpellProtectionShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubjectColorAndGrantShape<'a> {
    pub condition_tokens: Option<&'a [OwnedLexToken]>,
    pub subject_tokens: &'a [OwnedLexToken],
    pub color: ColorSet,
    pub ability_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnthemNoDefenderGrantShape {
    pub get_token: usize,
    pub anthem_end: usize,
}

pub fn parse_commander_creature_subject_tokens(
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

pub fn parse_anthem_goaded_shape(tokens: &[OwnedLexToken]) -> Option<AnthemGoadedShape> {
    primitives::parse_all(tokens, parse_anthem_goaded_lexed, "anthem and goaded line").ok()
}

pub fn parse_colored_spell_protection_tokens(
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

pub fn parse_subject_color_and_grant_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SubjectColorAndGrantShape<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (condition_tokens, clause_tokens) = split_leading_condition(tokens)?;
    let (color_tokens, ability_tokens) =
        primitives::split_lexed_once_on_separator(clause_tokens, || {
            (
                primitives::kw("and"),
                alt((primitives::kw("has"), primitives::kw("have"))),
            )
                .void()
        })?;
    let (subject_tokens, color) = primitives::parse_all(
        color_tokens,
        parse_subject_color_assignment,
        "subject color assignment",
    )
    .ok()?;
    let ability_tokens = super::trim_anthem_clause_tokens(ability_tokens);
    (!subject_tokens.is_empty() && !ability_tokens.is_empty()).then_some(
        SubjectColorAndGrantShape {
            condition_tokens,
            subject_tokens,
            color,
            ability_tokens,
        },
    )
}

pub fn parse_anthem_no_defender_grant_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AnthemNoDefenderGrantShape> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (anthem_tokens, ()) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        (
            primitives::kw("and"),
            primitives::phrase(&["can", "attack", "as", "though", "it"]),
            alt((
                primitives::phrase(&["didn't", "have"]),
                primitives::phrase(&["didnt", "have"]),
                primitives::phrase(&["did", "not", "have"]),
            )),
            primitives::kw("defender"),
            primitives::sentence_end(),
        )
            .void()
    })?;
    let get_token = primitives::find_prefix(anthem_tokens, || {
        alt((primitives::kw("get"), primitives::kw("gets"))).void()
    })?
    .0;
    Some(AnthemNoDefenderGrantShape {
        get_token,
        anthem_end: anthem_tokens.len(),
    })
}

pub fn parse_no_defender_granted_fragment_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            primitives::phrase(&["can", "attack", "as", "though", "it"]),
            alt((
                primitives::phrase(&["didn't", "have"]),
                primitives::phrase(&["didnt", "have"]),
                primitives::phrase(&["did", "not", "have"]),
            )),
            primitives::kw("defender"),
            primitives::sentence_end(),
        ),
        "granted no-defender attack fragment",
    )
    .is_ok()
}

pub fn parse_unblockable_keyword_fragment_tokens(
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

fn split_leading_condition(
    tokens: &[OwnedLexToken],
) -> Option<(Option<&[OwnedLexToken]>, &[OwnedLexToken])> {
    let Some((_, after_prefix)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"]))
    else {
        return Some((None, tokens));
    };
    let (condition_tokens, clause_tokens) =
        primitives::split_lexed_once_on_separator(after_prefix, || primitives::comma().void())?;
    let condition_tokens = super::trim_anthem_clause_tokens(condition_tokens);
    let clause_tokens = super::trim_anthem_clause_tokens(clause_tokens);
    if condition_tokens.is_empty() || clause_tokens.is_empty() {
        return None;
    }
    Some((Some(condition_tokens), clause_tokens))
}

fn parse_subject_color_assignment<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(&'a [OwnedLexToken], ColorSet)> {
    let subject_tokens = repeat_till(
        1..,
        any.void(),
        peek(alt((primitives::kw("is"), primitives::kw("are")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("is"), primitives::kw("are"))).parse_next(input)?;
    let color_word = primitives::word_parser_text.parse_next(input)?;
    let color = leaf::parse_leaf_color_complete(color_word)
        .map_err(|_| primitives::backtrack_err("subject color", "color word"))?;
    eof.parse_next(input)?;
    Ok((super::trim_anthem_clause_tokens(subject_tokens), color))
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

        let tokens = lex_line(
            "As long as there are seven or more cards in your graveyard, this creature is white and has \"{T}: Destroy target black creature.\"",
            0,
        )
        .unwrap();
        let shape = parse_subject_color_and_grant_tokens(&tokens).unwrap();
        assert!(shape.condition_tokens.is_some());
        assert_eq!(shape.color, ColorSet::WHITE);
        assert!(!shape.ability_tokens.is_empty());

        let tokens = lex_line(
            "As long as you control three or more artifacts, this creature gets +2/+2 and can attack as though it didn't have defender.",
            0,
        )
        .unwrap();
        let shape = parse_anthem_no_defender_grant_tokens(&tokens).expect("no-defender grant");
        assert!(shape.anthem_end > shape.get_token);

        let tokens = lex_line("can attack as though it didn't have defender", 0).unwrap();
        assert!(parse_no_defender_granted_fragment_tokens(&tokens));
    }
}
