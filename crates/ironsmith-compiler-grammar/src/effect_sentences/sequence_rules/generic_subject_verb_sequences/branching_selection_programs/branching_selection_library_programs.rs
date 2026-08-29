use super::*;

/// Preserves a conditional selected subset and one exact remainder across the
/// common four-sentence shape:
///
/// "Look at ... . If <predicate>, put N of those cards into your hand.
/// Otherwise, put M of them into your hand. Put the rest on the bottom ... ."
///
/// Both branches deliberately write the same `selected_tag`.  The final
/// remainder can therefore be expressed once as `looked - selected` instead
/// of taking whichever branch's last-object reference happened to survive
/// conditional lowering.
pub fn parse_look_at_top_conditional_hand_counts_then_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let Some((library_owner, count, false)) =
        effect_sentences::parse_top_cards_view_sentence(&first_tokens)
    else {
        return Ok(None);
    };

    let conditional_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    // Reuse the ordinary conditional sentence parser for the predicate.  It
    // already understands comparative controller predicates such as
    // "you control more creatures than each other player"; the standalone
    // predicate grammar is intentionally narrower and caused this sequence
    // rule to fall through even though the sentence parsed successfully on
    // its own.
    let Ok(parsed_conditional) = effect_sentences::parse_effect_sentence_lexed(&conditional_tokens)
    else {
        return Ok(None);
    };
    let [EffectAst::Conditional { predicate, .. }] = parsed_conditional.as_slice() else {
        return Ok(None);
    };
    let predicate = predicate.clone();
    // Sentence splitting removes the period and normalizes away the comma
    // between the condition and its action.  The consult-family conditional
    // splitter intentionally requires that punctuation boundary, so using it
    // here made otherwise-valid conditional sentences unreachable.  The
    // ordinary conditional parser above proves the sentence shape; locate the
    // branch's leading action verb to preserve the exact counted selection.
    let Some(if_true_start) =
        crate::slice_primitives::select_last_position(&conditional_tokens, |token| {
            token.is_word("put")
        })
    else {
        return Ok(None);
    };
    let if_true_tokens = trim_commas(&conditional_tokens[if_true_start..]);
    let Some(if_true_count) = parse_counted_looked_cards_into_your_hand_tokens(&if_true_tokens)
    else {
        return Ok(None);
    };

    let otherwise_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let if_false_tokens = strip_leading_token_words_any(&otherwise_tokens, &["otherwise"]);
    let Some(if_false_count) = parse_counted_looked_cards_into_your_hand_tokens(if_false_tokens)
    else {
        return Ok(None);
    };

    let remainder_tokens = trim_commas(sentences[sentence_idx + 3].lowered());
    let remainder_tokens = strip_leading_token_words_any(&remainder_tokens, &["then", "and"]);
    if !is_put_rest_on_bottom_of_library_sentence(remainder_tokens) {
        return Ok(None);
    }
    let Some(order) = crate::grammar::effects::parse_bottom_order(remainder_tokens) else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(&first_tokens, "looked_conditional_partition");
    let selected_tag = helper_tag_for_tokens(&conditional_tokens, "conditional_selected");
    let choice = |count: u32| {
        let mut filter = ObjectFilter::tagged(looked_tag.clone());
        filter.zone = Some(Zone::Library);
        vec![
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count: ChoiceCount::exactly(count as usize),
                player: PlayerAst::You,
                tag: selected_tag.clone(),
                zone: Zone::Library,
            },
            EffectAst::MoveTaggedGroupToZone {
                tag: selected_tag.clone(),
                zone: Zone::Hand,
            },
        ]
    };

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(library_owner, count, looked_tag.clone()),
        EffectAst::Conditional {
            predicate,
            if_true: choice(if_true_count),
            if_false: choice(if_false_count),
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag),
            order,
            library_owner,
        ),
    ]))
}

/// Preserves one looked-card producer and selected subset across a conditional
/// disposition of the exact complement:
///
/// "Look at ... . You may put a matching card ... onto the battlefield.
/// Then if <predicate>, put the rest into your hand. Otherwise, put the rest
/// on the bottom ... ."
///
/// Parsing either conditional branch in isolation makes `the rest` vulnerable
/// to whichever implicit object tag was most recently established. Build the
/// selection once, then give both branches the same looked-minus-selected
/// operands.
pub fn parse_look_at_top_optional_battlefield_then_conditional_remainder(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Keep authored number/type words intact. A card name can itself install
    // a short source alias matching one of those words (for example `Nine`
    // in Nine-Fingers Keene), and the normalized view would otherwise turn
    // the threshold and look count into a source reference.
    let conditional_tokens = trim_commas(sentences[sentence_idx + 2].lexed());
    // This specialist owns only the authored remainder branch.  Merely
    // recognizing a conditional here is not enough: conditional entry
    // modifiers such as Turntimber Symbiosis also follow an optional
    // looked-card move, and used to be rewritten into an invented
    // "put the rest into your hand" branch.
    let has_remainder_to_hand =
        crate::grammar::effects::parse_remainder_to_hand_presence(&conditional_tokens);
    if !has_remainder_to_hand {
        return Ok(None);
    }
    let Ok(parsed_conditional) = effect_sentences::parse_effect_sentence_lexed(&conditional_tokens)
    else {
        return Ok(None);
    };
    let [EffectAst::Conditional { predicate, .. }] = parsed_conditional.as_slice() else {
        return Ok(None);
    };
    let predicate = predicate.clone();

    let otherwise_tokens = trim_commas(sentences[sentence_idx + 3].lexed());
    if !otherwise_tokens
        .first()
        .is_some_and(|token| token.is_word("otherwise"))
    {
        return Ok(None);
    }
    let bottom_tokens = strip_leading_token_words_any(&otherwise_tokens, &["otherwise"]);
    let partition_sentences = [
        SentenceInput::from_lexed(sentences[sentence_idx].lexed()),
        SentenceInput::from_lexed(sentences[sentence_idx + 1].lexed()),
        SentenceInput::from_lexed(bottom_tokens),
    ];
    let Some(mut partition) = super::super::ordered_control_flow_programs::parse_top_cards_put_any_matching_to_zone_rest_bottom(
        &partition_sentences,
        0,
    )?
    else {
        return Ok(None);
    };
    let Some(bottom_remainder) = partition.pop() else {
        return Ok(None);
    };
    let [look, choose, move_selected] = partition.as_slice() else {
        return Ok(None);
    };
    let (
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LookAtTopCards {
                    tag: looked_tag, ..
                },
            ..
        }),
        EffectAst::ChooseTaggedObjectsInZone {
            tag: selected_tag, ..
        },
        EffectAst::ForEachTagged { tag: moved_tag, .. },
    ) = (look, choose, move_selected)
    else {
        return Ok(None);
    };
    if moved_tag != selected_tag
        || !matches!(
            &bottom_remainder,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        tag,
                        keep_tagged: Some(keep_tagged),
                        ..
                    },
                ..
            }) if tag == looked_tag && keep_tagged == selected_tag
        )
    {
        return Ok(None);
    }

    let hand_remainder = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::PutTaggedRemainderInZone {
            tag: looked_tag.clone(),
            keep_tagged: selected_tag.clone(),
            zone: Zone::Hand,
            surface: ironsmith_core::LibraryRemainderSurface::Rest,
        },
    );
    partition.push(EffectAst::Conditional {
        predicate,
        if_true: vec![hand_remainder],
        if_false: vec![bottom_remainder],
    });
    Ok(Some(partition))
}

/// Preserves the selected looked card, its conditional entry-time counters,
/// and the exact looked-minus-selected remainder across four sentences:
///
/// "Look at ... . You may put ... onto the battlefield. If that card ...,
/// it enters with ... counters. Put the rest on the bottom ... ."
///
/// The three-sentence looked partition already owns the producer, selected
/// tag, and complement.  Insert only a grammar-proven conditional entry
/// counter between its move and remainder; lowering can then fuse that typed
/// modifier into the battlefield entry without inventing a remainder branch.
pub fn parse_look_at_top_optional_battlefield_conditional_entry_counters_then_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let conditional_tokens = trim_commas(sentences[sentence_idx + 2].lexed());
    let Ok(parsed_conditional) = effect_sentences::parse_effect_sentence_lexed(&conditional_tokens)
    else {
        return Ok(None);
    };
    let [
        conditional @ EffectAst::Conditional {
            if_true, if_false, ..
        },
    ] = parsed_conditional.as_slice()
    else {
        return Ok(None);
    };
    if !if_false.is_empty()
        || !matches!(
            if_true.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutCounters { count, .. },
                ..
            })] if count.has_surface_hint(
                ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter
            )
        )
    {
        return Ok(None);
    }

    let partition_sentences = [
        SentenceInput::from_lexed(sentences[sentence_idx].lexed()),
        SentenceInput::from_lexed(sentences[sentence_idx + 1].lexed()),
        SentenceInput::from_lexed(sentences[sentence_idx + 3].lexed()),
    ];
    let Some(mut partition) = super::super::ordered_control_flow_programs::parse_top_cards_put_any_matching_to_zone_rest_bottom(
        &partition_sentences,
        0,
    )?
    else {
        return Ok(None);
    };
    let Some(remainder) = partition.pop() else {
        return Ok(None);
    };
    if !matches!(
        remainder,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. },
            ..
        })
    ) {
        return Ok(None);
    }
    partition.push(conditional.clone());
    partition.push(remainder);
    Ok(Some(partition))
}

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
                    TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
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

pub fn parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override(
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
    let Some(player) = look_at_top_cards_player(first_effect) else {
        return Ok(None);
    };

    let Some(base_count) =
        parse_counted_looked_cards_into_your_hand_tokens(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    let Some(kicked_count) = parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };
    if !is_put_rest_on_bottom_of_library_sentence(sentences[sentence_idx + 3].lowered()) {
        return Ok(None);
    }
    let Some(bottom_order) =
        crate::grammar::effects::parse_bottom_order(sentences[sentence_idx + 3].lowered())
    else {
        return Ok(None);
    };

    let kicked_looked_tag =
        crate::util::helper_tag_for_tokens(sentences[sentence_idx + 2].lowered(), "looked");
    let base_looked_tag =
        crate::util::helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "looked");
    let kicked_chosen_tag =
        crate::util::helper_tag_for_tokens(sentences[sentence_idx + 2].lowered(), "chosen");
    let base_chosen_tag =
        crate::util::helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::Conditional {
            predicate: crate::cards::builders::PredicateAst::ThisSpellWasKicked,
            if_true: EffectAst::compose_put_some_into_hand_rest_on_bottom_of_library(
                player,
                crate::effect::ChoiceCount::exactly(kicked_count as usize),
                kicked_looked_tag,
                kicked_chosen_tag,
                bottom_order,
            ),
            if_false: EffectAst::compose_put_some_into_hand_rest_on_bottom_of_library(
                player,
                crate::effect::ChoiceCount::exactly(base_count as usize),
                base_looked_tag,
                base_chosen_tag,
                bottom_order,
            ),
        },
    ]))
}

pub fn parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom(
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
    if look_at_top_cards_player(first_effect).is_none() {
        return Ok(None);
    }

    let Some((chooser, battlefield_filter, tapped)) =
        effect_sentences::parse_may_put_filtered_looked_card_onto_battlefield(
            sentences[sentence_idx + 1].lowered(),
        )?
    else {
        return Ok(None);
    };
    if !parse_if_you_dont_put_card_from_among_them_into_your_hand(
        sentences[sentence_idx + 2].lowered(),
    ) {
        return Ok(None);
    }
    if !is_put_rest_on_bottom_of_library_sentence(sentences[sentence_idx + 3].lowered()) {
        return Ok(None);
    }

    let Some((look_player, count, reveal)) = look_at_top_cards_player_count_reveal(first_effect)
    else {
        return Ok(None);
    };

    Ok(Some(
        compose_look_at_top_may_put_onto_battlefield_or_into_hand_rest_bottom(
            sentences[sentence_idx].lowered(),
            sentences[sentence_idx + 1].lowered(),
            look_player,
            count,
            reveal,
            chooser,
            battlefield_filter,
            tapped,
        ),
    ))
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

/// Composes a looked-card selection whose chosen card is revealed and moved
/// to hand before a condition examines that exact selected card. The final
/// remainder effect keeps the original looked pool authoritative.
pub fn parse_look_reveal_match_to_hand_if_selected_matches_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, false)) =
        effect_sentences::parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
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
    let Some(shape) = triple_grammar::parse_looked_hand_action_shape(&reveal_tokens, true) else {
        return Ok(None);
    };
    let mut choice_count = shape.count;
    if !matches!(action_match.actor, LeadingMayActor::Default) && choice_count.min > 0 {
        choice_count = ChoiceCount::up_to(choice_count.max.unwrap_or(choice_count.min));
    }
    let Some(mut filter) =
        effect_sentences::parse_looked_card_reveal_filter(&reveal_tokens[shape.filter])
    else {
        return Ok(None);
    };
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let Some((condition_filter, conditional_effects)) =
        parse_selected_card_leading_if(sentences[sentence_idx + 2].lowered())?
    else {
        return Ok(None);
    };
    let Some(triple_grammar::LookedRemainderShape::LibraryBottom(order)) =
        triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 3].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let selected_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let it = || TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: choice_count,
            player: chooser,
            tag: selected_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::ForEachTagged {
            tag: selected_tag.clone(),
            effects: vec![EffectAst::subject_verb_reveal_tagged(TagKey::from(
                crate::cards::builders::IT_TAG,
            ))],
        },
        EffectAst::ForEachTagged {
            tag: selected_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                it(),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(selected_tag.clone(), condition_filter),
            if_true: conditional_effects,
            if_false: Vec::new(),
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag),
            order,
            chooser,
        ),
    ]))
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
    let iterated = || TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None);
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

/// Preserves the singleton revealed-card provenance across an optional
/// your-turn battlefield move, its hand fallback, and the exact library
/// remainder.
pub fn parse_look_may_reveal_then_your_turn_battlefield_else_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, false)) =
        effect_sentences::parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some((mut filter, mut reveal_count)) =
        parse_may_reveal_up_to_from_looked_cards(sentences[sentence_idx + 1].lowered())?
    else {
        return Ok(None);
    };
    if reveal_count.min > 0 {
        reveal_count = ChoiceCount::up_to(reveal_count.max.unwrap_or(reveal_count.min));
    }
    if reveal_count.min != 0 || reveal_count.max != Some(1) || reveal_count.random {
        return Ok(None);
    }
    if !is_may_put_selected_onto_battlefield_on_your_turn(sentences[sentence_idx + 2].lowered())
        || !is_if_selected_not_put_onto_battlefield_put_into_hand(
            sentences[sentence_idx + 3].lowered(),
        )
    {
        return Ok(None);
    }
    let Some(triple_grammar::LookedRemainderShape::LibraryBottom(order)) =
        triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 4].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let selected_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "revealed");
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let iterated = || TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None);
    let battlefield_move = EffectAst::ForEachTagged {
        tag: selected_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            iterated(),
            Zone::Battlefield,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    };
    let hand_move = EffectAst::ForEachTagged {
        tag: selected_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            iterated(),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    };

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: reveal_count,
            player,
            tag: selected_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_reveal_tagged(selected_tag.clone()),
        EffectAst::Conditional {
            predicate: PredicateAst::YourTurn,
            if_true: vec![
                EffectAst::May {
                    effects: vec![battlefield_move],
                },
                EffectAst::IfResult {
                    predicate: IfResultPredicate::DidNot,
                    effects: vec![hand_move.clone()],
                },
            ],
            if_false: vec![hand_move],
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag),
            order,
            player,
        ),
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
pub(super) fn compose_look_at_top_may_put_onto_battlefield_or_into_hand_rest_bottom(
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

    let it = || TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None);
    let mut in_battlefield_choice_filter = ObjectFilter::default();
    in_battlefield_choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from(crate::cards::builders::IT_TAG),
            relation: TaggedOpbjectRelation::SameStableId,
        });
    let mut in_hand_choice_filter = ObjectFilter::default();
    in_hand_choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from(crate::cards::builders::IT_TAG),
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

pub fn parse_look_at_top_may_reveal_match_bargain_battlefield_else_hand_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        effect_sentences::parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if reveal_top {
        return Ok(None);
    }
    let Some((mut filter, reveal_count)) =
        parse_may_reveal_up_to_from_looked_cards(sentences[sentence_idx + 1].lowered())?
    else {
        return Ok(None);
    };

    if !quad_grammar::parse_bargained_revealed_battlefield_shape(
        sentences[sentence_idx + 2].lowered(),
    ) || !quad_grammar::parse_otherwise_revealed_hand_shape(
        sentences[sentence_idx + 3].lowered(),
    ) || !then_shuffle(sentences[sentence_idx + 4].lowered())
    {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let revealed_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "revealed");
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag),
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: reveal_count,
            player,
            tag: revealed_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_reveal_tagged(revealed_tag.clone()),
        EffectAst::Conditional {
            predicate: PredicateAst::ThisSpellPaidLabel("Bargain".into()),
            if_true: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(revealed_tag.clone(), None),
                Zone::Battlefield,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )],
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(revealed_tag, None),
                Zone::Hand,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ]))
}

/// "you may exile a <filter> card from among them" — the optional single-card
/// exile pick from a previously looked-at set.
pub(super) fn parse_may_exile_filtered_looked_card(
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

/// "Look at the top N cards of your library. You may exile a <filter> card
/// from among them. Put the rest on the bottom of your library in
/// a random/any order. You may cast the exiled card <this turn|without paying
/// its mana cost...>."
pub fn parse_look_at_top_may_exile_match_rest_bottom_cast_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        effect_sentences::parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if reveal_top {
        return Ok(None);
    }
    let Some(exile_filter) =
        parse_may_exile_filtered_looked_card(sentences[sentence_idx + 1].lowered())?
    else {
        return Ok(None);
    };
    let Some(order) = puts_looked_remainder_on_bottom(sentences[sentence_idx + 2].lowered()) else {
        return Ok(None);
    };
    let Some(permission) = parse_cast_or_play_tagged_clause(sentences[sentence_idx + 3].lowered())?
    else {
        return Ok(None);
    };
    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");

    // The final sentence can be either a temporary permission ("this turn")
    // or an immediate cast instruction during resolution. Both consume the
    // same selected exiled-card collection, but they must remain distinct at
    // runtime: an immediate free cast is not an until-end-of-turn grant.
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

    let mut choice_filter = exile_filter;
    choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: choice_filter,
            count: ChoiceCount::up_to(1),
            player,
            tag: exiled_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), false),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag.clone()),
            order,
            player,
        ),
        permission_effect,
    ]))
}

pub fn parse_look_at_top_exile_one_rest_bottom_cast_else_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        effect_sentences::parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if reveal_top {
        return Ok(None);
    }
    if !exiles_one_looked_card_face_down_and_bottoms_rest(sentences[sentence_idx + 1].lowered()) {
        return Ok(None);
    }
    let Some(cast_filter) = parse_exiled_card_cast_filter(sentences[sentence_idx + 2].lowered())?
    else {
        return Ok(None);
    };
    if !puts_exiled_card_into_hand_if_not_cast(sentences[sentence_idx + 3].lowered()) {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");
    let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
    choice_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: choice_filter,
            count: ChoiceCount::exactly(1),
            player,
            tag: exiled_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), true),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag.clone()),
            LibraryBottomOrderAst::Random,
            player,
        ),
        EffectAst::May {
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(exiled_tag.clone(), cast_filter),
                if_true: vec![EffectAst::subject_verb_cast_tagged(
                    exiled_tag.clone(),
                    player,
                    false,
                    false,
                    true,
                    None,
                )],
                if_false: Vec::new(),
            }],
        },
        EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(exiled_tag, None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
    ]))
}

pub fn parse_look_at_top_exile_counted_rest_bottom_play_while_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_clause = LexedClause::new(sentences[sentence_idx].lowered()).trimmed();
    let (look_tokens, exile_count, bottom_order) =
        if let Some(split) = quad_grammar::parse_look_exile_split_shape(first_clause.tokens()) {
            let Some((count, includes_remainder)) =
                parse_counted_looked_cards_exile_face_down(split.exile_tokens)
            else {
                return Ok(None);
            };
            let order = if includes_remainder {
                puts_looked_remainder_on_bottom(split.exile_tokens)
            } else {
                puts_looked_remainder_on_bottom(sentences[sentence_idx + 2].lowered())
            };
            let Some(order) = order else {
                return Ok(None);
            };
            (split.look_tokens, count, order)
        } else {
            let Some((count, includes_remainder)) =
                parse_counted_looked_cards_exile_face_down(sentences[sentence_idx + 1].lowered())
            else {
                return Ok(None);
            };
            let order = if includes_remainder {
                puts_looked_remainder_on_bottom(sentences[sentence_idx + 1].lowered())
            } else {
                puts_looked_remainder_on_bottom(sentences[sentence_idx + 2].lowered())
            };
            let Some(order) = order else {
                return Ok(None);
            };
            (first_clause.tokens(), count, order)
        };

    let Ok(look_effects) = effect_sentences::parse_effect_sentence_lexed(look_tokens) else {
        return Ok(None);
    };
    let [look_effect] = look_effects.as_slice() else {
        return Ok(None);
    };
    let Some(library_owner) = look_at_top_cards_player(look_effect) else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards { count, .. },
        ..
    }) = look_effect
    else {
        return Ok(None);
    };

    let Some(permission_effect) =
        parse_cast_or_play_tagged_clause(sentences[sentence_idx + 3].lowered())?
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
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");
    let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
    choice_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(library_owner, count.clone(), looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: choice_filter,
            count: exile_count,
            player: PlayerAst::You,
            tag: exiled_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), true),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag.clone()),
            bottom_order,
            library_owner,
        ),
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            exiled_tag,
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            filter,
        ),
    ]))
}

pub fn parse_search_reveal_named_match_battlefield_else_hand_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(searched_tag) = search_reveal_tag(&effects) else {
        return Ok(None);
    };
    let Some(named_filter) = named_revealed_card_filter(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    if !puts_it_onto_battlefield(sentences[sentence_idx + 1].lowered())
        || !otherwise_puts_that_card_into_hand(sentences[sentence_idx + 2].lowered())
        || !then_shuffle(sentences[sentence_idx + 3].lowered())
    {
        return Ok(None);
    }

    effects.push(EffectAst::Conditional {
        predicate: PredicateAst::TaggedMatches(searched_tag.clone(), named_filter),
        if_true: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(searched_tag.clone(), None),
            Zone::Battlefield,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )],
        if_false: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(searched_tag, None),
            Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::You,
        SubjectVerbActionAst::ShuffleLibrary,
    ));
    Ok(Some(effects))
}
