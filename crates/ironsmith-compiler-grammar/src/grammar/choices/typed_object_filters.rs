use crate::cards::builders::TagKey;
use crate::effect::ChoiceCount;
#[cfg(test)]
use crate::effect::Value;
#[cfg(test)]
use crate::filter::Comparison;
use crate::filter::TaggedObjectConstraint;
use crate::lexer::OwnedLexToken;
use crate::target::{ObjectFilter, TaggedOpbjectRelation};
use crate::zone::Zone;

use super::{
    ChoiceBecomeKind, ChoiceBecomeSubject, ChoiceBecomeSyntaxError, ChoiceClauseActor,
    ChoiceObjectClauseKind, ChoiceObjectClauseSyntaxError, ChoiceObjectCountSource,
    ChoiceObjectFilterFacts, ChoiceObjectReferenceFacts, ChosenCantBlockSyntaxError,
    TargetPlayerChoiceActor, parse_choice_become_shape, parse_choice_object_clause_tokens,
    parse_chosen_cant_block_shape, parse_target_player_choice_tokens,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TypedChoiceObjectClause {
    pub actor: ChoiceClauseActor,
    pub filter: ObjectFilter,
    pub count: ChoiceCount,
    pub count_source: Option<ChoiceObjectCountSource>,
    pub references: ChoiceObjectReferenceFacts,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedChoiceObjectClauseKind {
    Object(TypedChoiceObjectClause),
    CardName,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedTargetPlayerChoice {
    pub actor: TargetPlayerChoiceActor,
    pub count: ChoiceCount,
    pub count_source: Option<ChoiceObjectCountSource>,
    pub filter: ObjectFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedChosenCantBlock {
    pub filter: ObjectFilter,
    pub exclude_tagged_choice: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedChoiceBecomeSubject<'a> {
    Target(&'a [OwnedLexToken]),
    AllObjects(ObjectFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedChoiceBecomeShape<'a> {
    pub kind: ChoiceBecomeKind,
    pub subject: TypedChoiceBecomeSubject<'a>,
    pub tail_tokens: &'a [OwnedLexToken],
}

fn parse_typed_choice_filter_words(
    words: &[&str],
) -> Result<ObjectFilter, ChoiceObjectClauseSyntaxError> {
    if let Some(base) = words.strip_suffix(&["of", "that", "color"])
        .or_else(|| words.strip_suffix(&["of", "the", "chosen", "color"]))
    {
        let mut filter = if matches!(base, ["card"] | ["cards"]) {
            ObjectFilter::default()
        } else {
            parse_typed_choice_filter_words(base)?
        };
        filter.chosen_color = true;
        return Ok(filter);
    }
    if let Some(filter) = parse_choice_source_filter_words(words)? {
        return Ok(filter);
    }
    crate::object_filters::parse_object_filter_words(words, false)
        .map_err(|_| ChoiceObjectClauseSyntaxError::UnsupportedFilter)
}

/// "choose a source you control" / "choose a red source": a source is any
/// object that can deal damage (CR 120.1, 609.7a) — a permanent on the
/// battlefield or a spell on the stack — so the choice spans both zones as a
/// zone union. `None` when the chosen noun is not "source".
fn parse_choice_source_filter_words(
    words: &[&str],
) -> Result<Option<ObjectFilter>, ChoiceObjectClauseSyntaxError> {
    let Some(noun) = words
        .iter()
        .position(|word| matches!(*word, "source" | "sources"))
    else {
        return Ok(None);
    };
    let (descriptor, tail) = (&words[..noun], &words[noun + 1..]);
    let controller = match tail {
        [] => None,
        ["you", "control"] => Some(crate::target::PlayerFilter::You),
        ["you", "don't" | "dont", "control"] => Some(crate::target::PlayerFilter::NotYou),
        ["an", "opponent", "controls"] | ["your", "opponents", "control"] => {
            Some(crate::target::PlayerFilter::Opponent)
        }
        _ => return Ok(None),
    };
    let mut filter = if descriptor.is_empty() {
        ObjectFilter::default()
    } else {
        let Ok(mut filter) = crate::object_filters::parse_object_filter_words(descriptor, false)
        else {
            return Ok(None);
        };
        if !filter.any_of.is_empty() {
            return Ok(None);
        }
        filter.zone = None;
        filter
    };
    if controller.is_some() {
        filter.controller = controller;
    }
    filter.any_of = vec![
        ObjectFilter {
            zone: Some(Zone::Battlefield),
            ..ObjectFilter::default()
        },
        ObjectFilter {
            zone: Some(Zone::Stack),
            ..ObjectFilter::default()
        },
    ];
    Ok(Some(filter))
}

pub fn parse_typed_target_player_choice_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<TypedTargetPlayerChoice>, ChoiceObjectClauseSyntaxError> {
    let Some(shape) = parse_target_player_choice_tokens(tokens)? else {
        return Ok(None);
    };
    if shape.filter_is_player_target
        || super::super::effects::chain_splitting::find_chain_verb_tokens(shape.filter_tokens)
            .is_some()
    {
        return Ok(None);
    }

    let Some(TypedChoiceObjectClauseKind::Object(object_choice)) =
        parse_typed_choice_object_clause_tokens(shape.object_choice_tokens)?
    else {
        return Ok(None);
    };
    let filter =
        expand_graveyard_or_hand_disjunction_filter(object_choice.filter, shape.filter_facts);
    Ok(Some(TypedTargetPlayerChoice {
        actor: shape.actor,
        count: object_choice.count,
        count_source: object_choice.count_source,
        filter,
    }))
}

/// One token per filter word, each carrying the span of the first unused
/// authored word piece of `clause_tokens` with the same text; a word with no
/// authored source (a reader-inserted noun) stays synthetic.
fn authored_choice_filter_tokens(
    clause_tokens: &[OwnedLexToken],
    filter_words: &[String],
) -> Vec<OwnedLexToken> {
    let pieces = clause_tokens
        .iter()
        .flat_map(|token| token.parser_word_pieces().iter())
        .filter(|piece| piece.span.end > piece.span.start)
        .collect::<Vec<_>>();
    let mut used = vec![false; pieces.len()];
    filter_words
        .iter()
        .map(|word| {
            let found = pieces
                .iter()
                .enumerate()
                .find(|(index, piece)| !used[*index] && piece.text == *word)
                .map(|(index, piece)| (index, piece.span));
            match found {
                Some((index, span)) => {
                    used[index] = true;
                    OwnedLexToken::word(word.clone(), span)
                }
                None => OwnedLexToken::synthetic_word(word.clone()),
            }
        })
        .collect()
}

pub fn parse_typed_choice_object_clause_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<TypedChoiceObjectClauseKind>, ChoiceObjectClauseSyntaxError> {
    let Some(kind) = parse_choice_object_clause_tokens(tokens)? else {
        return Ok(None);
    };
    let ChoiceObjectClauseKind::Object(shape) = kind else {
        return Ok(Some(TypedChoiceObjectClauseKind::CardName));
    };

    // The shape trims and reorders owned copies of the body words. Rebuild
    // them as tokens carrying the authored span of the body word each one
    // came from, so the filter readers below claim (and the filter audit
    // obliges) the authored words they read, and nothing else.
    let authored_filter_tokens = authored_choice_filter_tokens(tokens, &shape.filter_words);
    let filter_words = crate::lexer::TokenWordView::new(&authored_filter_tokens).to_word_refs();
    let controller_tail =
        super::super::filters::parse_simple_object_filter_words(&filter_words, false)
            .is_some_and(|filter| filter.controller.is_some());
    if super::super::effects::chain_splitting::find_chain_verb_words(&filter_words).is_some()
        && !controller_tail
    {
        return Ok(None);
    }

    let references_it = shape.references.references_it;
    if shape.references.explicit_container_reference {
        // The container suffix the shape stripped from its word copies
        // ("from it", "in them", "from among them", "from there in"): the
        // choice's pool, read here on the authored tokens.
        let words: Vec<&OwnedLexToken> =
            tokens.iter().filter(|token| token.as_word().is_some()).collect();
        let texts: Vec<&str> = words.iter().map(|token| token.parser_text()).collect();
        let tail = match texts.as_slice() {
            [.., "from", "there", "in"] | [.., "from", "among", "them"] => 3,
            [.., "from" | "in", "it" | "them"] => 2,
            _ => 0,
        };
        for token in &words[words.len() - tail..] {
            crate::word_claims::claim_tokens(std::slice::from_ref(*token));
        }
    }
    let mut filter = if references_it && shape.filter_facts.bare_card {
        // "a card from it": any card of the referenced group.
        crate::word_claims::claim_tokens(&authored_filter_tokens);
        ObjectFilter::default()
    } else {
        parse_typed_choice_filter_words(&filter_words)?
    };
    filter = expand_graveyard_or_hand_disjunction_filter(filter, shape.filter_facts);
    if shape.references.source_aura_can_enchant {
        filter.could_be_enchanted_by_source = true;
    }
    if references_it {
        // A "card" noun names a card wherever the group went; a default
        // battlefield zone from the type word must not pin it there.
        let names_card_noun = filter_words
            .iter()
            .any(|word| matches!(*word, "card" | "cards"));
        if shape.references.references_object_group
            && names_card_noun
            && filter.zone == Some(Zone::Battlefield)
        {
            filter.zone = None;
        }
        if shape.references.references_object_group && filter.zone.is_none() {
            // "choose a creature card from among them" (Kaya, Spirits'
            // Justice; "Mill three cards. ... from among them"): the chosen
            // object is a member of the referenced group wherever that group
            // went, not a card in the chooser's hand. A zone-less tagged pool
            // lowers to a choice across the collection zones.
        } else if (shape.references.explicit_container_reference
            && matches!(filter.zone, None | Some(Zone::Battlefield)))
            || (shape.references.references_container_it && filter.zone.is_none())
        {
            filter.zone = Some(Zone::Hand);
        }
        if !filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        }) {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        }
        filter = expand_tagged_hand_or_graveyard_disjunction_filter(filter, shape.filter_facts);
    }
    if shape.references.excludes_chosen_this_way
        && !filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        })
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::ChosenObjects.bind()).into(),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
    }
    if matches!(
        filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        filter.controller = None;
    }
    if references_it {
        filter.controller = None;
        filter.owner = None;
    }

    Ok(Some(TypedChoiceObjectClauseKind::Object(
        TypedChoiceObjectClause {
            actor: shape.actor,
            filter,
            count: shape.count,
            count_source: shape.count_source,
            references: shape.references,
        },
    )))
}

pub fn parse_typed_chosen_cant_block_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<TypedChosenCantBlock>, ChosenCantBlockSyntaxError> {
    let Some(shape) = parse_chosen_cant_block_shape(tokens)? else {
        return Ok(None);
    };
    let filter = if shape.bare_other_reference {
        ObjectFilter::default()
    } else {
        crate::object_filters::parse_object_filter(shape.subject_tokens, false)
            .map_err(|_| ChosenCantBlockSyntaxError::UnsupportedObjectFilter)?
    };
    Ok(Some(TypedChosenCantBlock {
        filter,
        exclude_tagged_choice: shape.exclude_tagged_choice,
    }))
}

pub fn parse_typed_choice_become_shape<'a>(
    first: &'a [OwnedLexToken],
    second: &'a [OwnedLexToken],
) -> Result<Option<TypedChoiceBecomeShape<'a>>, ChoiceBecomeSyntaxError> {
    let Some(shape) = parse_choice_become_shape(first, second)? else {
        return Ok(None);
    };
    let subject = match shape.subject {
        ChoiceBecomeSubject::Target(tokens) => TypedChoiceBecomeSubject::Target(tokens),
        ChoiceBecomeSubject::AllObjects(tokens) => {
            let filter = crate::object_filters::parse_object_filter(tokens, false)
                .map_err(|_| ChoiceBecomeSyntaxError::UnsupportedObjectFilter)?;
            TypedChoiceBecomeSubject::AllObjects(filter)
        }
    };
    Ok(Some(TypedChoiceBecomeShape {
        kind: shape.kind,
        subject,
        tail_tokens: shape.tail_tokens,
    }))
}

fn expand_graveyard_or_hand_disjunction_filter(
    mut filter: ObjectFilter,
    facts: ChoiceObjectFilterFacts,
) -> ObjectFilter {
    if !facts.graveyard_and_hand {
        return filter;
    }

    filter.zone = None;
    filter.controller = None;
    filter.any_of = vec![
        ObjectFilter {
            zone: Some(Zone::Graveyard),
            ..ObjectFilter::default()
        },
        ObjectFilter {
            zone: Some(Zone::Hand),
            ..ObjectFilter::default()
        },
    ];
    filter
}

fn expand_tagged_hand_or_graveyard_disjunction_filter(
    mut filter: ObjectFilter,
    facts: ChoiceObjectFilterFacts,
) -> ObjectFilter {
    if !facts.tagged_graveyard_disjunction {
        return filter;
    }
    let graveyard_arm_is_plain_card = facts.graveyard_arm_is_plain_card;

    let mut hand_arm = filter.clone();
    hand_arm.zone = Some(Zone::Hand);
    hand_arm.controller = None;
    hand_arm.owner = None;
    hand_arm.any_of.clear();
    if !hand_arm
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
    {
        hand_arm.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    let mut graveyard_arm = filter.clone();
    graveyard_arm.zone = Some(Zone::Graveyard);
    graveyard_arm.any_of.clear();
    graveyard_arm.tagged_constraints.retain(|constraint| {
        constraint.tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str()
    });
    if graveyard_arm_is_plain_card {
        graveyard_arm.excluded_card_types.clear();
    }

    filter.zone = None;
    filter.controller = None;
    filter.owner = None;
    filter.tagged_constraints.clear();
    if graveyard_arm_is_plain_card {
        filter.excluded_card_types.clear();
    }
    filter.any_of = vec![hand_arm, graveyard_arm];
    filter
}

#[cfg(test)]
#[path = "typed_object_filters_inline_tests.rs"]
mod tests;
