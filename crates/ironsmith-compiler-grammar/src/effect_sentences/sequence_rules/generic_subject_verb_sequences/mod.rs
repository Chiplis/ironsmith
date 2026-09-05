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

    let current_tag = crate::tag::CompilerReferenceTag::IterativeLibraryCurrent.bind();
    let exiled_tag = crate::tag::CompilerReferenceTag::IterativeLibraryExiled.bind();
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

    let revealed_tag = crate::tag::CompilerReferenceTag::EachPlayerRevealedThisWay.bind();
    let mut shuffled_filter = ObjectFilter::permanent_card();
    shuffled_filter.zone = Some(Zone::Battlefield);
    shuffled_filter.owner = Some(PlayerFilter::IteratedPlayer);
    let iterated = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);

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

#[path = "trigger.rs"]
mod trigger_programs;
#[path = "library.rs"]
mod library_programs;
#[path = "core.rs"]
mod core_programs;
#[path = "combat.rs"]
mod combat_programs;


/// The closed-form "1/1 black Rat creature" definition, spelled as the shape
/// its text parses to rather than as text to tokenize; a test keeps the two
/// in step.
pub(crate) fn rat_token_definition() -> crate::model::token_definition::TokenDefinitionSpec {
    use crate::model::token_definition::{
        CreatureTokenRulesShape, CreatureTokenShape, TokenDefinitionSpec,
    };
    TokenDefinitionSpec::Creature(CreatureTokenShape {
        name: "Rat".to_string(),
        card_types: vec![crate::types::CardType::Creature],
        subtypes: vec![crate::types::Subtype::Rat],
        power_toughness: (1, 1),
        legendary: false,
        colors: crate::color::ColorSet::BLACK,
        use_source_chosen_color: false,
        use_source_chosen_creature_type: false,
        keywords: Vec::new(),
        rules: CreatureTokenRulesShape::default(),
    })
}

#[cfg(test)]
mod rat_token_definition_tests {
    #[test]
    fn rat_token_definition_is_the_shape_its_text_parses_to() {
        assert_eq!(
            Some(super::rat_token_definition()),
            crate::grammar::token_definitions::parse_token_definition_shape_text(
                "1/1 black Rat creature"
            ),
        );
    }
}
