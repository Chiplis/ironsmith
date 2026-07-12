use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::types::Subtype;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordView};
use super::super::primitives;
use super::{
    ChoiceClauseSeparator, ChoiceTypePhraseSyntaxError, parse_choice_basic_land_type_phrase_words,
    parse_choice_clause_separator_tokens, parse_choice_creature_type_phrase_words, word_phrase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChosenCantBlockShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) exclude_tagged_choice: bool,
    pub(crate) bare_other_reference: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChosenCantBlockSyntaxError {
    MissingSubject,
    MissingObjectFilter,
    UnsupportedObjectFilter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChoiceBecomeKind {
    CreatureType { excluded_subtypes: Vec<Subtype> },
    BasicLandType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoiceBecomeSubject<'a> {
    Target(&'a [OwnedLexToken]),
    AllObjects(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChoiceBecomeShape<'a> {
    pub(crate) kind: ChoiceBecomeKind,
    pub(crate) subject: ChoiceBecomeSubject<'a>,
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoiceBecomeSyntaxError {
    MissingCreatureSubtypeExclusion,
    UnsupportedCreatureSubtypeExclusion,
    UnsupportedCreatureTypeClause,
    UnsupportedBasicLandTypeClause,
    MissingSubject,
    MissingObjectFilter,
    UnsupportedObjectFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChoiceLibraryMoveShape<'a> {
    pub(crate) first_clause: &'a [OwnedLexToken],
    pub(crate) second_clause: &'a [OwnedLexToken],
    pub(crate) moved_tokens: &'a [OwnedLexToken],
    pub(crate) moved_is_tagged_choice: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoiceBattlefieldController {
    Preserve,
    You,
    Owner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChoiceBattlefieldMoveShape<'a> {
    pub(crate) first_clause: &'a [OwnedLexToken],
    pub(crate) second_clause: &'a [OwnedLexToken],
    pub(crate) tapped: bool,
    pub(crate) controller: ChoiceBattlefieldController,
}

pub(crate) fn parse_chosen_cant_block_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<ChosenCantBlockShape<'_>>, ChosenCantBlockSyntaxError> {
    let Some(negation) =
        super::super::activation_restrictions::parse_activation_negation_span_tokens(tokens)
    else {
        return Ok(None);
    };
    let tail_words = TokenWordView::new(&tokens[negation.end..]).word_refs();
    if primitives::parse_full_word_slice(&tail_words, parse_block_tail).is_none() {
        return Ok(None);
    }

    let mut subject_tokens = trim_punctuation_edges(&tokens[..negation.first]);
    if subject_tokens.is_empty() {
        return Err(ChosenCantBlockSyntaxError::MissingSubject);
    }
    let mut subject_input = LexStream::new(subject_tokens);
    let exclude_tagged_choice = alt((
        primitives::phrase(&["the", "other"]).void(),
        primitives::kw("other").void(),
        primitives::kw("another").void(),
    ))
    .parse_next(&mut subject_input)
    .is_ok();
    if exclude_tagged_choice {
        let consumed = subject_tokens.len().saturating_sub(subject_input.len());
        subject_tokens = trim_punctuation_edges(&subject_tokens[consumed..]);
    }
    let bare_other_reference = exclude_tagged_choice && subject_tokens.is_empty();
    if subject_tokens.is_empty() && !bare_other_reference {
        return Err(ChosenCantBlockSyntaxError::MissingObjectFilter);
    }

    Ok(Some(ChosenCantBlockShape {
        subject_tokens,
        exclude_tagged_choice,
        bare_other_reference,
    }))
}

pub(crate) fn parse_choice_become_shape<'a>(
    first: &'a [OwnedLexToken],
    second: &'a [OwnedLexToken],
) -> Result<Option<ChoiceBecomeShape<'a>>, ChoiceBecomeSyntaxError> {
    let first = trim_punctuation_edges(first);
    let first_words = TokenWordView::new(first).word_refs();
    let kind = match parse_choice_creature_type_phrase_words(&first_words) {
        Ok(Some(parsed)) => {
            if parsed.consumed != first_words.len() {
                return Err(ChoiceBecomeSyntaxError::UnsupportedCreatureTypeClause);
            }
            ChoiceBecomeKind::CreatureType {
                excluded_subtypes: parsed.excluded_subtypes,
            }
        }
        Ok(None) => {
            let Some(parsed) = parse_choice_basic_land_type_phrase_words(&first_words) else {
                return Ok(None);
            };
            if parsed.consumed != first_words.len() {
                return Err(ChoiceBecomeSyntaxError::UnsupportedBasicLandTypeClause);
            }
            ChoiceBecomeKind::BasicLandType
        }
        Err(ChoiceTypePhraseSyntaxError::MissingCreatureSubtypeExclusion) => {
            return Err(ChoiceBecomeSyntaxError::MissingCreatureSubtypeExclusion);
        }
        Err(ChoiceTypePhraseSyntaxError::UnsupportedCreatureSubtypeExclusion) => {
            return Err(ChoiceBecomeSyntaxError::UnsupportedCreatureSubtypeExclusion);
        }
        Err(
            ChoiceTypePhraseSyntaxError::MissingColorExclusion
            | ChoiceTypePhraseSyntaxError::UnsupportedColorExclusion,
        ) => return Ok(None),
    };

    let Some(separator) =
        parse_choice_clause_separator_tokens(second, ChoiceClauseSeparator::Become)
    else {
        return Ok(None);
    };
    if separator.first == 0 {
        return Ok(None);
    }
    let subject_tokens = trim_punctuation_edges(&second[..separator.first]);
    if subject_tokens.is_empty() {
        return Err(ChoiceBecomeSyntaxError::MissingSubject);
    }
    let tail_tokens = trim_punctuation_edges(&second[separator.end..]);

    let mut subject_input = LexStream::new(subject_tokens);
    let quantified = alt((primitives::kw("each"), primitives::kw("all")))
        .parse_next(&mut subject_input)
        .is_ok();
    let subject = if quantified {
        let consumed = subject_tokens.len().saturating_sub(subject_input.len());
        let filter_tokens = trim_punctuation_edges(&subject_tokens[consumed..]);
        if filter_tokens.is_empty() {
            return Err(ChoiceBecomeSyntaxError::MissingObjectFilter);
        }
        ChoiceBecomeSubject::AllObjects(filter_tokens)
    } else {
        ChoiceBecomeSubject::Target(subject_tokens)
    };

    Ok(Some(ChoiceBecomeShape {
        kind,
        subject,
        tail_tokens,
    }))
}

pub(crate) fn parse_that_type_tokens(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    primitives::parse_full_word_slice(&words, word_phrase(&["that", "type"])).is_some()
}

pub(crate) fn parse_choice_library_move_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChoiceLibraryMoveShape<'_>> {
    let separator = parse_choice_clause_separator_tokens(tokens, ChoiceClauseSeparator::And)?;
    let first_clause = trim_punctuation_edges(&tokens[..separator.first]);
    let second_clause = trim_punctuation_edges(&tokens[separator.end..]);
    if second_clause.is_empty() {
        return None;
    }

    let mut input = LexStream::new(second_clause);
    alt((primitives::kw("put"), primitives::kw("puts")))
        .parse_next(&mut input)
        .ok()?;
    let moved: &[OwnedLexToken] = repeat_till(0.., any.void(), peek(primitives::kw("on")).void())
        .map(|((), ())| ())
        .take()
        .parse_next(&mut input)
        .ok()?;
    primitives::kw("on").parse_next(&mut input).ok()?;
    primitives::phrase(&["top", "of"])
        .parse_next(&mut input)
        .ok()?;
    repeat_till(0.., any.void(), peek(primitives::kw("library")).void())
        .map(|((), ())| ())
        .parse_next(&mut input)
        .ok()?;
    primitives::kw("library").parse_next(&mut input).ok()?;

    let moved_tokens = trim_punctuation_edges(moved);
    Some(ChoiceLibraryMoveShape {
        first_clause,
        second_clause,
        moved_tokens,
        moved_is_tagged_choice: moved_tokens.is_empty() || parse_tagged_library_move(moved_tokens),
    })
}

pub(crate) fn parse_choice_battlefield_move_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChoiceBattlefieldMoveShape<'_>> {
    let separator = parse_choice_clause_separator_tokens(tokens, ChoiceClauseSeparator::Then)?;
    if separator.first == 0 || separator.end >= tokens.len() {
        return None;
    }
    let first_clause = trim_punctuation_edges(&tokens[..separator.first]);
    let second_clause = trim_punctuation_edges(&tokens[separator.end..]);
    let words = TokenWordView::new(second_clause).word_refs();
    let (tapped, controller) =
        primitives::parse_full_word_slice(&words, parse_battlefield_move_words)?;
    Some(ChoiceBattlefieldMoveShape {
        first_clause,
        second_clause,
        tapped,
        controller,
    })
}

fn parse_block_tail(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    primitives::word_slice_exact("block").parse_next(input)?;
    opt(word_phrase(&["this", "turn"])).void().parse_next(input)
}

fn parse_tagged_library_move(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    primitives::parse_full_word_slice(
        &words,
        alt((
            primitives::word_slice_exact("it").void(),
            primitives::word_slice_exact("them").void(),
            primitives::word_slice_exact("those").void(),
            word_phrase(&["those", "cards"]),
        )),
    )
    .is_some()
}

fn parse_battlefield_move_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<(bool, ChoiceBattlefieldController)> {
    primitives::word_slice_exact("you").parse_next(input)?;
    alt((
        primitives::word_slice_exact("put"),
        primitives::word_slice_exact("puts"),
    ))
    .parse_next(input)?;
    alt((
        primitives::word_slice_exact("it").void(),
        word_phrase(&["that", "card"]),
        word_phrase(&["that", "permanent"]),
    ))
    .parse_next(input)?;
    primitives::word_slice_exact("onto").parse_next(input)?;
    opt(primitives::word_slice_exact("the")).parse_next(input)?;
    primitives::word_slice_exact("battlefield").parse_next(input)?;

    let tapped_before = opt(primitives::word_slice_exact("tapped"))
        .parse_next(input)?
        .is_some();
    let controller = opt(parse_battlefield_controller_words)
        .parse_next(input)?
        .unwrap_or(ChoiceBattlefieldController::Preserve);
    let tapped_after = opt(primitives::word_slice_exact("tapped"))
        .parse_next(input)?
        .is_some();
    eof.parse_next(input)?;
    Ok((tapped_before || tapped_after, controller))
}

fn parse_battlefield_controller_words(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<ChoiceBattlefieldController> {
    alt((
        word_phrase(&["under", "your", "control"]).value(ChoiceBattlefieldController::You),
        alt((
            word_phrase(&["under", "its", "owners", "control"]),
            word_phrase(&["under", "its", "owner's", "control"]),
            word_phrase(&["under", "their", "owners", "control"]),
            word_phrase(&["under", "their", "owner's", "control"]),
            word_phrase(&["under", "that", "players", "control"]),
            word_phrase(&["under", "that", "player's", "control"]),
        ))
        .value(ChoiceBattlefieldController::Owner),
    ))
    .parse_next(input)
}

fn trim_punctuation_edges(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Comma | TokenKind::Period))
    {
        tokens = &tokens[1..];
    }
    while tokens
        .last()
        .is_some_and(|token| matches!(token.kind, TokenKind::Comma | TokenKind::Period))
    {
        tokens = &tokens[..tokens.len().saturating_sub(1)];
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn cant_block_shape_returns_subject_and_exclusion_fact() {
        let tokens = lex("Other creatures can't block this turn.");
        let parsed = parse_chosen_cant_block_shape(&tokens).unwrap().unwrap();
        assert!(parsed.exclude_tagged_choice);
        assert_eq!(
            TokenWordView::new(parsed.subject_tokens).word_refs(),
            ["creatures"]
        );

        let bare_other_tokens = lex("The other can't block this turn.");
        let bare_other = parse_chosen_cant_block_shape(&bare_other_tokens)
            .unwrap()
            .unwrap();
        assert!(bare_other.exclude_tagged_choice);
        assert!(bare_other.bare_other_reference);
        assert!(bare_other.subject_tokens.is_empty());
    }

    #[test]
    fn become_shape_returns_typed_kind_and_quantified_filter() {
        let first = lex("Choose a creature type other than Dragon.");
        let second = lex("All creatures become that type.");
        let parsed = parse_choice_become_shape(&first, &second).unwrap().unwrap();
        assert_eq!(
            parsed.kind,
            ChoiceBecomeKind::CreatureType {
                excluded_subtypes: vec![Subtype::Dragon]
            }
        );
        let ChoiceBecomeSubject::AllObjects(filter) = parsed.subject else {
            panic!("expected quantified object subject");
        };
        assert_eq!(TokenWordView::new(filter).word_refs(), ["creatures"]);
        assert!(parse_that_type_tokens(parsed.tail_tokens));
    }

    #[test]
    fn library_move_shape_preserves_explicit_target_or_tagged_choice() {
        let tagged_tokens =
            lex("Target player chooses a creature and puts it on top of their library.");
        let tagged = parse_choice_library_move_shape(&tagged_tokens).unwrap();
        assert!(tagged.moved_is_tagged_choice);

        let explicit_tokens = lex(
            "Target player chooses a creature and puts target artifact on top of their library.",
        );
        let explicit = parse_choice_library_move_shape(&explicit_tokens).unwrap();
        assert!(!explicit.moved_is_tagged_choice);
    }

    #[test]
    fn battlefield_move_shape_returns_controller_and_tapped_facts() {
        let tokens = lex(
            "Target opponent chooses a card, then you put that card onto the battlefield tapped under its owner's control.",
        );
        let parsed = parse_choice_battlefield_move_shape(&tokens).unwrap();
        assert!(parsed.tapped);
        assert_eq!(parsed.controller, ChoiceBattlefieldController::Owner);
    }
}
