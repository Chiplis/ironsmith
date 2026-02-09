use super::*;

#[derive(Clone)]
struct ModalHeader {
    min: u32,
    max: Option<u32>,
    commander_allows_both: bool,
    trigger: Option<TriggerSpec>,
    line_text: String,
}

struct PendingModal {
    header: ModalHeader,
    modes: Vec<EffectMode>,
}

pub(super) fn parse_text_with_annotations(
    builder: CardDefinitionBuilder,
    text: String,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let (mut builder, mut annotations, line_infos, oracle_lines) =
        collect_line_infos(builder, text.as_str())?;

    let mut level_abilities: Vec<LevelAbility> = Vec::new();
    let mut pending_modal: Option<PendingModal> = None;
    let allow_unsupported = parser_allow_unsupported_enabled();

    let mut idx = 0usize;
    while idx < line_infos.len() {
        let info = &line_infos[idx];

        if !allow_unsupported
            && let Some((min_level, max_level)) = parse_level_header(&info.normalized.normalized)
        {
            let mut level = LevelAbility::new(min_level, max_level);
            idx += 1;

            while idx < line_infos.len() {
                let next = &line_infos[idx];
                if parse_level_header(&next.normalized.normalized).is_some() {
                    break;
                }

                let normalized_line = next.normalized.normalized.as_str();
                if let Some(pt) = parse_power_toughness(normalized_line) {
                    if let (PtValue::Fixed(power), PtValue::Fixed(toughness)) =
                        (pt.power, pt.toughness)
                    {
                        level = level.with_pt(power, toughness);
                    }
                    idx += 1;
                    continue;
                }

                let tokens = tokenize_line(normalized_line, next.line_index);
                if let Some(actions) = parse_ability_line(&tokens) {
                    for action in actions {
                        if let Some(ability) = keyword_action_to_static_ability(action) {
                            level.abilities.push(ability);
                        }
                    }
                    idx += 1;
                    continue;
                }

                if let Some(abilities) = parse_static_ability_line(&tokens)? {
                    level.abilities.extend(abilities);
                    idx += 1;
                    continue;
                }

                return Err(CardTextError::ParseError(format!(
                    "unsupported level ability line: '{}'",
                    next.raw_line
                )));
            }

            level_abilities.push(level);
            continue;
        }

        if !allow_unsupported && let Some(pending) = pending_modal.as_mut() {
            if is_bullet_line(info.raw_line.as_str()) {
                let tokens = tokenize_line(&info.normalized.normalized, info.line_index);
                let effects_ast = parse_effect_sentences(&tokens)?;
                if effects_ast.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "modal bullet line produced no effects: '{}'",
                        info.raw_line
                    )));
                }

                collect_tag_spans_from_effects_with_context(
                    &effects_ast,
                    &mut annotations,
                    &info.normalized,
                );
                let effects = compile_statement_effects(&effects_ast)?;
                let description = info
                    .raw_line
                    .trim_start()
                    .trim_start_matches(|c: char| c == '•' || c == '*' || c == '-')
                    .trim()
                    .to_string();
                pending.modes.push(EffectMode {
                    description,
                    effects,
                });
                idx += 1;
                continue;
            }

            builder = finalize_pending_modal(builder, &mut pending_modal);
            continue;
        }

        if !allow_unsupported {
            let next_is_bullet = line_infos
                .get(idx + 1)
                .is_some_and(|next| is_bullet_line(next.raw_line.as_str()));
            if next_is_bullet {
                if let Some(header) = parse_modal_header(info)? {
                    pending_modal = Some(PendingModal {
                        header,
                        modes: Vec::new(),
                    });
                    idx += 1;
                    continue;
                }
            }
        }

        let parsed = match parse_line(&info.normalized.normalized, info.line_index) {
            Ok(parsed) => parsed,
            Err(err) if allow_unsupported => {
                let reason = format!("{err:?}");
                let short_reason = reason
                    .split(" (clause:")
                    .next()
                    .unwrap_or(reason.as_str())
                    .split(" (line:")
                    .next()
                    .unwrap_or(reason.as_str())
                    .trim()
                    .to_string();
                let marker = StaticAbility::custom(
                    "unsupported_line",
                    format!(
                        "Unsupported parser line fallback: {} ({})",
                        info.raw_line.trim(),
                        short_reason
                    ),
                );
                LineAst::StaticAbility(marker)
            }
            Err(err) => return Err(err),
        };

        collect_tag_spans_from_line(&parsed, &mut annotations, &info.normalized);
        builder = apply_line_ast(builder, parsed, info, allow_unsupported, &mut annotations)?;

        idx += 1;
    }

    builder = finalize_pending_modal(builder, &mut pending_modal);

    if !oracle_lines.is_empty() {
        builder = builder.oracle_text(oracle_lines.join("\n"));
    }
    if !level_abilities.is_empty() {
        builder = builder.with_level_abilities(level_abilities);
    }

    Ok((builder.build(), annotations))
}

fn collect_line_infos(
    mut builder: CardDefinitionBuilder,
    text: &str,
) -> Result<
    (
        CardDefinitionBuilder,
        ParseAnnotations,
        Vec<LineInfo>,
        Vec<String>,
    ),
    CardTextError,
> {
    let mut oracle_lines = Vec::new();

    let card_name = builder.card_builder.name_ref().to_string();
    let short_name = card_name
        .split(',')
        .next()
        .unwrap_or(card_name.as_str())
        .trim()
        .to_string();
    let full_lower = card_name.to_ascii_lowercase();
    let short_lower = short_name.to_ascii_lowercase();

    let mut annotations = ParseAnnotations::default();
    let mut line_infos = Vec::new();

    for (line_index, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(meta) = parse_metadata_line(line)? {
            builder = builder.apply_metadata(meta)?;
            continue;
        }

        oracle_lines.push(line.to_string());

        let Some(normalized) =
            normalize_line_for_parse(line, full_lower.as_str(), short_lower.as_str())
        else {
            if is_ignorable_unparsed_line(line) {
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported or unparseable line normalization: '{line}'"
            )));
        };

        annotations.record_original_line(line_index, &normalized.original);
        annotations.record_normalized_line(line_index, &normalized.normalized);
        annotations.record_char_map(line_index, normalized.char_map.clone());

        line_infos.push(LineInfo {
            line_index,
            raw_line: line.to_string(),
            normalized,
        });
    }

    Ok((builder, annotations, line_infos, oracle_lines))
}

fn apply_line_ast(
    mut builder: CardDefinitionBuilder,
    parsed: LineAst,
    info: &LineInfo,
    allow_unsupported: bool,
    annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    match parsed {
        LineAst::Abilities(actions) => {
            for action in actions {
                builder = builder.apply_keyword_action(action);
            }
        }
        LineAst::StaticAbility(ability) => {
            builder = builder
                .with_ability(Ability::static_ability(ability).with_text(info.raw_line.as_str()));
        }
        LineAst::StaticAbilities(abilities) => {
            for ability in abilities {
                builder = builder.with_ability(
                    Ability::static_ability(ability).with_text(info.raw_line.as_str()),
                );
            }
        }
        LineAst::Ability(parsed_ability) => {
            if let Some(ref effects_ast) = parsed_ability.effects_ast {
                collect_tag_spans_from_effects_with_context(
                    effects_ast,
                    annotations,
                    &info.normalized,
                );
            }

            let mut ability = parsed_ability.ability;
            if let AbilityKind::Mana(mana_ability) = &ability.kind
                && mana_ability.effects.is_none()
            {
                if let Some(options) =
                    parse_mana_output_options_for_line(&info.raw_line, info.line_index)?
                    && options.len() > 1
                {
                    for option in options {
                        let mut split = ability.clone();
                        if let AbilityKind::Mana(ref mut inner) = split.kind {
                            inner.mana = option;
                        }
                        builder = builder.with_ability(split.with_text(info.raw_line.as_str()));
                    }
                    return Ok(builder);
                }
            }

            if ability.text.is_none() {
                ability = ability.with_text(info.raw_line.as_str());
            }
            builder = builder.with_ability(ability);
        }
        LineAst::Statement { effects } => {
            if effects.is_empty() {
                if allow_unsupported {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        "empty effect statement".to_string(),
                    ));
                }
                return Err(CardTextError::ParseError(format!(
                    "line parsed to empty effect statement: '{}'",
                    info.raw_line
                )));
            }

            if let Some(enchant_filter) = effects.iter().find_map(|effect| {
                if let EffectAst::Enchant { filter } = effect {
                    Some(filter.clone())
                } else {
                    None
                }
            }) {
                builder.aura_attach_filter = Some(enchant_filter);
            }

            let compiled = match compile_statement_effects(&effects) {
                Ok(compiled) => compiled,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };

            if let Some(ref mut existing) = builder.spell_effect {
                existing.extend(compiled);
            } else {
                builder.spell_effect = Some(compiled);
            }
        }
        LineAst::AdditionalCost { effects } => {
            if effects.is_empty() {
                if allow_unsupported {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        "empty additional cost statement".to_string(),
                    ));
                }
                return Err(CardTextError::ParseError(format!(
                    "line parsed to empty additional-cost statement: '{}'",
                    info.raw_line
                )));
            }

            let compiled = match compile_statement_effects(&effects) {
                Ok(compiled) => compiled,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };

            builder.cost_effects.extend(compiled);
        }
        LineAst::AlternativeCost {
            mana_cost,
            cost_effects,
        } => {
            builder
                .alternative_casts
                .push(AlternativeCastingMethod::alternative_cost(
                    "Parsed alternative cost",
                    mana_cost,
                    cost_effects,
                ));
        }
        LineAst::Triggered { trigger, effects } => {
            let (compiled_effects, choices) =
                match compile_trigger_effects(Some(&trigger), &effects) {
                    Ok(compiled) => compiled,
                    Err(err) if allow_unsupported => {
                        return Ok(push_unsupported_marker(
                            builder,
                            info.raw_line.as_str(),
                            format!("{err:?}"),
                        ));
                    }
                    Err(err) => return Err(err),
                };

            let compiled_trigger = compile_trigger_spec(trigger);
            builder = builder.with_ability(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: compiled_trigger,
                    effects: compiled_effects,
                    choices,
                    intervening_if: None,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(info.raw_line.clone()),
            });
        }
    }

    Ok(builder)
}

fn push_unsupported_marker(
    builder: CardDefinitionBuilder,
    raw_line: &str,
    reason: String,
) -> CardDefinitionBuilder {
    builder.with_ability(
        Ability::static_ability(StaticAbility::custom(
            "unsupported_line",
            format!(
                "Unsupported parser line fallback: {} ({})",
                raw_line.trim(),
                reason
            ),
        ))
        .with_text(raw_line),
    )
}

fn parse_modal_header(info: &LineInfo) -> Result<Option<ModalHeader>, CardTextError> {
    let tokens = tokenize_line(&info.normalized.normalized, info.line_index);
    let token_words = words(&tokens);
    let Some(choose_idx) = tokens.iter().position(|token| token.is_word("choose")) else {
        return Ok(None);
    };

    let mut min = None;
    let mut max = None;
    let choose_tokens = &tokens[choose_idx + 1..];
    if choose_tokens.len() >= 3
        && choose_tokens[0].is_word("one")
        && choose_tokens[1].is_word("or")
        && choose_tokens[2].is_word("more")
    {
        min = Some(1);
        max = None;
    } else if choose_tokens.len() >= 3
        && choose_tokens[0].is_word("one")
        && choose_tokens[1].is_word("or")
        && choose_tokens[2].is_word("both")
    {
        min = Some(1);
        max = Some(2);
    } else if choose_tokens.len() >= 2
        && choose_tokens[0].is_word("up")
        && choose_tokens[1].is_word("to")
    {
        if let Some((value, _)) = parse_number(&choose_tokens[2..]) {
            min = Some(0);
            max = Some(value);
        }
    } else if let Some((value, _)) = parse_number(choose_tokens) {
        min = Some(value);
        max = Some(value);
    }

    let Some(min) = min else {
        return Ok(None);
    };

    let commander_allows_both = token_words.contains(&"commander") && token_words.contains(&"both");

    let mut trigger = None;
    if let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    {
        if choose_idx > comma_idx {
            let start_idx = if tokens.first().is_some_and(|token| {
                token.is_word("whenever") || token.is_word("when") || token.is_word("at")
            }) {
                1
            } else {
                0
            };
            if comma_idx > start_idx {
                let trigger_tokens = &tokens[start_idx..comma_idx];
                if !trigger_tokens.is_empty() {
                    trigger = Some(parse_trigger_clause(trigger_tokens)?);
                }
            }
        }
    }

    Ok(Some(ModalHeader {
        min,
        max,
        commander_allows_both,
        trigger,
        line_text: info.raw_line.clone(),
    }))
}

fn finalize_pending_modal(
    mut builder: CardDefinitionBuilder,
    pending_modal: &mut Option<PendingModal>,
) -> CardDefinitionBuilder {
    let Some(pending) = pending_modal.take() else {
        return builder;
    };

    let modes = pending.modes;
    if modes.is_empty() {
        return builder;
    }

    let mode_count = modes.len() as u32;
    let max = pending.header.max.unwrap_or(mode_count).min(mode_count);
    let min = pending.header.min.min(max);

    let modal_effect = if pending.header.commander_allows_both {
        let max_both = mode_count.min(2).max(1);
        let choose_both = if max_both == 1 {
            Effect::choose_one(modes.clone())
        } else {
            Effect::choose_up_to(max_both, 1, modes.clone())
        };
        let choose_one = Effect::choose_one(modes.clone());
        Effect::conditional(
            Condition::YouControlCommander,
            vec![choose_both],
            vec![choose_one],
        )
    } else if min == 1 && max == 1 {
        Effect::choose_one(modes)
    } else if min == max {
        Effect::choose_exactly(max, modes)
    } else {
        Effect::choose_up_to(max, min, modes)
    };

    if let Some(trigger) = pending.header.trigger {
        let compiled_trigger = compile_trigger_spec(trigger);
        builder = builder.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: compiled_trigger,
                effects: vec![modal_effect],
                choices: Vec::new(),
                intervening_if: None,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some(pending.header.line_text),
        });
    } else if let Some(ref mut existing) = builder.spell_effect {
        existing.push(modal_effect);
    } else {
        builder.spell_effect = Some(vec![modal_effect]);
    }

    builder
}

fn is_bullet_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('•') || trimmed.starts_with('*') || trimmed.starts_with('-')
}
