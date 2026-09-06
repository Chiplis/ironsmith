use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::LibraryActionAst;
use super::*;

pub fn parse_may_put_filtered_card_from_among_into_hand(
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

pub fn parse_delayed_dies_exile_top_power_choose_play(
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
            tag: looked_tag.clone().into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Ok(Some(vec![EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectDiesThisTurn {
        filter: None,
        effects: vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                Value::PowerOf(Box::new(ChooseSpec::Tagged(
                    (crate::tag::CompilerReferenceTag::It.bind()).into(),
                )))
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
                crate::tag::TagRef::of(looked_tag.clone()),
            ),
            EffectAst::subject_verb_exile(TargetAst::Tagged(crate::tag::TagRef::of(looked_tag), None), false),
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter: exiled_filter,
                count: ChoiceCount::exactly(1),
                player: PlayerAst::You,
                tag: crate::tag::TagRef::of(chosen_tag.clone()),
                zone: Zone::Exile,
            }),
            EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                crate::tag::TagRef::of(chosen_tag),
                PlayerAst::You,
                true,
                false,
            ),
        ],
    })]))
}

pub fn parse_mill_then_may_put_from_among_into_hand(
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
        parse_put_from_milled_cards_followup(second, player, milled_tag.key.clone())?
    else {
        return Ok(None);
    };

    if !conditional_followup && append_to_outer_if_result(&mut first_effect, &mut followup) {
        return Ok(Some(vec![first_effect]));
    }

    let mut effects = vec![first_effect];
    if conditional_followup {
        effects.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: followup,
        }));
    } else {
        effects.extend(followup);
    }
    Ok(Some(effects))
}

pub(crate) fn tag_single_mill_effect(effect: &mut EffectAst, tag: &TagKey) -> Option<PlayerAst> {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::Library(LibraryActionAst::Mill { .. }),
    }) = effect
    {
        let player = *player;
        let mill = effect.clone();
        *effect = EffectAst::TagAffected {
            effect: Box::new(mill),
            tag: crate::tag::TagRef::of(tag.clone()),
        };
        return Some(player);
    }

    let nested = match effect {
        EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })
        | EffectAst::Conditionals(ConditionalEffectAst::ResolvedIfResult { effects, .. })
        | EffectAst::Sequence { effects } => effects,
        _ => return None,
    };
    let [nested] = nested.as_mut_slice() else {
        return None;
    };
    tag_single_mill_effect(nested, tag)
}

pub(super) fn milled_choice_filter_branches(filter: &ObjectFilter) -> Option<Vec<ObjectFilter>> {
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

pub(crate) fn parse_put_from_milled_cards_followup(
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
    let Some(action_match) = sentence_markers::parse_leading_may_action_tokens(
        &action_sentence,
        &["put", "return"],
        true,
    ) else {
        return Ok(None);
    };
    let chooser = leading_may_actor_to_player(action_match.actor, default_player);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let Some((
        mut choice_count,
        mut filter,
        aggregate_constraint,
        zone,
        controller,
        tapped,
        attacking,
        attack_target_player,
        all_matching,
    )) = super::super::ordered_control_flow_programs::parse_counted_from_looked_cards_action(
        &action_tokens,
    )
    else {
        return Ok(None);
    };
    if aggregate_constraint.is_some() {
        return Ok(None);
    }
    if action_match.actor != LeadingMayActor::Default && choice_count == ChoiceCount::exactly(1) {
        choice_count = ChoiceCount::up_to(1);
    }
    if action_tokens.iter().any(|token| token.is_word("milled")) {
        filter.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Milled));
    }

    let chosen_tag = helper_tag_for_tokens(tokens, "chosen_milled");
    if all_matching {
        filter.zone = None;
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: milled_tag,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        let mut move_effect = EffectAst::subject_verb_move_to_zone_with_attack_target(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
            zone,
            false,
            controller,
            tapped,
            attacking,
            attack_target_player,
            false,
            None,
        );
        if action_match.verb == "return" {
            move_effect = move_effect
                .with_move_to_zone_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return);
        }
        return Ok(Some((
            vec![
                EffectAst::subject_verb_tag_matching_objects(
                    filter,
                    vec![Zone::Graveyard],
                    crate::tag::TagRef::of(chosen_tag.clone()),
                ),
                EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                    tag: crate::tag::TagRef::of(chosen_tag),
                    effects: vec![move_effect],
                }),
            ],
            conditional_followup,
        )));
    }
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
                tag: chosen_tag.clone().into(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
            effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter: branch,
                count: ChoiceCount::up_to(1),
                player: chooser,
                tag: crate::tag::TagRef::of(chosen_tag.clone()),
                zone: Zone::Graveyard,
            }));
        }
    } else {
        filter.zone = Some(Zone::Graveyard);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: milled_tag,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: choice_count,
            player: chooser,
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
            zone: Zone::Graveyard,
        }));
    }
    let mut move_effect = EffectAst::subject_verb_move_to_zone_with_attack_target(
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
        zone,
        false,
        controller,
        tapped,
        attacking,
        attack_target_player,
        false,
        None,
    );
    if action_match.verb == "return" {
        move_effect = move_effect
            .with_move_to_zone_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return);
    }
    effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::TagRef::of(chosen_tag),
        effects: vec![move_effect],
    }));
    Ok(Some((effects, conditional_followup)))
}

/// Shared body for the mill-then-choose follow-up, parameterized by the
/// optional "if you don't" branch so both the bare and the if-you-don't
/// callers compose the same reusable primitive sequence (mirroring the retired
/// `ChooseFromLookedCardsIntoHandRestIntoGraveyard` recipe). The milled cards
/// already sit in the graveyard, so the choose filter references them via
/// `crate::tag::CompilerReferenceTag::It.as_str()` (resolved to the mill's collection tag at lowering) and no
/// rest-into-graveyard split is emitted.
pub fn parse_mill_then_may_put_from_among_into_hand_with_if_not_chosen(
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
            action: SubjectVerbActionAst::Library(LibraryActionAst::Mill { .. }),
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
        super::super::ordered_control_flow_programs::compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
            chooser,
            filter,
            (crate::tag::CompilerReferenceTag::It.bind()).into(),
            chosen_tag.key.clone(),
            Zone::Graveyard,
            false,
            if_not_chosen,
            choice_count,
        ),
    );
    Ok(Some(effects))
}

pub fn parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    effect_sentences::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    )
}

pub fn parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((PlayerAst::You, count, true)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
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
pub(super) fn compose_reveal_top_put_matching_into_hand_rest_on_bottom(
    look_tokens: &[OwnedLexToken],
    matched_tokens: &[OwnedLexToken],
    count: Value,
    mut filter: ObjectFilter,
    order: LibraryBottomOrderAst,
) -> Vec<EffectAst> {
    let looked_tag = helper_tag_for_tokens(look_tokens, "revealed");
    let matched_tag = helper_tag_for_tokens(matched_tokens, "matched");
    filter.zone = None;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone().into(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    vec![
        EffectAst::subject_verb_look_at_top_cards(PlayerAst::You, count, crate::tag::TagRef::of(looked_tag.clone())),
        EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(looked_tag.clone())),
        EffectAst::subject_verb_tag_matching_objects(
            filter,
            vec![Zone::Library],
            crate::tag::TagRef::of(matched_tag.clone()),
        ),
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::TagRef::of(matched_tag.clone()),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        }),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            crate::tag::TagRef::of(looked_tag),
            Some(crate::tag::TagRef::of(matched_tag)),
            order,
            PlayerAst::You,
        ),
    ]
}

/// Composes the "reveal top N, put matching into hand, rest into graveyard" shape:
/// look + reveal-tagged + per-looked-card conditional split (matches filter -> hand,
/// else -> graveyard), matching the retired
/// `RevealTopPutMatchingIntoHandRestIntoGraveyard` recipe's lowering.
pub(super) fn compose_reveal_top_put_matching_into_hand_rest_into_graveyard(
    look_tokens: &[OwnedLexToken],
    count: Value,
    mut filter: ObjectFilter,
) -> Vec<EffectAst> {
    let looked_tag = helper_tag_for_tokens(look_tokens, "revealed");
    filter.zone = None;
    let iterated = || TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
    vec![
        EffectAst::subject_verb_look_at_top_cards(PlayerAst::You, count, crate::tag::TagRef::of(looked_tag.clone())),
        EffectAst::subject_verb_reveal_tagged(crate::tag::TagRef::of(looked_tag.clone())),
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::TagRef::of(looked_tag),
            effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    filter,
                ),
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
            })],
        }),
    ]
}

pub fn parse_consult_match_move_and_bottom_remainder(
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
            action: SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary { .. }),
            ..
        }))
    ) {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    if let Some(matched) = effect_grammar::parse_consult_matched_move_shape(&second_tokens)
        && matched.selection == effect_grammar::ConsultMoveSelectionShape::AllMatched
    {
        let followups = vec![
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::TagRef::of(parts.match_tag.clone()), None),
                matched.zone,
                false,
                if matched.controller_you {
                    ReturnControllerAst::You
                } else {
                    ReturnControllerAst::Preserve
                },
                false,
                None,
            )
            .with_move_to_zone_plural_surface_if(matched.target_plural_surface),
        ];
        return Ok(Some(wrap_optional_consult_effects(
            parts, optional, followups, false, false,
        )));
    }
    let Some(shape) = effect_grammar::parse_consult_move_bottom_shape(&second_tokens) else {
        return Ok(None);
    };
    if shape == effect_grammar::ConsultMoveBottomShape::MatchedToBattlefieldAndShuffle {
        let remainder = TargetAst::Object(
            ObjectFilter::tagged(parts.all_tag.clone()).not_tagged(parts.match_tag.clone()),
            None,
            None,
        );
        let followups = vec![
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::TagRef::of(parts.match_tag.clone()), None),
                Zone::Battlefield,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )
            .with_move_to_zone_plural_surface(),
            EffectAst::subject_verb_shuffle_objects_into_library(parts.player, remainder),
        ];
        return Ok(Some(wrap_optional_consult_effects(
            parts, optional, followups, false, false,
        )));
    }

    let effect_grammar::ConsultMoveBottomShape::MoveMatchAndBottom {
        zone,
        battlefield_tapped,
        attached_to_tokens,
        order,
    } = shape
    else {
        unreachable!("shuffle consult shape returned above")
    };

    let followups = vec![
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::TagRef::of(parts.match_tag.clone()), None),
            zone,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            battlefield_tapped,
            attached_to_tokens.map(|(start, end)| crate::util::parse_target_phrase(&second_tokens[start..end])).transpose()?,
        ),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            crate::tag::TagRef::of(parts.all_tag.clone()),
            Some(crate::tag::TagRef::of(parts.match_tag.clone())),
            order,
            PlayerAst::That,
        ),
    ];
    Ok(Some(wrap_optional_consult_effects(
        parts, optional, followups, false, false,
    )))
}
