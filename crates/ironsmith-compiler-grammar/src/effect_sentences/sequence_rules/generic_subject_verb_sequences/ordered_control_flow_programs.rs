use super::super::super::clause_pattern_helpers::parse_choose_target_prelude_sentence;
use super::super::super::clause_primitives::parse_choose_card_name_clause;
use super::super::super::dispatch_entry::{
    ConsultCastCost, consult_cast_effects, consult_stop_rule_is_single_match,
    parse_bargained_face_down_cast_mana_value_gate, parse_consult_bottom_remainder_clause,
    parse_consult_cast_clause, parse_consult_traversal_sentence,
    parse_if_declined_put_match_into_hand, parse_if_you_cant_sentence, parse_if_you_dont_sentence,
    parse_looked_card_choice_filter, parse_top_cards_view_sentence,
};
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, IfResultPredicate, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, ObjectFilter, PlayerAst, PredicateAst, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey,
    TargetAst, TextSpan,
};
use crate::effect::{ChoiceCount, Value};
use crate::effect_sentences;
use crate::effect_sentences::SentenceInput;
use crate::grammar::effects::{
    ExileLibraryPlayerShape, control_copy_attach_shapes::BattlefieldControllerShape,
    looked_card_shapes as looked_grammar, parse_exile_dynamic_top_library_shape,
    sequence_quad_shapes as quad_grammar, triple_sequence_shapes as triple_grammar,
};
use crate::grammar::lexical::TokenWordView;
use crate::grammar::sentence_markers::{self, ConditionalFollowupActor, LeadingMayActor};
use crate::grammar::shared_util::aggregate_constraints::lift_total_mana_value_choice_constraint;
use crate::lexer::OwnedLexToken;
use crate::object::CounterType;
use crate::object_filters::parse_object_filter_lexed;
use crate::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::target::ChooseSpec;
use crate::target::{ObjectRef, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::util::{
    helper_tag_for_tokens, parse_target_phrase, strip_leading_token_words_any, trim_commas,
};
use crate::zone::Zone;

/// Preserve a conditional `instead` arm together with the common sentence
/// that follows both outcomes:
///
/// `Target ... gets ... . Put a counter on it instead if ... . Then it deals ... .`
///
/// Parsing the three sentences independently loses the replacement sentence,
/// while attaching the final damage only to the nearest arm changes runtime
/// behavior. The exact typed shapes below prove one default modifier, one
/// conditional counter replacement, and one common damage continuation.
pub fn parse_target_modifier_counter_instead_then_common_damage(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let default_effects =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())?;
    let replacement_sentence = sentences[sentence_idx + 1].lowered();
    let replacement_effects = effect_sentences::parse_effect_sentence_lexed(replacement_sentence)?;
    let common_effects =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 2].lowered())?;

    let [
        default @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Pump { .. },
            ..
        }),
    ] = default_effects.as_slice()
    else {
        return Ok(None);
    };
    let parsed_replacement = match replacement_effects.as_slice() {
        [EffectAst::TrailingIf { predicate, effects }] => {
            Some((predicate.clone(), effects.clone()))
        }
        [EffectAst::ControlFlow(control)] => {
            let crate::model::ControlFlowNodeAst::Condition {
                condition,
                consequence_program,
                alternative_program: None,
                ..
            } = &control.node
            else {
                return Ok(None);
            };
            if condition.position != crate::model::ConditionPositionAst::Postcondition {
                return Ok(None);
            }
            let crate::model::ControlPredicateAst::State(predicate) = &condition.predicate else {
                return Ok(None);
            };
            let Some(program) = control.programs.get(*consequence_program) else {
                return Ok(None);
            };
            Some((predicate.clone(), program.effects.clone()))
        }
        _ => {
            // `instead if` changes the relationship between this sentence and
            // the preceding one; it is not part of the counter action itself.
            // Parse both owned clauses explicitly when the ordinary standalone
            // sentence route correctly declines that cross-sentence shape.
            let view = TokenWordView::new(replacement_sentence);
            let Some(instead_word) =
                crate::slice_primitives::select_position(&view.word_refs(), |word| {
                    *word == "instead"
                })
            else {
                return Ok(None);
            };
            let Some(instead_token) = view.map_word_to_token_start(instead_word) else {
                return Ok(None);
            };
            let action = effect_sentences::parse_effect_sentence_lexed(
                &replacement_sentence[..instead_token],
            )?;
            let Some(predicate) =
                crate::grammar::structure::parse_trailing_instead_if_predicate_lexed(
                    &replacement_sentence[instead_token..],
                )
            else {
                return Ok(None);
            };
            Some((predicate, action))
        }
    };
    let Some((predicate, replacement)) = parsed_replacement else {
        return Ok(None);
    };
    if !matches!(
        replacement.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutCounters { .. },
            ..
        })]
    ) || !sentences[sentence_idx + 1]
        .lowered()
        .iter()
        .any(|token| token.is_word("instead"))
    {
        return Ok(None);
    }
    if !matches!(
        common_effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamageEqualToPower { .. },
            ..
        })]
    ) {
        return Ok(None);
    }

    let predicate = match predicate {
        PredicateAst::ItMatches(filter) => PredicateAst::TargetMatches(filter),
        PredicateAst::TargetMatches(filter) => PredicateAst::TargetMatches(filter),
        _ => return Ok(None),
    };
    let mut if_true = replacement;
    if_true.extend(common_effects.clone());
    let mut if_false = vec![default.clone()];
    if_false.extend(common_effects);
    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate,
        if_true,
        if_false,
        attach_to_previous_ability: false,
    }]))
}

/// Keep a damage outcome, its excess-damage-derived exile count, and the
/// resulting play permission in one reference-resolution program.
pub fn parse_damage_then_excess_exile_top_then_play_until_next_turn(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_effects =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())?;
    let [
        damage @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamage { .. },
            ..
        }),
    ] = first_effects.as_slice()
    else {
        return Ok(None);
    };

    let exile_tokens = sentences[sentence_idx + 1].lowered();
    let exile_body = strip_leading_token_words_any(exile_tokens, &["exile"]);
    if exile_body.len() == exile_tokens.len() {
        return Ok(None);
    }
    let Some(shape) = parse_exile_dynamic_top_library_shape(exile_body, PlayerAst::Implicit) else {
        return Ok(None);
    };
    let ExileLibraryPlayerShape::Player(player) = shape.player else {
        return Ok(None);
    };
    if player != PlayerAst::You
        || shape.face_down
        || !matches!(
            shape.count.unhinted(),
            Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::ExcessDamage,
            }
        )
    {
        return Ok(None);
    }

    let tag = helper_tag_for_tokens(exile_tokens, "exiled");
    let Some(permission) = parse_cast_or_play_tagged_clause(sentences[sentence_idx + 2].lowered())?
    else {
        return Ok(None);
    };
    let Some(permission) =
        super::exile_permission_followups::rebind_permission_tag(permission, tag.clone())
    else {
        return Ok(None);
    };
    if !matches!(
        &permission,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                tag: permission_tag,
                player: PlayerAst::You,
                allow_land: true,
                until_next_end_step: false,
                ..
            },
            ..
        }) if permission_tag == &tag
    ) {
        return Ok(None);
    }

    Ok(Some(vec![
        damage.clone(),
        EffectAst::subject_verb_exile_top_of_library(player, shape.count, vec![tag], Vec::new()),
        permission,
    ]))
}

/// Preserve the exact result collection across:
///
/// "Each player mills a card. If a land card was milled this way, create ... .
/// Until end of turn, you may cast a spell from among those cards."
///
/// The tag is the semantic boundary: the land test and permission can see only
/// cards affected by this mill instruction, never unrelated graveyard cards.
pub fn parse_each_player_mill_then_land_result_then_cast_one_milled_spell(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = sentences[sentence_idx].lowered();
    let second_tokens = sentences[sentence_idx + 1].lowered();
    let third_tokens = sentences[sentence_idx + 2].lowered();
    if !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(first_tokens),
        &["each", "player", "mills", "a", "card"],
    ) || !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(second_tokens),
        &[
            "if", "a", "land", "card", "was", "milled", "this", "way", "create", "a", "treasure",
            "token",
        ],
    ) || !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(third_tokens),
        &[
            "until", "end", "of", "turn", "you", "may", "cast", "a", "spell", "from", "among",
            "those", "cards",
        ],
    ) {
        return Ok(None);
    }

    let Ok(mut mill_effects) = effect_sentences::parse_effect_sentence_lexed(first_tokens) else {
        return Ok(None);
    };
    let [mill_effect] = mill_effects.as_mut_slice() else {
        return Ok(None);
    };
    let milled_tag = helper_tag_for_tokens(first_tokens, "milled");
    if super::reference_linked_programs::tag_single_mill_effect(mill_effect, &milled_tag).is_none()
    {
        let exact_each_player_mill = matches!(
            mill_effect,
            EffectAst::ForEachPlayer { effects }
                if matches!(effects.as_slice(), [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Mill { .. },
                    ..
                })])
        );
        if !exact_each_player_mill {
            return Ok(None);
        }
        let whole_batch = mill_effect.clone();
        *mill_effect = EffectAst::TagAffected {
            effect: Box::new(whole_batch),
            tag: milled_tag.clone(),
        };
    }

    let Some(create_start) =
        crate::slice_primitives::select_position(second_tokens, |token| token.is_word("create"))
    else {
        return Ok(None);
    };
    let Ok(create_effects) =
        effect_sentences::parse_effect_sentence_lexed(&second_tokens[create_start..])
    else {
        return Ok(None);
    };
    if create_effects.len() != 1 {
        return Ok(None);
    }
    let mut land = ObjectFilter::default();
    land.card_types = vec![CardType::Land];
    land.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Milled));

    let permission_surface = ironsmith_core::GrantPlayTaggedSurface::default()
        .with_leading_duration(true)
        .with_object(ironsmith_core::GrantPlayTaggedObjectSurface::SpellsFromAmongThoseCards);
    let permission =
        EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_with_optional_surface(
            milled_tag.clone(),
            PlayerAst::You,
            false,
            false,
            ironsmith_core::value_model::ManaSpendMode::Normal,
            Some(permission_surface),
        )
        .with_tagged_play_max_plays(Some(1));

    Ok(Some(vec![
        mill_effects.pop().expect("one parsed mill effect"),
        EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(milled_tag, land),
            if_true: create_effects,
            if_false: Vec::new(),
        },
        permission,
    ]))
}

#[cfg(test)]
#[path = "ordered_control_flow_inline_mill_result_permission_tests.rs"]
mod mill_result_permission_tests;

fn look_at_top_cards_parts(effect: &EffectAst) -> Option<(PlayerAst, Value)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: crate::cards::builders::SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::LookAtTopCards { count, .. },
    }) = effect
    else {
        return None;
    };
    Some((*player, count.clone()))
}

fn chosen_kind_consult_branch_effects(
    tokens: &[OwnedLexToken],
    filter: ObjectFilter,
    order: crate::cards::builders::LibraryBottomOrderAst,
) -> Vec<EffectAst> {
    let all_tag = helper_tag_for_tokens(tokens, "revealed");
    let match_tag = helper_tag_for_tokens(tokens, "chosen");
    vec![
        EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::You,
            LibraryConsultModeAst::Reveal,
            filter,
            LibraryConsultStopRuleAst::FirstMatch,
            all_tag.clone(),
            match_tag.clone(),
        ),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(match_tag.clone(), None),
            Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            all_tag,
            Some(match_tag),
            order,
            PlayerAst::You,
        ),
    ]
}

/// Preserve both antecedents in a reveal/pump/cleanup sequence. The singular
/// “the creature” names the object that caused the trigger, while “the
/// revealed cards” names the whole collection exposed by the consult. Neither
/// should be rebound to the immediately preceding effect merely because the
/// instructions are split across sentences.
pub fn parse_consult_reveal_then_pump_triggering_creature_then_move_revealed(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // This complete looked-card rule owns its grammar and should not lose a
    // numeric/type word merely because it also happens to be a short alias of
    // the current source name.
    let first_tokens = trim_commas(sentences[sentence_idx].lexed());
    let Some(parts) = parse_consult_traversal_sentence(&first_tokens)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: LibraryConsultModeAst::Reveal,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lexed());
    let Ok(mut pump_effects) = effect_sentences::parse_effect_sentence_lexed(&second_tokens) else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PumpForEach { target, count, .. },
            ..
        }),
    ] = pump_effects.as_mut_slice()
    else {
        return Ok(None);
    };
    if !count.has_surface_hint(ironsmith_core::ValueSurfaceHint::CardsRevealedThisWay) {
        return Ok(None);
    }
    let definite_creature_subject = second_tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .take(2)
        .eq(["the", "creature"]);
    if !definite_creature_subject {
        return Ok(None);
    }
    *target = TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.key(), None);

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let Ok(mut cleanup_effects) = effect_sentences::parse_effect_sentence_lexed(&third_tokens)
    else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    target: cleanup_target,
                    zone: Zone::Graveyard,
                    target_plural_surface,
                    ..
                },
            ..
        }),
    ] = cleanup_effects.as_mut_slice()
    else {
        return Ok(None);
    };
    if !third_tokens.iter().any(|token| token.is_word("revealed"))
        || !third_tokens
            .iter()
            .any(|token| matches!(token.as_word(), Some("card" | "cards")))
    {
        return Ok(None);
    }
    *cleanup_target = TargetAst::Tagged(parts.all_tag.clone(), None);
    *target_plural_surface = true;

    let mut effects = parts.effects;
    effects.append(&mut pump_effects);
    effects.append(&mut cleanup_effects);
    Ok(Some(effects))
}

pub fn parse_choose_two_targets_counter_first_if_power_then_fight(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let Some(mut effects) = parse_choose_target_prelude_sentence(&first_tokens)? else {
        return Ok(None);
    };
    if effects.len() != 2 {
        return Ok(None);
    }

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let Some(shape) = triple_grammar::parse_counter_then_fight_shape(&second_tokens, &third_tokens)
    else {
        return Ok(None);
    };
    let required_power = shape.required_power;

    let first_tag = crate::tag::CompilerReferenceTag::Targeted0.key();
    let second_tag = crate::tag::CompilerReferenceTag::Targeted1.key();
    let mut power_filter = ObjectFilter::default();
    power_filter.power = Some(crate::filter::Comparison::GreaterThanOrEqual(
        required_power as i32,
    ));

    effects.push(EffectAst::Conditional {
        predicate: PredicateAst::TaggedMatches(first_tag.clone(), power_filter),
        if_true: vec![EffectAst::subject_verb_put_counters(
            CounterType::PlusOnePlusOne,
            Value::Fixed(1),
            TargetAst::Tagged(first_tag.clone(), None),
            None,
            false,
        )],
        if_false: Vec::new(),
    });
    effects.push(EffectAst::subject_verb_fight(
        TargetAst::Tagged(first_tag, None),
        TargetAst::Tagged(second_tag, None),
    ));

    Ok(Some(effects))
}

pub fn parse_reveal_top_opponent_chooses_one_then_move_and_followup(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((mut effects, chosen_tag, PlayerAst::TargetOpponent, None)) =
        super::reference_linked_programs::parse_reveal_top_and_choose_one_of_revealed(
            sentences[sentence_idx].lowered(),
            sentences[sentence_idx + 1].lowered(),
        )?
    else {
        return Ok(None);
    };
    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let Some(shape) = looked_grammar::parse_chosen_card_move_followup_shape(&third_tokens) else {
        return Ok(None);
    };
    let followup_tokens = trim_commas(&third_tokens[shape.followup]);
    let followups = effect_sentences::parse_effect_sentence_lexed(&followup_tokens)?;
    if followups.is_empty() {
        return Ok(None);
    }
    effects.push(
        super::reference_linked_programs::move_tagged_to_looked_destination(
            chosen_tag,
            shape.destination,
        ),
    );
    effects.extend(followups);
    Ok(Some(effects))
}

/// Preserve an opponent's selection from one exact revealed pool and tag the
/// unselected complement before either group moves. This makes both the
/// chooser and "the rest" executable rather than leaving them as pronouns for
/// generic reference resolution to guess at independently.
pub fn parse_reveal_top_opponent_chooses_then_partition(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((library_owner, count, true)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let selection_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(selection) =
        looked_grammar::parse_opponent_revealed_card_selection_shape(&selection_tokens)
    else {
        return Ok(None);
    };
    let Some(partition) =
        looked_grammar::parse_chosen_card_partition_shape(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };

    let revealed_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "revealed_pool");
    let opponent_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "choosing_opponent");
    let selected_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "revealed_choice");
    let remainder_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 2].lowered(), "revealed_remainder");

    let mut selected_filter = if let Some(range) = selection.filter {
        parse_looked_card_choice_filter(&selection_tokens[range]).ok_or_else(|| {
            CardTextError::ParseError(
                "unable to parse opponent's revealed-card selection filter".to_string(),
            )
        })?
    } else {
        ObjectFilter::default()
    };
    selected_filter.zone = Some(Zone::Library);
    selected_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: revealed_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let remainder_filter = super::reference_linked_programs::tagged_library_candidate_filter(
        &revealed_tag,
        std::slice::from_ref(&selected_tag),
    );

    Ok(Some(vec![
        EffectAst::subject_verb_reveal_top_cards(library_owner, count, revealed_tag.clone()),
        EffectAst::subject_verb_choose_player(
            PlayerAst::You,
            PlayerFilter::Opponent,
            opponent_tag,
            false,
            0,
        ),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: selected_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::That,
            tag: selected_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_tag_matching_objects(
            remainder_filter,
            vec![Zone::Library],
            remainder_tag.clone(),
        ),
        super::reference_linked_programs::move_tagged_to_looked_destination(
            selected_tag,
            partition.selected_destination,
        ),
        super::reference_linked_programs::move_tagged_to_looked_destination(
            remainder_tag,
            partition.remainder_destination,
        ),
    ]))
}

/// Joins a looked-card pool to an optional singleton placed back on top and
/// the exact complement placed on the bottom in a separate sentence.
pub fn parse_look_at_top_then_optional_one_top_then_remainder_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((PlayerAst::You, count, false)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(shape) = looked_grammar::parse_optional_looked_top_remainder_shape(
        sentences[sentence_idx + 1].lowered(),
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked_partition");
    let selected_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "partition_selected");
    let mut selected_filter = ObjectFilter::tagged(looked_tag.clone());
    selected_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(PlayerAst::You, count, looked_tag.clone()),
        EffectAst::May {
            effects: vec![
                EffectAst::ChooseTaggedObjectsInZone {
                    filter: selected_filter,
                    count: shape.count,
                    player: PlayerAst::You,
                    tag: selected_tag.clone(),
                    zone: Zone::Library,
                },
                EffectAst::ForEachTagged {
                    tag: selected_tag.clone(),
                    effects: vec![EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        Zone::Library,
                        true,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                },
            ],
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag),
            shape.remainder_order,
            PlayerAst::You,
        ),
    ]))
}

/// Keeps both sides of a same-name legality test explicit for looked cards:
/// the candidate comes from the looked library pool, while its name must
/// occur among permanents currently on the battlefield.
pub fn parse_look_at_top_may_put_same_name_as_permanent_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((PlayerAst::You, count, false)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if !triple_grammar::is_looked_same_name_permanent_battlefield_action(
        sentences[sentence_idx + 1].lowered(),
    ) {
        return Ok(None);
    }
    let Some(triple_grammar::LookedRemainderShape::LibraryBottom(order)) =
        triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let comparison_tag = helper_tag_for_tokens(
        sentences[sentence_idx + 1].lowered(),
        "same_name_permanents",
    );
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut selection_filter = ObjectFilter::default();
    selection_filter.zone = Some(Zone::Library);
    selection_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    selection_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: comparison_tag.clone(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(PlayerAst::You, count, looked_tag.clone()),
        EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::permanent(),
            vec![Zone::Battlefield],
            comparison_tag,
        ),
        EffectAst::May {
            effects: vec![
                EffectAst::ChooseTaggedObjectsInZone {
                    filter: selection_filter,
                    count: ChoiceCount::exactly(1),
                    player: PlayerAst::You,
                    tag: chosen_tag.clone(),
                    zone: Zone::Library,
                },
                EffectAst::ForEachTagged {
                    tag: chosen_tag.clone(),
                    effects: vec![EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                },
            ],
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(chosen_tag),
            order,
            PlayerAst::You,
        ),
    ]))
}

pub fn parse_choose_land_or_nonland_then_consult_to_hand_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let third = trim_commas(sentences[sentence_idx + 2].lowered());

    let Some(shape) =
        triple_grammar::parse_land_or_nonland_consult_sequence_tokens(&first, &second, &third)
    else {
        return Ok(None);
    };

    let land_filter = ObjectFilter {
        card_types: vec![CardType::Land],
        ..Default::default()
    };
    let nonland_filter = ObjectFilter {
        excluded_card_types: vec![CardType::Land],
        ..Default::default()
    };

    Ok(Some(vec![
        EffectAst::subject_verb_choose_named_option(
            PlayerAst::You,
            vec!["land".to_string(), "nonland".to_string()],
        ),
        EffectAst::Conditional {
            predicate: PredicateAst::SourceChosenOption("land".to_string()),
            if_true: chosen_kind_consult_branch_effects(
                sentences[sentence_idx + 1].lowered(),
                land_filter,
                shape.remainder_order,
            ),
            if_false: chosen_kind_consult_branch_effects(
                sentences[sentence_idx + 1].lowered(),
                nonland_filter,
                shape.remainder_order,
            ),
        },
    ]))
}

pub fn parse_mill_then_may_put_from_among_into_hand_then_if_you_dont(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Ok(first_effects) = effect_sentences::parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action: SubjectVerbActionAst::Mill { .. },
        }),
    ] = first_effects.as_slice()
    else {
        return Ok(None);
    };
    let Some((chooser, filter)) =
        super::reference_linked_programs::parse_may_put_filtered_card_from_among_into_hand(
            second,
            *player,
            Zone::Graveyard,
        )?
    else {
        return Ok(None);
    };
    let (if_not_chosen, choice_count) = if let Some(if_not_chosen) =
        parse_if_you_dont_sentence(sentences[sentence_idx + 2].lowered())?
    {
        (if_not_chosen, ChoiceCount::up_to(1))
    } else if let Some(if_not_chosen) =
        parse_if_you_cant_sentence(sentences[sentence_idx + 2].lowered())?
    {
        (if_not_chosen, ChoiceCount::exactly(1))
    } else {
        return Ok(None);
    };

    super::reference_linked_programs::parse_mill_then_may_put_from_among_into_hand_with_if_not_chosen(
        sentences,
        sentence_idx,
        *player,
        chooser,
        filter,
        if_not_chosen,
        choice_count,
    )
}

fn flatten_sequence_effects(effects: &[EffectAst]) -> Vec<EffectAst> {
    let mut flattened = Vec::new();
    for effect in effects {
        match effect {
            EffectAst::Sequence { effects } => flattened.extend(flatten_sequence_effects(effects)),
            _ => flattened.push(effect.clone()),
        }
    }
    flattened
}

fn is_payment_effect(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PayMana { .. }
                | SubjectVerbActionAst::PayEnergy { .. }
                | SubjectVerbActionAst::PayAnyEnergy { .. }
                | SubjectVerbActionAst::PayAnyLife { .. }
                | SubjectVerbActionAst::PayLife { .. }
                | SubjectVerbActionAst::LoseLife { .. },
            ..
        })
    )
}

fn parse_optional_payment_sentence(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(effects) = effect_sentences::parse_effect_sentence_lexed(tokens) else {
        return Ok(None);
    };
    let payment_effects = match effects.as_slice() {
        [EffectAst::May { effects }] => flatten_sequence_effects(effects),
        [EffectAst::MayByPlayer { player, effects }]
            if *player == default_player || *player == PlayerAst::You =>
        {
            flatten_sequence_effects(effects)
        }
        _ => return Ok(None),
    };
    if payment_effects.is_empty() || !payment_effects.iter().all(is_payment_effect) {
        return Ok(None);
    }
    Ok(Some(payment_effects))
}

pub fn parse_mill_then_optional_payment_if_you_do_put_from_among_into_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let third = sentences[sentence_idx + 2].lowered();
    let Ok(first_effects) = effect_sentences::parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action: SubjectVerbActionAst::Mill { .. },
        }),
    ] = first_effects.as_slice()
    else {
        return Ok(None);
    };

    let Some(payment_effects) = parse_optional_payment_sentence(second, *player)? else {
        return Ok(None);
    };

    let Some(followup) = sentence_markers::parse_conditional_followup_tokens(third) else {
        return Ok(None);
    };
    if followup.actor != ConditionalFollowupActor::You {
        return Ok(None);
    }
    let third = trim_commas(followup.tail_tokens);
    let Some((chooser, filter)) =
        super::reference_linked_programs::parse_may_put_filtered_card_from_among_into_hand(
            &third,
            *player,
            Zone::Graveyard,
        )?
    else {
        return Ok(None);
    };

    let chosen_tag = helper_tag_for_tokens(&third, "chosen");
    let followup = compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
        chooser,
        filter,
        TagKey::from(crate::cards::builders::IT_TAG),
        chosen_tag,
        Zone::Graveyard,
        false,
        Vec::new(),
        ChoiceCount::exactly(1),
    );

    let mut effects = first_effects;
    effects.push(EffectAst::May {
        effects: payment_effects,
    });
    effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: followup,
    });
    Ok(Some(effects))
}

pub fn parse_each_player_mill_then_exile_milled_creatures_then_create_power_token(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn is_mill_effect(effect: &EffectAst) -> bool {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Mill { .. },
                ..
            })
        )
    }

    fn rewrite_total_power_value(value: &mut Value, tag: &TagKey) {
        match value {
            Value::TotalPower(filter) => {
                *filter = ObjectFilter::tagged(tag.clone()).in_zone(Zone::Exile);
            }
            Value::SurfaceHinted { value, .. } => rewrite_total_power_value(value, tag),
            _ => {}
        }
    }

    fn rewrite_total_power_effect(effect: &mut EffectAst, tag: &TagKey) {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::SetBasePowerToughness {
                        power, toughness, ..
                    },
                ..
            }) => {
                rewrite_total_power_value(power, tag);
                rewrite_total_power_value(toughness, tag);
            }
            EffectAst::Sequence { effects }
            | EffectAst::May { effects }
            | EffectAst::MayByPlayer { effects, .. }
            | EffectAst::ForEachPlayer { effects }
            | EffectAst::ForEachOpponent { effects }
            | EffectAst::ForEachTagged { effects, .. }
            | EffectAst::ForEachTaggedWithControllerAtLastBlockedBy { effects, .. }
            | EffectAst::ForEachObject { effects, .. } => {
                for effect in effects {
                    rewrite_total_power_effect(effect, tag);
                }
            }
            _ => {}
        }
    }

    let first = sentences[sentence_idx].lowered();
    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let third = sentences[sentence_idx + 2].lowered();

    let Ok(first_effects) = effect_sentences::parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let first_is_mill = match first_effects.as_slice() {
        [effect] if is_mill_effect(effect) => true,
        [EffectAst::ForEachPlayer { effects }] if matches!(effects.as_slice(), [effect] if is_mill_effect(effect)) => {
            true
        }
        _ => false,
    };
    if !first_is_mill {
        return Ok(None);
    }

    if !triple_grammar::is_milled_creature_exile_shape(&second) {
        return Ok(None);
    }

    let milled_tag = helper_tag_for_tokens(first, "milled");
    let exiled_tag = helper_tag_for_tokens(&second, "exiled");
    let mut milled_creature_filter =
        ObjectFilter::tagged(milled_tag.clone()).in_zone(Zone::Graveyard);
    milled_creature_filter.card_types.push(CardType::Creature);

    let mut third_effects = effect_sentences::parse_effect_sentence_lexed(third)?;
    if third_effects.is_empty() {
        return Ok(None);
    }
    for effect in &mut third_effects {
        rewrite_total_power_effect(effect, &exiled_tag);
    }

    let mut effects = match first_effects.as_slice() {
        [effect] if is_mill_effect(effect) => vec![EffectAst::TagAffected {
            effect: Box::new(effect.clone()),
            tag: milled_tag,
        }],
        [EffectAst::ForEachPlayer { effects }] if matches!(effects.as_slice(), [effect] if is_mill_effect(effect)) =>
        {
            vec![EffectAst::ForEachPlayer {
                effects: vec![EffectAst::TagAffected {
                    effect: Box::new(effects[0].clone()),
                    tag: milled_tag,
                }],
            }]
        }
        _ => return Ok(None),
    };
    effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: milled_creature_filter,
        count: ChoiceCount::up_to(2),
        player: PlayerAst::You,
        tag: exiled_tag.clone(),
        zone: Zone::Graveyard,
    });
    effects.push(EffectAst::subject_verb_exile(
        TargetAst::Tagged(exiled_tag.clone(), None),
        false,
    ));
    effects.extend(third_effects);
    Ok(Some(effects))
}

pub fn parse_reveal_top_opponent_exiles_one_put_rest_hand_then_may_cast(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let Some((player, count, true)) = parse_top_cards_view_sentence(&first) else {
        return Ok(None);
    };
    if player != PlayerAst::You {
        return Ok(None);
    }

    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let third = trim_commas(sentences[sentence_idx + 2].lowered());
    let Some(shape) = triple_grammar::parse_opponent_exile_then_hand_shape(&second, &third) else {
        return Ok(None);
    };

    let revealed_tag = helper_tag_for_tokens(&first, "revealed");
    let opponent_tag = helper_tag_for_tokens(&second, "choosing_opponent");
    let exiled_tag = helper_tag_for_tokens(&first, "exiled");
    let mut exile_filter =
        if let Some(filter) = parse_looked_card_choice_filter(&second[shape.exile_filter]) {
            filter
        } else {
            return Ok(None);
        };
    exile_filter.zone = Some(Zone::Library);
    exile_filter =
        exile_filter.match_tagged(revealed_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);

    let rest_filter = ObjectFilter::tagged(revealed_tag.clone())
        .not_tagged(exiled_tag.clone())
        .in_zone(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_reveal_top_cards(PlayerAst::You, count, revealed_tag),
        EffectAst::subject_verb_choose_player(
            PlayerAst::You,
            PlayerFilter::Opponent,
            opponent_tag,
            false,
            0,
        ),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: exile_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::That,
            tag: exiled_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), false),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Object(rest_filter, None, None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::MayByPlayer {
            // The first opponent reference above creates the concrete player
            // choice. Both occurrences of "that opponent" refer to that
            // same chosen player; treating them as fresh Opponent filters
            // lets any opponent cast the card in multiplayer games.
            player: PlayerAst::That,
            effects: vec![EffectAst::subject_verb_cast_tagged(
                exiled_tag,
                PlayerAst::That,
                false,
                false,
                true,
                None,
            )],
        },
    ]))
}

pub fn parse_search_then_player_names_card_conditional_put_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let Some(_shape) = triple_grammar::parse_search_then_name_shape(
        &first,
        sentences[sentence_idx + 1].lowered(),
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };
    let searched_tag = crate::tag::CompilerReferenceTag::Searched.key();
    let mut search_filter = ObjectFilter::default();
    search_filter.owner = Some(PlayerFilter::DamagedPlayer);
    search_filter.zone = Some(Zone::Library);
    let search_effects = vec![EffectAst::ChooseObjectsAcrossZones {
        filter: search_filter,
        count: ChoiceCount::exactly(1),
        count_value: None,
        player: PlayerAst::You,
        tag: searched_tag.clone(),
        zones: vec![Zone::Library],
        search_mode: Some(crate::effect::SearchSelectionMode::Exact),
    }];
    let chosen_name_tag = crate::tag::CompilerReferenceTag::ChosenName.key();

    let mut creature_filter = ObjectFilter::default();
    creature_filter.card_types.push(CardType::Creature);
    let mut chosen_name_filter = ObjectFilter::default();
    chosen_name_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_name_tag.clone(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });

    let mut effects = search_effects;
    effects.push(EffectAst::subject_verb_choose_card_name(
        PlayerAst::That,
        None,
        chosen_name_tag,
    ));
    effects.push(EffectAst::Conditional {
        predicate: PredicateAst::And(
            Box::new(PredicateAst::TaggedMatches(
                searched_tag.clone(),
                creature_filter,
            )),
            Box::new(PredicateAst::Not(Box::new(PredicateAst::TaggedMatches(
                searched_tag.clone(),
                chosen_name_filter,
            )))),
        ),
        if_true: vec![EffectAst::May {
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(searched_tag.clone(), None),
                Zone::Battlefield,
                false,
                crate::cards::builders::ReturnControllerAst::You,
                false,
                None,
            )],
        }],
        if_false: Vec::new(),
    });
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::That,
        SubjectVerbActionAst::ShuffleLibrary,
    ));

    Ok(Some(effects))
}

pub fn parse_choose_name_reveal_top_matching_hand_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(choose_name) = parse_choose_card_name_clause(sentences[sentence_idx].lowered())?
    else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ChooseCardName {
            tag: chosen_tag, ..
        },
        ..
    }) = &choose_name
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(shape) = triple_grammar::parse_chosen_name_reveal_shape(
        &second_tokens,
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };
    let view_tokens = trim_commas(&second_tokens[shape.view]);
    let Some((player, count, true)) = parse_top_cards_view_sentence(&view_tokens) else {
        return Ok(None);
    };
    let looked_tag = helper_tag_for_tokens(&view_tokens, "revealed");
    let mut name_match_filter = ObjectFilter::default();
    name_match_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_tag.clone(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });

    Ok(Some(vec![
        choose_name,
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::subject_verb_reveal_tagged(looked_tag.clone()),
        EffectAst::ForEachTagged {
            tag: looked_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(TagKey::from(IT_TAG), name_match_filter),
                if_true: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Hand,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        },
    ]))
}

pub fn parse_search_two_then_put_one_hand_other_graveyard_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first_effects = effect_sentences::parse_effect_chain(&first_tokens)?;
    let (mut search_filter, count, count_value, chooser, library_player, search_mode) =
        match first_effects.as_slice() {
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::SearchLibrary {
                            filter,
                            chooser,
                            player,
                            search_mode,
                            count,
                            count_value,
                            ..
                        },
                    ..
                }),
            ] => (
                filter.clone(),
                *count,
                count_value.clone(),
                *chooser,
                *player,
                *search_mode,
            ),
            [
                EffectAst::ChooseObjectsAcrossZones {
                    filter,
                    count,
                    count_value,
                    player,
                    zones,
                    search_mode,
                    ..
                },
            ] if zones.len() == 1 && zones.first().is_some_and(|zone| *zone == Zone::Library) => (
                filter.clone(),
                *count,
                count_value.clone(),
                *player,
                *player,
                search_mode.unwrap_or(crate::effect::SearchSelectionMode::Exact),
            ),
            _ => return Ok(None),
        };
    if count.min != 2 || count.max != Some(2) || count_value.is_some() {
        return Ok(None);
    }

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    if !triple_grammar::is_search_two_disposition_then_shuffle_shape(&second_tokens, &third_tokens)
    {
        return Ok(None);
    }

    search_filter.zone = Some(Zone::Library);
    let searched_tag = helper_tag_for_tokens(&first_tokens, "searched");
    let hand_tag = helper_tag_for_tokens(&second_tokens, "hand");
    let mut hand_filter = ObjectFilter::tagged(searched_tag.clone());
    hand_filter.zone = Some(Zone::Library);
    let iterated_is_hand_card =
        ObjectFilter::default().same_stable_id_as_tagged(TagKey::from(IT_TAG));

    Ok(Some(vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter: search_filter,
            count,
            count_value,
            player: chooser,
            tag: searched_tag.clone(),
            zones: vec![Zone::Library],
            search_mode: Some(search_mode),
        },
        EffectAst::ChooseTaggedObjectsInZone {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            player: chooser,
            tag: hand_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(hand_tag.clone(), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::ForEachTagged {
            tag: searched_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(hand_tag, iterated_is_hand_card),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        },
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            library_player,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ]))
}

pub fn parse_search_face_down_exile_conditional_cast_else_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let third = sentences[sentence_idx + 2].lowered();
    let Ok(first_effects) = effect_sentences::parse_effect_chain(first) else {
        return Ok(None);
    };
    let searched_tag: TagKey = "searched_face_down".into();
    let has_face_down_search = first_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::ChooseObjectsAcrossZones { tag, .. } if *tag == searched_tag
        ) || matches!(
            effect,
            EffectAst::ChooseObjects { tag, .. } if *tag == searched_tag
        )
    }) && first_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Exile {
                        target: TargetAst::Tagged(tag, _),
                        face_down: true,
                        ..
                    },
                ..
            }) if *tag == searched_tag
        )
    });
    if !has_face_down_search {
        return Ok(None);
    }

    let Some(hand_effects) = parse_if_declined_put_match_into_hand(third, searched_tag.clone())
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let Some((operator, right)) = parse_bargained_face_down_cast_mana_value_gate(&second_tokens)?
    else {
        return Ok(None);
    };
    let combined_predicate = PredicateAst::And(
        Box::new(PredicateAst::ThisSpellPaidLabel("Bargain".into())),
        Box::new(PredicateAst::ValueComparison {
            left: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(searched_tag.clone()))),
            operator,
            right,
        }),
    );
    let mut effects = first_effects;
    effects.push(EffectAst::Conditional {
        predicate: combined_predicate,
        if_true: vec![
            EffectAst::May {
                effects: vec![EffectAst::subject_verb_cast_tagged(
                    searched_tag.clone(),
                    PlayerAst::Implicit,
                    false,
                    false,
                    true,
                    None,
                )],
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::WasDeclined,
                effects: hand_effects.clone(),
            },
        ],
        if_false: hand_effects,
    });
    Ok(Some(effects))
}

pub fn parse_exile_until_match_cast_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let third = sentences[sentence_idx + 2].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    let Some(clause) = parse_consult_cast_clause(second) else {
        return Ok(None);
    };
    if !matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost) {
        return Ok(None);
    }
    let Some(order) = parse_consult_bottom_remainder_clause(
        third,
        match parts.effects.last() {
            Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ConsultTopOfLibrary { mode, .. },
                ..
            })) => *mode,
            _ => return Ok(None),
        },
    ) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.extend(consult_cast_effects(&clause, parts.match_tag.clone())?);
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            parts.all_tag,
            None,
            order,
            parts.player,
        ),
    );
    Ok(Some(effects))
}

pub fn parse_exile_until_match_cast_else_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(sentences[sentence_idx].lowered())? else {
        return Ok(None);
    };
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Exile,
                stop_rule,
                ..
            },
        ..
    })) = parts.effects.last()
    else {
        return Ok(None);
    };
    if !consult_stop_rule_is_single_match(stop_rule) {
        return Ok(None);
    }
    let Some(clause) = parse_consult_cast_clause(sentences[sentence_idx + 1].lowered()) else {
        return Ok(None);
    };
    if !matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost) || clause.allow_land {
        return Ok(None);
    }
    let Some(hand_effects) = parse_if_declined_put_match_into_hand(
        sentences[sentence_idx + 2].lowered(),
        parts.match_tag.clone(),
    ) else {
        return Ok(None);
    };

    let cast_effects = consult_cast_effects(&clause, parts.match_tag)?;
    let mut effects = parts.effects;
    if cast_effects.len() == 1 {
        let single_effect = cast_effects.into_iter().next().ok_or_else(|| {
            CardTextError::ParseError("missing cast effect for consult follow-up".to_string())
        })?;
        let EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } = single_effect
        else {
            effects.push(single_effect);
            effects.push(EffectAst::IfResult {
                predicate: IfResultPredicate::WasDeclined,
                effects: hand_effects,
            });
            return Ok(Some(effects));
        };
        let mut gated_if_true = if_true;
        gated_if_true.push(EffectAst::IfResult {
            predicate: IfResultPredicate::WasDeclined,
            effects: hand_effects.clone(),
        });
        let mut gated_if_false = if_false;
        gated_if_false.extend(hand_effects);
        effects.push(EffectAst::Conditional {
            predicate,
            if_true: gated_if_true,
            if_false: gated_if_false,
        });
    } else {
        effects.extend(cast_effects);
        effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::WasDeclined,
            effects: hand_effects,
        });
    }
    Ok(Some(effects))
}

pub fn parse_top_cards_put_match_into_hand_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action_match) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["reveal", "put"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, player);
    let reveal_chosen = action_match.verb == "reveal";
    let action_tokens = trim_commas(action_match.tail_tokens);
    let Some(shape) = triple_grammar::parse_looked_hand_action_shape(&action_tokens, reveal_chosen)
    else {
        return Ok(None);
    };
    let mut choice_count = shape.count;
    if !matches!(action_match.actor, LeadingMayActor::Default) && choice_count.min > 0 {
        choice_count = ChoiceCount::up_to(choice_count.max.unwrap_or(choice_count.min));
    }
    let filter = if let Some(filter) =
        effect_sentences::parse_looked_card_choice_filter(&action_tokens[shape.filter])
    {
        filter
    } else {
        return Ok(None);
    };
    if triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 2].lowered())
        != Some(triple_grammar::LookedRemainderShape::Graveyard)
    {
        return Ok(None);
    }

    if choice_count == ChoiceCount::up_to(1)
        && filter.card_types.len() > 1
        && shape.filter_uses_and_or
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.static_abilities.is_empty()
        && filter.any_of.is_empty()
    {
        let looked_tag = helper_tag_for_tokens(
            sentences[sentence_idx].lowered(),
            if reveal_top { "revealed" } else { "looked" },
        );
        let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
        let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
            player,
            count,
            looked_tag.clone(),
        )];
        if reveal_top {
            effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
        }

        for card_type in &filter.card_types {
            let mut choice_filter = filter.clone();
            choice_filter.card_types = vec![*card_type];
            choice_filter.zone = Some(Zone::Library);
            choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: looked_tag.clone(),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: chosen_tag.clone(),
                    relation: TaggedOpbjectRelation::IsNotTaggedObject,
                });
            effects.push(EffectAst::ChooseTaggedObjectsInZone {
                filter: choice_filter,
                count: ChoiceCount::up_to(1),
                player: chooser,
                tag: chosen_tag.clone(),
                zone: Zone::Library,
            });
        }

        effects.push(EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        });
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutTaggedRemainderInZone {
                tag: looked_tag.clone(),
                keep_tagged: chosen_tag,
                zone: Zone::Graveyard,
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            },
        ));
        return Ok(Some(effects));
    }

    let looked_tag = helper_tag_for_tokens(
        sentences[sentence_idx].lowered(),
        if reveal_top { "revealed" } else { "looked" },
    );
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }
    effects.extend(
        compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
            chooser,
            filter,
            looked_tag,
            chosen_tag,
            Zone::Library,
            reveal_chosen,
            Vec::new(),
            choice_count,
        ),
    );
    Ok(Some(effects))
}

/// Composes the "choose from looked-at cards into hand, rest into graveyard"
/// follow-up shape from reusable primitives, mirroring the runtime effects the
/// retired `ChooseFromLookedCardsIntoHandRestIntoGraveyard` recipe lowered to.
///
/// `looked_tag` must reference the cards already looked at / milled by a prior
/// effect (the recipe read this from `ctx.last_object_tag`):
/// - For a library source, pass the explicit tag the prior look effect minted
///   so the rest-into-graveyard split can iterate that exact collection.
/// - For a graveyard source (e.g. after a mill), pass `IT_TAG` so the choose
///   filter resolves the prior milled collection via `resolve_it_tag`; the
///   rest already sits in the graveyard, so no split effect is emitted.
#[allow(clippy::too_many_arguments)]
pub fn compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
    chooser: PlayerAst,
    mut filter: ObjectFilter,
    looked_tag: TagKey,
    chosen_tag: TagKey,
    source_zone: Zone,
    reveal_chosen: bool,
    if_not_chosen: Vec<EffectAst>,
    choice_count: ChoiceCount,
) -> Vec<EffectAst> {
    // The producing action is authoritative. Generic object-filter parsing may
    // retain a battlefield default for a bare type word, but "from among
    // them" is scoped to the exact looked/revealed/milled collection.
    filter.zone = Some(source_zone);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let mut effects = vec![if source_zone == Zone::Library {
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: choice_count,
            player: chooser,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        }
    } else {
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: choice_count,
            count_value: None,
            player: chooser,
            tag: chosen_tag.clone(),
            zones: vec![source_zone],
            search_mode: None,
        }
    }];

    if reveal_chosen {
        effects.push(EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::subject_verb_reveal_tagged(chosen_tag.clone())],
        });
    }

    let move_to_hand = EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    };
    effects.push(move_to_hand);
    if !if_not_chosen.is_empty() {
        effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: if_not_chosen,
        });
    }

    if source_zone == Zone::Library {
        // Keep the source collection explicit here. Self-replacement clauses
        // such as Gather the Pack replace the chosen subset while the
        // remainder must continue to range over the original revealed set.
        // Encoding the split as a nested membership test preserves those two
        // independently-scoped tags through replacement lowering.
        let mut in_chosen_filter = ObjectFilter::default();
        in_chosen_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(crate::cards::builders::IT_TAG),
                relation: TaggedOpbjectRelation::SameStableId,
            });
        effects.push(EffectAst::ForEachTagged {
            tag: looked_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(chosen_tag, in_chosen_filter),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        });
    }

    effects
}

fn parse_any_number_revealed_this_way_choice(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, ObjectFilter)> {
    let choice_tokens = trim_commas(tokens);
    let shape = triple_grammar::parse_any_number_revealed_choice_shape(&choice_tokens)?;
    let filter_tokens = trim_commas(&choice_tokens[shape.filter]);
    let mut filter = effect_sentences::parse_looked_card_choice_filter(&filter_tokens)?;
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;
    Some((shape.count, filter))
}

fn looked_choice_filter_can_include_card_type(filter: &ObjectFilter, card_type: CardType) -> bool {
    filter.card_types.contains(&card_type)
        || filter
            .any_of
            .iter()
            .any(|branch| looked_choice_filter_can_include_card_type(branch, card_type))
}

pub fn parse_reveal_top_choose_any_revealed_land_nonland_split_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, true)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some((choice_count, mut filter)) =
        parse_any_number_revealed_this_way_choice(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    if !looked_choice_filter_can_include_card_type(&filter, CardType::Land) {
        return Ok(None);
    }
    if !triple_grammar::is_land_nonland_split_bottom_shape(sentences[sentence_idx + 2].lowered()) {
        return Ok(None);
    }

    let revealed_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "revealed");
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: revealed_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let land_filter = ObjectFilter {
        card_types: vec![CardType::Land],
        ..Default::default()
    };
    let iterated = TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None);
    Ok(Some(vec![
        EffectAst::subject_verb_reveal_top_cards(player, count, revealed_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: choice_count,
            player,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::ItMatches(land_filter),
                if_true: vec![EffectAst::subject_verb_put_onto_battlefield(
                    player,
                    iterated.clone(),
                    true,
                    ReturnControllerAst::Preserve,
                )],
                if_false: vec![EffectAst::subject_verb_put_onto_battlefield(
                    player,
                    iterated,
                    false,
                    ReturnControllerAst::Preserve,
                )],
            }],
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            revealed_tag,
            Some(chosen_tag),
            crate::cards::builders::LibraryBottomOrderAst::Random,
            player,
        ),
    ]))
}

pub fn parse_reveal_top_one_hand_gain_mana_value_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(shape) = triple_grammar::parse_reveal_one_gain_mana_value_shape(
        &first,
        &second,
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };
    let Some((player, count, true)) = parse_top_cards_view_sentence(&first[shape.view]) else {
        return Ok(None);
    };
    let Ok(mut gain_effects) = effect_sentences::parse_effect_sentence_lexed(&second) else {
        return Ok(None);
    };

    let revealed_tag = helper_tag_for_tokens(&first, "revealed");
    let chosen_tag = helper_tag_for_tokens(&first, "chosen");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainLife { amount },
            ..
        }),
    ] = gain_effects.as_mut_slice()
    else {
        return Ok(None);
    };
    *amount = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(chosen_tag.clone())));

    let mut choice_filter = ObjectFilter::tagged(revealed_tag.clone());
    choice_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_reveal_top_cards(player, count, revealed_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: choice_filter,
            count: ChoiceCount::exactly(1),
            player,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        gain_effects.remove(0),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutTaggedRemainderInZone {
                tag: revealed_tag,
                keep_tagged: chosen_tag,
                zone: Zone::Graveyard,
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            },
        ),
    ]))
}

/// Composes the "for each card type, put a card of that type from among the
/// revealed cards into your hand, rest on bottom" follow-up shape from reusable
/// primitives. This replaces the retired
/// `ChooseFromLookedCardsForEachCardType*IntoHandRestOnBottomOfLibrary` recipe
/// variants and lowers to the same runtime `Effect` tree they did.
///
/// Per card type, a `ChooseObjectsAcrossZones` (up to 1, of that type, from the
/// prior looked cards not already chosen, sharing one `chosen_tag`) is emitted;
/// when `spell_filter` is set, that choose is gated behind a value comparison
/// that the player has cast at least one matching spell of that type this turn.
/// The chosen cards then move to hand via `MoveTaggedGroupToZone` (which keeps
/// the iterated reference internal to lowering, so no bare `it` surfaces) and
/// the looked remainder goes to the bottom.
///
/// `looked_tag` must reference the cards already looked at by a prior effect.
fn compose_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom(
    player: PlayerAst,
    looked_tag: TagKey,
    chosen_tag: TagKey,
    card_types: &[CardType],
    spell_filter: Option<&ObjectFilter>,
    order: crate::cards::builders::LibraryBottomOrderAst,
) -> Vec<EffectAst> {
    let chooser_player_filter = PlayerFilter::You;
    let mut effects = Vec::new();
    for card_type in card_types {
        let mut choose_filter = ObjectFilter::default();
        choose_filter.zone = Some(Zone::Library);
        choose_filter.card_types.push(*card_type);
        choose_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: looked_tag.clone(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        choose_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: chosen_tag.clone(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });

        let choose = EffectAst::ChooseObjectsAcrossZones {
            filter: choose_filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player,
            tag: chosen_tag.clone(),
            zones: vec![Zone::Library],
            search_mode: None,
        };

        if let Some(spell_filter) = spell_filter {
            let mut typed_spell_filter = (*spell_filter).clone();
            if !typed_spell_filter.card_types.contains(card_type) {
                typed_spell_filter.card_types.push(*card_type);
            }
            effects.push(EffectAst::Conditional {
                predicate: PredicateAst::ValueComparison {
                    left: Value::SpellsCastThisTurnMatching {
                        player: chooser_player_filter.clone(),
                        filter: typed_spell_filter,
                        exclude_source: false,
                    },
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                },
                if_true: vec![choose],
                if_false: Vec::new(),
            });
        } else {
            effects.push(choose);
        }
    }

    effects.push(EffectAst::MoveTaggedGroupToZone {
        tag: chosen_tag.clone(),
        zone: Zone::Hand,
    });
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(chosen_tag),
            order,
            player,
        ),
    );

    effects
}

/// Composes a two-stage selection from one looked-at set: first up to one card
/// goes to hand, then any number of the remaining matching cards go to a public
/// zone, and everything not moved by either stage goes to the graveyard. Both
/// moves share a typed affected-object tag so the remainder excludes the union
/// even though the first subset has already left the source zone.
pub fn parse_top_cards_one_hand_then_matching_to_zone_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(hand_action) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["put"], false)
    else {
        return Ok(None);
    };
    if !looked_grammar::is_one_looked_card_into_hand_shape(hand_action.tail_tokens) {
        return Ok(None);
    }
    let chooser = effect_sentences::leading_may_actor_to_player(hand_action.actor, player);

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let third_action_tokens = strip_leading_token_words_any(&third_tokens, &["then", "and"]);
    let Some(matching_action) =
        sentence_markers::parse_leading_may_action_tokens(third_action_tokens, &["put"], true)
    else {
        return Ok(None);
    };
    let matching_chooser =
        effect_sentences::leading_may_actor_to_player(matching_action.actor, player);
    if matching_chooser != chooser
        || triple_grammar::parse_looked_remainder_shape(third_action_tokens)
            != Some(triple_grammar::LookedRemainderShape::Graveyard)
    {
        return Ok(None);
    }
    let Some((
        choice_count,
        mut matching_filter,
        aggregate_constraint,
        destination,
        controller,
        tapped,
        attacking,
        attack_target_player,
        all_matching,
    )) = parse_counted_from_looked_cards_action(matching_action.tail_tokens)
    else {
        return Ok(None);
    };
    if choice_count != ChoiceCount::any_number()
        || aggregate_constraint.is_some()
        || all_matching
        || !matches!(destination, Zone::Hand | Zone::Battlefield)
    {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(
        sentences[sentence_idx].lowered(),
        if reveal_top { "revealed" } else { "looked" },
    );
    let hand_tag = helper_tag_for_tokens(&second_tokens, "chosen_hand");
    let matching_tag = helper_tag_for_tokens(&third_tokens, "chosen_matching");
    let kept_tag = helper_tag_for_tokens(&third_tokens, "kept");

    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);
    matching_filter.zone = Some(Zone::Library);
    matching_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    matching_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: hand_tag.clone(),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }
    effects.extend([
        EffectAst::ChooseTaggedObjectsInZone {
            filter: hand_filter,
            count: ChoiceCount::up_to(1),
            player: chooser,
            tag: hand_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(hand_tag, None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )),
            tag: kept_tag.clone(),
        },
        EffectAst::ChooseTaggedObjectsInZone {
            filter: matching_filter,
            count: choice_count,
            player: matching_chooser,
            tag: matching_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_move_to_zone_with_attack_target(
                TargetAst::Tagged(matching_tag, None),
                destination,
                false,
                controller,
                tapped,
                attacking,
                attack_target_player,
                false,
                None,
            )),
            tag: kept_tag.clone(),
        },
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutTaggedRemainderInZone {
                tag: looked_tag,
                keep_tagged: kept_tag,
                zone: Zone::Graveyard,
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            },
        ),
    ]);
    Ok(Some(effects))
}

fn filter_mentions_card_type(filter: &ObjectFilter, card_type: CardType) -> bool {
    filter.card_types.contains(&card_type)
        || filter
            .any_of
            .iter()
            .any(|branch| filter_mentions_card_type(branch, card_type))
}

fn filter_only_mentions_creature_or_land_types(filter: &ObjectFilter) -> bool {
    filter
        .card_types
        .iter()
        .all(|card_type| matches!(card_type, CardType::Creature | CardType::Land))
        && filter.subtypes.is_empty()
        && filter
            .any_of
            .iter()
            .all(filter_only_mentions_creature_or_land_types)
}

/// Composes a selected looked-at subset that is revealed, removes the
/// unselected remainder, then sends selected lands and creatures to their
/// respective destinations. The land branch runs first, matching the ordered
/// zone-change semantics for cards that have both types.
pub fn parse_top_cards_reveal_selection_rest_bottom_then_land_creature_split(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, false)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(reveal_action) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["reveal"], false)
    else {
        return Ok(None);
    };
    let Some(shape) =
        triple_grammar::parse_looked_reveal_selection_shape(reveal_action.tail_tokens)
    else {
        return Ok(None);
    };
    if !triple_grammar::is_revealed_land_creature_split_shape(sentences[sentence_idx + 2].lowered())
    {
        return Ok(None);
    }

    let filter_tokens = trim_commas(&reveal_action.tail_tokens[shape.filter]);
    let Some(mut selection_filter) = parse_looked_card_choice_filter(&filter_tokens) else {
        return Ok(None);
    };
    if !filter_mentions_card_type(&selection_filter, CardType::Creature)
        || !filter_mentions_card_type(&selection_filter, CardType::Land)
        || !filter_only_mentions_creature_or_land_types(&selection_filter)
    {
        return Ok(None);
    }
    effect_sentences::normalize_search_library_filter(&mut selection_filter);

    let chooser = effect_sentences::leading_may_actor_to_player(reveal_action.actor, player);
    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let selected_tag = helper_tag_for_tokens(&second_tokens, "revealed_selection");
    selection_filter.zone = Some(Zone::Library);
    selection_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut land_filter = ObjectFilter::default();
    land_filter.card_types.push(CardType::Land);
    let iterated = TargetAst::Tagged(TagKey::from(IT_TAG), None);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: selection_filter,
            count: shape.count,
            player: chooser,
            tag: selected_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_reveal_tagged(selected_tag.clone()),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag.clone()),
            shape.remainder_order,
            player,
        ),
        EffectAst::ForEachTagged {
            tag: selected_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(TagKey::from(IT_TAG), land_filter),
                if_true: vec![EffectAst::subject_verb_move_to_zone(
                    iterated.clone(),
                    Zone::Battlefield,
                    false,
                    ReturnControllerAst::Preserve,
                    true,
                    None,
                )],
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    iterated,
                    Zone::Hand,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        },
    ]))
}

pub fn parse_counted_from_looked_cards_action(
    tokens: &[OwnedLexToken],
) -> Option<(
    ChoiceCount,
    ObjectFilter,
    Option<crate::effect::ChoiceAggregateConstraint>,
    Zone,
    ReturnControllerAst,
    bool,
    bool,
    Option<PlayerAst>,
    bool,
)> {
    let action_tokens = trim_commas(tokens);
    let shape = triple_grammar::parse_looked_move_action_shape(&action_tokens)?;
    let choice_filter_tokens = trim_commas(&action_tokens[shape.filter]);
    let mut filter = effect_sentences::parse_looked_card_choice_filter(&choice_filter_tokens)?;
    let aggregate_constraint =
        lift_total_mana_value_choice_constraint(&choice_filter_tokens, &mut filter);
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let (zone, controller, tapped, attacking, attack_target_player) = match shape.destination {
        triple_grammar::LookedMoveDestinationShape::Hand => (
            Zone::Hand,
            ReturnControllerAst::Preserve,
            false,
            false,
            None,
        ),
        triple_grammar::LookedMoveDestinationShape::Battlefield {
            tapped,
            attacking,
            attacks_that_player,
            controller,
        } => (
            Zone::Battlefield,
            match controller {
                Some(BattlefieldControllerShape::You) => ReturnControllerAst::You,
                Some(BattlefieldControllerShape::Owner) => ReturnControllerAst::Owner,
                None => ReturnControllerAst::Preserve,
            },
            tapped,
            attacking,
            attacks_that_player.then_some(PlayerAst::Defending),
        ),
    };

    Some((
        shape.count,
        filter,
        aggregate_constraint,
        zone,
        controller,
        tapped,
        attacking,
        attack_target_player,
        shape.all_matching,
    ))
}

/// Lowers a looked-card deployment followed by a shuffle as one tagged
/// program.  Parsing the sentences independently turns the plural deployment
/// target into a generic target choice, which loses the looked-pool relation
/// and renders as a separate "choose" plus "for each" procedure.  This
/// producer keeps the choice domain tied to the look tag and iterates exactly
/// the chosen tag before shuffling the same library.
pub fn parse_look_at_top_put_matching_onto_battlefield_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let Some((player, count, reveal_top)) = parse_top_cards_view_sentence(&first_tokens) else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action_match) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["put"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, player);
    let Some((
        mut choice_count,
        filter,
        aggregate_constraint,
        zone,
        controller,
        tapped,
        attacking,
        attack_target_player,
        all_matching,
    )) = parse_counted_from_looked_cards_action(action_match.tail_tokens)
    else {
        return Ok(None);
    };
    if zone != Zone::Battlefield || all_matching {
        return Ok(None);
    }
    if action_match.actor != LeadingMayActor::Default && choice_count == ChoiceCount::exactly(1) {
        choice_count = ChoiceCount::up_to(1);
    }

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let shuffle_tokens = strip_leading_token_words_any(&third_tokens, &["then", "and"]);
    let Ok(shuffle_effects) = effect_sentences::parse_effect_sentence_lexed(shuffle_tokens) else {
        return Ok(None);
    };
    if !matches!(
        shuffle_effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ShuffleLibrary,
            ..
        })]
    ) {
        return Ok(None);
    }

    let library_owner = match player {
        PlayerAst::Target | PlayerAst::TargetOpponent => PlayerAst::That,
        player => player,
    };
    let looked_tag = helper_tag_for_tokens(
        sentences[sentence_idx].lowered(),
        if reveal_top { "revealed" } else { "looked" },
    );
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut choose_filter = filter;
    choose_filter.zone = Some(Zone::Library);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut effects = vec![if reveal_top {
        EffectAst::subject_verb_reveal_top_cards(player, count, looked_tag.clone())
    } else {
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone())
    }];
    effects.push(if let Some(constraint) = aggregate_constraint {
        EffectAst::ChooseObjectsWithAggregateConstraint {
            filter: choose_filter,
            count: choice_count,
            player: chooser,
            tag: chosen_tag.clone(),
            constraint,
        }
    } else {
        EffectAst::ChooseTaggedObjectsInZone {
            filter: choose_filter,
            count: choice_count,
            player: chooser,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        }
    });

    let mut chosen_effects = vec![EffectAst::subject_verb_move_to_zone_with_attack_target(
        TargetAst::Tagged(TagKey::from(IT_TAG), None),
        Zone::Battlefield,
        false,
        controller,
        tapped,
        attacking,
        attack_target_player,
        false,
        None,
    )];
    if let Some((amount, counter_type)) =
        triple_grammar::parse_looked_move_action_shape(action_match.tail_tokens)
            .and_then(|shape| shape.entry_counter)
    {
        chosen_effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(amount as i32),
            TargetAst::Tagged(TagKey::from(IT_TAG), None),
            None,
            false,
        ));
    }
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag,
        effects: chosen_effects,
    });
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        library_owner,
        SubjectVerbActionAst::ShuffleLibrary,
    ));

    Ok(Some(effects))
}

/// Preserve a looked-at selection as one coherent public-card procedure:
/// look, reveal a filtered counted subset, move that revealed subset to hand,
/// then shuffle.  In particular, the optional `where X is ...` clause belongs
/// to the selection's mana-value bound rather than becoming an orphan clause.
pub fn parse_look_at_top_reveal_counted_to_hand_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if reveal_top {
        return Ok(None);
    }

    let Some(shape) =
        quad_grammar::parse_may_reveal_looked_card_shape(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    if !quad_grammar::parse_put_revealed_into_hand_then_shuffle_shape(
        sentences[sentence_idx + 2].lowered(),
    ) {
        return Ok(None);
    }

    let mut filter = effect_sentences::parse_looked_card_choice_filter(shape.filter_tokens)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "unable to parse revealed looked-card selection filter".to_string(),
            )
        })?;
    if let Some(x_value) = shape.x_value {
        let Some(crate::filter::Comparison::LessThanOrEqualExpr(maximum)) =
            filter.mana_value.as_mut()
        else {
            return Ok(None);
        };
        **maximum = crate::util::replace_unbound_x_with_value(
            (**maximum).clone(),
            &x_value,
            "looked-card mana-value selection",
        )?;
    }
    effect_sentences::normalize_search_library_filter(&mut filter);

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let revealed_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "revealed");
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag),
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: shape.count,
            player,
            tag: revealed_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_reveal_tagged(revealed_tag.clone()),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(revealed_tag, None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            player,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ]))
}

pub fn parse_top_cards_put_any_matching_to_zone_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first_tokens = strip_leading_token_words_any(&first_tokens, &["then"]);
    let (view_tokens, gate_on_previous_result) =
        if let Some(followup) = sentence_markers::parse_conditional_followup_tokens(first_tokens) {
            (trim_commas(followup.tail_tokens), true)
        } else {
            (first_tokens.to_vec(), false)
        };
    let Some((player, count, reveal_top)) = parse_top_cards_view_sentence(&view_tokens) else {
        return Ok(None);
    };
    let remainder_player = match player {
        PlayerAst::Target | PlayerAst::TargetOpponent => PlayerAst::That,
        player => player,
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action_match) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["put"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, player);
    let Some((
        mut choice_count,
        filter,
        aggregate_constraint,
        zone,
        controller,
        tapped,
        attacking,
        attack_target_player,
        all_matching,
    )) = parse_counted_from_looked_cards_action(action_match.tail_tokens)
    else {
        return Ok(None);
    };
    if all_matching && action_match.actor != LeadingMayActor::Default {
        return Ok(None);
    }
    if action_match.actor != LeadingMayActor::Default && choice_count == ChoiceCount::exactly(1) {
        choice_count = ChoiceCount::up_to(1);
    }

    let remainder_tokens = sentences[sentence_idx + 2].lexed();
    let remainder_surface = triple_grammar::looked_remainder_surface(remainder_tokens);
    let Some(remainder) = triple_grammar::parse_looked_remainder_shape(remainder_tokens) else {
        return Ok(None);
    };
    let order = match remainder {
        triple_grammar::LookedRemainderShape::LibraryBottom(order) => Some(order),
        triple_grammar::LookedRemainderShape::Graveyard => None,
    };

    let looked_tag = helper_tag_for_tokens(
        sentences[sentence_idx].lexed(),
        if reveal_top { "revealed" } else { "looked" },
    );
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lexed(), "chosen");
    let mut choose_filter = filter;
    choose_filter.zone = Some(Zone::Library);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut effects = vec![if reveal_top {
        EffectAst::subject_verb_reveal_top_cards(player, count, looked_tag.clone())
    } else {
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone())
    }];
    if all_matching {
        choose_filter.zone = None;
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            choose_filter,
            vec![Zone::Library],
            chosen_tag.clone(),
        ));
    } else {
        effects.push(if let Some(constraint) = aggregate_constraint {
            EffectAst::ChooseObjectsWithAggregateConstraint {
                filter: choose_filter,
                count: choice_count,
                player: chooser,
                tag: chosen_tag.clone(),
                constraint,
            }
        } else {
            EffectAst::ChooseTaggedObjectsInZone {
                filter: choose_filter,
                count: choice_count,
                player: chooser,
                tag: chosen_tag.clone(),
                zone: Zone::Library,
            }
        });
    }
    let mut chosen_effects = vec![EffectAst::subject_verb_move_to_zone_with_attack_target(
        TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
        zone,
        false,
        controller,
        tapped,
        attacking,
        attack_target_player,
        false,
        None,
    )];
    if let Some((amount, counter_type)) =
        triple_grammar::parse_looked_move_action_shape(action_match.tail_tokens)
            .and_then(|shape| shape.entry_counter)
    {
        chosen_effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(amount as i32),
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
            None,
            false,
        ));
    }
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: chosen_effects,
    });
    if let Some(order) = order {
        effects.push(
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library_with_surface(
                looked_tag,
                Some(chosen_tag),
                order,
                remainder_player,
                remainder_surface,
            ),
        );
    } else {
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutTaggedRemainderInZone {
                tag: looked_tag,
                keep_tagged: chosen_tag,
                zone: Zone::Graveyard,
                surface: remainder_surface,
            },
        ));
    }

    if gate_on_previous_result {
        Ok(Some(vec![EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        }]))
    } else {
        Ok(Some(effects))
    }
}

fn parse_cast_from_among_looked_cards_action(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Result<Option<(PlayerAst, ObjectFilter)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let Some(action_match) =
        sentence_markers::parse_leading_may_action_tokens(&sentence_tokens, &["cast"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, default_player);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let Some(shape) = triple_grammar::parse_looked_cast_action_shape(&action_tokens) else {
        return Ok(None);
    };
    let filter_tokens = trim_commas(&action_tokens[shape.filter]);
    let mentions_spell = shape.mentions_spell;
    let mut filter =
        if let Some(filter) = effect_sentences::parse_looked_card_choice_filter(&filter_tokens) {
            filter
        } else if mentions_spell {
            ObjectFilter::default()
        } else {
            return Ok(None);
        };

    if mentions_spell && filter.card_types.is_empty() {
        filter.excluded_card_types.push(CardType::Land);
    }
    filter.zone = Some(Zone::Library);
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    if filter.mana_value.is_none()
        && let Some(bound) = shape.mana_value_limit
    {
        filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(bound as i32));
    }

    Ok(Some((chooser, filter)))
}

pub fn parse_top_cards_may_cast_match_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some((chooser, mut filter)) =
        parse_cast_from_among_looked_cards_action(sentences[sentence_idx + 1].lowered(), player)?
    else {
        return Ok(None);
    };

    let Some(triple_grammar::LookedRemainderShape::LibraryBottom(order)) =
        triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(
        sentences[sentence_idx].lowered(),
        if reveal_top { "revealed" } else { "looked" },
    );
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen_cast");
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }
    effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: ChoiceCount::up_to(1),
        player: chooser,
        tag: chosen_tag.clone(),
        zone: Zone::Library,
    });
    effects.push(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: crate::model::ast::SubjectVerbSubjectAst {
            role: SubjectVerbRoleAst::Actor,
            player: chooser,
        },
        action: SubjectVerbActionAst::CastTagged {
            tag: chosen_tag.clone(),
            player: chooser,
            allow_land: false,
            as_copy: false,
            copy_cast_reminder_surface: false,
            copy_instruction_surface: None,
            without_paying_mana_cost: true,
            additional_mana_cost: None,
            cost_reduction: None,
            mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::Normal,
        },
    }));
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(chosen_tag),
            order,
            chooser,
        ),
    );

    Ok(Some(effects))
}

/// Three-sentence counterpart to the looked-card exile/cast quad:
///
/// "Look at ... . Exile up to one <filter> card from among them and put the
/// rest on the bottom ... . You may cast the exiled card ... ."
///
/// The compound middle sentence still lowers to the same typed selection,
/// exile, and complement program as the four-sentence surface.
pub fn parse_look_at_top_exile_match_and_rest_bottom_then_cast_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, false)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(shape) = quad_grammar::parse_exile_looked_card_and_remainder_shape(
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };
    if shape.count != ChoiceCount::up_to(1) {
        return Ok(None);
    }
    let Some(mut exile_filter) = parse_looked_card_choice_filter(shape.filter_tokens) else {
        return Ok(None);
    };
    let Some(permission) = parse_cast_or_play_tagged_clause(sentences[sentence_idx + 2].lowered())?
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    // This compound sentence authored an explicit "up to one" selection,
    // unlike the otherwise equivalent four-sentence "you may exile" shape.
    // Retain that surface on the internal result role while keeping the
    // conventional `exiled` prefix used by reference resolution.
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled_up_to");
    let permission_effect = match permission {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    player: permission_player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    surface,
                    ..
                },
            ..
        }) => EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_with_optional_surface(
            exiled_tag.clone(),
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            surface,
        ),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CastTagged {
                    player: permission_player,
                    allow_land,
                    as_copy,
                    without_paying_mana_cost,
                    additional_mana_cost,
                    cost_reduction,
                    mana_spend_mode,
                    ..
                },
            ..
        }) if !as_copy => {
            EffectAst::subject_verb_cast_tagged_with_additional_cost_and_mana_spend_mode(
                exiled_tag.clone(),
                permission_player,
                allow_land,
                false,
                without_paying_mana_cost,
                additional_mana_cost,
                cost_reduction,
                mana_spend_mode,
            )
        }
        _ => return Ok(None),
    };

    exile_filter.zone = Some(Zone::Library);
    exile_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: exile_filter,
            count: shape.count,
            player,
            tag: exiled_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), false),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag),
            shape.order,
            player,
        ),
        permission_effect,
    ]))
}

/// Preserve the selected card across the three authored sentences in the
/// hidden-card permission shape:
///
/// "Look at ... . Exile one face down and put the rest ... . For as long as
/// it remains exiled, you may cast it if ... ."
///
/// The ordinary two-sentence partition parser already proves the exact
/// looked/selected/remainder relationship. This rule rebinds the final cast
/// permission (and any explicit tagged-look instruction in the equivalent
/// plural grammar) to that proven selected-card tag.
pub fn parse_look_at_top_partition_face_down_then_filtered_permission(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) =
        super::reference_linked_programs::parse_look_at_top_then_partition_selected_and_remainder(
            sentences,
            sentence_idx,
        )?
    else {
        return Ok(None);
    };
    let [look_effect, choice_effect, exile_effect, remainder_effect] = effects.as_slice() else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards {
            tag: looked_tag, ..
        },
        ..
    }) = look_effect
    else {
        return Ok(None);
    };
    let (selected_tag, count, selected_filter, chooser) = match choice_effect {
        EffectAst::ChooseTaggedObjectsInZone {
            tag,
            count,
            filter,
            player,
            zone: Zone::Library,
        }
        | EffectAst::ChooseObjects {
            tag,
            count,
            count_value: None,
            filter,
            player,
        } => (tag, count, filter, player),
        _ => return Ok(None),
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Exile {
                target: TargetAst::Tagged(exile_tag, _),
                face_down: true,
                ..
            },
        ..
    }) = exile_effect
    else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag: remainder_tag,
                keep_tagged: Some(kept_tag),
                order: crate::cards::builders::LibraryBottomOrderAst::ChooserChooses,
                ..
            },
        ..
    }) = remainder_effect
    else {
        return Ok(None);
    };
    let expected_selected_filter = ObjectFilter::tagged(looked_tag.clone()).in_zone(Zone::Library);
    if !count.is_single()
        || chooser != &PlayerAst::You
        || selected_filter != &expected_selected_filter
        || exile_tag != selected_tag
        || remainder_tag != looked_tag
        || kept_tag != selected_tag
    {
        return Ok(None);
    }
    let selected_tag = selected_tag.clone();

    let permission_tokens = sentences[sentence_idx + 2].lexed();
    let permission_words = crate::lexer::parser_token_word_refs(permission_tokens);
    if crate::grammar::primitives::parse_word_sequence_prefix(
        &permission_words,
        &["until", "end", "of", "turn"],
    )
    .is_some()
    {
        return Ok(None);
    }
    if crate::word_primitives::parse_any_sequence_complete(
        &permission_words,
        &[
            &[
                "for", "as", "long", "as", "it", "remains", "exiled", "you", "may", "cast", "it",
                "if", "its", "a", "creature", "spell",
            ],
            &[
                "for", "as", "long", "as", "it", "remains", "exiled", "you", "may", "cast", "it",
                "if", "it", "s", "a", "creature", "spell",
            ],
            &[
                "for", "as", "long", "as", "it", "remains", "exiled", "you", "may", "cast", "it",
                "if", "it", "is", "a", "creature", "spell",
            ],
        ],
    ) {
        effects.push(
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                selected_tag,
                PlayerAst::You,
                false,
                false,
                false,
                Some(ObjectFilter::creature()),
            ),
        );
        return Ok(Some(effects));
    }

    let Some(permission) = parse_cast_or_play_tagged_clause(permission_tokens)? else {
        return Ok(None);
    };
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
                during_turns_counter_put_on_source: None,
                spell_cost_increase: None,
                lands_enter_tapped: false,
                ..
            },
        ..
    }) = &permission
    {
        effects.push(
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                selected_tag,
                *player,
                *allow_land,
                *without_paying_mana_cost,
                *allow_any_color_for_cast,
                filter.clone(),
            ),
        );
        return Ok(Some(effects));
    }
    let EffectAst::Sequence {
        effects: permission_effects,
    } = permission
    else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                crate::cards::builders::SubjectVerbSubjectAst {
                    player: PlayerAst::You,
                    ..
                },
            action:
                SubjectVerbActionAst::LookAtObjects {
                    filter: look_filter,
                },
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    filter,
                    during_turns_counter_put_on_source: None,
                    spell_cost_increase: None,
                    lands_enter_tapped: false,
                    ..
                },
            ..
        }),
    ] = permission_effects.as_slice()
    else {
        return Ok(None);
    };
    if look_filter.zone != Some(Zone::Exile)
        || look_filter.tagged_constraints.len() != 1
        || !look_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return Ok(None);
    }

    let mut rebound_look_filter = look_filter.clone();
    rebound_look_filter.tagged_constraints[0].tag = selected_tag.clone();
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::You,
        SubjectVerbActionAst::LookAtObjects {
            filter: rebound_look_filter,
        },
    ));
    effects.push(
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            selected_tag,
            *player,
            *allow_land,
            *without_paying_mana_cost,
            *allow_any_color_for_cast,
            filter.clone(),
        ),
    );
    Ok(Some(effects))
}

fn target_ast_contains_stack_object(target: &TargetAst) -> bool {
    fn filter_contains_stack_object(filter: &ObjectFilter) -> bool {
        filter.zone == Some(Zone::Stack) || filter.any_of.iter().any(filter_contains_stack_object)
    }

    match target {
        TargetAst::Spell(_) => true,
        TargetAst::Object(filter, _, _) | TargetAst::ObjectOrPlayer(filter, _, _) => {
            filter_contains_stack_object(filter)
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_ast_contains_stack_object(inner)
        }
        _ => false,
    }
}

/// Keep an explicitly announced stack target and its copy-assignment body in
/// one reference-resolution program. If the two copy sentences are lowered as
/// a later standalone statement, their otherwise-correct `__it__` reference
/// has no declared-target import and can fall back to an unrelated object
/// domain.
pub fn parse_explicit_stack_target_then_copy_for_each_target(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())?;
    let [
        target_effect @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::TargetOnly {
                    target: declared_target,
                    explicit_declaration: true,
                },
            ..
        }),
    ] = first.as_slice()
    else {
        return Ok(None);
    };
    if !target_ast_contains_stack_object(declared_target) {
        return Ok(None);
    }

    let Some(copy_effects) =
        super::reference_linked_programs::parse_copy_for_each_target_then_each_copy_targets_different(
            sentences,
            sentence_idx + 1,
        )?
    else {
        return Ok(None);
    };
    let [
        copy_effect @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CopySpellForEachTarget {
                    target: TargetAst::Tagged(tag, _),
                    ..
                },
            ..
        }),
    ] = copy_effects.as_slice()
    else {
        return Ok(None);
    };
    if tag.as_str() != IT_TAG {
        return Ok(None);
    }

    Ok(Some(vec![target_effect.clone(), copy_effect.clone()]))
}

/// Preserve the authored optional-action surface for a looked-card selection
/// that enters with a counter. `May { exact-one choice }` is semantically
/// equivalent to an up-to-one choice, but unlike a bare up-to choice it also
/// proves Oracle's "You may put" wording for rendering.
pub fn parse_look_at_top_may_put_with_counter_then_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, false)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["put"], false)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action.actor, player);
    let Some((
        selected_count,
        mut selected_filter,
        None,
        Zone::Battlefield,
        controller,
        tapped,
        false,
        None,
        false,
    )) = parse_counted_from_looked_cards_action(action.tail_tokens)
    else {
        return Ok(None);
    };
    if !selected_count.is_single() {
        return Ok(None);
    }
    let Some((counter_amount, counter_type)) =
        triple_grammar::parse_looked_move_action_shape(action.tail_tokens)
            .and_then(|shape| shape.entry_counter)
    else {
        return Ok(None);
    };
    let Some(triple_grammar::LookedRemainderShape::LibraryBottom(order)) =
        triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let selected_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "selected");
    selected_filter.zone = Some(Zone::Library);
    selected_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let iterated = TargetAst::Tagged(TagKey::from(IT_TAG), None);
    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::May {
            effects: vec![
                EffectAst::ChooseTaggedObjectsInZone {
                    filter: selected_filter,
                    count: ChoiceCount::exactly(1),
                    player: chooser,
                    tag: selected_tag.clone(),
                    zone: Zone::Library,
                },
                EffectAst::ForEachTagged {
                    tag: selected_tag.clone(),
                    effects: vec![
                        EffectAst::subject_verb_move_to_zone_with_attack_target(
                            iterated.clone(),
                            Zone::Battlefield,
                            false,
                            controller,
                            tapped,
                            false,
                            None,
                            false,
                            None,
                        ),
                        EffectAst::subject_verb_put_counters(
                            counter_type,
                            Value::Fixed(counter_amount as i32).with_surface_hint(
                                ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter,
                            ),
                            iterated,
                            None,
                            false,
                        ),
                    ],
                },
            ],
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag),
            order,
            chooser,
        ),
    ]))
}

#[cfg(test)]
#[path = "ordered_control_flow_inline_hidden_filtered_permission_tests_2.rs"]
mod hidden_filtered_permission_tests;

#[cfg(test)]
#[path = "ordered_control_flow_inline_optional_looked_entry_counter_tests_3.rs"]
mod optional_looked_entry_counter_tests;

#[cfg(test)]
#[path = "ordered_control_flow_inline_explicit_stack_copy_assignment_tests_4.rs"]
mod explicit_stack_copy_assignment_tests;

fn parse_reveal_matching_from_looked_cards_into_hand_action(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Result<Option<(PlayerAst, ChoiceCount, ObjectFilter, bool)>, CardTextError> {
    let second_tokens = trim_commas(tokens);
    let Some(action_match) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["reveal"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, default_player);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let Some(shape) = triple_grammar::parse_looked_hand_action_shape(&action_tokens, true) else {
        return Ok(None);
    };
    let mut choice_count = shape.count;
    if !matches!(action_match.actor, LeadingMayActor::Default) && choice_count.min > 0 {
        choice_count = ChoiceCount::up_to(choice_count.max.unwrap_or(choice_count.min));
    }
    let filter_tokens = trim_commas(&action_tokens[shape.filter]);
    let mut filter =
        effect_sentences::parse_looked_card_choice_filter(&filter_tokens).ok_or_else(|| {
            CardTextError::ParseError("unable to parse revealed looked-card filter".to_string())
        })?;
    filter.zone = Some(Zone::Library);

    Ok(Some((
        chooser,
        choice_count,
        filter,
        shape.filter_uses_and_or,
    )))
}

fn looked_card_choice_filter_branches(filter: &ObjectFilter) -> Option<Vec<ObjectFilter>> {
    if filter.card_types.len() > 1
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.static_abilities.is_empty()
        && filter.any_of.is_empty()
    {
        return Some(
            filter
                .card_types
                .iter()
                .map(|card_type| {
                    let mut branch = filter.clone();
                    branch.card_types = vec![*card_type];
                    branch
                })
                .collect(),
        );
    }

    if filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.static_abilities.is_empty()
        && !filter.any_of.is_empty()
    {
        return Some(filter.any_of.clone());
    }

    None
}

#[cfg(test)]
#[path = "ordered_control_flow_inline_tests_5.rs"]
mod tests;

#[path = "ordered_control_flow_programs/ordered_control_flow_trigger_programs.rs"]
mod ordered_control_flow_trigger_programs;
pub use ordered_control_flow_trigger_programs::parse_consult_cleanup_then_typed_when_result;
#[path = "ordered_control_flow_programs/ordered_control_flow_combat_programs.rs"]
mod ordered_control_flow_combat_programs;
pub use ordered_control_flow_combat_programs::parse_destroy_historically_blocked_then_reanimate_from_historical_controller;
#[path = "ordered_control_flow_programs/ordered_control_flow_library_programs.rs"]
mod ordered_control_flow_library_programs;
use ordered_control_flow_library_programs::{
    compose_choose_from_looked_cards_onto_battlefield_and_into_hand_rest_on_bottom,
    parse_choose_from_looked_cards_for_each_filter,
};
pub use ordered_control_flow_library_programs::{
    parse_look_at_top_put_one_hand_bottom_cast_non_hand_put_all_hand,
    parse_look_at_top_reveal_match_put_rest_bottom,
    parse_look_at_top_reveal_match_put_top_rest_bottom,
    parse_look_at_top_split_hand_bottom_exile_then_play_exiled,
    parse_prefix_then_consult_match_move_and_bottom_remainder,
    parse_top_cards_choose_for_each_filter_one_battlefield_others_hand_rest_graveyard,
    parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom,
    parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom,
    parse_top_cards_put_match_onto_battlefield_and_match_into_hand_rest_bottom,
    parse_top_cards_reveal_any_matching_to_hand_rest_bottom,
};
#[path = "ordered_control_flow_programs/ordered_control_flow_choice_programs.rs"]
mod ordered_control_flow_choice_programs;
use ordered_control_flow_choice_programs::parse_keyword_choice_filter;
