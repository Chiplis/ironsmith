use super::*;

pub(super) fn parse_choose_from_looked_cards_for_each_filter(
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

pub fn parse_top_cards_choose_for_each_filter_one_battlefield_others_hand_rest_graveyard(
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
            // Each authored `a card with <keyword>` slot is mandatory when a
            // matching revealed card exists. Runtime choice bounds naturally
            // collapse exact-one to zero when that slot has no candidates.
            count: ChoiceCount::exactly(1),
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
        // "Put one of the chosen cards" is likewise mandatory whenever the
        // preceding keyword slots produced at least one card.
        count: ChoiceCount::exactly(1),
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
                crate::tag::CompilerReferenceTag::It.key(),
                ObjectFilter::tagged(battlefield_tag.clone()),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
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
                crate::tag::CompilerReferenceTag::It.key(),
                ObjectFilter::tagged(chosen_tag),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
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

pub fn parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(
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
    let mut spell_filter =
        crate::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed(
            &filter_prefix_tokens,
        );
    spell_filter.zone = Some(Zone::Stack);
    spell_filter.has_mana_cost = true;

    let Some(order) =
        triple_grammar::parse_card_type_iteration_order(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "revealed");
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut effects = vec![EffectAst::subject_verb_reveal_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
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

pub fn parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom(
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
    let mut effects = vec![EffectAst::subject_verb_reveal_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
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
                CardType::Kindred,
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
pub(crate) fn compose_choose_from_looked_cards_onto_battlefield_and_into_hand_rest_on_bottom(
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

pub fn parse_look_at_top_reveal_match_put_rest_bottom(
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
    let selected_reference_surface = {
        let words = TokenWordView::new(&reveal_tokens).word_refs();
        if crate::word_primitives::sequence_occurs(&words, &["that", "card"]) {
            ironsmith_core::SearchResultReferenceSurface::ThatCard
        } else {
            ironsmith_core::SearchResultReferenceSurface::It
        }
    };
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
        effects: vec![EffectAst::subject_verb_reveal_tagged(
            crate::tag::CompilerReferenceTag::It.key(),
        )],
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
                Zone::Hand,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )
            .with_move_to_zone_target_reference_surface(selected_reference_surface),
        ],
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

