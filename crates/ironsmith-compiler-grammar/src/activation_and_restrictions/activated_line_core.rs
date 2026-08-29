use super::*;
use crate::grammar::activated_lines::{
    self as activated_line_grammar, ActivatedAbilitiesReductionRemainder,
    ActivatedBlockRequirement, ActivatedDevotionParseError, ActivatedLoyaltyShorthand,
    CostReductionLineHead, EntersTappedLineShape, ThisAbilityReductionRemainder,
    ThisCostReductionRemainder, ThisSpellReductionRemainder,
};
use crate::grammar::leaf::parse_leaf_fixed_mana_output_tokens;
use crate::lexer::render_token_slice;
use crate::model::ast::SubjectVerbActionAst;
use crate::util::SubjectAst;

pub type ActivationRestrictionCompatWords<'a> = grammar::TokenWordView<'a>;

pub fn joined_activation_clause_text(tokens: &[OwnedLexToken]) -> String {
    crate::lexer::token_word_refs(tokens).join(" ")
}

pub fn parse_prefixed_activated_ability_label(
    tokens: &[OwnedLexToken],
    cost_start: usize,
) -> Option<String> {
    if cost_start == 0 {
        return None;
    }

    let prefix = ActivationRestrictionCompatWords::new(&tokens[..cost_start]);
    match prefix.get(prefix.len().saturating_sub(1)) {
        Some("boast") => Some("Boast".to_string()),
        Some("exhaust") => Some("Exhaust".to_string()),
        Some("renew") => Some("Renew".to_string()),
        _ => None,
    }
}

pub fn parse_hand_keyword_activated_body_lexed(
    body_tokens: &[OwnedLexToken],
    keyword: &str,
    display_label: &str,
    clause_text: &str,
) -> Result<Option<ParsedAbility>, CardTextError> {
    if body_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "{keyword} line missing activated ability body (clause: '{clause_text}')",
        )));
    }

    let ability_tokens = trim_commas(body_tokens);
    let Some(mut parsed) = parse_activated_line_with_raw(&ability_tokens)? else {
        return Ok(None);
    };
    *parsed.text_mut() = Some(display_label.to_string());
    *parsed.functional_zones_mut() = vec![Zone::Hand];
    Ok(Some(parsed))
}

pub fn parse_activated_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_activated_line_with_raw(tokens)
}

fn subject_allows_direct_mana_output(subject: &Option<SubjectAst>) -> bool {
    matches!(
        subject,
        None | Some(SubjectAst::Player(PlayerAst::You | PlayerAst::Implicit))
    )
}

#[inline(never)]
fn parse_direct_controller_sacrifice_draw_program(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(period_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.kind == TokenKind::Period)
    else {
        return Ok(None);
    };
    let action_tokens = &tokens[..period_idx];
    let timing_tokens = &tokens[period_idx + 1..];
    if timing_tokens.len() != 5
        || !timing_tokens
            .iter()
            .zip(["activate", "only", "as", "a", "sorcery"])
            .all(|(token, expected)| token.is_word(expected))
    {
        return Ok(None);
    }
    let Some(sacrifice_idx) = crate::slice_primitives::select_position(action_tokens, |token| {
        token.is_word("sacrifice") || token.is_word("sacrifices")
    }) else {
        return Ok(None);
    };
    let Some(and_idx) = action_tokens
        .iter()
        .enumerate()
        .skip(sacrifice_idx + 1)
        .find_map(|(index, token)| token.is_word("and").then_some(index))
    else {
        return Ok(None);
    };
    let sacrifice_tokens = &action_tokens[..and_idx];
    let draw_tokens = &action_tokens[and_idx + 1..];
    if sacrifice_idx == 0
        || !draw_tokens
            .first()
            .is_some_and(|token| token.is_word("draw") || token.is_word("draws"))
    {
        return Ok(None);
    }
    let controller_words = crate::lexer::token_word_refs(&sacrifice_tokens[..sacrifice_idx]);
    if controller_words.last().copied() != Some("controller")
        || controller_words.len() < 2
        || matches!(controller_words.first().copied(), Some("target" | "that"))
    {
        return Ok(None);
    }
    let sacrifice_object_words =
        crate::lexer::token_word_refs(&sacrifice_tokens[sacrifice_idx + 1..]);
    let draw_words = crate::lexer::token_word_refs(&draw_tokens[1..]);
    if !crate::word_primitives::parse_sequence_complete(&sacrifice_object_words, &["it"])
        || !crate::word_primitives::parse_any_sequence_complete(
            &draw_words,
            &[&["a", "card"], &["one", "card"]],
        )
    {
        return Ok(None);
    }
    let sacrifice = EffectAst::subject_verb_sacrifice(
        PlayerAst::ItsController,
        ObjectFilter::tagged(IT_TAG),
        1,
        None,
    );
    let draw = EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::ItsController,
        SubjectVerbActionAst::Draw {
            count: Value::Fixed(1),
        },
    );
    Ok(Some(vec![sacrifice, draw]))
}

#[inline(never)]
fn parse_direct_controller_sacrifice_draw_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(colon_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.kind == TokenKind::Colon)
    else {
        return Ok(None);
    };
    let before_colon = &tokens[..colon_idx];
    let effect_tokens = &tokens[colon_idx + 1..];
    if !before_colon
        .first()
        .is_some_and(|token| token.kind == TokenKind::ManaGroup)
    {
        return Ok(None);
    }
    let cost_start = 0;
    let cost_tokens = before_colon;
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return Ok(None);
    }
    let Some(effects) = parse_direct_controller_sacrifice_draw_program(effect_tokens)? else {
        return Ok(None);
    };
    let ability_label = parse_prefixed_activated_ability_label(tokens, cost_start);
    let ability_display_text = prefixed_activated_ability_display_text(
        ability_label.as_deref(),
        cost_tokens,
        effect_tokens,
    );
    let mana_cost = parse_compiler_activation_cost(cost_tokens)?;
    let reference_imports = ReferenceImports::default();
    let functional_zones = vec![Zone::Battlefield];
    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost,
                effects: ironsmith_core::ResolutionProgram::default(),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                is_loyalty_ability: false,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones,
        }
        .into(),
        text: ability_display_text,
        effects_ast: Some(effects),
        reference_imports,
        trigger_spec: None,
    }))
}

#[inline(never)]
fn parse_direct_simple_effect_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(colon_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.kind == TokenKind::Colon)
    else {
        return Ok(None);
    };
    let cost_tokens = &tokens[..colon_idx];
    let effect_tokens = &tokens[colon_idx + 1..];
    if cost_tokens.is_empty()
        || effect_tokens.is_empty()
        || !cost_tokens
            .first()
            .is_some_and(|token| token.kind == TokenKind::ManaGroup)
    {
        return Ok(None);
    }
    let trimmed_effect_tokens = crate::util::trim_edge_punctuation_tokens(effect_tokens);
    if trimmed_effect_tokens.is_empty()
        || trimmed_effect_tokens
            .iter()
            .any(|token| token.kind == TokenKind::Period)
    {
        return Ok(None);
    }
    let effect = if trimmed_effect_tokens[0].is_word("destroy") && trimmed_effect_tokens.len() >= 2
    {
        EffectAst::subject_verb_destroy(parse_target_phrase(&trimmed_effect_tokens[1..])?)
    } else if crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(trimmed_effect_tokens),
        &["draw", "a", "card"],
    ) {
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: Value::Fixed(1),
            },
        )
    } else {
        return Ok(None);
    };
    let mana_cost = parse_compiler_activation_cost(cost_tokens)?;
    let reference_imports =
        super::super::util::compiler_activation_cost_reference_imports(&mana_cost);
    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost,
                effects: ironsmith_core::ResolutionProgram::default(),
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                is_loyalty_ability: false,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
        }
        .into(),
        text: prefixed_activated_ability_display_text(None, cost_tokens, effect_tokens),
        effects_ast: Some(vec![effect]),
        reference_imports,
        trigger_spec: None,
    }))
}

#[inline(never)]
pub fn parse_activated_line_with_raw(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    if let Some(parsed) = parse_direct_simple_effect_ability(tokens)? {
        return Ok(Some(parsed));
    }
    if let Some(parsed) = parse_direct_controller_sacrifice_draw_ability(tokens)? {
        return Ok(Some(parsed));
    }
    parse_activated_line_with_raw_remaining(tokens)
}

#[inline(never)]
fn parse_activated_line_with_raw_remaining(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(line_split) = activated_line_grammar::parse_activated_line_split_tokens(tokens) else {
        return Ok(None);
    };

    // A symbol-led composite cost starts at the symbol even when a later
    // word-led component (for example, "Sacrifice") is also recognized as a
    // cost head. Choosing the later head silently drops `{T}` from granted
    // activated abilities.
    let cost_start = if line_split
        .before_colon
        .first()
        .is_some_and(|token| token.kind == TokenKind::ManaGroup)
    {
        0
    } else {
        find_activation_cost_start(line_split.before_colon).unwrap_or(0)
    };
    let cost_tokens = &line_split.before_colon[cost_start..];
    let effect_tokens = line_split.after_colon;
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return Ok(None);
    }
    let ability_label = parse_prefixed_activated_ability_label(tokens, cost_start);
    let ability_display_text = prefixed_activated_ability_display_text(
        ability_label.as_deref(),
        cost_tokens,
        effect_tokens,
    );
    let trimmed_effect_tokens = crate::util::trim_edge_punctuation_tokens(effect_tokens);
    if let Some(effects) = parse_direct_controller_sacrifice_draw_program(effect_tokens)? {
        let mana_cost = parse_compiler_activation_cost(cost_tokens)?;
        let reference_imports =
            super::super::util::compiler_activation_cost_reference_imports(&mana_cost);
        let effect_sentences = crate::lexer::split_lexed_sentences(effect_tokens);
        let functional_zones =
            infer_activated_functional_zones_lexed(cost_tokens, &effect_sentences);
        return Ok(Some(ParsedAbility {
            ability: Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost,
                    effects: ironsmith_core::ResolutionProgram::default(),
                    choices: vec![],
                    timing: ActivationTiming::SorcerySpeed,
                    is_loyalty_ability: false,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones,
            }
            .into(),
            text: ability_display_text,
            effects_ast: Some(effects),
            reference_imports,
            trigger_spec: None,
        }));
    }
    let direct_effect = if crate::lexer::split_lexed_sentences(effect_tokens).len() == 1 {
        if let Some(effect) = crate::effect_sentences::parse_anaphoric_object_deals_damage_clause(
            trimmed_effect_tokens,
        )? {
            Some(effect)
        } else if trimmed_effect_tokens
            .first()
            .is_some_and(|token| token.is_word("regenerate"))
            && trimmed_effect_tokens.len() > 1
        {
            Some(EffectAst::subject_verb_regenerate(parse_target_phrase(
                &trimmed_effect_tokens[1..],
            )?))
        } else if let Some(
            crate::grammar::effects::clause_pattern_shapes::KeywordMechanicShape::Numeric {
                keyword:
                    crate::grammar::effects::clause_pattern_shapes::NumericKeywordShape::Bolster,
                amount,
            },
        ) = crate::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(
            trimmed_effect_tokens,
        ) {
            Some(EffectAst::subject_verb_bolster(amount))
        } else if let Some(shape) =
            crate::grammar::effects::clause_dispatch_shapes::parse_clause_subject_verb_shape(
                trimmed_effect_tokens,
            )
            && shape.kind == crate::grammar::effects::chain_splitting::ChainVerbKind::Get
        {
            crate::effect_sentences::parse_get_pump_clause(
                shape.subject_tokens,
                shape.action_tokens,
                trimmed_effect_tokens,
            )?
        } else {
            None
        }
    } else {
        None
    };
    if let Some(effect) = direct_effect {
        let mana_cost = parse_compiler_activation_cost(cost_tokens)?;
        let reference_imports =
            super::super::util::compiler_activation_cost_reference_imports(&mana_cost);
        let functional_zones =
            infer_activated_functional_zones_lexed(cost_tokens, &[trimmed_effect_tokens]);
        return Ok(Some(ParsedAbility {
            ability: Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost,
                    effects: ironsmith_core::ResolutionProgram::default(),
                    choices: vec![],
                    timing: ActivationTiming::AnyTime,
                    is_loyalty_ability: false,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones,
            }
            .into(),
            text: ability_display_text,
            effects_ast: Some(vec![effect]),
            reference_imports,
            trigger_spec: None,
        }));
    }
    let loyalty_shorthand_cost = parse_loyalty_shorthand_activation_cost(cost_tokens);
    let mut effect_sentences = grammar::split_lexed_slices_on_period(effect_tokens);
    let functional_zones = infer_activated_functional_zones_lexed(cost_tokens, &effect_sentences);
    let mut timing = ActivationTiming::AnyTime;
    let scanned_modifiers = collect_activated_sentence_modifiers(&effect_sentences, timing);
    let mana_activation_condition = scanned_modifiers.mana_activation_condition;
    let mut additional_activation_restrictions =
        scanned_modifiers.additional_activation_restrictions;
    if ability_label.as_deref() == Some("Exhaust")
        && !scanned_modifiers.has_exhaust_once_restriction
    {
        additional_activation_restrictions
            .push("Activate each exhaust ability only once.".to_string());
    }
    let mana_usage_restrictions = scanned_modifiers.mana_usage_restrictions;
    let inline_effects_ast = scanned_modifiers.inline_effects_ast;
    effect_sentences = scanned_modifiers.kept_sentences;
    timing = scanned_modifiers.timing;
    let mana_activation_condition =
        combine_mana_activation_condition(mana_activation_condition, timing);
    if !effect_sentences.is_empty() {
        let primary_sentence = &effect_sentences[0];
        let x_defined_by_cost = activation_cost_mentions_x(cost_tokens);
        if let Some(primary_mana) =
            activated_line_grammar::parse_primary_mana_clause_tokens(primary_sentence)
        {
            let mana_cost = if let Some(cost) = &loyalty_shorthand_cost {
                cost.clone()
            } else {
                parse_compiler_activation_cost(cost_tokens)?
            };
            let reference_imports =
                super::super::util::compiler_activation_cost_reference_imports(&mana_cost);

            let mut extra_effects_ast = inline_effects_ast.clone();
            if effect_sentences.len() > 1 {
                for sentence in &effect_sentences[1..] {
                    if sentence.is_empty() {
                        continue;
                    }
                    let ast = parse_effect_sentence_lexed(sentence)?;
                    extra_effects_ast.extend(ast);
                }
            }

            let mana_tokens = primary_mana.mana_tokens;
            let mana_subject = primary_mana.subject_tokens.map(parse_subject);
            let dynamic_amount = if primary_mana.has_for_each {
                Some(
                    parse_dynamic_cost_modifier_value(mana_tokens)?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported dynamic mana amount (clause: '{}')",
                            joined_activation_clause_text(primary_sentence)
                        ))
                    })?,
                )
            } else {
                parse_devotion_value_from_add_clause(mana_tokens)?
                    .or_else(|| parse_add_mana_equal_amount_value(mana_tokens))
            };

            let loyalty_timing = if loyalty_shorthand_cost.is_some() {
                ActivationTiming::SorcerySpeed
            } else {
                timing
            };
            let loyalty_restrictions =
                loyalty_additional_restrictions(loyalty_shorthand_cost.is_some());
            let is_loyalty_ability = loyalty_shorthand_cost.is_some();
            let build_additional_restrictions = || {
                let mut restrictions = loyalty_restrictions.clone();
                restrictions.extend(additional_activation_restrictions.clone());
                restrictions
            };
            if primary_mana.requires_general_effect {
                let mut mana_ast = parse_add_mana(mana_tokens, mana_subject)?;
                resolve_activated_mana_x_requirements(
                    &mut mana_ast,
                    primary_sentence,
                    x_defined_by_cost,
                )?;
                let ability = Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects: ironsmith_core::ResolutionProgram::default(),
                        choices: vec![],
                        timing: loyalty_timing,
                        is_loyalty_ability,
                        additional_restrictions: build_additional_restrictions(),
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: mana_activation_condition.clone(),
                        mana_usage_restrictions: mana_usage_restrictions.clone(),
                    }),
                    functional_zones: functional_zones.clone(),
                };
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability: ability.into(),
                    text: ability_display_text.clone(),
                    effects_ast: Some(effects_ast),
                    reference_imports: reference_imports.clone(),
                    trigger_spec: None,
                }));
            }

            if let Some(mana) = parse_leaf_fixed_mana_output_tokens(mana_tokens) {
                if dynamic_amount.is_none()
                    && extra_effects_ast.is_empty()
                    && subject_allows_direct_mana_output(&mana_subject)
                {
                    let ability = Ability {
                        kind: AbilityKind::Activated(ActivatedAbility {
                            mana_cost,
                            effects: ironsmith_core::ResolutionProgram::default(),
                            choices: vec![],
                            timing: loyalty_timing,
                            is_loyalty_ability,
                            additional_restrictions: build_additional_restrictions(),
                            activation_restrictions: vec![],
                            mana_output: Some(mana),
                            activation_condition: mana_activation_condition.clone(),
                            mana_usage_restrictions: mana_usage_restrictions.clone(),
                        }),
                        functional_zones: functional_zones.clone(),
                    };
                    return Ok(Some(ParsedAbility {
                        ability: ability.into(),
                        text: ability_display_text.clone(),
                        effects_ast: None,
                        reference_imports: ReferenceImports::default(),
                        trigger_spec: None,
                    }));
                }
                let mut mana_ast = parse_add_mana(mana_tokens, mana_subject)?;
                resolve_activated_mana_x_requirements(
                    &mut mana_ast,
                    primary_sentence,
                    x_defined_by_cost,
                )?;
                let ability = Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects: ironsmith_core::ResolutionProgram::default(),
                        choices: vec![],
                        timing: loyalty_timing,
                        is_loyalty_ability,
                        additional_restrictions: build_additional_restrictions(),
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: mana_activation_condition.clone(),
                        mana_usage_restrictions: mana_usage_restrictions.clone(),
                    }),
                    functional_zones: functional_zones.clone(),
                };
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability: ability.into(),
                    text: ability_display_text.clone(),
                    effects_ast: Some(effects_ast),
                    reference_imports,
                    trigger_spec: None,
                }));
            }
        }
    }

    // Generic activated ability: parse costs and effects from "<costs>: <effects>"
    let mana_cost = if let Some(cost) = &loyalty_shorthand_cost {
        cost.clone()
    } else {
        parse_compiler_activation_cost(cost_tokens)?
    };
    let effect_tokens_joined = join_sentences_with_period(
        &effect_sentences
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>(),
    );
    if effect_sentences.is_empty()
        && !additional_activation_restrictions.is_empty()
        && inline_effects_ast.is_empty()
    {
        return Ok(Some(ParsedAbility {
            ability: {
                Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects: ironsmith_core::ResolutionProgram::default(),
                        choices: vec![],
                        timing,
                        is_loyalty_ability: loyalty_shorthand_cost.is_some(),
                        additional_restrictions: additional_activation_restrictions,
                        activation_restrictions: vec![],
                        mana_output: None,
                        activation_condition: mana_activation_condition.clone(),
                        mana_usage_restrictions,
                    }),
                    functional_zones,
                }
            }
            .into(),
            text: ability_display_text.clone(),
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        }));
    }
    let mut effects_ast = parse_effect_sentences_lexed(&effect_tokens_joined)?;
    effects_ast.extend(inline_effects_ast);
    let counter_result_comes_from_cost = activation_cost_removes_dynamic_counters(&mana_cost);
    for effect in &mut effects_ast {
        replace_removed_counter_metric_with_x(effect);
        if counter_result_comes_from_cost {
            replace_counter_removed_pump_with_x(effect);
        }
    }
    if effects_ast.is_empty() {
        return Ok(None);
    }
    let reference_imports =
        super::super::util::compiler_activation_cost_reference_imports(&mana_cost);
    if loyalty_shorthand_cost.is_some() {
        timing = ActivationTiming::SorcerySpeed;
        for restriction in loyalty_additional_restrictions(true) {
            let already_present = additional_activation_restrictions.iter().any(|existing| {
                let existing_lower = existing.to_ascii_lowercase();
                let restriction_lower = restriction.to_ascii_lowercase();
                existing.eq_ignore_ascii_case(restriction.as_str())
                    || (existing_lower.matches("once each turn").next().is_some()
                        && restriction_lower.matches("once each turn").next().is_some())
            });
            if !already_present {
                additional_activation_restrictions.push(restriction);
            }
        }
    }

    Ok(Some(ParsedAbility {
        ability: {
            Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost,
                    effects: ironsmith_core::ResolutionProgram::default(),
                    choices: vec![],
                    timing,
                    is_loyalty_ability: loyalty_shorthand_cost.is_some(),
                    additional_restrictions: additional_activation_restrictions,
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: mana_activation_condition.clone(),
                    mana_usage_restrictions,
                }),
                functional_zones,
            }
        }
        .into(),
        text: ability_display_text,
        effects_ast: Some(effects_ast),
        reference_imports,
        trigger_spec: None,
    }))
}

fn prefixed_activated_ability_display_text(
    ability_label: Option<&str>,
    cost_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
) -> Option<String> {
    ability_label.map(|label| {
        format!(
            "{label} — {}: {}",
            render_token_slice(cost_tokens).trim(),
            render_token_slice(effect_tokens).trim()
        )
    })
}

pub fn activation_cost_mentions_x(tokens: &[OwnedLexToken]) -> bool {
    activated_line_grammar::parse_activation_cost_x_fact_tokens(tokens).is_some()
}

pub fn resolve_activated_mana_x_requirements(
    effect: &mut EffectAst,
    sentence_tokens: &[OwnedLexToken],
    x_defined_by_cost: bool,
) -> Result<(), CardTextError> {
    let clause_words = ActivationRestrictionCompatWords::new(sentence_tokens).to_word_refs();
    let clause = clause_words.join(" ");
    let x_clause = activated_line_grammar::parse_activated_mana_x_clause_tokens(sentence_tokens);
    if let Some(where_tokens) = x_clause.where_clause_tokens {
        let where_value = parse_value_binding_clause_lexed(where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause in mana ability (clause: '{clause}')"
            ))
        })?;
        replace_unbound_x_in_effect_anywhere(effect, &where_value, &clause)?;
    }

    // A phrase such as "for each counter removed this way" refers to the
    // preceding activation cost, not to an EffectAst producer. The cost
    // executor exposes that count as activation X, so bind the parser's
    // pending filtered metric before the normal reference pass sees it.
    replace_removed_counter_metric_with_x(effect);

    if mana_effect_contains_unbound_x(effect)
        && !x_defined_by_cost
        && !x_clause.removed_counters_this_way
    {
        return Err(CardTextError::ParseError(format!(
            "unresolved X in mana ability without an X activation cost or where-x definition (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(())
}

fn replace_removed_counter_metric_with_x(effect: &mut EffectAst) {
    fn replace_value(value: &mut Value) {
        let hints = value.surface_hints().to_vec();
        if matches!(
            value.unhinted(),
            Value::PendingPriorEffectMetric(query)
                if query.action == Some(ironsmith_core::PriorEffectAction::Removed)
        ) {
            *value = Value::X.with_surface_hints(hints);
            return;
        }
        match value {
            Value::Add(left, right) | Value::Min(left, right) => {
                replace_value(left);
                replace_value(right);
            }
            Value::Scaled(inner, _)
            | Value::DividedRoundedDown(inner, _)
            | Value::HalfRoundedDown(inner)
            | Value::SurfaceHinted { value: inner, .. } => replace_value(inner),
            _ => {}
        }
    }

    if let EffectAst::SubjectVerb(subject_verb) = effect {
        match &mut subject_verb.action {
            SubjectVerbActionAst::AddManaScaled { amount, .. }
            | SubjectVerbActionAst::AddManaAnyColor { amount, .. }
            | SubjectVerbActionAst::AddManaAnyOneColor { amount }
            | SubjectVerbActionAst::AddManaChosenColor { amount, .. }
            | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount, .. }
            | SubjectVerbActionAst::AddManaCommanderIdentity { amount } => replace_value(amount),
            _ => {}
        }
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for nested_effect in nested {
            replace_removed_counter_metric_with_x(nested_effect);
        }
    });
}

fn activation_cost_removes_dynamic_counters(
    cost: &ironsmith_core::TotalCost<crate::model::CompilerCost>,
) -> bool {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => costs.iter().any(|cost| {
            matches!(
                cost,
                crate::model::CompilerCost::RemoveCounters {
                    display_x: true,
                    ..
                }
            )
        }),
        ironsmith_core::TotalCostKind::OneOf(branches) => branches
            .iter()
            .any(activation_cost_removes_dynamic_counters),
    }
}

fn replace_counter_removed_pump_with_x(effect: &mut EffectAst) {
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::PumpByLastEffect {
            power,
            toughness,
            target,
            duration,
            includes_this_way,
        } = &subject_verb.action
    {
        let basis = Value::X.with_surface_hint(if *includes_this_way {
            ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay
        } else {
            ironsmith_core::ValueSurfaceHint::CountersRemoved
        });
        let scale = |multiplier: i32| match multiplier {
            0 => Value::Fixed(0),
            1 => basis.clone(),
            _ => Value::Scaled(Box::new(basis.clone()), multiplier),
        };
        *effect = EffectAst::subject_verb_pump(
            scale(*power),
            scale(*toughness),
            target.clone(),
            duration.clone(),
            None,
        );
        return;
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for nested_effect in nested {
            replace_counter_removed_pump_with_x(nested_effect);
        }
    });
}

pub fn mana_effect_contains_unbound_x(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::AddManaScaled { amount, .. }
            | SubjectVerbActionAst::AddManaAnyColor { amount, .. }
            | SubjectVerbActionAst::AddManaAnyOneColor { amount }
            | SubjectVerbActionAst::AddManaChosenColor { amount, .. }
            | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount, .. }
            | SubjectVerbActionAst::AddManaCommanderIdentity { amount } => {
                value_contains_unbound_x(amount)
            }
            SubjectVerbActionAst::AddManaColorsAmong { .. }
            | SubjectVerbActionAst::AddOneManaAnyColorAmong { .. } => false,
            _ => false,
        },
        _ => {
            let mut contains_unbound_x = false;
            for_each_nested_effects(effect, true, |nested| {
                if nested.iter().any(mana_effect_contains_unbound_x) {
                    contains_unbound_x = true;
                }
            });
            contains_unbound_x
        }
    }
}

pub fn parse_loyalty_shorthand_activation_cost(
    cost_tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::TotalCost<crate::model::CompilerCost>> {
    match activated_line_grammar::parse_loyalty_shorthand_activation_tokens(cost_tokens)? {
        ActivatedLoyaltyShorthand::Add(0) => {
            Some(ironsmith_core::TotalCost::from_costs(Vec::new()))
        }
        ActivatedLoyaltyShorthand::Add(amount) => {
            Some(ironsmith_core::TotalCost::from_costs(vec![
                crate::model::CompilerCost::PutCounters {
                    counter_type: CounterType::Loyalty,
                    count: amount,
                    filter: None,
                },
            ]))
        }
        ActivatedLoyaltyShorthand::RemoveX => Some(ironsmith_core::TotalCost::from_costs(vec![
            crate::model::CompilerCost::RemoveCounters {
                counter_type: Some(CounterType::Loyalty),
                count: 0,
                filter: None,
                display_x: true,
                dynamic: true,
                single_object: true,
                remove_all: false,
            },
        ])),
        ActivatedLoyaltyShorthand::Remove(amount) => {
            Some(ironsmith_core::TotalCost::from_costs(vec![
                crate::model::CompilerCost::RemoveCounters {
                    counter_type: Some(CounterType::Loyalty),
                    count: amount,
                    filter: None,
                    display_x: false,
                    dynamic: false,
                    single_object: true,
                    remove_all: false,
                },
            ]))
        }
    }
}

pub fn loyalty_additional_restrictions(is_loyalty_shorthand: bool) -> Vec<String> {
    if !is_loyalty_shorthand {
        return Vec::new();
    }
    vec!["Activate only once each turn.".to_string()]
}

pub fn infer_activated_functional_zones_lexed(
    cost_tokens: &[OwnedLexToken],
    effect_sentences: &[&[OwnedLexToken]],
) -> Vec<Zone> {
    crate::grammar::functional_zones::parse_activated_functional_zones_tokens(
        cost_tokens,
        effect_sentences,
    )
}

pub fn parse_activate_only_timing_lexed(tokens: &[OwnedLexToken]) -> Option<ActivationTiming> {
    activated_sentence_parsers::parse_activate_only_timing_lexed(tokens)
}

pub fn flatten_mana_activation_conditions(
    condition: &crate::ConditionExpr,
    out: &mut Vec<crate::ConditionExpr>,
) {
    match condition {
        crate::ConditionExpr::And(left, right) => {
            flatten_mana_activation_conditions(left, out);
            flatten_mana_activation_conditions(right, out);
        }
        _ => out.push(condition.clone()),
    }
}

pub fn rebuild_mana_activation_conditions(
    conditions: Vec<crate::ConditionExpr>,
) -> Option<crate::ConditionExpr> {
    let mut iter = conditions.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, next| {
        crate::ConditionExpr::And(Box::new(acc), Box::new(next))
    }))
}

pub fn combine_mana_activation_condition(
    base: Option<crate::ConditionExpr>,
    timing: ActivationTiming,
) -> Option<crate::ConditionExpr> {
    if timing == ActivationTiming::AnyTime {
        return base;
    }
    merge_mana_activation_conditions(base, crate::ConditionExpr::ActivationTiming(timing))
}

pub fn merge_mana_activation_conditions(
    base: Option<crate::ConditionExpr>,
    condition: crate::ConditionExpr,
) -> Option<crate::ConditionExpr> {
    let mut conditions: Vec<crate::ConditionExpr> = Vec::new();
    if let Some(base) = base {
        flatten_mana_activation_conditions(&base, &mut conditions);
    }
    if !conditions.contains(&condition) {
        conditions.push(condition);
    }
    rebuild_mana_activation_conditions(conditions)
}

pub fn is_activate_only_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_activate_only_restriction_sentence_lexed(tokens)
}

pub fn is_activate_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_activate_only_restriction_sentence_lexed(tokens)
}

pub fn parse_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::model::compiler_semantic::CompilerManaUsageRestriction> {
    activated_sentence_parsers::parse_mana_usage_restriction_sentence_lexed(tokens)
}

pub fn is_any_player_may_activate_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_any_player_may_activate_sentence_lexed(tokens)
}

pub fn is_trigger_only_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_trigger_only_restriction_sentence_lexed(tokens)
}

pub fn is_trigger_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_trigger_only_restriction_sentence_lexed(tokens)
}

pub fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    activated_sentence_parsers::parse_triggered_times_each_turn_lexed(tokens)
}

pub fn parse_named_number(word: &str) -> Option<u32> {
    parse_cardinal_u32(word)
}

pub fn parse_activation_cost(
    tokens: &[OwnedLexToken],
) -> Result<ironsmith_core::TotalCost<crate::model::CompilerCost>, CardTextError> {
    parse_compiler_activation_cost(tokens)
}

pub fn parse_compiler_activation_cost(
    tokens: &[OwnedLexToken],
) -> Result<ironsmith_core::TotalCost<crate::model::CompilerCost>, CardTextError> {
    if let Some(cost) =
        super::keyword_action_costs::parse_single_graveyard_bottom_library_compiler_payment(tokens)
    {
        return Ok(cost);
    }
    let cst = parse_activation_cost_tokens(tokens)?;
    Ok(crate::semantic_assembly::assemble_activation_cost(&cst)?.to_core_total_cost())
}

pub fn parse_devotion_value_from_add_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    activated_line_grammar::parse_activated_devotion_value_tokens(tokens).map_err(|error| {
        let detail = match error {
            ActivatedDevotionParseError::UnsupportedPlayer => {
                "unsupported devotion player in clause".to_string()
            }
            ActivatedDevotionParseError::MissingColorAfterDevotion => {
                "missing color after devotion clause".to_string()
            }
            ActivatedDevotionParseError::MissingColor => "missing devotion color".to_string(),
            ActivatedDevotionParseError::UnsupportedColor(color) => {
                format!("unsupported devotion color '{color}'")
            }
        };
        CardTextError::ParseError(format!("{detail} (clause: '{}')", words.join(" ")))
    })
}

pub fn color_from_color_set(colors: ColorSet) -> Option<crate::color::Color> {
    let mut found = None;
    for color in [
        crate::color::Color::White,
        crate::color::Color::Blue,
        crate::color::Color::Black,
        crate::color::Color::Red,
        crate::color::Color::Green,
    ] {
        if colors.intersection(ColorSet::from_color(color)).count() > 0 {
            if found.is_some() {
                return None;
            }
            found = Some(color);
        }
    }
    found
}

#[cfg(test)]
pub fn parse_activation_condition_lexed(tokens: &[OwnedLexToken]) -> Option<crate::ConditionExpr> {
    activated_sentence_parsers::parse_activation_condition_lexed(tokens)
}

pub fn parse_cardinal_u32(word: &str) -> Option<u32> {
    let token = OwnedLexToken::word(word.to_string(), TextSpan::synthetic());
    parse_number(&[token]).map(|(value, _)| value)
}

pub fn parse_enters_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause = joined_activation_clause_text(tokens);
    match activated_line_grammar::parse_enters_tapped_line_shape(tokens) {
        EntersTappedLineShape::NoMatch
        | EntersTappedLineShape::NegatedUntap
        | EntersTappedLineShape::AttackingVariant => Ok(None),
        EntersTappedLineShape::EntersTapped => Ok(Some(StaticAbility::enters_tapped_ability())),
        EntersTappedLineShape::MixedNegatedUntap => Err(CardTextError::ParseError(format!(
            "unsupported mixed enters-tapped and negated-untap clause (clause: '{clause}')"
        ))),
        EntersTappedLineShape::UnsupportedTrailing => Err(CardTextError::ParseError(format!(
            "unsupported trailing enters-tapped clause (clause: '{clause}')"
        ))),
    }
}

pub fn parse_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::lexer::token_word_refs(tokens);
    let Some(head) = activated_line_grammar::parse_cost_reduction_line_head_tokens(tokens) else {
        return Ok(None);
    };

    match head {
        CostReductionLineHead::ThisCost {
            amount_tokens,
            diagnostic_amount_word,
            diagnostic_tail,
        } => {
            let parsed_amount = parse_cost_modifier_amount(amount_tokens);
            let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
            let amount_fixed = if let Value::Fixed(value) = amount_value {
                value
            } else {
                1
            };
            let remaining_tokens = amount_tokens.get(used..).unwrap_or_default();
            if activated_line_grammar::parse_this_cost_reduction_remainder_tokens(remaining_tokens)
                == ThisCostReductionRemainder::ForEach
                && let Some(dynamic) = parse_dynamic_cost_modifier_value(remaining_tokens)?
            {
                let reduction = scale_dynamic_cost_modifier_value(dynamic, amount_fixed);
                return Ok(Some(StaticAbility::new(
                    crate::static_abilities::ThisSpellCostReduction::new(
                        reduction,
                        crate::static_abilities::ThisSpellCostCondition::Always,
                    ),
                )));
            }

            let amount_text = if diagnostic_amount_word.chars().all(|ch| ch.is_ascii_digit()) {
                format!("{{{diagnostic_amount_word}}}")
            } else {
                diagnostic_amount_word.to_string()
            };
            let text = format!("This cost is reduced by {amount_text} {diagnostic_tail}");
            Err(CardTextError::ParseError(format!(
                "unsupported cost-reduction static clause (clause: '{text}')"
            )))
        }
        CostReductionLineHead::ActivatedAbilitiesOf {
            subject_tokens,
            amount_tokens,
        } => {
            let mut filter = parse_object_filter(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported activated-ability cost reduction subject (clause: '{}')",
                    line_words.join(" ")
                ))
            })?;
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }

            let Some((amount_value, used)) = parse_cost_modifier_amount(amount_tokens) else {
                return Ok(None);
            };
            let reduction = match amount_value {
                Value::Fixed(value) if value > 0 => value as u32,
                _ => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported activated-ability cost reduction amount (clause: '{}')",
                        line_words.join(" ")
                    )));
                }
            };
            let Some(remainder) =
                activated_line_grammar::parse_activated_abilities_reduction_remainder_tokens(
                    &amount_tokens[used..],
                )
            else {
                return Ok(None);
            };
            let (minimum_total_mana, uses_ability_activation_cost_surface) = match remainder {
                ActivatedAbilitiesReductionRemainder::Unbounded => (None, false),
                ActivatedAbilitiesReductionRemainder::MinimumOneMana => (Some(1), false),
                ActivatedAbilitiesReductionRemainder::MinimumOneManaAbilityActivationCost => {
                    (Some(1), true)
                }
            };
            let subject = render_token_slice(subject_tokens);
            let mut display =
                format!("Activated abilities of {subject} cost {{{reduction}}} less to activate");
            if uses_ability_activation_cost_surface {
                display.push_str(
                    ". This effect can't reduce the mana in that ability's activation cost to less than one mana",
                );
            }
            Ok(Some(
                StaticAbility::reduce_activated_ability_costs_with_display(
                    filter,
                    reduction,
                    minimum_total_mana,
                    display,
                ),
            ))
        }
        CostReductionLineHead::ThisAbility { amount_tokens } => {
            let Some((amount_value, used)) = parse_cost_modifier_amount(amount_tokens) else {
                return Ok(None);
            };
            let reduction = match amount_value {
                Value::Fixed(value) if value > 0 => value as u32,
                _ => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported activated-ability cost reduction amount (clause: '{}')",
                        line_words.join(" ")
                    )));
                }
            };
            let tail_tokens = trim_commas(&amount_tokens[used..]);
            match activated_line_grammar::parse_this_ability_reduction_remainder_tokens(
                &tail_tokens,
            ) {
                ThisAbilityReductionRemainder::Unconditional => {
                    Ok(Some(StaticAbility::reduce_activated_ability_costs(
                        ObjectFilter::source(),
                        reduction,
                        None,
                    )))
                }
                ThisAbilityReductionRemainder::Targets {
                    count_and_filter_tokens,
                } => {
                    let (count, used) = parse_number(count_and_filter_tokens).ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported activated-ability target condition count (clause: '{}')",
                            line_words.join(" ")
                        ))
                    })?;
                    let mut filter = parse_object_filter(&count_and_filter_tokens[used..], false)
                        .map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported activated-ability target condition filter (clause: '{}')",
                            line_words.join(" ")
                        ))
                    })?;
                    if filter.zone.is_none() {
                        filter.zone = Some(Zone::Battlefield);
                    }
                    Ok(Some(
                        StaticAbility::reduce_activated_ability_costs_if_targets(
                            ObjectFilter::source(),
                            reduction,
                            crate::static_abilities::ActivatedAbilityCostCondition::TargetsExactly {
                                count: count as usize,
                                filter,
                            },
                            None,
                        ),
                    ))
                }
                ThisAbilityReductionRemainder::ForEach { filter_tokens } => {
                    if let Some(Value::BasicLandTypesAmong(lands_filter)) =
                        parse_dynamic_cost_modifier_value(&tail_tokens)?
                    {
                        return Ok(Some(
                            StaticAbility::reduce_activated_ability_costs_for_each_basic_land_type(
                                ObjectFilter::source(),
                                reduction,
                                lands_filter,
                                None,
                            ),
                        ));
                    }
                    let mut per_filter =
                        parse_object_filter(filter_tokens, false).map_err(|_| {
                            CardTextError::ParseError(format!(
                                "unsupported activated-ability cost reduction tail (clause: '{}')",
                                line_words.join(" ")
                            ))
                        })?;
                    if per_filter.zone.is_none() {
                        per_filter.zone = Some(Zone::Battlefield);
                    }
                    Ok(Some(
                        StaticAbility::reduce_activated_ability_costs_for_each(
                            ObjectFilter::source(),
                            reduction,
                            per_filter,
                            None,
                        ),
                    ))
                }
                ThisAbilityReductionRemainder::UnsupportedCondition => {
                    Err(CardTextError::ParseError(format!(
                        "unsupported activated-ability cost reduction condition (clause: '{}')",
                        line_words.join(" ")
                    )))
                }
                ThisAbilityReductionRemainder::NotReduction => Ok(None),
            }
        }
        CostReductionLineHead::ThisSpell { amount_tokens } => {
            let parsed_amount = parse_cost_modifier_amount(amount_tokens);
            let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
            let amount_fixed = if let Value::Fixed(value) = amount_value {
                value
            } else {
                1
            };
            let remaining_tokens = &amount_tokens[used..];
            let remainder = activated_line_grammar::parse_this_spell_reduction_remainder_tokens(
                remaining_tokens,
            );
            if remainder == ThisSpellReductionRemainder::NotReduction {
                return Ok(None);
            }
            if let Some(dynamic) = parse_dynamic_cost_modifier_value(remaining_tokens)? {
                let reduction =
                    crate::static_abilities::CostReduction::new(ObjectFilter::default(), dynamic);
                return Ok(Some(StaticAbility::new(reduction)));
            }
            if parsed_amount.is_none() {
                return Ok(None);
            }
            if remainder == ThisSpellReductionRemainder::CardTypesInGraveyard {
                if amount_fixed != 1 {
                    return Ok(None);
                }
                let reduction = crate::effect::Value::CardTypesInGraveyard(PlayerFilter::You);
                let cost_reduction =
                    crate::static_abilities::CostReduction::new(ObjectFilter::default(), reduction);
                return Ok(Some(StaticAbility::new(cost_reduction)));
            }
            Ok(None)
        }
    }
}

pub fn scale_dynamic_cost_modifier_value(dynamic: Value, multiplier: i32) -> Value {
    if multiplier <= 0 {
        return Value::Fixed(0);
    }
    if multiplier == 1 {
        return dynamic;
    }
    match dynamic {
        Value::Count(filter) => Value::CountScaled(filter, multiplier),
        Value::CountScaled(filter, factor) => Value::CountScaled(filter, factor * multiplier),
        other => {
            let mut scaled = other.clone();
            for _ in 1..multiplier {
                scaled = Value::Add(Box::new(scaled), Box::new(other.clone()));
            }
            scaled
        }
    }
}

pub fn parse_all_creatures_able_to_block_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    if activated_line_grammar::parse_activated_block_requirement_words(&words)
        == Some(ActivatedBlockRequirement::AllCreaturesBlockSource)
    {
        return Ok(Some(StaticAbilityAst::Static(StaticAbility::restriction(
            crate::effect::Restriction::must_block_specific_attacker(
                ObjectFilter::creature(),
                ObjectFilter::source(),
            ),
            "All creatures able to block this creature do so".to_string(),
        ))));
    }
    Ok(None)
}

pub fn parse_source_must_be_blocked_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    if activated_line_grammar::parse_activated_block_requirement_words(&words)
        == Some(ActivatedBlockRequirement::SourceMustBeBlocked)
    {
        return Ok(Some(StaticAbility::restriction(
            crate::effect::Restriction::must_be_blocked(ObjectFilter::source()),
            "this creature must be blocked if able".to_string(),
        )));
    }
    Ok(None)
}
