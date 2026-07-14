use super::super::super::dispatch_entry::{
    ConsultSentenceParts, consult_cast_effects, consult_stop_rule_is_single_match,
    leading_may_actor_to_player, parse_consult_cast_clause, parse_consult_traversal_sentence,
    parse_looked_card_choice_filter, parse_looked_card_reveal_filter,
    parse_top_cards_view_sentence, parse_top_of_your_library_count, target_references_it,
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
use crate::runtime_backend::grammar::effects::triple_sequence_shapes as triple_grammar;
use crate::runtime_backend::grammar::effects::{
    self as effect_grammar, parse_reciprocal_creature_control_sequence_tokens,
};
use crate::runtime_backend::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed;
use crate::runtime_backend::object_filters::parse_object_filter_lexed;
use crate::runtime_backend::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::runtime_backend::util::trim_commas;
use crate::runtime_backend::util::{helper_tag_for_tokens, parse_subject};
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

fn move_looked_partition_group(
    tag: TagKey,
    destination: effect_grammar::LookedPartitionDestination,
    library_owner: PlayerAst,
) -> EffectAst {
    let (zone, to_top, order) = match destination {
        effect_grammar::LookedPartitionDestination::Hand => (Zone::Hand, false, None),
        effect_grammar::LookedPartitionDestination::Graveyard => (Zone::Graveyard, false, None),
        effect_grammar::LookedPartitionDestination::LibraryTop(order) => {
            (Zone::Library, true, Some(order))
        }
        effect_grammar::LookedPartitionDestination::LibraryBottom(order) => {
            (Zone::Library, false, Some(order))
        }
    };
    let mut effect = EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(tag, None),
        zone,
        to_top,
        ReturnControllerAst::Preserve,
        false,
        None,
    )
    .with_library_order(order, PlayerAst::You);

    if matches!(zone, Zone::Hand | Zone::Graveyard) {
        if library_owner == PlayerAst::You {
            effect = effect.with_destination_player_surface(Some(PlayerAst::You));
        } else {
            effect = effect.with_destination_player_reference_surface(Some(
                ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer,
            ));
        }
    }
    effect
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
    ];
    if shape.untap && shape.untap_before_control {
        effects.push(EffectAst::subject_verb_untap_all(both_tagged.clone()));
    }
    effects.extend([
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
    ]);
    if shape.untap && !shape.untap_before_control {
        effects.push(EffectAst::subject_verb_untap_all(both_tagged.clone()));
    }
    if shape.grant_haste {
        let mut haste = EffectAst::subject_verb_grant_abilities_all(
            both_tagged,
            vec![GrantedAbilityAst::StaticAbility(StaticAbility::haste())],
            shape.duration,
        );
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesAll {
                    set_quantifier_surface,
                    ..
                },
            ..
        }) = &mut haste
        {
            *set_quantifier_surface = Some(ironsmith_core::SetQuantifierSurface::Each);
        }
        effects.push(haste);
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

    let counted = match &shape {
        effect_grammar::LookExileFaceDownShape::Counted {
            look,
            exile,
            count,
            bottom_order,
        } => Some((look, exile, count, Some(*bottom_order))),
        effect_grammar::LookExileFaceDownShape::CountedGraveyardRemainder {
            look,
            exile,
            count,
        } => Some((look, exile, count, None)),
        effect_grammar::LookExileFaceDownShape::Single { .. } => None,
    };
    if let Some((look, exile, exile_count, bottom_order)) = counted {
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

        let remainder = if let Some(bottom_order) = bottom_order {
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag.clone(),
                Some(exiled_tag.clone()),
                bottom_order,
                library_owner,
            )
        } else {
            EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit,
                SubjectVerbActionAst::PutTaggedRemainderInZone {
                    tag: looked_tag.clone(),
                    keep_tagged: exiled_tag.clone(),
                    zone: Zone::Graveyard,
                },
            )
        };

        return Ok(Some(vec![
            EffectAst::subject_verb_look_at_top_cards(library_owner, count, looked_tag.clone()),
            EffectAst::ChooseTaggedObjectsInZone {
                filter: choice_filter,
                count: *exile_count,
                player: PlayerAst::You,
                tag: exiled_tag.clone(),
                zone: Zone::Library,
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), true),
            remainder,
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
    let Some(effect_grammar::LookedCardDisposition::HandAndLibraryBottom(bottom_order)) =
        effect_grammar::parse_looked_card_disposition(&second_tokens)
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let hand_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "hand");
    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
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
            looked_tag,
            Some(hand_tag),
            bottom_order,
            player,
        ),
    ]))
}

pub(crate) fn parse_look_at_top_then_partition_selected_and_remainder(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((library_owner, count, false)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(shape) = effect_grammar::parse_looked_card_partition_shape(&trim_commas(
        sentences[sentence_idx + 1].lowered(),
    )) else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked_partition");
    let selected_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "partition_selected");
    let remainder_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "partition_remainder");
    let selected_filter = tagged_library_candidate_filter(&looked_tag, &[]);
    let remainder_filter =
        tagged_library_candidate_filter(&looked_tag, std::slice::from_ref(&selected_tag));

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(library_owner, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: selected_filter,
            count: shape.selected_count,
            player: PlayerAst::You,
            tag: selected_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_tag_matching_objects(
            remainder_filter,
            vec![Zone::Library],
            remainder_tag.clone(),
        ),
        move_looked_partition_group(selected_tag, shape.selected_destination, library_owner),
        move_looked_partition_group(remainder_tag, shape.remainder_destination, library_owner),
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
    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            player,
            tag: hand_tag.clone(),
            zone: Zone::Library,
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
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutTaggedRemainderInZone {
                tag: looked_tag,
                keep_tagged: hand_tag,
                zone: Zone::Graveyard,
            },
        ),
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

    let chosen_tag = helper_tag_for_tokens(&first, "graveyard_cast_target");
    let cast_spell_tag = helper_tag_for_tokens(&first, "cast_spell");
    let mut filter = ObjectFilter::default();
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);
    filter.card_types = if shape.includes_artifact && shape.artifact_first {
        vec![CardType::Artifact, CardType::Instant, CardType::Sorcery]
    } else if shape.includes_artifact {
        vec![CardType::Instant, CardType::Sorcery, CardType::Artifact]
    } else {
        vec![CardType::Instant, CardType::Sorcery]
    };
    if let Some(limit) = shape.mana_value_limit {
        filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(limit));
    }

    let replacement_filter = ObjectFilter::tagged(cast_spell_tag.clone()).in_zone(Zone::Stack);

    Ok(Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_target_only(TargetAst::Object(
                filter,
                Some(TextSpan::synthetic()),
                None,
            ))),
            tag: chosen_tag.clone(),
        },
        EffectAst::May {
            effects: vec![EffectAst::TagAffected {
                effect: Box::new(EffectAst::subject_verb_cast_tagged(
                    chosen_tag,
                    PlayerAst::You,
                    false,
                    false,
                    shape.without_paying_mana_cost,
                    None,
                )),
                tag: cast_spell_tag,
            }],
        },
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![EffectAst::subject_verb_register_future_zone_replacement(
                replacement_filter,
                Some(Zone::Stack),
                Some(Zone::Graveyard),
                Zone::Exile,
                ZoneReplacementDurationAst::OneShot,
                crate::cards::builders::FutureZoneReplacementCausePolicyAst::Any,
                false,
            )],
        },
    ]))
}

pub(crate) fn parse_filtered_future_exile_then_return_next_end_step(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !effect_grammar::is_filtered_future_exile_return_next_end_step_shape(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    ) {
        return Ok(None);
    }

    let linked_filter = ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile);
    Ok(Some(vec![
        EffectAst::subject_verb_register_future_zone_replacement(
            ObjectFilter::permanent().controlled_by(PlayerFilter::You),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ZoneReplacementDurationAst::UntilEndOfTurn,
            crate::cards::builders::FutureZoneReplacementCausePolicyAst::Any,
            true,
        ),
        EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::subject_verb_return_all_to_battlefield(
                linked_filter,
                false,
                false,
                ReturnControllerAst::Owner,
            )],
        },
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
        false,
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
        false,
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
                    preserve_other_types,
                    type_retention_surface,
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
                preserve_other_types,
                type_retention_surface,
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
                ))))
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
                looked_tag.clone(),
            ),
            EffectAst::subject_verb_exile(TargetAst::Tagged(looked_tag, None), false),
            EffectAst::ChooseTaggedObjectsInZone {
                filter: exiled_filter,
                count: ChoiceCount::exactly(1),
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
                zone: Zone::Exile,
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
    let [first_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    let mut first_effect = first_effect.clone();
    let milled_tag = helper_tag_for_tokens(first, "milled");
    let Some(player) = tag_single_mill_effect(&mut first_effect, &milled_tag) else {
        return Ok(None);
    };
    let Some((mut followup, conditional_followup)) =
        parse_put_from_milled_cards_followup(second, player, milled_tag)?
    else {
        return Ok(None);
    };

    if !conditional_followup && append_to_outer_if_result(&mut first_effect, &mut followup) {
        return Ok(Some(vec![first_effect]));
    }

    let mut effects = vec![first_effect];
    if conditional_followup {
        effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: followup,
        });
    } else {
        effects.extend(followup);
    }
    Ok(Some(effects))
}

fn tag_single_mill_effect(effect: &mut EffectAst, tag: &TagKey) -> Option<PlayerAst> {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::Mill { .. },
    }) = effect
    {
        let player = *player;
        let mill = effect.clone();
        *effect = EffectAst::TagAffected {
            effect: Box::new(mill),
            tag: tag.clone(),
        };
        return Some(player);
    }

    let nested = match effect {
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ResolvedIfResult { effects, .. }
        | EffectAst::Sequence { effects } => effects,
        _ => return None,
    };
    let [nested] = nested.as_mut_slice() else {
        return None;
    };
    tag_single_mill_effect(nested, tag)
}

fn append_to_outer_if_result(effect: &mut EffectAst, followup: &mut Vec<EffectAst>) -> bool {
    let effects = match effect {
        EffectAst::IfResult { effects, .. } | EffectAst::ResolvedIfResult { effects, .. } => {
            effects
        }
        _ => return false,
    };
    effects.append(followup);
    true
}

fn milled_choice_filter_branches(filter: &ObjectFilter) -> Option<Vec<ObjectFilter>> {
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

fn parse_put_from_milled_cards_followup(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
    milled_tag: TagKey,
) -> Result<Option<(Vec<EffectAst>, bool)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let (conditional_followup, action_sentence) = if let Some(conditional) =
        sentence_markers::parse_conditional_followup_tokens(&sentence_tokens)
    {
        (true, trim_commas(conditional.tail_tokens))
    } else {
        (false, sentence_tokens)
    };
    let Some(action_match) =
        sentence_markers::parse_leading_may_action_tokens(&action_sentence, &["put"], true)
    else {
        return Ok(None);
    };
    let chooser = leading_may_actor_to_player(action_match.actor, default_player);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let Some((
        mut choice_count,
        filter,
        aggregate_constraint,
        zone,
        tapped,
        attacking,
        attack_target_player,
        all_matching,
    )) = super::triples::parse_counted_from_looked_cards_action(&action_tokens)
    else {
        return Ok(None);
    };
    if aggregate_constraint.is_some() || all_matching {
        return Ok(None);
    }
    if action_match.actor != LeadingMayActor::Default && choice_count == ChoiceCount::exactly(1) {
        choice_count = ChoiceCount::up_to(1);
    }

    let chosen_tag = helper_tag_for_tokens(tokens, "chosen_milled");
    let uses_and_or = action_tokens.iter().any(|token| token.is_word("and/or"));
    let branch_filters = (uses_and_or && choice_count == ChoiceCount::up_to(1))
        .then(|| milled_choice_filter_branches(&filter))
        .flatten();
    let mut effects = Vec::new();
    if let Some(branches) = branch_filters {
        for mut branch in branches {
            branch.zone = Some(Zone::Graveyard);
            branch.tagged_constraints.push(TaggedObjectConstraint {
                tag: milled_tag.clone(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
            branch.tagged_constraints.push(TaggedObjectConstraint {
                tag: chosen_tag.clone(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
            effects.push(EffectAst::ChooseTaggedObjectsInZone {
                filter: branch,
                count: ChoiceCount::up_to(1),
                player: chooser,
                tag: chosen_tag.clone(),
                zone: Zone::Graveyard,
            });
        }
    } else {
        let mut filter = filter;
        filter.zone = Some(Zone::Graveyard);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: milled_tag,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        effects.push(EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: choice_count,
            player: chooser,
            tag: chosen_tag.clone(),
            zone: Zone::Graveyard,
        });
    }
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag,
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
    Ok(Some((effects, conditional_followup)))
}

pub(crate) fn parse_top_cards_put_any_matching_to_zone_rest_same_sentence(
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
    let Some(remainder) = triple_grammar::parse_looked_remainder_shape(&second_tokens) else {
        return Ok(None);
    };
    let order = match remainder {
        triple_grammar::LookedRemainderShape::LibraryBottom(order) => Some(order),
        triple_grammar::LookedRemainderShape::Graveyard => None,
    };

    let chooser = leading_may_actor_to_player(action_match.actor, player);
    let Some((
        mut choice_count,
        filter,
        aggregate_constraint,
        zone,
        tapped,
        attacking,
        attack_target_player,
        all_matching,
    )) = super::triples::parse_counted_from_looked_cards_action(action_match.tail_tokens)
    else {
        return Ok(None);
    };
    if all_matching && action_match.actor != LeadingMayActor::Default {
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
    if let Some(order) = order {
        effects.push(
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag,
                Some(chosen_tag),
                order,
                chooser,
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

    Ok(Some(effects))
}

/// Parses the optional-look pair shared by Fertile Thicket, Planar Atlas,
/// Munda, and similar cards:
///
/// "You may look at the top N cards ... . If you do, reveal <counted filter>
/// from among them, then put those cards on top ... and the rest on the
/// bottom ... ."
pub(crate) fn parse_optional_look_then_reveal_put_top_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let [you, may, view_tokens @ ..] = first_tokens.as_slice() else {
        return Ok(None);
    };
    if !you.is_word("you") || !may.is_word("may") {
        return Ok(None);
    }
    let Some((player, count, false)) = parse_top_cards_view_sentence(view_tokens) else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(followup) = sentence_markers::parse_conditional_followup_tokens(&second_tokens) else {
        return Ok(None);
    };
    if followup.actor != ConditionalFollowupActor::You {
        return Ok(None);
    }
    let followup_tokens = trim_commas(followup.tail_tokens);
    let Some(reveal_action) =
        sentence_markers::parse_leading_may_action_tokens(&followup_tokens, &["reveal"], true)
    else {
        return Ok(None);
    };
    let Some(shape) =
        triple_grammar::parse_looked_top_and_remainder_action_shape(reveal_action.tail_tokens)
    else {
        return Ok(None);
    };
    let filter_tokens = trim_commas(&reveal_action.tail_tokens[shape.filter]);
    let Some(mut filter) = parse_looked_card_reveal_filter(&filter_tokens) else {
        return Ok(None);
    };
    effect_sentences::normalize_search_library_filter(&mut filter);

    let chooser = leading_may_actor_to_player(reveal_action.actor, player);
    let looked_tag = helper_tag_for_tokens(&first_tokens, "looked");
    let chosen_tag = helper_tag_for_tokens(&second_tokens, "revealed");
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let followup_effects = vec![
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: shape.count,
            player: chooser,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::subject_verb_reveal_tagged(chosen_tag.clone())],
        },
        EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                Zone::Library,
                true,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag.clone(),
            Some(chosen_tag),
            shape.remainder_order,
            player,
        ),
    ];

    Ok(Some(vec![
        EffectAst::May {
            effects: vec![EffectAst::subject_verb_look_at_top_cards(
                player, count, looked_tag,
            )],
        },
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: followup_effects,
        },
    ]))
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

pub(crate) fn parse_exile_until_match_put_counters_on_match(
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

    let Ok(counter_effects) = effect_sentences::parse_effect_sentence_lexed(second) else {
        return Ok(None);
    };
    let [EffectAst::SubjectVerb(counter_effect)] = counter_effects.as_slice() else {
        return Ok(None);
    };
    let mut counter_effect = counter_effect.clone();
    let SubjectVerbActionAst::PutCounters { target, .. } = &mut counter_effect.action else {
        return Ok(None);
    };
    if !target_references_it(target) {
        return Ok(None);
    }
    let reference_span = match &*target {
        TargetAst::Tagged(_, span) | TargetAst::Source(span) => *span,
        TargetAst::Object(_, _, span) => *span,
        _ => None,
    };
    *target = TargetAst::Tagged(parts.match_tag.clone(), reference_span);

    let mut effects = parts.effects;
    effects.push(EffectAst::SubjectVerb(counter_effect));
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

#[cfg(test)]
mod looked_partition_tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn parse_pair(first: &str, second: &str) -> Option<Vec<EffectAst>> {
        let first = lex_line(first, 0).expect("first sentence should lex");
        let second = lex_line(second, 1).expect("second sentence should lex");
        let sentences = [
            SentenceInput::from_lexed(&first),
            SentenceInput::from_lexed(&second),
        ];
        parse_look_at_top_then_partition_selected_and_remainder(&sentences, 0)
            .expect("partition parser should not error")
    }

    fn move_parts(effect: &EffectAst) -> (Zone, bool, Option<LibraryBottomOrderAst>, PlayerAst) {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    zone,
                    to_top,
                    library_order,
                    library_order_chooser,
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected a structured move-to-zone effect: {effect:?}");
        };
        (*zone, *to_top, *library_order, *library_order_chooser)
    }

    #[test]
    fn target_library_partition_keeps_you_as_chooser_and_tags_the_complement() {
        let effects = parse_pair(
            "Look at the top five cards of target opponent's library",
            "Put one of those cards into that player's graveyard and the rest on top of their library in any order",
        )
        .expect("Cruel Fate shape should parse");
        assert_eq!(effects.len(), 5);

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: library_owner,
                    ..
                },
            action:
                SubjectVerbActionAst::LookAtTopCards {
                    tag: looked_tag, ..
                },
        }) = &effects[0]
        else {
            panic!("expected looked-card provenance: {:?}", effects[0]);
        };
        assert_eq!(*library_owner, PlayerAst::TargetOpponent);

        let EffectAst::ChooseTaggedObjectsInZone {
            player,
            tag: selected_tag,
            count,
            zone,
            ..
        } = &effects[1]
        else {
            panic!("expected selected-subset choice: {:?}", effects[1]);
        };
        assert_eq!(*player, PlayerAst::You);
        assert_eq!(*count, ChoiceCount::exactly(1));
        assert_eq!(*zone, Zone::Library);

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TagMatchingObjects { filter, zones, .. },
            ..
        }) = &effects[2]
        else {
            panic!("expected a structured complement tag: {:?}", effects[2]);
        };
        assert_eq!(zones, &[Zone::Library]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *looked_tag
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *selected_tag
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));

        assert_eq!(move_parts(&effects[3]).0, Zone::Graveyard);
        assert_eq!(
            move_parts(&effects[4]),
            (
                Zone::Library,
                true,
                Some(LibraryBottomOrderAst::ChooserChooses),
                PlayerAst::You,
            )
        );
    }

    #[test]
    fn selected_and_remainder_library_orders_are_independent() {
        let effects = parse_pair(
            "Look at the top five cards of target player's library",
            "Put any number of them on the bottom of that library in a random order and the rest on top of the library in any order",
        )
        .expect("Ransack shape should parse");
        let EffectAst::ChooseTaggedObjectsInZone { count, player, .. } = &effects[1] else {
            panic!("expected selected-subset choice: {:?}", effects[1]);
        };
        assert_eq!(*count, ChoiceCount::any_number());
        assert_eq!(*player, PlayerAst::You);
        assert_eq!(
            move_parts(&effects[3]),
            (
                Zone::Library,
                false,
                Some(LibraryBottomOrderAst::Random),
                PlayerAst::You,
            )
        );
        assert_eq!(
            move_parts(&effects[4]),
            (
                Zone::Library,
                true,
                Some(LibraryBottomOrderAst::ChooserChooses),
                PlayerAst::You,
            )
        );
    }

    #[test]
    fn counted_hand_selection_and_singular_graveyard_remainder_stay_disjoint() {
        let effects = parse_pair(
            "Look at the top three cards of your library",
            "Put two of them into your hand and the other into your graveyard",
        )
        .expect("counted hand/graveyard partition should parse");
        assert_eq!(effects.len(), 5);

        let EffectAst::ChooseTaggedObjectsInZone {
            count,
            player,
            tag: selected_tag,
            zone,
            ..
        } = &effects[1]
        else {
            panic!("expected selected-subset choice: {:?}", effects[1]);
        };
        assert_eq!(*count, ChoiceCount::exactly(2));
        assert_eq!(*player, PlayerAst::You);
        assert_eq!(*zone, Zone::Library);

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TagMatchingObjects { filter, zones, .. },
            ..
        }) = &effects[2]
        else {
            panic!(
                "expected the exact complement to be tagged: {:?}",
                effects[2]
            );
        };
        assert_eq!(zones, &[Zone::Library]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *selected_tag
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
        assert_eq!(move_parts(&effects[3]).0, Zone::Hand);
        assert_eq!(move_parts(&effects[4]).0, Zone::Graveyard);
    }

    #[test]
    fn existing_bottom_and_rearrange_controls_do_not_match_partition_pair() {
        for second in [
            "Put one of them into your hand and the rest on the bottom of your library in any order",
            "Put them back in any order",
        ] {
            assert!(
                parse_pair("Look at the top five cards of your library", second).is_none(),
                "control should remain on its existing parser path: {second}"
            );
        }
    }

    #[test]
    fn face_down_exile_keeps_the_graveyard_complement_and_permission_tag() {
        let first = lex_line(
            "Look at the top three cards of that player's library, exile one of them face down, then put the rest into their graveyard",
            0,
        )
        .expect("first sentence should lex");
        let second = lex_line(
            "You may cast that card for as long as it remains exiled, and mana of any type can be spent to cast that spell",
            1,
        )
        .expect("second sentence should lex");
        let sentences = [
            SentenceInput::from_lexed(&first),
            SentenceInput::from_lexed(&second),
        ];
        let effects = parse_look_at_top_then_exile_face_down_then_play_while_exiled(&sentences, 0)
            .expect("face-down partition parser should not error")
            .expect("Thief of Sanity shape should parse");
        assert_eq!(effects.len(), 5);

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LookAtTopCards {
                    tag: looked_tag, ..
                },
            ..
        }) = &effects[0]
        else {
            panic!("expected looked-card producer: {:?}", effects[0]);
        };
        let EffectAst::ChooseTaggedObjectsInZone {
            tag: exiled_tag,
            count,
            filter,
            zone,
            ..
        } = &effects[1]
        else {
            panic!("expected face-down selected subset: {:?}", effects[1]);
        };
        assert_eq!(*count, ChoiceCount::exactly(1));
        assert_eq!(*zone, Zone::Library);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *looked_tag
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderInZone {
                    tag,
                    keep_tagged,
                    zone: Zone::Graveyard,
                },
            ..
        }) = &effects[3]
        else {
            panic!("expected exact graveyard complement: {:?}", effects[3]);
        };
        assert_eq!(tag, looked_tag);
        assert_eq!(keep_tagged, exiled_tag);

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    tag: permission_tag,
                    ..
                },
            ..
        }) = &effects[4]
        else {
            panic!("expected tagged cast permission: {:?}", effects[4]);
        };
        assert_eq!(permission_tag, exiled_tag);
    }
}
