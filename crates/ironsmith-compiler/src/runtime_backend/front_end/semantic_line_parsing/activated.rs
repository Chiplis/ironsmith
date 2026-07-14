use super::super::effect_ast_traversal::for_each_nested_effects_mut;
use super::super::grammar::activated_lowering as activated_grammar;
use super::super::grammar::activated_lowering::{
    ActivatedManaEffectKind, ActivatedRestrictionSentenceKind, ActivatedXDefinitionIntro,
};
use super::super::grammar::effects::parse_fixed_mana_output_clause_spec_lexed;
use super::super::grammar::restriction_facts::{
    parse_activation_restriction_surface_tokens, parse_mana_restriction_surface_tokens,
    parse_mana_restriction_tokens,
};
use super::super::ir::RewriteActivatedLine;
use super::*;
use crate::effect::Effect;
use crate::object::CounterType;
use crate::runtime_backend::semantic::ParsedManaRestriction;
use crate::runtime_backend::util::activation_cost_reference_imports;
use ironsmith_core::TotalCostKind;

fn activated_effect_may_be_mana_ability_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_grammar::parse_activated_mana_effect_kind(tokens).is_some()
}

fn activated_effect_is_for_each_color_among_add_mana_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_grammar::parse_activated_mana_effect_kind(tokens)
        == Some(ActivatedManaEffectKind::ColorsAmong)
}

fn activation_cost_defines_x_for_mana_ability(cost: &TotalCost) -> bool {
    if cost.mana_cost().is_some_and(crate::mana::ManaCost::has_x) {
        return true;
    }

    fn value_uses_x(value: &crate::effect::Value) -> bool {
        use crate::effect::Value;

        match value {
            Value::X | Value::XTimes(_) => true,
            Value::Scaled(inner, _)
            | Value::DividedRoundedDown(inner, _)
            | Value::HalfRoundedDown(inner) => value_uses_x(inner),
            Value::Add(left, right) => value_uses_x(left) || value_uses_x(right),
            _ => false,
        }
    }

    cost.costs().iter().any(|component| match component {
        Cost::Mana(cost) => cost.has_x(),
        Cost::Energy(amount) | Cost::Life(amount) | Cost::Mill(amount) => value_uses_x(amount),
        Cost::RemoveAnyCountersFromSource { .. } => true,
        Cost::Effect(effect) => {
            effect
                .downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
                .is_some_and(|effect| effect.display_x)
                || effect
                    .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                    .is_some_and(|effect| effect.count.is_dynamic_x())
                || effect
                    .downcast_ref::<crate::effects::SacrificeEffect>()
                    .is_some_and(|_| false)
                || effect
                    .downcast_ref::<crate::effects::DiscardEffect>()
                    .is_some_and(|_| false)
                || effect
                    .downcast_ref::<crate::effects::MillEffect>()
                    .is_some_and(|_| false)
                || effect
                    .downcast_ref::<crate::effects::PayEnergyEffect>()
                    .is_some_and(|effect| value_uses_x(&effect.amount))
                || effect
                    .downcast_ref::<crate::effects::RemoveCountersEffect>()
                    .is_some_and(|effect| value_uses_x(&effect.count))
        }
        _ => false,
    })
}

fn activation_cost_sets_x_from_counter_removal(cost: &TotalCost) -> bool {
    fn component_sets_x(component: &Cost) -> bool {
        match component {
            Cost::RemoveCounters { .. } | Cost::RemoveAnyCountersFromSource { .. } => true,
            Cost::Effect(effect) => {
                effect
                    .downcast_ref::<crate::effects::RemoveCountersEffect>()
                    .is_some_and(|effect| {
                        matches!(effect.target.base(), crate::target::ChooseSpec::Source)
                    })
                    || effect
                        .downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
                        .is_some()
                    || effect
                        .downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
                        .is_some()
            }
            _ => false,
        }
    }

    match cost.kind() {
        TotalCostKind::All(components) => components.iter().any(component_sets_x),
        TotalCostKind::OneOf(branches) => branches
            .iter()
            .any(activation_cost_sets_x_from_counter_removal),
    }
}

fn bind_event_amount_to_cost_x(value: &mut crate::effect::Value) {
    use crate::effect::{EventValueSpec, Value};

    match value {
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) => {
            *value = Value::X;
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) => {
            *value = Value::Add(Box::new(Value::X), Box::new(Value::Fixed(*offset)));
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            bind_event_amount_to_cost_x(left);
            bind_event_amount_to_cost_x(right);
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner) => {
            bind_event_amount_to_cost_x(inner);
        }
        Value::SurfaceHinted { value, .. } => bind_event_amount_to_cost_x(value),
        _ => {}
    }
}

fn bind_event_amounts_to_cost_x_in_effect(effect: &mut EffectAst) {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::DealDamage { amount, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { amount, .. }
            | SubjectVerbActionAst::DealDistributedDamage { amount, .. }
            | SubjectVerbActionAst::DealDamageEach { amount, .. }
            | SubjectVerbActionAst::Mill { count: amount }
            | SubjectVerbActionAst::Draw { count: amount }
            | SubjectVerbActionAst::AddManaScaled { amount, .. }
            | SubjectVerbActionAst::AddManaAnyColor { amount, .. }
            | SubjectVerbActionAst::AddManaAnyOneColor { amount }
            | SubjectVerbActionAst::AddManaChosenColor { amount, .. }
            | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount, .. }
            | SubjectVerbActionAst::AddManaCommanderIdentity { amount } => {
                bind_event_amount_to_cost_x(amount);
            }
            _ => {}
        },
        _ => {}
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for inner in nested {
            bind_event_amounts_to_cost_x_in_effect(inner);
        }
    });
}

fn bind_event_amounts_to_cost_x(effects: &mut [EffectAst]) {
    for effect in effects {
        bind_event_amounts_to_cost_x_in_effect(effect);
    }
}

fn effect_ast_is_mana_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => matches!(
            &subject_verb.action,
            SubjectVerbActionAst::AddMana { .. }
                | SubjectVerbActionAst::AddManaScaled { .. }
                | SubjectVerbActionAst::AddManaAnyColor { .. }
                | SubjectVerbActionAst::AddManaAnyOneColor { .. }
                | SubjectVerbActionAst::AddManaChosenColor { .. }
                | SubjectVerbActionAst::AddManaFromLandCouldProduce { .. }
                | SubjectVerbActionAst::AddManaColorsAmong { .. }
                | SubjectVerbActionAst::AddManaCommanderIdentity { .. }
                | SubjectVerbActionAst::AddManaImprintedColors
        ),
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            (!if_true.is_empty() && if_true.iter().all(effect_ast_is_mana_effect))
                || (!if_false.is_empty() && if_false.iter().all(effect_ast_is_mana_effect))
        }
        _ => false,
    }
}

fn effects_ast_can_lower_as_mana_ability(effects: &[EffectAst]) -> bool {
    !effects.is_empty() && effects.iter().all(effect_ast_is_mana_effect)
}

struct SplitRewriteActivatedEffectText {
    effect_text: String,
    effect_parse_tokens: Vec<OwnedLexToken>,
    restrictions: ParsedRestrictions,
    mana_restrictions: Vec<ParsedManaRestriction>,
    x_cant_be_zero: bool,
}

fn parse_standalone_x_definition_value(tokens: &[OwnedLexToken]) -> Option<crate::effect::Value> {
    let shape = activated_grammar::parse_activated_x_definition_tokens(tokens)?;
    let value_tokens = match shape.intro {
        ActivatedXDefinitionIntro::WhereXIs => tokens.to_vec(),
        ActivatedXDefinitionIntro::XIs => {
            let mut synthetic = vec![
                OwnedLexToken::word("where".to_string(), TextSpan::synthetic()),
                OwnedLexToken::word("x".to_string(), TextSpan::synthetic()),
                OwnedLexToken::word("is".to_string(), TextSpan::synthetic()),
            ];
            synthetic.extend_from_slice(shape.value_tokens);
            synthetic
        }
    };

    parse_value_binding_clause(&value_tokens).or_else(|| {
        shape.exiled_card_mana_value.then(|| {
            crate::effect::Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                crate::tag::SOURCE_EXILED_TAG,
            ))))
        })
    })
}

fn is_standalone_x_definition_sentence(tokens: &[OwnedLexToken]) -> bool {
    parse_standalone_x_definition_value(tokens).is_some()
}

fn activated_x_definition_value(tokens: &[OwnedLexToken]) -> Option<crate::effect::Value> {
    split_lexed_sentences(tokens)
        .into_iter()
        .find_map(parse_standalone_x_definition_value)
        .or_else(|| {
            let shape = activated_grammar::find_activated_x_definition_tokens(tokens)?;
            let offset = tokens.len().checked_sub(shape.value_tokens.len() + 3)?;
            parse_standalone_x_definition_value(&tokens[offset..])
        })
}

fn bind_activated_x_definition_to_mana_cost(
    cost: TotalCost,
    x_value: Option<crate::effect::Value>,
) -> TotalCost {
    let Some(x_value) = x_value else {
        return cost;
    };

    cost.try_map(|component| {
        if let Some(mana_cost) = component.mana_cost_ref()
            && mana_cost.has_x()
        {
            Ok(Cost::dynamic_mana(ironsmith_core::DynamicManaCost::from_x(
                mana_cost.clone(),
                x_value.clone(),
            )))
        } else {
            Ok(component)
        }
    })
    .unwrap_or_else(|_: std::convert::Infallible| unreachable!())
}

fn finalize_rewrite_activated_effect_sentences(
    mut restrictions: ParsedRestrictions,
    sentence_tokens: Vec<Vec<OwnedLexToken>>,
) -> SplitRewriteActivatedEffectText {
    let mut effect_sentences = Vec::new();
    let mut effect_sentence_tokens = Vec::new();
    let mut mana_restrictions = Vec::new();
    let mut x_cant_be_zero = false;

    for tokens in sentence_tokens {
        let sentence = render_token_slice(&tokens).trim().to_string();
        let restriction_kind = activated_grammar::classify_activated_restriction_sentence(&tokens);
        if restriction_kind == Some(ActivatedRestrictionSentenceKind::ManaSource) {
            restrictions
                .activation
                .push(parse_activation_restriction_surface_tokens(&tokens));
        } else if let Some(parsed) = parse_mana_restriction_tokens(&tokens) {
            mana_restrictions.push(parsed);
        } else if matches!(
            restriction_kind,
            Some(ActivatedRestrictionSentenceKind::SpendThisManaOnly)
                | Some(ActivatedRestrictionSentenceKind::WhenSpendThisManaToCast)
        ) {
            mana_restrictions.push(parse_mana_restriction_surface_tokens(&tokens));
        } else if super::super::grammar::effects::dispatch_entry_shapes::is_x_cant_be_zero_tokens(
            &tokens,
        ) {
            x_cant_be_zero = true;
        } else if is_standalone_x_definition_sentence(&tokens) {
            continue;
        } else if is_any_player_may_activate_sentence_lexed(&tokens) {
            restrictions
                .activation
                .push(parse_activation_restriction_surface_tokens(&tokens));
        } else {
            effect_sentences.push(sentence);
            effect_sentence_tokens.push(tokens);
        }
    }

    SplitRewriteActivatedEffectText {
        effect_text: effect_sentences.join(". "),
        effect_parse_tokens: join_sentences_with_period(&effect_sentence_tokens),
        restrictions,
        mana_restrictions,
        x_cant_be_zero,
    }
}

fn split_rewrite_activated_effect_text(
    effect_parse_tokens: &[OwnedLexToken],
) -> SplitRewriteActivatedEffectText {
    let (sentence_tokens, restrictions) = split_tokens_for_parse(effect_parse_tokens);
    finalize_rewrite_activated_effect_sentences(restrictions, sentence_tokens)
}

fn parse_activated_effects_lexed(
    _effect_text: &str,
    tokens: &[OwnedLexToken],
    _line_index: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    if activated_effect_is_for_each_color_among_add_mana_lexed(tokens) {
        return Ok(vec![
            crate::runtime_backend::activation_helpers::parse_add_mana(tokens, None)?,
        ]);
    }
    if let Some(effects) = parse_each_player_and_their_creatures_damage_sentence(tokens) {
        return Ok(effects);
    }
    if let Ok(effects) = parse_effect_sentences_lexed(tokens) {
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

fn rewrite_self_replacements_as_conditionals(effect: EffectAst) -> EffectAst {
    match effect {
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => EffectAst::Conditional {
            predicate,
            if_true: if_true
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
            if_false: if_false
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
        },
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            ..
        } => EffectAst::Conditional {
            predicate,
            if_true: if_true
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
            if_false: if_false
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
        },
        other => other,
    }
}

fn normalize_mana_replacement_effects(effects: Vec<EffectAst>) -> Vec<EffectAst> {
    effects
        .into_iter()
        .map(|effect| match effect {
            EffectAst::SelfReplacement { .. } => effect,
            other => rewrite_self_replacements_as_conditionals(other),
        })
        .collect()
}

pub(crate) struct ParsedActivatedLine {
    pub(crate) chunk: LineAst,
    pub(crate) restrictions: ParsedRestrictions,
}

pub(crate) fn parse_activated_line(
    info: LineInfo,
    cost: TotalCost,
    cost_parse_tokens: Vec<OwnedLexToken>,
    effect_parse_tokens: Vec<OwnedLexToken>,
    timing_hint: ActivationTiming,
    is_loyalty_ability: bool,
    presentation: Option<PresentationLabel>,
    chosen_option: Option<ChosenOptionContext>,
) -> Result<ParsedActivatedLine, CardTextError> {
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
            cost,
            cost_parse_tokens: cost_parse_tokens.clone(),
            effect_parse_tokens: effect_parse_tokens.clone(),
            timing_hint,
            is_loyalty_ability,
            chosen_option,
        },
        &effect_parse_tokens,
    )
}

fn parse_activated_line_impl(
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

    let normalized_cost =
        bind_activated_x_definition_to_mana_cost(line.cost.clone(), x_definition_value);
    let original_effect_mentions_where_x =
        activated_grammar::contains_where_x_definition(original_effect_parse_tokens);
    let ability_text = rewrite_activated_display_text(line);
    let presentation_display = activated_presentation_display(line);
    let mut additional_activation_restrictions = if line.presentation_kind
        == Some(crate::runtime_backend::ir::ActivatedPresentationKind::Exhaust)
    {
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
                    effects: ResolutionProgram::from_effects(vec![Effect::put_counters_on_source(
                        CounterType::Level,
                        1,
                    )]),
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
                    effects: ResolutionProgram::default(),
                    choices: vec![],
                    timing: line.timing_hint.clone(),
                    is_loyalty_ability: line.is_loyalty_ability,
                    additional_restrictions: additional_activation_restrictions.clone(),
                    activation_restrictions: vec![],
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
            || effects_ast.first().is_some_and(effect_ast_is_mana_effect)
        {
            let functional_zones = infer_rewrite_activated_functional_zones(line)?;
            let reference_imports = activation_cost_reference_imports(&normalized_cost);
            let mut parsed = ParsedAbility {
                ability: Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost: normalized_cost.clone(),
                        effects: ResolutionProgram::default(),
                        choices: vec![],
                        timing: line.timing_hint.clone(),
                        is_loyalty_ability: line.is_loyalty_ability,
                        additional_restrictions: additional_activation_restrictions.clone(),
                        activation_restrictions: vec![],
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
    if activation_cost_sets_x_from_counter_removal(&normalized_cost) {
        bind_event_amounts_to_cost_x(&mut effects_ast);
    }
    let functional_zones = infer_rewrite_activated_functional_zones(line)?;
    let reference_imports = activation_cost_reference_imports(&normalized_cost);
    let mut parsed = ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: normalized_cost,
                effects: ResolutionProgram::default(),
                choices: vec![],
                timing: line.timing_hint.clone(),
                is_loyalty_ability: line.is_loyalty_ability,
                additional_restrictions: additional_activation_restrictions,
                activation_restrictions: vec![],
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

fn apply_chosen_option_condition_to_activated(
    parsed: &mut ParsedAbility,
    chosen_option: Option<&ChosenOptionContext>,
) {
    let Some(context) = chosen_option else {
        return;
    };
    let condition = condition_for_chosen_option(context);
    let AbilityKind::Activated(activated) = parsed.kind_mut() else {
        return;
    };
    activated.activation_condition = Some(match activated.activation_condition.take() {
        Some(existing) => crate::ConditionExpr::And(Box::new(existing), Box::new(condition)),
        None => condition,
    });
    if let Some(threshold) = context.station_threshold() {
        // Renderer-only surface metadata derived from the typed station fact;
        // no later stage parses Oracle text to recover this threshold.
        activated
            .additional_restrictions
            .push(format!("__ironsmith_station_threshold:{threshold}"));
    }
}

fn rewrite_activated_display_text(line: &RewriteActivatedLine) -> Option<String> {
    let display = activated_presentation_display(line)?;
    Some(format!(
        "{display} — {}: {}",
        render_token_slice(&line.cost_parse_tokens).trim(),
        render_token_slice(&line.effect_parse_tokens).trim()
    ))
}

fn activated_presentation_display(line: &RewriteActivatedLine) -> Option<String> {
    line.presentation
        .as_ref()
        .and_then(PresentationLabel::display_prefix)
        .or_else(|| {
            line.presentation_kind
                .map(|kind| kind.display().to_string())
        })
}

fn infer_rewrite_activated_functional_zones(
    line: &RewriteActivatedLine,
) -> Result<Vec<Zone>, CardTextError> {
    Ok(line.functional_zones.clone())
}
