use crate::runtime_backend::effect_sentences::{
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
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::DestroyAll { filter, .. }
                    | SubjectVerbActionAst::ExileAll { filter, .. },
                ..
            }) => {
                if filter.with_counter.is_none() {
                    filter.with_counter = Some(counter_constraint);
                }
            }
            _ => {}
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

pub(crate) fn lower_where_x_shape(
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
            let object_tokens =
                crate::runtime_backend::lexer::synthetic_word_tokens(&[object_kind.as_str()]);
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
            let object_tokens =
                crate::runtime_backend::lexer::synthetic_word_tokens(&[object_kind]);
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
    let clause_words = crate::runtime_backend::token_word_refs(shape.tail_tokens);
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
    let words = crate::runtime_backend::token_word_refs(tokens);
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
    let words = crate::runtime_backend::token_word_refs(tokens);
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
        crate::runtime_backend::effect_ast_traversal::for_each_nested_effects_mut(
            effect,
            true,
            |nested| {
                if !found {
                    found = set_first_continuous_set_quantifier(nested, surface);
                }
            },
        );
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
        crate::runtime_backend::effect_ast_traversal::for_each_nested_effects_mut(
            effect,
            true,
            |nested| {
                if !found {
                    found = set_first_return_set_reference_surface(nested, surface);
                }
            },
        );
        if found {
            return true;
        }
    }
    false
}

fn parse_bounded_x_mana_payment_sentence(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let may_shape = effect_grammar::clause_dispatch_shapes::parse_leading_may_clause_shape(tokens)?;
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

pub(crate) fn parse_effect_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = stacker::maybe_grow(32 * 1024 * 1024, 64 * 1024 * 1024, || {
        if let Some(effects) = parse_bounded_x_mana_payment_sentence(tokens) {
            Ok(effects)
        } else {
            parse_effect_sentence_lexed_inner(tokens)
        }
    })?;
    if let Some(surface) = parse_set_quantifier_surface(tokens) {
        set_first_continuous_set_quantifier(&mut effects, surface);
    }
    if let Some(surface) = parse_return_set_reference_surface(tokens) {
        set_first_return_set_reference_surface(&mut effects, &surface);
    }
    Ok(
        crate::runtime_backend::effect_sentences::preserve_coordinated_effect_chain_surface(
            tokens, effects,
        ),
    )
}

fn has_unrecognized_leading_effect_label(tokens: &[OwnedLexToken]) -> bool {
    if crate::runtime_backend::grammar::structure::split_leading_result_prefix_lexed(tokens)
        .is_some()
    {
        return false;
    }
    effect_grammar::labeled_dispatch::parse_leading_effect_label_tokens(tokens).is_some_and(
        |shape| shape.kind == effect_grammar::labeled_dispatch::LeadingEffectLabelKind::Unknown,
    )
}

fn parse_manifest_dread_graveyard_card_to_hand(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let words = crate::runtime_backend::token_word_refs(tokens);
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
    stacker::maybe_grow(32 * 1024 * 1024, 64 * 1024 * 1024, || {
        parse_effect_sentence_lexed_inner_unstacked(tokens)
    })
}

fn parse_attacking_doesnt_tap_if_source_untapped(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let action_tokens =
        crate::runtime_backend::token_primitives::strip_leading_if_you_do_lexed(tokens);
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
        crate::runtime_backend::effect_ast_traversal::for_each_nested_effects_mut(
            effect,
            true,
            bind_numeric_result_counter_amounts,
        );
    }
}

fn parse_effect_sentence_lexed_inner_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
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
            predicate: crate::cards::builders::IfResultPredicate::DidNot,
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

    // A genuine top-level conjunction with a leading duration needs chain
    // carry before broad gain/subject recognizers see an isolated arm. The
    // grammar predicate rejects quoted/list conjunctions and `then` chains.
    if effect_grammar::chain_carry::coordinated_effect_chain_leading_duration(tokens) == Some(true)
    {
        return super::parse_effect_chain_lexed(tokens);
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
            if !matches!(search_player, PlayerAst::You | PlayerAst::Implicit) {
                if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject,
                    action: SubjectVerbActionAst::ShuffleLibrary,
                }) = &mut effects[idx]
                {
                    subject.player = search_player;
                }
            }
        }
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

    if let Some(effect) =
        crate::runtime_backend::permission_helpers::parse_cast_or_play_tagged_clause(tokens)?
    {
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
        super::player_subject_sequences::split_quantified_opponent_then_controller_clauses(tokens)
    {
        let mut effects = Vec::new();
        for clause in clauses {
            effects.extend(parse_effect_sentence_lexed_inner(clause)?);
        }
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
        let source_words = crate::runtime_backend::token_word_refs(shape.source_tokens);
        let count = crate::runtime_backend::front_end::grammar::shared_util::count_shapes::mana_from_source_spent_to_cast_value(
            &source_words,
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
        count_words.extend(crate::runtime_backend::token_word_refs(shape.filter_tokens));
        if let Some((count, used)) =
            crate::runtime_backend::util::parse_for_each_count_value_words(&count_words)
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
    ) {
        if let Some(effects) = parse_delayed_until_next_end_step_sentence(tokens)? {
            return Ok(effects);
        }
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
    let quoted_ability_shape = sentence_shapes::parse_quoted_ability_sentence_tokens(tokens);
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
    if quoted_ability_shape.is_some()
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
        crate::runtime_backend::effect_sentences::dispatch_entry::future_zone_replacement_from_sentence_tokens(tokens)
    {
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
            action:
                SubjectVerbActionAst::DealDamageEqualToPower {
                    source,
                    ..
                },
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
        crate::runtime_backend::effect_ast_traversal::for_each_nested_effects_mut(
            effect,
            true,
            |nested| {
                rebind_plural_create_followup_damage_source(nested);
            },
        );
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
        crate::runtime_backend::families::keyword_static::parse_where_x_is_aggregate_filter_value(
            where_shape.where_tokens,
        );
    let turn_history_where = aggregate_where.is_none().then(|| {
        crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_value_binding(
            where_shape.where_tokens,
        )
    }).flatten();
    let full_where_is_count_value = !where_shape.comma_tail_has_effect_clause
        && (turn_history_where.is_some()
            || crate::runtime_backend::families::keyword_static::parse_where_x_is_sum_of_number_of_filter_values(
                where_shape.where_tokens,
            )
            .is_some()
            || crate::runtime_backend::families::keyword_static::parse_where_x_is_number_of_filter_value(
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
    // Preserve the established source-relative interpretation for a where-X
    // binding that is followed by another effect clause. The follow-up is
    // dispatched independently; it must not cause the primary clause's
    // contextual `its` value to be promoted to a new target choice.
    let typed_where_references_target = where_shape.stripped_references_target
        && !where_shape.comma_tail_has_effect_clause
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
            crate::runtime_backend::front_end::grammar::values::
                parse_players_who_control_more_than_you_value_lexed(primary_where_tokens)
        })
        .flatten();
    let complete_number_where =
        (turn_history_where.is_none() && participant_comparison_where.is_none())
            .then(|| {
                crate::runtime_backend::families::keyword_static::
                    parse_where_x_is_number_of_filter_value(primary_where_tokens)
            })
            .flatten();
    let typed_where_value = if turn_history_where.is_none()
        && participant_comparison_where.is_none()
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
        let specific_where_value =
            super::dispatch_entry::parse_exact_where_x_value_expression(primary_where_tokens)
            .or_else(|| {
                crate::runtime_backend::front_end::grammar::values::parse_players_who_control_more_than_you_value_lexed(
                primary_where_tokens,
            )
            })
            .or_else(|| {
                crate::runtime_backend::families::keyword_static::parse_where_x_is_sum_of_number_of_filter_values(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::runtime_backend::families::keyword_static::parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::runtime_backend::families::keyword_static::parse_where_x_is_number_of_different_powers_filter_value(
                    primary_where_tokens,
                )
            });
        let number_of_filter_value = specific_where_value
            .or_else(|| {
                crate::runtime_backend::families::keyword_static::parse_where_x_is_colored_mana_symbols_value(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                crate::runtime_backend::families::keyword_static::parse_where_x_is_number_of_filter_value(
                    primary_where_tokens,
                )
            })
            .or_else(|| {
                activation_time_trimmed.as_deref().and_then(
                    crate::runtime_backend::families::keyword_static::parse_where_x_is_number_of_filter_value,
                )
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
    let where_value =
        crate::runtime_backend::effect_sentences::dispatch_entry::with_where_x_surface_hints(
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

    #[test]
    fn for_each_mana_from_source_repeats_the_typed_effect() {
        let tokens = crate::runtime_backend::lex_line(
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
        let tokens =
            crate::runtime_backend::lex_line("For each {U}{U} spent to cast it, draw a card.", 0)
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
        let body_tokens = crate::runtime_backend::lex_line(
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

        let tokens = crate::runtime_backend::lex_line(
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
}
