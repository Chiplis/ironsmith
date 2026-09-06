use crate::cards::builders::PermissionEffectAst;
use crate::cards::builders::ObjectChoiceEffectAst;
use crate::cards::builders::StatChangeActionAst;
use crate::cards::builders::DamageActionAst;
use crate::cards::builders::PermanentStateActionAst;
use crate::cards::builders::ZoneMoveActionAst;
use crate::cards::builders::CharacteristicActionAst;
use crate::cards::builders::GrantActionAst;
use crate::effect_sentences::{
    SubjectVerbPrimitiveClause, parse_sentence_delayed_next_step_unless_pays,
    parse_sentence_delayed_timing_suffix,
    parse_sentence_each_player_return_with_additional_counter,
    parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard,
    parse_sentence_unless_pays,
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
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. }),
            ..
        }) = effect
            && filter.with_counter.is_none()
        {
            filter.with_counter = Some(counter_constraint);
        }
    }
}

fn is_loss_become_base_pt_coordinated_chain(effects: &[EffectAst]) -> bool {
    let [effect] = effects else {
        return false;
    };
    let coordinated: Vec<&EffectAst> = match effect {
        EffectAst::Coordination(coordination) => coordination.effects().collect(),
        EffectAst::Coordinated { effects, .. } => effects.iter().collect(),
        _ => return false,
    };
    let [first, second, third] = coordinated.as_slice() else {
        return false;
    };
    matches!(
        first,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll { .. }),
            ..
        })
    ) && matches!(
        second,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes { .. }),
            ..
        })
    ) && matches!(
        third,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness { .. }),
            ..
        })
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
    let source_ref = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
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
    let Some(and_token_idx) =
        crate::slice_primitives::select_last_position(&subject_and_action, |token| {
            token.as_word() == Some("and")
        })
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
    let Some(subject_end_token_idx) = action_words.map_word_or_end_to_token_boundary(verb_word_idx)
    else {
        return Ok(None);
    };
    let shared_subject_tokens = trim_edge_punctuation(&action_tokens[..subject_end_token_idx]);
    if shared_subject_tokens.is_empty() {
        return Ok(None);
    }

    let restriction_filter = if starts_with_target_indicator(&shared_subject_tokens) {
        ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind())
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

    if crate::word_primitives::parse_sequence_prefix(&words, &["tap", "all"])
        && let Some(relation_idx) =
            crate::word_primitives::parse_sequence_start(&words, &["blocking", "target"])
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

    if crate::word_primitives::parse_sequence_prefix(&words, &["return", "all"])
        && let Some(relation_idx) = crate::word_primitives::parse_sequence_start(
            &words,
            &["blocking", "or", "blocked", "by", "target"],
        )
        && relation_idx > 2
        && let Some(destination_idx) =
            crate::word_primitives::parse_sequence_start(&words[relation_idx + 5..], &["to"])
                .map(|offset| relation_idx + 5 + offset)
        && destination_idx > relation_idx + 4
        && crate::word_primitives::parse_any_sequence_complete(
            &words[destination_idx..],
            &[
                &["to", "their", "owner's", "hand"],
                &["to", "their", "owners", "hand"],
                &["to", "their", "owners'", "hands"],
                &["to", "their", "owners", "hands"],
            ],
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
            let tag = crate::tag::CompilerReferenceTag::WhereXCommanderManaValue.bind();
            let choice = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
                zones: vec![Zone::Battlefield, Zone::Command],
                search_mode: None,
            });
            (
                Some(choice),
                Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(tag.key.clone()))),
            )
        }
        sentence_shapes::WhereXValueShape::ChosenObjectsPowerDifference { object_kind } => {
            let object_tokens = crate::lexer::synthetic_word_tokens([object_kind.as_str()]);
            let mut filter = parse_object_filter(&object_tokens, false).ok()?;
            filter = filter.match_tagged(
                crate::tag::CompilerReferenceTag::ChosenObjects.bind(),
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
                Reference::TaggedIt => {
                    crate::target::ChooseSpec::Tagged((crate::tag::CompilerReferenceTag::It.bind()).into())
                }
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
            Value::PowerOf(Box::new(crate::target::ChooseSpec::Tagged(
                (crate::tag::CompilerReferenceTag::TapCost0.bind()).into(),
            )))
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
                    crate::target::ChooseSpec::Tagged(
                        (crate::tag::CompilerReferenceTag::SacrificeCost0.bind()).into(),
                    )
                    .with_surface_hint(
                        crate::target::ChooseSpecSurfaceHint::SourceReference(
                            crate::target::SourceReferenceSurface::ThisPermanentType(format!(
                                "the sacrificed {object_kind}"
                            )),
                        ),
                    ),
                )),
            )
        }
        sentence_shapes::WhereXValueShape::ColorsAmongSacrificed { object_kind } => {
            let object_kind = object_kind.trim_end_matches('s');
            let object_tokens = crate::lexer::synthetic_word_tokens([object_kind]);
            let mut filter = parse_object_filter(&object_tokens, false).ok()?;
            filter = filter.match_tagged(
                crate::tag::CompilerReferenceTag::Sacrificed0.bind(),
                TaggedOpbjectRelation::IsTaggedObject,
            );
            (None, Value::ColorsAmong(filter))
        }
        sentence_shapes::WhereXValueShape::TwoPlusSacrificedManaValue => (
            None,
            Value::Add(
                Box::new(Value::Fixed(2)),
                Box::new(Value::ManaValueOf(Box::new(
                    crate::target::ChooseSpec::Tagged((crate::tag::CompilerReferenceTag::It.bind()).into()),
                ))),
            ),
        ),
        sentence_shapes::WhereXValueShape::SourceExiledManaValue => (
            None,
            Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(
                (crate::tag::CompilerReferenceTag::SourceExiled.bind()).into(),
            ))),
        ),
        sentence_shapes::WhereXValueShape::PriorEffectMetric(query) => {
            (None, Value::PendingPriorEffectMetric(query))
        }
        sentence_shapes::WhereXValueShape::DiedThisWayMetric(query) => (
            None,
            Value::PendingPriorEffectMetric(query)
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::DiedThisWay),
        ),
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
                    Box::new(ChooseSpec::Tagged(
                        (crate::tag::CompilerReferenceTag::It.bind()).into(),
                    )),
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
            action: SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { .. }) | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { .. }),
            ..
        })
    );
    if !first_is_tap {
        return Ok(None);
    }
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { amount, target, .. }),
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
        TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            Some(TextSpan::synthetic()),
        ),
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
    let verb = crate::slice_primitives::select_position(&words, |word| {
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
        let object_end = crate::slice_primitives::select_position(&words[verb + 1..], |word| {
            matches!(*word, "to" | "from")
        })
        .map_or(words.len(), |offset| verb + 1 + offset);
        &words[verb + 1..object_end]
    } else {
        &words[..verb]
    };
    if crate::word_primitives::sequence_occurs(subject, &["all"]) {
        Some(ironsmith_core::SetQuantifierSurface::All)
    } else if crate::word_primitives::sequence_occurs(subject, &["each"]) {
        Some(ironsmith_core::SetQuantifierSurface::Each)
    } else if crate::word_primitives::sequence_occurs(subject, &["those"]) {
        Some(ironsmith_core::SetQuantifierSurface::Those)
    } else {
        None
    }
}

fn parse_return_set_reference_surface(tokens: &[OwnedLexToken]) -> Option<String> {
    let words = crate::lexer::token_word_refs(tokens);
    let verb = crate::slice_primitives::select_position(&words, |word| {
        matches!(*word, "return" | "returns")
    })?;
    let object_end = crate::slice_primitives::select_position(&words[verb + 1..], |word| {
        matches!(*word, "to" | "from")
    })
    .map_or(words.len(), |offset| verb + 1 + offset);
    let object = &words[verb + 1..object_end];
    let quantifier =
        crate::slice_primitives::select_position(object, |word| matches!(*word, "each" | "those"))?;
    Some(object[quantifier..].join(" "))
}

fn set_first_continuous_set_quantifier(
    effects: &mut [EffectAst],
    surface: ironsmith_core::SetQuantifierSurface,
) -> bool {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect {
            let slot = match action {
                SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                    set_quantifier_surface,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                    set_quantifier_surface,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                    set_quantifier_surface,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                    set_quantifier_surface,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll {
                    set_quantifier_surface,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                    set_quantifier_surface,
                    ..
                })
                | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
                    set_quantifier_surface,
                    ..
                }) => Some(set_quantifier_surface),
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
                SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
                    set_reference_surface,
                    ..
                }),
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
            EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player,
                effects: vec![EffectAst::subject_verb_pay_mana_up_to(
                    player,
                    payment_shape.cost,
                    maximum,
                )],
            })
        }
        effect_grammar::clause_dispatch_shapes::LeadingMayActorShape::Implicit => EffectAst::Permissions(PermissionEffectAst::May {
            effects: vec![EffectAst::subject_verb_pay_mana_up_to(
                PlayerAst::You,
                payment_shape.cost,
                maximum,
            )],
        }),
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

#[cfg(test)]
#[path = "sentence_shape_predicates_inline_spent_mana_repeat_tests.rs"]
mod spent_mana_repeat_tests;

#[path = "sentence_shape_predicates/sentence_shape_predicates_core.rs"]
mod sentence_shape_predicates_core_programs;
use sentence_shape_predicates_core_programs::{
    has_unrecognized_leading_effect_label, parse_effect_sentence_lexed_inner,
    parse_effect_sentence_lexed_inner_unstacked, parse_effect_sentence_with_where_x_lexed,
};
pub use sentence_shape_predicates_core_programs::{
    parse_effect_sentence_lexed, parse_effect_sentence_lexed_with_context,
};
#[path = "sentence_shape_predicates/sentence_shape_predicates_combat.rs"]
mod sentence_shape_predicates_combat_programs;
pub(super) use sentence_shape_predicates_combat_programs::parse_attacking_doesnt_tap_if_source_untapped;
use sentence_shape_predicates_combat_programs::{
    parse_explicit_assign_no_combat_damage_followup, parse_required_damage_fanout,
    rebind_plural_create_followup_damage_source, restore_authored_damage_source_surface,
};
#[path = "sentence_shape_predicates/sentence_shape_predicates_library.rs"]
mod sentence_shape_predicates_library_programs;
use sentence_shape_predicates_library_programs::{
    parse_manifest_dread_graveyard_card_to_hand, parse_prefix_then_look_at_top_exile_one,
    parse_put_cards_from_single_graveyard_on_bottom_owner_library_sentence,
    parse_source_and_blocked_creatures_top_library_shuffle_sentence,
};
#[path = "sentence_shape_predicates/sentence_shape_predicates_counter.rs"]
mod sentence_shape_predicates_counter_programs;
use sentence_shape_predicates_counter_programs::bind_numeric_result_counter_amounts;
#[path = "sentence_shape_predicates/sentence_shape_predicates_object_action.rs"]
mod sentence_shape_predicates_object_action_programs;
use sentence_shape_predicates_object_action_programs::parse_create_token_then_copy_spell_chain;
