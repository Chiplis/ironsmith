use winnow::combinator::{alt, eof, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachedSubject {
    EnchantedCreature,
    EnchantedPermanent,
    EnchantedLand,
    EnchantedArtifact,
    EnchantedEquipment,
    EquippedCreature,
    EquippedPermanent,
}

impl AttachedSubject {
    pub fn display(self) -> &'static str {
        match self {
            Self::EnchantedCreature => "enchanted creature",
            Self::EnchantedPermanent => "enchanted permanent",
            Self::EnchantedLand => "enchanted land",
            Self::EnchantedArtifact => "enchanted artifact",
            Self::EnchantedEquipment => "enchanted equipment",
            Self::EquippedCreature => "equipped creature",
            Self::EquippedPermanent => "equipped permanent",
        }
    }

    pub fn is_equipped(self) -> bool {
        matches!(self, Self::EquippedCreature | Self::EquippedPermanent)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedHasSpec<'a> {
    pub subject: AttachedSubject,
    pub subject_tokens: &'a [OwnedLexToken],
    pub ability_tokens: &'a [OwnedLexToken],
    pub has_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachedConditionSuffix<'a> {
    None {
        ability_tokens: &'a [OwnedLexToken],
    },
    Clause {
        ability_tokens: &'a [OwnedLexToken],
        condition_tokens: &'a [OwnedLexToken],
    },
    YourTurn {
        ability_tokens: &'a [OwnedLexToken],
    },
    OtherTurns {
        ability_tokens: &'a [OwnedLexToken],
    },
}

/// Removes the grammatical subject/copula at the start of an attached-object
/// condition. The returned tail can be prefixed with the explicit attached
/// subject without reinterpreting a word slice in the semantic caller.
pub fn strip_attached_condition_pronoun(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, tail) = primitives::parse_prefix(
        tokens,
        alt((
            semantic_phrase(&["it", "is"]),
            semantic_phrase(&["it", "s"]),
        )),
    )?;
    Some(tail)
}

impl<'a> AttachedConditionSuffix<'a> {
    pub fn ability_tokens(self) -> &'a [OwnedLexToken] {
        match self {
            Self::None { ability_tokens }
            | Self::Clause { ability_tokens, .. }
            | Self::YourTurn { ability_tokens }
            | Self::OtherTurns { ability_tokens } => ability_tokens,
        }
    }
}

pub fn parse_attached_has_tokens(tokens: &[OwnedLexToken]) -> Option<AttachedHasSpec<'_>> {
    let initial_len = tokens.len();
    let mut input = LexStream::new(tokens);
    let subject = parse_attached_subject_lexed(&mut input).ok()?;
    let has_token = initial_len.checked_sub(input.len())?;
    semantic_kw("has").parse_next(&mut input).ok()?;
    let ability_start = initial_len.checked_sub(input.len())?;
    let ability_tokens = tokens.get(ability_start..)?;
    if ability_tokens.is_empty() {
        return None;
    }
    Some(AttachedHasSpec {
        subject,
        subject_tokens: tokens.get(..has_token)?,
        ability_tokens,
        has_token,
    })
}

pub fn parse_equipped_creature_has_tokens(tokens: &[OwnedLexToken]) -> Option<AttachedHasSpec<'_>> {
    let parsed = parse_attached_has_tokens(tokens)?;
    (parsed.subject == AttachedSubject::EquippedCreature).then_some(parsed)
}

pub fn parse_enchanted_has_tokens(tokens: &[OwnedLexToken]) -> Option<AttachedHasSpec<'_>> {
    let parsed = parse_attached_has_tokens(tokens)?;
    matches!(
        parsed.subject,
        AttachedSubject::EnchantedCreature | AttachedSubject::EnchantedPermanent
    )
    .then_some(parsed)
}

pub fn parse_chosen_landwalk_tokens(tokens: &[OwnedLexToken]) -> Option<bool> {
    primitives::parse_all(
        tokens,
        (
            alt((
                (semantic_kw("snow"), semantic_kw("landwalk")).value(true),
                semantic_kw("landwalk").value(false),
            )),
            semantic_phrase(&["of", "the", "chosen", "type"]),
            semantic_finish,
        )
            .map(|(snow, (), ())| snow),
        "attached chosen landwalk",
    )
    .ok()
}

pub fn split_attached_condition_suffix_tokens(
    tokens: &[OwnedLexToken],
) -> AttachedConditionSuffix<'_> {
    if let Ok((ability_tokens, condition_tokens)) = primitives::parse_all(
        tokens,
        parse_as_long_as_suffix_lexed,
        "attached as-long-as suffix",
    ) {
        return AttachedConditionSuffix::Clause {
            ability_tokens: trim_lexed_commas(ability_tokens),
            condition_tokens: trim_lexed_commas(condition_tokens),
        };
    }
    if let Ok(ability_tokens) = primitives::parse_all(
        tokens,
        parse_your_turn_suffix_lexed,
        "attached during-your-turn suffix",
    ) {
        return AttachedConditionSuffix::YourTurn {
            ability_tokens: trim_lexed_commas(ability_tokens),
        };
    }
    if let Ok(ability_tokens) = primitives::parse_all(
        tokens,
        parse_other_turns_suffix_lexed,
        "attached during-other-turns suffix",
    ) {
        return AttachedConditionSuffix::OtherTurns {
            ability_tokens: trim_lexed_commas(ability_tokens),
        };
    }
    AttachedConditionSuffix::None {
        ability_tokens: trim_lexed_commas(tokens),
    }
}

pub(super) fn parse_attached_subject_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedSubject> {
    alt((
        (semantic_kw("enchanted"), semantic_kw("creature"))
            .value(AttachedSubject::EnchantedCreature),
        (semantic_kw("enchanted"), semantic_kw("permanent"))
            .value(AttachedSubject::EnchantedPermanent),
        (semantic_kw("enchanted"), semantic_kw("land")).value(AttachedSubject::EnchantedLand),
        (semantic_kw("enchanted"), semantic_kw("artifact"))
            .value(AttachedSubject::EnchantedArtifact),
        (semantic_kw("enchanted"), semantic_kw("equipment"))
            .value(AttachedSubject::EnchantedEquipment),
        (semantic_kw("equipped"), semantic_kw("creature")).value(AttachedSubject::EquippedCreature),
        (semantic_kw("equipped"), semantic_kw("permanent"))
            .value(AttachedSubject::EquippedPermanent),
    ))
    .parse_next(input)
}

pub(super) fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    (
        repeat::<_, _, (), _, _>(
            0..,
            any.verify(move |token: &&OwnedLexToken| {
                (token.parser_word_pieces().is_empty()
                    || token.is_word("a")
                    || token.is_word("an")
                    || token.is_word("the"))
                    && !token.is_word(expected)
            })
            .void(),
        ),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

pub(super) fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

pub(super) fn semantic_noise<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| {
        token.parser_word_pieces().is_empty()
            || token.is_word("a")
            || token.is_word("an")
            || token.is_word("the")
    })
    .void()
    .parse_next(input)
}

pub(super) fn semantic_finish<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    eof.void().parse_next(input)
}

fn parse_as_long_as_suffix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    let ability_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(semantic_phrase(&["as", "long", "as"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    semantic_phrase(&["as", "long", "as"]).parse_next(input)?;
    let condition_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((repeat::<_, _, (), _, _>(0.., semantic_noise), eof)),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok((ability_tokens, condition_tokens))
}

fn parse_your_turn_suffix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let ability_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(semantic_phrase(&["during", "your", "turn"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    semantic_phrase(&["during", "your", "turn"]).parse_next(input)?;
    semantic_finish(input)?;
    Ok(ability_tokens)
}

fn parse_other_turns_suffix_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let ability_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(semantic_phrase(&[
            "during", "turns", "other", "than", "yours",
        ])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    semantic_phrase(&["during", "turns", "other", "than", "yours"]).parse_next(input)?;
    semantic_finish(input)?;
    Ok(ability_tokens)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_attached_subject_and_condition_suffix() {
        let tokens = lex_line("Equipped creature has deathtouch during your turn.", 0).unwrap();
        let has = parse_attached_has_tokens(&tokens).unwrap();
        assert_eq!(has.subject, AttachedSubject::EquippedCreature);
        assert!(matches!(
            split_attached_condition_suffix_tokens(has.ability_tokens),
            AttachedConditionSuffix::YourTurn { .. }
        ));
    }
}
