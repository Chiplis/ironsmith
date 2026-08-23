use crate::effect_sentences::{
    SubjectVerbPrimitiveClause, parse_sentence_delayed_next_step_unless_pays,
    parse_sentence_delayed_timing_suffix,
    parse_sentence_each_player_return_with_additional_counter,
    parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard,
};

macro_rules! sentence_unsupported_adapters_lexed {
    ($(($adapter:ident, $predicate:ident)),* $(,)?) => {
        $(
            pub(super) fn $adapter(view: &LexClauseView<'_>) -> bool {
                let words = view.words.to_word_refs();
                $predicate(words.as_slice(), view.tokens)
            }
        )*
    };
}

fn trailing_counter_constraint(
    tokens: &[OwnedLexToken],
) -> Option<crate::filter::CounterConstraint> {
    match sentence_shapes::parse_trailing_counter_constraint_tokens(tokens)? {
        sentence_shapes::TrailingCounterConstraintShape::NoCounters => None,
        sentence_shapes::TrailingCounterConstraintShape::Constraint(constraint) => Some(constraint),
    }
}

fn apply_trailing_counter_constraint_to_destroy_all(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
) {
    let Some(counter_constraint) = trailing_counter_constraint(tokens) else {
        return;
    };
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::DestroyAll { filter, .. }
                    | SubjectVerbActionAst::ExileAll { filter, .. },
                ..
            }) = effect
            && filter.with_counter.is_none() {
                filter.with_counter = Some(counter_constraint);
            }
    }
}

fn is_loss_become_base_pt_coordinated_chain(effects: &[EffectAst]) -> bool {
    let [
        EffectAst::Coordinated {
            effects: coordinated,
            ..
        },
    ] = effects
    else {
        return false;
    };
    matches!(
        coordinated.as_slice(),
        [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RemoveAbilitiesAll { .. },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::AddSubtypes { .. },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::SetBasePowerToughness { .. },
                ..
            }),
        ]
    )
}

fn parse_target_deals_power_damage_to_other_and_self_where_x(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = sentence_shapes::parse_power_damage_self_tokens(tokens) else {
        return Ok(None);
    };
    let source_tokens = trim_edge_punctuation(shape.source_tokens);
    let first_target_tokens = trim_edge_punctuation(shape.first_target_tokens);
    if source_tokens.is_empty() || first_target_tokens.is_empty() {
        return Ok(None);
    }

    let source = parse_target_phrase(&source_tokens)?;
    let first_target = parse_target_phrase(&first_target_tokens)?;
    let source_ref = TargetAst::Tagged(TagKey::from(IT_TAG), None);
    Ok(Some(vec![
        EffectAst::subject_verb_target_only(source.clone()),
        EffectAst::subject_verb_damage_equal_to_power(source_ref.clone(), first_target),
        EffectAst::subject_verb_damage_equal_to_power(source_ref.clone(), source_ref),
    ]))
}

fn parse_conjoined_must_be_blocked_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Conditional sentences own their effect payload and recursively dispatch
    // it after the predicate has been removed. Do not treat a condition's
    // gain/get verb as the shared subject action.
    if token_slice_first_is(tokens, "if") {
        return Ok(None);
    }
    let Some(shape) =
        effect_grammar::clause_primitive_shapes::parse_combat_requirement_shape(tokens)
    else {
        return Ok(None);
    };
    if shape.kind != effect_grammar::clause_primitive_shapes::CombatRequirementKind::MustBeBlocked {
        return Ok(None);
    }

    let subject_and_action = trim_edge_punctuation(shape.subject_tokens);
    let Some(and_token_idx) = subject_and_action
        .iter()
        .rposition(|token| token.as_word() == Some("and"))
    else {
        return Ok(None);
    };
    if subject_and_action[and_token_idx + 1..]
        .iter()
        .any(|token| token.as_word().is_some())
    {
        return Ok(None);
    }

    let action_tokens = trim_edge_punctuation(&subject_and_action[..and_token_idx]);
    let Some((verb, verb_word_idx)) = super::lex_chain_helpers::find_verb_lexed(&action_tokens)
    else {
        return Ok(None);
    };
    if !matches!(verb, super::Verb::Get | super::Verb::Gain) {
        return Ok(None);
    }
    let action_words = TokenWordView::new(&action_tokens);
    let Some(subject_end_token_idx) = action_words.token_boundary_for_word_or_end(verb_word_idx)
    else {
        return Ok(None);
    };
    let shared_subject_tokens = trim_edge_punctuation(&action_tokens[..subject_end_token_idx]);
    if shared_subject_tokens.is_empty() {
        return Ok(None);
    }

    let restriction_filter = if starts_with_target_indicator(&shared_subject_tokens) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        let target = parse_target_phrase(&shared_subject_tokens)?;
        target_ast_to_object_filter(target).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported shared subject in conjoined must-be-blocked clause (clause: '{}')",
                render_token_slice(tokens)
            ))
        })?
    };

    // Parse the isolated get/gain head through the normal subject-verb route
    // without recursively re-entering this sentence dispatcher.
    let mut parsed_head =
        if let Some((_, effects)) = parse_top_level_subject_verb_recognition(&action_tokens)? {
            effects
        } else {
            parse_effect_sentence_inner_lexed(&action_tokens)?
        };
    if let Some(surface) = parse_set_quantifier_surface(&action_tokens) {
        set_first_continuous_set_quantifier(&mut parsed_head, surface);
    }
    if parsed_head.is_empty() {
        return Ok(None);
    }
    let mut leading_duration = false;
    if parsed_head.len() == 1 && matches!(parsed_head.first(), Some(EffectAst::Coordinated { .. }))
    {
        let EffectAst::Coordinated {
            effects,
            leading_duration: nested_leading_duration,
            result_conjunction: false,
        } = parsed_head.remove(0)
        else {
            unreachable!("coordinated head was checked before removal")
        };
        parsed_head = effects;
        leading_duration = nested_leading_duration;
    }
    parsed_head.push(EffectAst::subject_verb_cant(
        crate::effect::Restriction::must_be_blocked(restriction_filter),
        Until::EndOfTurn,
        None,
    ));

    Ok(Some(vec![EffectAst::Coordinated {
        effects: parsed_head,
        leading_duration,
        result_conjunction: false,
    }]))
}

fn parse_attack_or_block_then_prohibition_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn exact_attack_or_block_prohibition_duration(effects: &[EffectAst]) -> Option<Until> {
        fn visit(
            effect: &EffectAst,
            target_declarations: &mut usize,
            duration: &mut Option<Until>,
        ) -> bool {
            match effect {
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::TargetOnly { .. },
                    ..
                }) => {
                    *target_declarations += 1;
                    true
                }
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::Cant {
                            restriction: crate::effect::Restriction::AttackOrBlock(_),
                            duration: candidate,
                            ..
                        },
                    ..
                }) if duration.is_none() => {
                    *duration = Some(candidate.clone());
                    true
                }
                EffectAst::Coordinated { effects, .. } => effects
                    .iter()
                    .all(|effect| visit(effect, target_declarations, duration)),
                _ => false,
            }
        }

        let mut target_declarations = 0;
        let mut duration = None;
        if effects
            .iter()
            .all(|effect| visit(effect, &mut target_declarations, &mut duration))
            && target_declarations == 1
        {
            duration
        } else {
            None
        }
    }

    for (and_idx, token) in tokens.iter().enumerate() {
        if token.as_word() != Some("and") {
            continue;
        }
        let first_tokens = trim_edge_punctuation(&tokens[..and_idx]);
        let second_tokens = trim_edge_punctuation(&tokens[and_idx + 1..]);
        let Some(shape) =
            effect_grammar::clause_primitive_shapes::parse_combat_requirement_shape(&first_tokens)
        else {
            continue;
        };
        if shape.kind
            != effect_grammar::clause_primitive_shapes::CombatRequirementKind::AttackOrBlock
        {
            continue;
        }
        let expected_duration = match shape.duration {
            effect_grammar::clause_primitive_shapes::CombatRequirementDuration::Turn => {
                Until::EndOfTurn
            }
            effect_grammar::clause_primitive_shapes::CombatRequirementDuration::Combat => {
                Until::EndOfCombat
            }
        };
        let Some(requirement) =
            super::clause_primitives::parse_attack_or_block_this_turn_if_able_clause(
                &first_tokens,
            )?
        else {
            continue;
        };
        let Some(prohibition) = parse_cant_effect_sentence_lexed(&second_tokens)? else {
            continue;
        };
        if exact_attack_or_block_prohibition_duration(&prohibition)
            != Some(expected_duration.clone())
        {
            continue;
        }

        let mut effects = vec![requirement];
        effects.extend(prohibition);
        return Ok(Some(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]));
    }
    Ok(None)
}

/// Parse mass effects whose affected set is defined relative to an explicit
/// creature target in the same sentence. A plain `blocking`/`blocked` filter
/// only describes combat status; it cannot preserve which combat the target
/// participates in.
fn parse_target_relative_combat_set_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);

    if words.starts_with(&["tap", "all"])
        && let Some(relation_idx) = words
            .windows(2)
            .position(|window| window == ["blocking", "target"])
        && relation_idx > 2
    {
        let object_tokens =
            crate::lexer::synthetic_word_tokens(words[2..relation_idx].iter().copied());
        let target_tokens =
            crate::lexer::synthetic_word_tokens(words[relation_idx + 1..].iter().copied());
        let mut affected = parse_object_filter(&object_tokens, false)?;
        affected.blocking = true;
        affected.in_combat_with = Some(ObjectRef::Target);
        affected.set_plural_object_noun_surface(true);
        affected.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::All));
        let target = parse_target_phrase(&target_tokens)?;
        return Ok(Some(vec![
            EffectAst::subject_verb_target_only(target),
            EffectAst::subject_verb_tap_all(affected),
        ]));
    }

    if words.starts_with(&["return", "all"])
        && let Some(relation_idx) = words
            .windows(5)
            .position(|window| window == ["blocking", "or", "blocked", "by", "target"])
        && relation_idx > 2
        && let Some(destination_idx) = words[relation_idx + 5..]
            .iter()
            .position(|word| *word == "to")
            .map(|offset| relation_idx + 5 + offset)
        && destination_idx > relation_idx + 4
        && matches!(
            &words[destination_idx..],
            ["to", "their", "owner's", "hand"]
                | ["to", "their", "owners", "hand"]
                | ["to", "their", "owners'", "hands"]
                | ["to", "their", "owners", "hands"]
        )
    {
        let object_tokens =
            crate::lexer::synthetic_word_tokens(words[2..relation_idx].iter().copied());
        let target_tokens = crate::lexer::synthetic_word_tokens(
            words[relation_idx + 4..destination_idx].iter().copied(),
        );
        let mut affected = parse_object_filter(&object_tokens, false)?;
        affected.in_combat_with = Some(ObjectRef::Target);
        affected.set_plural_object_noun_surface(true);
        affected.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::All));
        let target = parse_target_phrase(&target_tokens)?;
        return Ok(Some(vec![
            EffectAst::subject_verb_target_only(target),
            EffectAst::subject_verb_return_all_to_hand(affected),
        ]));
    }

    Ok(None)
}

pub fn lower_where_x_shape(
    shape: sentence_shapes::WhereXValueShape,
) -> Option<(Option<EffectAst>, Value)> {
    use sentence_shapes::{WhereXMetricShape as Metric, WhereXReferenceShape as Reference};

    let lowered = match shape {
        sentence_shapes::WhereXValueShape::CommanderManaValueChoice => {
            let mut filter = ObjectFilter::default();
            filter.is_commander = true;
            filter.owner = Some(PlayerFilter::You);
            let tag = TagKey::from("__where_x_commander_mana_value");
            let choice = EffectAst::ChooseObjectsAcrossZones {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
                zones: vec![Zone::Battlefield, Zone::Command],
                search_mode: None,
            };
            (
                Some(choice),
                Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(tag))),
            )
        }
        sentence_shapes::WhereXValueShape::ChosenObjectsPowerDifference { object_kind } => {
            let object_tokens = crate::lexer::synthetic_word_tokens([object_kind.as_str()]);
            let mut filter = parse_object_filter(&object_tokens, false).ok()?;
            filter = filter.match_tagged(
                TagKey::from(CHOSEN_OBJECTS_TAG),
                TaggedOpbjectRelation::IsTaggedObject,
            );
            (
                None,
                Value::absolute_difference(
                    Value::GreatestPower(filter.clone()),
                    Value::LeastPower(filter),
                )
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::Difference),
            )
        }
        sentence_shapes::WhereXValueShape::ReferenceMetric { reference, metric } => {
            let choose = match reference {
                Reference::Source => crate::target::ChooseSpec::Source,
                Reference::Target => crate::target::ChooseSpec::target(
                    crate::target::ChooseSpec::Object(ObjectFilter::default()),
                ),
                Reference::TaggedIt => crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG)),
            };
            let value = match (reference, metric) {
                (Reference::Source, Metric::Power) => Value::SourcePower,
                (Reference::Source, Metric::Toughness) => Value::SourceToughness,
                (_, Metric::Power) => Value::PowerOf(Box::new(choose)),
                (_, Metric::Toughness) => Value::ToughnessOf(Box::new(choose)),
                (_, Metric::ManaValue) => Value::ManaValueOf(Box::new(choose)),
            };
            (None, value)
        }
        sentence_shapes::WhereXValueShape::TapCostPower => (
            None,
            Value::PowerOf(Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(
                "tap_cost_0",
            ))))
            .with_surface_hint(
                ironsmith_core::ValueSurfaceHint::CharacteristicOfObjectThisWay {
                    card_type: crate::types::CardType::Creature,
                    action: ironsmith_core::PriorEffectAction::Tapped,
                },
            ),
        ),
        sentence_shapes::WhereXValueShape::CommanderCastCount => {
            (None, Value::CommanderCastCount(PlayerFilter::You))
        }
        sentence_shapes::WhereXValueShape::CardTypesInYourGraveyard => {
            (None, Value::CardTypesInGraveyard(PlayerFilter::You))
        }
        sentence_shapes::WhereXValueShape::SacrificeCostManaValue { object_kind } => {
            let object_kind = match object_kind {
                sentence_shapes::SacrificeCostObjectKindShape::CardType(card_type) => {
                    card_type.name()
                }
                sentence_shapes::SacrificeCostObjectKindShape::Permanent => "permanent",
            };
            (
                None,
                Value::ManaValueOf(Box::new(
                    crate::target::ChooseSpec::Tagged(TagKey::from("sacrifice_cost_0"))
                        .with_surface_hint(crate::target::ChooseSpecSurfaceHint::SourceReference(
                            crate::target::SourceReferenceSurface::ThisPermanentType(format!(
                                "the sacrificed {object_kind}"
                            )),
                        )),
                )),
            )
        }
        sentence_shapes::WhereXValueShape::ColorsAmongSacrificed { object_kind } => {
            let object_kind = object_kind.trim_end_matches('s');
            let object_tokens = crate::lexer::synthetic_word_tokens([object_kind]);
            let mut filter = parse_object_filter(&object_tokens, false).ok()?;
            filter = filter.match_tagged(
                TagKey::from("sacrificed_0"),
                TaggedOpbjectRelation::IsTaggedObject,
            );
            (None, Value::ColorsAmong(filter))
        }
        sentence_shapes::WhereXValueShape::TwoPlusSacrificedManaValue => (
            None,
            Value::Add(
                Box::new(Value::Fixed(2)),
                Box::new(Value::ManaValueOf(Box::new(
                    crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG)),
                ))),
            ),
        ),
        sentence_shapes::WhereXValueShape::SourceExiledManaValue => (
            None,
            Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(
                crate::tag::SOURCE_EXILED_TAG,
            )))),
        ),
        sentence_shapes::WhereXValueShape::PriorEffectMetric(query) => {
            (None, Value::PendingPriorEffectMetric(query))
        }
        sentence_shapes::WhereXValueShape::RemovedCountersThisWay => (None, Value::X),
        sentence_shapes::WhereXValueShape::CountersOn {
            reference,
            counter_type,
        } => {
            let value = match (reference, counter_type) {
                (Reference::Source, Some(counter_type)) => Value::CountersOnSource(counter_type),
                (Reference::Source, None) => Value::CountersOn(Box::new(ChooseSpec::Source), None),
                (Reference::Target, counter_type) => Value::CountersOn(
                    Box::new(ChooseSpec::target(ChooseSpec::Object(
                        ObjectFilter::default(),
                    ))),
                    counter_type,
                ),
                (Reference::TaggedIt, counter_type) => Value::CountersOn(
                    Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                    counter_type,
                ),
            };
            (None, value)
        }
    };
    Some(lowered)
}

fn parse_tap_then_damage_for_number_tapped_this_way(
    stripped: &[OwnedLexToken],
    where_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = sentence_shapes::parse_tapped_this_way_binding_tokens(stripped, where_tokens)
    else {
        return Ok(None);
    };

    let mut effects = parse_effect_sentence_inner_lexed(stripped)?;
    if effects.len() != 2 {
        return Ok(None);
    }
    let first_is_tap = matches!(
        &effects[0],
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tap { .. } | SubjectVerbActionAst::TapAll { .. },
            ..
        })
    );
    if !first_is_tap {
        return Ok(None);
    }
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::DealDamage { amount, target, .. },
        ..
    }) = &mut effects[1]
    else {
        return Ok(None);
    };
    if !matches!(amount, Value::X) {
        return Ok(None);
    }

    *amount = Value::EventValue(EventValueSpec::Amount);
    if shape.damage_to_active_player {
        *target = TargetAst::Player(PlayerFilter::Active, None);
    }
    Ok(Some(effects))
}

fn parse_next_spell_grant_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::next_spell_family::parse_next_spell_grant_sentence_lexed(tokens)
}

pub(super) fn sentence_has_enters_as_copy_rule_lexed(view: &LexClauseView<'_>) -> bool {
    effect_grammar::is_enters_as_copy_clause_lexed(view.tokens)
}

sentence_unsupported_adapters_lexed!(
    (
        sentence_has_each_player_lose_discard_sacrifice_chain_rule_lexed,
        sentence_has_each_player_lose_discard_sacrifice_chain
    ),
    (
        sentence_has_each_player_exile_sacrifice_return_exiled_clause_rule_lexed,
        sentence_has_each_player_exile_sacrifice_return_exiled_clause
    ),
    (
        sentence_has_put_one_of_them_into_hand_rest_clause_rule_lexed,
        sentence_has_put_one_of_them_into_hand_rest_clause
    ),
    (
        sentence_has_loses_all_abilities_with_becomes_clause_rule_lexed,
        sentence_has_loses_all_abilities_with_becomes_clause
    ),
    (
        sentence_has_spent_to_cast_this_spell_without_condition_rule_lexed,
        sentence_has_spent_to_cast_this_spell_without_condition
    ),
    (
        sentence_has_would_enter_instead_replacement_clause_rule_lexed,
        sentence_has_would_enter_instead_replacement_clause
    ),
    (
        sentence_has_different_mana_value_constraint_rule_lexed,
        sentence_has_different_mana_value_constraint
    ),
    (
        sentence_has_most_common_color_constraint_rule_lexed,
        sentence_has_most_common_color_constraint
    ),
    (
        sentence_has_power_vs_count_constraint_rule_lexed,
        sentence_has_power_vs_count_constraint
    ),
    (
        sentence_has_put_into_graveyards_from_battlefield_this_turn_rule_lexed,
        sentence_has_put_into_graveyards_from_battlefield_this_turn
    ),
    (
        sentence_has_phase_out_until_leaves_clause_rule_lexed,
        sentence_has_phase_out_until_leaves_clause
    ),
    (
        sentence_has_same_name_as_another_in_hand_clause_rule_lexed,
        sentence_has_same_name_as_another_in_hand_clause
    ),
    (
        sentence_has_for_each_mana_from_spent_to_cast_clause_rule_lexed,
        sentence_has_for_each_mana_from_spent_to_cast_clause
    ),
    (
        sentence_has_when_you_sacrifice_this_way_clause_rule_lexed,
        sentence_has_when_you_sacrifice_this_way_clause
    ),
    (
        sentence_has_greatest_mana_value_clause_rule_lexed,
        sentence_has_greatest_mana_value_clause
    ),
    (
        sentence_has_least_power_among_creatures_clause_rule_lexed,
        sentence_has_least_power_among_creatures_clause
    ),
    (
        sentence_has_villainous_choice_clause_rule_lexed,
        sentence_has_villainous_choice_clause
    ),
    (
        sentence_has_divided_evenly_clause_rule_lexed,
        sentence_has_divided_evenly_clause
    ),
    (
        sentence_has_different_names_clause_rule_lexed,
        sentence_has_different_names_clause
    ),
    (
        sentence_has_chosen_at_random_clause_rule_lexed,
        sentence_has_chosen_at_random_clause
    ),
    (
        sentence_has_defending_players_choice_clause_rule_lexed,
        sentence_has_defending_players_choice_clause
    ),
    (
        sentence_has_target_creature_token_player_planeswalker_clause_rule_lexed,
        sentence_has_target_creature_token_player_planeswalker_clause
    ),
    (
        sentence_has_if_you_sacrifice_an_island_this_way_clause_rule_lexed,
        sentence_has_if_you_sacrifice_an_island_this_way_clause
    ),
    (
        sentence_has_spent_to_cast_clause_rule_lexed,
        sentence_has_spent_to_cast_clause
    ),
    (
        sentence_has_face_down_clause_rule_lexed,
        sentence_has_face_down_clause
    ),
    (
        sentence_has_return_each_creature_that_isnt_list_clause_rule_lexed,
        sentence_has_return_each_creature_that_isnt_list_clause
    ),
    (
        sentence_has_unsupported_negated_untap_clause_rule_lexed,
        sentence_has_unsupported_negated_untap_clause
    ),
);

pub(super) fn sentence_looks_like_supported_negated_untap_clause(tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::looks_like_supported_negated_untap_clause_lexed(tokens)
}

fn sentence_has_each_player_lose_discard_sacrifice_chain(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_each_player_lose_discard_sacrifice_chain_sentence_lexed(tokens)
}

fn sentence_has_each_player_exile_sacrifice_return_exiled_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_each_player_exile_sacrifice_return_exiled_clause_sentence_lexed(tokens)
}

fn sentence_has_put_one_of_them_into_hand_rest_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_put_one_of_them_into_hand_rest_clause_sentence_lexed(tokens)
}

fn sentence_has_loses_all_abilities_with_becomes_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_loses_all_abilities_with_becomes_clause_sentence_lexed(tokens)
}

fn sentence_has_spent_to_cast_this_spell_without_condition(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_spent_to_cast_this_spell_without_condition_sentence_lexed(tokens)
}

fn sentence_has_would_enter_instead_replacement_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_would_enter_instead_replacement_clause_sentence_lexed(tokens)
}

fn sentence_has_different_mana_value_constraint(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_different_mana_value_constraint_sentence_lexed(tokens)
}

fn sentence_has_most_common_color_constraint(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_most_common_color_constraint_sentence_lexed(tokens)
}

fn sentence_has_power_vs_count_constraint(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_power_vs_count_constraint_sentence_lexed(tokens)
}

fn sentence_has_put_into_graveyards_from_battlefield_this_turn(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_put_into_graveyards_from_battlefield_this_turn_sentence_lexed(tokens)
}

fn sentence_has_phase_out_until_leaves_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_phase_out_until_leaves_clause_sentence_lexed(tokens)
}

fn sentence_has_same_name_as_another_in_hand_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_same_name_as_another_in_hand_clause_sentence_lexed(tokens)
}

fn sentence_has_for_each_mana_from_spent_to_cast_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_for_each_mana_from_spent_to_cast_clause_sentence_lexed(tokens)
}

fn sentence_has_when_you_sacrifice_this_way_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_when_you_sacrifice_this_way_clause_sentence_lexed(tokens)
}

fn sentence_has_greatest_mana_value_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_greatest_mana_value_clause_sentence_lexed(words)
}

fn sentence_has_least_power_among_creatures_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_least_power_among_creatures_clause_sentence_lexed(words)
}

fn sentence_has_villainous_choice_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_villainous_choice_clause_sentence_lexed(tokens)
}

fn sentence_has_divided_evenly_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_divided_evenly_clause_sentence_lexed(words)
}

fn sentence_has_different_names_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_different_names_clause_sentence_lexed(words)
}

fn sentence_has_chosen_at_random_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_chosen_at_random_clause_sentence_lexed(words)
}

fn sentence_has_defending_players_choice_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_defending_players_choice_clause_sentence_lexed(tokens)
}

fn sentence_has_target_creature_token_player_planeswalker_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_target_creature_token_player_planeswalker_clause_sentence_lexed(tokens)
}

fn sentence_has_if_you_sacrifice_an_island_this_way_clause(
    words: &[&str],
    _: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_if_you_sacrifice_an_island_this_way_clause_sentence_lexed(words)
}

fn sentence_has_spent_to_cast_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_spent_to_cast_clause_sentence_lexed(words)
}

fn sentence_has_face_down_clause(words: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_face_down_clause_sentence_lexed(words, tokens)
}

fn sentence_has_return_each_creature_that_isnt_list_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_return_each_creature_that_isnt_list_clause_sentence_lexed(tokens)
}

fn sentence_has_unsupported_negated_untap_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_unsupported_negated_untap_clause_sentence_lexed(tokens)
}

fn parse_it_is_aura_enchantment_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = sentence_shapes::parse_aura_enchantment_tokens(tokens) else {
        return Ok(None);
    };
    let attachment_tokens = trim_edge_punctuation(shape.attachment_tokens);
    if attachment_tokens.is_empty() {
        return Ok(None);
    }
    let attachment_filter = if let Ok(filter) = parse_object_filter_lexed(&attachment_tokens, false)
    {
        filter
    } else if shape.attachment_mentions_you_control {
        ObjectFilter::creature().you_control()
    } else {
        ObjectFilter::creature()
    };
    let clause_words = crate::lexer::token_word_refs(shape.tail_tokens);
    let mut granted_abilities = Vec::new();
    for ability_tokens in shape.granted_ability_tokens {
        let ability_tokens = trim_edge_punctuation(ability_tokens);
        if ability_tokens.is_empty() {
            continue;
        }
        let (mut parsed, _) = super::gain_ability::parse_granted_abilities_for_gain_clause(
            &ability_tokens,
            &clause_words,
            false,
        )?;
        granted_abilities.append(&mut parsed);
    }
    let mut effects = vec![EffectAst::subject_verb_become_aura_enchantment_with_grants(
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic())),
        attachment_filter,
        granted_abilities,
        Until::Forever,
    )];

    if shape.loses_all_abilities {
        effects.push(EffectAst::subject_verb_remove_abilities_all(
            ObjectFilter::default(),
            Vec::new(),
            Until::Forever,
        ));
    }
    Ok(Some(effects))
}

fn parse_set_quantifier_surface(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::SetQuantifierSurface> {
    let words = crate::lexer::token_word_refs(tokens);
    let verb = words.iter().position(|word| {
        matches!(
            *word,
            "get"
                | "gets"
                | "gain"
                | "gains"
                | "lose"
                | "loses"
                | "have"
                | "has"
                | "attack"
                | "attacks"
                | "block"
                | "blocks"
                | "return"
                | "returns"
        )
    })?;
    let subject = if matches!(words[verb], "return" | "returns") {
        let object_end = words[verb + 1..]
            .iter()
            .position(|word| matches!(*word, "to" | "from"))
            .map_or(words.len(), |offset| verb + 1 + offset);
        &words[verb + 1..object_end]
    } else {
        &words[..verb]
    };
    if subject.contains(&"all") {
        Some(ironsmith_core::SetQuantifierSurface::All)
    } else if subject.contains(&"each") {
        Some(ironsmith_core::SetQuantifierSurface::Each)
    } else if subject.contains(&"those") {
        Some(ironsmith_core::SetQuantifierSurface::Those)
    } else {
        None
    }
}

fn parse_return_set_reference_surface(tokens: &[OwnedLexToken]) -> Option<String> {
    let words = crate::lexer::token_word_refs(tokens);
    let verb = words
        .iter()
        .position(|word| matches!(*word, "return" | "returns"))?;
    let object_end = words[verb + 1..]
        .iter()
        .position(|word| matches!(*word, "to" | "from"))
        .map_or(words.len(), |offset| verb + 1 + offset);
    let object = &words[verb + 1..object_end];
    let quantifier = object
        .iter()
        .position(|word| matches!(*word, "each" | "those"))?;
    Some(object[quantifier..].join(" "))
}

fn set_first_continuous_set_quantifier(
    effects: &mut [EffectAst],
    surface: ironsmith_core::SetQuantifierSurface,
) -> bool {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect {
            let slot = match action {
                SubjectVerbActionAst::Pump {
                    set_quantifier_surface,
                    ..
                }
                | SubjectVerbActionAst::PumpAll {
                    set_quantifier_surface,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesAll {
                    set_quantifier_surface,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesToTarget {
                    set_quantifier_surface,
                    ..
                }
                | SubjectVerbActionAst::RemoveAbilitiesAll {
                    set_quantifier_surface,
                    ..
                }
                | SubjectVerbActionAst::SetBasePowerToughness {
                    set_quantifier_surface,
                    ..
                }
                | SubjectVerbActionAst::ReturnToHand {
                    set_quantifier_surface,
                    ..
                } => Some(set_quantifier_surface),
                _ => None,
            };
            if let Some(slot) = slot {
                *slot = Some(surface);
                return true;
            }
        }

        let mut found = false;
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            if !found {
                found = set_first_continuous_set_quantifier(nested, surface);
            }
        });
        if found {
            return true;
        }
    }
    false
}

fn set_first_return_set_reference_surface(effects: &mut [EffectAst], surface: &str) -> bool {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ReturnToHand {
                    set_reference_surface,
                    ..
                },
            ..
        }) = effect
        {
            *set_reference_surface = Some(surface.to_string());
            return true;
        }

        let mut found = false;
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            if !found {
                found = set_first_return_set_reference_surface(nested, surface);
            }
        });
        if found {
            return true;
        }
    }
    false
}

fn parse_bounded_x_mana_payment_sentence(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let may_shape = effect_grammar::clause_dispatch_shapes::parse_leading_may_shape(tokens)?;
    let payment_shape = effect_grammar::misc_action_shapes::parse_bounded_x_payment_tokens(
        may_shape.effect_tokens,
    )?;
    let maximum = match payment_shape.maximum {
        effect_grammar::misc_action_shapes::BoundedXMaximumShape::TriggeringLifeGained => {
            Value::EventValue(EventValueSpec::LifeAmount)
        }
    };

    Some(vec![match may_shape.actor {
        effect_grammar::clause_dispatch_shapes::LeadingMayActorShape::Player(player) => {
            EffectAst::MayByPlayer {
                player,
                effects: vec![EffectAst::subject_verb_pay_mana_up_to(
                    player,
                    payment_shape.cost,
                    maximum,
                )],
            }
        }
        effect_grammar::clause_dispatch_shapes::LeadingMayActorShape::Implicit => EffectAst::May {
            effects: vec![EffectAst::subject_verb_pay_mana_up_to(
                PlayerAst::You,
                payment_shape.cost,
                maximum,
            )],
        },
    }])
}

fn parse_gain_ability_before_effect_chain(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = super::gain_ability::parse_gain_ability_sentence(tokens)? {
        return Ok(effects);
    }
    super::parse_effect_chain_lexed(tokens)
}

/// Return whether a quoted outer gain carries an authored `unless` tail.
///
/// The broad restriction dispatcher scans the whole token stream for a
/// negation, including inside quoted granted rules. For
/// `<subject> gains "... can't ..." until ... unless ...`, that nested
/// negation belongs to the granted ability rather than the outer sentence.
/// Requiring an outer gain verb before the opening quote and `unless` after
/// the closing quote keeps this preemption tied to the complete gain grammar.
fn quoted_gain_has_trailing_unless(tokens: &[OwnedLexToken]) -> bool {
    let Some(open_quote) = tokens.iter().position(OwnedLexToken::is_quote) else {
        return false;
    };
    let Some(close_quote) = tokens.iter().rposition(OwnedLexToken::is_quote) else {
        return false;
    };
    open_quote < close_quote
        && tokens[..open_quote]
            .iter()
            .any(|token| token.is_any_word(&["gain", "gains", "have", "has", "lose", "loses"]))
        && tokens[close_quote + 1..]
            .iter()
            .any(|token| token.is_word("unless"))
}

/// Preserve an ordered token-creation/spell-copy pair after document
/// sentence normalization has consumed the comma before `then`.
///
/// Both arms must independently lower to the typed actions, so an unrelated
/// descriptive `then` tail cannot be mistaken for a second effect.
fn parse_create_token_then_copy_spell_chain(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(then_idx) = tokens.iter().position(|token| token.is_word("then")) else {
        return Ok(None);
    };
    let create_tokens = trim_edge_punctuation(&tokens[..then_idx]);
    let copy_tokens = trim_edge_punctuation(&tokens[then_idx + 1..]);
    if !create_tokens
        .first()
        .is_some_and(|token| token.is_word("create"))
        || !copy_tokens
            .first()
            .is_some_and(|token| token.is_word("copy"))
    {
        return Ok(None);
    }

    // Parse the two grammar-proven arms directly. The general chain parser is
    // allowed to normalize and wrap an isolated arm for carry semantics, but
    // this specialist needs one exact producer followed by one exact copy.
    // The leading-token guards above make the dedicated action parsers the
    // narrowest reusable boundary for this ordered pair.
    let create_effect = super::creation_handlers::parse_create(&create_tokens[1..], None)?;
    if !matches!(
        &create_effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CreateTokenWithMods { .. },
            ..
        })
    ) {
        return Ok(None);
    }
    let Some(copy_effect) = super::clause_pattern_helpers::parse_copy_spell_clause(&copy_tokens)?
    else {
        return Ok(None);
    };
    if !matches!(
        copy_effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell { .. },
            ..
        })
    ) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::CommaThen {
        effects: vec![create_effect, copy_effect],
    }]))
}

/// Split an explicit no-combat-damage action from a preceding action whose
/// object filter may itself contain an authored `and` list.
///
/// The general semantic chain splitter deliberately preserves conjunctions
/// inside target phrases. That is normally correct, but a complete trailing
/// `this creature assigns no combat damage this turn` clause is independently
/// grammar-proven and must not be absorbed into a broad destroy target. Both
/// arms still have to lower on their own before this route claims the line.
fn parse_explicit_assign_no_combat_damage_followup(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(prefix) = split_leading_result_prefix_lexed(tokens)
        && let Some(effects) =
            parse_explicit_assign_no_combat_damage_followup(prefix.trailing_tokens)?
    {
        return Ok(Some(vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects,
            },
        }]));
    }

    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_word("and") {
            continue;
        }
        let first = trim_edge_punctuation(&tokens[..idx]);
        let second = trim_edge_punctuation(&tokens[idx + 1..]);
        if first.is_empty()
            || second.is_empty()
            || !matches!(
                effect_grammar::clause_dispatch_shapes::parse_assigns_no_combat_damage_shape(
                    &second,
                ),
                Some(
                    effect_grammar::clause_dispatch_shapes::AssignsNoCombatDamageShape::Supported { .. }
                )
            )
        {
            continue;
        }

        let mut effects = parse_effect_sentence_lexed(&first)?;
        let mut followup = parse_effect_sentence_lexed(&second)?;
        if effects.is_empty() || followup.is_empty() {
            continue;
        }
        effects.append(&mut followup);
        return Ok(Some(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]));
    }
    Ok(None)
}

pub fn parse_effect_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    // A coordinated zone-pair declaration also contains the words
    // `target player`, so the generic complete-target preemption below can
    // otherwise claim it and ask the ordinary target parser to interpret the
    // trailing graveyard as part of one object target. Preserve the strict
    // typed two-zone bundle before entering that broader route.
    if let Some(effects) =
        super::search_library::parse_exile_hand_and_graveyard_bundle_sentence(tokens)?
    {
        return Ok(effects);
    }
    // Triggered, activated, and modal bodies can enter this single-sentence
    // dispatcher without passing through the document-level sentence loop.
    // A complete target declaration containing a relative history clause
    // ("cards ... that were put there") must remain one typed target effect;
    // otherwise the subject/verb planner can reinterpret the relative `put`
    // as a zone-change action.
    if let Some(shape) = effect_grammar::clause_dispatch_shapes::parse_choose_target_shape(tokens)
        && parse_target_phrase(shape.target_tokens).is_ok()
    {
        return Ok(vec![super::parse_effect_clause_lexed(tokens)?]);
    }
    // Triggered and activated line parsers can enter this single-sentence
    // dispatcher directly, bypassing the document-level pass that normally
    // hides quoted token rules from outer effect-chain parsing. Keep those
    // rule bodies inside the token blueprint: parse the create action from
    // the stripped surface, then reattach every quoted ability under the
    // token's own source identity.
    let stripped_tokens = strip_embedded_token_rules_text(tokens);
    let has_embedded_token_rules = stripped_tokens.len() != tokens.len();
    let parse_tokens = if has_embedded_token_rules {
        stripped_tokens.as_slice()
    } else {
        tokens
    };
    let mut effects = crate::stack::maybe_grow(32 * 1024 * 1024, 64 * 1024 * 1024, || {
        if let Some(effects) = parse_prefix_then_look_at_top_exile_one(parse_tokens)? {
            Ok(effects)
        } else if let Some(effects) = parse_bounded_x_mana_payment_sentence(parse_tokens) {
            Ok(effects)
        } else {
            parse_effect_sentence_lexed_inner(parse_tokens)
        }
    })?;
    if has_embedded_token_rules {
        super::creation_handlers::attach_inline_token_granted_abilities_to_last_create(
            &mut effects,
            tokens,
        );
    }
    if let Some(surface) = parse_set_quantifier_surface(parse_tokens) {
        set_first_continuous_set_quantifier(&mut effects, surface);
    }
    if let Some(surface) = parse_return_set_reference_surface(parse_tokens) {
        set_first_return_set_reference_surface(&mut effects, &surface);
    }
    Ok(crate::effect_sentences::preserve_coordinated_effect_chain_surface(parse_tokens, effects))
}

fn parse_prefix_then_look_at_top_exile_one(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    for then_idx in (1..tokens.len()).filter(|idx| tokens[*idx].is_word("then")) {
        let prefix = trim_edge_punctuation(&tokens[..then_idx]);
        let followup = trim_edge_punctuation(&tokens[then_idx + 1..]);
        if prefix.is_empty() || followup.is_empty() {
            continue;
        }
        let Some(mut looked) = parse_look_at_top_then_exile_one_sentence(&followup)? else {
            continue;
        };
        let mut effects = parse_effect_sentence_lexed_inner(&prefix)?;
        if effects.is_empty() {
            continue;
        }
        effects.append(&mut looked);
        return Ok(Some(effects));
    }
    Ok(None)
}

fn has_unrecognized_leading_effect_label(tokens: &[OwnedLexToken]) -> bool {
    if crate::grammar::structure::split_leading_result_prefix_lexed(tokens).is_some() {
        return false;
    }
    effect_grammar::labeled_dispatch::parse_leading_effect_label_tokens(tokens).is_some_and(
        |shape| shape.kind == effect_grammar::labeled_dispatch::LeadingEffectLabelKind::Unknown,
    )
}

fn parse_manifest_dread_graveyard_card_to_hand(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let words = crate::lexer::token_word_refs(tokens);
    if words
        != [
            "put",
            "a",
            "card",
            "you",
            "put",
            "into",
            "your",
            "graveyard",
            "this",
            "way",
            "into",
            "your",
            "hand",
        ]
    {
        return None;
    }

    let mut filter = ObjectFilter::tagged(TagKey::from(crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG));
    filter.zone = Some(Zone::Graveyard);
    Some(vec![EffectAst::subject_verb_move_to_zone(
        TargetAst::Object(filter, None, None),
        Zone::Hand,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    )])
}

fn parse_effect_sentence_lexed_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    crate::stack::maybe_grow(32 * 1024 * 1024, 64 * 1024 * 1024, || {
        parse_effect_sentence_lexed_inner_unstacked(tokens)
    })
}

fn parse_attacking_doesnt_tap_if_source_untapped(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let action_tokens = crate::token_primitives::strip_leading_if_you_do_lexed(tokens);
    let wrapped_if_result = action_tokens.len() < tokens.len();
    let action_tokens = trim_commas(action_tokens);
    let Some(shape) =
        sentence_shapes::parse_attacking_doesnt_tap_if_source_untapped_tokens(&action_tokens)
    else {
        return Ok(None);
    };
    let filter = parse_object_filter(shape.affected_tokens, false)?;
    let effects = vec![
        EffectAst::subject_verb_grant_abilities_all_dynamically_with_condition(
            filter,
            vec![crate::cards::builders::GrantedAbilityAst::StaticAbility(
                crate::static_abilities::StaticAbility::vigilance(),
            )],
            Until::EndOfCombat,
            crate::ConditionExpr::SourceIsUntapped,
        ),
    ];
    if wrapped_if_result {
        return Ok(Some(vec![EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects,
        }]));
    }
    Ok(Some(effects))
}

fn bind_numeric_result_counter_amounts(effects: &mut [EffectAst]) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutCounters { count, .. },
            ..
        }) = effect
            && matches!(
                count,
                Value::EventValue(crate::effect::EventValueSpec::Amount)
            )
        {
            *count = Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::Count,
            };
        }
        crate::model::visit::for_each_nested_effects_mut(
            effect,
            true,
            bind_numeric_result_counter_amounts,
        );
    }
}

/// Parse a conjunction only when every arm independently proves either an
/// executable action or a complete negated restriction, and both kinds are
/// present.
///
/// This proof is shared by the direct sentence dispatcher and the complete
/// effect-body entrypoint. The latter probes tolerant whole-body specialists
/// before dispatching individual sentences; without this earlier bridge, a
/// broad restriction parser can absorb a preceding animation into the
/// restriction's subject filter.
pub(super) fn parse_fully_typed_mixed_restriction_action_chain(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut saw_restriction = false;
    let mut saw_affirmative_action = false;
    for (segment_idx, segment) in segments.iter().enumerate() {
        if super::super::activation_and_restrictions::find_negation_span(segment).is_some() {
            let standalone_restriction = parse_cant_effect_sentence_lexed(segment)?.is_some();
            let shared_subject_restriction = if !standalone_restriction && segment_idx > 0 {
                let previous = segments[segment_idx - 1];
                if let Some((_, verb_idx)) = super::lex_chain_helpers::find_verb_lexed(previous) {
                    let subject = &previous[..verb_idx];
                    if effect_grammar::chain_carry::parse_carryable_subject_tokens(subject)
                        .is_some()
                    {
                        let mut expanded = subject.to_vec();
                        expanded.extend(segment.iter().cloned());
                        parse_cant_effect_sentence_lexed(&expanded)?.is_some()
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };
            if standalone_restriction || shared_subject_restriction {
                saw_restriction = true;
            } else {
                return Ok(None);
            }
        } else if super::lex_chain_helpers::segment_has_effect_head_lexed(segment) {
            saw_affirmative_action = true;
        } else {
            return Ok(None);
        }
    }

    if saw_restriction && saw_affirmative_action {
        // A bare prohibition after an animation inherits the animation's
        // subject ("this artifact becomes ... and can't be blocked").  The
        // ordinary chain parser cannot discover a verb in that second arm,
        // so its tolerant whole-clause `can't` fallback can absorb the
        // animation words into the restriction's object filter.  At this
        // point the two-arm shape and both typed halves have already been
        // proved; lower the animation and the expanded shared-subject
        // prohibition independently instead of asking the generic chain
        // heuristic to rediscover the boundary.
        if let [affirmative, restriction] = segments.as_slice()
            && let Some((super::Verb::Become, verb_word_idx)) =
                super::lex_chain_helpers::find_verb_lexed(affirmative)
        {
            let affirmative_words = TokenWordView::new(affirmative);
            let Some(verb_token_idx) =
                affirmative_words.token_boundary_for_word_or_end(verb_word_idx)
            else {
                return Ok(None);
            };
            let Some(body_token_idx) =
                affirmative_words.token_boundary_for_word_or_end(verb_word_idx + 1)
            else {
                return Ok(None);
            };
            let subject = trim_edge_punctuation(&affirmative[..verb_token_idx]);
            let body = trim_edge_punctuation(&affirmative[body_token_idx..]);
            if !subject.is_empty() && !body.is_empty() {
                let animation = super::clause_dispatch::parse_become_clause(&subject, &body)?;
                let mut expanded_restriction = subject;
                expanded_restriction.extend(restriction.iter().cloned());
                if let Some(mut restrictions) =
                    parse_cant_effect_sentence_lexed(&expanded_restriction)?
                {
                    let mut effects = Vec::with_capacity(1 + restrictions.len());
                    effects.push(animation);
                    effects.append(&mut restrictions);
                    return Ok(Some(vec![EffectAst::Coordinated {
                        effects,
                        leading_duration: false,
                        result_conjunction: false,
                    }]));
                }
            }
        }

        // The ordinary chain entrypoint decides whether to split from the
        // number of independently recognized effect heads. A subjectless
        // restriction arm ("and can't be blocked") deliberately has no
        // standalone head, so that heuristic can send this already-proven
        // mixed shape back to the broad `can't` parser and lose the leading
        // animation. Force the segmented carry route now that every arm and
        // the mixed-kind invariant have been proved above, then restore the
        // authored coordination surface.
        let effects = super::chain_carry::parse_effect_chain_inner_lexed(tokens)?;
        return Ok(Some(
            super::chain_carry::preserve_coordinated_effect_chain_surface(tokens, effects),
        ));
    }
    Ok(None)
}

fn parse_effect_sentence_lexed_inner_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    // A trailing subjectless permission belongs to the same subject as the
    // preceding action: "this creature gets ... and can attack ... as though
    // it didn't have defender."  The standalone permission recognizer cannot
    // treat the whole prefix as an object subject, while the broad effect
    // chain otherwise mistakes the comparison's `have defender` for an
    // ability grant. Split only the grammar-proven final `and can attack`
    // clause, parse the prefix normally, and reattach the typed permission to
    // the prefix's explicit subject.
    let words = crate::lexer::parser_token_word_refs(tokens);
    if let Some(and_word_idx) = words
        .windows(3)
        .rposition(|window| window == ["and", "can", "attack"])
    {
        let word_view = TokenWordView::new(tokens);
        let and_token_idx = word_view
            .token_boundary_for_word_or_end(and_word_idx)
            .unwrap_or(tokens.len());
        let can_token_idx = word_view
            .token_boundary_for_word_or_end(and_word_idx + 1)
            .unwrap_or(tokens.len());
        let prefix = trim_edge_punctuation(&tokens[..and_token_idx]);
        if !prefix.is_empty()
            && let Some((_, verb_word_idx)) = super::lex_chain_helpers::find_verb_lexed(&prefix)
        {
            let prefix_words = TokenWordView::new(&prefix);
            let subject_end = prefix_words
                .token_boundary_for_word_or_end(verb_word_idx)
                .unwrap_or(prefix.len());
            let subject = trim_edge_punctuation(&prefix[..subject_end]);
            if !subject.is_empty() {
                // The trailing clause is deliberately subjectless. Bind it to
                // the preceding effect-chain result (`it`) rather than
                // reparsing the original `target ...` phrase as an untargeted
                // all-objects filter.
                if let Some(permission) =
                    parse_can_attack_as_though_no_defender_clause(&tokens[can_token_idx..])?
                {
                    let parsed_prefix = parse_effect_sentence_lexed(&prefix)?;
                    if !parsed_prefix.is_empty() {
                        let mut coordinated = Vec::new();
                        for effect in parsed_prefix {
                            match effect {
                                EffectAst::Coordinated { effects, .. } => {
                                    coordinated.extend(effects);
                                }
                                effect => coordinated.push(effect),
                            }
                        }
                        coordinated.push(permission);
                        return Ok(vec![EffectAst::Coordinated {
                            effects: coordinated,
                            leading_duration: false,
                            result_conjunction: false,
                        }]);
                    }
                }
            }
        }
    }
    // This permission contains the words `have defender`, but those words are
    // inside an `as though` comparison rather than an ability to grant. Claim
    // the complete typed combat-permission clause before the broad gain-
    // ability routes can reduce it to granting defender itself.
    if let Some(effect) = parse_can_attack_as_though_no_defender_clause(tokens)? {
        return Ok(vec![effect]);
    }
    // Keep a demonstrative per-object reward ahead of the broad gain-life
    // sentence parser. The latter can otherwise reduce "the controller of
    // each of those artifacts" to the ability controller and discard both
    // the iteration and prior-result provenance.
    if let Some(effect) =
        super::chain_carry::parse_each_prior_affected_object_controller_mana_value_life(tokens)
    {
        return Ok(vec![effect]);
    }
    if let Some(effects) = parse_destroy_attached_object_then_source_damage_to_controller(tokens)? {
        return Ok(effects);
    }
    // This grammar-proven cast-origin grant must own the complete sentence.
    // Later gain-ability routes permissively accept the leading `as you cast`
    // phrase as an object-filter subject and lose both the hand provenance
    // and authored duration surface.
    if let Some(effect) = parse_as_you_cast_from_zone_this_turn_grant(tokens)? {
        return Ok(vec![effect]);
    }
    // Delayed payment clauses can name any supported next step. Route the
    // complete sentence before broad subject/verb parsing splits at `unless`;
    // otherwise an action-first draw-step clause is reduced to a life-loss
    // action with an unsupported timing tail. Parsing the action prefix
    // recurses with the timing marker removed, so this route terminates.
    if let Some(effects) =
        parse_sentence_delayed_next_step_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(effects);
    }
    if let Some(effects) = parse_attacking_doesnt_tap_if_source_untapped(tokens)? {
        return Ok(effects);
    }
    // A trailing condition on a mass-destruction instruction governs whether
    // that instruction happens. Route the grammar-proven condition before the
    // broad destroy subject/verb primitive can consume only its leading
    // action and silently discard the predicate.
    if crate::grammar::structure::split_trailing_if_clause_lexed(tokens).is_some()
        && let Ok(effect) = super::chain_carry::parse_effect_clause_with_trailing_if_lexed(tokens)
        && matches!(
            &effect,
            EffectAst::TrailingIf {
                effects,
                ..
            } if matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::DestroyAll { .. },
                    ..
                })]
            )
        )
    {
        return Ok(vec![effect]);
    }
    if let Some(effects) =
        super::player_subject_sequences::parse_each_player_exile_sacrifice_return_exiled(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effect) =
        super::chain_carry::parse_may_have_any_number_tagged_phase_out_lexed(tokens)
    {
        return Ok(vec![effect]);
    }
    if let Some(effects) = super::dispatch_entry::parse_if_you_dont_sentence(tokens)? {
        return Ok(vec![EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::ExplicitDidNot,
            effects,
        }]);
    }
    if let Some(diag) = super::sentence_unsupported::diagnose_known_partial_parse_lexed(tokens) {
        return Err(diag);
    }

    // A causative `unless its controller has <source> deal ...` clause is one
    // action choice. The broad subject/verb recognizer can otherwise claim
    // only the embedded damage phrase and discard both the primary action
    // and the `unless` relationship.
    if let Some(effects) = super::subject_verb_primitives::
        parse_sentence_damage_unless_controller_has_source_deal_damage(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(effects);
    }
    // Shared-characteristic fanouts are one linked target set. In particular,
    // a broad destroy parser must not reduce `target enchantment and each
    // other enchantment that shares a color with it` to two unrelated
    // targets before the typed relation is recorded.
    if let Some(effects) = super::fanout_family::parse_shared_color_target_fanout_sentence(tokens)?
    {
        return Ok(effects);
    }

    // A keyword-bundle pump contains an authored `and so on for ...` list,
    // not a conjunction of executable actions. Route that complete typed
    // shape before the broad leading-duration chain predicate can split it
    // into only the first two conditional pump clauses.
    if let Some(effects) =
        super::subject_verb_special_recognizers::parse_keyword_bundle_pump_sentence(tokens)?
    {
        return Ok(effects);
    }

    // A genuine top-level conjunction with a leading duration needs chain
    // carry before broad gain/subject recognizers see an isolated arm. The
    // grammar predicate rejects quoted/list conjunctions and `then` chains.
    if effect_grammar::chain_carry::coordinated_effect_chain_leading_duration(tokens) == Some(true)
    {
        return super::parse_effect_chain_lexed(tokens);
    }

    let quoted_ability_shape = sentence_shapes::parse_quoted_ability_sentence_tokens(tokens);
    // A quoted restriction is payload of the outer gain, not a top-level
    // restriction. Keep its duration and trailing `unless` together before
    // the broad `can't` route sees the nested negation.
    if quoted_ability_shape.is_some()
        && quoted_gain_has_trailing_unless(tokens)
        && let Some(effects) = super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(effects);
    }

    if let Some(effects) = parse_explicit_assign_no_combat_damage_followup(tokens)? {
        return Ok(effects);
    }

    // A source pump followed by `can't be blocked this turn` is one shared-
    // subject program. Preserve both typed effects before any generic chain
    // or prohibition route can reinterpret the leading source/pump words as
    // the blocked-object filter and silently retain only the restriction.
    if let Some(effects) = parse_source_gets_unblockable_subject_verb(tokens)? {
        return Ok(effects);
    }
    // The target variant is the same atomic program: one explicit target,
    // one P/T modification, and a same-target blocking restriction. Give it
    // the same early ownership as the source form so the broad pump route
    // cannot accept only the leading `gets ...` clause and discard the
    // coordinated `can't be blocked this turn` tail.
    if let Some(effects) = parse_target_gets_unblockable_subject_verb(tokens)? {
        return Ok(effects);
    }

    // An explicit subject and executable verb after a coordinated `and`
    // starts a new action, even when the leading action has a broad target-
    // list recognizer. Without this proof, phrases such as `destroy target
    // artifact ... and this creature assigns no combat damage` can be
    // swallowed as one malformed destroy-target union. The semantic splitter
    // keeps type/color conjunctions inside their filters, while the explicit
    // effect-head requirement excludes ordinary target lists.
    let explicit_action_segments =
        super::lex_chain_helpers::split_effect_chain_on_and_lexed(tokens);
    if explicit_action_segments.len() >= 2
        && explicit_action_segments
            .iter()
            .all(|segment| super::lex_chain_helpers::segment_has_effect_head_lexed(segment))
    {
        return super::parse_effect_chain_lexed(tokens);
    }

    // A mixed restriction/action conjunction belongs to the shared-subject
    // chain. The broad top-level `can't` recognizer can otherwise accept the
    // first arm and silently discard a following affirmative action such as
    // `becomes ...`. Prove both halves independently before preempting it:
    // every negated arm must be a complete typed restriction, every other
    // arm must begin with an executable effect head, and both kinds must be
    // present. This deliberately excludes pure coordinated restrictions.
    // Use the semantic chain splitter rather than a raw `and` split so an
    // internal characteristic list such as `blue and black` remains inside
    // the animation arm.
    if let Some(effects) = parse_fully_typed_mixed_restriction_action_chain(tokens)? {
        return Ok(effects);
    }

    // Pure coordinated restrictions are already fully understood by the
    // cant grammar. Route them before a broad subject parser can claim the
    // object of the first restriction (for example, `life`) as a new subject.
    // Requiring negation in every top-level arm leaves mixed `can't ... and
    // gain ...` clauses to the coordinated chain route.
    let cant_segments = split_lexed_slices_on_and(tokens);
    if !cant_segments.is_empty()
        && cant_segments.iter().all(|segment| {
            super::super::activation_and_restrictions::find_negation_span(segment).is_some()
        })
        && let Some(effects) = parse_cant_effect_sentence_lexed(tokens)?
    {
        return Ok(effects);
    }

    // These shapes must be recognized before the broad sentence-shape
    // predicates below. Otherwise a result-prefixed sentence can be claimed
    // by generic target parsing, and a leading roll clause can be reduced to
    // the unsupported `two d6` fragment.
    if let Some(effect_grammar::SentencePreludeShape::RollDiceChooseOneResult {
        count,
        sides,
        die_text,
    }) = effect_grammar::parse_sentence_prelude_shape_tokens(tokens)
    {
        return Ok(vec![
            EffectAst::subject_verb_roll_dice_choose_result_with_die_text(
                PlayerAst::Implicit,
                count,
                sides,
                Some(die_text),
            ),
        ]);
    }

    // A result gate can govern an action that is scheduled for a later step,
    // as in "If you do, unattach it at the beginning of the next end step."
    // Preserve that timing before the broad result-prefix route strips the
    // suffix and parses only the immediate action. The delayed parser keeps
    // the result gate outside the scheduled payload, so it can still bind to
    // the preceding optional effect.
    if let Some(effects) =
        parse_sentence_delayed_timing_suffix(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(effects);
    }

    if let Some(prefix) = split_leading_result_prefix_lexed(tokens) {
        let mut trailing_effects = super::parse_effect_chain_inner_lexed(prefix.trailing_tokens)?;
        if matches!(
            &prefix.predicate,
            crate::cards::builders::IfResultPredicate::Value(_)
        ) {
            bind_numeric_result_counter_amounts(&mut trailing_effects);
        }
        let mut result = vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
        }];
        super::preserve_leading_result_coordination_lexed(tokens, &mut result);
        return Ok(result);
    }

    // Explicit player offers must retain both their actor and optionality
    // before broad subject/verb parsing claims the action. This is especially
    // important for split actors such as "that player or that permanent's
    // controller may ...", whose second branch is otherwise discarded.
    if super::parse_leading_player_may_lexed(tokens).is_some() {
        // A singular immediate "you may cast it" instruction is a choice
        // made during resolution, not a persistent cast permission. Keep the
        // explicit May wrapper before the broader tagged-permission parser
        // below gets a chance to lower only the cast action.
        if let Some(spec) = parse_may_cast_it_sentence(tokens) {
            return Ok(vec![build_may_cast_tagged_effect(&spec)]);
        }
        // A tagged play/cast permission may itself contain a second authored
        // "you may" in its mana-spending rider. Preserve that complete typed
        // permission before generic chain splitting treats the rider as an
        // unrelated `spend` action.
        if let Some(effect) = crate::permission_helpers::parse_cast_or_play_tagged_clause(tokens)? {
            return Ok(vec![effect]);
        }
        return super::parse_effect_chain_lexed(tokens);
    }

    fn search_followup_shuffle_player(effect: &EffectAst) -> Option<PlayerAst> {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::SearchLibrary { player, .. },
                ..
            }) => Some(*player),
            _ => None,
        }
    }

    fn normalize_search_followup_shuffles(effects: &mut [EffectAst]) {
        for idx in 0..effects.len() {
            let is_default_shuffle = matches!(
                effects.get(idx),
                Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject,
                    action: SubjectVerbActionAst::ShuffleLibrary,
                }))
                    if matches!(subject.player, PlayerAst::You | PlayerAst::Implicit)
            );
            if !is_default_shuffle {
                continue;
            }
            let Some(search_player) = effects[..idx]
                .iter()
                .rev()
                .find_map(search_followup_shuffle_player)
            else {
                continue;
            };
            if !matches!(search_player, PlayerAst::You | PlayerAst::Implicit)
                && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject,
                    action: SubjectVerbActionAst::ShuffleLibrary,
                }) = &mut effects[idx]
                {
                    subject.player = search_player;
                }
        }
    }

    // A duration-scoped trigger may itself contain a damage action. Preserve
    // the grammar-proven outer `Until ..., whenever ...` scope before the
    // broad damage recognizers examine the whole sentence as a direct action.
    // The trigger parser recursively dispatches only the smaller payload, so
    // this route cannot claim an ordinary leading-duration continuous effect.
    if let Some(effect) = super::clause_primitives::parse_until_duration_triggered_clause(tokens)? {
        return Ok(vec![effect]);
    }

    // A delayed trigger may contain a compound damage fanout as its payload.
    // Preserve the outer `whenever ... this turn` scope before the broad
    // fanout recognizer examines the whole sentence as a direct action.
    if let Some(effects) = parse_sentence_delayed_trigger_this_turn(tokens)? {
        return Ok(effects);
    }

    if let Some(effects) = super::fanout_family::parse_compound_damage_fanout_sentence(tokens)? {
        return Ok(effects);
    }

    // A comma-following consequence is sometimes presented as `Then if ...`
    // when it is parsed in isolation from the preceding sentence. Treat the
    // sequencing marker as surface glue before the conditional grammar runs;
    // otherwise the generic dispatcher detaches the iterator subject from
    // its comma-delimited effect payload.
    let conditional_tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
        &tokens[1..]
    } else {
        tokens
    };
    if let Some(effects) = parse_player_villainous_choice_statement(conditional_tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = super::bundle_rules::parse_consult_disposition_bundle(tokens) {
        return Ok(effects);
    }
    if let Some(effect) =
        super::dispatch_entry::future_zone_replacement_from_sentence_tokens(tokens)
    {
        return Ok(vec![effect]);
    }
    if let Some(schedule) =
        effect_grammar::delayed_sentence_shapes::parse_delayed_schedule_sentence_shape(tokens)
    {
        let effects = parse_effect_sentence_lexed_inner(schedule.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "delayed schedule sentence missing effect payload".to_string(),
            ));
        }
        let delayed = match schedule.step {
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::UntapStep => {
                EffectAst::DelayedUntilNextUntapStep {
                    player: schedule.player,
                    effects,
                }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::Upkeep => {
                EffectAst::DelayedUntilNextUpkeep {
                    player: schedule.player,
                    effects,
                }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::DrawStep => {
                EffectAst::DelayedUntilNextDrawStep {
                    player: schedule.player,
                    effects,
                }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::MainPhase => {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                    PlayerAst::That => PlayerFilter::IteratedPlayer,
                    PlayerAst::Target => PlayerFilter::target_player(),
                    PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                    _ => PlayerFilter::Any,
                };
                EffectAst::DelayedUntilNextMainPhase { player, effects }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::FirstMainPhase => {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                    PlayerAst::That => PlayerFilter::IteratedPlayer,
                    PlayerAst::Target => PlayerFilter::target_player(),
                    PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                    _ => PlayerFilter::Any,
                };
                EffectAst::DelayedUntilNextFirstMainPhase { player, effects }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::EndStep
                if schedule.start_next_turn =>
            {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerAst::You,
                    PlayerAst::That => PlayerAst::That,
                    PlayerAst::Target => PlayerAst::Target,
                    PlayerAst::TargetOpponent => PlayerAst::TargetOpponent,
                    _ => PlayerAst::Any,
                };
                EffectAst::DelayedUntilEndStepOfExtraTurn { player, effects }
            }
            effect_grammar::delayed_sentence_shapes::DelayedScheduleStep::EndStep => {
                let player = match schedule.player {
                    PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                    PlayerAst::That => PlayerFilter::IteratedPlayer,
                    PlayerAst::Target => PlayerFilter::target_player(),
                    PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                    _ => PlayerFilter::Any,
                };
                EffectAst::DelayedUntilNextEndStep { player, effects }
            }
        };
        return Ok(vec![delayed]);
    }
    if let Some(effects) =
        super::subject_verb_primitives::parse_sentence_you_and_attacking_player_each_draw_and_lose(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(effects);
    }
    if conditional_tokens.first().is_some_and(|token| token.is_word("if"))
        && let Some(effects) = super::subject_verb_primitives::
            parse_if_any_tagged_cards_share_card_type_with_triggering_spell(
                SubjectVerbPrimitiveClause::new(conditional_tokens),
            )?
    {
        return Ok(effects);
    }
    if conditional_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
        && let Some(effects) =
            super::subject_verb_primitives::parse_if_enters_with_additional_counter_sentence(
                SubjectVerbPrimitiveClause::new(conditional_tokens),
            )?
    {
        return Ok(effects);
    }
    // The damage-replacement counter form begins with `If`, but its leading
    // clause describes an event rather than a state predicate. Route the
    // typed subject/verb recognizer before the generic conditional parser
    // attempts to interpret that event as a predicate.
    if let Some(effect) = parse_generic_damage_replacement_counters_subject_verb(tokens)? {
        return Ok(vec![effect]);
    }
    if conditional_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
        && let Some(effects) =
            parse_conditional_sentence_family_lexed(conditional_tokens, parse_effect_chain_lexed)?
    {
        return Ok(effects);
    }

    // Redirect clauses begin with an affected-object phrase rather than a
    // normal subject/verb pair (`All damage ... is dealt ...`). Dispatch the
    // typed redirect grammar before the generic extension parser reports a
    // missing verb.
    if let Some(effects) =
        super::clause_pattern_helpers::parse_redirect_next_damage_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        super::clause_pattern_helpers::parse_prevent_next_time_damage_sentence(tokens)?
    {
        return Ok(effects);
    }
    // Choice-complement sentences also look like ordinary subject/verb
    // clauses. Route the typed grammar before generic subject recognition can
    // interpret the `then` complement as a separate mechanic marker.
    let dispatch_shape = effect_grammar::labeled_dispatch::parse_labeled_dispatch_shape(tokens);
    if dispatch_shape.each_player_choose
        && let Some(effect) = parse_choice_complement_subject_verb(tokens)?
    {
        return Ok(vec![effect]);
    }

    if let Some(effect) = crate::permission_helpers::parse_cast_or_play_tagged_clause(tokens)? {
        return Ok(vec![effect]);
    }

    // This complete producer/copy chain must precede the intentionally
    // tolerant standalone copy parser below. That parser can locate a later
    // `copy that spell` clause inside surrounding text, but claiming this
    // exact chain there would discard the token-producing first arm.
    if let Some(effects) = parse_create_token_then_copy_spell_chain(tokens)? {
        crate::parse_trace::event(
            "effect-route: create-token-then-copy after punctuation normalization",
        );
        return Ok(effects);
    }

    // A complete spell-copy clause can contain an `except that the copy is`
    // characteristic modifier.  Route that typed action before broad
    // subject/verb recognition sees only the modifier's final `is` verb and
    // turns the source spell itself into a continuous color-setting effect.
    if let Some(effect) = super::clause_pattern_helpers::parse_copy_spell_clause(tokens)? {
        return Ok(vec![effect]);
    }

    if let Some(effects) =
        super::subject_verb_special_recognizers::parse_keyword_bundle_pump_sentence(tokens)?
    {
        return Ok(effects);
    }

    if let Some(effects) =
        super::subject_verb_special_recognizers::parse_scaled_target_power_sentence(tokens)?
    {
        return Ok(effects);
    }

    if let Some(effects) = parse_next_spell_grant_sentence_lexed(tokens)? {
        return Ok(effects);
    }

    // Matching-spell cost reductions can also be phrased with a leading
    // duration ("Until your next turn, ... spells ... cost ... less"). Give
    // that typed shape precedence over the generic chain parser, which can
    // otherwise reinterpret the spell restriction as a static ability grant
    // to an inferred object.
    if let Some(effect) = lower_matching_spell_cost_reduction_sentence(tokens) {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Cost subject=spell recognizer=matching-spell-reduction",
        );
        return Ok(vec![effect]);
    }

    if let Some(effects) = parse_manifest_dread_graveyard_card_to_hand(tokens) {
        return Ok(effects);
    }

    if let Some(effects) =
        parse_sentence_delayed_timing_suffix(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(effects);
    }

    if let Some(shape) = effect_grammar::parse_spell_cast_this_way_tax_tokens(tokens) {
        let mut spell_filter = ObjectFilter::spell().without_type(crate::types::CardType::Land);
        spell_filter.zone = None;
        if let Some(caster) = shape.taxed_caster {
            spell_filter.cast_by = Some(caster);
        }
        return Ok(vec![EffectAst::subject_verb_grant_to_target(
            TargetAst::Tagged(TagKey::from(IT_TAG), None),
            crate::grant::Grantable::Ability(crate::static_abilities::StaticAbility::new(
                crate::static_abilities::CostIncreaseManaCost::new(
                    spell_filter,
                    shape.additional_cost,
                ),
            )),
            crate::grant::GrantDuration::Forever,
        )]);
    }

    if let Some(effects) = parse_attack_or_block_then_prohibition_sentence(tokens)? {
        return Ok(effects);
    }

    if let Some(effects) =
        super::optional_companion_fanout::parse_optional_companion_fanout_sentence(tokens)?
    {
        return Ok(effects);
    }

    if let Some(effects) =
        super::player_subject_sequences::parse_controller_and_defending_player_discard_or_sacrifice(
            tokens,
        )
    {
        return Ok(effects);
    }

    if let Some(clauses) =
        super::player_subject_sequences::split_explicit_player_subject_clauses(tokens)
    {
        let mut effects = Vec::new();
        for clause in clauses {
            effects.extend(parse_effect_sentence_lexed_inner(clause)?);
        }
        return Ok(effects);
    }

    if let Some(effects) = parse_target_relative_combat_set_sentence(tokens)? {
        return Ok(effects);
    }

    if let Some(effects) = parse_conjoined_must_be_blocked_sentence(tokens)? {
        return Ok(effects);
    }

    if let Some(effects) =
        super::parse_destroy_then_temporary_cant_attack_block_chain_lexed(tokens)?
    {
        return Ok(effects);
    }
    // "If <player refs> would gain life this turn, that player gains no life
    // instead." == a can't-gain-life window for those players (Flames of the
    // Blood Hand). Intercept before leading-if splitting since the would-gain
    // predicate isn't a state condition.
    if sentence_shapes::parses_cant_gain_life_replacement_tokens(tokens) {
        return Ok(vec![EffectAst::subject_verb_cant(
            crate::effect::Restriction::gain_life(crate::target::PlayerFilter::DamagedPlayer),
            crate::effect::Until::EndOfTurn,
            None,
        )]);
    }
    if let Some(effects) = parse_reveal_source_exiled_permanents_sentence_lexed(tokens) {
        return Ok(effects);
    }
    if let Some(effect) =
        parse_put_cards_from_single_graveyard_on_bottom_owner_library_sentence(tokens)
    {
        return Ok(vec![effect]);
    }
    if let Some(effect) = parse_source_and_blocked_creatures_top_library_shuffle_sentence(tokens) {
        return Ok(vec![effect]);
    }
    // Preserve voter-relative player predicates before the generic player
    // subject machinery rewrites `each opponent` to an iterated `that
    // player` action and discards the qualifying vote relationship.
    if let Some(effects) = parse_vote_affinity_subject_verb(tokens)? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Vote subject=explicit recognizer=vote-affinity",
        );
        return Ok(effects);
    }
    if let Some(effect) = parse_vote_subject_verb(tokens)? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Vote subject=explicit recognizer=vote-procedure",
        );
        return Ok(vec![effect]);
    }
    // Numeric die-result branches also have the surface shape
    // "for each <noun phrase>, <effect>".  Route the typed keyword shape
    // before the generic object iterator so "odd/even result" is not sent to
    // object-filter lowering.
    if matches!(
        effect_grammar::clause_pattern_shapes::parse_keyword_mechanic_tokens(tokens),
        Some(effect_grammar::clause_pattern_shapes::KeywordMechanicShape::OddEvenResult { .. })
    ) && let Some(effect) = parse_keyword_mechanic_clause(tokens)?
    {
        return Ok(vec![effect]);
    }
    // Counter-result clauses also have the generic surface shape
    // `for each <noun phrase>, <effect>`. Route their typed grammar shapes
    // first so `counter(s) removed this way` is not treated as an object
    // filter or target phrase.
    if let Some(effect) = parse_for_each_counter_removed_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) =
        super::clause_dispatch::parse_for_each_counter_group_removed_this_way_clause(tokens)?
    {
        return Ok(vec![effect]);
    }
    if let Some(effect) = super::clause_dispatch::parse_for_each_prevent_damage_clause(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effects) =
        super::search_library::parse_for_each_destroyed_this_way_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        super::search_library::parse_for_each_sacrificed_this_way_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        super::search_library::parse_for_each_put_into_graveyard_this_way_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) = super::search_library::parse_for_each_exiled_this_way_sentence(tokens)? {
        return Ok(effects);
    }
    // This typed search sequence contains an internal `then` chain. Route it
    // before the generic object iterator can interpret "each of them" as an
    // object filter and detach the final put-on-top clause.
    if effect_grammar::parse_each_chosen_player_search_put_top_shape(tokens).is_some()
        && let Some(effects) = parse_search_library_sentence_lexed(tokens)?
    {
        return Ok(effects);
    }
    if let Some(shape) =
        effect_grammar::for_each_shapes::parse_for_each_mana_symbol_spent_effect_shape(tokens)
    {
        let base = Value::ManaSymbolSpentToCastThisSpell {
            symbol: shape.symbol,
            reference: shape.reference,
        };
        let count = if shape.group_size == 1 {
            base
        } else {
            Value::DividedRoundedDown(Box::new(base), shape.group_size as i32)
        }
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "for-each mana-symbol clause has no effect payload".to_string(),
            ));
        }
        return Ok(vec![EffectAst::RepeatEffects { count, effects }]);
    }
    if let Some(shape) =
        effect_grammar::for_each_shapes::parse_for_each_spent_mana_effect_shape(tokens)
    {
        let source_words = crate::lexer::token_word_refs(shape.source_tokens);
        let count = crate::grammar::shared_util::count_shapes::mana_from_source_spent_to_cast_value_with_reference(
            &source_words,
            shape.reference,
        )
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported for-each spent-mana source (source: '{}')",
                render_token_slice(shape.source_tokens).trim()
            ))
        })?
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "for-each spent-mana clause has no effect payload (effect: '{}')",
                render_token_slice(shape.effect_tokens).trim()
            )));
        }
        return Ok(vec![EffectAst::RepeatEffects { count, effects }]);
    }
    if let Some(shape) = effect_grammar::for_each_shapes::parse_for_each_object_effect_shape(tokens)
    {
        let mut count_words = vec!["for", "each"];
        count_words.extend(crate::lexer::token_word_refs(shape.filter_tokens));
        if let Some((count, used)) = crate::util::parse_for_each_count_value_words(&count_words)
            && used == count_words.len()
            && !matches!(count.unhinted(), Value::Count(_))
        {
            let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "for-each scalar sentence missing effect payload".to_string(),
                ));
            }
            return Ok(vec![EffectAst::RepeatEffects {
                count: count.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
                effects,
            }]);
        }
    }
    if let Some(shape) =
        effect_grammar::for_each_shapes::parse_for_each_dynamic_target_effect_shape(tokens)
    {
        let mut filter = parse_object_filter_lexed(shape.filter_tokens, false)?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "for-each dynamic target sentence missing effect payload".to_string(),
            ));
        }
        let tag = TagKey::from(IT_TAG);
        return Ok(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::dynamic_x(),
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::ForEachTagged { tag, effects },
        ]);
    }
    if let Some(shape) = effect_grammar::for_each_shapes::parse_for_each_object_effect_shape(tokens)
    {
        let filter = super::for_each_helpers::parse_for_each_object_filter(shape.filter_tokens)?;
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "for-each object sentence missing effect payload".to_string(),
            ));
        }
        return Ok(vec![EffectAst::ForEachObject { filter, effects }]);
    }
    if let Some(effects) = super::bundle_rules::parse_consult_disposition_bundle(tokens) {
        return Ok(effects);
    }
    let delayed_shape = sentence_shapes::parse_delayed_sentence_tokens(tokens);
    if matches!(
        delayed_shape,
        Some(sentence_shapes::DelayedSentenceShape::NextEndStep)
    )
        && let Some(effects) = parse_delayed_until_next_end_step_sentence(tokens)? {
            return Ok(effects);
        }
    if matches!(
        delayed_shape,
        Some(sentence_shapes::DelayedSentenceShape::NextCombat)
    ) && let Some(effects) = parse_delayed_next_combat_phase_this_turn_sentence(tokens)?
    {
        return Ok(effects);
    }
    if let Some(effects) = parse_it_is_aura_enchantment_sentence_lexed(tokens)? {
        return Ok(effects);
    }
    let quoted_animation_grant = tokens
        .iter()
        .filter(|token| token.kind == crate::lexer::TokenKind::Quote)
        .count()
        >= 2
        && tokens.iter().any(|token| token.is_word("becomes"))
        && tokens.iter().any(|token| token.is_word("gains"));
    if quoted_ability_shape.is_some()
        && let Some(effects) =
            super::fanout_family::parse_shared_color_target_fanout_sentence(tokens)?
    {
        return Ok(effects);
    }
    // Preserve the chooser on optional quoted restrictions. The broad quoted
    // grant parser can otherwise consume the whole sentence before the chain
    // parser turns the leading "you may have" into a MayByPlayer node.
    if quoted_ability_shape.is_some() && super::parse_leading_player_may_lexed(tokens).is_some() {
        return super::parse_effect_chain_lexed(tokens);
    }
    // A leading conditional owns the whole sentence. Do not let a quoted
    // ability's inner verbs make the broad gain parser consume the unsplit
    // condition and body; the conditional route below parses the body with
    // this same gain parser after removing the predicate.
    if (quoted_ability_shape.is_some() || quoted_animation_grant)
        && !matches!(
            sentence_shapes::parse_leading_if_sentence_tokens(tokens),
            Some(sentence_shapes::LeadingIfSentenceShape { replacement: false })
        )
        && let Some(effects) = super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(effects);
    }
    if effect_grammar::gain_ability_shapes::parse_source_tapped_gain_duration_shape(tokens)
        .is_some()
        && let Some(effects) = super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(effects);
    }
    if sentence_shapes::parse_immediate_sacrifice_sentence_tokens(tokens).is_some() {
        let mut effects = super::parse_effect_chain_inner_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(sentence_shapes::DelayedSentenceShape::EndOfCombat { remainder_tokens }) =
        delayed_shape
    {
        let remainder = trim_commas(remainder_tokens);
        if remainder.is_empty() {
            return Err(CardTextError::ParseError(
                "end-of-combat delayed trigger missing effect payload".to_string(),
            ));
        }
        let effects = parse_effect_sentence_lexed_inner(&remainder)?;
        return Ok(vec![EffectAst::DelayedUntilEndOfCombat { effects }]);
    }

    if let Some(effect) = parse_additional_phase_sentence(tokens) {
        return Ok(vec![effect]);
    }

    // Future replacement clauses use an `If ... would ... instead` surface,
    // but their condition is an event predicate rather than an ordinary
    // state predicate.  Recognize the typed replacement before the generic
    // leading-if splitter asks the predicate grammar to parse `would die`.
    if let Some(effect) =
        crate::effect_sentences::dispatch_entry::future_zone_replacement_from_sentence_tokens(
            tokens,
        )
    {
        return Ok(vec![effect]);
    }

    if let Some(effect) = parse_triggering_object_had_counters_create_tokens(tokens)? {
        return Ok(vec![effect]);
    }

    let leading_if_shape = sentence_shapes::parse_leading_if_sentence_tokens(tokens);
    if matches!(
        leading_if_shape,
        Some(sentence_shapes::LeadingIfSentenceShape { replacement: false })
    ) {
        // A quoted ability can contain its own verbs. Parse the conditional
        // body as an outer gain grant first so a nested trigger such as
        // `"At the beginning of the end step, sacrifice this permanent."`
        // cannot steal dispatch from `the copy gains ...`.
        let conditional = if quoted_ability_shape.is_some() {
            parse_conditional_sentence_family_lexed(
                tokens,
                parse_gain_ability_before_effect_chain,
            )
        } else if effect_grammar::control_copy_attach_shapes::contains_source_exiled_owner_library_bottom_shape(tokens)
        {
            parse_conditional_sentence_family_lexed(
                tokens,
                parse_effect_chain_preserving_source_exiled_owner_library_bottom,
            )
        } else {
            parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)
        };
        if let Ok(Some(mut effects)) = conditional {
            if matches!(effects.as_slice(), [EffectAst::Conditional { .. }]) {
                apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
                normalize_search_followup_shuffles(&mut effects);
                return Ok(effects);
            }
            if matches!(effects.as_slice(), [EffectAst::IfResult { .. }]) {
                super::preserve_leading_result_coordination_lexed(tokens, &mut effects);
                normalize_search_followup_shuffles(&mut effects);
                return Ok(effects);
            }
        }
    }

    if has_unrecognized_leading_effect_label(tokens) {
        return Err(CardTextError::ParseError(
            "unknown labeled effect prefix".to_string(),
        ));
    }

    if let Some(effects) = parse_sentence_each_player_return_with_additional_counter(
        SubjectVerbPrimitiveClause::new(tokens),
    )? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Return subject=each-player recognizer=return-with-additional-counter",
        );
        return Ok(effects);
    }
    if let Some(effects) =
        parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Reveal subject=each-player recognizer=top-count-permanents-rest-graveyard",
        );
        return Ok(effects);
    }

    // Preserve an inline continuation after a reveal-until traversal before
    // the broad subject/verb recognizer claims only the leading reveal.
    if let Some(effects) =
        super::consult_family::parse_consult_traversal_with_inline_followup(tokens)?
    {
        return Ok(effects);
    }

    if effect_grammar::sentence_predicate_shapes::parse_where_x_sentence_tokens(tokens)
        .is_some_and(|shape| shape.comma_tail_has_effect_clause)
    {
        // A semicolon/comma after the where-X binding begins another effect
        // clause. Route the grammar-confirmed layout before broad gain and
        // subject/verb probes can absorb the trailing clause's subject into
        // the first `gets` modifier and report a malformed binding.
        crate::parse_trace::event("effect-route: where-x binding with trailing effect clause");
        let mut effects = parse_effect_sentence_with_where_x_lexed(tokens)?;
        apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
        return Ok(effects);
    }

    // A three-arm continuous clause has one grammatical subject even though
    // its comma before `becomes` also looks like an ordinary effect-chain
    // boundary. Preserve the grammar-confirmed coordinated model before the
    // fallback chain splitter expands the middle arm and treats its subtype
    // payload as a new object-filter subject.
    if let Some(effects) = super::gain_ability::parse_gain_ability_sentence(tokens)?
        && is_loss_become_base_pt_coordinated_chain(&effects)
    {
        return Ok(effects);
    }

    // Same-object exile/return programs own their complete `, then` clause.
    // In particular, a timing suffix on the exile action scopes both actions:
    // "exile it at end of combat, then return it ..." is one delayed program.
    // Route that typed shape before the general comma-then splitter turns it
    // into two immediate zone changes and loses the timing wrapper.
    if let Some(effects) = parse_exile_then_return_same_object_sentence(tokens)? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Exile subject=explicit recognizer=exile-return-same-object",
        );
        return Ok(effects);
    }

    // A looked-card partition owns its internal `, then` boundary.  Route the
    // grammar-proven full program before the generic chain splitter; otherwise
    // the leading look/exile actions can be mistaken for additional trigger
    // text and only the remainder move survives (for example, Clone Shell's
    // "look ..., exile one face down, then put the rest ..." trigger).
    if let Some(effects) =
        parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(tokens)
    {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Look subject=implicit recognizer=face-down-looked-partition",
        );
        return Ok(effects);
    }

    // The comma-then boundary in the each-player exile-top/cast program is
    // internal to one collection-producing effect.  Its typed recognizer
    // accumulates every iterated player's exiled card under one tag before
    // granting the trailing cast permissions.  Generic chain splitting would
    // instead lower the leading library object as one unowned card and lose
    // both the player loop and the collection relationship.
    if let Some(effects) =
        parse_generic_each_player_exile_top_then_cast_any_number_subject_verb(tokens)?
    {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Exile subject=each-player recognizer=exile-top-cast",
        );
        return Ok(effects);
    }

    // Once the specialist whole-sentence shapes above have had a chance to
    // claim the clause, an authored `, then` boundary must be parsed as an
    // executable chain before the broad subject/verb recognizer runs.  Broad
    // action parsers deliberately accept descriptive suffixes, so asking one
    // of them to parse the whole clause can otherwise keep only the leading
    // action (for example, `create a token, then copy that spell`) and silently
    // discard the follow-up.
    if super::lex_chain_helpers::has_explicit_comma_then_boundary_lexed(tokens) {
        // A where-X binding scopes the complete ordered program. Strip and
        // parse that binding before handing the action body to the chain
        // parser; otherwise both actions survive but the later X remains
        // unbound because the generic chain route never sees the value tail.
        if has_where_x_value_binding(tokens) {
            let mut effects = parse_effect_sentence_with_where_x_lexed(tokens)?;
            apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
            return Ok(effects);
        }
        return super::parse_effect_chain_lexed(tokens);
    }

    // `Put ... or remove ... counter` is a single typed counter operation,
    // not the generic action-choice form represented by `UnlessAction`.
    // Let the counter verb handler confirm the complete shape before the
    // broad top-level `or` splitter examines the sentence.
    if tokens.first().is_some_and(|token| token.is_word("put"))
        && let Ok(effect) =
            super::verb_dispatch::parse_effect_with_verb(super::Verb::Put, None, &tokens[1..])
        && matches!(
            &effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutOrRemoveCounters { .. },
                ..
            })
        )
    {
        return Ok(vec![effect]);
    }

    // A serial negative keyword predicate is part of the mass-damage object
    // filter, not a top-level alternative-action list. Prove the complete
    // direct clause first and only preempt when it yields one damage sweep
    // with multiple excluded static abilities.
    let words = crate::lexer::parser_token_word_refs(tokens);
    let has_negative_ability_predicate = words
        .windows(2)
        .any(|window| matches!(window, ["doesn't" | "doesnt", "have"]))
        || words
            .windows(3)
            .any(|window| matches!(window, ["does", "not", "have"]));
    if has_negative_ability_predicate
        && let Some(each_idx) = tokens.iter().position(|token| token.is_word("each"))
        && let Some(filter_tokens) = tokens.get(each_idx + 1..)
        && let Ok(serial_filter) =
            crate::object_filters::parse_object_filter_lexed(filter_tokens, false)
        && serial_filter.excluded_static_abilities.len() >= 2
        && let Ok(mut effect) = super::clause_dispatch::parse_effect_clause_lexed(tokens)
    {
        // The broad damage primitive deliberately accepts the first complete
        // object-filter prefix.  Reattach the grammar-proven full serial
        // predicate before the later top-level `or` dispatcher can interpret
        // the final keyword as an alternative executable action.
        let repaired = match &mut effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::DealDamageEach { filter, .. },
                ..
            }) => {
                *filter = serial_filter;
                true
            }
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::DealDamageEqualToPower {
                        target: crate::model::TargetAst::Object(filter, _, _),
                        ..
                    },
                ..
            }) => {
                *filter = serial_filter;
                true
            }
            _ => false,
        };
        if repaired {
            return Ok(vec![effect]);
        }
    }

    // An explicit top-level action choice must be split before the broad
    // subject/verb recognizer. Otherwise a later gain/lose verb can accept
    // the complete leading action as an object-filter subject and silently
    // retain only the final ability-grant branch.
    if let Some(unless_action) = super::parse_or_action_clause_lexed(tokens)? {
        return Ok(vec![unless_action]);
    }

    if let Some((route, mut effects)) = parse_top_level_subject_verb_recognition(tokens)? {
        crate::parse_trace::event(format!("effect-route: {route}"));
        normalize_search_followup_shuffles(&mut effects);
        return Ok(effects);
    }
    // The sentence dispatcher has exhausted its specialized routes here.
    // Delegate to the lower-level chain parser; calling this dispatcher again
    // with the same tokens recurses forever for ordinary subject/verb clauses.
    let mut effects = super::parse_effect_chain_inner_lexed(tokens)?;
    apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
    normalize_search_followup_shuffles(&mut effects);
    Ok(effects)
}

fn parse_source_and_blocked_creatures_top_library_shuffle_sentence(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    sentence_shapes::parse_source_blocked_library_shuffle_tokens(tokens)?;

    let mut blocked_creature = ObjectFilter::creature();
    blocked_creature.blocked_by_source = true;
    let mut moved_objects = ObjectFilter::default();
    moved_objects.any_of = vec![ObjectFilter::source(), blocked_creature];

    Some(EffectAst::ForEachObject {
        filter: moved_objects,
        effects: vec![
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                Zone::Library,
                true,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::ItsOwner,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
        ],
    })
}

fn parse_put_cards_from_single_graveyard_on_bottom_owner_library_sentence(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let shape = sentence_shapes::parse_single_graveyard_library_bottom_tokens(tokens)?;
    let count = usize::try_from(shape.count).ok()?;

    let filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .single_graveyard();
    Some(EffectAst::subject_verb_move_to_zone(
        TargetAst::WithCount(
            Box::new(TargetAst::Object(filter, None, None)),
            ChoiceCount::exactly(count),
        ),
        Zone::Library,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ))
}

fn rebind_plural_create_followup_damage_source(effects: &mut [EffectAst]) {
    for index in 1..effects.len() {
        let previous_creates_more_than_one = matches!(
            &effects[index - 1],
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CreateTokenWithMods { count, .. },
                ..
            }) if !matches!(count.unhinted(), Value::Fixed(1))
        );
        if !previous_creates_more_than_one {
            continue;
        }
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamageEqualToPower { source, .. },
            ..
        }) = &mut effects[index]
        else {
            continue;
        };
        let TargetAst::Tagged(tag, span) = source else {
            continue;
        };
        if tag.as_str() == crate::cards::builders::IT_TAG {
            // A singular `it` cannot denote a plural token result. Preserve
            // the authored pronoun span while binding the damage producer to
            // the ability's source instead of the last created token.
            *source = TargetAst::Source(*span);
        }
    }

    for effect in effects {
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            rebind_plural_create_followup_damage_source(nested);
        });
    }
}

fn parse_effect_sentence_with_where_x_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    fn replace_search_filter_x(effect: &mut EffectAst, replacement: &Value) {
        let (filter, count, count_value) = match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::SearchLibrary {
                        filter,
                        count,
                        count_value,
                        ..
                    },
                ..
            }) => (filter, count, count_value),
            EffectAst::ChooseObjects {
                filter,
                count,
                count_value,
                ..
            }
            | EffectAst::ChooseObjectsAcrossZones {
                filter,
                count,
                count_value,
                ..
            } => (filter, count, count_value),
            _ => return,
        };

        if count.dynamic_x && count_value.is_none() {
            *count_value = Some(replacement.clone());
        }
        if let Some(mana_value) = filter.mana_value.as_mut() {
            use crate::filter::Comparison;

            match mana_value {
                Comparison::EqualExpr(value)
                | Comparison::NotEqualExpr(value)
                | Comparison::LessThanExpr(value)
                | Comparison::LessThanOrEqualExpr(value)
                | Comparison::GreaterThanExpr(value)
                | Comparison::GreaterThanOrEqualExpr(value)
                    if matches!(value.as_ref(), Value::X) =>
                {
                    **value = replacement.clone();
                }
                _ => {}
            }
        }
    }

    fn bind_dynamic_target_count(target: &mut TargetAst, replacement: &Value) {
        fn bind_comparison_x(
            comparison: &mut Option<crate::filter::Comparison>,
            replacement: &Value,
        ) {
            let Some(
                crate::filter::Comparison::EqualExpr(value)
                | crate::filter::Comparison::NotEqualExpr(value)
                | crate::filter::Comparison::LessThanExpr(value)
                | crate::filter::Comparison::LessThanOrEqualExpr(value)
                | crate::filter::Comparison::GreaterThanExpr(value)
                | crate::filter::Comparison::GreaterThanOrEqualExpr(value),
            ) = comparison
            else {
                return;
            };
            if matches!(value.as_ref(), Value::X) {
                **value = replacement.clone();
            }
        }

        fn bind_filter_x(filter: &mut crate::target::ObjectFilter, replacement: &Value) {
            bind_comparison_x(&mut filter.power, replacement);
            bind_comparison_x(&mut filter.toughness, replacement);
            bind_comparison_x(&mut filter.mana_value, replacement);
            if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
                bind_filter_x(attached_to, replacement);
            }
            for branch in &mut filter.any_of {
                bind_filter_x(branch, replacement);
            }
        }

        match target {
            TargetAst::Object(filter, _, _) => bind_filter_x(filter, replacement),
            TargetAst::WithCount(inner, count) => {
                bind_dynamic_target_count(inner, replacement);
                if count.is_dynamic_x() {
                    let old = std::mem::replace(target, TargetAst::Source(None));
                    if let TargetAst::WithCount(inner, count) = old {
                        *target = TargetAst::WithCountValue(inner, count, replacement.clone());
                    }
                }
            }
            TargetAst::WithCountValue(inner, _, value) => {
                bind_dynamic_target_count(inner, replacement);
                if matches!(value, Value::X) {
                    *value = replacement.clone();
                }
            }
            _ => {}
        }
    }

    fn bind_dynamic_target_counts(effect: &mut EffectAst, replacement: &Value) {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return;
        };
        match action {
            SubjectVerbActionAst::Explore { target }
            | SubjectVerbActionAst::Endure { target, .. }
            | SubjectVerbActionAst::Connive { target, .. }
            | SubjectVerbActionAst::ExchangeTextBoxes { target }
            | SubjectVerbActionAst::Attach { target, .. }
            | SubjectVerbActionAst::Unattach { object: target }
            | SubjectVerbActionAst::ReturnToHand { target, .. }
            | SubjectVerbActionAst::MayMoveToZone { target, .. }
            | SubjectVerbActionAst::ReturnToBattlefield { target, .. }
            | SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. }
            | SubjectVerbActionAst::MoveToZone { target, .. }
            | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target }
            | SubjectVerbActionAst::TargetOnly { target, .. }
            | SubjectVerbActionAst::Pump { target, .. }
            | SubjectVerbActionAst::SetBasePowerToughness { target, .. }
            | SubjectVerbActionAst::BecomeBasePtCreature { target, .. }
            | SubjectVerbActionAst::SetBasePower { target, .. }
            | SubjectVerbActionAst::PumpForEach { target, .. }
            | SubjectVerbActionAst::PumpByLastEffect { target, .. }
            | SubjectVerbActionAst::AddCardTypes { target, .. }
            | SubjectVerbActionAst::SetCardTypes { target, .. }
            | SubjectVerbActionAst::RemoveCardTypes { target, .. }
            | SubjectVerbActionAst::AddSubtypes { target, .. }
            | SubjectVerbActionAst::RemoveSubtypes { target, .. }
            | SubjectVerbActionAst::AddColors { target, .. }
            | SubjectVerbActionAst::AddAllSubtypesOfFamily { target, .. }
            | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { target, .. }
            | SubjectVerbActionAst::BecomeBasicLandType { target, .. }
            | SubjectVerbActionAst::SetColors { target, .. }
            | SubjectVerbActionAst::MakeColorless { target, .. }
            | SubjectVerbActionAst::BecomeBasicLandTypeChoice { target, .. }
            | SubjectVerbActionAst::BecomeCreatureTypeChoice { target, .. }
            | SubjectVerbActionAst::BecomeColorChoice { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. }
            | SubjectVerbActionAst::RedirectNextTimeDamageToSource { target, .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                source: target,
            }
            | SubjectVerbActionAst::RetargetStackObject { target, .. }
            | SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDistributedDamage { target, .. }
            | SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target } => {
                bind_dynamic_target_count(target, replacement)
            }
            SubjectVerbActionAst::Destroy { target, .. } => {
                bind_dynamic_target_count(target, replacement)
            }
            SubjectVerbActionAst::PutCounters {
                target,
                target_count,
                ..
            } => {
                bind_dynamic_target_count(target, replacement);
                if let Some(count) = target_count
                    .as_ref()
                    .copied()
                    .filter(|count| count.is_dynamic_x())
                    && !matches!(target, TargetAst::WithCountValue(_, _, _))
                {
                    let inner = std::mem::replace(target, TargetAst::Source(None));
                    *target =
                        TargetAst::WithCountValue(Box::new(inner), count, replacement.clone());
                }
            }
            SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                protected_target,
                destination_target,
                ..
            } => {
                if let Some(target) = protected_target {
                    bind_dynamic_target_count(target, replacement);
                }
                if let Some(target) = destination_target {
                    bind_dynamic_target_count(target, replacement);
                }
            }
            SubjectVerbActionAst::Fight {
                creature1,
                creature2,
            }
            | SubjectVerbActionAst::DealDamageEqualToPower {
                source: creature1,
                target: creature2,
                ..
            }
            | SubjectVerbActionAst::BecomeCopy {
                target: creature1,
                source: creature2,
                ..
            } => {
                bind_dynamic_target_count(creature1, replacement);
                bind_dynamic_target_count(creature2, replacement);
            }
            SubjectVerbActionAst::CreateTokenCopyFromSource { source, .. } => {
                bind_dynamic_target_count(source, replacement);
            }
            SubjectVerbActionAst::CreateTokenWithMods {
                attached_to: Some(target),
                ..
            } => bind_dynamic_target_count(target, replacement),
            _ => {}
        }
    }

    let clause_display = render_token_slice(tokens).trim().to_string();
    let Some(where_shape) = sentence_shapes::parse_where_x_sentence_tokens(tokens) else {
        return parse_effect_sentence_inner_lexed(tokens);
    };
    let aggregate_where =
        crate::keyword_static::parse_where_x_is_aggregate_filter_value(where_shape.where_tokens);
    let turn_history_where = aggregate_where
        .is_none()
        .then(|| {
            crate::grammar::shared_util::value_semantics::parse_turn_history_value_binding(
                where_shape.where_tokens,
            )
        })
        .flatten();
    let full_where_is_count_value = !where_shape.comma_tail_has_effect_clause
        && (turn_history_where.is_some()
            || crate::keyword_static::parse_where_x_is_sum_of_number_of_filter_values(
                where_shape.where_tokens,
            )
            .is_some()
            || crate::keyword_static::parse_where_x_is_number_of_filter_value(
                where_shape.where_tokens,
            )
            .is_some());
    let layout = where_shape.layout(full_where_is_count_value);
    let primary_where_tokens = layout.primary_where_tokens;
    let trailing_after_where = layout.trailing_after_where;
    let stripped = trim_edge_punctuation(where_shape.stripped_tokens);

    if let Some(effects) = parse_target_deals_power_damage_to_other_and_self_where_x(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) =
        parse_tap_then_damage_for_number_tapped_this_way(&stripped, primary_where_tokens)?
    {
        return Ok(effects);
    }

    let mut prelude_effects = Vec::new();
    // Only the action before the where-X binding determines what a possessive
    // reference denotes. A later effect clause is dispatched independently
    // and cannot turn "target creature ... where X is its power" back into a
    // source-relative value.
    let typed_where_references_target = where_shape.stripped_references_target
        && !sentence_shapes::starts_with_source_deals_x_tokens(&stripped);
    // Prefer the complete number-of family before the generic typed value
    // shape. The latter can correctly find the trailing object scope while
    // still losing the aggregate being measured, as in "the number of
    // abilities from among ... found among creatures you control."
    // A player-comparison value ends in an object noun ("more lands than
    // you"), but its cardinality is the number of qualifying players. Parse
    // that participant domain before the generic number-of-filter family can
    // collapse it to a battlefield-object count.
    let participant_comparison_where = turn_history_where
        .is_none()
        .then(|| {
            crate::grammar::values::parse_players_who_control_more_than_you_value_lexed(
                primary_where_tokens,
            )
        })
        .flatten();
    let exact_where_value = (turn_history_where.is_none()
        && participant_comparison_where.is_none())
    .then(|| super::dispatch_entry::parse_exact_where_x_value_expression(primary_where_tokens))
    .flatten();
    let complete_number_where = (turn_history_where.is_none()
        && participant_comparison_where.is_none()
        && exact_where_value.is_none())
    .then(|| crate::keyword_static::parse_where_x_is_number_of_filter_value(primary_where_tokens))
    .flatten();
    let typed_where_value = if turn_history_where.is_none()
        && participant_comparison_where.is_none()
        && exact_where_value.is_none()
        && complete_number_where.is_none()
    {
        sentence_shapes::parse_where_x_value_shape_tokens(
            primary_where_tokens,
            typed_where_references_target,
        )
        .and_then(lower_where_x_shape)
    } else {
        None
    };
    let where_value = if let Some(value) = aggregate_where {
        value
    } else if let Some(value) = turn_history_where {
        value
    } else if let Some(value) = participant_comparison_where {
        value
    } else if let Some(value) = exact_where_value {
        value
    } else if let Some(value) = complete_number_where {
        value
    } else if let Some((prelude, value)) = typed_where_value {
        if let Some(prelude) = prelude {
            prelude_effects.push(prelude);
        }
        value
    } else {
        let activation_time_trimmed =
            sentence_shapes::parse_before_activation_time_tokens(primary_where_tokens)
                .map(trim_edge_punctuation);
        let specific_where_value = super::dispatch_entry::parse_exact_where_x_value_expression(
            primary_where_tokens,
        )
        .or_else(|| {
            crate::grammar::values::parse_players_who_control_more_than_you_value_lexed(
                primary_where_tokens,
            )
        })
        .or_else(|| {
            crate::keyword_static::parse_where_x_is_sum_of_number_of_filter_values(
                primary_where_tokens,
            )
        })
        .or_else(|| {
            crate::keyword_static::parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(
                primary_where_tokens,
            )
        })
        .or_else(|| {
            crate::keyword_static::parse_where_x_is_number_of_different_powers_filter_value(
                primary_where_tokens,
            )
        });
        let number_of_filter_value = specific_where_value
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_colored_mana_symbols_value(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::keyword_static::parse_where_x_is_number_of_filter_value(primary_where_tokens)
            })
            .or_else(|| {
                activation_time_trimmed
                    .as_deref()
                    .and_then(crate::keyword_static::parse_where_x_is_number_of_filter_value)
            });
        if let Some(value) = number_of_filter_value {
            value
        } else if let Some(trimmed) = activation_time_trimmed.as_deref() {
            parse_value_binding_clause_lexed(trimmed).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported where-x clause (clause: '{}')",
                    &clause_display
                ))
            })?
        } else {
            parse_value_binding_clause_lexed(primary_where_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported where-x clause (clause: '{}')",
                    &clause_display
                ))
            })?
        }
    };
    let where_value = crate::effect_sentences::dispatch_entry::with_where_x_surface_hints(
        where_value,
        primary_where_tokens,
    );

    let search_like = where_shape.stripped_starts_search;
    let mut effects = if search_like && !trailing_after_where.is_empty() {
        let mut recombined = stripped.clone();
        recombined.extend(trailing_after_where.clone());
        parse_effect_sentence_lexed(&recombined)?
    } else {
        let mut parsed = parse_effect_sentence_inner_lexed(&stripped)?;
        if parsed.is_empty() && !stripped.is_empty() {
            parsed.push(super::parse_effect_clause_lexed(&stripped)?);
        }
        if !trailing_after_where.is_empty() {
            let mut trailing_effects = parse_effect_sentence_lexed(&trailing_after_where)?;
            parsed.append(&mut trailing_effects);
        }
        parsed
    };
    rebind_plural_create_followup_damage_source(&mut effects);
    replace_unbound_x_in_effects_anywhere(&mut effects, &where_value, &clause_display)?;
    for effect in &mut effects {
        replace_search_filter_x(effect, &where_value);
        bind_dynamic_target_counts(effect, &where_value);
    }
    if !prelude_effects.is_empty() {
        prelude_effects.append(&mut effects);
        return Ok(prelude_effects);
    }
    Ok(effects)
}

#[cfg(test)]
mod spent_mana_repeat_tests {
    use super::*;
    use crate::IfResultPredicate;

    #[test]
    fn as_though_no_defender_preempts_the_broad_defender_grant_route() {
        let tokens = crate::lexer::lex_line(
            "This creature can attack this turn as though it didn't have defender.",
            0,
        )
        .expect("permission should lex");
        let effects = parse_effect_sentence_lexed(&tokens).expect("permission should parse");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("CanAttackAsThoughNoDefender"), "{debug}");
        assert!(!debug.contains("KeywordAction(Defender)"), "{debug}");

        let near_miss =
            crate::lexer::lex_line("This creature gains defender until end of turn.", 0)
                .expect("ordinary defender grant should lex");
        let effects = parse_effect_sentence_lexed(&near_miss)
            .expect("ordinary defender grant should still parse");
        let debug = format!("{effects:#?}");
        assert!(!debug.contains("CanAttackAsThoughNoDefender"), "{debug}");
        assert!(debug.contains("Defender"), "{debug}");

        let coordinated = crate::lexer::lex_line(
            "Target creature you control gets +1/+0 until end of turn and can attack as though it didn't have defender.",
            0,
        )
        .expect("coordinated permission should lex");
        let effects =
            parse_effect_sentence_lexed(&coordinated).expect("coordinated permission should parse");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("CanAttackAsThoughNoDefender"), "{debug}");
        assert!(debug.contains("Tagged"), "{debug}");
        assert!(!debug.contains("GrantAbilitiesAll"), "{debug}");
    }

    #[test]
    fn direct_sentence_route_keeps_put_history_inside_target_declaration() {
        let tokens = crate::lexer::lex_line(
            "Choose up to three target permanent cards in graveyards that were put there from the battlefield this turn.",
            0,
        )
        .expect("historical target declaration should lex");
        let effects = parse_effect_sentence_lexed(&tokens)
            .expect("direct sentence route should keep the complete target declaration");
        let debug = format!("{effects:#?}");

        assert_eq!(effects.len(), 1, "{debug}");
        assert!(debug.contains("TargetOnly"), "{debug}");
        assert!(
            debug.contains("entered_graveyard_from_battlefield_this_turn: true"),
            "{debug}"
        );
        assert!(!debug.contains("MoveToZone"), "{debug}");
    }

    #[test]
    fn keyword_bundle_list_preempts_leading_duration_chain_splitting() {
        let tokens = crate::lexer::lex_line(
            "until end of turn, each other creature you control gets +1/+1 if it has flying, +1/+1 if it has first strike, and so on for double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, protection, reach, trample, vigilance, and partner.",
            0,
        )
        .expect("keyword-bundle sentence should lex");
        let effects =
            parse_effect_sentence_lexed(&tokens).expect("keyword-bundle sentence should parse");

        assert_eq!(effects.len(), 14, "{effects:#?}");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("Flying"), "{debug}");
        assert!(debug.contains("Partner"), "{debug}");
    }

    #[test]
    fn for_each_mana_from_source_repeats_the_typed_effect() {
        let tokens = crate::lexer::lex_line(
            "For each mana from a Desert spent to cast this spell, create a tapped Treasure token.",
            0,
        )
        .expect("spent-mana sentence should lex");
        let effects =
            parse_effect_sentence_lexed(&tokens).expect("spent-mana sentence should parse");
        let [EffectAst::RepeatEffects { count, effects }] = effects.as_slice() else {
            panic!("expected one typed repeat effect, got {effects:#?}");
        };
        assert!(count.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
        let Value::ManaFromSourceSpentToCastThisSpell {
            source_filter,
            include_source_noun,
            ..
        } = count.unhinted()
        else {
            panic!("expected a mana-source repeat count, got {count:#?}");
        };
        assert!(!include_source_noun);
        assert_eq!(source_filter.subtypes, [crate::types::Subtype::Desert]);
        assert!(
            format!("{effects:#?}").contains("CreateToken"),
            "{effects:#?}"
        );
    }

    #[test]
    fn for_each_repeated_mana_symbol_uses_a_divided_typed_count() {
        let tokens = crate::lexer::lex_line("For each {U}{U} spent to cast it, draw a card.", 0)
            .expect("mana-symbol sentence should lex");
        let effects =
            parse_effect_sentence_lexed(&tokens).expect("mana-symbol sentence should parse");
        let [EffectAst::RepeatEffects { count, effects }] = effects.as_slice() else {
            panic!("expected one typed repeat effect, got {effects:#?}");
        };
        assert!(count.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
        assert!(matches!(
            count.unhinted(),
            Value::DividedRoundedDown(inner, 2)
                if matches!(
                    inner.as_ref(),
                    Value::ManaSymbolSpentToCastThisSpell {
                        symbol: crate::mana::ManaSymbol::Blue,
                        reference: ironsmith_core::ManaSpentCastReferenceSurface::It,
                    }
                )
        ));
        assert!(format!("{effects:#?}").contains("Draw"), "{effects:#?}");
    }

    #[test]
    fn conditional_quoted_grant_keeps_the_outer_gain_semantics() {
        let body_tokens = crate::lexer::lex_line(
            "The copy gains haste and \"At the beginning of the end step, sacrifice this permanent.\"",
            0,
        )
        .expect("quoted gain body should lex");
        let direct_gain = super::super::gain_ability::parse_gain_ability_sentence(&body_tokens)
            .expect("quoted gain body should parse without falling back")
            .expect("quoted gain body should be recognized as a gain");
        assert!(
            format!("{direct_gain:#?}").contains("GrantAbilitiesToTarget"),
            "{direct_gain:#?}"
        );

        let tokens = crate::lexer::lex_line(
            "If it's a permanent spell, the copy gains haste and \"At the beginning of the end step, sacrifice this permanent.\"",
            0,
        )
        .expect("conditional quoted grant should lex");
        let effects =
            parse_effect_sentence_lexed(&tokens).expect("conditional quoted grant should parse");
        let [EffectAst::Conditional { if_true, .. }] = effects.as_slice() else {
            panic!("expected one typed conditional, got {effects:#?}");
        };
        let debug = format!("{if_true:#?}");
        assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
        assert!(debug.contains("Haste"), "{debug}");
        assert!(debug.contains("BeginningOfEndStep"), "{debug}");
        assert!(debug.contains("Sacrifice"), "{debug}");
    }

    #[test]
    fn quoted_restriction_grant_keeps_trailing_defending_player_unless_payment() {
        let tokens = crate::lexer::lex_line(
            "It gains \"This creature can't be blocked.\" until end of turn unless defending player sacrifices a creature of their choice.",
            0,
        )
        .expect("quoted restriction gain should lex");
        let effects = parse_effect_sentence_lexed(&tokens)
            .expect("quoted restriction gain should parse before the broad can't route");

        let [
            EffectAst::UnlessPays {
                effects: granted_effects,
                player: PlayerAst::Defending,
                cost,
                before_delayed_step: false,
            },
        ] = effects.as_slice()
        else {
            panic!("expected a defending-player unless payment, got {effects:#?}");
        };
        assert!(format!("{cost:#?}").contains("Sacrifice"), "{cost:#?}");
        let debug = format!("{granted_effects:#?}");
        assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
        assert!(debug.contains("duration: EndOfTurn"), "{debug}");
        assert!(debug.contains("RuleRestriction"), "{debug}");
    }

    #[test]
    fn public_sentence_route_keeps_result_gated_unattach_delayed() {
        let tokens = crate::lexer::lex_line(
            "If you do, unattach it at the beginning of the next end step.",
            0,
        )
        .expect("delayed unattach sentence should lex");
        let effects = parse_effect_sentence_lexed(&tokens)
            .expect("public sentence route should preserve the delayed action");

        let [
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: gated,
            },
        ] = effects.as_slice()
        else {
            panic!("expected an outer result gate, got {effects:#?}");
        };
        let [
            EffectAst::DelayedUntilNextEndStep {
                player: PlayerFilter::Any,
                effects: delayed,
            },
        ] = gated.as_slice()
        else {
            panic!("expected a delayed next-end-step payload, got {gated:#?}");
        };
        assert!(
            matches!(
                delayed.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Unattach { .. },
                    ..
                })]
            ),
            "{delayed:#?}"
        );
    }
}
