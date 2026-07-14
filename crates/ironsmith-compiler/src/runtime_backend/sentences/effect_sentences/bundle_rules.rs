use winnow::Parser as _;

use super::super::activation_and_restrictions::choice_object_clauses::{
    parse_choose_card_type_phrase_words, parse_target_player_choose_objects_clause,
    parse_you_choose_objects_clause, parse_you_choose_objects_clause_with_count_value,
};
use super::super::lexer::{OwnedLexToken, split_lexed_sentences};
use super::super::object_filters::parse_object_filter_lexed;
use super::super::permission_helpers::{
    parse_cast_or_play_tagged_clause, parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
use super::super::util::{
    helper_tag_for_tokens, parse_subject, parse_target_phrase, span_from_tokens, trim_commas, words,
};
use super::dispatch_entry::parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard;
use super::zone_handlers::{parse_exile_top_library_clause, split_exile_face_down_suffix};
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan, Verb,
};
use crate::effect::Value;
use crate::filter::AlternativeCastKind;
use crate::object::CounterType;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::front_end::grammar::effects as bundle_grammar;
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

pub(crate) fn parse_same_sentence_copy_and_may_cast_copy(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<
        crate::runtime_backend::activation_and_restrictions::trigger_subject_filters::MayCastTaggedSpec,
    >,
    CardTextError,
>{
    use super::super::grammar::primitives as grammar;

    let split = grammar::split_lexed_once_on_separator(tokens, || grammar::kw("and").void())
        .or_else(|| grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void()));
    let Some((copy_slice, tail_slice)) = split else {
        return Ok(None);
    };

    let copy_tokens = trim_commas(copy_slice).to_vec();
    if !effect_sentences::is_simple_copy_reference_sentence(&copy_tokens) {
        return Ok(None);
    }

    let tail_tokens = trim_commas(tail_slice).to_vec();
    let Some(spec) = effect_sentences::parse_may_cast_it_sentence(&tail_tokens) else {
        return Ok(None);
    };
    if !spec.as_copy {
        return Ok(None);
    }

    Ok(Some(spec))
}

fn parse_exile_top_library_then_play_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((verb, verb_idx)) = effect_sentences::find_verb(first_sentence) else {
        return Ok(None);
    };
    if verb != Verb::Exile {
        return Ok(None);
    }

    let exile_subject = if verb_idx == 0 {
        None
    } else {
        Some(parse_subject(&trim_commas(&first_sentence[..verb_idx])))
    };
    let exile_tokens = trim_commas(&first_sentence[verb_idx + 1..]);
    let (exile_core, face_down) = split_exile_face_down_suffix(&exile_tokens);
    let exile_effect = if face_down {
        let default_player = exile_subject
            .and_then(|subject| match subject {
                crate::cards::builders::SubjectAst::Player(player) => Some(player),
                _ => None,
            })
            .unwrap_or(PlayerAst::Implicit);
        let Some(shape) = bundle_grammar::parse_exile_top_library_shape(exile_core, default_player)
        else {
            return Ok(None);
        };
        let bundle_grammar::ExileLibraryPlayerShape::EachOpponent = shape.player else {
            return Ok(None);
        };
        let Value::Fixed(count) = shape.count.unhinted() else {
            return Ok(None);
        };
        let Ok(count) = usize::try_from(*count) else {
            return Ok(None);
        };
        let chosen_tag = helper_tag_for_tokens(exile_core, "top_exiled_choice");
        let collection_tag = helper_tag_for_tokens(exile_core, "exiled");
        let mut filter = ObjectFilter::default().in_zone(Zone::Library);
        filter.owner = Some(PlayerFilter::IteratedPlayer);
        EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::ChooseObjectsTopOfLibrary {
                    filter,
                    count: ChoiceCount::exactly(count),
                    count_value: None,
                    // The effect controller receives the printed permission
                    // to inspect the face-down cards. Choosing the sole top
                    // card also records that viewer for the subsequent exile.
                    player: PlayerAst::You,
                    tag: chosen_tag.clone(),
                },
                EffectAst::TagAffected {
                    effect: Box::new(EffectAst::subject_verb_exile(
                        TargetAst::Tagged(chosen_tag, None),
                        true,
                    )),
                    // Accumulate all cards across the opponent loop so one
                    // trailing permission covers the complete collection.
                    tag: collection_tag,
                },
            ],
        }
    } else {
        let Some(effect) = parse_exile_top_library_clause(&exile_tokens, exile_subject) else {
            return Ok(None);
        };
        effect
    };
    let permission_effect = if let Some(effect) =
        parse_until_end_of_turn_may_play_tagged_clause(second_sentence)?
    {
        effect
    } else if let Some(effect) = parse_until_your_next_turn_may_play_tagged_clause(second_sentence)?
    {
        effect
    } else if let Some(effect) = parse_cast_or_play_tagged_clause(second_sentence)? {
        effect
    } else {
        let effects = effect_sentences::parse_effect_sentence_lexed(second_sentence)?;
        let [effect] = effects.as_slice() else {
            return Ok(None);
        };
        effect.clone()
    };

    // Face-down collection permissions can spell out that the controller may
    // "look at and play" the cards. Some generic surfaces preserve the look as
    // an explicit action before the persistent grant. Here the top-library
    // chooser is already `You`, which both identifies the cards and records
    // their face-down viewer, so retain the durable grant and bind it to the
    // accumulated collection produced by the opponent loop.
    let permission_effect = match permission_effect {
        EffectAst::Sequence { effects } if face_down => {
            let [look, permission] = effects.as_slice() else {
                return Ok(None);
            };
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject,
                action: SubjectVerbActionAst::LookAtObjects { filter },
            }) = look
            else {
                return Ok(None);
            };
            if subject.player != PlayerAst::You
                || filter.zone != Some(Zone::Exile)
                || !filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == IT_TAG
                        && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                })
            {
                return Ok(None);
            }
            permission.clone()
        }
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects,
        }
        | EffectAst::May { effects }
            if face_down && effects.len() == 1 =>
        {
            effects.into_iter().next().expect("checked one effect")
        }
        permission => permission,
    };

    let Some(tag) = (match &exile_effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::ExileTopOfLibrary { tags, .. } => tags.first().cloned(),
            _ => None,
        },
        EffectAst::ForEachOpponent { effects } => match effects.as_slice() {
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::ExileTopOfLibrary {
                            accumulated_tags, ..
                        },
                    ..
                }),
            ] => accumulated_tags.first().cloned(),
            [
                EffectAst::ChooseObjectsTopOfLibrary { .. },
                EffectAst::TagAffected { tag, .. },
            ] => Some(tag.clone()),
            _ => None,
        },
        _ => None,
    }) else {
        return Ok(None);
    };

    let permission_effect = match permission_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    ..
                },
            ..
        }) => EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
            tag,
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
        ),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                    player,
                    allow_land,
                    until_next_end_step,
                    ..
                },
            ..
        }) => {
            if until_next_end_step {
                EffectAst::subject_verb_grant_play_tagged_until_your_next_end_step(
                    tag, player, allow_land, false,
                )
            } else {
                EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                    tag, player, allow_land, false,
                )
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    filter,
                    ..
                },
            ..
        }) => EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            tag,
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            filter,
        ),
        _ => return Ok(None),
    };

    Ok(Some(vec![exile_effect, permission_effect]))
}

fn parse_hidden_exile_partition_permission_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [look_sentence, partition_sentence, permission_sentence] = sentences.as_slice() else {
        return Ok(None);
    };

    let mut partition_tokens = look_sentence.to_vec();
    partition_tokens.extend_from_slice(partition_sentence);
    let Some(mut effects) =
        super::dispatch_inner::parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(
            &partition_tokens,
        )
    else {
        return Ok(None);
    };

    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LookAtTopCards { .. },
            ..
        }),
        EffectAst::ChooseObjects {
            tag: selected_tag, ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Exile {
                    target: TargetAst::Tagged(exile_tag, None),
                    face_down: true,
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    keep_tagged: Some(kept_tag),
                    ..
                },
            ..
        }),
    ] = effects.as_mut_slice()
    else {
        return Ok(None);
    };
    if selected_tag != exile_tag || selected_tag != kept_tag {
        return Ok(None);
    }

    let linked_tag = helper_tag_for_tokens(partition_sentence, "exiled");
    *selected_tag = linked_tag.clone();
    *exile_tag = linked_tag.clone();
    *kept_tag = linked_tag.clone();

    let Some(mut permission) = parse_cast_or_play_tagged_clause(permission_sentence)? else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player: PlayerAst::You,
                allow_land: true,
                without_paying_mana_cost: false,
                filter: None,
                ..
            },
        ..
    }) = &mut permission
    else {
        return Ok(None);
    };
    *tag = linked_tag;
    effects.push(permission);

    Ok(Some(effects))
}

fn parse_may_cast_spell_for_alternative_cost_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let kind =
        bundle_grammar::parse_alternative_cost_bundle_shape(first_sentence, second_sentence)?.kind;

    let mut filter = ObjectFilter::nonland()
        .in_zone(Zone::Hand)
        .with_alternative_cast(kind);
    filter.owner = Some(PlayerFilter::You);
    Some(vec![
        EffectAst::may_cast_matching_spell_with_alternative_cost(
            PlayerAst::You,
            filter,
            Zone::Hand,
            kind,
        ),
    ])
}

fn parse_choose_type_then_phase_out_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(first_sentence)?
    else {
        return Ok(None);
    };
    if !choose_count.is_single() {
        return Ok(None);
    }

    if bundle_grammar::parse_chosen_type_reference_shape(second_sentence).is_none() {
        return Ok(None);
    }

    let mut effects = effect_sentences::parse_effect_sentence_lexed(second_sentence)?;
    let [
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action: crate::cards::builders::SubjectVerbActionAst::PhaseOutAll { filter },
            ..
        }),
    ] = effects.as_mut_slice()
    else {
        return Ok(None);
    };

    if choose_filter.card_types.is_empty() {
        return Ok(None);
    }

    let mut phase_out_filter = (*filter).clone();
    phase_out_filter.card_types = choose_filter.card_types.clone();
    phase_out_filter.excluded_subtypes = choose_filter.excluded_subtypes.clone();
    if choose_filter
        .card_types
        .iter()
        .any(|value| *value == crate::types::CardType::Enchantment)
        && choose_filter
            .excluded_subtypes
            .iter()
            .any(|value| *value == Subtype::Aura)
        && !phase_out_filter
            .excluded_subtypes
            .iter()
            .any(|value| *value == Subtype::Aura)
    {
        phase_out_filter.excluded_subtypes.push(Subtype::Aura);
    }
    phase_out_filter = phase_out_filter.match_tagged(
        TagKey::from(IT_TAG),
        TaggedOpbjectRelation::SharesPermanentType,
    );

    let mut choose_filter = choose_filter;
    if choose_filter.controller.is_none() && choose_filter.owner.is_none() {
        choose_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            _ => PlayerFilter::target_player(),
        });
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_phase_out_all(phase_out_filter),
    ]))
}

fn looks_like_source_leaves_return_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    bundle_grammar::parse_source_leaves_return_shape(tokens).is_some()
}

fn promote_exile_effect_to_source_leaves(effect: EffectAst) -> Option<EffectAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match subject_verb.action {
            SubjectVerbActionAst::Exile { target, face_down } => Some(
                EffectAst::subject_verb_exile_until_source_leaves(target, face_down),
            ),
            SubjectVerbActionAst::ExileAll { filter, face_down } => {
                Some(EffectAst::subject_verb_exile_until_source_leaves(
                    TargetAst::Object(filter, None, None),
                    face_down,
                ))
            }
            _ => None,
        },
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } if if_false.is_empty() && if_true.len() == 1 => {
            let inner = promote_exile_effect_to_source_leaves(if_true.into_iter().next().unwrap())?;
            Some(EffectAst::Conditional {
                predicate,
                if_true: vec![inner],
                if_false,
            })
        }
        _ => None,
    }
}

fn parse_exile_then_source_leaves_return_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !looks_like_source_leaves_return_followup_sentence(second_sentence) {
        return Ok(None);
    }

    let first_effects = effect_sentences::parse_effect_sentence_lexed(first_sentence)?;
    let [first_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    let Some(rewritten_first_effect) = promote_exile_effect_to_source_leaves(first_effect.clone())
    else {
        return Ok(None);
    };

    Ok(Some(vec![rewritten_first_effect]))
}

fn parse_reveal_from_outside_game_or_choose_face_up_exile_to_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let shape = bundle_grammar::parse_outside_game_choice_shape(first, second).map_err(|_| {
        CardTextError::ParseError(format!(
            "missing outside-game clause in reveal-or-choose bundle (clause: '{}')",
            words(&trim_commas(first)).join(" ")
        ))
    })?;
    let Some(shape) = shape else {
        return Ok(None);
    };
    let reveal_filter = parse_object_filter_lexed(shape.reveal_filter, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported outside-game reveal filter in reveal-or-choose bundle (clause: '{}')",
            words(&trim_commas(first)).join(" ")
        ))
    })?;
    let mut choose_filter =
        parse_object_filter_lexed(shape.choose_filter, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported exile choice filter in reveal-or-choose bundle (clause: '{}')",
                words(&trim_commas(first)).join(" ")
            ))
        })?;

    if reveal_filter.card_types != choose_filter.card_types
        || reveal_filter.subtypes != choose_filter.subtypes
        || reveal_filter.owner != choose_filter.owner
    {
        return Ok(None);
    }

    choose_filter.zone = None;

    let chosen_tag = TagKey::from("outside_game_or_exile_selected");
    let effects = vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter: choose_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag.clone(),
            zones: vec![Zone::OutsideGame, Zone::Exile],
            search_mode: None,
        },
        EffectAst::subject_verb_reveal_tagged(chosen_tag.clone()),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(chosen_tag, span_from_tokens(second)),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ];

    Ok(Some(vec![EffectAst::May { effects }]))
}

fn parse_reveal_from_outside_game_to_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = bundle_grammar::parse_outside_game_wish_shape(tokens) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter_lexed(&shape.filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported outside-game wish filter in clause '{}'",
            words(&trim_commas(tokens)).join(" ")
        ))
    })?;
    filter.owner = Some(PlayerFilter::You);
    filter.zone = Some(Zone::OutsideGame);

    let wish_tag = TagKey::from("searched_outside_game");
    let effects = vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: PlayerAst::You,
            tag: wish_tag.clone(),
            zones: vec![Zone::OutsideGame],
            search_mode: Some(crate::effect::SearchSelectionMode::Optional),
        },
        EffectAst::subject_verb_reveal_tagged(wish_tag.clone()),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(wish_tag, span_from_tokens(tokens)),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ];
    let mut outer = vec![EffectAst::May { effects }];
    if shape.exile_source {
        outer.push(EffectAst::subject_verb_exile(
            TargetAst::Source(None),
            false,
        ));
    }

    Ok(Some(outer))
}

fn parse_choose_objects_then_for_each_of_those_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: Option<&[OwnedLexToken]>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut normalized_first = first.to_vec();
    for token in &mut normalized_first {
        token.lowercase_word();
    }

    let Some((player, filter, count)) = parse_you_choose_objects_clause(&normalized_first)?
        .or_else(|| {
            parse_target_player_choose_objects_clause(&normalized_first)
                .ok()
                .flatten()
        })
    else {
        return Ok(None);
    };
    let choose_tag = TagKey::from(IT_TAG);

    let Some(loop_shape) = bundle_grammar::parse_for_each_chosen_shape(second) else {
        return Ok(None);
    };
    let loop_body_effects = effect_sentences::parse_effect_sentence_lexed(loop_shape.body)?;
    if loop_body_effects.is_empty() {
        return Ok(None);
    }

    let mut combined = vec![EffectAst::ChooseObjects {
        filter,
        count,
        count_value: None,
        player,
        tag: choose_tag.clone(),
    }];
    combined.push(EffectAst::ForEachTagged {
        tag: choose_tag,
        effects: loop_body_effects,
    });
    if let Some(third) = third {
        let trailing_effects = effect_sentences::parse_effect_sentence_lexed(third)?;
        if trailing_effects.is_empty() {
            return Ok(None);
        }
        combined.extend(trailing_effects);
    }
    Ok(Some(combined))
}

fn parse_discard_reveal_choose_discard_chosen_bundle(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [first, second, third] = sentences else {
        return Ok(None);
    };
    let Some(shape) = bundle_grammar::parse_discard_reveal_choice_shape(first, second, third)
    else {
        return Ok(None);
    };
    let revealed_player = match shape.revealed_player {
        bundle_grammar::RevealedHandPlayer::TargetPlayer => PlayerAst::Target,
        bundle_grammar::RevealedHandPlayer::TargetOpponent => PlayerAst::TargetOpponent,
    };

    let Some((chooser, choose_filter, choose_count, count_value)) =
        parse_you_choose_objects_clause_with_count_value(shape.choose_clause)?
    else {
        return Ok(None);
    };
    let discarded_tag = TagKey::from("discarded_this_way");
    let count_value =
        count_value.map(|_| Value::Count(ObjectFilter::tagged(discarded_tag.clone())));

    let mut discarded_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
    discarded_filter.zone = Some(Zone::Hand);

    Ok(Some(vec![
        EffectAst::subject_verb_discard(
            PlayerAst::Implicit,
            Value::Fixed(0),
            false,
            true,
            None,
            Some(discarded_tag),
        ),
        EffectAst::subject_verb_reveal_hand(revealed_player),
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_discard(
            PlayerAst::That,
            Value::Count(discarded_filter.clone()),
            false,
            false,
            Some(discarded_filter),
            None,
        ),
    ]))
}

fn parse_selected_hand_double_choice_discard_bundle(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [first, second, third] = sentences else {
        return Ok(None);
    };
    let Some(shape) = bundle_grammar::parse_selected_hand_double_choice_shape(first, second, third)
    else {
        return Ok(None);
    };
    let revealed_player = match shape.revealed_player {
        bundle_grammar::RevealedHandPlayer::TargetPlayer => PlayerAst::Target,
        bundle_grammar::RevealedHandPlayer::TargetOpponent => PlayerAst::TargetOpponent,
    };

    let parse_choice = |choice_tokens: &[OwnedLexToken]| {
        let mut clause = shape.choice_prefix.to_vec();
        clause.extend_from_slice(choice_tokens);
        parse_you_choose_objects_clause_with_count_value(&clause)
    };
    let Some((first_chooser, first_filter, first_count, first_count_value)) =
        parse_choice(shape.first_choice)?
    else {
        return Ok(None);
    };
    let Some((second_chooser, second_filter, second_count, second_count_value)) =
        parse_choice(shape.second_choice)?
    else {
        return Ok(None);
    };
    if first_chooser != PlayerAst::You
        || second_chooser != PlayerAst::You
        || !first_count.is_single()
        || !second_count.is_single()
        || first_count_value.is_some()
        || second_count_value.is_some()
        || first_filter.zone != Some(Zone::Hand)
        || second_filter.zone != Some(Zone::Hand)
    {
        return Ok(None);
    }

    // A non-implicit shared tag accumulates both independent selections at
    // runtime. The final discard therefore consumes their union without
    // replacing the first choice when the second choice resolves.
    let selected_tag = helper_tag_for_tokens(second, "selected_hand");
    let discarded_filter = ObjectFilter::tagged(selected_tag.clone()).in_zone(Zone::Hand);

    Ok(Some(vec![
        EffectAst::subject_verb_reveal_hand(revealed_player),
        EffectAst::ChooseObjects {
            filter: first_filter,
            count: first_count,
            count_value: None,
            player: PlayerAst::You,
            tag: selected_tag.clone(),
        },
        EffectAst::ChooseObjects {
            filter: second_filter,
            count: second_count,
            count_value: None,
            player: PlayerAst::You,
            tag: selected_tag.clone(),
        },
        EffectAst::subject_verb_discard(
            PlayerAst::That,
            Value::Count(discarded_filter.clone()),
            false,
            false,
            Some(discarded_filter),
            None,
        ),
    ]))
}

fn chosen_counter_target(
    shape: bundle_grammar::ChosenCounterTarget<'_>,
    first: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    match shape {
        bundle_grammar::ChosenCounterTarget::PermanentOrSuspendedCard => Ok(TargetAst::Object(
            ObjectFilter {
                any_of: vec![
                    ObjectFilter::permanent(),
                    ObjectFilter::default()
                        .in_zone(Zone::Exile)
                        .with_alternative_cast(AlternativeCastKind::Suspend)
                        .with_counter_type(CounterType::Time),
                ],
                ..ObjectFilter::default()
            },
            span_from_tokens(first),
            None,
        )),
        bundle_grammar::ChosenCounterTarget::Clause(tokens) => parse_target_phrase(tokens),
    }
}

fn parse_choose_counter_on_target_then_put_or_remove_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = bundle_grammar::parse_chosen_counter_bundle_shape(first, second) else {
        return Ok(None);
    };
    if shape.action != bundle_grammar::ChosenCounterAction::PutOrRemove {
        return Ok(None);
    }
    let target = chosen_counter_target(shape.target, first)?;
    Ok(Some(vec![
        EffectAst::subject_verb_one_counter_kind_put_or_remove(target),
    ]))
}

fn parse_choose_counter_on_target_then_put_additional_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = bundle_grammar::parse_chosen_counter_bundle_shape(first, second) else {
        return Ok(None);
    };
    if shape.action != bundle_grammar::ChosenCounterAction::PutAdditional {
        return Ok(None);
    }
    let target = chosen_counter_target(shape.target, first)?;
    Ok(Some(vec![
        EffectAst::subject_verb_put_counter_of_chosen_kind(target),
    ]))
}

fn parse_search_library_slots_to_hand_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = bundle_grammar::parse_search_library_slots_shape(tokens) else {
        return Ok(None);
    };

    let mut slots = Vec::new();
    for item in shape.filters {
        let mut filter = parse_object_filter_lexed(&item, false)?;
        if let Some(name) = bundle_grammar::parse_explicit_card_name_surface_tokens(&item) {
            filter.name = Some(name);
        }
        filter.zone = if shape.multi_zone {
            None
        } else {
            Some(Zone::Library)
        };
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        slots.push(crate::cards::builders::SearchLibrarySlotAst {
            filter,
            optional: true,
        });
    }

    Ok(Some(vec![
        EffectAst::subject_verb_search_library_slots_to_hand(
            PlayerAst::You,
            slots,
            true,
            TagKey::from("search_library_slots_progress"),
        ),
    ]))
}

fn search_library_slots_to_hand_effect_from_items(
    filter_items: Vec<Vec<OwnedLexToken>>,
) -> Result<EffectAst, CardTextError> {
    let mut slots = Vec::new();
    for item in filter_items {
        let mut filter = parse_object_filter_lexed(&item, false)?;
        if let Some(name) = bundle_grammar::parse_explicit_card_name_surface_tokens(&item) {
            filter.name = Some(name);
        }
        filter.zone = Some(Zone::Library);
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        slots.push(crate::cards::builders::SearchLibrarySlotAst {
            filter,
            optional: true,
        });
    }

    Ok(EffectAst::subject_verb_search_library_slots_to_hand(
        PlayerAst::You,
        slots,
        true,
        TagKey::from("search_library_slots_progress"),
    ))
}

fn parse_kicked_search_library_slots_replacement_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = bundle_grammar::parse_kicked_search_library_slots_shape(tokens) else {
        return Ok(None);
    };

    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate: PredicateAst::ThisSpellWasKicked,
        if_true: vec![search_library_slots_to_hand_effect_from_items(
            shape.replacement_filters,
        )?],
        if_false: vec![search_library_slots_to_hand_effect_from_items(vec![
            shape.default_filter,
        ])?],
        attach_to_previous_ability: false,
    }]))
}

fn parse_kicked_counter_mana_value_replacement_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let fact = bundle_grammar::parse_kicked_counter_replacement_tokens(tokens)?;
    let base_target = match fact.base.target {
        bundle_grammar::CounterSpellTargetReference::Explicit(target) => target,
        bundle_grammar::CounterSpellTargetReference::PriorSpell(_) => return None,
    };
    let kicked_target = match fact.kicked.target {
        bundle_grammar::CounterSpellTargetReference::Explicit(target) => target,
        bundle_grammar::CounterSpellTargetReference::PriorSpell(span) => TargetAst::Spell(span),
    };

    let counter_if_matches = |target: TargetAst, limit: Value, filter: ObjectFilter| {
        let Value::Fixed(limit) = limit else {
            return None;
        };
        if filter.mana_value.as_ref() != Some(&crate::target::Comparison::LessThanOrEqual(limit)) {
            return None;
        }
        Some(EffectAst::Conditional {
            predicate: PredicateAst::ItMatches(filter),
            if_true: vec![EffectAst::subject_verb_counter(target)],
            if_false: Vec::new(),
        })
    };
    let base = counter_if_matches(base_target, fact.base.limit, fact.base.filter)?;
    let kicked = counter_if_matches(kicked_target, fact.kicked.limit, fact.kicked.filter)?;

    Some(vec![EffectAst::SelfReplacement {
        predicate: PredicateAst::ThisSpellWasKicked,
        if_true: vec![kicked],
        if_false: vec![base],
        attach_to_previous_ability: false,
    }])
}

fn multi_zone_search_destination_effects(
    shape: &bundle_grammar::KickedMultiZoneSearchDestinationShape,
    destination: Zone,
) -> Vec<EffectAst> {
    let searched_tag = TagKey::from("searched_multi_zone");
    vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter: shape.filter.clone(),
            count: shape.count.clone(),
            count_value: None,
            player: PlayerAst::You,
            tag: searched_tag.clone(),
            zones: shape.zones.clone(),
            search_mode: Some(shape.search_mode),
        },
        EffectAst::subject_verb_reveal_tagged(searched_tag.clone()),
        EffectAst::ForEachTagged {
            tag: searched_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(searched_tag, None),
                destination,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ]
}

fn parse_kicked_multi_zone_search_destination_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_kicked_multi_zone_search_destination_tokens(tokens)?;
    Some(vec![EffectAst::SelfReplacement {
        predicate: PredicateAst::ThisSpellWasKicked,
        if_true: multi_zone_search_destination_effects(&shape, shape.kicked_destination),
        if_false: multi_zone_search_destination_effects(&shape, shape.default_destination),
        attach_to_previous_ability: false,
    }])
}

fn parse_persistent_exile_play_tax_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_persistent_exile_play_tax_tokens(tokens)?;
    let tagged = TagKey::from(IT_TAG);
    let target = TargetAst::Object(shape.target_filter, Some(TextSpan::synthetic()), None);
    let mut spell_filter = ObjectFilter::spell()
        .without_type(CardType::Land)
        .cast_by(shape.taxed_caster);
    spell_filter.zone = None;

    Some(vec![
        EffectAst::subject_verb_exile(target, false),
        EffectAst::subject_verb_grant_by_spec(
            crate::grant::GrantSpec::new(
                crate::grant::Grantable::play_from(),
                ObjectFilter::tagged(tagged.clone()),
                Zone::Exile,
            ),
            shape.permission_player,
            crate::grant::GrantDuration::Forever,
        ),
        EffectAst::subject_verb_grant_to_target(
            TargetAst::Tagged(tagged, None),
            crate::grant::Grantable::Ability(crate::static_abilities::StaticAbility::new(
                crate::static_abilities::CostIncreaseManaCost::new(
                    spell_filter,
                    shape.additional_cost,
                ),
            )),
            crate::grant::GrantDuration::Forever,
        ),
    ])
}

fn parse_look_hand_optional_exile_play_tax_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    fn object_filter_mut(target: &mut TargetAst) -> Option<&mut ObjectFilter> {
        match target {
            TargetAst::Object(filter, ..) => Some(filter),
            TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) => {
                object_filter_mut(inner)
            }
            _ => None,
        }
    }

    let sentences = split_lexed_sentences(tokens);
    let [
        look_sentence,
        exile_sentence,
        permission_sentence,
        tax_sentence,
    ] = sentences.as_slice()
    else {
        return None;
    };

    let mut look_effects = effect_sentences::parse_effect_sentence_lexed(look_sentence).ok()?;
    let [look_effect] = look_effects.as_mut_slice() else {
        return None;
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtHand {
            target: hand_target,
        },
        ..
    }) = look_effect
    else {
        return None;
    };
    let TargetAst::Player(hand_owner, _) = hand_target else {
        return None;
    };
    let hand_owner = hand_owner.clone();

    let mut optional_exile = effect_sentences::parse_effect_sentence_lexed(exile_sentence).ok()?;
    let [optional] = optional_exile.as_mut_slice() else {
        return None;
    };
    let exile_effects = match optional {
        EffectAst::May { effects } => effects,
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects,
        } => effects,
        _ => return None,
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Exile {
                    target,
                    face_down: false,
                },
            ..
        }),
    ] = exile_effects.as_mut_slice()
    else {
        return None;
    };
    let exile_filter = object_filter_mut(target)?;
    if !exile_filter.excluded_card_types.contains(&CardType::Land) {
        return None;
    }
    // "from it" refers to the hand established by the first sentence. Make
    // that provenance executable instead of leaving an unscoped nonland-card
    // choice that could select from another zone or player.
    exile_filter.zone = Some(Zone::Hand);
    exile_filter.owner = Some(hand_owner);
    let exile_filter = exile_filter.clone();
    let exiled_tag = helper_tag_for_tokens(exile_sentence, "exiled");

    let permission_effects =
        effect_sentences::parse_effect_sentence_lexed(permission_sentence).ok()?;
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    tag,
                    player: PlayerAst::ItsOwner,
                    allow_land: true,
                    without_paying_mana_cost: false,
                    allow_any_color_for_cast,
                    filter: None,
                },
            ..
        }),
    ] = permission_effects.as_slice()
    else {
        return None;
    };
    if tag.as_str() != IT_TAG
        || *allow_any_color_for_cast != ironsmith_core::value_model::ManaSpendMode::Normal
    {
        return None;
    }

    let tax = bundle_grammar::parse_spell_cast_this_way_tax_tokens(tax_sentence)?;
    let mut spell_filter = ObjectFilter::spell().without_type(CardType::Land);
    if let Some(caster) = tax.taxed_caster {
        spell_filter = spell_filter.cast_by(caster);
    }
    spell_filter.zone = None;

    Some(vec![
        look_effect.clone(),
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects: vec![
                EffectAst::ChooseObjects {
                    filter: exile_filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: exiled_tag.clone(),
                },
                EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), false),
            ],
        },
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            exiled_tag.clone(),
            PlayerAst::ItsOwner,
            true,
            false,
            false,
            None,
        ),
        EffectAst::subject_verb_grant_to_target(
            TargetAst::Tagged(exiled_tag, None),
            crate::grant::Grantable::Ability(crate::static_abilities::StaticAbility::new(
                crate::static_abilities::CostIncreaseManaCost::new(
                    spell_filter,
                    tax.additional_cost,
                ),
            )),
            crate::grant::GrantDuration::Forever,
        ),
    ])
}

fn parse_discard_redraw_mana_value_ladder_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_discard_redraw_mana_value_ladder_tokens(tokens)?;
    let discarded_tag = helper_tag_for_tokens(tokens, "discarded_mana_ladder");
    let selected_tag = helper_tag_for_tokens(tokens, "selected_mana_ladder");

    let mut effects = vec![
        EffectAst::subject_verb_discard(
            PlayerAst::You,
            Value::CardsInHand(PlayerFilter::You)
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::AllCardsInHand),
            false,
            false,
            None,
            Some(discarded_tag.clone()),
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: Value::PendingEffectMetric {
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::Count,
                },
            },
        ),
    ];

    for mana_value in shape.mana_values {
        let mut filter = shape.filter.clone();
        filter.zone = Some(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
        filter.mana_value = Some(crate::filter::Comparison::Equal(mana_value as i32));
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: discarded_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        effects.push(EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: PlayerAst::You,
            tag: selected_tag.clone(),
        });
    }

    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(selected_tag, None),
        Zone::Battlefield,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    Some(effects)
}

fn parse_controller_sacrifice_consult_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_controller_sacrifice_consult_tokens(tokens)?;
    let revealed_tag = TagKey::from("controller_consult_revealed");
    let matched_tag = TagKey::from("controller_consult_matched");
    let target = TargetAst::Object(shape.target_filter, Some(TextSpan::synthetic()), None);
    Some(vec![
        EffectAst::subject_verb_sacrifice(
            PlayerAst::ItsController,
            ObjectFilter::default(),
            1,
            Some(target),
        ),
        EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::That,
            LibraryConsultModeAst::Reveal,
            shape.match_filter,
            LibraryConsultStopRuleAst::FirstMatch,
            revealed_tag,
            matched_tag.clone(),
        ),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(matched_tag, None),
            shape.destination,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::ItsController,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ])
}

fn parse_each_player_shuffle_then_consult_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_each_player_shuffle_then_consult_tokens(tokens)?;
    let mut shuffled_filter = shape.shuffled_filter;
    shuffled_filter.owner = Some(PlayerFilter::IteratedPlayer);
    let mut qualifying_filter = shape.qualifying_filter;
    qualifying_filter.owner = Some(PlayerFilter::IteratedPlayer);
    let mut tagged_library_filter = ObjectFilter::default();
    tagged_library_filter.zone = Some(Zone::Library);

    let shuffled_tag = TagKey::from("each_player_shuffled");
    let qualifying_tag = TagKey::from("each_player_qualifying_shuffled");
    let revealed_tag = TagKey::from("each_player_consult_revealed");
    let matched_tag = TagKey::from("each_player_consult_matched");
    Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_tag_matching_objects(
                shuffled_filter.clone(),
                vec![Zone::Battlefield],
                shuffled_tag.clone(),
            ),
            EffectAst::subject_verb_tag_matching_objects(
                qualifying_filter,
                vec![Zone::Battlefield],
                qualifying_tag.clone(),
            ),
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(shuffled_tag, None),
                Zone::Library,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::That,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::That,
                    tag: qualifying_tag,
                    filter: tagged_library_filter,
                },
                if_true: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::That,
                        LibraryConsultModeAst::Reveal,
                        shape.match_filter,
                        LibraryConsultStopRuleAst::FirstMatch,
                        revealed_tag.clone(),
                        matched_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(matched_tag.clone(), None),
                        shape.destination,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                        revealed_tag,
                        Some(matched_tag),
                        shape.remainder_order,
                        PlayerAst::That,
                    ),
                ],
                if_false: Vec::new(),
            },
        ],
    }])
}

fn parse_proliferate_choose_phase_out_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_proliferate_choose_phase_out_tokens(tokens)?;
    let chosen_tag = TagKey::from(IT_TAG);
    let phase_out_filter = ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .match_tagged(chosen_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
    Some(vec![
        EffectAst::subject_verb_proliferate(Value::Fixed(1)),
        EffectAst::ChooseObjects {
            filter: shape.filter,
            count: shape.count,
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag,
        },
        EffectAst::subject_verb_phase_out_all(phase_out_filter),
    ])
}

fn parse_tap_controlled_objects_then_empty_mana_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_tap_controlled_objects_then_empty_mana_tokens(tokens)?;
    Some(vec![
        EffectAst::subject_verb_target_only(TargetAst::Player(
            PlayerFilter::Any,
            span_from_tokens(tokens),
        )),
        EffectAst::subject_verb_tap_all(shape.filter),
        EffectAst::subject_verb_empty_mana_pool(PlayerAst::Target),
    ])
}

fn parse_energy_pay_any_destroy_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_energy_pay_any_destroy_tokens(tokens)?;
    Some(vec![
        EffectAst::subject_verb_energy_counters(PlayerAst::You, shape.energy),
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects: vec![EffectAst::subject_verb_pay_any_energy(
                PlayerAst::You,
                shape.minimum_payment,
            )],
        },
        EffectAst::subject_verb_destroy_all(shape.filter),
    ])
}

#[path = "bundle_rules/consult_bundles.rs"]
mod consult_bundles;
pub(super) use consult_bundles::parse_consult_disposition_bundle;
use consult_bundles::{
    parse_consult_then_put_matches_battlefield_rest_bottom_bundle,
    parse_reveal_repeated_disposition_bundle, parse_reveal_until_land_put_all_graveyard_bundle,
};

fn parse_bid_life_for_control_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_life_bid_shape(tokens)?;
    let target = parse_target_phrase(shape.target).ok()?;

    Some(vec![EffectAst::BidLife {
        target: target.clone(),
        starting_bid: 0,
        winner_effects: vec![EffectAst::subject_verb_gain_control(
            PlayerAst::Implicit,
            target,
            crate::effect::Until::Forever,
        )],
    }])
}

fn parse_regenerate_then_gain_control_if_regenerates_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_regenerate_control_shape(first, second)?;
    let regenerate_target = parse_target_phrase(shape.regenerate_target).ok()?;
    let control_target = parse_target_phrase(shape.control_target).ok()?;
    let follow_up = EffectAst::subject_verb_gain_control(
        PlayerAst::Implicit,
        control_target,
        crate::effect::Until::Forever,
    );

    Some(vec![
        EffectAst::subject_verb_regenerate_with_follow_up_effects(
            regenerate_target,
            vec![follow_up],
        ),
    ])
}

pub(crate) fn parse_typed_effect_bundle_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    if let Ok(Some(effects)) = parse_hidden_exile_partition_permission_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_discard_redraw_mana_value_ladder_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_energy_pay_any_destroy_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_consult_disposition_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_reveal_repeated_disposition_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_reveal_from_outside_game_to_hand(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_look_hand_optional_exile_play_tax_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_persistent_exile_play_tax_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_controller_sacrifice_consult_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_each_player_shuffle_then_consult_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_proliferate_choose_phase_out_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_tap_controlled_objects_then_empty_mana_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_reveal_until_land_put_all_graveyard_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_bid_life_for_control_bundle(tokens) {
        return Some(effects);
    }
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() == 2
        && let Some(effects) =
            parse_regenerate_then_gain_control_if_regenerates_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_consult_then_put_matches_battlefield_rest_bottom_bundle(
            sentences[0],
            sentences[1],
        )
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_then_source_leaves_return_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_top_library_then_play_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Some(effects) =
            parse_may_cast_spell_for_alternative_cost_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_type_then_phase_out_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_reveal_from_outside_game_or_choose_face_up_exile_to_hand(
            sentences[0],
            sentences[1],
        )
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_selected_hand_double_choice_discard_bundle(&sentences)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_discard_reveal_choose_discard_chosen_bundle(&sentences)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_choose_objects_then_for_each_of_those_bundle(
            sentences[0],
            sentences[1],
            Some(sentences[2]),
        )
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_objects_then_for_each_of_those_bundle(sentences[0], sentences[1], None)
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_counter_on_target_then_put_or_remove_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_counter_on_target_then_put_additional_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            effect_sentences::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
                sentences[0],
                sentences[1],
            )
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && {
            let first_words = crate::runtime_backend::token_word_refs(sentences[0]);
            let choice_words = if first_words.first().copied() == Some("you") {
                &first_words[1..]
            } else {
                &first_words[..]
            };
            matches!(
                parse_choose_card_type_phrase_words(choice_words),
                Ok(Some((consumed, _))) if consumed == choice_words.len()
            )
        }
        && let Ok(Some(mut effects)) =
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
                sentences[1],
                sentences[2],
            )
    {
        let first_words = crate::runtime_backend::token_word_refs(sentences[0]);
        let choice_words = if first_words.first().copied() == Some("you") {
            &first_words[1..]
        } else {
            &first_words[..]
        };
        let (_, options) = parse_choose_card_type_phrase_words(choice_words)
            .ok()
            .flatten()
            .expect("validated choose-card-type bundle prefix");
        let mut combined = vec![EffectAst::subject_verb_choose_card_type(
            PlayerAst::You,
            options,
        )];
        combined.append(&mut effects);
        return Some(combined);
    }
    if let Ok(Some(effects)) = parse_kicked_search_library_slots_replacement_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_kicked_counter_mana_value_replacement_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_kicked_multi_zone_search_destination_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_search_library_slots_to_hand_bundle(tokens) {
        return Some(effects);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn conditional_mana_value_limit(effect: &EffectAst) -> Option<i32> {
        let EffectAst::Conditional {
            predicate: PredicateAst::ItMatches(filter),
            if_true,
            if_false,
        } = effect
        else {
            return None;
        };
        if !if_false.is_empty()
            || !matches!(
                if_true.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Counter {
                        target: TargetAst::Spell(_),
                    },
                    ..
                })]
            )
        {
            return None;
        }
        match filter.mana_value.as_ref() {
            Some(crate::target::Comparison::LessThanOrEqual(limit)) => Some(*limit),
            _ => None,
        }
    }

    #[test]
    fn kicked_counter_bundle_builds_self_replacement_ast_before_lowering() {
        let tokens = lex_line(
            "Counter target spell if its mana value is 3 or less. If this spell was kicked, counter that spell if its mana value is 7 or less instead.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).unwrap();
        let [
            EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                attach_to_previous_ability,
            },
        ] = effects.as_slice()
        else {
            panic!("expected a typed self-replacement AST, got {effects:#?}");
        };

        assert_eq!(predicate, &PredicateAst::ThisSpellWasKicked);
        assert!(!*attach_to_previous_ability);
        assert_eq!(
            if_false.first().and_then(conditional_mana_value_limit),
            Some(3)
        );
        assert_eq!(
            if_true.first().and_then(conditional_mana_value_limit),
            Some(7)
        );
    }

    #[test]
    fn selected_hand_double_choice_builds_distinct_filters_with_one_accumulating_tag() {
        let tokens = lex_line(
            "Target opponent reveals their hand. You choose from it a nonland card with mana value 3 or less and a card with mana value 4 or greater. That player discards those cards.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).expect("selected-hand bundle");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RevealHand,
                ..
            }),
            EffectAst::ChooseObjects {
                filter: first_filter,
                count: first_count,
                tag: first_tag,
                ..
            },
            EffectAst::ChooseObjects {
                filter: second_filter,
                count: second_count,
                tag: second_tag,
                ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Discard {
                        count: Value::Count(discard_filter),
                        filter: Some(card_filter),
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected reveal, two choices, and one tagged discard, got {effects:#?}");
        };

        assert_eq!(first_count, &ChoiceCount::exactly(1));
        assert_eq!(second_count, &ChoiceCount::exactly(1));
        assert_eq!(first_tag, second_tag);
        assert!(first_filter.excluded_card_types.contains(&CardType::Land));
        assert!(matches!(
            first_filter.mana_value.as_ref(),
            Some(crate::target::Comparison::LessThanOrEqual(3))
        ));
        assert!(second_filter.excluded_card_types.is_empty());
        assert!(matches!(
            second_filter.mana_value.as_ref(),
            Some(crate::target::Comparison::GreaterThanOrEqual(4))
        ));
        for filter in [discard_filter, card_filter] {
            assert!(filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    && &constraint.tag == first_tag
            }));
        }
    }

    #[test]
    fn each_opponent_top_card_permission_preserves_the_accumulated_collection() {
        let tokens = lex_line(
            "Exile the top card of each opponent's library face down. You may look at and play those cards for as long as they remain exiled.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).expect("exile/permission bundle");
        let [
            EffectAst::ForEachOpponent {
                effects: exile_each,
            },
            permission,
        ] = effects.as_slice()
        else {
            panic!("expected each-opponent exile plus shared permission, got {effects:#?}");
        };
        let [
            EffectAst::ChooseObjectsTopOfLibrary {
                player: PlayerAst::You,
                ..
            },
            EffectAst::TagAffected {
                tag: collection_tag,
                ..
            },
        ] = exile_each.as_slice()
        else {
            panic!("expected typed top-library exile, got {exile_each:#?}");
        };
        assert!(matches!(
            permission,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled { tag, .. },
                ..
            }) if tag == collection_tag
        ));
    }

    #[test]
    fn hidden_exile_partition_uses_one_tag_for_choice_remainder_and_permission() {
        let tokens = lex_line(
            "Look at the top two cards of target opponent's library. Exile one of them face down and put the other on the bottom of that library. You may play the exiled card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).expect("hidden-exile bundle");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::LookAtTopCards { .. },
                ..
            }),
            EffectAst::ChooseObjects {
                tag: selected_tag, ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Exile {
                        target: TargetAst::Tagged(exile_tag, None),
                        face_down: true,
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        keep_tagged: Some(kept_tag),
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                        tag: permission_tag,
                        allow_any_color_for_cast:
                            ironsmith_core::value_model::ManaSpendMode::AnyColor,
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected linked hidden-exile partition, got {effects:#?}");
        };

        assert_eq!(selected_tag, exile_tag);
        assert_eq!(selected_tag, kept_tag);
        assert_eq!(selected_tag, permission_tag);
    }

    #[test]
    fn looked_hand_exile_permission_tax_stays_in_one_linked_program() {
        let tokens = lex_line(
            "Look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it. A spell cast this way costs {2} more to cast.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).expect("linked hand-exile bundle");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::LookAtHand { .. },
                ..
            }),
            EffectAst::MayByPlayer {
                player: PlayerAst::You,
                effects: optional_exile,
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                        tag: permission_tag,
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantToTarget {
                        target: TargetAst::Tagged(tax_tag, None),
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one linked hand-exile program, got {effects:#?}");
        };
        let [
            EffectAst::ChooseObjects {
                filter,
                player: PlayerAst::You,
                tag: choice_tag,
                ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Exile {
                        target: TargetAst::Tagged(exile_tag, None),
                        face_down: false,
                    },
                ..
            }),
        ] = optional_exile.as_slice()
        else {
            panic!("expected an optional typed choose/exile pair, got {optional_exile:#?}");
        };
        assert_eq!(filter.zone, Some(Zone::Hand));
        assert!(matches!(&filter.owner, Some(PlayerFilter::Target(_))));
        assert_eq!(choice_tag, exile_tag);
        assert_eq!(choice_tag, permission_tag);
        assert_eq!(choice_tag, tax_tag);
    }
}
