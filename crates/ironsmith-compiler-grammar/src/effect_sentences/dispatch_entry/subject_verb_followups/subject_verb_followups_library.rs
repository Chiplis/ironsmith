use super::*;

pub(super) fn is_if_card_put_into_exile_this_way_sentence(tokens: &[OwnedLexToken]) -> bool {
    grammar::match_word_prefix(
        tokens,
        &[
            "if", "a", "card", "is", "put", "into", "exile", "this", "way",
        ],
    )
    .is_some()
        || grammar::match_word_prefix(
            tokens,
            &["if", "card", "is", "put", "into", "exile", "this", "way"],
        )
        .is_some()
        || grammar::match_word_prefix(
            tokens,
            &[
                "if", "a", "card", "was", "put", "into", "exile", "this", "way",
            ],
        )
        .is_some()
        || grammar::match_word_prefix(
            tokens,
            &["if", "card", "was", "put", "into", "exile", "this", "way"],
        )
        .is_some()
}

pub(super) fn pre_rule_when_milled_this_way_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_conditional_followup(sentence_tokens) else {
        return Ok(None);
    };
    if shape.kind != followup_shapes::ConditionalFollowupKind::WhenMilledThisWay {
        return Ok(None);
    }
    let mut plan = SentenceParsePlan::new(trim_commas(shape.continuation_tokens).to_vec());
    plan.wrap_if_result = Some(IfResultPredicate::Did);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

pub(super) fn first_library_search_shape(
    effects: &[EffectAst],
) -> Option<(ObjectFilter, Vec<Zone>, ChoiceCount)> {
    for effect in effects {
        if let EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            zones,
            ..
        } = effect
            && zones.contains(&Zone::Library)
        {
            return Some((filter.clone(), zones.clone(), *count));
        }
        let mut nested_shape = None;
        for_each_nested_effects(effect, true, |nested| {
            if nested_shape.is_none() {
                nested_shape = first_library_search_shape(nested);
            }
        });
        if nested_shape.is_some() {
            return nested_shape;
        }
    }
    None
}

pub(super) fn replace_matching_library_search_count(
    effects: &mut [EffectAst],
    replacement_filter: &ObjectFilter,
    replacement_zones: &[Zone],
    replacement_count: &ChoiceCount,
) -> bool {
    for effect in effects {
        if let EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            zones,
            ..
        } = effect
            && zones.as_slice() == replacement_zones
            && filter == replacement_filter
        {
            *count = *replacement_count;
            return true;
        }
        let mut replaced = false;
        for_each_nested_effects_mut(effect, true, |nested| {
            if !replaced {
                replaced = replace_matching_library_search_count(
                    nested,
                    replacement_filter,
                    replacement_zones,
                    replacement_count,
                );
            }
        });
        if replaced {
            return true;
        }
    }
    false
}

/// A count-only search self-replacement ("search for up to three ... instead
/// of two") changes the selection count of the earlier search procedure. It
/// does not discard that procedure's reveal, split destinations, or trailing
/// shuffle. Clone the complete prior procedure into both arms and change only
/// the matching library selection in the replacement arm.
pub(super) fn materialize_search_count_self_replacement(
    state_effects: &mut Vec<EffectAst>,
    predicate: PredicateAst,
    parsed_replacement: &[EffectAst],
    sentence_tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let words = LexedClause::new(sentence_tokens).word_refs();
    let count_only_surface = crate::word_primitives::sequence_occurs(&words, &["instead", "of"])
        && words.iter().filter(|word| **word == "search").count() == 1
        && !words
            .iter()
            .any(|word| matches!(*word, "put" | "reveal" | "shuffle" | "exile"));
    if !count_only_surface {
        return None;
    }

    let (replacement_filter, replacement_zones, replacement_count) =
        first_library_search_shape(parsed_replacement)?;
    let search_idx = crate::slice_primitives::select_last_position(state_effects, |effect| {
        first_library_search_shape(std::slice::from_ref(effect)).is_some_and(
            |(filter, zones, _)| filter == replacement_filter && zones == replacement_zones,
        )
    })?;

    let default_effects = state_effects.split_off(search_idx);
    let mut replacement_effects = default_effects.clone();
    if !replace_matching_library_search_count(
        &mut replacement_effects,
        &replacement_filter,
        &replacement_zones,
        &replacement_count,
    ) {
        state_effects.extend(default_effects);
        return None;
    }

    Some(EffectAst::SelfReplacement {
        predicate,
        if_true: replacement_effects,
        if_false: default_effects,
        attach_to_previous_ability: false,
    })
}

/// Preserve the comparison set in "for each of those cards that has the same
/// mana value as another card revealed this way." A plain `ForEachTagged`
/// would otherwise count every revealed card, while comparing the iterated
/// card to the implicit `__it__` tag would make every card match itself.
pub(super) fn post_rule_revealed_same_mana_value_as_another_iterator(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let words = crate::lexer::token_word_refs(sentence_tokens);
    const PREFIX: &[&str] = &[
        "for", "each", "of", "those", "cards", "that", "has", "the", "same", "mana", "value", "as",
        "another", "card", "revealed", "this", "way",
    ];
    if !crate::word_primitives::parse_sequence_prefix(&words, PREFIX) {
        return Ok(None);
    }

    let Some(revealed_tag) = state.effects.iter().rev().find_map(|effect| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RevealTagged { tag },
            ..
        }) => Some(tag.clone()),
        _ => None,
    }) else {
        return Ok(None);
    };

    let conditional_effects = match sentence_effects.as_mut_slice() {
        [EffectAst::ForEachTagged { tag, effects }]
            if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && !effects.is_empty() =>
        {
            std::mem::take(effects)
        }
        [EffectAst::RepeatEffects { count, effects }]
            if !effects.is_empty()
                && matches!(
                    count.unhinted(),
                    Value::PendingPriorEffectMetric(query)
                        if query.source == ironsmith_core::EffectMetricSource::AffectedObjects
                            && query.metric == ironsmith_core::EffectMetric::Count
                            && query.player.is_none()
                            && matches!(
                                query.action,
                                None | Some(ironsmith_core::PriorEffectAction::Revealed)
                            )
                            && query.counter_type.is_none()
                ) =>
        {
            std::mem::take(effects)
        }
        _ => return Ok(None),
    };
    let filter = ObjectFilter::default().match_tagged(
        revealed_tag.clone(),
        crate::filter::TaggedOpbjectRelation::SameManaValueAsAnotherTagged,
    );
    *sentence_effects = vec![EffectAst::ForEachTagged {
        tag: revealed_tag,
        effects: vec![EffectAst::TrailingIf {
            predicate: PredicateAst::ItMatches(filter),
            effects: conditional_effects,
        }],
    }];
    Ok(Some(PostParseFollowupResult::Annotated))
}

pub(super) fn preserve_search_owner_anaphor_in_self_replacement(effects: &mut [EffectAst]) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    filter,
                    chooser: PlayerAst::Implicit,
                    player,
                    ..
                },
            ..
        }) = effect
            && *player == PlayerAst::Target
            && matches!(
                filter.owner.as_ref(),
                Some(PlayerFilter::Target(_) | PlayerFilter::IteratedPlayer)
            )
        {
            // The owner filter retains the executable target identity. Inside
            // both arms of a self-replacement, the action surface is an
            // anaphoric reuse of that one target, not a second target choice.
            *player = PlayerAst::That;
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            preserve_search_owner_anaphor_in_self_replacement(nested);
        });
    }
}

pub(super) fn first_search_library_owner(effects: &[EffectAst]) -> Option<PlayerFilter> {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SearchLibrary { filter, .. },
            ..
        }) = effect
            && let Some(owner) = filter.owner.clone()
        {
            return Some(owner);
        }
        let mut nested_owner = None;
        for_each_nested_effects(effect, true, |nested| {
            if nested_owner.is_none() {
                nested_owner = first_search_library_owner(nested);
            }
        });
        if nested_owner.is_some() {
            return nested_owner;
        }
    }
    None
}

pub(super) fn bind_self_replacement_search_owner(
    effects: &mut [EffectAst],
    established: &PlayerFilter,
) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    filter,
                    chooser: PlayerAst::Implicit,
                    player: PlayerAst::That,
                    ..
                },
            ..
        }) = effect
            && matches!(filter.owner.as_ref(), Some(PlayerFilter::IteratedPlayer))
        {
            filter.owner = Some(established.clone());
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            bind_self_replacement_search_owner(nested, established);
        });
    }
}

pub(super) fn mill_count_from_effect(effect: &EffectAst) -> Option<Value> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Mill { count },
            ..
        }) => Some(count.clone()),
        _ => None,
    }
}

pub(super) fn replace_mill_event_amounts_with_value(
    effects: &mut [EffectAst],
    replacement: &Value,
) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Mill { count },
            ..
        }) = effect
        {
            replace_event_amount_with_value(count, replacement);
        }

        for_each_nested_effects_mut(effect, true, |nested| {
            replace_mill_event_amounts_with_value(nested, replacement);
        });
    }
}

pub(super) fn chosen_card_tag_from_hand_choice_branch(effects: &[EffectAst]) -> Option<TagKey> {
    fn collect_exact_hand_choices(effects: &[EffectAst], tags: &mut Vec<TagKey>) {
        for (first, second) in effects.iter().zip(effects.iter().skip(1)) {
            let (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::RevealCardsFromHand {
                            tag: revealed_tag, ..
                        },
                    ..
                }),
                EffectAst::ChooseObjects {
                    filter,
                    count,
                    tag: chosen_tag,
                    ..
                },
            ) = (first, second)
            else {
                continue;
            };
            let chooses_one = count.min == 1
                && count.max == Some(1)
                && !count.dynamic_x
                && !count.up_to_x
                && !count.random;
            if chooses_one
                && tagged_object_reference(filter) == Some(revealed_tag)
                && !tags.iter().any(|tag| tag == chosen_tag)
            {
                tags.push(chosen_tag.clone());
            }
        }

        for effect in effects {
            match effect {
                EffectAst::ChooseObjects {
                    filter, count, tag, ..
                } if filter.zone == Some(Zone::Hand)
                    && count.min == 1
                    && count.max == Some(1)
                    && !count.dynamic_x
                    && !count.up_to_x
                    && !count.random =>
                {
                    if !tags.iter().any(|existing| existing == tag) {
                        tags.push(tag.clone());
                    }
                }
                EffectAst::Coordinated {
                    effects: nested, ..
                }
                | EffectAst::Sequence { effects: nested }
                | EffectAst::SourceSentence {
                    effects: nested, ..
                } => {
                    collect_exact_hand_choices(nested, tags);
                }
                _ => {}
            }
        }
    }

    let mut tags = Vec::new();
    collect_exact_hand_choices(effects, &mut tags);
    let [tag] = tags.as_slice() else {
        return None;
    };
    Some(tag.clone())
}

pub(super) fn is_dependent_that_player_discard(effect: &EffectAst, chosen_tag: &TagKey) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject:
            SubjectVerbSubjectAst {
                role: SubjectVerbRoleAst::AffectedPlayer,
                player: PlayerAst::That,
            },
        action:
            SubjectVerbActionAst::Discard {
                count: Value::Fixed(1),
                random: false,
                any_number: false,
                filter: Some(filter),
                tag: None,
            },
    }) = effect
    else {
        return false;
    };
    filter.zone == Some(Zone::Hand) && tagged_object_reference(filter) == Some(chosen_tag)
}

/// Keep a dependent "That player discards that card" inside the `if you do`
/// branch that established both the player and the chosen-card antecedents.
/// Reference resolution runs after sentence grouping, so moving the AST here
/// lets both demonstratives bind within the branch instead of leaking to the
/// surrounding ability sequence.
pub(super) fn post_rule_hand_reveal_choice_discard_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let [discard] = sentence_effects.as_slice() else {
        return Ok(None);
    };
    let Some(EffectAst::IfResult {
        effects: branch_effects,
        ..
    }) = state.effects.last_mut()
    else {
        return Ok(None);
    };
    let Some(chosen_tag) = chosen_card_tag_from_hand_choice_branch(branch_effects) else {
        return Ok(None);
    };
    if !is_dependent_that_player_discard(discard, &chosen_tag) {
        return Ok(None);
    }

    branch_effects.append(sentence_effects);
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}

pub(super) fn effect_references_prior_exiled_card(effect: &EffectAst) -> bool {
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell {
                target: TargetAst::Tagged(tag, _),
                ..
            },
            ..
        }) if tag.as_str() == crate::tag::CompilerReferenceTag::PriorExiledCard.as_str()
    ) {
        return true;
    }

    let mut found = false;
    for_each_nested_effects(effect, true, |nested| {
        if !found {
            found = nested.iter().any(effect_references_prior_exiled_card);
        }
    });
    found
}

pub(super) fn bind_cast_tag_to_prior_exiled_card(effect: &mut EffectAst) {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::CastTagged {
            tag, as_copy: true, ..
        },
        ..
    }) = effect
        && tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
    {
        *tag = crate::tag::CompilerReferenceTag::PriorExiledCard.bind();
        return;
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for effect in nested {
            bind_cast_tag_to_prior_exiled_card(effect);
        }
    });
}

/// Bind an authored cross-ability "the exiled card" reference to cards
/// linked to the source permanent.  A same-ability exile is handled by
/// `tag_latest_prior_exile`; when there is no such effect, Magic's linked
/// ability wording refers to the object exiled by another ability of this
/// source (for example, an Imprint ability).
pub(super) fn bind_prior_exiled_card_to_source_link(effect: &mut EffectAst) {
    let is_prior_exiled_copy = matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell {
                target: TargetAst::Tagged(tag, _),
                ..
            },
            ..
        }) if tag.as_str() == crate::tag::CompilerReferenceTag::PriorExiledCard.as_str()
    );
    if is_prior_exiled_copy {
        // Copying a card outside the stack is represented by selecting the
        // linked exiled card and letting the following CastTagged(as_copy)
        // create/cast the copy. This is the same generic program used for the
        // explicit "a card exiled with this artifact" wording.
        *effect = EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::default().in_zone(Zone::Exile).match_tagged(
                crate::tag::CompilerReferenceTag::SourceExiled.bind(),
                crate::target::TaggedOpbjectRelation::IsTaggedObject,
            ),
            count: crate::ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.bind(),
            zones: vec![Zone::Exile],
            search_mode: None,
        };
        return;
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for effect in nested {
            bind_prior_exiled_card_to_source_link(effect);
        }
    });
}

/// Keep an authored "the exiled card" reference tied to the exact object
/// moved by the latest prior exile, even when the reference occurs inside a
/// delayed trigger whose ordinary `it` antecedent is the triggering object.
pub(super) fn post_rule_prior_exiled_card_reference(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    if !sentence_effects
        .iter()
        .any(effect_references_prior_exiled_card)
    {
        return Ok(None);
    }
    if !tag_latest_prior_exile(state.effects) {
        for effect in sentence_effects {
            bind_prior_exiled_card_to_source_link(effect);
        }
    }
    Ok(Some(PostParseFollowupResult::Annotated))
}
