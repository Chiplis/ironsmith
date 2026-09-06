use crate::cards::builders::ObjectChoiceEffectAst;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::ZoneMoveActionAst;
use crate::cards::builders::LibraryActionAst;
use crate::cards::builders::CounterActionAst;
use super::*;
use crate::effect_sentences::parse_artifact_enchantment_or_token_filter;
use crate::grammar::effects as effect_grammar;

fn sacrifice_choice_filter(mut filter: ObjectFilter) -> ObjectFilter {
    if filter.controller.is_none() {
        filter.controller = Some(PlayerFilter::You);
    }
    filter
}

pub fn parse_sentence_each_player_return_with_additional_counter(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_return_with_additional_counter_sentence(clause)
}

pub fn parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_each_player_reveal_permanents_shape(clause.tokens())
    else {
        return Ok(None);
    };
    let count = shape.count;

    let revealed_tag_key = helper_tag_for_tokens(clause.tokens(), "revealed");
    let iterated_target =
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span());

    Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::That,
                count,
                crate::tag::TagRef::of(revealed_tag_key.clone()),
            ),
            EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(revealed_tag_key.clone())),
            EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                tag: crate::tag::TagRef::of(revealed_tag_key),
                effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                    predicate: PredicateAst::ItMatches(shape.matching_filter),
                    if_true: vec![EffectAst::subject_verb_move_to_zone(
                        iterated_target.clone(),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Owner,
                        shape.matching_enters_tapped,
                        None,
                    )],
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        iterated_target,
                        shape.remainder_zone,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                })],
            }),
        ],
    })]))
}

pub fn parse_return_then_do_same_for_subtypes_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_return_same_subtypes_shape(clause.tokens()) else {
        return Ok(None);
    };
    let mut effects = parse_effect_chain(shape.return_tokens)?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let base_effect = effects[0].clone();
    for subtype in shape.subtypes {
        let Some(cloned) = clone_return_effect_with_subtype(&base_effect, subtype) else {
            return Ok(None);
        };
        effects.push(cloned);
    }

    Ok(Some(effects))
}

fn split_choose_same_followup_filters(filter: &ObjectFilter) -> Vec<ObjectFilter> {
    match filter.mana_value.clone() {
        Some(crate::filter::Comparison::OneOf(values)) if !values.is_empty() => values
            .into_iter()
            .map(|value| {
                let mut cloned = filter.clone();
                cloned.mana_value = Some(crate::filter::Comparison::Equal(value));
                cloned
            })
            .collect(),
        _ => vec![filter.clone()],
    }
}

pub fn parse_choose_then_do_same_for_filter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_choose_same_filter_shape(clause.tokens()) else {
        return Ok(None);
    };
    let Some((player, base_filter, count)) = parse_you_choose_objects_clause(shape.head_tokens)?
        .or_else(|| {
            crate::grammar::primitives::probe_shape(parse_target_player_choose_objects_clause(
                shape.head_tokens,
            ))
            .flatten()
        })
    else {
        return Ok(None);
    };
    let tag = crate::tag::CompilerReferenceTag::It.bind();

    let followup_filter = parse_object_filter(shape.filter_tokens, false)?;
    if followup_filter.controller.is_some() || followup_filter.owner.is_some() {
        return Ok(None);
    }

    let merged_filter = merge_filters(&base_filter, &followup_filter);
    let followup_filters = split_choose_same_followup_filters(&merged_filter);
    if followup_filters.is_empty() {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
        filter: base_filter.clone(),
        count,
        count_value: None,
        player,
        tag: tag.clone(),
    })];
    for filter in followup_filters {
        effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count,
            count_value: None,
            player,
            tag: tag.clone(),
        }));
    }

    Ok(Some(effects))
}

fn parse_choose_objects_clause_for_chain(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    if let Some(parsed) = clause.parse_value_with_lexed(parse_you_choose_objects_clause)? {
        return Ok(Some(parsed));
    }
    clause.parse_value_with_lexed(parse_target_player_choose_objects_clause)
}

fn preserve_choose_clause_it_reference(references_prior_choice: bool, filter: &mut ObjectFilter) {
    if !references_prior_choice {
        return;
    }
    if filter.zone.is_none() || filter.zone == Some(Zone::Battlefield) {
        filter.zone = Some(Zone::Hand);
    }
    filter.controller = None;
    filter.owner = None;
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
}

pub fn parse_choose_then_choose_objects_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_choose_sequence_shape(clause.tokens()) else {
        return Ok(None);
    };
    let Some((first_player, mut first_filter, first_count)) =
        parse_choose_objects_clause_for_chain(SubjectVerbPrimitiveClause::new(shape.head_tokens))?
    else {
        return Ok(None);
    };

    preserve_choose_clause_it_reference(shape.head_references_prior_choice, &mut first_filter);

    let first = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
        filter: first_filter,
        count: first_count,
        count_value: None,
        player: first_player,
        tag: crate::tag::CompilerReferenceTag::It.bind(),
    });

    let Some((mut second_player, mut second_filter, second_count)) =
        parse_choose_objects_clause_for_chain(SubjectVerbPrimitiveClause::new(shape.tail_tokens))?
    else {
        // A direct choice can be followed by a quantified participant choice,
        // e.g. `then each other player chooses ...`. Keep that participant
        // loop typed so the chosen-set normalization pass can union both
        // producers before a later complement consumer.
        let Some(participant_choice) =
            super::super::for_each_helpers::parse_for_each_player_clause(shape.tail_tokens)?
        else {
            return Ok(None);
        };
        return Ok(Some(vec![first, participant_choice]));
    };

    preserve_choose_clause_it_reference(shape.tail_references_prior_choice, &mut second_filter);

    if second_player == PlayerAst::Implicit {
        second_player = first_player;
    }

    Ok(Some(vec![
        first,
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: second_filter,
            count: second_count,
            count_value: None,
            player: second_player,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
        }),
    ]))
}

pub fn parse_sentence_return_then_do_same_for_subtypes(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_then_do_same_for_subtypes_sentence(clause)
}

pub fn parse_sentence_choose_then_choose_objects(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_choose_then_choose_objects_sentence(clause)
}

pub fn parse_sentence_choose_then_do_same_for_filter(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_choose_then_do_same_for_filter_sentence(clause)
}

pub fn parse_sacrifice_any_number_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_sacrifice_choice_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.count != ChoiceCount::any_number() {
        return Ok(None);
    }

    let parsed_filter =
        if let Some(filter) = parse_artifact_enchantment_or_token_filter(shape.filter_tokens) {
            filter
        } else {
            parse_object_filter(shape.filter_tokens, false)?
        };
    let filter = sacrifice_choice_filter(parsed_filter);
    let tag = crate::tag::CompilerReferenceTag::It.bind();

    let mut effects = vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::any_number(),
            count_value: None,
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        }),
        EffectAst::subject_verb_sacrifice_all(PlayerAst::Implicit, ObjectFilter::tagged(tag)),
    ];
    if let Some(tail_tokens) = shape.tail_tokens {
        let mut tail_effects = parse_effect_chain(tail_tokens)?;
        effects.append(&mut tail_effects);
    }

    Ok(Some(effects))
}

pub fn parse_sentence_sacrifice_any_number(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_any_number_sentence(clause)
}

pub fn parse_sacrifice_one_or_more_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_sacrifice_choice_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.count != ChoiceCount::at_least(1) {
        return Ok(None);
    }
    let parsed_filter =
        if let Some(filter) = parse_artifact_enchantment_or_token_filter(shape.filter_tokens) {
            filter
        } else {
            parse_object_filter(shape.filter_tokens, false)?
        };
    let filter = sacrifice_choice_filter(parsed_filter);
    let tag = crate::tag::CompilerReferenceTag::It.bind();
    Ok(Some(vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count: shape.count,
            count_value: None,
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        }),
        EffectAst::subject_verb_sacrifice_all(PlayerAst::Implicit, ObjectFilter::tagged(tag)),
    ]))
}

pub fn parse_sentence_sacrifice_one_or_more(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_one_or_more_sentence(clause)
}

pub fn parse_sentence_keyword_then_chain(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_then_sequence_shape(clause.tokens()) else {
        return Ok(None);
    };
    let Some(head_effect) = parse_keyword_mechanic_clause(shape.head_tokens)? else {
        return Ok(None);
    };

    let mut effects = vec![head_effect];
    let tail_clause = SubjectVerbPrimitiveClause::new(shape.tail_tokens);
    if let Some(mut counter_effects) = parse_sentence_put_counter_sequence(tail_clause)? {
        effects.append(&mut counter_effects);
        return Ok(Some(effects));
    }

    let mut tail_effects = parse_effect_chain(shape.tail_tokens)?;
    effects.append(&mut tail_effects);
    Ok(Some(effects))
}

pub fn parse_sentence_chain_then_keyword(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_then_sequence_shape(clause.tokens()) else {
        return Ok(None);
    };
    let Some(keyword_effect) = parse_keyword_mechanic_clause(shape.tail_tokens)? else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(shape.head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    head_effects.push(keyword_effect);
    Ok(Some(head_effects))
}

pub fn parse_sentence_return_then_create(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_return_create_shape(clause.tokens()) else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(shape.return_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(shape.create_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub fn parse_sentence_exile_then_may_put_from_exile(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_exile_may_put_shape(clause.tokens()) else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(shape.exile_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    let mut tail_effects = parse_effect_chain(shape.put_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub fn parse_exile_then_shuffle_graveyard_into_library_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_exile_shuffle_shape(clause.tokens()) else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(shape.head_tokens)?;
    if !head_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { .. }),
                ..
            }) | EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { .. }),
                ..
            }) | EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves { .. }),
                ..
            })
        )
    }) {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(shape.tail_tokens)?;
    if !tail_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Library(LibraryActionAst::ShuffleGraveyardIntoLibrary { .. }),
                ..
            })
        )
    }) {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

pub fn parse_exile_source_with_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_exile_source_counter_shape(clause.tokens()) else {
        return Ok(None);
    };
    let (count, counter_type) = match parse_counter_descriptor(shape.descriptor_tokens) {
        Ok(descriptor) => descriptor,
        Err(error) if shape.source_reference => return Err(error),
        Err(_) => return Ok(None),
    };
    let (exile_target, counter_target) = if shape.source_reference {
        let source = TargetAst::Source(clause.span());
        (source.clone(), source)
    } else if shape.it_reference {
        let it = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span());
        (it.clone(), it)
    } else {
        (
            parse_target_phrase(shape.target_tokens)?,
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span()),
        )
    };
    Ok(Some(vec![
        EffectAst::subject_verb_exile(exile_target, false),
        EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            counter_target,
            None,
            false,
        ),
    ]))
}

pub fn parse_sentence_exile_source_with_counters(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_source_with_counters_sentence(clause)
}

pub fn parse_sentence_comma_then_chain_special(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_comma_then_special_shape(clause.tokens()) else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(shape.head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(shape.tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    if shape.tail == effect_grammar::CommaThenTailShape::ThatPlayer
        && head_is_single_return_to_hand(&head_effects)
    {
        bind_that_player_tail_to_returned_owner(&mut tail_effects);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

fn head_is_single_return_to_hand(effects: &[EffectAst]) -> bool {
    let [effect] = effects else {
        return false;
    };

    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { .. }),
            ..
        })
    )
}

fn bind_that_player_tail_to_returned_owner(effects: &mut [EffectAst]) {
    for effect in effects {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && subject_verb.subject.player == PlayerAst::That
        {
            subject_verb.subject.player = PlayerAst::ItsOwner;
        }
    }
}

pub fn parse_destroy_then_land_controller_graveyard_count_damage_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_destroy_land_damage_shape(clause.tokens()) else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(shape.destroy_tokens)?;
    if !head_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { .. }),
                ..
            })
        )
    }) {
        return Ok(None);
    }

    let mut count_filter = ObjectFilter::default();
    count_filter.zone = Some(Zone::Graveyard);
    let tagged_ref = crate::target::ObjectRef::tagged(crate::tag::CompilerReferenceTag::It.bind());
    count_filter.owner = Some(PlayerFilter::ControllerOf(tagged_ref.clone()));
    count_filter.card_types.push(CardType::Land);
    head_effects.push(EffectAst::subject_verb_damage(
        Value::Count(count_filter),
        TargetAst::Player(
            PlayerFilter::ControllerOf(tagged_ref),
            span_from_tokens(shape.damage_tokens),
        ),
    ));
    Ok(Some(head_effects))
}

pub fn parse_sentence_destroy_all_attached_to_target(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_destroy_attached_shape(clause.tokens()) else {
        return Ok(None);
    };
    let filter = parse_object_filter(shape.filter_tokens, false)?;
    let target = parse_target_phrase(shape.target_tokens)?;
    Ok(Some(vec![EffectAst::subject_verb_destroy_all_attached_to(
        filter, target,
    )]))
}

pub fn parse_sentence_destroy_then_land_controller_graveyard_count_damage(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_then_land_controller_graveyard_count_damage_sentence(clause)
}

pub fn find_color_choice_phrase(clause: SubjectVerbPrimitiveClause<'_>) -> Option<(usize, usize)> {
    effect_grammar::parse_color_choice_phrase_span(clause.tokens())
        .map(|shape| (shape.start, shape.len))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn targeted_graveyard_exile_accepts_a_named_counter_payload() {
        let tokens = lex_line(
            "exile up to one target Assassin creature card from your graveyard with a memory counter on it",
            0,
        )
        .expect("exile-with-counter sentence should lex");
        let effects =
            parse_exile_source_with_counters_sentence(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("exile-with-counter sentence should parse")
                .expect("typed exile-with-counter shape should match");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 2, "{debug}");
        assert!(debug.contains("action: Exile"), "{debug}");
        assert!(debug.contains("action: PutCounters"), "{debug}");
        assert!(matches!(
            effects.get(1),
            Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
                    counter_type: crate::CounterType::Named(name),
                    ..
                }),
                ..
            })) if name.as_str() == "memory"
        ));
        assert!(matches!(
            effects.first(),
            Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
                    target: TargetAst::WithCount(inner, _),
                    ..
                }),
                ..
            })) if matches!(
                inner.as_ref(),
                TargetAst::Object(filter, ..) if filter.zone == Some(Zone::Graveyard)
            )
        ));
    }
}
