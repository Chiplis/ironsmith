use winnow::combinator::{alt, eof};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{literal, take_till};

use super::super::{leaf, permission_shapes, primitives};
use crate::cards::builders::{PlayerAst, SubjectAst, TagKey, TargetAst};
use crate::effect::Value;
use crate::lexer::{LexStream, OwnedLexToken, TokenWordView};
use crate::target::{ChooseSpec, ObjectFilter};

#[path = "become_shapes/descriptors.rs"]
mod descriptors;
#[path = "become_shapes/subjects.rs"]
mod subjects;
#[path = "become_shapes/surface.rs"]
mod surface;

pub use descriptors::*;
pub use subjects::*;
pub use surface::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ControllerOwnerSubjectShape {
    pub subject: SubjectAst,
    pub target: TargetAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasePowerToughnessSubjectShape<'a> {
    pub target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub struct BecomePowerToughnessTail<'a> {
    pub descriptor_words: &'a [&'a str],
    pub power: Value,
    pub toughness: Value,
}

#[derive(Debug, Clone)]
pub struct FilteredObjectAnimationShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub dependent_subject: bool,
    pub removes_all_abilities: bool,
    pub preserve_other_types: bool,
    pub descriptor: BecomeCreatureDescriptor,
    pub power: Value,
    pub toughness: Value,
}

fn possessive_word_stem(input: &mut &str) -> WResult<String> {
    let stem: &str =
        take_till(1.., |character: char| matches!(character, '\'' | '’')).parse_next(input)?;
    let plural = alt((
        literal("'s").value(false),
        literal("’s").value(false),
        literal("'").value(true),
        literal("’").value(true),
    ))
    .parse_next(input)?;
    eof.parse_next(input)?;
    if !plural {
        return Ok(stem.to_string());
    }
    let Some(singular) = stem.strip_suffix('s') else {
        return Err(primitives::backtrack_err(
            "possessive subject",
            "plural s before apostrophe",
        ));
    };
    Ok(singular.to_string())
}

pub fn parse_possessive_subject_stem(word: &str) -> Option<String> {
    crate::grammar::primitives::probe_shape(possessive_word_stem.parse(word))
}

fn enchanted_target() -> TargetAst {
    let mut filter = ObjectFilter::creature();
    filter
        .tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::Enchanted.bind()).into(),
            relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
        });
    TargetAst::Object(filter, None, None)
}

fn controller_owner_suffix(input: &mut LexStream<'_>) -> WResult<PlayerAst> {
    alt((
        alt((primitives::kw("controller"), primitives::kw("controllers")))
            .value(PlayerAst::ItsController),
        alt((primitives::kw("owner"), primitives::kw("owners"))).value(PlayerAst::ItsOwner),
    ))
    .parse_next(input)
}

fn controller_owner_prefix(input: &mut LexStream<'_>) -> WResult<PlayerAst> {
    alt((
        primitives::phrase(&["the", "controller", "of"]).value(PlayerAst::ItsController),
        primitives::phrase(&["controller", "of"]).value(PlayerAst::ItsController),
        primitives::phrase(&["the", "owner", "of"]).value(PlayerAst::ItsOwner),
        primitives::phrase(&["owner", "of"]).value(PlayerAst::ItsOwner),
    ))
    .parse_next(input)
}

fn normalize_trailing_possessive(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    let Some(last) = normalized.last_mut() else {
        return normalized;
    };
    let Some(stem) = last.as_word().and_then(parse_possessive_subject_stem) else {
        return normalized;
    };
    last.replace_word(stem);
    normalized
}

fn parsed_controller_owner_shape(
    context: Option<crate::parse_context::ParseContextView<'_>>,
    player: PlayerAst,
    target_tokens: &[OwnedLexToken],
) -> Option<ControllerOwnerSubjectShape> {
    let target_tokens = normalize_trailing_possessive(target_tokens);
    let target_words = TokenWordView::new(&target_tokens).to_word_refs();
    let persistent_source_surface = context
        .and_then(|context| {
            crate::util::source_reference_surface_for_words_with_context(context, &target_words)
        })
        .or_else(|| crate::util::source_reference_surface_for_words(&target_words))
        .or_else(|| crate::util::this_source_surface_for_words(&target_words));
    let target = if let Some(context) = context {
        crate::grammar::primitives::probe_shape(crate::util::parse_target_phrase_with_context(
            context,
            &target_tokens,
        ))?
    } else {
        crate::grammar::primitives::probe_shape(crate::util::parse_target_phrase(&target_tokens))?
    };
    let target = match (persistent_source_surface, target) {
        (Some(surface), TargetAst::Source(_)) => {
            // A quoted attached-object ability is parsed in a temporary
            // name-only source context.  Span-indexed surface metadata
            // disappears when that nested parse returns, so carry the
            // authored source identity in the AST itself.
            TargetAst::Object(
                ObjectFilter::source().with_source_surface(surface),
                None,
                None,
            )
        }
        (Some(surface), TargetAst::Object(mut filter, target_span, reference_span))
            if filter.source =>
        {
            filter.source_surface = Some(surface);
            TargetAst::Object(filter, target_span, reference_span)
        }
        (_, target) => target,
    };
    Some(ControllerOwnerSubjectShape {
        subject: SubjectAst::Player(player),
        target,
    })
}

pub fn parse_controller_owner_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ControllerOwnerSubjectShape> {
    parse_controller_owner_subject_tokens_with_optional_context(None, tokens)
}

pub fn parse_controller_owner_subject_tokens_with_context(
    context: crate::parse_context::ParseContextView<'_>,
    tokens: &[OwnedLexToken],
) -> Option<ControllerOwnerSubjectShape> {
    parse_controller_owner_subject_tokens_with_optional_context(Some(context), tokens)
}

fn parse_controller_owner_subject_tokens_with_optional_context(
    context: Option<crate::parse_context::ParseContextView<'_>>,
    tokens: &[OwnedLexToken],
) -> Option<ControllerOwnerSubjectShape> {
    const TRIGGERING_STACK_CONTROLLER: &[&[&str]] = &[
        &["that", "spell", "or", "ability's", "controller"],
        &["that", "spell", "or", "ability", "s", "controller"],
        &["that", "spell", "or", "abilitys", "controller"],
    ];
    const ENCHANTED_CONTROLLER: &[&[&str]] = &[
        &["enchanted", "creature", "s", "controller"],
        &["enchanted", "creatures", "controller"],
        &["enchanted", "creature's", "controller"],
    ];
    const ENCHANTED_OWNER: &[&[&str]] = &[
        &["enchanted", "creature", "s", "owner"],
        &["enchanted", "creatures", "owner"],
        &["enchanted", "creature's", "owner"],
    ];

    let words = TokenWordView::new(tokens).to_word_refs();
    if TRIGGERING_STACK_CONTROLLER
        .iter()
        .any(|expected| permission_shapes::exact_words(&words, expected))
    {
        return Some(ControllerOwnerSubjectShape {
            subject: SubjectAst::TriggeringSourceController,
            target: TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::TriggeringSource.bind(),
                None,
            ),
        });
    }

    if tokens.len() == 2 && tokens[0].is_word("its") {
        let player = if tokens[1].is_word("controller") {
            PlayerAst::ItsController
        } else if tokens[1].is_word("owner") {
            PlayerAst::ItsOwner
        } else {
            return None;
        };
        return Some(ControllerOwnerSubjectShape {
            subject: SubjectAst::Player(player),
            target: TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
        });
    }

    if ENCHANTED_CONTROLLER
        .iter()
        .any(|expected| permission_shapes::exact_tokens(tokens, expected))
    {
        return Some(ControllerOwnerSubjectShape {
            subject: SubjectAst::Player(PlayerAst::ItsController),
            target: enchanted_target(),
        });
    }
    if ENCHANTED_OWNER
        .iter()
        .any(|expected| permission_shapes::exact_tokens(tokens, expected))
    {
        return Some(ControllerOwnerSubjectShape {
            subject: SubjectAst::Player(PlayerAst::ItsOwner),
            target: enchanted_target(),
        });
    }

    if let Some((target_tokens, player)) =
        primitives::split_lexed_once_before_suffix(tokens, 1, || controller_owner_suffix)
        && let Some(parsed) = parsed_controller_owner_shape(context, player, target_tokens)
    {
        return Some(parsed);
    }

    let (player, target_tokens) = primitives::parse_prefix(tokens, controller_owner_prefix)?;
    (!target_tokens.is_empty())
        .then(|| parsed_controller_owner_shape(context, player, target_tokens))
        .flatten()
}

#[cfg(test)]
#[path = "become_shapes_inline_tests.rs"]
mod tests;

#[path = "become_shapes/resource.rs"]
mod resource_programs;
pub use resource_programs::parse_become_iterated_mana_value_pt_words;
#[path = "become_shapes/reference.rs"]
mod reference_programs;
pub use reference_programs::{
    parse_base_power_toughness_subject_tokens, parse_filtered_object_animation_tokens,
};
#[path = "become_shapes/counter.rs"]
mod counter_programs;
use counter_programs::parse_become_iterated_counter_value_words;
pub use counter_programs::parse_counter_state_pronoun_tokens;
#[path = "become_shapes/object_action.rs"]
mod object_action_programs;
pub use object_action_programs::parse_become_base_pt_words;
#[path = "become_shapes/condition.rs"]
mod condition_programs;
use condition_programs::parse_modifier_words;
