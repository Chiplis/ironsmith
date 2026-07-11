use super::super::super::dispatch_entry::{
    ConsultSentenceParts, consult_cast_effects, consult_stop_rule_is_single_match,
    leading_may_actor_to_player, parse_consult_cast_clause, parse_consult_traversal_sentence,
    parse_looked_card_choice_filter, parse_looked_card_reveal_filter,
    parse_top_cards_view_sentence, parse_top_of_your_library_count,
};
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, GrantedAbilityAst, IT_TAG, IfResultPredicate,
    LibraryBottomOrderAst, ObjectFilter, OwnedLexToken, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey, TargetAst, TextSpan,
    ZoneReplacementDurationAst,
};
use crate::effect::Value;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::runtime_backend::front_end::grammar::sentence_markers::{
    self, ConditionalFollowupActor, LeadingMayActor,
};
use crate::runtime_backend::grammar::effects::{
    self as effect_grammar, parse_reciprocal_creature_control_sequence_tokens,
};
use crate::runtime_backend::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed;
use crate::runtime_backend::object_filters::parse_object_filter_lexed;
use crate::runtime_backend::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::runtime_backend::util::{helper_tag_for_tokens, parse_subject};
use crate::runtime_backend::util::{parse_choice_count_token_prefix_consumed, trim_commas};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::zone::Zone;

fn look_at_top_cards_parts(effect: &EffectAst) -> Option<(PlayerAst, Value)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::LookAtTopCards { count, .. },
    }) = effect
    else {
        return None;
    };
    Some((*player, count.clone()))
}

fn top_cards_parts_with_reveal(effect: &EffectAst) -> Option<(PlayerAst, Value, bool)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::LookAtTopCards { count, reveal, .. },
    }) = effect
    else {
        return None;
    };
    Some((*player, count.clone(), *reveal))
}

fn tagged_library_candidate_filter(candidate: &TagKey, excluded: &[TagKey]) -> ObjectFilter {
    let mut filter = ObjectFilter::tagged(candidate.clone()).in_zone(Zone::Library);
    for tag in excluded {
        filter = filter.not_tagged(tag.clone());
    }
    filter
}

pub(super) fn move_tagged_to_looked_destination(
    tag: TagKey,
    destination: effect_grammar::LookedCardDestinationShape,
) -> EffectAst {
    let (zone, to_top) = match destination {
        effect_grammar::LookedCardDestinationShape::Hand => (Zone::Hand, false),
        effect_grammar::LookedCardDestinationShape::Graveyard => (Zone::Graveyard, false),
        effect_grammar::LookedCardDestinationShape::LibraryTop => (Zone::Library, true),
        effect_grammar::LookedCardDestinationShape::LibraryBottom => (Zone::Library, false),
    };
    EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(tag, None),
        zone,
        to_top,
        ReturnControllerAst::Preserve,
        false,
        None,
    )
}

fn compose_distinct_three_way_looked_disposition(
    first_tokens: &[OwnedLexToken],
    second_tokens: &[OwnedLexToken],
    player: PlayerAst,
    count: Value,
    destinations: [effect_grammar::LookedCardDestinationShape; 3],
) -> Vec<EffectAst> {
    let candidate_tag = helper_tag_for_tokens(first_tokens, "looked_candidates");
    let chosen_tags = [
        helper_tag_for_tokens(second_tokens, "looked_choice_0"),
        helper_tag_for_tokens(second_tokens, "looked_choice_1"),
        helper_tag_for_tokens(second_tokens, "looked_choice_2"),
    ];
    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        candidate_tag.clone(),
    )];
    for (index, tag) in chosen_tags.iter().enumerate() {
        effects.push(EffectAst::ChooseTaggedObjectsInZone {
            filter: tagged_library_candidate_filter(&candidate_tag, &chosen_tags[..index]),
            count: ChoiceCount::exactly(1),
            player,
            tag: tag.clone(),
            zone: Zone::Library,
        });
    }
    for (tag, destination) in chosen_tags.into_iter().zip(destinations) {
        effects.push(move_tagged_to_looked_destination(tag, destination));
    }
    effects
}

pub(super) fn parse_reveal_top_and_choose_one_of_revealed(
    first_tokens: &[OwnedLexToken],
    second_tokens: &[OwnedLexToken],
) -> Result<
    Option<(
        Vec<EffectAst>,
        TagKey,
        PlayerAst,
        Option<effect_grammar::LookedCardDestinationShape>,
    )>,
    CardTextError,
> {
    let first_effects = effect_sentences::parse_effect_sentence_lexed(first_tokens)?;
    let [first_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    let Some((library_owner, count, true)) = top_cards_parts_with_reveal(first_effect) else {
        return Ok(None);
    };
    let Some(shape) = effect_grammar::parse_revealed_card_choice_shape(second_tokens) else {
        return Ok(None);
    };
    let chooser = match shape.chooser {
        effect_grammar::RevealedCardChooserShape::You => PlayerAst::You,
        effect_grammar::RevealedCardChooserShape::TargetOpponent => PlayerAst::TargetOpponent,
    };
    let candidate_tag = helper_tag_for_tokens(first_tokens, "revealed_candidates");
    let chosen_tag = helper_tag_for_tokens(second_tokens, "revealed_choice");
    let effects = vec![
        EffectAst::subject_verb_reveal_top_cards(library_owner, count, candidate_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: tagged_library_candidate_filter(&candidate_tag, &[]),
            count: ChoiceCount::exactly(1),
            player: chooser,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        },
    ];
    Ok(Some((effects, chosen_tag, chooser, shape.destination)))
}

pub(crate) fn parse_directional_adjacent_player_control(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let choice_sentence = sentences[sentence_idx].lowered();
    let gain_sentence = sentences[sentence_idx + 1].lowered();

    let Some(shape) = effect_grammar::parse_directional_adjacent_player_control_shape(
        choice_sentence,
        gain_sentence,
    ) else {
        return Ok(None);
    };
    let object_tokens = trim_commas(&choice_sentence[shape.choice_object]);
    let filter = parse_object_filter_lexed(&object_tokens, false)?;

    Ok(Some(vec![EffectAst::DirectionalAdjacentPlayerControl {
        filter,
        left_option: "left".to_string(),
        right_option: "right".to_string(),
    }]))
}

pub(crate) fn parse_reciprocal_creature_control_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = parse_reciprocal_creature_control_sequence_tokens(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
        sentences[sentence_idx + 2].lowered(),
    )?
    else {
        return Ok(None);
    };

    let your_tag = TagKey::from(TWIST_YOUR_CREATURES_TAG);
    let target_tag = TagKey::from(TWIST_OPPONENT_CREATURES_TAG);
    let your_tagged = ObjectFilter::tagged(your_tag.clone());
    let target_tagged = ObjectFilter::tagged(target_tag.clone());
    let mut both_tagged = ObjectFilter::default();
    both_tagged.any_of = vec![your_tagged.clone(), target_tagged.clone()];

    let mut effects = vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TagMatchingObjects {
                filter: shape.your_creatures,
                zones: vec![Zone::Battlefield],
                tag: your_tag,
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TagMatchingObjects {
                filter: shape.target_player_creatures,
                zones: vec![Zone::Battlefield],
                tag: target_tag,
            },
        ),
        EffectAst::subject_verb_gain_control(
            PlayerAst::Implicit,
            TargetAst::Object(target_tagged, None, None),
            shape.duration.clone(),
        ),
        EffectAst::subject_verb_gain_control(
            PlayerAst::TargetOpponent,
            TargetAst::Object(your_tagged, None, None),
            shape.duration.clone(),
        ),
    ];
    if shape.untap {
        effects.push(EffectAst::subject_verb_untap_all(both_tagged.clone()));
    }
    if shape.grant_haste {
        effects.push(EffectAst::subject_verb_grant_abilities_all(
            both_tagged,
            vec![GrantedAbilityAst::StaticAbility(StaticAbility::haste())],
            shape.duration,
        ));
    }

    Ok(Some(effects))
}

fn parse_optional_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ConsultSentenceParts, bool)>, CardTextError> {
    if let Some(parts) = parse_consult_traversal_sentence(tokens)? {
        return Ok(Some((parts, false)));
    }
    let Some(shape) = effect_grammar::parse_optional_sequence_prefix_shape(tokens) else {
        return Ok(None);
    };
    let stripped = trim_commas(&tokens[shape.tail]);
    parse_consult_traversal_sentence(&stripped).map(|parts| parts.map(|parts| (parts, true)))
}

fn strip_leading_if_you_do_sentence(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    let Some(followup) = sentence_markers::parse_conditional_followup_tokens(tokens) else {
        return (trim_commas(tokens), false);
    };
    if followup.actor != ConditionalFollowupActor::You {
        return (trim_commas(tokens), false);
    }
    (trim_commas(followup.tail_tokens), true)
}

fn wrap_optional_consult_effects(
    parts: ConsultSentenceParts,
    optional: bool,
    followups: Vec<EffectAst>,
    gate_on_result: bool,
) -> Vec<EffectAst> {
    let mut effects = Vec::new();
    if optional {
        effects.push(EffectAst::May {
            effects: parts.effects,
        });
    } else {
        effects.extend(parts.effects);
    }
    if gate_on_result || optional {
        effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: followups,
        });
    } else {
        effects.extend(followups);
    }
    effects
}

fn mark_target_set_same_controller(target: TargetAst) -> TargetAst {
    match target {
        TargetAst::Object(mut filter, target_span, it_span) => {
            filter.target_set_same_controller = true;
            TargetAst::Object(filter, target_span, it_span)
        }
        TargetAst::WithCount(inner, count) => {
            TargetAst::WithCount(Box::new(mark_target_set_same_controller(*inner)), count)
        }
        TargetAst::WithCountValue(inner, count, value) => TargetAst::WithCountValue(
            Box::new(mark_target_set_same_controller(*inner)),
            count,
            value,
        ),
        other => other,
    }
}

// These tags are stable semantic identities used by the reciprocal-control
// model. Keep their established names so compiled definitions remain stable
// across parser migrations.
const TWIST_YOUR_CREATURES_TAG: &str = "__twist_your_creatures__";
const TWIST_OPPONENT_CREATURES_TAG: &str = "__twist_opponent_creatures__";

pub(crate) fn parse_exile_face_down_pile_then_cloak(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = sentences[sentence_idx].lowered();
    let Some(shape) = effect_grammar::parse_cloak_pile_sequence_shape(
        first_tokens,
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };

    let target = effect_sentences::parse_target_phrase(shape.target_tokens)?;
    let pile_tag = helper_tag_for_tokens(first_tokens, "cloak_pile");
    let library_tag = helper_tag_for_tokens(first_tokens, "cloak_top");
    let target_exile = EffectAst::TagAffected {
        effect: Box::new(EffectAst::subject_verb_exile(target, true)),
        tag: pile_tag.clone(),
    };
    let library_exile = EffectAst::TagAffected {
        effect: Box::new(EffectAst::subject_verb_exile(
            TargetAst::Tagged(library_tag.clone(), None),
            true,
        )),
        tag: pile_tag.clone(),
    };

    Ok(Some(vec![
        target_exile,
        EffectAst::subject_verb_look_at_top_cards(
            shape.library_owner,
            shape.library_count,
            library_tag,
        ),
        library_exile,
        EffectAst::subject_verb_move_to_zone_with_attacking(
            TargetAst::Tagged(pile_tag, None),
            Zone::Battlefield,
            false,
            ReturnControllerAst::You,
            shape.enters_tapped,
            false,
            true,
            None,
        ),
    ]))
}

pub(crate) fn parse_look_at_top_then_exile_face_down_then_play_while_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = sentences[sentence_idx].lowered();
    let Some(shape) = effect_grammar::parse_look_exile_face_down_shape(first_tokens) else {
        return Ok(None);
    };

    if let effect_grammar::LookExileFaceDownShape::Counted {
        look,
        exile,
        count: exile_count,
        bottom_order,
    } = &shape
    {
        let look_tokens = trim_commas(&first_tokens[look.clone()]);
        let exile_tokens = trim_commas(&first_tokens[exile.clone()]);
        let Ok(look_effects) = effect_sentences::parse_effect_sentence_lexed(&look_tokens) else {
            return Ok(None);
        };
        let [look_effect] = look_effects.as_slice() else {
            return Ok(None);
        };
        let Some((library_owner, count)) = look_at_top_cards_parts(look_effect) else {
            return Ok(None);
        };

        let Some(permission_effect) =
            parse_cast_or_play_tagged_clause(sentences[sentence_idx + 1].lowered())?
        else {
            return Ok(None);
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    player: permission_player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    filter,
                    ..
                },
            ..
        }) = permission_effect
        else {
            return Ok(None);
        };

        let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
        let exiled_tag = helper_tag_for_tokens(&exile_tokens, "exiled");
        let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
        choice_filter.zone = Some(Zone::Library);

        return Ok(Some(vec![
            EffectAst::subject_verb_look_at_top_cards(library_owner, count, looked_tag.clone()),
            EffectAst::ChooseObjects {
                filter: choice_filter,
                count: *exile_count,
                count_value: None,
                player: PlayerAst::You,
                tag: exiled_tag.clone(),
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), true),
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag,
                Some(exiled_tag.clone()),
                *bottom_order,
                PlayerAst::You,
            ),
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                exiled_tag,
                permission_player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
            ),
        ]));
    }

    let effect_grammar::LookExileFaceDownShape::Single { look } = shape else {
        unreachable!("counted look/exile shape returned above")
    };
    let look_tokens = trim_commas(&first_tokens[look]);
    let Ok(look_effects) = effect_sentences::parse_effect_sentence_lexed(&look_tokens) else {
        return Ok(None);
    };
    let [look_effect] = look_effects.as_slice() else {
        return Ok(None);
    };
    let Some((player, count)) = look_at_top_cards_parts(look_effect) else {
        return Ok(None);
    };

    let Some(permission_effect) =
        parse_cast_or_play_tagged_clause(sentences[sentence_idx + 1].lowered())?
    else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                player: permission_player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
                ..
            },
        ..
    }) = permission_effect
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::subject_verb_exile(TargetAst::Tagged(looked_tag.clone(), None), true),
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            looked_tag,
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            filter,
        ),
    ]))
}

pub(crate) fn parse_look_at_top_then_put_one_hand_other_bottom(
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
    if let Some(shape) =
        effect_grammar::parse_three_way_looked_card_disposition_shape(&second_tokens)
    {
        if count != Value::Fixed(3)
            || shape != effect_grammar::ThreeWayLookedCardDispositionShape::HandTopBottom
        {
            return Ok(None);
        }
        return Ok(Some(compose_distinct_three_way_looked_disposition(
            sentences[sentence_idx].lowered(),
            &second_tokens,
            player,
            count,
            shape.destinations(),
        )));
    }
    if effect_grammar::parse_looked_card_disposition(&second_tokens)
        != Some(effect_grammar::LookedCardDisposition::HandAndLibraryBottom)
    {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let hand_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "hand");
    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseObjects {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player,
            tag: hand_tag.clone(),
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
            looked_tag,
            Some(hand_tag),
            LibraryBottomOrderAst::ChooserChooses,
            player,
        ),
    ]))
}

pub(crate) fn parse_look_at_top_then_put_one_hand_other_graveyard(
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
    if let Some(shape) =
        effect_grammar::parse_three_way_looked_card_disposition_shape(&second_tokens)
    {
        if count != Value::Fixed(3)
            || shape != effect_grammar::ThreeWayLookedCardDispositionShape::HandGraveyardBottom
        {
            return Ok(None);
        }
        return Ok(Some(compose_distinct_three_way_looked_disposition(
            sentences[sentence_idx].lowered(),
            &second_tokens,
            player,
            count,
            shape.destinations(),
        )));
    }
    if effect_grammar::parse_looked_card_disposition(&second_tokens)
        != Some(effect_grammar::LookedCardDisposition::HandAndGraveyard)
    {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let hand_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "hand");
    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);
    let mut in_chosen_filter = ObjectFilter::default();
    in_chosen_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::SameStableId,
        });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseObjects {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player,
            tag: hand_tag.clone(),
        },
        EffectAst::ForEachTagged {
            tag: hand_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::ForEachTagged {
            tag: looked_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(hand_tag, in_chosen_filter),
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
    ]))
}

pub(crate) fn parse_reveal_top_then_choose_revealed_and_move(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((mut effects, chosen_tag, _, Some(destination))) =
        parse_reveal_top_and_choose_one_of_revealed(
            sentences[sentence_idx].lowered(),
            sentences[sentence_idx + 1].lowered(),
        )?
    else {
        return Ok(None);
    };
    effects.push(move_tagged_to_looked_destination(chosen_tag, destination));
    Ok(Some(effects))
}

pub(crate) fn parse_choose_draw_main_or_combat_phase_then_skip_chosen_this_turn(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !effect_grammar::parse_choose_then_skip_phase_shape(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    ) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_choose_named_option(
            PlayerAst::That,
            vec![
                "draw step".to_string(),
                "main phase".to_string(),
                "combat phase".to_string(),
            ],
        ),
        EffectAst::Conditional {
            predicate: PredicateAst::SourceChosenOption("draw step".to_string()),
            if_true: vec![EffectAst::subject_verb_skip_draw_step(PlayerAst::That)],
            if_false: vec![EffectAst::Conditional {
                predicate: PredicateAst::SourceChosenOption("main phase".to_string()),
                if_true: vec![EffectAst::subject_verb_skip_main_phases_this_turn(
                    PlayerAst::That,
                )],
                if_false: vec![EffectAst::subject_verb_skip_combat_phases_this_turn(
                    PlayerAst::That,
                )],
            }],
        },
    ]))
}

pub(crate) fn parse_choose_same_controller_targets_then_sacrifice_one(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(
        parse_same_controller_targets_choose_sacrifice(sentences, sentence_idx)?
            .map(|(effects, _, _)| effects),
    )
}

pub(crate) fn parse_choose_same_controller_targets_then_sacrifice_one_return_other(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((mut effects, target_set_tag, chosen_tag)) =
        parse_same_controller_targets_choose_sacrifice(sentences, sentence_idx)?
    else {
        return Ok(None);
    };

    if !effect_grammar::is_return_other_to_owner_hand_shape(sentences[sentence_idx + 2].lowered()) {
        return Ok(None);
    }

    let mut other_filter = ObjectFilter::tagged(target_set_tag);
    other_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_tag,
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
    effects.push(EffectAst::subject_verb_return_to_hand(
        TargetAst::Object(other_filter, None, None),
        false,
    ));
    Ok(Some(effects))
}

fn parse_same_controller_targets_choose_sacrifice(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<(Vec<EffectAst>, TagKey, TagKey)>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(shape) =
        effect_grammar::parse_same_controller_sacrifice_shape(&first_tokens, &second_tokens)
    else {
        return Ok(None);
    };
    let target = mark_target_set_same_controller(effect_sentences::parse_target_phrase(
        &trim_commas(&first_tokens[shape.target]),
    )?);
    let TargetAst::WithCount(_, target_count) = &target else {
        return Ok(None);
    };
    if target_count.min != 2 || target_count.max != Some(2) || target_count.is_random() {
        return Ok(None);
    }

    let target_set_tag = helper_tag_for_tokens(&first_tokens, "target_set");
    let chosen_tag = helper_tag_for_tokens(&second_tokens, "chosen");
    Ok(Some((
        vec![
            EffectAst::subject_verb_target_only(target),
            EffectAst::SnapshotLastObjectTag {
                into: target_set_tag.clone(),
            },
            EffectAst::ChooseObjects {
                filter: ObjectFilter::tagged(target_set_tag.clone()),
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::ItsController,
                tag: chosen_tag.clone(),
            },
            EffectAst::subject_verb_sacrifice(
                PlayerAst::That,
                ObjectFilter::tagged(chosen_tag.clone()),
                1,
                None,
            ),
        ],
        target_set_tag,
        chosen_tag,
    )))
}

fn rest_action_effect(
    action: effect_grammar::RestActionShape,
    filter: ObjectFilter,
    player: PlayerAst,
) -> EffectAst {
    match action {
        effect_grammar::RestActionShape::Destroy => EffectAst::subject_verb_destroy_all(filter),
        effect_grammar::RestActionShape::Exile => EffectAst::subject_verb_exile_all(filter, false),
        effect_grammar::RestActionShape::Sacrifice => {
            EffectAst::subject_verb_sacrifice_all(player, filter)
        }
    }
}

fn append_rest_action_after_choice(
    effect: EffectAst,
    action: effect_grammar::RestActionShape,
) -> Option<Vec<EffectAst>> {
    match effect {
        EffectAst::ChooseObjects {
            filter,
            tag,
            count,
            count_value,
            player,
        } => {
            let rest_filter = filter.clone().not_tagged(tag.clone());
            Some(vec![
                EffectAst::ChooseObjects {
                    filter,
                    tag,
                    count,
                    count_value,
                    player,
                },
                rest_action_effect(action, rest_filter, player),
            ])
        }
        EffectAst::ForEachPlayer { effects } => {
            let [inner] = effects.as_slice() else {
                return None;
            };
            let EffectAst::ChooseObjects {
                filter,
                tag,
                count,
                count_value,
                player,
            } = inner.clone()
            else {
                return None;
            };
            let rest_filter = filter.clone().not_tagged(tag.clone());
            Some(vec![EffectAst::ForEachPlayer {
                effects: vec![
                    EffectAst::ChooseObjects {
                        filter,
                        tag,
                        count,
                        count_value,
                        player,
                    },
                    rest_action_effect(action, rest_filter, player),
                ],
            }])
        }
        EffectAst::ForEachOpponent { effects } => {
            let [inner] = effects.as_slice() else {
                return None;
            };
            let EffectAst::ChooseObjects {
                filter,
                tag,
                count,
                count_value,
                player,
            } = inner.clone()
            else {
                return None;
            };
            let rest_filter = filter.clone().not_tagged(tag.clone());
            Some(vec![EffectAst::ForEachOpponent {
                effects: vec![
                    EffectAst::ChooseObjects {
                        filter,
                        tag,
                        count,
                        count_value,
                        player,
                    },
                    rest_action_effect(action, rest_filter, player),
                ],
            }])
        }
        _ => None,
    }
}

pub(crate) fn parse_choose_then_affect_rest(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(action) =
        effect_grammar::parse_rest_action_shape(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [first] = first_effects.as_slice() else {
        return Ok(None);
    };
    Ok(append_rest_action_after_choice(first.clone(), action))
}

pub(crate) fn parse_may_cast_target_graveyard_spell_then_exile_replacement(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(shape) = effect_grammar::parse_graveyard_cast_replacement_shape(&first, &second)
    else {
        return Ok(None);
    };

    let tag = TagKey::from(crate::cards::builders::IT_TAG);
    let mut filter = ObjectFilter::default();
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);
    filter.card_types = vec![CardType::Instant, CardType::Sorcery];
    if shape.includes_artifact {
        filter.card_types.push(CardType::Artifact);
    }
    if let Some(limit) = shape.mana_value_limit {
        filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(limit));
    }

    let replacement_filter = ObjectFilter {
        zone: Some(Zone::Stack),
        card_types: vec![CardType::Instant, CardType::Sorcery],
        tagged_constraints: vec![TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        }],
        ..ObjectFilter::default()
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
        },
        EffectAst::May {
            effects: vec![EffectAst::subject_verb_cast_tagged(
                tag.clone(),
                PlayerAst::You,
                false,
                false,
                shape.without_paying_mana_cost,
                None,
            )],
        },
        EffectAst::subject_verb_register_future_zone_replacement(
            replacement_filter,
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Exile,
            ZoneReplacementDurationAst::OneShot,
        ),
    ]))
}

fn target_for_referenced_stack_object(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    tokens: &[OwnedLexToken],
) -> TargetAst {
    let previous = sentence_idx
        .checked_sub(1)
        .map(|idx| sentences[idx].lowered());
    match effect_grammar::parse_stack_object_reference_shape(tokens, previous) {
        effect_grammar::StackObjectReferenceShape::Source => TargetAst::Source(None),
        effect_grammar::StackObjectReferenceShape::PreviousChosen => {
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None)
        }
        effect_grammar::StackObjectReferenceShape::Triggering => {
            TargetAst::Tagged(TagKey::from("triggering"), None)
        }
    }
}

pub(crate) fn parse_tempting_offer_copy_spell_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !effect_grammar::is_tempting_offer_copy_sequence(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
        sentences[sentence_idx + 2].lowered(),
        sentences[sentence_idx + 3].lowered(),
    ) {
        return Ok(None);
    }

    let stack_spell_filter = ObjectFilter {
        zone: Some(Zone::Stack),
        card_types: vec![CardType::Instant, CardType::Sorcery],
        has_mana_cost: true,
        ..Default::default()
    };
    let target_spell = TargetAst::Tagged(TagKey::from(IT_TAG), None);
    let opponent_copy = EffectAst::subject_verb_copy_spell(
        target_spell.clone(),
        Value::Fixed(1),
        PlayerAst::That,
        true,
        Vec::new(),
    );
    let your_copy_count = Value::PendingEffectMetricOffset {
        source: ironsmith_core::EffectMetricSource::Outcome,
        metric: ironsmith_core::EffectMetric::PlayersWithPositiveCount,
        offset: 1,
    };
    let your_copy = EffectAst::subject_verb_copy_spell(
        target_spell,
        your_copy_count,
        PlayerAst::You,
        true,
        Vec::new(),
    );

    Ok(Some(vec![
        EffectAst::subject_verb_target_only(TargetAst::Object(
            stack_spell_filter,
            Some(TextSpan::synthetic()),
            None,
        )),
        EffectAst::ForEachOpponent {
            effects: vec![EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: vec![opponent_copy],
            }],
        },
        your_copy,
    ]))
}

fn parse_copy_for_each_candidate_filter(
    tokens: &[OwnedLexToken],
) -> Result<(Option<ObjectFilter>, Option<PlayerFilter>, bool), CardTextError> {
    let Some(shape) = effect_grammar::parse_copy_candidate_shape(tokens) else {
        return Ok((None, None, false));
    };
    let candidate_tokens = trim_commas(&tokens[shape.candidate]);
    if shape.kind == effect_grammar::CopyCandidateKind::PlayerOrPermanent {
        return Ok((
            Some(ObjectFilter::permanent()),
            Some(PlayerFilter::Any),
            shape.exclude_current_targets,
        ));
    }
    if shape.kind == effect_grammar::CopyCandidateKind::Player {
        return Ok((None, Some(PlayerFilter::Any), shape.exclude_current_targets));
    }

    let mut filter = parse_object_filter_lexed(&candidate_tokens, false)?;
    filter.other = false;
    filter.could_be_targeted_by = None;
    Ok((Some(filter), None, shape.exclude_current_targets))
}

fn parse_copy_for_each_target_sentence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = trim_commas(tokens);
    let Some(shape) = effect_grammar::parse_copy_for_each_shape(&tokens) else {
        return Ok(None);
    };
    let wrap_if_result = shape.wrap_if_result;
    let (target, object_filter, player_filter, player, exclude_current_targets) = match shape.layout
    {
        effect_grammar::CopyForEachLayout::CopyThenForEach {
            subject,
            target,
            candidate,
        } => {
            let player = match parse_subject(&tokens[subject]) {
                SubjectAst::Player(player) => player,
                SubjectAst::This => PlayerAst::Implicit,
            };
            let target_tokens = trim_commas(&tokens[target]);
            let candidate_tokens = trim_commas(&tokens[candidate]);
            let (object_filter, player_filter, exclude_current_targets) =
                parse_copy_for_each_candidate_filter(&candidate_tokens)?;
            (
                target_for_referenced_stack_object(sentences, sentence_idx, &target_tokens),
                object_filter,
                player_filter,
                player,
                exclude_current_targets,
            )
        }
        effect_grammar::CopyForEachLayout::ForEachThenPutCopy { target, candidate } => {
            let target_tokens = trim_commas(&tokens[target]);
            let candidate_tokens = trim_commas(&tokens[candidate]);
            let (object_filter, player_filter, exclude_current_targets) =
                parse_copy_for_each_candidate_filter(&candidate_tokens)?;
            (
                target_for_referenced_stack_object(sentences, sentence_idx, &target_tokens),
                object_filter,
                player_filter,
                PlayerAst::Implicit,
                exclude_current_targets,
            )
        }
    };
    let effect = EffectAst::subject_verb_copy_spell_for_each_target(
        target,
        object_filter,
        player_filter,
        player,
        exclude_current_targets,
        Vec::new(),
    );
    Ok(Some(if wrap_if_result {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![effect],
        }
    } else {
        effect
    }))
}

pub(crate) fn parse_copy_for_each_target_then_each_copy_targets_different(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !effect_grammar::each_copy_targets_different_shape(sentences[sentence_idx + 1].lowered()) {
        return Ok(None);
    }
    let Some(effect) = parse_copy_for_each_target_sentence(
        sentences,
        sentence_idx,
        sentences[sentence_idx].lowered(),
    )?
    else {
        return Ok(None);
    };
    Ok(Some(vec![effect]))
}

pub(crate) fn parse_for_each_tagged_copy_then_copy_targets_it(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let Some(shape) = effect_grammar::parse_tagged_copy_retarget_shape(
        &first_tokens,
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };
    let copy_target_tokens = trim_commas(&first_tokens[shape.copy_target]);
    let copy_effect = EffectAst::subject_verb_copy_spell(
        target_for_referenced_stack_object(sentences, sentence_idx, &copy_target_tokens),
        Value::Fixed(1),
        PlayerAst::You,
        false,
        Vec::new(),
    );

    let second_effects =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 1].lowered())?;
    let [
        retarget @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RetargetStackObject { .. },
            ..
        }),
    ] = second_effects.as_slice()
    else {
        return Ok(None);
    };
    let for_each = EffectAst::ForEachTagged {
        tag: TagKey::from(crate::cards::builders::IT_TAG),
        effects: vec![copy_effect, retarget.clone()],
    };

    Ok(Some(vec![if shape.wrap_if_result {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![for_each],
        }
    } else {
        for_each
    }]))
}

pub(crate) fn parse_may_put_filtered_card_from_among_into_hand(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
    zone: Zone,
) -> Result<Option<(PlayerAst, ObjectFilter)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let Some(action_match) =
        sentence_markers::parse_leading_may_action_tokens(&sentence_tokens, &["put"], true)
    else {
        return Ok(None);
    };
    let chooser = leading_may_actor_to_player(action_match.actor, default_player);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let Some(shape) = effect_grammar::parse_looked_card_into_hand_shape(&action_tokens) else {
        return Ok(None);
    };
    let mut filter =
        if let Some(filter) = parse_looked_card_choice_filter(&action_tokens[shape.filter]) {
            filter
        } else {
            return Ok(None);
        };
    filter.zone = Some(zone);

    Ok(Some((chooser, filter)))
}

fn retarget_source_self_animate_effect(effect: EffectAst) -> EffectAst {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::BecomeBasePtCreature {
                    power,
                    toughness,
                    target,
                    card_types,
                    subtypes,
                    subtype_families,
                    colors,
                    abilities,
                    granted_abilities,
                    duration,
                },
            ..
        }) => {
            let target = match target {
                TargetAst::Tagged(tag, span) if tag.as_str() == crate::cards::builders::IT_TAG => {
                    TargetAst::Source(span)
                }
                target => target,
            };
            EffectAst::subject_verb_become_base_pt_creature(
                power,
                toughness,
                target,
                card_types,
                subtypes,
                subtype_families,
                colors,
                abilities,
                granted_abilities,
                duration,
            )
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => EffectAst::Conditional {
            predicate,
            if_true: if_true
                .into_iter()
                .map(retarget_source_self_animate_effect)
                .collect(),
            if_false: if_false
                .into_iter()
                .map(retarget_source_self_animate_effect)
                .collect(),
        },
        EffectAst::IfResult { predicate, effects } => EffectAst::IfResult {
            predicate,
            effects: effects
                .into_iter()
                .map(retarget_source_self_animate_effect)
                .collect(),
        },
        other => other,
    }
}

fn contains_triggered_life_gain_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainLife { .. },
            ..
        }) => true,
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            if_true.iter().any(contains_triggered_life_gain_effect)
                || if_false.iter().any(contains_triggered_life_gain_effect)
        }
        EffectAst::IfResult { effects, .. } => {
            effects.iter().any(contains_triggered_life_gain_effect)
        }
        _ => false,
    }
}

fn contains_tagged_source_animation(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::BecomeBasePtCreature {
                    target, duration, ..
                },
            ..
        }) => {
            let self_animate_target = matches!(
                target,
                TargetAst::Tagged(tag, _) if tag.as_str() == crate::cards::builders::IT_TAG
            ) || matches!(target, TargetAst::Source(_));
            *duration == crate::effect::Until::EndOfTurn && self_animate_target
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            if_true.iter().any(contains_tagged_source_animation)
                || if_false.iter().any(contains_tagged_source_animation)
        }
        EffectAst::IfResult { effects, .. } => effects.iter().any(contains_tagged_source_animation),
        _ => false,
    }
}

fn parse_self_animate_followup_effects(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Ok(effects) = effect_sentences::parse_effect_sentence_lexed(tokens)
        && effects.iter().any(contains_tagged_source_animation)
    {
        return Ok(Some(effects));
    }

    let Some(shape) = effect_grammar::parse_conditional_self_animate_tail(tokens) else {
        return Ok(None);
    };
    let tail = trim_commas(&tokens[shape.effect]);
    let effects = effect_sentences::parse_effect_sentence_lexed(&tail)?;
    if effects.iter().any(contains_tagged_source_animation) {
        Ok(Some(effects))
    } else {
        Ok(None)
    }
}

pub(crate) fn parse_whenever_gain_life_then_self_animate_source(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();

    if !effect_grammar::has_life_gain_surface(first) {
        return Ok(None);
    }

    let first_effects = effect_sentences::parse_effect_sentence_lexed(first)?;
    if !first_effects
        .iter()
        .any(contains_triggered_life_gain_effect)
    {
        return Ok(None);
    }

    let Some(second_effects) = parse_self_animate_followup_effects(second)? else {
        return Ok(None);
    };

    let mut effects = first_effects;
    effects.extend(
        second_effects
            .into_iter()
            .map(retarget_source_self_animate_effect),
    );
    Ok(Some(effects))
}

pub(crate) fn parse_gain_life_then_self_animate_source(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();

    if !effect_grammar::has_life_gain_surface(first) {
        return Ok(None);
    }

    let first_effects = effect_sentences::parse_effect_sentence_lexed(first)?;
    if !first_effects
        .iter()
        .any(contains_triggered_life_gain_effect)
    {
        return Ok(None);
    }

    let Some(second_effects) = parse_self_animate_followup_effects(second)? else {
        return Ok(None);
    };

    let mut effects = first_effects;
    effects.extend(
        second_effects
            .into_iter()
            .map(retarget_source_self_animate_effect),
    );
    Ok(Some(effects))
}

pub(crate) fn parse_choose_then_do_same_for_filter_then_return_to_battlefield(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) = effect_sentences::parse_sentence_choose_then_do_same_for_filter(
        effect_sentences::SubjectVerbPrimitiveClause::new(sentences[sentence_idx].lowered()),
    )?
    else {
        return Ok(None);
    };

    let Some(return_shape) = effect_grammar::parse_return_tagged_battlefield_shape(
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };

    effects.push(EffectAst::subject_verb_return_to_battlefield(
        TargetAst::Tagged(
            TagKey::from(crate::cards::builders::IT_TAG),
            effect_sentences::span_from_tokens(sentences[sentence_idx + 1].lowered()),
        ),
        return_shape.tapped,
        false,
        false,
        ReturnControllerAst::Preserve,
        None,
    ));
    Ok(Some(effects))
}

pub(crate) fn parse_delayed_dies_exile_top_power_choose_play(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !effect_grammar::is_delayed_dies_exile_play_shape(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    ) {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "chosen");
    let mut exiled_filter = ObjectFilter::default();
    exiled_filter.zone = Some(Zone::Exile);
    exiled_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Ok(Some(vec![EffectAst::DelayedWhenLastObjectDiesThisTurn {
        filter: None,
        effects: vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                    crate::cards::builders::IT_TAG,
                )))),
                looked_tag.clone(),
            ),
            EffectAst::subject_verb_exile(TargetAst::Tagged(looked_tag, None), false),
            EffectAst::ChooseObjects {
                filter: exiled_filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
            },
            EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                chosen_tag,
                PlayerAst::You,
                true,
                false,
            ),
        ],
    }]))
}

pub(crate) fn parse_mill_then_may_put_from_among_into_hand(
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
        parse_may_put_filtered_card_from_among_into_hand(second, *player, Zone::Graveyard)?
    else {
        return Ok(None);
    };

    parse_mill_then_may_put_from_among_into_hand_with_if_not_chosen(
        sentences,
        sentence_idx,
        *player,
        chooser,
        filter,
        Vec::new(),
        ChoiceCount::up_to(1),
    )
}

pub(crate) fn parse_top_cards_put_any_matching_to_zone_rest_bottom_same_sentence(
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
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["put"], true)
    else {
        return Ok(None);
    };
    if !effect_grammar::has_rest_on_library_bottom_surface(&second_tokens) {
        return Ok(None);
    }
    let Some(order) = effect_grammar::parse_bottom_order(&second_tokens) else {
        return Ok(None);
    };

    let chooser = leading_may_actor_to_player(action_match.actor, player);
    let Some((mut choice_count, filter, zone, tapped, attacking, attack_target_player)) =
        super::triples::parse_counted_from_looked_cards_action(action_match.tail_tokens)
    else {
        return Ok(None);
    };
    if reveal_top && choice_count != ChoiceCount::any_number() {
        return Ok(None);
    }
    if action_match.actor != LeadingMayActor::Default && choice_count == ChoiceCount::exactly(1) {
        choice_count = ChoiceCount::up_to(1);
    }

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
    effects.push(EffectAst::ChooseObjects {
        filter: choose_filter,
        count: choice_count,
        count_value: None,
        player: chooser,
        tag: chosen_tag.clone(),
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone_with_attack_target(
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
            zone,
            false,
            ReturnControllerAst::Preserve,
            tapped,
            attacking,
            attack_target_player,
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

/// Shared body for the mill-then-choose follow-up, parameterized by the
/// optional "if you don't" branch so both the bare and the if-you-don't
/// callers compose the same reusable primitive sequence (mirroring the retired
/// `ChooseFromLookedCardsIntoHandRestIntoGraveyard` recipe). The milled cards
/// already sit in the graveyard, so the choose filter references them via
/// `IT_TAG` (resolved to the mill's collection tag at lowering) and no
/// rest-into-graveyard split is emitted.
pub(crate) fn parse_mill_then_may_put_from_among_into_hand_with_if_not_chosen(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    player: PlayerAst,
    chooser: PlayerAst,
    filter: ObjectFilter,
    if_not_chosen: Vec<EffectAst>,
    choice_count: ChoiceCount,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let Ok(first_effects) = effect_sentences::parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Mill { .. },
            ..
        }),
    ] = first_effects.as_slice()
    else {
        return Ok(None);
    };
    let _ = player;

    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut effects = vec![first_effects[0].clone()];
    effects.extend(
        super::triples::compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
            chooser,
            filter,
            TagKey::from(crate::cards::builders::IT_TAG),
            chosen_tag,
            Zone::Graveyard,
            false,
            if_not_chosen,
            choice_count,
        ),
    );
    Ok(Some(effects))
}

pub(crate) fn parse_exile_until_match_grant_play_this_turn(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ConsultTopOfLibrary {
                    mode: crate::cards::builders::LibraryConsultModeAst::Exile,
                    stop_rule,
                    ..
                },
            ..
        })) if consult_stop_rule_is_single_match(stop_rule)
    ) {
        return Ok(None);
    }

    let Some(clause) = parse_consult_cast_clause(second) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.extend(consult_cast_effects(&clause, parts.match_tag)?);
    Ok(Some(effects))
}

pub(crate) fn parse_target_player_chooses_then_other_cant_block(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    effect_sentences::parse_target_player_chooses_then_other_cant_block(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    )
}

pub(crate) fn parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    effect_sentences::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    )
}

pub(crate) fn parse_choose_creature_type_then_become_type(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    effect_sentences::parse_choose_creature_type_then_become_type(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    )
}

pub(crate) fn parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(count) = parse_top_of_your_library_count(
        sentences[sentence_idx].lowered(),
        effect_grammar::dispatch_entry_shapes::TopLibraryAction::Reveal,
    ) else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(shape) = effect_grammar::parse_reveal_top_matching_followup_shape(&second_tokens)
    else {
        return Ok(None);
    };
    let filter_tokens = trim_commas(&second_tokens[shape.filter]);
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    if shape.chosen_type_reference {
        filter.chosen_creature_type = true;
    }
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let effects = match shape.remainder {
        effect_grammar::RevealTopRemainder::LibraryBottom(order) => {
            compose_reveal_top_put_matching_into_hand_rest_on_bottom(
                sentences[sentence_idx].lowered(),
                &second_tokens,
                count,
                filter,
                order,
            )
        }
        effect_grammar::RevealTopRemainder::Graveyard => {
            compose_reveal_top_put_matching_into_hand_rest_into_graveyard(
                sentences[sentence_idx].lowered(),
                count,
                filter,
            )
        }
    };

    Ok(Some(effects))
}

/// Composes the "reveal top N, put all matching into hand, rest on bottom" shape
/// from reusable primitives (look + reveal-tagged + tag-matching + move-group +
/// remainder-to-bottom), matching the runtime effects the retired
/// `RevealTopPutMatchingIntoHandRestOnBottomOfLibrary` recipe lowered to.
fn compose_reveal_top_put_matching_into_hand_rest_on_bottom(
    look_tokens: &[OwnedLexToken],
    matched_tokens: &[OwnedLexToken],
    count: u32,
    mut filter: ObjectFilter,
    order: LibraryBottomOrderAst,
) -> Vec<EffectAst> {
    let looked_tag = helper_tag_for_tokens(look_tokens, "revealed");
    let matched_tag = helper_tag_for_tokens(matched_tokens, "matched");
    filter.zone = None;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    vec![
        EffectAst::subject_verb_look_at_top_cards(
            PlayerAst::You,
            Value::Fixed(count as i32),
            looked_tag.clone(),
        ),
        EffectAst::subject_verb_reveal_tagged(looked_tag.clone()),
        EffectAst::subject_verb_tag_matching_objects(
            filter,
            vec![Zone::Library],
            matched_tag.clone(),
        ),
        EffectAst::ForEachTagged {
            tag: matched_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(matched_tag),
            order,
            PlayerAst::You,
        ),
    ]
}

/// Composes the "reveal top N, put matching into hand, rest into graveyard" shape:
/// look + reveal-tagged + per-looked-card conditional split (matches filter -> hand,
/// else -> graveyard), matching the retired
/// `RevealTopPutMatchingIntoHandRestIntoGraveyard` recipe's lowering.
fn compose_reveal_top_put_matching_into_hand_rest_into_graveyard(
    look_tokens: &[OwnedLexToken],
    count: u32,
    mut filter: ObjectFilter,
) -> Vec<EffectAst> {
    let looked_tag = helper_tag_for_tokens(look_tokens, "revealed");
    filter.zone = None;
    let iterated = || TargetAst::Tagged(TagKey::from(IT_TAG), None);
    vec![
        EffectAst::subject_verb_look_at_top_cards(
            PlayerAst::You,
            Value::Fixed(count as i32),
            looked_tag.clone(),
        ),
        EffectAst::subject_verb_reveal_tagged(looked_tag.clone()),
        EffectAst::ForEachTagged {
            tag: looked_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(TagKey::from(IT_TAG), filter),
                if_true: vec![EffectAst::subject_verb_move_to_zone(
                    iterated(),
                    Zone::Hand,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    iterated(),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        },
    ]
}

pub(crate) fn parse_consult_match_move_and_bottom_remainder(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
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

    let second_tokens = trim_commas(second);
    let Some(shape) = effect_grammar::parse_consult_move_bottom_shape(&second_tokens) else {
        return Ok(None);
    };
    if shape == effect_grammar::ConsultMoveBottomShape::MatchedToBattlefieldAndShuffle {
        let mut effects = parts.effects;
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag, None),
            Zone::Battlefield,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            parts.player,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(effects));
    }

    let effect_grammar::ConsultMoveBottomShape::MoveMatchAndBottom {
        zone,
        battlefield_tapped,
        order,
    } = shape
    else {
        unreachable!("shuffle consult shape returned above")
    };

    let mut effects = parts.effects;
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(parts.match_tag.clone(), None),
        zone,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped,
        None,
    ));
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            parts.all_tag,
            Some(parts.match_tag),
            order,
            parts.player,
        ),
    );
    Ok(Some(effects))
}

pub(crate) fn parse_conditional_consult_match_move_and_bottom_remainder(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let Some(shape) = effect_grammar::parse_conditional_consult_shape(&first_tokens) else {
        return Ok(None);
    };
    let predicate_tokens = trim_commas(&first_tokens[shape.predicate]);
    let effect_tokens = trim_commas(&first_tokens[shape.effect]);

    if shape.if_result {
        let synthetic = [
            SentenceInput::from_lexed(&effect_tokens),
            SentenceInput::from_lexed(sentences[sentence_idx + 1].lowered()),
        ];
        let Some(effects) = parse_consult_match_move_and_bottom_remainder(&synthetic, 0)? else {
            return Ok(None);
        };

        return Ok(Some(vec![EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        }]));
    }

    let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(&predicate_tokens) else {
        return Ok(None);
    };

    let synthetic = [
        SentenceInput::from_lexed(&effect_tokens),
        SentenceInput::from_lexed(sentences[sentence_idx + 1].lowered()),
    ];
    let Some(if_true) = parse_consult_match_move_and_bottom_remainder(&synthetic, 0)? else {
        return Ok(None);
    };

    Ok(Some(vec![EffectAst::Conditional {
        predicate,
        if_true,
        if_false: Vec::new(),
    }]))
}

pub(crate) fn parse_consult_match_move_all_to_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };

    if !effect_grammar::is_consult_move_all_to_graveyard_shape(second) {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(parts.all_tag, None),
        Zone::Graveyard,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    Ok(Some(effects))
}

pub(crate) fn parse_consult_match_into_hand_exile_others(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }

    let (second_tokens, _gate_on_result) = strip_leading_if_you_do_sentence(second);
    if !effect_grammar::is_consult_hand_then_exile_others_shape(&second_tokens) {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(parts.match_tag.clone(), None),
        Zone::Hand,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::ForEachTagged {
        tag: parts.all_tag,
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                ObjectFilter::tagged(parts.match_tag),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::subject_verb_exile(
                TargetAst::Tagged(
                    crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                    None,
                ),
                false,
            )],
        }],
    });
    Ok(Some(effects))
}

pub(crate) fn parse_consult_match_into_battlefield_or_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Exile,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    if !effect_grammar::is_consult_battlefield_or_hand_shape(&second_tokens) {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::May {
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag.clone(), None),
            Zone::Battlefield,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::DidNot,
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag, None),
            Zone::Hand,
            false,
            ReturnControllerAst::You,
            false,
            None,
        )],
    });

    Ok(Some(effects))
}

/// Parses the two-sentence pattern:
///   S1: "Reveal cards from the top of your library until you reveal a <filter> card."
///   S2: "Put that card into your hand and all other cards revealed this way into your graveyard."
///
/// This covers cards like Hermit Druid and similar "reveal until, match to hand, rest to graveyard"
/// patterns.
pub(crate) fn parse_consult_match_into_hand_others_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some((parts, optional)) = parse_optional_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }

    let (second_tokens, gate_on_result) = strip_leading_if_you_do_sentence(second);
    if !effect_grammar::is_consult_hand_others_graveyard_shape(&second_tokens) {
        return Ok(None);
    }

    let followups = vec![
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag.clone(), None),
            Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::ForEachTagged {
            tag: parts.all_tag.clone(),
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(
                    crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                    ObjectFilter::tagged(parts.match_tag.clone()),
                ),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(
                        crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                        None,
                    ),
                    Zone::Graveyard,
                    false,
                    crate::cards::builders::ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        },
    ];
    Ok(Some(wrap_optional_consult_effects(
        parts,
        optional,
        followups,
        gate_on_result,
    )))
}

pub(crate) fn parse_consult_match_into_battlefield_others_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some((parts, optional)) = parse_optional_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }

    let (second_tokens, gate_on_result) = strip_leading_if_you_do_sentence(second);
    let Some(shape) = effect_grammar::parse_consult_battlefield_graveyard_shape(&second_tokens)
    else {
        return Ok(None);
    };
    if let effect_grammar::ConsultBattlefieldGraveyardShape::RemainderThenMatch { controller_you } =
        shape
    {
        let controller = if controller_you {
            crate::cards::builders::ReturnControllerAst::You
        } else {
            crate::cards::builders::ReturnControllerAst::Preserve
        };
        let followups = vec![
            EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit,
                SubjectVerbActionAst::PutTaggedRemainderInZone {
                    tag: parts.all_tag.clone(),
                    keep_tagged: parts.match_tag.clone(),
                    zone: Zone::Graveyard,
                },
            ),
            EffectAst::subject_verb_put_onto_battlefield(
                PlayerAst::Implicit,
                TargetAst::Tagged(parts.match_tag.clone(), None),
                false,
                controller,
            ),
        ];
        return Ok(Some(wrap_optional_consult_effects(
            parts,
            optional,
            followups,
            gate_on_result,
        )));
    }

    let followups = vec![
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag.clone(), None),
            Zone::Battlefield,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::ForEachTagged {
            tag: parts.all_tag.clone(),
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(
                    crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                    ObjectFilter::tagged(parts.match_tag.clone()),
                ),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(
                        crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                        None,
                    ),
                    Zone::Graveyard,
                    false,
                    crate::cards::builders::ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        },
    ];
    Ok(Some(wrap_optional_consult_effects(
        parts,
        optional,
        followups,
        gate_on_result,
    )))
}
