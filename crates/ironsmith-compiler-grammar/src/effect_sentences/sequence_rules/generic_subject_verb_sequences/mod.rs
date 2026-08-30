use super::SentenceInput;

#[path = "branching_selection.rs"]
pub mod branching_selection_programs;
pub mod exile_permission_followups;
pub mod exiled_collections;
pub mod graveyard_copy_cast;
pub mod optional_sacrifice_discard;
#[path = "ordered_control_flow.rs"]
pub mod ordered_control_flow_programs;
#[path = "reference_linked.rs"]
pub mod reference_linked_programs;

use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, ObjectFilter, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey,
    TargetAst,
};
use crate::effect::{EventValueSpec, Value};
use crate::effect_sentences;
use crate::effect_sentences::dispatch_entry::parse_consult_traversal_sentence;
use crate::grammar::effects::generic_sequence_shapes as sequence_grammar;
use crate::object::CounterType;
use crate::object_filters::parse_object_filter_lexed;
use crate::target::PlayerFilter;
use crate::util::helper_tag_for_tokens;
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

pub fn parse_destroy_for_each_destroyed_consult_exile_put_shuffle(
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

pub fn parse_parameterized_flashback_grant_sequence(
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
        crate::model::CompilerGrantableCore::flashback_from_cards_mana_cost(),
        crate::grant::GrantDuration::UntilEndOfTurn,
    )]))
}

pub fn parse_prefixed_library_consult_hand_exile_sequence(
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
    let Some(mut combined) = reference_linked_programs::parse_consult_match_into_hand_exile_others(
        sentences,
        sentence_idx + 1,
    )?
    else {
        return Ok(None);
    };
    let mut effects = prefix_effects;
    effects.append(&mut combined);
    Ok(Some(effects))
}

pub fn parse_iterative_library_procedure_sequence(
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

    let current_tag = crate::tag::CompilerReferenceTag::IterativeLibraryCurrent.key();
    let exiled_tag = crate::tag::CompilerReferenceTag::IterativeLibraryExiled.key();
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

pub fn parse_each_player_repeat_pay_life_tokens_sequence(
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
            effects: vec![EffectAst::SourceSentence {
                effects: vec![EffectAst::ForEachPlayer {
                    effects: vec![EffectAst::subject_verb_pay_any_life(PlayerAst::That, 0)],
                }],
                leading_then: false,
                starting_with_controller: true,
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
                    definition:
                        crate::grammar::token_definitions::parse_token_definition_shape_text(
                            "1/1 black Rat creature",
                        )
                        .expect("closed-form Rat token definition must remain parseable"),
                    count: Value::PendingEffectMetric {
                        source: ironsmith_core::EffectMetricSource::Outcome,
                        metric: ironsmith_core::EffectMetric::Count,
                    },
                    dynamic_power_toughness: None,
                    player: PlayerAst::That,
                    actor_surface_explicit: false,
                    attached_to: None,
                    tapped: false,
                    attacking: false,
                    attack_target_player: None,
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

pub fn parse_starting_each_player_optional_repeat_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = sequence_grammar::parse_starting_each_player_optional_repeat_shape(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };

    let Ok(parsed) = effect_sentences::parse_effect_sentence_lexed(shape.each_player_clause_tokens)
        .or_else(|_| effect_sentences::parse_effect_chain(shape.each_player_clause_tokens))
    else {
        return Ok(None);
    };
    let [
        EffectAst::ForEachPlayer {
            effects: per_player_effects,
        },
    ] = parsed.as_slice()
    else {
        return Ok(None);
    };
    if !matches!(
        per_player_effects.as_slice(),
        [EffectAst::May { .. } | EffectAst::MayByPlayer { .. }]
    ) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::RepeatProcess {
        effects: vec![EffectAst::SourceSentence {
            effects: parsed,
            leading_then: false,
            starting_with_controller: true,
        }],
        continue_effect_index: 0,
        continue_predicate: IfResultPredicate::Did,
    }]))
}

pub fn parse_each_player_shuffle_reveal_then_put_revealed_types_bottom(
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

    let revealed_tag = crate::tag::CompilerReferenceTag::EachPlayerRevealedThisWay.key();
    let mut shuffled_filter = ObjectFilter::permanent_card();
    shuffled_filter.zone = Some(Zone::Battlefield);
    shuffled_filter.owner = Some(PlayerFilter::IteratedPlayer);
    let iterated = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None);

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_shuffle_all_objects_into_library(
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

pub fn parse_damage_prevention_counter_sequence(
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

/// Preserve a prevention shield's exact target and actual prevented amount
/// through a creature-only counter instruction scheduled for the next end
/// step. The delayed scheduler already captures the immediately preceding
/// prevention shield when its payload uses `EventValue::Amount`.
pub fn parse_damage_prevention_delayed_counter_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [
        first_effect @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PreventDamage { target, .. },
            ..
        }),
    ] = first_effects.as_slice()
    else {
        return Ok(None);
    };
    if !matches!(target, TargetAst::AnyTarget(Some(_))) {
        return Ok(None);
    }

    let words = crate::lexer::token_word_refs(sentences[sentence_idx + 1].lowered());
    let expected_suffix = [
        "put",
        "a",
        "+0/+1",
        "counter",
        "on",
        "it",
        "for",
        "each",
        "1",
        "damage",
        "prevented",
        "this",
        "way",
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "next",
        "end",
        "step",
    ];
    let prefix_len = if crate::word_primitives::parse_any_sequence_prefix(
        &words,
        &[
            &["if", "its", "a", "creature"],
            &["if", "it's", "a", "creature"],
        ],
    ) {
        4
    } else if crate::word_primitives::parse_sequence_prefix(
        &words,
        &["if", "it", "is", "a", "creature"],
    ) {
        5
    } else {
        return Ok(None);
    };
    if !crate::word_primitives::parse_sequence_complete(&words[prefix_len..], &expected_suffix) {
        return Ok(None);
    }

    let put = EffectAst::subject_verb_put_counters(
        CounterType::PlusZeroPlusOne,
        Value::PendingPriorEffectMetric(
            ironsmith_core::PriorEffectMetricQuery::new(
                ironsmith_core::EffectMetricSource::Outcome,
                ironsmith_core::EffectMetric::DamagePrevented,
            )
            .with_action(ironsmith_core::PriorEffectAction::Prevented),
        ),
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::Targeted0.key(), None),
        None,
        false,
    );
    let conditional = EffectAst::Conditional {
        predicate: PredicateAst::TargetMatches(ObjectFilter::creature()),
        if_true: vec![EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![put],
        }],
        if_false: Vec::new(),
    };
    Ok(Some(vec![first_effect.clone(), conditional]))
}

/// Keep a destroy set and its authored no-regeneration rider as one action.
/// Re-parsing `They` independently loses relative selectors such as
/// `that aren't enchanted` from the destroyed set.
pub fn parse_destroy_then_no_regeneration_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::lexer::token_word_refs(sentences[sentence_idx + 1].lowered());
    if !crate::word_primitives::last_is(&words, "regenerated")
        || !crate::word_primitives::first_is_any(&words, &["it", "they", "those"])
        || !crate::slice_primitives::contains_any(&words, &["cant", "can't"])
    {
        return Ok(None);
    }
    let Some(first_tail) = sentences[sentence_idx]
        .lowered()
        .first()
        .is_some_and(|token| token.is_word("destroy"))
        .then_some(&sentences[sentence_idx].lowered()[1..])
    else {
        return Ok(None);
    };
    let mut first = super::super::zone_handlers::parse_destroy(first_tail)?;
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = &mut first else {
        return Ok(None);
    };
    let authored_unenchanted = {
        // Parser-token normalization may simplify a relative attachment-state
        // clause before this two-sentence rule runs. Keep the normalized tail
        // for ordinary destroy parsing, but prove the negative Aura predicate
        // from the retained authored sentence.
        let authored_tail = if sentences[sentence_idx]
            .lexed()
            .first()
            .is_some_and(|token| token.is_word("destroy"))
        {
            &sentences[sentence_idx].lexed()[1..]
        } else {
            first_tail
        };
        let words = crate::lexer::token_word_refs(authored_tail);
        crate::word_primitives::any_sequence_occurs(
            &words,
            &[
                &["that", "aren't", "enchanted"],
                &["that", "arent", "enchanted"],
                &["that", "are", "not", "enchanted"],
            ],
        )
    };
    let singular_followup = crate::word_primitives::first_is(&words, "it");
    match action {
        SubjectVerbActionAst::Destroy {
            no_regeneration, ..
        } => *no_regeneration = true,
        SubjectVerbActionAst::DestroyAll {
            no_regeneration, ..
        }
        | SubjectVerbActionAst::DestroyAllOfChosenColor {
            no_regeneration, ..
        } if !singular_followup => *no_regeneration = true,
        _ => return Ok(None),
    }
    if authored_unenchanted {
        let mut aura = ObjectFilter::enchantment();
        aura.subtypes.push(crate::types::Subtype::Aura);
        match action {
            SubjectVerbActionAst::Destroy { target, .. } => {
                if let Some(filter) =
                    super::super::zone_counter_helpers::target_object_filter_mut(target)
                    && filter.without_attached_object.is_none()
                {
                    filter.without_attached_object = Some(Box::new(aura));
                }
            }
            SubjectVerbActionAst::DestroyAll { filter, .. }
                if filter.without_attached_object.is_none() =>
            {
                filter.without_attached_object = Some(Box::new(aura));
            }
            _ => {}
        }
    }
    Ok(Some(vec![first]))
}

/// Preserve an exhaustive, shared owner-relative union across hand and
/// graveyard. The ordinary sentence route otherwise lowers the two zones as
/// unrelated generic exile clauses.
pub fn parse_reveal_then_exile_noncreature_nonland_hand_graveyard_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let second_words = crate::lexer::token_word_refs(sentences[sentence_idx + 1].lowered());
    if !crate::word_primitives::parse_sequence_prefix(
        &second_words,
        &["exile", "all", "noncreature", "nonland", "cards", "from"],
    ) || !crate::slice_primitives::contains_all(&second_words, &["that", "hand", "graveyard"])
        || !crate::slice_primitives::contains_any(&second_words, &["player", "players", "player's"])
    {
        return Ok(None);
    }
    let mut effects =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())?;
    if effects.is_empty() {
        return Ok(None);
    }
    let mut hand = ObjectFilter::default();
    hand.zone = Some(Zone::Hand);
    let mut graveyard = ObjectFilter::default();
    graveyard.zone = Some(Zone::Graveyard);
    let mut union = ObjectFilter::default();
    union.owner = Some(PlayerFilter::target_opponent());
    union.excluded_card_types = vec![
        crate::types::CardType::Creature,
        crate::types::CardType::Land,
    ];
    union.any_of = vec![hand, graveyard];
    effects.push(EffectAst::subject_verb_exile_all(union, false));
    Ok(Some(effects))
}

#[path = "trigger.rs"]
mod trigger_programs;
pub use trigger_programs::parse_delayed_upkeep_unless_pays_sequence;
#[path = "library.rs"]
mod library_programs;
pub use library_programs::{
    parse_next_damage_prevention_exile_top_sequence,
    parse_search_delayed_upkeep_unless_pays_sequence,
};
#[path = "core.rs"]
mod core_programs;
pub use core_programs::parse_tap_lock_sequence;
#[path = "combat.rs"]
mod combat_programs;
pub use combat_programs::{
    parse_damage_prevention_reflect_to_any_target_sequence,
    parse_next_damage_prevention_gain_life_sequence,
};
