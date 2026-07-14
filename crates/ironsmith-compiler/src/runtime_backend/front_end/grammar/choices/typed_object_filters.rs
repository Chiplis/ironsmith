use crate::cards::builders::{IT_TAG, TagKey};
use crate::effect::{ChoiceCount, Value};
use crate::filter::{Comparison, TaggedObjectConstraint};
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, TokenWordView};
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
pub(crate) struct TypedChoiceObjectClause {
    pub(crate) actor: ChoiceClauseActor,
    pub(crate) filter: ObjectFilter,
    pub(crate) count: ChoiceCount,
    pub(crate) count_source: Option<ChoiceObjectCountSource>,
    pub(crate) references: ChoiceObjectReferenceFacts,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TypedChoiceObjectClauseKind {
    Object(TypedChoiceObjectClause),
    CardName,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypedTargetPlayerChoice {
    pub(crate) actor: TargetPlayerChoiceActor,
    pub(crate) count: ChoiceCount,
    pub(crate) filter: ObjectFilter,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypedChosenCantBlock {
    pub(crate) filter: ObjectFilter,
    pub(crate) exclude_tagged_choice: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TypedChoiceBecomeSubject<'a> {
    Target(&'a [OwnedLexToken]),
    AllObjects(ObjectFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypedChoiceBecomeShape<'a> {
    pub(crate) kind: ChoiceBecomeKind,
    pub(crate) subject: TypedChoiceBecomeSubject<'a>,
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChoiceAggregateKind {
    GreatestManaValue,
    GreatestPower,
}

fn split_choice_aggregate_words<'a, 'b>(
    words: &'a [&'b str],
) -> Option<(ChoiceAggregateKind, &'a [&'b str], &'a [&'b str])> {
    for (marker, kind) in [
        (
            &["with", "the", "greatest", "mana", "value", "among"][..],
            ChoiceAggregateKind::GreatestManaValue,
        ),
        (
            &["with", "the", "greatest", "power", "among"][..],
            ChoiceAggregateKind::GreatestPower,
        ),
    ] {
        let Some(index) = words
            .windows(marker.len())
            .position(|window| window == marker)
        else {
            continue;
        };
        let object_words = words.get(..index)?;
        let scope_words = words.get(index + marker.len()..)?;
        if !object_words.is_empty() && !scope_words.is_empty() {
            return Some((kind, object_words, scope_words));
        }
    }
    None
}

fn parse_typed_choice_filter_words(
    words: &[&str],
) -> Result<ObjectFilter, ChoiceObjectClauseSyntaxError> {
    let Some((kind, object_words, scope_words)) = split_choice_aggregate_words(words) else {
        return crate::runtime_backend::object_filters::parse_object_filter_words(words, false)
            .map_err(|_| ChoiceObjectClauseSyntaxError::UnsupportedFilter);
    };

    let mut filter =
        crate::runtime_backend::object_filters::parse_object_filter_words(object_words, false)
            .map_err(|_| ChoiceObjectClauseSyntaxError::UnsupportedFilter)?;
    let scope =
        crate::runtime_backend::object_filters::parse_object_filter_words(scope_words, false)
            .map_err(|_| ChoiceObjectClauseSyntaxError::UnsupportedFilter)?;

    // The selected object is a member of the aggregate's comparison set.
    // Preserve its explicitly parsed noun while inheriting the set's player
    // and zone boundaries (for example, "among creatures they control").
    if filter.card_types.is_empty() {
        filter.card_types = scope.card_types.clone();
    }
    if filter.controller.is_none() {
        filter.controller = scope.controller.clone();
    }
    if filter.owner.is_none() {
        filter.owner = scope.owner.clone();
    }
    if filter.zone.is_none() {
        filter.zone = scope.zone.clone();
    }
    match kind {
        ChoiceAggregateKind::GreatestManaValue => {
            filter.mana_value = Some(Comparison::EqualExpr(Box::new(Value::GreatestManaValue(
                scope,
            ))));
        }
        ChoiceAggregateKind::GreatestPower => {
            filter.power = Some(Comparison::EqualExpr(Box::new(Value::GreatestPower(scope))));
        }
    }
    Ok(filter)
}

pub(crate) fn parse_typed_target_player_choice_tokens(
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

    let filter_words = TokenWordView::new(shape.filter_tokens).word_refs();
    let filter = parse_typed_choice_filter_words(&filter_words)?;
    let filter = expand_graveyard_or_hand_disjunction_filter(filter, shape.filter_facts);
    Ok(Some(TypedTargetPlayerChoice {
        actor: shape.actor,
        count: shape.count,
        filter,
    }))
}

pub(crate) fn parse_typed_choice_object_clause_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<TypedChoiceObjectClauseKind>, ChoiceObjectClauseSyntaxError> {
    let Some(kind) = parse_choice_object_clause_tokens(tokens)? else {
        return Ok(None);
    };
    let ChoiceObjectClauseKind::Object(shape) = kind else {
        return Ok(Some(TypedChoiceObjectClauseKind::CardName));
    };

    let filter_words = shape
        .filter_words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let controller_tail =
        super::super::filters::parse_simple_object_filter_words(&filter_words, false)
            .is_some_and(|filter| filter.controller.is_some());
    if super::super::effects::chain_splitting::find_chain_verb_words(&filter_words).is_some()
        && !controller_tail
    {
        return Ok(None);
    }

    let references_it = shape.references.references_it;
    let mut filter = if references_it && shape.filter_facts.bare_card {
        ObjectFilter::default()
    } else {
        parse_typed_choice_filter_words(&filter_words)?
    };
    filter = expand_graveyard_or_hand_disjunction_filter(filter, shape.filter_facts);
    if references_it {
        if shape.references.explicit_container_reference
            && matches!(filter.zone, None | Some(Zone::Battlefield))
        {
            filter.zone = Some(Zone::Hand);
        } else if shape.references.references_container_it && filter.zone.is_none() {
            filter.zone = Some(Zone::Hand);
        }
        if !filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
        {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        }
        filter = expand_tagged_hand_or_graveyard_disjunction_filter(filter, shape.filter_facts);
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

pub(crate) fn parse_typed_chosen_cant_block_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<TypedChosenCantBlock>, ChosenCantBlockSyntaxError> {
    let Some(shape) = parse_chosen_cant_block_shape(tokens)? else {
        return Ok(None);
    };
    let filter = if shape.bare_other_reference {
        ObjectFilter::default()
    } else {
        crate::runtime_backend::object_filters::parse_object_filter(shape.subject_tokens, false)
            .map_err(|_| ChosenCantBlockSyntaxError::UnsupportedObjectFilter)?
    };
    Ok(Some(TypedChosenCantBlock {
        filter,
        exclude_tagged_choice: shape.exclude_tagged_choice,
    }))
}

pub(crate) fn parse_typed_choice_become_shape<'a>(
    first: &'a [OwnedLexToken],
    second: &'a [OwnedLexToken],
) -> Result<Option<TypedChoiceBecomeShape<'a>>, ChoiceBecomeSyntaxError> {
    let Some(shape) = parse_choice_become_shape(first, second)? else {
        return Ok(None);
    };
    let subject = match shape.subject {
        ChoiceBecomeSubject::Target(tokens) => TypedChoiceBecomeSubject::Target(tokens),
        ChoiceBecomeSubject::AllObjects(tokens) => {
            let filter = crate::runtime_backend::object_filters::parse_object_filter(tokens, false)
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
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        hand_arm.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    let mut graveyard_arm = filter.clone();
    graveyard_arm.zone = Some(Zone::Graveyard);
    graveyard_arm.any_of.clear();
    graveyard_arm
        .tagged_constraints
        .retain(|constraint| constraint.tag.as_str() != IT_TAG);
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
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;
    use crate::types::CardType;

    #[test]
    fn typed_choice_object_clause_returns_filter_and_reference_facts() {
        let tokens = lex_line("You choose a card from it.", 0).unwrap();
        let TypedChoiceObjectClauseKind::Object(parsed) =
            parse_typed_choice_object_clause_tokens(&tokens)
                .unwrap()
                .unwrap()
        else {
            panic!("expected object choice");
        };
        assert!(parsed.references.references_it);
        assert_eq!(parsed.filter.zone, Some(Zone::Hand));
        assert!(
            parsed
                .filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == IT_TAG)
        );
    }

    #[test]
    fn typed_choice_preserves_greatest_value_domain_and_implicit_actor() {
        let tokens = lex_line(
            "Choose a creature with the greatest mana value among creatures they control.",
            0,
        )
        .unwrap();
        let TypedChoiceObjectClauseKind::Object(parsed) =
            parse_typed_choice_object_clause_tokens(&tokens)
                .unwrap()
                .unwrap()
        else {
            panic!("expected object choice");
        };

        assert_eq!(parsed.actor, ChoiceClauseActor::Implicit);
        assert_eq!(
            parsed.filter.controller,
            Some(crate::target::PlayerFilter::IteratedPlayer)
        );
        assert!(matches!(
            parsed.filter.mana_value,
            Some(Comparison::EqualExpr(value))
                if matches!(value.as_ref(), Value::GreatestManaValue(scope)
                    if scope.controller == Some(crate::target::PlayerFilter::IteratedPlayer))
        ));
    }

    #[test]
    fn typed_target_and_sequence_choice_filters_are_owned_by_grammar() {
        let target = lex_line("Target opponent chooses a creature.", 0).unwrap();
        let parsed = parse_typed_target_player_choice_tokens(&target)
            .unwrap()
            .unwrap();
        assert_eq!(parsed.filter.card_types, [CardType::Creature]);

        let block = lex_line("Other creatures can't block this turn.", 0).unwrap();
        let parsed = parse_typed_chosen_cant_block_tokens(&block)
            .unwrap()
            .unwrap();
        assert!(parsed.exclude_tagged_choice);
        assert_eq!(parsed.filter.card_types, [CardType::Creature]);
    }

    #[test]
    fn typed_become_sequence_returns_an_object_filter() {
        let first = lex_line("Choose a creature type.", 0).unwrap();
        let second = lex_line("All creatures become that type.", 0).unwrap();
        let parsed = parse_typed_choice_become_shape(&first, &second)
            .unwrap()
            .unwrap();
        let TypedChoiceBecomeSubject::AllObjects(filter) = parsed.subject else {
            panic!("expected an all-objects subject");
        };
        assert_eq!(filter.card_types, [CardType::Creature]);
    }

    #[test]
    fn typed_choice_filter_rejects_an_effect_clause() {
        let tokens = lex_line("You choose a creature and sacrifice it.", 0).unwrap();
        assert!(
            parse_typed_choice_object_clause_tokens(&tokens)
                .unwrap()
                .is_none()
        );
    }
}
