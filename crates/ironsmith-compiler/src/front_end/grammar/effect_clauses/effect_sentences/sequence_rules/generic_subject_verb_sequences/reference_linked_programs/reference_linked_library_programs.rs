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

/// Bind "from among them" to the exact cards affected by the immediately
/// preceding mill instruction. The up-to-one tagged choice is both the
/// optionality and the executable provenance boundary; unrelated cards that
/// were already in the graveyard cannot be chosen.
pub fn parse_mill_then_may_cast_from_among(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = crate::util::trim_edge_punctuation_tokens(sentences[sentence_idx].lowered());
    let second_tokens = sentences[sentence_idx + 1].lowered();
    let second_words = crate::lexer::token_word_refs(second_tokens);
    let maximum_mana_value = if crate::word_primitives::parse_sequence_complete(
        &second_words,
        &[
            "you", "may", "cast", "an", "instant", "or", "sorcery", "spell", "from", "among",
            "them", "without", "paying", "its", "mana", "cost",
        ],
    ) {
        None
    } else if crate::word_primitives::parse_sequence_complete(
        &second_words,
        &[
            "you", "may", "cast", "an", "instant", "or", "sorcery", "spell", "with", "mana",
            "value", "x", "or", "less", "from", "among", "them", "without", "paying", "its",
            "mana", "cost",
        ],
    ) {
        Some(Value::X)
    } else {
        return Ok(None);
    };

    let Ok(mut effects) = effect_sentences::parse_effect_sentence_lexed(first_tokens)
        .or_else(|_| effect_sentences::parse_effect_chain(first_tokens))
    else {
        return Ok(None);
    };
    let allowed_prefix = |effect: &EffectAst| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::TargetOnly { .. },
                ..
            })
        )
    };
    let mill_index = effects
        .iter()
        .enumerate()
        .filter_map(|(index, effect)| {
            let mut probe = effect.clone();
            tag_single_mill_effect(
                &mut probe,
                &crate::tag::CompilerReferenceTag::MillProbe.key(),
            )
            .map(|_| index)
        })
        .collect::<Vec<_>>();
    let [mill_index] = mill_index.as_slice() else {
        return Ok(None);
    };
    if effects
        .iter()
        .enumerate()
        .any(|(index, effect)| index != *mill_index && !allowed_prefix(effect))
    {
        return Ok(None);
    }

    let milled_tag = helper_tag_for_tokens(first_tokens, "milled_castable");
    let Some(_milled_player) = tag_single_mill_effect(&mut effects[*mill_index], &milled_tag)
    else {
        return Ok(None);
    };
    let chosen_tag = helper_tag_for_tokens(second_tokens, "chosen_milled_castable");
    let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    if let Some(maximum) = maximum_mana_value {
        let mut instant = ObjectFilter::default();
        instant.card_types = vec![CardType::Instant];
        let mut sorcery = ObjectFilter::default();
        sorcery.card_types = vec![CardType::Sorcery];
        let comparison = crate::filter::Comparison::LessThanOrEqualExpr(Box::new(maximum));
        instant.mana_value = Some(comparison.clone());
        sorcery.mana_value = Some(comparison);
        filter.any_of = vec![instant, sorcery];
    } else {
        filter.card_types = vec![CardType::Instant, CardType::Sorcery];
    }
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: milled_tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: ChoiceCount::up_to(1),
        player: PlayerAst::You,
        tag: chosen_tag.clone(),
        zone: Zone::Graveyard,
    });
    effects.push(EffectAst::subject_verb_cast_tagged(
        chosen_tag,
        PlayerAst::You,
        false,
        false,
        true,
        None,
    ));
    Ok(Some(effects))
}

pub(in super::super) fn tag_single_mill_effect(
    effect: &mut EffectAst,
    tag: &TagKey,
) -> Option<PlayerAst> {
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

pub(super) fn parse_put_from_milled_cards_followup(
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
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
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
                    chosen_tag.clone(),
                ),
                EffectAst::ForEachTagged {
                    tag: chosen_tag,
                    effects: vec![move_effect],
                },
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
    let mut move_effect = EffectAst::subject_verb_move_to_zone_with_attack_target(
        TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
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
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag,
        effects: vec![move_effect],
    });
    Ok(Some((effects, conditional_followup)))
}

pub fn parse_top_cards_put_any_matching_to_zone_rest_same_sentence(
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
        controller,
        tapped,
        attacking,
        attack_target_player,
        all_matching,
    )) = super::super::ordered_control_flow_programs::parse_counted_from_looked_cards_action(
        action_match.tail_tokens,
    )
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
            controller,
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
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
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
pub fn parse_optional_look_then_reveal_put_top_rest_bottom(
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
        super::super::ordered_control_flow_programs::compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
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
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    vec![
        EffectAst::subject_verb_look_at_top_cards(PlayerAst::You, count, looked_tag.clone()),
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
pub(super) fn compose_reveal_top_put_matching_into_hand_rest_into_graveyard(
    look_tokens: &[OwnedLexToken],
    count: Value,
    mut filter: ObjectFilter,
) -> Vec<EffectAst> {
    let looked_tag = helper_tag_for_tokens(look_tokens, "revealed");
    filter.zone = None;
    let iterated = || TargetAst::Tagged(TagKey::from(IT_TAG), None);
    vec![
        EffectAst::subject_verb_look_at_top_cards(PlayerAst::You, count, looked_tag.clone()),
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
            action: SubjectVerbActionAst::ConsultTopOfLibrary { .. },
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
                TargetAst::Tagged(parts.match_tag.clone(), None),
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
        let followups = vec![
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(parts.match_tag.clone(), None),
                Zone::Battlefield,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                parts.player,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
        ];
        return Ok(Some(wrap_optional_consult_effects(
            parts, optional, followups, false, false,
        )));
    }

    let effect_grammar::ConsultMoveBottomShape::MoveMatchAndBottom {
        zone,
        battlefield_tapped,
        order,
    } = shape
    else {
        unreachable!("shuffle consult shape returned above")
    };

    let followups = vec![
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag.clone(), None),
            zone,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            battlefield_tapped,
            None,
        ),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            parts.all_tag.clone(),
            Some(parts.match_tag.clone()),
            order,
            parts.player,
        ),
    ];
    Ok(Some(wrap_optional_consult_effects(
        parts, optional, followups, false, false,
    )))
}

pub fn parse_conditional_consult_match_move_and_bottom_remainder(
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
