use super::*;

pub(super) fn parse_activated_effects_lexed(
    _effect_text: &str,
    tokens: &[OwnedLexToken],
    _line_index: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effect) = parse_choose_color_of_matching_object_mana_effect(tokens)? {
        return Ok(vec![effect]);
    }
    if activated_effect_is_for_each_color_among_add_mana_lexed(tokens) {
        return Ok(vec![crate::activation_helpers::parse_add_mana(
            tokens, None,
        )?]);
    }
    if let Some(effects) = parse_each_player_and_their_creatures_damage_sentence(tokens) {
        return Ok(effects);
    }
    if let Some(effects) = parse_hidden_look_partition_activated(tokens)? {
        return Ok(effects);
    }
    if let Some(effects) = parse_named_source_leading_gain_activated(tokens)? {
        return Ok(effects);
    }
    // Keep the P/T modification and evasion restriction as one activated
    // program. The broad restriction-oriented source-boundary parser can
    // otherwise claim only the trailing `can't be blocked` arm.
    if let Some(effects) =
        crate::effect_sentences::parse_source_gets_unblockable_subject_verb(tokens)?
    {
        return Ok(effects);
    }
    let words = token_word_refs(tokens);
    if crate::grammar::effects::gain_ability_shapes::parse_leading_gain_duration_shape(&words)
        .is_some()
        && let Some(effects) = crate::effect_sentences::parse_gain_ability_sentence(tokens)?
    {
        fn contains_compound_members(effects: &[EffectAst]) -> bool {
            let mut pump = false;
            let mut grant = false;
            fn inspect(effects: &[EffectAst], pump: &mut bool, grant: &mut bool) {
                for effect in effects {
                    if let EffectAst::SubjectVerb(subject_verb) = effect {
                        *pump |= matches!(subject_verb.action, SubjectVerbActionAst::Pump { .. });
                        *grant |= matches!(
                            subject_verb.action,
                            SubjectVerbActionAst::GrantAbilitiesToTarget { .. }
                        );
                    }
                    crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                        inspect(nested, pump, grant)
                    });
                }
            }
            inspect(effects, &mut pump, &mut grant);
            pump && grant
        }
        // Activated bodies such as "Until end of turn, this creature gets
        // +1/+1 ... and gains menace" are one coordinated modifier.  The
        // generic source-boundary path can otherwise claim the leading
        // `gets` as an unrelated counter-gain action.
        if contains_compound_members(&effects) {
            return Ok(effects);
        }
    }
    if let Ok(effects) = parse_effect_sentences_preserving_source_boundaries(tokens) {
        return Ok(effects);
    }

    let sentence_chunks = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentence_chunks.is_empty() {
        return Err(CardTextError::ParseError(
            "rewrite activated effect parser found no sentences".to_string(),
        ));
    }

    let mut effects = Vec::new();
    for sentence_lexed in sentence_chunks {
        if let Some(effect) = parse_next_spell_cost_reduction_sentence(sentence_lexed) {
            effects.push(effect);
            continue;
        }
        effects.extend(parse_effect_sentences_lexed(sentence_lexed)?);
    }
    Ok(effects)
}

pub fn parse_activated_line(
    info: LineInfo,
    mut compiler_cost: crate::model::CompilerTotalCost,
    cost_parse_tokens: Vec<OwnedLexToken>,
    effect_parse_tokens: Vec<OwnedLexToken>,
    timing_hint: ActivationTiming,
    is_loyalty_ability: bool,
    presentation: Option<PresentationLabel>,
    chosen_option: Option<ChosenOptionContext>,
) -> Result<ParsedActivatedLine, CardTextError> {
    // Labeled/public activation parsing first produces the generic cost CST.
    // Reconcile the exact zone-movement payment from its retained cost tokens
    // before that CST's broad `put ... cards` interpretation can survive as a
    // counter-placement cost. The grammar is strict about count, source zone,
    // ownership scope, and library destination.
    if let Some(cost) =
        crate::activation_and_restrictions::parse_single_graveyard_bottom_library_compiler_payment(
            &cost_parse_tokens,
        )
    {
        compiler_cost = crate::model::CompilerTotalCost::from_core_total_cost(cost);
    }
    parse_activated_line_impl(
        &RewriteActivatedLine {
            functional_zones: activated_grammar::parse_activated_functional_zones_tokens(
                &cost_parse_tokens,
                &effect_parse_tokens,
            ),
            presentation_kind: activated_grammar::parse_activated_presentation_kind_tokens(
                &info.source_tokens,
            ),
            presentation,
            info,
            compiler_cost,
            cost_parse_tokens: cost_parse_tokens.clone(),
            effect_parse_tokens: effect_parse_tokens.clone(),
            timing_hint,
            is_loyalty_ability,
            chosen_option,
        },
        &effect_parse_tokens,
    )
}

pub(super) fn parse_activated_line_impl(
    line: &RewriteActivatedLine,
    original_effect_parse_tokens: &[OwnedLexToken],
) -> Result<ParsedActivatedLine, CardTextError> {
    let x_definition_value = activated_x_definition_value(original_effect_parse_tokens);
    let has_x_definition_value = x_definition_value.is_some();
    let SplitRewriteActivatedEffectText {
        effect_text,
        effect_parse_tokens,
        restrictions,
        mana_restrictions,
        x_cant_be_zero,
    } = split_rewrite_activated_effect_text(original_effect_parse_tokens);
    if effect_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite activated lowering produced no parsed effect text for '{}'",
            line.info.raw_line
        )));
    }

    let normalized_cost = bind_activated_x_definition_to_mana_cost(
        line.compiler_cost.to_core_total_cost(),
        x_definition_value,
    );
    let original_effect_mentions_where_x =
        activated_grammar::contains_where_x_definition(original_effect_parse_tokens);
    let ability_text = rewrite_activated_display_text(line);
    let presentation_display = activated_presentation_display(line);
    let is_forecast = presentation_display
        .as_deref()
        .is_some_and(|display| display.eq_ignore_ascii_case("Forecast"));
    let normalized_cost = if is_forecast {
        mark_forecast_reveal_duration(normalized_cost)
    } else {
        normalized_cost
    };
    let parsed_restriction_timing =
        crate::slice_primitives::select_position(&restrictions.activation, |restriction| {
            restriction
                .timing
                .is_some_and(|timing| timing != ActivationTiming::AnyTime)
        })
        .and_then(|idx| restrictions.activation[idx].timing);
    let activation_timing = if is_forecast {
        ActivationTiming::DuringSourceOwnersUpkeep
    } else if line.timing_hint != ActivationTiming::AnyTime {
        line.timing_hint
    } else if authored_trailing_sorcery_speed_restriction(&line.info.raw_line) {
        ActivationTiming::SorcerySpeed
    } else {
        parsed_restriction_timing.unwrap_or(ActivationTiming::AnyTime)
    };
    let activation_restrictions = is_forecast
        .then_some(crate::ConditionExpr::MaxActivationsPerTurn(1))
        .into_iter()
        .collect::<Vec<_>>();
    let mut additional_activation_restrictions =
        if line.presentation_kind == Some(crate::ir::ActivatedPresentationKind::Exhaust) {
            vec!["Activate each exhaust ability only once.".to_string()]
        } else {
            Vec::new()
        };
    if let Some(display) = presentation_display.as_deref() {
        additional_activation_restrictions.push(format!("__ironsmith_activation_label:{display}"));
    }
    if x_cant_be_zero {
        additional_activation_restrictions.push("X can't be 0.".to_string());
    }
    if activated_grammar::contains_add_x_mana(&effect_parse_tokens)
        && !has_x_definition_value
        && !original_effect_mentions_where_x
        && !activation_cost_defines_x_for_mana_ability(&normalized_cost)
    {
        return Err(CardTextError::ParseError(
            "unresolved X in mana ability".to_string(),
        ));
    }

    if let Some(level) = activated_grammar::parse_level_number_tokens(&effect_parse_tokens) {
        let parsed = ParsedAbility {
            ability: Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: normalized_cost,
                    effects: ironsmith_core::ResolutionProgram::from_effects(vec![
                        EffectAst::subject_verb_put_counters(
                            CounterType::Level,
                            Value::Fixed(1),
                            TargetAst::Source(None),
                            None,
                            false,
                        ),
                    ]),
                    choices: vec![],
                    timing: ActivationTiming::SorcerySpeed,
                    is_loyalty_ability: line.is_loyalty_ability,
                    additional_restrictions: vec![format!("__ironsmith_class_level:{level}")],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones: vec![Zone::Battlefield],
            }
            .into(),
            text: Some(line.info.raw_line.trim().to_string()),
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        };
        return Ok(ParsedActivatedLine {
            chunk: LineAst::Ability(parsed),
            restrictions,
        });
    }

    if let Some(spec) = parse_fixed_mana_output_clause_spec_lexed(&effect_parse_tokens) {
        let functional_zones = infer_rewrite_activated_functional_zones(line)?;
        let mut parsed = ParsedAbility {
            ability: Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: normalized_cost.clone(),
                    effects: ironsmith_core::ResolutionProgram::default(),
                    choices: vec![],
                    timing: activation_timing,
                    is_loyalty_ability: line.is_loyalty_ability,
                    additional_restrictions: additional_activation_restrictions.clone(),
                    activation_restrictions: activation_restrictions.clone(),
                    mana_output: Some(spec.mana),
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones: if functional_zones.is_empty() {
                    vec![Zone::Battlefield]
                } else {
                    functional_zones
                },
            }
            .into(),
            text: ability_text.clone(),
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        };
        apply_pending_mana_restrictions(&mut parsed, &mana_restrictions)?;
        apply_chosen_option_condition_to_activated(&mut parsed, line.chosen_option.as_ref());
        return Ok(ParsedActivatedLine {
            chunk: LineAst::Ability(parsed),
            restrictions,
        });
    }

    if activated_effect_may_be_mana_ability_lexed(&effect_parse_tokens) {
        let effects_ast = normalize_mana_replacement_effects(parse_activated_effects_lexed(
            effect_text.as_str(),
            &effect_parse_tokens,
            line.info.line_index,
        )?);
        if effects_ast_can_lower_as_mana_ability(&effects_ast)
            || effects_ast
                .first()
                .is_some_and(effect_ast_starts_with_mana_effect)
        {
            let functional_zones = infer_rewrite_activated_functional_zones(line)?;
            let reference_imports = compiler_activation_cost_reference_imports(&normalized_cost);
            let mut parsed = ParsedAbility {
                ability: Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost: normalized_cost.clone(),
                        effects: ironsmith_core::ResolutionProgram::default(),
                        choices: vec![],
                        timing: activation_timing,
                        is_loyalty_ability: line.is_loyalty_ability,
                        additional_restrictions: additional_activation_restrictions.clone(),
                        activation_restrictions: activation_restrictions.clone(),
                        mana_output: Some(vec![]),
                        activation_condition: None,
                        mana_usage_restrictions: vec![],
                    }),
                    functional_zones: if functional_zones.is_empty() {
                        vec![Zone::Battlefield]
                    } else {
                        functional_zones
                    },
                }
                .into(),
                text: ability_text.clone(),
                effects_ast: Some(effects_ast),
                reference_imports,
                trigger_spec: None,
            };
            apply_pending_mana_restrictions(&mut parsed, &mana_restrictions)?;
            apply_chosen_option_condition_to_activated(&mut parsed, line.chosen_option.as_ref());

            return Ok(ParsedActivatedLine {
                chunk: LineAst::Ability(parsed),
                restrictions,
            });
        }
        return Err(CardTextError::ParseError(format!(
            "rewrite activated lowering does not yet support mana-style activated effect '{}'",
            line.info.raw_line
        )));
    }

    let mut effects_ast = parse_activated_effects_lexed(
        effect_text.as_str(),
        &effect_parse_tokens,
        line.info.line_index,
    )?;
    reconcile_named_source_exile_surfaces(&line.info, &mut effects_ast);
    if activation_cost_sets_x_from_counter_removal(&normalized_cost) {
        bind_event_amounts_to_cost_x(&mut effects_ast);
    }
    let functional_zones = infer_rewrite_activated_functional_zones(line)?;
    let reference_imports = compiler_activation_cost_reference_imports(&normalized_cost);
    let mut parsed = ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: normalized_cost,
                effects: ironsmith_core::ResolutionProgram::default(),
                choices: vec![],
                timing: activation_timing,
                is_loyalty_ability: line.is_loyalty_ability,
                additional_restrictions: additional_activation_restrictions,
                activation_restrictions,
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: if functional_zones.is_empty() {
                vec![Zone::Battlefield]
            } else {
                functional_zones
            },
        }
        .into(),
        text: ability_text,
        effects_ast: Some(effects_ast),
        reference_imports,
        trigger_spec: None,
    };
    apply_pending_mana_restrictions(&mut parsed, &mana_restrictions)?;
    apply_chosen_option_condition_to_activated(&mut parsed, line.chosen_option.as_ref());

    Ok(ParsedActivatedLine {
        chunk: LineAst::Ability(parsed),
        restrictions,
    })
}

pub(super) fn rewrite_activated_display_text(line: &RewriteActivatedLine) -> Option<String> {
    let display = activated_presentation_display(line)?;
    Some(format!(
        "{display} — {}: {}",
        render_token_slice(&line.cost_parse_tokens).trim(),
        render_token_slice(&line.effect_parse_tokens).trim()
    ))
}

pub(super) fn activated_presentation_display(line: &RewriteActivatedLine) -> Option<String> {
    line.presentation
        .as_ref()
        .and_then(PresentationLabel::display_prefix)
        .or_else(|| {
            line.presentation_kind
                .map(|kind| kind.display().to_string())
        })
}

pub(super) fn infer_rewrite_activated_functional_zones(
    line: &RewriteActivatedLine,
) -> Result<Vec<Zone>, CardTextError> {
    Ok(line.functional_zones.clone())
}
