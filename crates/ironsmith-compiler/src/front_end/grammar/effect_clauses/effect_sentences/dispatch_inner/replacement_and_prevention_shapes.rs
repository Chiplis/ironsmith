use crate::grammar::effects as replacement_grammar;
pub(crate) fn parse_monstrosity_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = replacement_grammar::parse_monstrosity_shape(tokens) else {
        return Ok(None);
    };
    Ok(Some(EffectAst::subject_verb_monstrosity(shape.amount)))
}

pub(crate) fn parse_for_each_counter_removed_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = replacement_grammar::parse_counter_removed_pump_shape(tokens) else {
        return Ok(None);
    };

    Ok(Some(EffectAst::subject_verb_pump_by_last_effect(
        shape.power,
        shape.toughness,
        TargetAst::Source(None),
        Until::EndOfTurn,
        shape.includes_this_way,
    )))
}

pub(crate) fn is_exile_that_token_at_end_of_combat(tokens: &[OwnedLexToken]) -> bool {
    replacement_grammar::parse_token_end_combat_action_shape(tokens)
        == Some(replacement_grammar::TokenEndCombatActionShape::Exile)
}

pub(crate) fn is_exile_that_token_at_end_of_combat_lexed(tokens: &[OwnedLexToken]) -> bool {
    is_exile_that_token_at_end_of_combat(tokens)
}

pub(crate) fn is_sacrifice_that_token_at_end_of_combat(tokens: &[OwnedLexToken]) -> bool {
    replacement_grammar::parse_token_end_combat_action_shape(tokens)
        == Some(replacement_grammar::TokenEndCombatActionShape::Sacrifice)
}

pub(crate) fn is_sacrifice_that_token_at_end_of_combat_lexed(tokens: &[OwnedLexToken]) -> bool {
    is_sacrifice_that_token_at_end_of_combat(tokens)
}

pub(crate) fn parse_take_extra_turn_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    Ok(replacement_grammar::parse_extra_turn_shape(tokens)
        .map(|shape| EffectAst::subject_verb_extra_turn_after_turn(shape.player, shape.anchor)))
}

pub(crate) fn parse_additional_phase_sentence(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    replacement_grammar::parse_additional_phases_shape(tokens)
        .map(|shape| EffectAst::subject_verb_additional_phases(shape.phases))
}
pub(crate) fn parse_destroy_or_exile_all_split_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = replacement_grammar::parse_split_all_shape(tokens) else {
        return Ok(None);
    };

    if shape.connective == replacement_grammar::SplitAllConnectiveShape::Or {
        let mut modes = Vec::with_capacity(shape.filter_tokens.len());
        for filter_tokens in shape.filter_tokens {
            let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in split all choice (clause: '{}')",
                    render_token_slice(tokens).trim()
                ))
            })?;
            let effect = match shape.verb {
                replacement_grammar::SplitAllVerbShape::Destroy => {
                    EffectAst::subject_verb_destroy_all(filter)
                }
                replacement_grammar::SplitAllVerbShape::Exile => {
                    EffectAst::subject_verb_exile_all(filter, false)
                }
            };
            modes.push(crate::cards::builders::ChooseOneModeAst {
                description: String::new(),
                effects: vec![effect],
            });
        }
        return Ok(Some(vec![EffectAst::ChooseOneOf { modes }]));
    }

    // A coordinated all-object clause can carry independent scope on each
    // authored branch, for example controller, owner, attachment, or combat
    // state. The complete object-filter grammar preserves those branches and
    // their authored connective as one typed union. Prefer that result before
    // the legacy simple-list splitter parses each noun independently.
    if let Ok(filter) = parse_object_filter(shape.body_tokens, false).map(|filter| {
        super::zone_handlers::scope_types_away_from_requantified_bare_card_domains(
            shape.body_tokens,
            filter,
        )
    }) && filter.any_of.len() >= 2
    {
        let effect = match shape.verb {
            replacement_grammar::SplitAllVerbShape::Destroy => {
                EffectAst::subject_verb_destroy_all(filter)
            }
            replacement_grammar::SplitAllVerbShape::Exile => {
                EffectAst::subject_verb_exile_all(filter, false)
            }
        };
        return Ok(Some(vec![effect]));
    }

    let mut filters = Vec::new();
    for filter_tokens in shape.filter_tokens {
        let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in split all clause (clause: '{}')",
                render_token_slice(tokens).trim()
            ))
        })?;
        filters.push(filter);
    }

    if filters.len() >= 2 {
        // Keep a conjoined all-object instruction as one producer. Besides
        // matching the simultaneous rules action, this gives later
        // "destroyed/exiled this way" references one exact result tag rather
        // than pointing only at the final syntactic arm.
        let mut union = ObjectFilter::default();
        union.any_of = filters;
        let effect = match shape.verb {
            replacement_grammar::SplitAllVerbShape::Destroy => {
                EffectAst::subject_verb_destroy_all(union)
            }
            replacement_grammar::SplitAllVerbShape::Exile => {
                EffectAst::subject_verb_exile_all(union, false)
            }
        };
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}

pub(crate) fn parse_exile_then_return_same_object_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn target_references_tag(target: &TargetAst, expected: &str) -> bool {
        match target {
            TargetAst::Tagged(tag, _) => tag.as_str() == expected,
            TargetAst::Object(filter, _, _) => filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == expected
                    && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
            }),
            _ => false,
        }
    }
    fn target_references_it_tag(target: &TargetAst) -> bool {
        target_references_tag(target, IT_TAG)
    }
    fn target_references_source_exiled_tag(target: &TargetAst) -> bool {
        target_references_tag(target, crate::tag::SOURCE_EXILED_TAG)
    }

    let Some(shape) = replacement_grammar::parse_exile_return_same_shape(tokens) else {
        return Ok(None);
    };
    crate::parse_trace::event(format!(
        "exile-return-same: counter_tokens={:?} return_tokens_len={}",
        shape
            .counter_tokens
            .map(crate::token_word_refs),
        shape.return_tokens.len()
    ));

    let mut first_effects = parse_effect_chain_inner(shape.exile_tokens)?;
    if !first_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Exile { .. },
                ..
            })
        )
    }) {
        return Ok(None);
    }
    let source_exiled_tag = TagKey::from(crate::tag::SOURCE_EXILED_TAG);
    for effect in &mut first_effects {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Exile { .. },
                ..
            })
        ) {
            let exile = effect.clone();
            *effect = EffectAst::TagAffected {
                effect: Box::new(exile),
                tag: source_exiled_tag.clone(),
            };
            break;
        }
    }

    // Preserve return follow-up clauses (for example "with a +1/+1 counter on it")
    // while still rewriting the "it" return target to the tagged exiled object.
    let mut second_effects = if let Some(effects) = parse_sentence_return_with_counters_on_it(
        super::SubjectVerbPrimitiveClause::new(shape.return_tokens),
    )? {
        crate::parse_trace::event("exile-return-same: with-counters parser matched".to_string());
        effects
    } else {
        crate::parse_trace::event(
            "exile-return-same: with-counters parser MISSED, chain fallback".to_string(),
        );
        parse_effect_chain_inner(shape.return_tokens)?
    };
    let has_counter_followup = second_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutCounters { .. },
                ..
            })
        )
    });
    if !has_counter_followup && let Some(counter_tokens) = shape.counter_tokens {
        let (count, counter_type) =
            super::zone_counter_helpers::parse_counter_descriptor(counter_tokens)?;
        second_effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            TargetAst::Tagged(TagKey::from(IT_TAG), None),
            None,
            false,
        ));
    }
    let mut rewrote_return = false;
    for effect in &mut second_effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ReturnToBattlefield { target, .. },
                ..
            }) if target_references_it_tag(target)
                || target_references_source_exiled_tag(target) =>
            {
                if target_references_it_tag(target) {
                    *target = TargetAst::Tagged(source_exiled_tag.clone(), None);
                }
                rewrote_return = true;
            }
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::MoveToZone {
                        target,
                        zone: Zone::Battlefield,
                        ..
                    },
                ..
            }) if target_references_it_tag(target)
                || target_references_source_exiled_tag(target) =>
            {
                // Returns with battlefield-entry modifiers such as "face
                // down" use the generic move-to-zone AST rather than the
                // simpler ReturnToBattlefield variant. They still need the
                // exact exile-result tag so the blink sequence is retained.
                if target_references_it_tag(target) {
                    *target = TargetAst::Tagged(source_exiled_tag.clone(), None);
                }
                rewrote_return = true;
            }
            EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
                SubjectVerbActionAst::ReturnToHand { target, .. }
                    if target_references_it_tag(target)
                        || target_references_source_exiled_tag(target) =>
                {
                    if target_references_it_tag(target) {
                        *target = TargetAst::Tagged(source_exiled_tag.clone(), None);
                    }
                    rewrote_return = true;
                }
                _ => {}
            },
            _ => {}
        }
    }
    if !rewrote_return {
        return Ok(None);
    }

    if shape.delayed_until_end_of_combat {
        let mut delayed_effects = first_effects;
        delayed_effects.extend(second_effects);
        return Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
            effects: delayed_effects,
        }]));
    }

    first_effects.extend(second_effects);
    Ok(Some(first_effects))
}

pub(crate) fn parse_exile_up_to_one_each_target_type_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = replacement_grammar::parse_exile_each_target_type_shape(tokens) else {
        return Ok(None);
    };

    let mut filters = Vec::new();
    for filter_tokens in shape.filter_tokens {
        let mut filter = parse_object_filter(filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in 'exile up to one each target type' clause (clause: '{}')",
                render_token_slice(tokens).trim()
            ))
        })?;
        // These are explicit independent targets, not objects the chooser
        // controls by default. Preserve an explicit "you control" clause, but
        // otherwise keep the target unrestricted.
        if filter.controller.is_none() {
            filter.controller = Some(PlayerFilter::Any);
        }
        filters.push(filter);
    }

    if filters.len() < 2 {
        return Ok(None);
    }

    let tag = helper_tag_for_tokens(tokens, "exiled");
    let mut effects = filters
        .into_iter()
        .map(|filter| EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
        })
        .collect::<Vec<_>>();
    effects.push(EffectAst::subject_verb_exile(
        TargetAst::Tagged(tag, None),
        false,
    ));

    Ok(Some(effects))
}

pub(crate) fn parse_look_at_hand_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = replacement_grammar::parse_look_hand_shape(tokens) else {
        return Ok(None);
    };
    let target = match shape.player {
        replacement_grammar::LookHandPlayerShape::TargetPlayer => {
            TargetAst::Player(PlayerFilter::target_player(), Some(TextSpan::synthetic()))
        }
        replacement_grammar::LookHandPlayerShape::TargetOpponent => {
            TargetAst::Player(PlayerFilter::target_opponent(), Some(TextSpan::synthetic()))
        }
        replacement_grammar::LookHandPlayerShape::Opponent => {
            TargetAst::Player(PlayerFilter::Opponent, None)
        }
        replacement_grammar::LookHandPlayerShape::IteratedPlayer => {
            TargetAst::Player(PlayerFilter::IteratedPlayer, None)
        }
    };
    let mut effects = vec![EffectAst::subject_verb_look_at_hand(target)];
    if shape.choose_card_name {
        effects.push(EffectAst::subject_verb_choose_card_name(
            PlayerAst::You,
            None,
            TagKey::from(IT_TAG),
        ));
    }
    Ok(Some(effects))
}

pub(crate) fn parse_look_at_top_then_exile_one_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = replacement_grammar::parse_look_top_exile_one_shape(tokens) else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(tokens, "looked");
    let chosen_tag = helper_tag_for_tokens(tokens, "chosen");
    let mut looked_filter = ObjectFilter::tagged(looked_tag.clone());
    looked_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(
            shape.player,
            Value::Fixed(shape.count as i32),
            looked_tag,
        ),
        EffectAst::ChooseObjects {
            filter: looked_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag.clone(),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(chosen_tag, None), shape.face_down),
    ]))
}

pub(crate) fn parse_gain_life_equal_to_age_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Legacy fallback previously returned a hardcoded 0-life effect for age-counter clauses.
    // Let generic life parsing handle these so counter-scaled amounts compile correctly.
    let _ = tokens;
    Ok(None)
}

pub(crate) fn parse_you_and_each_opponent_voted_with_you_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = replacement_grammar::parse_voted_with_you_scry_shape(tokens) else {
        return Ok(None);
    };
    let count = shape.count;

    let you_effect = EffectAst::May {
        effects: vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::Chooser,
            PlayerAst::You,
            SubjectVerbActionAst::Scry {
                count: count.clone(),
            },
        )],
    };

    let opponent_effect = EffectAst::ForEachTaggedPlayer {
        tag: TagKey::from("voted_with_you"),
        effects: vec![EffectAst::May {
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::Chooser,
                PlayerAst::Implicit,
                SubjectVerbActionAst::Scry { count },
            )],
        }],
    };

    Ok(Some(vec![you_effect, opponent_effect]))
}

#[cfg(test)]
#[path = "replacement_and_prevention_shapes/replacement_and_prevention_shape_tests.rs"]
mod replacement_and_prevention_shape_tests;
