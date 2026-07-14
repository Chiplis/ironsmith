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
use crate::object::CounterType;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::runtime_backend::front_end::grammar::sentence_markers::{
    self, ConditionalFollowupActor, LeadingMayActor,
};
use crate::runtime_backend::front_end::lexer::OwnedLexToken;
use crate::runtime_backend::grammar::effects::{
    control_copy_attach_shapes::BattlefieldControllerShape, looked_card_shapes as looked_grammar,
    sequence_quad_shapes as quad_grammar, triple_sequence_shapes as triple_grammar,
};
use crate::runtime_backend::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::runtime_backend::util::{
    helper_tag_for_tokens, strip_leading_token_words_any, trim_commas,
};
use crate::target::ChooseSpec;
use crate::target::{PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::zone::Zone;

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

pub(crate) fn parse_choose_two_targets_counter_first_if_power_then_fight(
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

    let first_tag = TagKey::from("targeted_0");
    let second_tag = TagKey::from("targeted_1");
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

pub(crate) fn parse_reveal_top_opponent_chooses_one_then_move_and_followup(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((mut effects, chosen_tag, PlayerAst::TargetOpponent, None)) =
        super::pairs::parse_reveal_top_and_choose_one_of_revealed(
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
    effects.push(super::pairs::move_tagged_to_looked_destination(
        chosen_tag,
        shape.destination,
    ));
    effects.extend(followups);
    Ok(Some(effects))
}

pub(crate) fn parse_choose_land_or_nonland_then_consult_to_hand_bottom(
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

pub(crate) fn parse_mill_then_may_put_from_among_into_hand_then_if_you_dont(
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
    let Some((chooser, filter)) = super::pairs::parse_may_put_filtered_card_from_among_into_hand(
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

    super::pairs::parse_mill_then_may_put_from_among_into_hand_with_if_not_chosen(
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
                | SubjectVerbActionAst::LoseLife { .. },
            ..
        })
    )
}

fn parse_optional_payment_sentence(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let effects = effect_sentences::parse_effect_sentence_lexed(tokens)?;
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

pub(crate) fn parse_mill_then_optional_payment_if_you_do_put_from_among_into_hand(
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
    let Some((chooser, filter)) = super::pairs::parse_may_put_filtered_card_from_among_into_hand(
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

pub(crate) fn parse_each_player_mill_then_exile_milled_creatures_then_create_power_token(
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

pub(crate) fn parse_reveal_top_opponent_exiles_one_put_rest_hand_then_may_cast(
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
        EffectAst::ChooseTaggedObjectsInZone {
            filter: exile_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::Opponent,
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
            player: PlayerAst::Opponent,
            effects: vec![EffectAst::subject_verb_cast_tagged(
                exiled_tag,
                PlayerAst::Opponent,
                false,
                false,
                true,
                None,
            )],
        },
    ]))
}

pub(crate) fn parse_search_then_player_names_card_conditional_put_then_shuffle(
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
    let searched_tag = TagKey::from("searched");
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
    let chosen_name_tag = TagKey::from("__chosen_name__");

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

pub(crate) fn parse_choose_name_reveal_top_matching_hand_rest_graveyard(
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

pub(crate) fn parse_search_two_then_put_one_hand_other_graveyard_then_shuffle(
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

pub(crate) fn parse_search_face_down_exile_conditional_cast_else_hand(
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

pub(crate) fn parse_exile_until_match_cast_rest_bottom(
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

pub(crate) fn parse_exile_until_match_cast_else_hand(
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

pub(crate) fn parse_top_cards_put_match_into_hand_rest_graveyard(
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
pub(crate) fn compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
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

pub(crate) fn parse_reveal_top_choose_any_revealed_land_nonland_split_rest_bottom(
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

pub(crate) fn parse_reveal_top_one_hand_gain_mana_value_rest_graveyard(
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
pub(crate) fn parse_top_cards_one_hand_then_matching_to_zone_rest_graveyard(
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
pub(crate) fn parse_top_cards_reveal_selection_rest_bottom_then_land_creature_split(
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

pub(crate) fn parse_counted_from_looked_cards_action(
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

/// Preserve a looked-at selection as one coherent public-card procedure:
/// look, reveal a filtered counted subset, move that revealed subset to hand,
/// then shuffle.  In particular, the optional `where X is ...` clause belongs
/// to the selection's mana-value bound rather than becoming an orphan clause.
pub(crate) fn parse_look_at_top_reveal_counted_to_hand_then_shuffle(
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
        **maximum = crate::runtime_backend::util::replace_unbound_x_with_value(
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

fn lift_total_mana_value_choice_constraint(
    tokens: &[OwnedLexToken],
    filter: &mut ObjectFilter,
) -> Option<crate::effect::ChoiceAggregateConstraint> {
    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    if !words
        .windows(3)
        .any(|window| window == ["total", "mana", "value"])
    {
        return None;
    }

    let mut maximum = match filter.mana_value.take()? {
        crate::filter::Comparison::LessThanOrEqual(maximum) => Value::Fixed(maximum),
        crate::filter::Comparison::LessThanOrEqualExpr(maximum) => *maximum,
        other => {
            filter.mana_value = Some(other);
            return None;
        }
    };

    if let Some(sacrificed_idx) = words.iter().position(|word| *word == "sacrificed") {
        let object_kind = words
            .get(sacrificed_idx + 1)
            .map(|word| word.trim_end_matches("'s"))
            .filter(|word| !word.is_empty())
            .unwrap_or("permanent");
        maximum = Value::ManaValueOf(Box::new(
            ChooseSpec::Tagged(TagKey::from("sacrifice_cost_0")).with_surface_hint(
                crate::target::ChooseSpecSurfaceHint::SourceReference(
                    crate::target::SourceReferenceSurface::ThisPermanentType(format!(
                        "the sacrificed {object_kind}"
                    )),
                ),
            ),
        ));
    }

    Some(crate::effect::ChoiceAggregateConstraint::total_mana_value_at_most(maximum))
}

pub(crate) fn parse_top_cards_put_any_matching_to_zone_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let (view_tokens, gate_on_previous_result) = if let Some(followup) =
        sentence_markers::parse_conditional_followup_tokens(&first_tokens)
    {
        (trim_commas(followup.tail_tokens), true)
    } else {
        (first_tokens, false)
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

    let Some(remainder) =
        triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };
    let order = match remainder {
        triple_grammar::LookedRemainderShape::LibraryBottom(order) => Some(order),
        triple_grammar::LookedRemainderShape::Graveyard => None,
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
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag,
                Some(chosen_tag),
                order,
                remainder_player,
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

pub(crate) fn parse_top_cards_may_cast_match_rest_bottom(
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
        subject: crate::runtime_backend::ast::SubjectVerbSubjectAst {
            role: SubjectVerbRoleAst::Actor,
            player: chooser,
        },
        action: SubjectVerbActionAst::CastTagged {
            tag: chosen_tag.clone(),
            player: chooser,
            allow_land: false,
            as_copy: false,
            without_paying_mana_cost: true,
            cost_reduction: None,
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
pub(crate) fn parse_look_at_top_exile_match_and_rest_bottom_then_cast_exiled(
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
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");
    let permission_effect = match permission {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    player: permission_player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    ..
                },
            ..
        }) => EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
            exiled_tag.clone(),
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
        ),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CastTagged {
                    player: permission_player,
                    allow_land,
                    as_copy,
                    without_paying_mana_cost,
                    cost_reduction,
                    ..
                },
            ..
        }) if !as_copy => EffectAst::subject_verb_cast_tagged(
            exiled_tag.clone(),
            permission_player,
            allow_land,
            false,
            without_paying_mana_cost,
            cost_reduction,
        ),
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

pub(crate) fn parse_top_cards_reveal_any_matching_to_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let (view_tokens, gate_on_previous_result) = if let Some(followup) =
        sentence_markers::parse_conditional_followup_tokens(&first_tokens)
    {
        (trim_commas(followup.tail_tokens), true)
    } else {
        (first_tokens, false)
    };
    let Some((player, count, reveal_top)) = parse_top_cards_view_sentence(&view_tokens) else {
        return Ok(None);
    };
    if reveal_top {
        return Ok(None);
    }
    let Some((chooser, choice_count, mut filter, filter_uses_and_or)) =
        parse_reveal_matching_from_looked_cards_into_hand_action(
            sentences[sentence_idx + 1].lowered(),
            player,
        )?
    else {
        return Ok(None);
    };
    let Some(triple_grammar::LookedRemainderShape::LibraryBottom(order)) =
        triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let revealed_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "revealed");

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];

    if choice_count == ChoiceCount::up_to(1)
        && filter_uses_and_or
        && let Some(choice_filters) = looked_card_choice_filter_branches(&filter)
    {
        for mut choice_filter in choice_filters {
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
                    tag: revealed_tag.clone(),
                    relation: TaggedOpbjectRelation::IsNotTaggedObject,
                });
            effects.push(EffectAst::ChooseTaggedObjectsInZone {
                filter: choice_filter,
                count: ChoiceCount::up_to(1),
                player: chooser,
                tag: revealed_tag.clone(),
                zone: Zone::Library,
            });
        }
    } else {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        effects.push(EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: choice_count,
            player: chooser,
            tag: revealed_tag.clone(),
            zone: Zone::Library,
        });
    }

    effects.push(EffectAst::subject_verb_reveal_tagged(revealed_tag.clone()));
    effects.push(EffectAst::ForEachTagged {
        tag: revealed_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
            Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(revealed_tag),
            order,
            chooser,
        ),
    );
    if gate_on_previous_result {
        Ok(Some(vec![EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        }]))
    } else {
        Ok(Some(effects))
    }
}

fn parse_keyword_choice_filter(segment: &[OwnedLexToken]) -> Option<ObjectFilter> {
    if segment.is_empty() {
        return None;
    }
    effect_sentences::parse_looked_card_choice_filter(segment).or_else(|| {
        let mut expanded = vec![
            OwnedLexToken::word("a".to_string(), TextSpan::synthetic()),
            OwnedLexToken::word("card".to_string(), TextSpan::synthetic()),
            OwnedLexToken::word("with".to_string(), TextSpan::synthetic()),
        ];
        expanded.extend_from_slice(segment);
        effect_sentences::parse_looked_card_choice_filter(&expanded)
    })
}

fn parse_choose_from_looked_cards_for_each_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<ObjectFilter>>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let Some(shape) = triple_grammar::parse_keyword_choice_segments_shape(&sentence_tokens) else {
        return Ok(None);
    };

    let mut filters = Vec::new();
    for segment in shape.segments {
        let Some(filter) = parse_keyword_choice_filter(&sentence_tokens[segment]) else {
            return Err(CardTextError::ParseError(
                "unable to parse looked-card keyword choice filter".to_string(),
            ));
        };
        filters.push(filter);
    }

    if filters.len() < 3 {
        return Ok(None);
    }
    Ok(Some(filters))
}

pub(crate) fn parse_top_cards_choose_for_each_filter_one_battlefield_others_hand_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(choice_filters) =
        parse_choose_from_looked_cards_for_each_filter(sentences[sentence_idx + 1].lowered())?
    else {
        return Ok(None);
    };
    if !triple_grammar::is_one_chosen_battlefield_others_hand_rest_graveyard_shape(
        sentences[sentence_idx + 2].lowered(),
    ) {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "revealed");
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let battlefield_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 2].lowered(), "battlefield");

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }

    for filter in choice_filters {
        let mut choose_filter = filter;
        choose_filter.zone = Some(Zone::Library);
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
        effects.push(EffectAst::ChooseTaggedObjectsInZone {
            filter: choose_filter,
            count: ChoiceCount::up_to(1),
            player,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        });
    }

    let mut battlefield_filter = ObjectFilter::default();
    battlefield_filter.zone = Some(Zone::Library);
    battlefield_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: battlefield_filter,
        count: ChoiceCount::up_to(1),
        player,
        tag: battlefield_tag.clone(),
        zone: Zone::Library,
    });
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(battlefield_tag.clone(), None),
        Zone::Battlefield,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                TagKey::from(crate::cards::builders::IT_TAG),
                ObjectFilter::tagged(battlefield_tag.clone()),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                Zone::Hand,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )],
        }],
    });
    effects.push(EffectAst::ForEachTagged {
        tag: looked_tag,
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                TagKey::from(crate::cards::builders::IT_TAG),
                ObjectFilter::tagged(chosen_tag),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                Zone::Graveyard,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )],
        }],
    });

    Ok(Some(effects))
}

pub(crate) fn parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if !reveal_top {
        return Ok(None);
    }

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(triple_grammar::CardTypeIterationShape::AmongCastSpells { spell_filter }) =
        triple_grammar::parse_card_type_iteration_shape(
            &second_tokens,
            sentences[sentence_idx + 2].lowered(),
        )
    else {
        return Ok(None);
    };
    let filter_prefix_tokens = trim_commas(&second_tokens[spell_filter]);
    let mut spell_filter = crate::runtime_backend::parse_spell_filter_lexed(&filter_prefix_tokens);
    spell_filter.zone = Some(Zone::Stack);
    spell_filter.has_mana_cost = true;

    let Some(order) =
        triple_grammar::parse_card_type_iteration_order(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "revealed");
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut effects = vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::subject_verb_reveal_tagged(looked_tag.clone()),
    ];
    effects.extend(
        compose_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom(
            player,
            looked_tag,
            chosen_tag,
            &[
                CardType::Artifact,
                CardType::Battle,
                CardType::Enchantment,
                CardType::Instant,
                CardType::Kindred,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Sorcery,
            ],
            Some(&spell_filter),
            order,
        ),
    );
    Ok(Some(effects))
}

pub(crate) fn parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if !reveal_top {
        return Ok(None);
    }

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    if triple_grammar::parse_card_type_iteration_shape(
        &second_tokens,
        sentences[sentence_idx + 2].lowered(),
    ) != Some(triple_grammar::CardTypeIterationShape::All)
    {
        return Ok(None);
    }
    let Some(order) =
        triple_grammar::parse_card_type_iteration_order(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "revealed");
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut effects = vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::subject_verb_reveal_tagged(looked_tag.clone()),
    ];
    effects.extend(
        compose_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom(
            player,
            looked_tag,
            chosen_tag,
            &[
                CardType::Artifact,
                CardType::Battle,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Instant,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Sorcery,
            ],
            None,
            order,
        ),
    );
    Ok(Some(effects))
}

pub(crate) fn parse_look_at_top_split_hand_bottom_exile_then_play_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if !triple_grammar::is_hand_bottom_exile_split_shape(sentences[sentence_idx + 1].lowered()) {
        return Ok(None);
    }

    let Some(permission) = parse_cast_or_play_tagged_clause(sentences[sentence_idx + 2].lowered())?
    else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                player: permission_player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                ..
            },
        ..
    }) = permission
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let hand_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "hand");
    let bottom_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "bottom");
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }

    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);
    effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: hand_filter,
        count: ChoiceCount::exactly(1),
        player,
        tag: hand_tag.clone(),
        zone: Zone::Library,
    });

    let mut bottom_filter = ObjectFilter::tagged(looked_tag.clone()).not_tagged(hand_tag.clone());
    bottom_filter.zone = Some(Zone::Library);
    effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: bottom_filter,
        count: ChoiceCount::exactly(1),
        player,
        tag: bottom_tag.clone(),
        zone: Zone::Library,
    });

    let mut exile_filter = ObjectFilter::tagged(looked_tag.clone())
        .not_tagged(hand_tag.clone())
        .not_tagged(bottom_tag.clone());
    exile_filter.zone = Some(Zone::Library);
    effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: exile_filter,
        count: ChoiceCount::exactly(1),
        player,
        tag: exiled_tag.clone(),
        zone: Zone::Library,
    });

    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(hand_tag, None),
        Zone::Hand,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(bottom_tag, None),
        Zone::Library,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::subject_verb_exile(
        TargetAst::Tagged(exiled_tag.clone(), None),
        false,
    ));
    effects.push(EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
        exiled_tag,
        permission_player,
        allow_land,
        without_paying_mana_cost,
        allow_any_color_for_cast,
    ));

    Ok(Some(effects))
}

pub(crate) fn parse_look_at_top_put_one_hand_bottom_cast_non_hand_put_all_hand(
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

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    if !triple_grammar::is_nonhand_replacement_looked_split_shape(
        &second_tokens,
        sentences[sentence_idx + 2].lowered(),
    ) {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let hand_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "hand");
    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);

    let look_effect = EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone());
    let default_effects = vec![
        look_effect.clone(),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            player,
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
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag.clone(),
            Some(hand_tag),
            crate::cards::builders::LibraryBottomOrderAst::ChooserChooses,
            player,
        ),
    ];
    let replacement_effects = vec![
        look_effect,
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(looked_tag, None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ];

    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate: PredicateAst::ThisSpellWasCastFromNonHand,
        if_true: replacement_effects,
        if_false: default_effects,
        attach_to_previous_ability: false,
    }]))
}

pub(crate) fn parse_top_cards_put_match_onto_battlefield_and_match_into_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some((chooser, battlefield_filter, tapped, hand_filter)) =
        effect_sentences::parse_may_put_filtered_looked_card_onto_battlefield_and_filtered_into_hand(
            sentences[sentence_idx + 1].lowered(),
        )?
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
    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }
    effects.extend(
        compose_choose_from_looked_cards_onto_battlefield_and_into_hand_rest_on_bottom(
            sentences[sentence_idx + 1].lowered(),
            looked_tag,
            chooser,
            battlefield_filter,
            hand_filter,
            tapped,
            order,
        ),
    );
    Ok(Some(effects))
}

/// Composes the "put a matching card onto the battlefield AND a matching card
/// into your hand, rest on bottom" follow-up shape from reusable primitives,
/// mirroring the runtime effects the retired
/// `ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary` recipe
/// lowered to:
/// - choose up to one matching looked card (`battlefield_tag`),
/// - put it onto the battlefield, tagging the put cards with a shared
///   `kept_tag` (`TagAffected`),
/// - choose up to one other matching looked card not already chosen for the
///   battlefield (`hand_tag`), move it to hand tagging it with the same
///   `kept_tag`,
/// - put the looked remainder (excluding `kept_tag`) on the bottom of the
///   library.
#[allow(clippy::too_many_arguments)]
fn compose_choose_from_looked_cards_onto_battlefield_and_into_hand_rest_on_bottom(
    choose_tokens: &[OwnedLexToken],
    looked_tag: TagKey,
    chooser: PlayerAst,
    mut battlefield_filter: ObjectFilter,
    mut hand_filter: ObjectFilter,
    tapped: bool,
    order: crate::cards::builders::LibraryBottomOrderAst,
) -> Vec<EffectAst> {
    let battlefield_tag = helper_tag_for_tokens(choose_tokens, "chosen");
    let hand_tag = helper_tag_for_tokens(choose_tokens, "chosen_hand");
    let kept_tag = helper_tag_for_tokens(choose_tokens, "kept");

    battlefield_filter.zone = Some(Zone::Library);
    battlefield_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    hand_filter.zone = Some(Zone::Library);
    hand_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    hand_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: battlefield_tag.clone(),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });

    vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter: battlefield_filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: chooser,
            tag: battlefield_tag.clone(),
            zones: vec![Zone::Library],
            search_mode: None,
        },
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_put_onto_battlefield(
                chooser,
                TargetAst::Tagged(battlefield_tag, None),
                tapped,
                ReturnControllerAst::Preserve,
            )),
            tag: kept_tag.clone(),
        },
        EffectAst::ChooseObjectsAcrossZones {
            filter: hand_filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: chooser,
            tag: hand_tag.clone(),
            zones: vec![Zone::Library],
            search_mode: None,
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
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(kept_tag),
            order,
            chooser,
        ),
    ]
}

pub(crate) fn parse_look_at_top_reveal_match_put_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [first_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    let Some((player, count)) = look_at_top_cards_parts(first_effect) else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action_match) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["reveal"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, player);
    let reveal_tokens = trim_commas(action_match.tail_tokens);
    let Some(shape) = triple_grammar::parse_looked_hand_action_shape(&reveal_tokens, true) else {
        return Ok(None);
    };
    let mut choice_count = shape.count;
    if !matches!(action_match.actor, LeadingMayActor::Default) && choice_count.min > 0 {
        choice_count = ChoiceCount::up_to(choice_count.max.unwrap_or(choice_count.min));
    }
    let mut filter = if let Some(filter) =
        effect_sentences::parse_looked_card_reveal_filter(&reveal_tokens[shape.filter])
    {
        filter
    } else {
        return Ok(None);
    };
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let Some(triple_grammar::LookedRemainderShape::LibraryBottom(order)) =
        triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut choose_filter = filter;
    choose_filter.zone = Some(Zone::Library);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: choose_filter,
        count: choice_count,
        player: chooser,
        tag: chosen_tag.clone(),
        zone: Zone::Library,
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_reveal_tagged(TagKey::from(
            crate::cards::builders::IT_TAG,
        ))],
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
            Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
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

pub(crate) fn parse_look_at_top_reveal_match_put_top_rest_bottom(
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
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["reveal"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, player);
    let reveal_tokens = trim_commas(action_match.tail_tokens);
    let Some(shape) = triple_grammar::parse_looked_top_action_shape(&reveal_tokens) else {
        return Ok(None);
    };
    let mut filter = if let Some(filter) =
        effect_sentences::parse_looked_card_reveal_filter(&reveal_tokens[shape.filter])
    {
        filter
    } else {
        return Ok(None);
    };
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let Some(triple_grammar::LookedRemainderShape::LibraryBottom(order)) =
        triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
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

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }
    effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter: choose_filter,
        count: ChoiceCount::up_to(1),
        player: chooser,
        tag: chosen_tag.clone(),
        zone: Zone::Library,
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_reveal_tagged(chosen_tag.clone())],
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
            Zone::Library,
            true,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
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

pub(crate) fn parse_prefix_then_consult_match_move_and_bottom_remainder(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
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
        super::pairs::parse_consult_match_move_and_bottom_remainder(sentences, sentence_idx + 1)?
    else {
        return Ok(None);
    };
    let mut effects = prefix_effects;
    effects.append(&mut combined);
    Ok(Some(effects))
}

/// A trailing reflexive result of a library consult must attach to the consult,
/// not to the intervening cleanup instruction. Keep the cleanup last in the
/// runtime sequence while preserving its explicit full revealed-set tag.
pub(crate) fn parse_consult_cleanup_then_typed_when_result(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(consult) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [
        consult @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary { .. },
            ..
        }),
    ] = consult.as_slice()
    else {
        return Ok(None);
    };

    let Ok(cleanup) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    let [
        cleanup @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. },
            ..
        }),
    ] = cleanup.as_slice()
    else {
        return Ok(None);
    };

    let Ok(followup) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };
    let [when_result @ EffectAst::WhenResult { .. }] = followup.as_slice() else {
        return Ok(None);
    };

    Ok(Some(vec![
        consult.clone(),
        when_result.clone(),
        cleanup.clone(),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{lex_line, split_lexed_sentences};

    #[test]
    fn composes_compound_looked_exile_remainder_and_cast_sequence() {
        let tokens = lex_line(
            "Look at that many cards from the top of your library. Exile up to one nonland card from among them and put the rest on the bottom of your library in a random order. You may cast the exiled card without paying its mana cost.",
            0,
        )
        .expect("lex");
        let split = split_lexed_sentences(&tokens);
        let sentences = split
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();
        let effects = parse_look_at_top_exile_match_and_rest_bottom_then_cast_exiled(&sentences, 0)
            .expect("parse")
            .expect("compound looked-card shape");

        assert_eq!(effects.len(), 5);
        assert!(matches!(
            &effects[0],
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::LookAtTopCards {
                    count: Value::EventValue(crate::effect::EventValueSpec::Amount),
                    ..
                },
                ..
            })
        ));
        let EffectAst::ChooseTaggedObjectsInZone { filter, count, .. } = &effects[1] else {
            panic!("expected typed looked-card choice: {:#?}", effects[1]);
        };
        assert_eq!(*count, ChoiceCount::up_to(1));
        assert!(filter.excluded_card_types.contains(&CardType::Land));
        assert!(matches!(
            &effects[3],
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. },
                ..
            })
        ));
        assert!(matches!(
            &effects[4],
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CastTagged {
                    without_paying_mana_cost: true,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn consult_cleanup_reflexive_keeps_variable_damage_and_full_set_cleanup() {
        let tokens = lex_line(
            "Reveal cards from the top of your library until you reveal a nonland card. Put the revealed cards on the bottom of your library in a random order. When you reveal a nonland card this way, this deals damage equal to that card's mana value to any target.",
            0,
        )
        .expect("lex");
        let split = split_lexed_sentences(&tokens);
        let sentences = split
            .iter()
            .map(|sentence| SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();

        let effects = parse_consult_cleanup_then_typed_when_result(&sentences, 0)
            .expect("parse")
            .expect("consult/cleanup/reflexive shape");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::ConsultTopOfLibrary {
                        all_tag, match_tag, ..
                    },
                ..
            }),
            EffectAst::WhenResult {
                effects: reflexive, ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        tag, keep_tagged, ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected consult, reflexive result, and cleanup: {effects:#?}");
        };
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::DealDamage { amount, target, .. },
                ..
            }),
        ] = reflexive.as_slice()
        else {
            panic!("expected variable reflexive damage: {reflexive:#?}");
        };

        assert!(matches!(amount.unhinted(), Value::ManaValueOf(_)));
        assert!(matches!(target, TargetAst::AnyTarget(_)));
        assert_ne!(all_tag, match_tag);
        assert_eq!(tag.as_str(), "__last_revealed__");
        assert!(keep_tagged.is_none());
    }
}
