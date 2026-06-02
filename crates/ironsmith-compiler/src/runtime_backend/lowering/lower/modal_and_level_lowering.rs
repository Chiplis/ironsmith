use super::*;

pub(crate) fn try_merge_modal_into_remove_mode(
    effects: &mut crate::resolution::ResolutionProgram,
    modal_effect: crate::effect::Effect,
    predicate: crate::effect::EffectPredicate,
) -> bool {
    let Some(last_effect) = effects.pop() else {
        return false;
    };

    let Some(choose_mode) = last_effect.downcast_ref::<crate::effects::ChooseModeEffect>() else {
        effects.push(last_effect);
        return false;
    };
    if choose_mode.modes.len() < 2 {
        effects.push(last_effect);
        return false;
    }

    let Some(remove_mode_idx) = find_index(choose_mode.modes.as_slice(), |mode| {
        lex_line(mode.description.as_str(), 0)
            .ok()
            .is_some_and(|tokens| tokens_start_with_remove(&tokens))
    }) else {
        effects.push(last_effect);
        return false;
    };

    let mut modes = choose_mode.modes.clone();
    let remove_mode = &mut modes[remove_mode_idx];
    let gate_id = crate::effect::EffectId(1_000_000_000);
    if let Some(last_remove_effect) = remove_mode.effects.pop() {
        remove_mode.effects.push(crate::effect::Effect::with_id(
            gate_id.0,
            last_remove_effect,
        ));
        remove_mode.effects.push(crate::effect::Effect::if_then(
            gate_id,
            predicate,
            vec![modal_effect],
        ));
    } else {
        remove_mode.effects.push(modal_effect);
    }

    #[cfg(not(test))]
    effects.push(crate::effect::Effect::new(
        crate::effects::ChooseModeEffect {
            modes,
            min: choose_mode.min.clone(),
            max: choose_mode.max.clone(),
            allow_repeat: choose_mode.allow_repeat,
            random: choose_mode.random,
            choose_count: choose_mode.choose_count.clone(),
            min_choose_count: choose_mode.min_choose_count.clone(),
            allow_repeated_modes: choose_mode.allow_repeated_modes,
            mode_point_costs: choose_mode.mode_point_costs.clone(),
            disallow_previously_chosen_modes: choose_mode.disallow_previously_chosen_modes,
            disallow_previously_chosen_modes_this_turn: choose_mode
                .disallow_previously_chosen_modes_this_turn,
        },
    ));
    #[cfg(feature = "serialization")]
    effects.push(crate::effect::Effect::new(
        crate::effects::ChooseModeEffect {
            modes,
            random: choose_mode.random,
            choose_count: choose_mode.choose_count.clone(),
            min_choose_count: choose_mode.min_choose_count.clone(),
            allow_repeated_modes: choose_mode.allow_repeated_modes,
            mode_point_costs: choose_mode.mode_point_costs.clone(),
            disallow_previously_chosen_modes: choose_mode.disallow_previously_chosen_modes,
            disallow_previously_chosen_modes_this_turn: choose_mode
                .disallow_previously_chosen_modes_this_turn,
        },
    ));
    true
}

fn header_mentions_modal_point_cost(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("worth of modes") && lower.contains("{p}")
}

fn parse_leading_modal_point_cost(text: &str) -> Option<u32> {
    let mut rest = text.trim_start();
    let mut count = 0u32;
    loop {
        let trimmed = rest.trim_start();
        if !trimmed.get(..3).is_some_and(|prefix| prefix.eq_ignore_ascii_case("{p}")) {
            rest = trimmed;
            break;
        }
        count += 1;
        rest = &trimmed[3..];
    }
    if count == 0 {
        return None;
    }
    let rest = rest.trim_start();
    rest.starts_with('—').then_some(count)
}

pub(crate) fn rewrite_lower_parsed_modal(
    mut builder: CardDefinitionBuilder,
    pending_modal: NormalizedModalAst,
    allow_unsupported: bool,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let NormalizedModalAst {
        header,
        prepared_prefix,
        modes,
    } = pending_modal;
    let crate::cards::builders::ParsedModalHeader {
        min: header_min,
        max: header_max,
        random: random_mode_choice,
        same_mode_more_than_once,
        mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn,
        commander_allows_both,
        choose_both_control_card_types,
        trigger,
        activated,
        x_replacement: _,
        prefix_effects_ast: _,
        modal_gate,
        line_text,
    } = header;

    let (prefix_effects, prefix_choices) = if prepared_prefix.is_none() {
        (crate::resolution::ResolutionProgram::default(), Vec::new())
    } else if trigger.is_some() || activated.is_some() {
        match materialize_prepared_effects_with_trigger_context(
            prepared_prefix
                .as_ref()
                .expect("prepared prefix exists when checked above"),
        ) {
            Ok(lowered) => (lowered.effects, lowered.choices),
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(builder, line_text.as_str(), format!("{err:?}"));
                return Ok(builder);
            }
            Err(err) => return Err(err),
        }
    } else {
        match rewrite_lower_prepared_statement_effects(
            prepared_prefix
                .as_ref()
                .expect("prepared prefix exists when checked above"),
        ) {
            Ok(lowered) => (lowered.effects, lowered.choices),
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(builder, line_text.as_str(), format!("{err:?}"));
                return Ok(builder);
            }
            Err(err) => return Err(err),
        }
    };

    let weighted_mode_points = header_mentions_modal_point_cost(line_text.as_str());
    let mut compiled_modes = Vec::new();
    let mut mode_point_costs = Vec::new();
    for mode in modes {
        let point_cost = parse_leading_modal_point_cost(mode.info.raw_line.as_str()).unwrap_or(1);
        let effects = match rewrite_lower_prepared_statement_effects(&mode.prepared) {
            Ok(lowered) => lowered.effects,
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(
                    builder,
                    mode.info.raw_line.as_str(),
                    format!("{err:?}"),
                );
                continue;
            }
            Err(err) => return Err(err),
        };
        compiled_modes.push(crate::effect::EffectMode {
            description: mode.description,
            effects: effects.to_vec(),
        });
        mode_point_costs.push(point_cost);
    }

    if compiled_modes.is_empty() {
        return Ok(builder);
    }

    let mode_count = compiled_modes.len() as i32;
    let default_max = crate::effect::Value::Fixed(mode_count);
    let max = header_max.unwrap_or_else(|| default_max.clone());
    let min = header_min;
    let is_fixed_one =
        |value: &crate::effect::Value| matches!(value, crate::effect::Value::Fixed(1));
    let apply_modal_metadata = |effect: crate::effect::Effect| {
        let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() else {
            return effect;
        };
        let mut choose_mode = choose_mode.clone();
        if same_mode_more_than_once {
            choose_mode = choose_mode.with_repeated_modes();
        }
        if random_mode_choice {
            choose_mode = choose_mode.with_random_mode_choice();
        }
        if weighted_mode_points {
            choose_mode = choose_mode.with_mode_point_costs(mode_point_costs.clone());
        }
        if mode_must_be_unchosen {
            choose_mode = if mode_must_be_unchosen_this_turn {
                choose_mode.with_previously_unchosen_modes_only_this_turn()
            } else {
                choose_mode.with_previously_unchosen_modes_only()
            };
        }
        crate::effect::Effect::new(choose_mode)
    };

    let choose_both_condition = if commander_allows_both {
        Some(crate::effect::Condition::YouControlCommander)
    } else if choose_both_control_card_types.is_empty() {
        None
    } else {
        let mut conditions = choose_both_control_card_types.iter().map(|card_type| {
            crate::effect::Condition::PlayerControls {
                player: crate::target::PlayerFilter::You,
                filter: crate::filter::ObjectFilter {
                    card_types: vec![*card_type],
                    ..Default::default()
                },
            }
        });
        let first = conditions
            .next()
            .expect("non-empty card-type choose-both list");
        Some(conditions.fold(first, |left, right| {
            crate::effect::Condition::And(Box::new(left), Box::new(right))
        }))
    };

    let modal_effect = if let Some(choose_both_condition) = choose_both_condition {
        let max_both = (mode_count.min(2)).max(1);
        let choose_both = if max_both == 1 {
            apply_modal_metadata(crate::effect::Effect::choose_one(compiled_modes.clone()))
        } else {
            #[cfg(not(feature = "serialization"))]
            let choose_up_to = crate::effect::Effect::choose_up_to_with_min(
                crate::effect::Value::Fixed(max_both),
                crate::effect::Value::Fixed(1),
                compiled_modes.clone(),
            );
            #[cfg(feature = "serialization")]
            let choose_up_to = crate::effect::Effect::choose_up_to(
                crate::effect::Value::Fixed(max_both),
                crate::effect::Value::Fixed(1),
                compiled_modes.clone(),
            );
            apply_modal_metadata(choose_up_to)
        };
        let choose_one = apply_modal_metadata(crate::effect::Effect::choose_one(compiled_modes.clone()));
        crate::effect::Effect::conditional(
            choose_both_condition,
            vec![choose_both],
            vec![choose_one],
        )
    } else if same_mode_more_than_once && min == max {
        apply_modal_metadata(crate::effect::Effect::choose_exactly_allow_repeated_modes(
            max.clone(),
            compiled_modes,
        ))
    } else if is_fixed_one(&min) && is_fixed_one(&max) {
        apply_modal_metadata(crate::effect::Effect::choose_one(compiled_modes))
    } else if min == max {
        apply_modal_metadata(crate::effect::Effect::choose_exactly(
            max.clone(),
            compiled_modes,
        ))
    } else {
        #[cfg(not(feature = "serialization"))]
        let choose_up_to =
            crate::effect::Effect::choose_up_to_with_min(max.clone(), min.clone(), compiled_modes);
        #[cfg(feature = "serialization")]
        let choose_up_to =
            crate::effect::Effect::choose_up_to(max.clone(), min.clone(), compiled_modes);
        apply_modal_metadata(choose_up_to)
    };

    let mut combined_effects = prefix_effects;
    if let Some(modal_gate) = modal_gate {
        if modal_gate.remove_mode_only
            && try_merge_modal_into_remove_mode(
                &mut combined_effects,
                modal_effect.clone(),
                modal_gate.predicate.clone(),
            )
        {
        } else if let Some(last_effect) = combined_effects.pop() {
            let gate_id = crate::effect::EffectId(1_000_000_000);
            combined_effects.push(crate::effect::Effect::with_id(gate_id.0, last_effect));
            combined_effects.push(crate::effect::Effect::if_then(
                gate_id,
                modal_gate.predicate,
                vec![modal_effect],
            ));
        } else {
            combined_effects.push(modal_effect);
        }
    } else {
        combined_effects.push(modal_effect);
    }

    let modal_lowered = LoweredEffects {
        effects: combined_effects.clone(),
        choices: prefix_choices.clone(),
        exports: ReferenceExports::default(),
    };
    rewrite_validate_iterated_player_bindings_in_lowered_effects(
        &modal_lowered,
        trigger
            .as_ref()
            .is_some_and(rewrite_trigger_binds_player_reference_context),
        if trigger.is_some() {
            "triggered modal ability effects"
        } else if activated.is_some() {
            "activated modal ability effects"
        } else {
            "modal spell effects"
        },
    )?;

    if let Some(trigger) = trigger {
        let mut ability = rewrite_parsed_triggered_ability(
            trigger,
            Vec::new(),
            vec![Zone::Battlefield],
            Some(line_text),
            None,
            None,
            ReferenceImports::default(),
        )
        .into_runtime();
        if let AbilityKind::Triggered(triggered) = &mut ability.kind {
            triggered.effects = combined_effects.clone();
            triggered.choices = prefix_choices;
        }
        builder = builder.with_ability(ability);
    } else if let Some(activated) = activated {
        builder = builder.with_ability(Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: activated.mana_cost,
                effects: combined_effects.clone(),
                choices: prefix_choices,
                timing: activated.timing,
                is_loyalty_ability: activated.is_loyalty_ability,
                additional_restrictions: activated.additional_restrictions,
                activation_restrictions: activated.activation_restrictions,
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: activated.functional_zones,
        });
    } else if let Some(ref mut existing) = builder.spell_effect {
        existing.extend(combined_effects);
    } else {
        builder.spell_effect = Some(combined_effects);
    }

    Ok(builder)
}
