use super::super::effect_ast_traversal::for_each_nested_effects_mut;
use super::super::ir::RewriteActivatedLine;
use super::*;
use ironsmith_core::TotalCostKind;

fn activated_effect_may_be_mana_ability_lexed(tokens: &[OwnedLexToken]) -> bool {
    let line_words = token_word_refs(tokens);
    word_refs_find(line_words.as_slice(), "add").is_some()
        && matches!(
            line_words.as_slice(),
            ["add", ..]
                | ["adds", ..]
                | ["you", "add", ..]
                | ["that", "player", "add", ..]
                | ["that", "player", "adds", ..]
                | ["target", "player", "add", ..]
                | ["target", "player", "adds", ..]
        )
}

fn activation_cost_defines_x_for_mana_ability(cost: &TotalCost) -> bool {
    if cost.mana_cost().is_some_and(crate::mana::ManaCost::has_x) {
        return true;
    }

    fn value_uses_x(value: &crate::effect::Value) -> bool {
        use crate::effect::Value;

        match value {
            Value::X | Value::XTimes(_) => true,
            Value::Scaled(inner, _) | Value::HalfRoundedDown(inner) => value_uses_x(inner),
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
        Value::Scaled(inner, _) | Value::HalfRoundedDown(inner) => {
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
            | SubjectVerbActionAst::DealDamageEach { amount, .. }
            | SubjectVerbActionAst::Mill { count: amount }
            | SubjectVerbActionAst::Draw { count: amount } => {
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

fn extract_fixed_mana_output_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<ManaSymbol>> {
    let Some(add_idx) = find_index(tokens, |token| {
        token.is_word("add") || token.is_word("adds")
    }) else {
        return None;
    };
    let prefix_words = token_word_refs(&tokens[..add_idx]);
    if !matches!(prefix_words.as_slice(), [] | ["you"]) {
        return None;
    }

    let mana: Vec<_> = tokens[add_idx + 1..]
        .iter()
        .try_fold(Vec::new(), |mut acc, token| match token.kind {
            TokenKind::ManaGroup => {
                let inner = token.slice.trim_start_matches('{').trim_end_matches('}');
                acc.push(parse_mana_symbol(inner).ok()?);
                Some(acc)
            }
            TokenKind::Period | TokenKind::Comma => Some(acc),
            _ => None,
        })?;

    if mana.is_empty() { None } else { Some(mana) }
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
    mana_restrictions: Vec<String>,
}

fn finalize_rewrite_activated_effect_sentences(
    mut restrictions: ParsedRestrictions,
    sentence_tokens: Vec<Vec<OwnedLexToken>>,
) -> SplitRewriteActivatedEffectText {
    let mut effect_sentences = Vec::new();
    let mut effect_sentence_tokens = Vec::new();
    let mut mana_restrictions = Vec::new();

    for tokens in sentence_tokens {
        let sentence = render_token_slice(&tokens).trim().to_string();
        let sentence_words = token_word_refs(&tokens);
        if parse_mana_usage_restriction_sentence_lexed(&tokens).is_some()
            || parse_mana_spend_bonus_sentence_lexed(&tokens).is_some()
            || word_refs_have_prefix(
                sentence_words.as_slice(),
                &["spend", "this", "mana", "only"],
            )
            || word_refs_have_prefix(
                sentence_words.as_slice(),
                &["when", "you", "spend", "this", "mana", "to", "cast"],
            )
        {
            mana_restrictions.push(sentence);
        } else if is_any_player_may_activate_sentence_lexed(&tokens) {
            restrictions.activation.push(sentence);
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
    }
}

pub(super) fn align_rewrite_activated_parse_sentences(
    parsed_sentences: &[String],
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<Vec<OwnedLexToken>>> {
    fn concat_token_slices(parts: &[Vec<OwnedLexToken>]) -> Vec<OwnedLexToken> {
        let mut joined = Vec::new();
        for part in parts {
            joined.extend(part.clone());
        }
        joined
    }

    let token_sentences = split_lexed_sentences(effect_parse_tokens);
    let mut aligned = Vec::with_capacity(parsed_sentences.len());
    let mut start_idx = 0usize;

    for parsed_sentence in parsed_sentences {
        let mut matched = None;
        let mut candidate_start = start_idx;
        while candidate_start < token_sentences.len() {
            let mut grouped = Vec::new();
            let mut probe = candidate_start;
            while probe < token_sentences.len() {
                grouped.push(token_sentences[probe].to_vec());
                let joined = concat_token_slices(&grouped);
                let joined_text = render_token_slice(&joined).trim().to_string();
                if joined_text == *parsed_sentence {
                    matched = Some((probe + 1, joined));
                    break;
                }
                if !str_starts_with(parsed_sentence.as_str(), joined_text.as_str()) {
                    break;
                }
                probe += 1;
            }

            if matched.is_some() {
                break;
            }
            candidate_start += 1;
        }

        let Some((next_start, joined_tokens)) = matched else {
            return None;
        };
        aligned.push(joined_tokens);
        start_idx = next_start;
    }

    Some(aligned)
}

fn split_rewrite_activated_effect_text(
    line: &RewriteActivatedLine,
    effect_parse_tokens: &[OwnedLexToken],
) -> SplitRewriteActivatedEffectText {
    let (parsed_sentences, restrictions) = split_text_for_parse(
        line.effect_text.as_str(),
        line.effect_text.as_str(),
        line.info.line_index,
    );
    if let Some(aligned_sentences) =
        align_rewrite_activated_parse_sentences(&parsed_sentences, effect_parse_tokens)
    {
        return finalize_rewrite_activated_effect_sentences(restrictions, aligned_sentences);
    }

    if parsed_sentences.is_empty() {
        return finalize_rewrite_activated_effect_sentences(restrictions, Vec::new());
    }

    split_rewrite_activated_effect_text_fallback(line, parsed_sentences, restrictions)
}

fn split_rewrite_activated_effect_text_fallback(
    line: &RewriteActivatedLine,
    parsed_sentences: Vec<String>,
    mut restrictions: ParsedRestrictions,
) -> SplitRewriteActivatedEffectText {
    let mut effect_sentences = Vec::new();
    let mut sentence_tokens = Vec::new();
    let mut mana_restrictions = Vec::new();
    for sentence in parsed_sentences {
        let Ok(tokens) = lexed_tokens(sentence.as_str(), line.info.line_index) else {
            mana_restrictions.push(sentence);
            continue;
        };
        let sentence_words = token_word_refs(&tokens);
        if parse_mana_usage_restriction_sentence_lexed(&tokens).is_some()
            || parse_mana_spend_bonus_sentence_lexed(&tokens).is_some()
            || word_refs_have_prefix(
                sentence_words.as_slice(),
                &["spend", "this", "mana", "only"],
            )
            || word_refs_have_prefix(
                sentence_words.as_slice(),
                &["when", "you", "spend", "this", "mana", "to", "cast"],
            )
        {
            mana_restrictions.push(sentence);
        } else if is_any_player_may_activate_sentence_lexed(&tokens) {
            restrictions.activation.push(sentence);
        } else {
            effect_sentences.push(sentence);
            sentence_tokens.push(tokens);
        }
    }
    SplitRewriteActivatedEffectText {
        effect_text: effect_sentences.join(". "),
        effect_parse_tokens: join_sentences_with_period(&sentence_tokens),
        restrictions,
        mana_restrictions,
    }
}

fn parse_activated_effects_lexed(
    effect_text: &str,
    tokens: &[OwnedLexToken],
    _line_index: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) =
        parse_each_player_and_their_creatures_damage_sentence_rewrite(effect_text, tokens)
    {
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
        if let Some(effect) = parse_next_spell_cost_reduction_sentence_rewrite(sentence_lexed) {
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

pub(crate) struct LoweredRewriteActivatedLine {
    pub(crate) chunk: LineAst,
    pub(crate) restrictions: ParsedRestrictions,
}

pub(crate) fn lower_rewrite_activated_to_chunk(
    info: LineInfo,
    cost: TotalCost,
    cost_parse_tokens: Vec<OwnedLexToken>,
    effect_text: String,
    effect_parse_tokens: Vec<OwnedLexToken>,
    timing_hint: ActivationTiming,
    is_loyalty_ability: bool,
    chosen_option_label: Option<String>,
) -> Result<LoweredRewriteActivatedLine, CardTextError> {
    lower_rewrite_activated_to_chunk_impl(
        &RewriteActivatedLine {
            info,
            cost,
            cost_parse_tokens: cost_parse_tokens.clone(),
            effect_text,
            effect_parse_tokens: effect_parse_tokens.clone(),
            timing_hint,
            is_loyalty_ability,
            chosen_option_label,
        },
        &cost_parse_tokens,
        &effect_parse_tokens,
    )
}

fn lower_rewrite_activated_to_chunk_impl(
    line: &RewriteActivatedLine,
    cost_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<LoweredRewriteActivatedLine, CardTextError> {
    let SplitRewriteActivatedEffectText {
        effect_text,
        effect_parse_tokens,
        restrictions,
        mana_restrictions,
    } = split_rewrite_activated_effect_text(line, effect_parse_tokens);
    if effect_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite activated lowering produced no parsed effect text for '{}'",
            line.info.raw_line
        )));
    }

    let normalized_cost = line.cost.clone();
    let ability_text = rewrite_activated_display_text(line);
    let normalized_effect_text = effect_text.to_ascii_lowercase();
    let normalized_raw_line = line.info.raw_line.to_ascii_lowercase();

    if str_contains(normalized_effect_text.as_str(), "add x mana")
        && !str_contains(normalized_raw_line.as_str(), "where x is")
        && !activation_cost_defines_x_for_mana_ability(&normalized_cost)
    {
        return Err(CardTextError::ParseError(
            "unresolved X in mana ability".to_string(),
        ));
    }

    if let Some(mana_output) = extract_fixed_mana_output_lexed(&effect_parse_tokens) {
        let functional_zones = infer_rewrite_activated_functional_zones(
            line,
            cost_parse_tokens,
            effect_text.as_str(),
            &effect_parse_tokens,
        )?;
        let mut parsed = ParsedAbility {
            ability: Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: normalized_cost.clone(),
                    effects: ResolutionProgram::default(),
                    choices: vec![],
                    timing: line.timing_hint.clone(),
                    is_loyalty_ability: line.is_loyalty_ability,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: Some(mana_output),
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
        apply_chosen_option_condition_to_activated(
            &mut parsed,
            line.chosen_option_label.as_deref(),
        );
        return Ok(LoweredRewriteActivatedLine {
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
            let functional_zones = infer_rewrite_activated_functional_zones(
                line,
                cost_parse_tokens,
                effect_text.as_str(),
                &effect_parse_tokens,
            )?;
            let reference_imports = find_first_sacrifice_cost_choice_tag(&normalized_cost)
                .or_else(|| find_last_exile_cost_choice_tag(&normalized_cost))
                .map(ReferenceImports::with_last_object_tag)
                .unwrap_or_default();
            let mut parsed = ParsedAbility {
                ability: Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost: normalized_cost.clone(),
                        effects: ResolutionProgram::default(),
                        choices: vec![],
                        timing: line.timing_hint.clone(),
                        is_loyalty_ability: line.is_loyalty_ability,
                        additional_restrictions: vec![],
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
            apply_chosen_option_condition_to_activated(
                &mut parsed,
                line.chosen_option_label.as_deref(),
            );

            return Ok(LoweredRewriteActivatedLine {
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
    let functional_zones = infer_rewrite_activated_functional_zones(
        line,
        cost_parse_tokens,
        effect_text.as_str(),
        &effect_parse_tokens,
    )?;
    let reference_imports = find_first_sacrifice_cost_choice_tag(&normalized_cost)
        .or_else(|| find_last_exile_cost_choice_tag(&normalized_cost))
        .map(ReferenceImports::with_last_object_tag)
        .unwrap_or_default();
    let mut parsed = ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: normalized_cost,
                effects: ResolutionProgram::default(),
                choices: vec![],
                timing: line.timing_hint.clone(),
                is_loyalty_ability: line.is_loyalty_ability,
                additional_restrictions: vec![],
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
    apply_chosen_option_condition_to_activated(&mut parsed, line.chosen_option_label.as_deref());

    Ok(LoweredRewriteActivatedLine {
        chunk: LineAst::Ability(parsed),
        restrictions,
    })
}

fn apply_chosen_option_condition_to_activated(
    parsed: &mut ParsedAbility,
    chosen_option_label: Option<&str>,
) {
    let Some(label) = chosen_option_label else {
        return;
    };
    let condition = condition_for_chosen_option_label(label);
    let AbilityKind::Activated(activated) = parsed.kind_mut() else {
        return;
    };
    activated.activation_condition = Some(match activated.activation_condition.take() {
        Some(existing) => crate::ConditionExpr::And(Box::new(existing), Box::new(condition)),
        None => condition,
    });
}

fn rewrite_activated_display_text(line: &RewriteActivatedLine) -> Option<String> {
    let raw = line.info.raw_line.trim();
    let raw_lower = raw.to_ascii_lowercase();

    for display in [
        "Boast",
        "Renew",
        "Channel",
        "Cohort",
        "Teleport",
        "Transmute",
    ] {
        let needle = format!("{} —", display.to_ascii_lowercase());
        if let Some(idx) = str_find(raw_lower.as_str(), needle.as_str()) {
            return Some(raw[idx..].trim().to_string());
        }
    }

    if let Some(chosen) = line.chosen_option_label.as_deref() {
        for display in [
            "Boast",
            "Renew",
            "Channel",
            "Cohort",
            "Teleport",
            "Transmute",
        ] {
            if chosen.eq_ignore_ascii_case(display)
                && let Some((_, tail)) = str_split_once_char(raw, '—')
            {
                return Some(format!("{display} — {}", tail.trim()));
            }
        }
    }

    None
}

fn infer_rewrite_activated_functional_zones(
    line: &RewriteActivatedLine,
    cost_parse_tokens: &[OwnedLexToken],
    effect_text: &str,
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Vec<Zone>, CardTextError> {
    let raw_lower = line.info.raw_line.to_ascii_lowercase();
    if str_contains(raw_lower.as_str(), "exile this card from your graveyard")
        || str_contains(
            raw_lower.as_str(),
            "exile this creature from your graveyard",
        )
        || str_contains(
            raw_lower.as_str(),
            "exile this permanent from your graveyard",
        )
    {
        return Ok(vec![Zone::Graveyard]);
    }
    let fallback_cost_text;
    let fallback_cost_tokens;
    let cost_tokens = if cost_parse_tokens.is_empty() {
        fallback_cost_text = line.cost.display();
        fallback_cost_tokens = lexed_tokens(fallback_cost_text.as_str(), line.info.line_index)?;
        fallback_cost_tokens.as_slice()
    } else {
        cost_parse_tokens
    };
    if effect_parse_tokens.is_empty() {
        let effect_tokens = lexed_tokens(effect_text, line.info.line_index)?;
        return Ok(infer_activated_functional_zones_lexed(
            cost_tokens,
            &split_lexed_sentences(&effect_tokens),
        ));
    }
    Ok(infer_activated_functional_zones_lexed(
        cost_tokens,
        &split_lexed_sentences(effect_parse_tokens),
    ))
}
