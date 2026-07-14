use super::SentenceInput;

pub(crate) mod exile_permission_followups;
pub(crate) mod exiled_collections;
pub(crate) mod pairs;
pub(crate) mod quads;
pub(crate) mod triples;
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, ObjectFilter, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst,
    SubjectVerbSubjectAst, TagKey, TargetAst,
};
use crate::effect::{EventValueSpec, Value};
use crate::object::CounterType;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::dispatch_entry::parse_consult_traversal_sentence;
use crate::runtime_backend::grammar::effects::generic_sequence_shapes as sequence_grammar;
use crate::runtime_backend::object_filters::parse_object_filter_lexed;
use crate::runtime_backend::util::helper_tag_for_tokens;
use crate::target::PlayerFilter;
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GenericSequenceVerb {
    GainParameterizedAbility,
    SearchLibraryProcedure,
    IterateLibraryProcedure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GenericSubjectVerbSequence {
    pub(super) verb: GenericSequenceVerb,
    pub(super) consumed_sentences: usize,
}

impl GenericSubjectVerbSequence {
    pub(super) fn parameterized_flashback_grant() -> Self {
        Self {
            verb: GenericSequenceVerb::GainParameterizedAbility,
            consumed_sentences: 2,
        }
    }

    pub(super) fn prefixed_library_consult() -> Self {
        Self {
            verb: GenericSequenceVerb::SearchLibraryProcedure,
            consumed_sentences: 3,
        }
    }

    pub(super) fn iterative_library_procedure() -> Self {
        Self {
            verb: GenericSequenceVerb::IterateLibraryProcedure,
            consumed_sentences: 3,
        }
    }
}

fn effect_ast_is_destroy(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Destroy { .. } | SubjectVerbActionAst::DestroyAll { .. },
            ..
        })
    )
}

pub(crate) fn parse_destroy_for_each_destroyed_consult_exile_put_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = sentences[sentence_idx].lowered();
    let Ok(first_effects) = effect_sentences::parse_effect_sentence_lexed(first_tokens)
        .or_else(|_| effect_sentences::parse_effect_chain(first_tokens))
    else {
        return Ok(None);
    };
    let [destroy_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    if !effect_ast_is_destroy(destroy_effect) {
        return Ok(None);
    }

    let Some(loop_shape) =
        sequence_grammar::parse_destroy_consult_loop_shape(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    let Some(parts) = parse_consult_traversal_sentence(loop_shape.consult_tokens)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary { .. },
            ..
        }))
    ) {
        return Ok(None);
    }

    let third_tokens = sentences[sentence_idx + 2].lowered();
    if !sequence_grammar::parse_put_exiled_then_shuffle_shape(third_tokens) {
        return Ok(None);
    }

    let destroyed_tag = helper_tag_for_tokens(first_tokens, "destroyed");
    let mut loop_effects = parts.effects;
    loop_effects.push(EffectAst::subject_verb_exile(
        TargetAst::Tagged(parts.match_tag.clone(), None),
        false,
    ));
    loop_effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(parts.match_tag, None),
        Zone::Battlefield,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    loop_effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::ItsController,
        SubjectVerbActionAst::ShuffleLibrary,
    ));

    Ok(Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(destroy_effect.clone()),
            tag: destroyed_tag.clone(),
        },
        EffectAst::ForEachTagged {
            tag: destroyed_tag,
            effects: loop_effects,
        },
    ]))
}

pub(crate) fn parse_parameterized_flashback_grant_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let _shape = GenericSubjectVerbSequence::parameterized_flashback_grant();
    let Some(shape) = sequence_grammar::parse_flashback_grant_shape(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };
    let target = effect_sentences::parse_target_phrase(shape.target_tokens)?;

    Ok(Some(vec![EffectAst::subject_verb_grant_to_target(
        target,
        crate::grant::Grantable::flashback_from_cards_mana_cost(),
        crate::grant::GrantDuration::UntilEndOfTurn,
    )]))
}

pub(crate) fn parse_prefixed_library_consult_hand_exile_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let _shape = GenericSubjectVerbSequence::prefixed_library_consult();
    let Ok(prefix_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
            .or_else(|_| effect_sentences::parse_effect_chain(sentences[sentence_idx].lowered()))
    else {
        return Ok(None);
    };
    if prefix_effects.is_empty() {
        return Ok(None);
    }
    let Some(mut combined) =
        pairs::parse_consult_match_into_hand_exile_others(sentences, sentence_idx + 1)?
    else {
        return Ok(None);
    };
    let mut effects = prefix_effects;
    effects.append(&mut combined);
    Ok(Some(effects))
}

pub(crate) fn parse_iterative_library_procedure_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let _shape = GenericSubjectVerbSequence::iterative_library_procedure();
    if !sequence_grammar::parse_iterative_library_sequence_shape(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
        sentences[sentence_idx + 2].lowered(),
    ) {
        return Ok(None);
    }

    let current_tag = TagKey::from("iterative_library_current");
    let exiled_tag = TagKey::from("iterative_library_exiled");
    let all_exiled_filter = ObjectFilter::tagged(exiled_tag.clone()).in_zone(Zone::Exile);
    Ok(Some(vec![EffectAst::RepeatProcess {
        effects: vec![
            EffectAst::subject_verb_exile_top_of_library(
                PlayerAst::You,
                Value::Fixed(1),
                vec![current_tag.clone()],
                vec![exiled_tag.clone()],
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::And(
                    Box::new(PredicateAst::TaggedMatches(
                        current_tag.clone(),
                        ObjectFilter::default().in_zone(Zone::Exile),
                    )),
                    Box::new(PredicateAst::ValueComparison {
                        left: Value::Count(all_exiled_filter.clone()),
                        operator: crate::effect::ValueComparisonOperator::Equal,
                        right: Value::DistinctNames(all_exiled_filter),
                    }),
                ),
                if_true: vec![EffectAst::subject_verb_may_move_to_zone(
                    PlayerAst::You,
                    TargetAst::Tagged(current_tag.clone(), None),
                    Zone::Hand,
                )],
                if_false: Vec::new(),
            },
        ],
        continue_effect_index: 1,
        continue_predicate: IfResultPredicate::WasDeclined,
    }]))
}

pub(crate) fn parse_each_player_repeat_pay_life_tokens_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !sequence_grammar::parse_each_player_pay_life_sequence_shape(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
        sentences[sentence_idx + 2].lowered(),
    ) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::RepeatProcess {
            effects: vec![EffectAst::ForEachPlayer {
                effects: vec![EffectAst::subject_verb_pay_any_life(PlayerAst::That, 0)],
            }],
            continue_effect_index: 0,
            continue_predicate: IfResultPredicate::Did,
        },
        EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::That,
                SubjectVerbActionAst::CreateTokenWithMods {
                    name: "1/1 black Rat creature".to_string(),
                    definition: crate::runtime_backend::grammar::token_definitions::parse_token_definition_shape_text(
                        "1/1 black Rat creature",
                    )
                    .expect("closed-form Rat token definition must remain parseable"),
                    count: Value::PendingEffectMetric {
                        source: ironsmith_core::EffectMetricSource::Outcome,
                        metric: ironsmith_core::EffectMetric::Count,
                    },
                    dynamic_power_toughness: None,
                    player: PlayerAst::That,
                    attached_to: None,
                    tapped: false,
                    attacking: false,
                    exile_at_end_of_combat: false,
                    sacrifice_at_end_of_combat: false,
                    sacrifice_at_next_end_step: false,
                    exile_at_next_end_step: false,
                    next_end_step_player: PlayerFilter::Any,
                    granted_abilities: Vec::new(),
                    ability_presentation: None,
                },
            )],
        },
    ]))
}

pub(crate) fn parse_each_player_shuffle_reveal_then_put_revealed_types_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = sequence_grammar::parse_each_player_reveal_types_shape(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };
    let mut battlefield_filter = parse_object_filter_lexed(shape.battlefield_filter_tokens, false)?;
    battlefield_filter.zone = None;

    if let Some(extra_tokens) = shape.extra_filter_tokens {
        let extra_filter = parse_object_filter_lexed(extra_tokens, false)?;
        for card_type in extra_filter.card_types {
            crate::slice_primitives::push_unique(&mut battlefield_filter.card_types, card_type);
        }
        for subtype in extra_filter.subtypes {
            crate::slice_primitives::push_unique(&mut battlefield_filter.subtypes, subtype);
        }
    }

    if battlefield_filter.card_types.is_empty() && battlefield_filter.subtypes.is_empty() {
        return Ok(None);
    }

    let revealed_tag = TagKey::from("__each_player_revealed_this_way");
    let mut shuffled_filter = ObjectFilter::permanent_card();
    shuffled_filter.zone = Some(Zone::Battlefield);
    shuffled_filter.owner = Some(PlayerFilter::IteratedPlayer);
    let iterated = TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None);

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_shuffle_objects_into_library(
                PlayerAst::That,
                TargetAst::Object(shuffled_filter, None, None),
            ),
            EffectAst::subject_verb_reveal_top_cards(
                PlayerAst::That,
                Value::PendingEffectMetric {
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::Count,
                },
                revealed_tag.clone(),
            ),
            EffectAst::ForEachTagged {
                tag: revealed_tag,
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::ItMatches(battlefield_filter),
                    if_true: vec![EffectAst::subject_verb_move_to_zone(
                        iterated.clone(),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Owner,
                        false,
                        None,
                    )],
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        iterated,
                        Zone::Library,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                }],
            },
        ],
    }]))
}

pub(crate) fn parse_damage_prevention_counter_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(first_effect) = first_effects.first() else {
        return Ok(None);
    };
    if first_effects.len() != 1 {
        return Ok(None);
    }

    let (amount, target, duration) = match first_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PreventDamage {
                    amount,
                    target,
                    duration,
                    ..
                },
            ..
        }) => (Some(amount.clone()), target.clone(), duration.clone()),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PreventAllDamageToTarget {
                    target, duration, ..
                },
            ..
        }) => (None, target.clone(), duration.clone()),
        _ => return Ok(None),
    };

    if !sequence_grammar::parse_prevention_counter_followup_shape(
        sentences[sentence_idx + 1].lowered(),
    ) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_prevent_damage_to_target_put_counters(
            amount,
            target,
            duration,
            CounterType::PlusOnePlusOne,
        ),
    ]))
}

pub(crate) fn parse_damage_prevention_reflect_to_any_target_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(first_effect) = first_effects.first() else {
        return Ok(None);
    };
    if first_effects.len() != 1 {
        return Ok(None);
    }

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PreventDamage {
                amount,
                target,
                duration,
                source_of_your_choice,
                protect_you_and_permanents_you_control,
                ..
            },
        ..
    }) = first_effect
    else {
        return Ok(None);
    };

    if !sequence_grammar::parse_prevention_reflect_followup_shape(
        sentences[sentence_idx + 1].lowered(),
    ) {
        return Ok(None);
    }

    let follow_up = EffectAst::subject_verb_damage(
        Value::EventValue(EventValueSpec::Amount),
        TargetAst::AnyTarget(None),
    );
    Ok(Some(vec![
        EffectAst::subject_verb_prevent_damage_with_options(
            amount.clone(),
            target.clone(),
            duration.clone(),
            *source_of_your_choice,
            *protect_you_and_permanents_you_control,
            vec![follow_up],
        ),
    ]))
}

pub(crate) fn parse_next_damage_prevention_gain_life_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [first_effect] = first_effects.as_mut_slice() else {
        return Ok(None);
    };

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PreventNextTimeDamage {
                follow_up_effects, ..
            },
        ..
    }) = first_effect
    else {
        return Ok(None);
    };
    if !follow_up_effects.is_empty() {
        return Ok(None);
    }

    let Ok(second_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    let [second_effect] = second_effects.as_slice() else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject:
            SubjectVerbSubjectAst {
                player: PlayerAst::You,
                ..
            },
        action: SubjectVerbActionAst::GainLife { amount },
    }) = second_effect
    else {
        return Ok(None);
    };
    if !matches!(amount, Value::EventValue(EventValueSpec::Amount)) {
        return Ok(None);
    }

    follow_up_effects.push(second_effect.clone());
    Ok(Some(first_effects))
}

pub(crate) fn parse_next_damage_prevention_exile_top_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [first_effect] = first_effects.as_mut_slice() else {
        return Ok(None);
    };

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PreventNextTimeDamage {
                follow_up_effects, ..
            },
        ..
    }) = first_effect
    else {
        return Ok(None);
    };
    if !follow_up_effects.is_empty() {
        return Ok(None);
    }

    if !sequence_grammar::parse_prevention_exile_top_followup_shape(
        sentences[sentence_idx + 1].lowered(),
    ) {
        return Ok(None);
    }

    follow_up_effects.push(EffectAst::subject_verb_exile_top_of_library(
        PlayerAst::You,
        Value::EventValue(EventValueSpec::Amount),
        Vec::new(),
        Vec::new(),
    ));
    Ok(Some(first_effects))
}

pub(crate) fn parse_tap_lock_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action: crate::cards::builders::SubjectVerbActionAst::TapAll { filter },
            ..
        }),
    ] = first_effects.as_slice()
    else {
        return Ok(None);
    };

    let second_tokens = sentences[sentence_idx + 1].lowered();
    if !sequence_grammar::parse_source_tapped_lock_shape(second_tokens) {
        return Ok(None);
    }

    let Some((duration, clause_tokens)) =
        effect_sentences::parse_restriction_duration(second_tokens)?
    else {
        return Ok(None);
    };
    if !sequence_grammar::parse_untap_clause_prefix_shape(&clause_tokens) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_tap_all(filter.clone()),
        EffectAst::subject_verb_cant(
            crate::effect::Restriction::untap(filter.clone()),
            duration,
            Some(crate::ConditionExpr::SourceIsTapped),
        ),
    ]))
}

pub(crate) fn parse_search_delayed_upkeep_unless_pays_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = effect_sentences::parse_effect_chain(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if first_effects.is_empty() {
        return Ok(None);
    }

    let Some(shape) = sequence_grammar::parse_delayed_upkeep_payment_shape(
        sentences[sentence_idx + 1].lowered(),
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };

    let mut effects = first_effects;
    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::subject_verb_lose_game(PlayerAst::You)],
            player: PlayerAst::You,
            cost: crate::cost::TotalCost::mana(shape.mana),
        }],
    });
    Ok(Some(effects))
}
