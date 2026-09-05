use super::*;

/// Keeps the original looked-card pool authoritative when an intervening
/// optional sacrifice establishes a newer last-object reference:
///
/// "Look at ... . You may sacrifice ... . If you do, you may put a card from
/// among those cards onto the battlefield. Put the rest on the bottom ... ."
///
/// The dynamic selection filter may still refer to the sacrificed object (for
/// example through X), but its candidate domain is explicitly the earlier
/// `looked_tag`; the complement is likewise computed from that tag.
pub fn parse_look_then_may_sacrifice_if_did_select_battlefield_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let Some((library_owner, count, false)) =
        effect_sentences::parse_top_cards_view_sentence(&first_tokens)
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let sacrifice_tokens = strip_leading_token_words_any(&second_tokens, &["then"]);
    let Ok(sacrifice_effects) = effect_sentences::parse_effect_sentence_lexed(sacrifice_tokens)
    else {
        return Ok(None);
    };
    if !sacrifice_effects.iter().any(effect_ast_contains_sacrifice) {
        return Ok(None);
    }

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let Some(followup) =
        crate::grammar::sentence_markers::parse_conditional_followup_tokens(&third_tokens)
    else {
        return Ok(None);
    };
    if followup.actor != crate::grammar::sentence_markers::ConditionalFollowupActor::You {
        return Ok(None);
    }
    let where_x_at = crate::slice_primitives::select_position(followup.tail_tokens, |token| {
        token.is_word("where")
    });
    let action_tokens = trim_commas(
        where_x_at
            .and_then(|idx| followup.tail_tokens.get(..idx))
            .unwrap_or(followup.tail_tokens),
    );
    let Some((chooser, mut filter, tapped)) =
        effect_sentences::parse_may_put_filtered_looked_card_onto_battlefield(&action_tokens)?
    else {
        return Ok(None);
    };
    if let Some(where_x_at) = where_x_at {
        let where_x_tokens = trim_commas(&followup.tail_tokens[where_x_at..]);
        let Some(x_value) = crate::keyword_static::parse_value_binding_clause(&where_x_tokens)
        else {
            return Ok(None);
        };
        let Some(crate::filter::Comparison::LessThanOrEqualExpr(maximum)) =
            filter.mana_value.as_mut()
        else {
            return Ok(None);
        };
        **maximum = crate::util::replace_unbound_x_with_value(
            (**maximum).clone(),
            &x_value,
            "looked-card selection after an intervening action",
        )?;
    }

    let remainder_tokens = trim_commas(sentences[sentence_idx + 3].lowered());
    if !is_put_rest_on_bottom_of_library_sentence(&remainder_tokens) {
        return Ok(None);
    }
    let Some(order) = crate::grammar::effects::parse_bottom_order(&remainder_tokens) else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(&first_tokens, "looked_before_sacrifice");
    let selected_tag = helper_tag_for_tokens(&third_tokens, "selected_after_sacrifice");
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        library_owner,
        count,
        looked_tag.clone(),
    )];
    effects.extend(sacrifice_effects);
    effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count: ChoiceCount::up_to(1),
                player: chooser,
                tag: selected_tag.clone(),
                zone: Zone::Library,
            },
            EffectAst::ForEachTagged {
                tag: selected_tag.clone(),
                effects: vec![EffectAst::subject_verb_put_onto_battlefield(
                    chooser,
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    tapped,
                    ReturnControllerAst::Preserve,
                )],
            },
        ],
    });
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag),
            order,
            library_owner,
        ),
    );
    Ok(Some(effects))
}

pub(super) fn parse_selected_card_leading_if(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ObjectFilter, Vec<EffectAst>)>, CardTextError> {
    let Some((condition_tokens, action_tokens)) =
        crate::grammar::primitives::split_lexed_once_on_comma(tokens)
    else {
        return Ok(None);
    };
    let condition_tokens = trim_commas(condition_tokens);
    let descriptor_tokens = if condition_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
        && condition_tokens
            .get(1)
            .is_some_and(|token| token.is_word("it's") || token.is_word("it’s"))
    {
        condition_tokens.get(2..).unwrap_or_default()
    } else if condition_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
        && condition_tokens
            .get(1)
            .is_some_and(|token| token.is_word("it"))
        && condition_tokens
            .get(2)
            .is_some_and(|token| token.is_word("is"))
    {
        condition_tokens.get(3..).unwrap_or_default()
    } else {
        return Ok(None);
    };
    if descriptor_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = parse_object_filter_lexed(descriptor_tokens, false)?;
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let action_tokens = trim_commas(action_tokens);
    let effects = effect_sentences::parse_effect_sentence_lexed(&action_tokens)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some((filter, effects)))
}

/// Composes two independent optional selections from one public looked-card
/// pool and computes the graveyard group as the exact complement of both
/// selected tags.
pub fn parse_reveal_top_optional_battlefield_then_hand_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, true)) =
        effect_sentences::parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(battlefield_action) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["put"], false)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(battlefield_action.actor, player);
    let Some((
        mut battlefield_count,
        mut battlefield_filter,
        None,
        Zone::Battlefield,
        battlefield_controller,
        battlefield_tapped,
        battlefield_attacking,
        battlefield_attack_target,
        false,
    )) = super::super::ordered_control_flow_programs::parse_counted_from_looked_cards_action(
        battlefield_action.tail_tokens,
    )
    else {
        return Ok(None);
    };
    if battlefield_count.min > 0 {
        battlefield_count =
            ChoiceCount::up_to(battlefield_count.max.unwrap_or(battlefield_count.min));
    }
    let battlefield_entry_counter =
        triple_grammar::parse_looked_move_action_shape(battlefield_action.tail_tokens)
            .and_then(|shape| shape.entry_counter);

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let Some(hand_action) =
        sentence_markers::parse_leading_may_action_tokens(&third_tokens, &["put"], false)
    else {
        return Ok(None);
    };
    let hand_chooser = effect_sentences::leading_may_actor_to_player(hand_action.actor, player);
    let Some((
        mut hand_count,
        mut hand_filter,
        None,
        Zone::Hand,
        ReturnControllerAst::Preserve,
        false,
        false,
        None,
        false,
    )) = super::super::ordered_control_flow_programs::parse_counted_from_looked_cards_action(
        hand_action.tail_tokens,
    )
    else {
        return Ok(None);
    };
    if hand_chooser != chooser {
        return Ok(None);
    }
    if hand_count.min > 0 {
        hand_count = ChoiceCount::up_to(hand_count.max.unwrap_or(hand_count.min));
    }
    let Some(triple_grammar::LookedRemainderShape::Graveyard) =
        triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 3].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "revealed");
    let battlefield_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "battlefield");
    let hand_tag = helper_tag_for_tokens(sentences[sentence_idx + 2].lowered(), "hand");
    let remainder_tag = helper_tag_for_tokens(sentences[sentence_idx + 3].lowered(), "remainder");
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
    let mut remainder_filter = ObjectFilter::tagged(looked_tag.clone());
    remainder_filter = remainder_filter
        .not_tagged(battlefield_tag.clone())
        .not_tagged(hand_tag.clone());
    let iterated = || TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
    let mut battlefield_effects = vec![EffectAst::subject_verb_move_to_zone_with_attack_target(
        iterated(),
        Zone::Battlefield,
        false,
        battlefield_controller,
        battlefield_tapped,
        battlefield_attacking,
        battlefield_attack_target,
        false,
        None,
    )];
    if let Some((amount, counter_type)) = battlefield_entry_counter {
        battlefield_effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            crate::effect::Value::Fixed(amount as i32),
            iterated(),
            None,
            false,
        ));
    }

    Ok(Some(vec![
        EffectAst::subject_verb_reveal_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: battlefield_filter,
            count: battlefield_count,
            player: chooser,
            tag: battlefield_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::ForEachTagged {
            tag: battlefield_tag,
            effects: battlefield_effects,
        },
        EffectAst::ChooseTaggedObjectsInZone {
            filter: hand_filter,
            count: hand_count,
            player: chooser,
            tag: hand_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::ForEachTagged {
            tag: hand_tag,
            effects: vec![EffectAst::subject_verb_move_to_zone(
                iterated(),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::subject_verb_tag_matching_objects(
            remainder_filter,
            vec![Zone::Library],
            remainder_tag.clone(),
        ),
        EffectAst::MoveTaggedGroupToZone {
            tag: remainder_tag,
            zone: Zone::Graveyard,
        },
    ]))
}

/// Composes the "look at the top N, you may put a matching card onto the
/// battlefield; if you don't, put a card into your hand; put the rest on the
/// bottom" shape from reusable primitives, mirroring the runtime effects the
/// retired `ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary`
/// recipe lowered to:
/// - look at the top N (minting an explicit `looked_tag`),
/// - choose up to one matching looked card (`battlefield_tag`),
/// - under an internal effect id, for each chosen card put it onto the
///   battlefield; if that did not happen, choose exactly one looked card and
///   move it to hand (`hand_tag`),
/// - for each looked card not chosen for battlefield or hand, move it to the
///   bottom of the library.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compose_look_at_top_may_put_onto_battlefield_or_into_hand_rest_bottom(
    look_tokens: &[OwnedLexToken],
    choose_tokens: &[OwnedLexToken],
    look_player: PlayerAst,
    count: crate::effect::Value,
    reveal: bool,
    chooser: PlayerAst,
    mut battlefield_filter: ObjectFilter,
    tapped: bool,
) -> Vec<EffectAst> {
    let looked_tag = helper_tag_for_tokens(look_tokens, if reveal { "revealed" } else { "looked" });
    let battlefield_tag = helper_tag_for_tokens(choose_tokens, "chosen");
    let hand_tag = helper_tag_for_tokens(choose_tokens, "chosen_hand");

    battlefield_filter.zone = Some(Zone::Library);
    battlefield_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);

    let it = || TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
    let mut in_battlefield_choice_filter = ObjectFilter::default();
    in_battlefield_choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::It.bind(),
            relation: TaggedOpbjectRelation::SameStableId,
        });
    let mut in_hand_choice_filter = ObjectFilter::default();
    in_hand_choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::It.bind(),
            relation: TaggedOpbjectRelation::SameStableId,
        });

    let mut look =
        EffectAst::subject_verb_look_at_top_cards(look_player, count, looked_tag.clone());
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards { reveal: r, .. },
        ..
    }) = &mut look
    {
        *r = reveal;
    }

    vec![
        look,
        EffectAst::ChooseTaggedObjectsInZone {
            filter: battlefield_filter,
            count: ChoiceCount::up_to(1),
            player: chooser,
            tag: battlefield_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::IfEffectDidNotHappen {
            effect: Box::new(EffectAst::ForEachTagged {
                tag: battlefield_tag.clone(),
                effects: vec![EffectAst::subject_verb_put_onto_battlefield(
                    chooser,
                    it(),
                    tapped,
                    ReturnControllerAst::Preserve,
                )],
            }),
            otherwise: vec![
                EffectAst::ChooseTaggedObjectsInZone {
                    filter: hand_filter,
                    count: ChoiceCount::exactly(1),
                    player: chooser,
                    tag: hand_tag.clone(),
                    zone: Zone::Library,
                },
                EffectAst::ForEachTagged {
                    tag: hand_tag.clone(),
                    effects: vec![EffectAst::subject_verb_move_to_zone(
                        it(),
                        Zone::Hand,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                },
            ],
        },
        EffectAst::ForEachTagged {
            tag: looked_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(
                    battlefield_tag,
                    in_battlefield_choice_filter,
                ),
                if_true: Vec::new(),
                if_false: vec![EffectAst::Conditional {
                    predicate: PredicateAst::TaggedMatches(hand_tag, in_hand_choice_filter),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        it(),
                        Zone::Library,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                }],
            }],
        },
    ]
}

/// "you may exile a <filter> card from among them" — the optional single-card
/// exile pick from a previously looked-at set.
pub(crate) fn parse_may_exile_filtered_looked_card(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let mut filter = if let Some(shape) = quad_grammar::parse_may_exile_looked_card_shape(tokens) {
        let Some(filter) = effect_sentences::parse_looked_card_choice_filter(shape.filter_tokens)
        else {
            return Ok(None);
        };
        filter
    } else {
        let words = crate::lexer::token_word_refs(tokens);
        if !crate::word_primitives::parse_any_sequence_complete(
            &words,
            &[
                &["you", "may", "exile", "one", "of", "those", "cards"],
                &["you", "may", "exile", "one", "of", "them"],
            ],
        ) {
            return Ok(None);
        }
        ObjectFilter::default()
    };
    filter.zone = Some(Zone::Library);
    Ok(Some(filter))
}

