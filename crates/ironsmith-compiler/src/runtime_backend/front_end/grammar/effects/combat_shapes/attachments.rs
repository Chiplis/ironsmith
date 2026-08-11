use winnow::combinator::{alt, eof, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::ChoiceCount;
use crate::runtime_backend::grammar::primitives;
use crate::runtime_backend::lexer::{
    LexStream, OwnedLexToken, parser_token_word_refs, trim_lexed_commas,
};
use crate::runtime_backend::util::{
    is_source_reference_words, parse_choice_count_token_prefix_consumed,
    trim_edge_punctuation_tokens,
};

const SOURCE_ATTACHMENT_PREFIXES: &[&[&str]] = &[
    &["this", "equipment"],
    &["this", "aura"],
    &["this", "enchantment"],
    &["this", "artifact"],
];
const TAGGED_PLAIN: &[&[&str]] = &[&["it"], &["them"]];
const TAGGED_EQUIPMENT: &[&[&str]] = &[&["that", "equipment"], &["those", "equipment"]];
const TAGGED_AURA: &[&[&str]] = &[&["that", "aura"], &["those", "auras"]];
const TAGGED_ARTIFACT: &[&[&str]] = &[&["that", "artifact"], &["those", "artifacts"]];
const TAGGED_ENCHANTMENT: &[&[&str]] = &[&["that", "enchantment"]];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatAttachTaggedObjectShape {
    Plain,
    Equipment,
    Aura,
    Artifact,
    Enchantment,
}

#[path = "attachments/attached_references.rs"]
mod attached_references;
pub(crate) use attached_references::*;

#[derive(Debug, Clone)]
pub(crate) enum CombatAttachObjectShape<'a> {
    Source,
    Tagged(CombatAttachTaggedObjectShape),
    All {
        object_tokens: &'a [OwnedLexToken],
    },
    Counted {
        count: ChoiceCount,
        object_tokens: &'a [OwnedLexToken],
        starts_with_target: bool,
    },
    Target {
        target_tokens: &'a [OwnedLexToken],
    },
    NameLikeSource,
    GeneralTarget {
        target_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatAttachClauseError {
    MissingDestination,
    MissingObjectOrDestination,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CombatAttachClauseShape<'a> {
    DestinationFirstTagged {
        tagged_tokens: &'a [OwnedLexToken],
        object_tokens: &'a [OwnedLexToken],
    },
    Standard {
        object_tokens: &'a [OwnedLexToken],
        target_tokens: &'a [OwnedLexToken],
        triggering_object_to_token: bool,
        target_is_tagged: bool,
    },
}

fn exact_phrase(tokens: &[OwnedLexToken], phrases: &'static [&'static [&'static str]]) -> bool {
    primitives::parse_all(
        tokens,
        (primitives::any_phrase(phrases), eof).void(),
        "combat-attachment-phrase",
    )
    .is_ok()
}

pub(crate) fn parse_combat_attach_tagged_object_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CombatAttachTaggedObjectShape> {
    if exact_phrase(tokens, TAGGED_PLAIN) {
        Some(CombatAttachTaggedObjectShape::Plain)
    } else if exact_phrase(tokens, TAGGED_EQUIPMENT) {
        Some(CombatAttachTaggedObjectShape::Equipment)
    } else if exact_phrase(tokens, TAGGED_AURA) {
        Some(CombatAttachTaggedObjectShape::Aura)
    } else if exact_phrase(tokens, TAGGED_ARTIFACT) {
        Some(CombatAttachTaggedObjectShape::Artifact)
    } else if exact_phrase(tokens, TAGGED_ENCHANTMENT) {
        Some(CombatAttachTaggedObjectShape::Enchantment)
    } else {
        None
    }
}

pub(crate) fn parse_combat_attach_object_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Option<CombatAttachObjectShape<'_>> {
    let words = parser_token_word_refs(tokens);
    if words.is_empty() {
        return None;
    }
    if is_source_reference_words(&words)
        || primitives::parse_prefix(tokens, primitives::any_phrase(SOURCE_ATTACHMENT_PREFIXES))
            .is_some()
    {
        return Some(CombatAttachObjectShape::Source);
    }
    if let Some(shape) = parse_combat_attach_tagged_object_shape_lexed(tokens) {
        return Some(CombatAttachObjectShape::Tagged(shape));
    }
    if let Some(((), object_tokens)) =
        primitives::parse_prefix(tokens, primitives::kw("all").void())
    {
        let object_tokens = trim_lexed_commas(object_tokens);
        if object_tokens.is_empty() {
            return None;
        }
        return Some(CombatAttachObjectShape::All { object_tokens });
    }
    if let Some((count, used)) = parse_choice_count_token_prefix_consumed(tokens) {
        let object_tokens = trim_lexed_commas(tokens.get(used..)?);
        if object_tokens.is_empty() {
            return None;
        }
        return Some(CombatAttachObjectShape::Counted {
            count,
            object_tokens,
            starts_with_target: primitives::parse_prefix(
                object_tokens,
                primitives::any_phrase(&[&["target"], &["targets"]]),
            )
            .is_some(),
        });
    }
    if primitives::parse_prefix(tokens, primitives::kw("target")).is_some() {
        return Some(CombatAttachObjectShape::Target {
            target_tokens: tokens,
        });
    }
    let name_like_source = words.len() >= 2
        && !words
            .iter()
            .any(|word| matches!(*word, "target" | "targets"))
        && words
            .iter()
            .all(|word| word.chars().all(|ch| ch.is_ascii_alphanumeric()));
    if name_like_source {
        Some(CombatAttachObjectShape::NameLikeSource)
    } else {
        Some(CombatAttachObjectShape::GeneralTarget {
            target_tokens: tokens,
        })
    }
}

fn last_to_marker(tokens: &[OwnedLexToken]) -> Option<usize> {
    tokens.iter().enumerate().rev().find_map(|(idx, token)| {
        (token.as_word() == Some("to")
            // In "attach ... to up to one target ...", the final `to` is
            // part of the target-count prefix rather than the attachment
            // separator. Splitting there silently widens 0..1 to exactly 1.
            && idx
                .checked_sub(1)
                .and_then(|previous| tokens.get(previous))
                .and_then(OwnedLexToken::as_word)
                != Some("up"))
        .then_some(idx)
    })
}

pub(crate) fn parse_combat_attach_clause_shape_lexed(
    tokens: &[OwnedLexToken],
) -> Result<CombatAttachClauseShape<'_>, CombatAttachClauseError> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    if tokens.is_empty() {
        return Err(CombatAttachClauseError::MissingObjectOrDestination);
    }

    if let Some(((), after_to)) = primitives::parse_prefix(tokens, primitives::kw("to").void()) {
        let rest = trim_lexed_commas(after_to);
        if let Some(first) = rest.first()
            && exact_phrase(std::slice::from_ref(first), &[&["it"], &["them"]])
        {
            let object_tokens = trim_lexed_commas(&rest[1..]);
            if object_tokens.is_empty() {
                return Err(CombatAttachClauseError::MissingObjectOrDestination);
            }
            return Ok(CombatAttachClauseShape::DestinationFirstTagged {
                tagged_tokens: std::slice::from_ref(first),
                object_tokens,
            });
        }
    }

    let Some(to_idx) = last_to_marker(tokens) else {
        return Err(CombatAttachClauseError::MissingDestination);
    };
    if to_idx == 0 || to_idx + 1 >= tokens.len() {
        return Err(CombatAttachClauseError::MissingObjectOrDestination);
    }
    let object_tokens = trim_lexed_commas(&tokens[..to_idx]);
    let target_tokens = trim_lexed_commas(&tokens[to_idx + 1..]);
    if object_tokens.is_empty() || target_tokens.is_empty() {
        return Err(CombatAttachClauseError::MissingObjectOrDestination);
    }
    Ok(CombatAttachClauseShape::Standard {
        object_tokens,
        target_tokens,
        triggering_object_to_token: exact_phrase(object_tokens, &[&["it"]])
            && exact_phrase(target_tokens, &[&["the", "token"]]),
        target_is_tagged: matches!(
            parse_combat_attach_tagged_object_shape_lexed(target_tokens),
            Some(CombatAttachTaggedObjectShape::Plain)
        ),
    })
}

#[cfg(test)]
#[path = "attachments/tests.rs"]
mod tests;
