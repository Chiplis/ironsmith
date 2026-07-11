use winnow::Parser as _;

use super::super::activation_and_restrictions::choice_object_clauses::{
    parse_choose_card_type_phrase_words, parse_target_player_choose_objects_clause,
    parse_you_choose_objects_clause, parse_you_choose_objects_clause_with_count_value,
};
use super::super::lexer::{OwnedLexToken, parser_token_word_refs, split_lexed_sentences};
use super::super::object_filters::parse_object_filter_lexed;
use super::super::permission_helpers::{
    parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
use super::super::util::{
    helper_tag_for_tokens, parse_subject, parse_target_phrase, span_from_tokens, trim_commas, words,
};
use super::dispatch_entry::parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard;
use super::zone_handlers::parse_exile_top_library_clause;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan, Verb,
};
use crate::effect::{EventValueSpec, Value};
use crate::filter::AlternativeCastKind;
use crate::object::CounterType;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::front_end::grammar::effects as bundle_grammar;
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

pub(crate) fn parse_same_sentence_copy_and_may_cast_copy(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<(
        Vec<EffectAst>,
        crate::runtime_backend::activation_and_restrictions::trigger_subject_filters::MayCastTaggedSpec,
    )>,
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

    let copy_effects = effect_sentences::parse_effect_sentence_lexed(&copy_tokens)?;
    Ok(Some((copy_effects, spec)))
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
    let Some(exile_effect) = parse_exile_top_library_clause(&exile_tokens, exile_subject) else {
        return Ok(None);
    };
    let permission_effect = if let Some(effect) =
        parse_until_end_of_turn_may_play_tagged_clause(second_sentence)?
    {
        effect
    } else if let Some(effect) = parse_until_your_next_turn_may_play_tagged_clause(second_sentence)?
    {
        effect
    } else {
        return Ok(None);
    };

    let Some(tag) = (match &exile_effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::ExileTopOfLibrary { tags, .. } => tags.first().cloned(),
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
                    player, allow_land, ..
                },
            ..
        }) => EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
            tag, player, allow_land, false,
        ),
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
    phase_out_filter =
        phase_out_filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::SharesCardType);

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

fn parse_proliferate_then_choose_permanents_phase_out_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    bundle_grammar::parse_proliferate_phase_out_pair_shape(first_sentence, second_sentence)?;

    let eligible_filter = ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You);
    let chosen_tag = TagKey::from(IT_TAG);
    let mut phase_out_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    phase_out_filter =
        phase_out_filter.match_tagged(chosen_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);

    Some(vec![
        EffectAst::subject_verb_proliferate(Value::Fixed(1)),
        EffectAst::ChooseObjects {
            filter: eligible_filter,
            count: ChoiceCount::any_number(),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag,
        },
        EffectAst::subject_verb_phase_out_all(phase_out_filter),
    ])
}

fn parse_proliferate_then_choose_permanents_phase_out_single_sentence(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    bundle_grammar::parse_proliferate_phase_out_single_shape(tokens)?;

    let eligible_filter = ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You);
    let chosen_tag = TagKey::from(IT_TAG);
    let mut phase_out_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    phase_out_filter =
        phase_out_filter.match_tagged(chosen_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);

    Some(vec![
        EffectAst::subject_verb_proliferate(Value::Fixed(1)),
        EffectAst::ChooseObjects {
            filter: eligible_filter,
            count: ChoiceCount::any_number(),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag,
        },
        EffectAst::subject_verb_phase_out_all(phase_out_filter),
    ])
}

fn parse_draw_create_treasure_lose_life_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    bundle_grammar::parse_draw_treasure_lose_life_shape(tokens)?;

    let amount = Value::EventValue(EventValueSpec::Amount);
    Some(vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: amount.clone(),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::CreateTokenWithMods {
                name: "Treasure".to_string(),
                definition: crate::runtime_backend::token_definition::TokenDefinitionSpec::Builtin(
                    crate::runtime_backend::token_definition::BuiltinTokenShape::Treasure,
                ),
                count: amount.clone(),
                dynamic_power_toughness: None,
                player: PlayerAst::You,
                attached_to: None,
                tapped: true,
                attacking: false,
                exile_at_end_of_combat: false,
                sacrifice_at_end_of_combat: false,
                sacrifice_at_next_end_step: false,
                exile_at_next_end_step: false,
                next_end_step_player: PlayerFilter::Any,
                granted_abilities: Vec::new(),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LoseLife { amount },
        ),
    ])
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

    let chosen_tag = TagKey::from("__coax_or_karn_selected__");
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

fn search_library_and_graveyard_doctors_effects(destination: Zone) -> Vec<EffectAst> {
    let searched_tag = TagKey::from("searched_multi_zone");
    let mut filter = ObjectFilter::default();
    filter.owner = Some(PlayerFilter::You);
    filter.subtypes = vec![Subtype::Doctor];

    vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: ChoiceCount::up_to(5),
            count_value: None,
            player: PlayerAst::You,
            tag: searched_tag.clone(),
            zones: vec![Zone::Library, Zone::Graveyard],
            search_mode: Some(crate::effect::SearchSelectionMode::Optional),
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

fn parse_kicked_multi_zone_search_to_battlefield_replacement_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    bundle_grammar::parse_kicked_doctors_replacement_shape(tokens)?;

    Some(vec![EffectAst::SelfReplacement {
        predicate: PredicateAst::ThisSpellWasKicked,
        if_true: search_library_and_graveyard_doctors_effects(Zone::Battlefield),
        if_false: search_library_and_graveyard_doctors_effects(Zone::Hand),
        attach_to_previous_ability: false,
    }])
}

fn parse_soul_partition_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    bundle_grammar::parse_soul_partition_shape(tokens)?;
    let sentences = split_lexed_sentences(tokens);
    let first_sentence = sentences.first()?;
    let mut effects = effect_sentences::parse_effect_sentences_lexed(first_sentence).ok()?;
    effects.push(EffectAst::subject_verb_grant_by_spec(
        crate::grant::GrantSpec::new(
            crate::grant::Grantable::play_from(),
            crate::filter::ObjectFilter::tagged(crate::cards::builders::TagKey::from(IT_TAG)),
            Zone::Exile,
        ),
        crate::cards::builders::PlayerAst::ItsOwner,
        crate::grant::GrantDuration::Forever,
    ));
    effects.push(EffectAst::subject_verb_grant_to_target(
        crate::cards::builders::TargetAst::Tagged(
            crate::cards::builders::TagKey::from(IT_TAG),
            None,
        ),
        crate::grant::Grantable::Ability(crate::static_abilities::StaticAbility::new(
            crate::static_abilities::CostIncreaseManaCost::new(
                crate::filter::ObjectFilter::spell()
                    .without_type(crate::types::CardType::Land)
                    .cast_by(crate::PlayerFilter::Opponent),
                crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(2)]),
            ),
        )),
        crate::grant::GrantDuration::Forever,
    ));
    Some(effects)
}

fn parse_empty_laboratory_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    bundle_grammar::parse_empty_laboratory_shape(tokens)?;

    let sacrificed_tag = TagKey::from("sacrificed_0");
    let revealed_tag = TagKey::from("etl_revealed");
    let matched_tag = TagKey::from("etl_matched");

    let mut zombie_you_control = ObjectFilter::creature().controlled_by(PlayerFilter::You);
    zombie_you_control.subtypes.push(Subtype::Zombie);

    let mut zombie_creature_card = ObjectFilter::creature();
    zombie_creature_card.subtypes.push(Subtype::Zombie);
    zombie_creature_card.zone = None;

    Some(vec![
        EffectAst::ChooseObjects {
            filter: zombie_you_control,
            count: ChoiceCount::dynamic_x(),
            count_value: None,
            player: PlayerAst::You,
            tag: sacrificed_tag.clone(),
        },
        EffectAst::subject_verb_sacrifice_all(PlayerAst::You, ObjectFilter::tagged(sacrificed_tag)),
        EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::You,
            crate::cards::builders::LibraryConsultModeAst::Reveal,
            zombie_creature_card,
            crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(
                crate::effect::Value::EventValue(crate::effect::EventValueSpec::Amount),
            ),
            revealed_tag.clone(),
            matched_tag.clone(),
        ),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(matched_tag.clone(), None),
            Zone::Battlefield,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            revealed_tag,
            Some(matched_tag),
            crate::cards::builders::LibraryBottomOrderAst::Random,
            PlayerAst::You,
        ),
    ])
}

fn parse_shape_anew_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    bundle_grammar::parse_shape_anew_shape(tokens)?;

    let revealed_tag = TagKey::from("shape_anew_revealed");
    let matched_tag = TagKey::from("shape_anew_matched");
    let mut artifact_card = ObjectFilter::artifact();
    artifact_card.zone = None;
    let target = TargetAst::Object(
        ObjectFilter::artifact().in_zone(Zone::Battlefield),
        Some(TextSpan::synthetic()),
        None,
    );

    Some(vec![
        EffectAst::subject_verb_sacrifice(
            PlayerAst::ItsController,
            ObjectFilter::default(),
            1,
            Some(target),
        ),
        EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::That,
            crate::cards::builders::LibraryConsultModeAst::Reveal,
            artifact_card,
            crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
            revealed_tag,
            matched_tag.clone(),
        ),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(matched_tag, None),
            Zone::Battlefield,
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

#[path = "bundle_rules/consult_bundles.rs"]
mod consult_bundles;
pub(super) use consult_bundles::parse_consult_disposition_bundle;
use consult_bundles::{
    parse_consult_then_put_matches_battlefield_rest_bottom_bundle,
    parse_reveal_repeated_disposition_bundle, parse_reveal_until_land_put_all_graveyard_bundle,
};

fn parse_tap_lands_then_empty_mana_pool_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    bundle_grammar::parse_tap_lands_empty_mana_shape(tokens)?;

    let mut lands = ObjectFilter::default();
    lands.zone = Some(Zone::Battlefield);
    lands.controller = Some(PlayerFilter::target_player());
    lands.card_types.push(CardType::Land);
    Some(vec![
        EffectAst::subject_verb_target_only(TargetAst::Player(
            PlayerFilter::Any,
            span_from_tokens(tokens),
        )),
        EffectAst::subject_verb_tap_all(lands),
        EffectAst::subject_verb_empty_mana_pool(PlayerAst::That),
    ])
}

fn parse_collision_of_realms_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    bundle_grammar::parse_collision_of_realms_shape(tokens)?;

    let mut owned_creatures = ObjectFilter::creature();
    owned_creatures.zone = Some(Zone::Battlefield);
    owned_creatures.owner = Some(PlayerFilter::IteratedPlayer);

    let mut owned_nontoken_creatures = owned_creatures.clone();
    owned_nontoken_creatures.nontoken = true;

    let mut tagged_library_filter = ObjectFilter::default();
    tagged_library_filter.zone = Some(Zone::Library);

    let mut creature_card = ObjectFilter::creature();
    creature_card.zone = None;

    let tagged_creatures = TagKey::from("collision_all_shuffled");
    let tagged_nontoken = TagKey::from("collision_nontoken_shuffled");
    let revealed_tag = TagKey::from("collision_revealed");
    let matched_tag = TagKey::from("collision_matched");

    Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_tag_matching_objects(
                owned_creatures.clone(),
                vec![Zone::Battlefield],
                tagged_creatures.clone(),
            ),
            EffectAst::subject_verb_tag_matching_objects(
                owned_nontoken_creatures,
                vec![Zone::Battlefield],
                tagged_nontoken.clone(),
            ),
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(tagged_creatures, None),
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
                    tag: tagged_nontoken,
                    filter: tagged_library_filter,
                },
                if_true: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::That,
                        LibraryConsultModeAst::Reveal,
                        creature_card,
                        LibraryConsultStopRuleAst::FirstMatch,
                        revealed_tag.clone(),
                        matched_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(matched_tag.clone(), None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                        revealed_tag,
                        Some(matched_tag),
                        LibraryBottomOrderAst::Random,
                        PlayerAst::That,
                    ),
                ],
                if_false: Vec::new(),
            },
        ],
    }])
}

fn parse_nissas_encouragement_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    bundle_grammar::parse_nissas_encouragement_shape(tokens)?;

    let searched_tag = TagKey::from("searched_named");
    let zones = vec![Zone::Library, Zone::Graveyard];
    let names = ["Forest", "Brambleweft Behemoth", "Nissa, Genesis Mage"];
    let mut effects = Vec::new();
    for name in names {
        let mut filter = ObjectFilter::default();
        filter.name = Some(name.to_string());
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: searched_tag.clone(),
            zones: zones.clone(),
            search_mode: Some(crate::effect::SearchSelectionMode::Exact),
        });
    }
    effects.push(EffectAst::subject_verb_reveal_tagged(searched_tag.clone()));
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(searched_tag, None),
        Zone::Hand,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::You,
        SubjectVerbActionAst::ShuffleLibrary,
    ));
    Some(effects)
}

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

fn parse_each_player_choose_unselected_bounce_then_draw_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    bundle_grammar::parse_each_player_bounce_draw_shape(tokens)?;

    let chosen_tag = TagKey::from("chosen_this_way");
    let mut effects = vec![
        EffectAst::ForEachPlayer {
            effects: vec![EffectAst::ChooseObjects {
                filter: ObjectFilter::nonland_permanent()
                    .controlled_by(PlayerFilter::IteratedPlayer),
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::Implicit,
                tag: chosen_tag.clone(),
            }],
        },
        EffectAst::subject_verb_return_all_to_hand(
            ObjectFilter::nonland_permanent().not_tagged(chosen_tag),
        ),
    ];
    effects.push(EffectAst::ForEachPlayersFiltered {
        filter: PlayerFilter::CardsInHandAtLeastMoreThanYou {
            base: Box::new(PlayerFilter::Opponent),
            count: 1,
        },
        effects: vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: Value::Fixed(1),
            },
        )],
    });
    Some(effects)
}

pub(crate) fn parse_exact_card_effect_bundle_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    if let Some(effects) = parse_consult_disposition_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_reveal_repeated_disposition_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_reveal_from_outside_game_to_hand(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_tap_lands_then_empty_mana_pool_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_soul_partition_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_empty_laboratory_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_shape_anew_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_reveal_until_land_put_all_graveyard_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_collision_of_realms_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_nissas_encouragement_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_bid_life_for_control_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_draw_create_treasure_lose_life_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) =
        parse_proliferate_then_choose_permanents_phase_out_single_sentence(tokens)
    {
        return Some(effects);
    }
    if let Some(effects) = parse_each_player_choose_unselected_bounce_then_draw_bundle(tokens) {
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
        && let Some(effects) =
            parse_proliferate_then_choose_permanents_phase_out_bundle(sentences[0], sentences[1])
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
    if let Some(effects) = parse_kicked_multi_zone_search_to_battlefield_replacement_bundle(tokens)
    {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_search_library_slots_to_hand_bundle(tokens) {
        return Some(effects);
    }
    match bundle_grammar::parse_special_exact_bundle_shape(tokens) {
        Some(bundle_grammar::SpecialExactBundleShape::ThassasOracle) => {
            let looked_tag = TagKey::from("thassas_oracle_looked");
            return Some(vec![
                EffectAst::subject_verb_look_at_top_cards(
                    PlayerAst::You,
                    Value::Devotion {
                        player: PlayerFilter::You,
                        color: crate::color::Color::Blue,
                    },
                    looked_tag.clone(),
                ),
                EffectAst::subject_verb_rearrange_looked_cards_in_library(
                    PlayerAst::You,
                    looked_tag,
                    ChoiceCount::up_to(1),
                ),
                EffectAst::Conditional {
                    predicate: crate::cards::builders::PredicateAst::ValueComparison {
                        left: Value::Devotion {
                            player: PlayerFilter::You,
                            color: crate::color::Color::Blue,
                        },
                        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::CardsInLibrary(PlayerFilter::You),
                    },
                    if_true: vec![EffectAst::subject_verb_win_game(PlayerAst::You)],
                    if_false: Vec::new(),
                },
            ]);
        }
        Some(bundle_grammar::SpecialExactBundleShape::GeistblastFromGraveyard) => {
            return Some(vec![EffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::ThisSpellWasCastFromZone(
                    Zone::Graveyard,
                ),
                if_true: vec![EffectAst::subject_verb_copy_spell(
                    TargetAst::Source(None),
                    Value::Fixed(1),
                    PlayerAst::Implicit,
                    true,
                    Vec::new(),
                )],
                if_false: Vec::new(),
            }]);
        }
        None => {}
    }

    None
}
