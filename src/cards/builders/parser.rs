use super::*;

#[derive(Clone)]
struct ModalHeader {
    min: u32,
    max: Option<u32>,
    same_mode_more_than_once: bool,
    mode_must_be_unchosen: bool,
    mode_must_be_unchosen_this_turn: bool,
    commander_allows_both: bool,
    trigger: Option<TriggerSpec>,
    activated: Option<ModalActivatedHeader>,
    x_replacement: Option<Value>,
    prefix_effects: Vec<Effect>,
    prefix_choices: Vec<ChooseSpec>,
    modal_gate: Option<ModalGate>,
    line_text: String,
}

#[derive(Clone)]
struct ModalActivatedHeader {
    mana_cost: TotalCost,
    functional_zones: Vec<Zone>,
    timing: ActivationTiming,
    additional_restrictions: Vec<String>,
}

struct PendingModal {
    header: ModalHeader,
    modes: Vec<EffectMode>,
}

#[derive(Clone)]
struct ModalGate {
    predicate: EffectPredicate,
    remove_mode_only: bool,
}

#[derive(Default)]
struct PendingRestrictions {
    activation: Vec<String>,
    trigger: Vec<String>,
}

pub(super) fn parse_text_with_annotations(
    builder: CardDefinitionBuilder,
    text: String,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let (mut builder, mut annotations, line_infos) = collect_line_infos(builder, text.as_str())?;

    let mut level_abilities: Vec<LevelAbility> = Vec::new();
    let mut pending_modal: Option<PendingModal> = None;
    let mut pending_restrictions = PendingRestrictions::default();
    let mut last_restrictable_ability: Option<usize> = None;
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
                    reject_unimplemented_keyword_actions(&actions, next.raw_line.as_str())?;
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

        if let Some(pending) = pending_modal.as_mut() {
            if is_bullet_line(info.raw_line.as_str()) {
                let tokens = tokenize_line(&info.normalized.normalized, info.line_index);
                let mut effects_ast = match parse_effect_sentences(&tokens) {
                    Ok(effects_ast) => effects_ast,
                    Err(err) if allow_unsupported => {
                        builder = push_unsupported_marker(
                            builder,
                            info.raw_line.as_str(),
                            format!("{err:?}"),
                        );
                        idx += 1;
                        continue;
                    }
                    Err(err) => return Err(err),
                };
                if effects_ast.is_empty() {
                    if allow_unsupported {
                        builder = push_unsupported_marker(
                            builder,
                            info.raw_line.as_str(),
                            "modal bullet line produced no effects".to_string(),
                        );
                        idx += 1;
                        continue;
                    }
                    return Err(CardTextError::ParseError(format!(
                        "modal bullet line produced no effects: '{}'",
                        info.raw_line
                    )));
                }

                if let Some(replacement) = pending.header.x_replacement.as_ref()
                    && let Err(err) = replace_modal_header_x_in_effects_ast(
                        &mut effects_ast,
                        replacement,
                        pending.header.line_text.as_str(),
                    )
                {
                    if allow_unsupported {
                        builder = push_unsupported_marker(
                            builder,
                            info.raw_line.as_str(),
                            format!("{err:?}"),
                        );
                        idx += 1;
                        continue;
                    }
                    return Err(err);
                }

                collect_tag_spans_from_effects_with_context(
                    &effects_ast,
                    &mut annotations,
                    &info.normalized,
                );
                let effects = match compile_statement_effects(&effects_ast) {
                    Ok(effects) => effects,
                    Err(err) if allow_unsupported => {
                        builder = push_unsupported_marker(
                            builder,
                            info.raw_line.as_str(),
                            format!("{err:?}"),
                        );
                        idx += 1;
                        continue;
                    }
                    Err(err) => return Err(err),
                };
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

        let next_is_bullet = line_infos
            .get(idx + 1)
            .is_some_and(|next| is_bullet_line(next.raw_line.as_str()));
        if next_is_bullet {
            match parse_modal_header(info) {
                Ok(Some(header)) => {
                    pending_modal = Some(PendingModal {
                        header,
                        modes: Vec::new(),
                    });
                    idx += 1;
                    continue;
                }
                Ok(None) => {}
                Err(err) if allow_unsupported => {
                    builder = push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    );
                    idx += 1;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }

        let line_sentences =
            split_sentences_for_parse(&info.normalized.normalized, info.line_index);
        let mut parsed_portion = Vec::new();
        for sentence in line_sentences {
            if sentence.is_empty() {
                continue;
            }

            if queue_restriction(&sentence, info.line_index, &mut pending_restrictions) {
                continue;
            }

            parsed_portion.push(sentence);
        }

        for restriction in extract_parenthetical_restrictions(&info.raw_line) {
            let _ = queue_restriction(&restriction, info.line_index, &mut pending_restrictions);
        }

        let mut handled_restrictions_for_new_ability = false;

        if !parsed_portion.is_empty() {
            let parse_chunks = split_trigger_sentence_chunks(&parsed_portion, info.line_index);
            for line_text in parse_chunks {
                let parsed = match parse_line(&line_text, info.line_index) {
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

                let mut handled_followup_statement = false;
                if let LineAst::Statement { effects } = &parsed {
                    if apply_instead_followup_statement_to_last_ability(
                        &mut builder,
                        last_restrictable_ability,
                        effects,
                        info,
                        allow_unsupported,
                        &mut annotations,
                    )? {
                        handled_followup_statement = true;
                        handled_restrictions_for_new_ability = true;
                    }
                }

                if handled_followup_statement {
                    continue;
                }

                let abilities_before = builder.abilities.len();
                collect_tag_spans_from_line(&parsed, &mut annotations, &info.normalized);
                builder =
                    apply_line_ast(builder, parsed, info, allow_unsupported, &mut annotations)?;
                let abilities_after = builder.abilities.len();

                for ability_idx in abilities_before..abilities_after {
                    apply_pending_restrictions_to_ability(
                        &mut builder.abilities[ability_idx],
                        &mut pending_restrictions,
                    );
                    handled_restrictions_for_new_ability = true;
                }

                if abilities_after > abilities_before {
                    let mut last_restrictable = None;
                    for ability_idx in (abilities_before..abilities_after).rev() {
                        if is_restrictable_ability(&builder.abilities[ability_idx]) {
                            last_restrictable = Some(ability_idx);
                            break;
                        }
                    }
                    if last_restrictable.is_some() {
                        last_restrictable_ability = last_restrictable;
                    }
                }
            }
        }

        if !handled_restrictions_for_new_ability
            && let Some(index) = last_restrictable_ability
            && index < builder.abilities.len()
        {
            apply_pending_restrictions_to_ability(
                &mut builder.abilities[index],
                &mut pending_restrictions,
            );
        }

        if !pending_restrictions.activation.is_empty() || !pending_restrictions.trigger.is_empty() {
            pending_restrictions.activation.clear();
            pending_restrictions.trigger.clear();
        }

        idx += 1;
    }

    builder = finalize_pending_modal(builder, &mut pending_modal);

    if !level_abilities.is_empty() {
        builder = builder.with_level_abilities(level_abilities);
    }

    builder = normalize_channel_spell_effect(builder);

    Ok((builder.build(), annotations))
}

fn collect_line_infos(
    mut builder: CardDefinitionBuilder,
    text: &str,
) -> Result<(CardDefinitionBuilder, ParseAnnotations, Vec<LineInfo>), CardTextError> {
    fn normalize_card_name_for_self_reference(name: &str) -> String {
        let lower = name.to_ascii_lowercase();
        let bytes = lower.as_bytes();
        if bytes.len() > 2 && bytes[1] == b'-' && bytes[0].is_ascii_alphabetic() {
            lower[2..].to_string()
        } else {
            lower
        }
    }

    let card_name = builder.card_builder.name_ref().to_string();
    let short_name = card_name
        .split(',')
        .next()
        .unwrap_or(card_name.as_str())
        .trim()
        .to_string();
    let full_lower = normalize_card_name_for_self_reference(card_name.as_str());
    let short_lower = normalize_card_name_for_self_reference(short_name.as_str());

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

        let split_lines = split_parse_line_variants(line);
        for (split_index, split_line) in split_lines.iter().enumerate() {
            let Some(normalized) =
                normalize_line_for_parse(split_line, full_lower.as_str(), short_lower.as_str())
            else {
                if is_ignorable_unparsed_line(split_line) {
                    continue;
                }
                return Err(CardTextError::ParseError(format!(
                    "unsupported or unparseable line normalization: '{split_line}'"
                )));
            };

            let virtual_line_index = line_index.saturating_mul(8).saturating_add(split_index);
            annotations.record_original_line(virtual_line_index, &normalized.original);
            annotations.record_normalized_line(virtual_line_index, &normalized.normalized);
            annotations.record_char_map(virtual_line_index, normalized.char_map.clone());

            line_infos.push(LineInfo {
                line_index: virtual_line_index,
                raw_line: split_line.to_string(),
                normalized,
            });
        }
    }

    if !line_infos.is_empty() {
        let oracle_text = line_infos
            .iter()
            .map(|info| info.raw_line.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        builder = builder.oracle_text(oracle_text);
    }

    Ok((builder, annotations, line_infos))
}

fn normalize_channel_spell_effect(mut builder: CardDefinitionBuilder) -> CardDefinitionBuilder {
    use crate::ability::{ActivationTiming, ManaAbility, ManaAbilityCondition};
    use crate::effect::{EffectPredicate, Value};
    use crate::mana::ManaSymbol;
    use crate::target::{ChooseSpec, PlayerFilter};

    if builder.card_builder.name_ref() != "Channel" {
        return builder;
    }
    let Some(effects) = builder.spell_effect.as_ref() else {
        return builder;
    };
    if effects.len() != 2 {
        return builder;
    }

    let Some(with_id) = effects[0].downcast_ref::<crate::effects::WithIdEffect>() else {
        return builder;
    };
    let Some(lose) = with_id.effect.downcast_ref::<crate::effects::LoseLifeEffect>() else {
        return builder;
    };
    if !matches!(lose.amount, Value::Fixed(1)) {
        return builder;
    }
    if !matches!(lose.player, ChooseSpec::Player(PlayerFilter::You)) {
        return builder;
    }

    let Some(if_effect) = effects[1].downcast_ref::<crate::effects::IfEffect>() else {
        return builder;
    };
    if if_effect.condition != with_id.id
        || !matches!(if_effect.predicate, EffectPredicate::Happened)
        || !if_effect.else_.is_empty()
        || if_effect.then.len() != 1
    {
        return builder;
    }
    let Some(add_mana) = if_effect.then[0].downcast_ref::<crate::effects::AddManaEffect>() else {
        return builder;
    };
    if add_mana.player != PlayerFilter::You || add_mana.mana != vec![ManaSymbol::Colorless] {
        return builder;
    }

    // Rewrite to a temporary granted mana ability until end of turn.
    let ability = ManaAbility {
        mana_cost: crate::cost::TotalCost::from_cost(crate::costs::Cost::life(1)),
        mana: vec![ManaSymbol::Colorless],
        effects: None,
        activation_condition: Some(ManaAbilityCondition::Timing(ActivationTiming::AnyTime)),
    };
    builder.spell_effect = Some(vec![crate::effect::Effect::new(
        crate::effects::GrantManaAbilityUntilEotEffect::new(ability),
    )]);
    builder
}

fn split_parse_line_variants(line: &str) -> Vec<String> {
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("as an additional cost to cast this spell")
        && let Some(period_idx) = line.find('.')
    {
        let first = line[..=period_idx].trim();
        let second = line[period_idx + 1..].trim();
        if !first.is_empty() && !second.is_empty() {
            return vec![first.to_string(), second.to_string()];
        }
    }

    let marker = ". when you spend this mana to cast ";
    let marker_compact = ".when you spend this mana to cast ";
    let split_at = lower.find(marker).or_else(|| lower.find(marker_compact));
    if let Some(idx) = split_at {
        let first = line[..=idx].trim();
        let second = line[idx + 1..].trim();
        if first.contains(':') && !second.is_empty() {
            return vec![first.to_string(), second.to_string()];
        }
    }

    for marker in [
        ". this cost is reduced by ",
        ".this cost is reduced by ",
        ". this spell costs ",
        ".this spell costs ",
    ] {
        if let Some(idx) = lower.find(marker) {
            let first = line[..=idx].trim();
            let second = line[idx + 1..].trim();
            if !first.is_empty() && !second.is_empty() {
                return vec![first.to_string(), second.to_string()];
            }
        }
    }
    vec![line.to_string()]
}

fn title_case_words(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn color_set_name(colors: ColorSet) -> Option<&'static str> {
    if colors == ColorSet::WHITE {
        return Some("white");
    }
    if colors == ColorSet::BLUE {
        return Some("blue");
    }
    if colors == ColorSet::BLACK {
        return Some("black");
    }
    if colors == ColorSet::RED {
        return Some("red");
    }
    if colors == ColorSet::GREEN {
        return Some("green");
    }
    None
}

fn keyword_action_line_text(action: &KeywordAction) -> String {
    match action {
        KeywordAction::Flying => "Flying".to_string(),
        KeywordAction::Menace => "Menace".to_string(),
        KeywordAction::Hexproof => "Hexproof".to_string(),
        KeywordAction::Haste => "Haste".to_string(),
        KeywordAction::Improvise => "Improvise".to_string(),
        KeywordAction::Convoke => "Convoke".to_string(),
        KeywordAction::AffinityForArtifacts => "Affinity for artifacts".to_string(),
        KeywordAction::Delve => "Delve".to_string(),
        KeywordAction::FirstStrike => "First strike".to_string(),
        KeywordAction::DoubleStrike => "Double strike".to_string(),
        KeywordAction::Deathtouch => "Deathtouch".to_string(),
        KeywordAction::Lifelink => "Lifelink".to_string(),
        KeywordAction::Vigilance => "Vigilance".to_string(),
        KeywordAction::Trample => "Trample".to_string(),
        KeywordAction::Reach => "Reach".to_string(),
        KeywordAction::Defender => "Defender".to_string(),
        KeywordAction::Flash => "Flash".to_string(),
        KeywordAction::Phasing => "Phasing".to_string(),
        KeywordAction::Indestructible => "Indestructible".to_string(),
        KeywordAction::Shroud => "Shroud".to_string(),
        KeywordAction::Ward(amount) => format!("Ward {{{amount}}}"),
        KeywordAction::Wither => "Wither".to_string(),
        KeywordAction::Infect => "Infect".to_string(),
        KeywordAction::Undying => "Undying".to_string(),
        KeywordAction::Persist => "Persist".to_string(),
        KeywordAction::Prowess => "Prowess".to_string(),
        KeywordAction::Exalted => "Exalted".to_string(),
        KeywordAction::Storm => "Storm".to_string(),
        KeywordAction::Toxic(amount) => format!("Toxic {amount}"),
        KeywordAction::Fear => "Fear".to_string(),
        KeywordAction::Intimidate => "Intimidate".to_string(),
        KeywordAction::Shadow => "Shadow".to_string(),
        KeywordAction::Horsemanship => "Horsemanship".to_string(),
        KeywordAction::Flanking => "Flanking".to_string(),
        KeywordAction::Landwalk(subtype) => {
            let mut subtype = format!("{subtype:?}").to_ascii_lowercase();
            subtype.push_str("walk");
            title_case_words(&subtype)
        }
        KeywordAction::Bloodthirst(amount) => format!("Bloodthirst {amount}"),
        KeywordAction::Rampage(amount) => format!("Rampage {amount}"),
        KeywordAction::Bushido(amount) => format!("Bushido {amount}"),
        KeywordAction::Changeling => "Changeling".to_string(),
        KeywordAction::ProtectionFrom(colors) => {
            if let Some(color_name) = color_set_name(*colors) {
                return format!("Protection from {color_name}");
            }
            "Protection from colors".to_string()
        }
        KeywordAction::ProtectionFromAllColors => "Protection from all colors".to_string(),
        KeywordAction::ProtectionFromColorless => "Protection from colorless".to_string(),
        KeywordAction::ProtectionFromCardType(card_type) => {
            format!("Protection from {:?}", card_type).to_ascii_lowercase()
        }
        KeywordAction::ProtectionFromSubtype(subtype) => {
            format!("Protection from {:?}", subtype).to_ascii_lowercase()
        }
        KeywordAction::Unblockable => "This creature can't be blocked".to_string(),
        KeywordAction::Devoid => "Devoid".to_string(),
        KeywordAction::Annihilator(amount) => format!("Annihilator {amount}"),
        KeywordAction::Crew { amount, .. } => format!("Crew {amount}"),
        KeywordAction::Marker(name) => title_case_words(name),
        KeywordAction::MarkerText(text) => text.clone(),
    }
}

fn keyword_actions_line_text(actions: &[KeywordAction], separator: &str) -> Option<String> {
    if actions.is_empty() {
        return None;
    }
    let parts = actions
        .iter()
        .map(keyword_action_line_text)
        .collect::<Vec<_>>();
    Some(parts.join(separator))
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
            let keyword_segment = info
                .raw_line
                .split('(')
                .next()
                .unwrap_or(info.raw_line.as_str());
            let separator = if keyword_segment.contains(';') {
                "; "
            } else {
                ", "
            };
            let line_text = if actions
                .iter()
                .any(|action| matches!(action, KeywordAction::Crew { .. }))
            {
                // Preserve "Activate only ..." restrictions that are often written inline
                // on the Crew line, while still stripping reminder text in parentheses.
                Some(keyword_segment.trim().to_string())
            } else {
                keyword_actions_line_text(&actions, separator)
            };
            for action in actions {
                let ability_count_before = builder.abilities.len();
                builder = builder.apply_keyword_action(action);
                if let Some(line_text) = line_text.as_ref() {
                    for ability in &mut builder.abilities[ability_count_before..] {
                        ability.text = Some(line_text.clone());
                    }
                }
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

            let normalized_line = info.normalized.normalized.as_str().to_ascii_lowercase();
            if normalized_line.contains(" instead")
                && compiled.len() == 1
                && let Some(ref mut existing) = builder.spell_effect
                && !existing.is_empty()
                && let Some(replacement) =
                    compiled[0].downcast_ref::<crate::effects::ConditionalEffect>()
                && replacement.if_false.is_empty()
                && let Some(previous_target) = existing
                    .last()
                    .and_then(|effect| effect.downcast_ref::<crate::effects::DealDamageEffect>())
                    .map(|damage| damage.target.clone())
                && replacement.if_true.len() == 1
                && let Some(replacement_damage) =
                    replacement.if_true[0].downcast_ref::<crate::effects::DealDamageEffect>()
            {
                let mut replacement = replacement.clone();
                // If the replacement clause omitted a target, the parser will default it to
                // "target player or planeswalker". For spells that are modifying prior damage,
                // we want to preserve the original target.
                if replacement_damage.target == ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
                {
                    replacement.if_true = vec![Effect::deal_damage(
                        replacement_damage.amount.clone(),
                        previous_target,
                    )];
                }

                let previous = existing.pop().expect("checked non-empty above");
                existing.push(Effect::new(crate::effects::ConditionalEffect::new(
                    replacement.condition,
                    replacement.if_true,
                    vec![previous],
                )));
            } else if let Some(ref mut existing) = builder.spell_effect {
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
        LineAst::OptionalCost(cost) => {
            builder = builder.optional_cost(cost);
        }
        LineAst::AdditionalCostChoice { options } => {
            if options.len() < 2 {
                if allow_unsupported {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        "additional cost choice requires at least two options".to_string(),
                    ));
                }
                return Err(CardTextError::ParseError(format!(
                    "line parsed to invalid additional-cost choice (line: '{}')",
                    info.raw_line
                )));
            }

            let mut modes = Vec::new();
            for option in options {
                if option.effects.is_empty() {
                    if allow_unsupported {
                        return Ok(push_unsupported_marker(
                            builder,
                            info.raw_line.as_str(),
                            "additional cost choice option produced no effects".to_string(),
                        ));
                    }
                    return Err(CardTextError::ParseError(format!(
                        "line parsed to empty additional-cost option (line: '{}')",
                        info.raw_line
                    )));
                }

                let compiled = match compile_statement_effects(&option.effects) {
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
                modes.push(EffectMode {
                    description: option.description.trim().to_string(),
                    effects: compiled,
                });
            }
            builder.cost_effects.push(Effect::choose_one(modes));
        }
        LineAst::AlternativeCastingMethod(method) => {
            builder.alternative_casts.push(method);
        }
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } => {
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
                    intervening_if: max_triggers_per_turn
                        .map(crate::ability::InterveningIfCondition::MaxTimesEachTurn),
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
    let same_mode_more_than_once = token_words
        .windows(5)
        .any(|window| window == ["same", "mode", "more", "than", "once"]);
    let mode_must_be_unchosen_this_turn = token_words.windows(6).any(|window| {
        window == ["that", "hasnt", "been", "chosen", "this", "turn"]
            || window == ["that", "hasn't", "been", "chosen", "this", "turn"]
    }) || token_words
        .windows(7)
        .any(|window| window == ["that", "has", "not", "been", "chosen", "this", "turn"]);
    let mode_must_be_unchosen = mode_must_be_unchosen_this_turn
        || token_words.windows(4).any(|window| {
            window == ["that", "hasnt", "been", "chosen"]
                || window == ["that", "hasn't", "been", "chosen"]
        })
        || token_words
            .windows(5)
            .any(|window| window == ["that", "has", "not", "been", "chosen"]);

    let mut trigger = None;
    let mut activated = None;
    let x_replacement = parse_modal_header_x_replacement(&tokens, choose_idx);
    let mut effect_start_idx = 0usize;
    if let Some(colon_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Colon(_)))
        .filter(|idx| *idx < choose_idx)
    {
        let cost_region = &tokens[..colon_idx];
        if let Some(cost_start) = find_activation_cost_start(cost_region) {
            let cost_tokens = &cost_region[cost_start..];
            if !cost_tokens.is_empty() && starts_with_activation_cost(cost_tokens) {
                let (mana_cost, cost_effects) = parse_activation_cost(cost_tokens)?;
                let mana_cost = crate::ability::merge_cost_effects(mana_cost, cost_effects);

                let prechoose_tokens = trim_commas(&tokens[colon_idx + 1..choose_idx]).to_vec();
                let effect_sentences = if prechoose_tokens.is_empty() {
                    Vec::new()
                } else {
                    split_on_period(&prechoose_tokens)
                };
                let functional_zones =
                    infer_activated_functional_zones(cost_tokens, &effect_sentences);

                activated = Some(ModalActivatedHeader {
                    mana_cost,
                    functional_zones,
                    timing: ActivationTiming::AnyTime,
                    additional_restrictions: Vec::new(),
                });
                effect_start_idx = colon_idx + 1;
            }
        }
    }

    if activated.is_none()
        && let Some(comma_idx) = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        && choose_idx > comma_idx
    {
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
        effect_start_idx = comma_idx + 1;
    }

    let prechoose_tokens = trim_commas(&tokens[effect_start_idx..choose_idx]).to_vec();
    let (prefix_effects_ast, modal_gate) = parse_modal_header_prefix_effects(&prechoose_tokens)?;
    let (prefix_effects, prefix_choices) = if prefix_effects_ast.is_empty() {
        (Vec::new(), Vec::new())
    } else if let Some(trigger_spec) = trigger.as_ref() {
        compile_trigger_effects(Some(trigger_spec), &prefix_effects_ast)?
    } else if activated.is_some() {
        compile_trigger_effects(None, &prefix_effects_ast)?
    } else {
        (compile_statement_effects(&prefix_effects_ast)?, Vec::new())
    };

    Ok(Some(ModalHeader {
        min,
        max,
        same_mode_more_than_once,
        mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn,
        commander_allows_both,
        trigger,
        activated,
        x_replacement,
        prefix_effects,
        prefix_choices,
        modal_gate,
        line_text: info.raw_line.clone(),
    }))
}

fn parse_modal_header_x_replacement(tokens: &[Token], choose_idx: usize) -> Option<Value> {
    let choose_tail = tokens.get(choose_idx + 1..)?;
    let choose_tail_words = words(choose_tail);
    let x_word_idx = choose_tail_words.iter().position(|word| *word == "x")?;
    if choose_tail_words.get(x_word_idx + 1).copied() != Some("is") {
        return None;
    }

    let x_token_idx = token_index_for_word_index(choose_tail, x_word_idx)?;
    let x_clause_tokens = trim_commas(&choose_tail[x_token_idx..]);
    parse_x_is_value_clause(&x_clause_tokens)
}

fn parse_x_is_value_clause(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["x", "is"]) {
        return None;
    }

    if (words.contains(&"spell") || words.contains(&"spells"))
        && (words.contains(&"cast") || words.contains(&"casts"))
        && words.contains(&"turn")
    {
        let player = if words
            .iter()
            .any(|word| matches!(*word, "you" | "your" | "youve"))
        {
            PlayerFilter::You
        } else if words
            .iter()
            .any(|word| matches!(*word, "opponent" | "opponents"))
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::Any
        };
        return Some(Value::SpellsCastThisTurn(player));
    }

    let mut with_where = Vec::with_capacity(tokens.len() + 1);
    with_where.push(Token::Word("where".to_string(), TextSpan::synthetic()));
    with_where.extend_from_slice(tokens);
    parse_where_x_value_clause(&with_where)
}

fn replace_modal_header_x_in_effects_ast(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_modal_header_x_in_effect_ast(effect, replacement, clause)?;
    }
    Ok(())
}

fn replace_modal_header_x_in_value(
    value: &mut Value,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    if !value_contains_unbound_x(value) {
        return Ok(());
    }
    *value = replace_unbound_x_with_value(value.clone(), replacement, clause)?;
    Ok(())
}

fn replace_modal_header_x_in_effect_ast(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::Draw { count: amount, .. }
        | EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::PreventDamage { amount, .. }
        | EffectAst::PreventDamageEach { amount, .. }
        | EffectAst::Scry { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::Discard { count: amount, .. }
        | EffectAst::Mill { count: amount, .. }
        | EffectAst::PutCounters { count: amount, .. }
        | EffectAst::PutCountersAll { count: amount, .. }
        | EffectAst::RemoveUpToAnyCounters { amount, .. }
        | EffectAst::RemoveCountersAll { amount, .. }
        | EffectAst::SetLifeTotal { amount, .. }
        | EffectAst::PoisonCounters { count: amount, .. }
        | EffectAst::EnergyCounters { count: amount, .. }
        | EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaFromLandCouldProduce { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. }
        | EffectAst::CreateToken { count: amount, .. }
        | EffectAst::CreateTokenCopy { count: amount, .. }
        | EffectAst::CreateTokenCopyFromSource { count: amount, .. }
        | EffectAst::CreateTokenWithMods { count: amount, .. }
        | EffectAst::Monstrosity { amount, .. } => {
            replace_modal_header_x_in_value(amount, replacement, clause)?;
        }
        EffectAst::Pump {
            power, toughness, ..
        }
        | EffectAst::SetBasePowerToughness {
            power, toughness, ..
        }
        | EffectAst::PumpAll {
            power, toughness, ..
        } => {
            replace_modal_header_x_in_value(power, replacement, clause)?;
            replace_modal_header_x_in_value(toughness, replacement, clause)?;
        }
        EffectAst::SetBasePower { power, .. } => {
            replace_modal_header_x_in_value(power, replacement, clause)?;
        }
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::ForEachPlayerDoesNot { effects }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::UnlessPays { effects, .. }
        | EffectAst::VoteOption { effects, .. } => {
            replace_modal_header_x_in_effects_ast(effects, replacement, clause)?;
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            replace_modal_header_x_in_effects_ast(effects, replacement, clause)?;
            replace_modal_header_x_in_effects_ast(alternative, replacement, clause)?;
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            replace_modal_header_x_in_effects_ast(if_true, replacement, clause)?;
            replace_modal_header_x_in_effects_ast(if_false, replacement, clause)?;
        }
        _ => {}
    }

    Ok(())
}

fn parse_modal_header_prefix_effects(
    tokens: &[Token],
) -> Result<(Vec<EffectAst>, Option<ModalGate>), CardTextError> {
    if tokens.is_empty() {
        return Ok((Vec::new(), None));
    }

    let (prefix_tokens, modal_gate) = strip_trailing_modal_gate_clause(tokens);
    if prefix_tokens.is_empty() {
        return Ok((Vec::new(), modal_gate));
    }

    let effects = parse_effect_sentences(&prefix_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(
            "modal header prefix produced no effects".to_string(),
        ));
    }

    Ok((effects, modal_gate))
}

fn strip_trailing_modal_gate_clause(tokens: &[Token]) -> (Vec<Token>, Option<ModalGate>) {
    let sentence_start = tokens
        .iter()
        .rposition(|token| matches!(token, Token::Period(_)))
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let sentence_tokens = trim_commas(&tokens[sentence_start..]);
    if sentence_tokens.is_empty() {
        return (tokens.to_vec(), None);
    }
    if !sentence_tokens
        .first()
        .is_some_and(|token| token.is_word("if") || token.is_word("when"))
    {
        return (tokens.to_vec(), None);
    }

    let comma_idx = sentence_tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .unwrap_or(sentence_tokens.len());
    if comma_idx <= 1 {
        return (tokens.to_vec(), None);
    }

    let predicate_tokens = &sentence_tokens[1..comma_idx];
    let Some(predicate) = parse_if_result_predicate(predicate_tokens) else {
        return (tokens.to_vec(), None);
    };

    let trailing_tokens = if comma_idx < sentence_tokens.len() {
        trim_commas(&sentence_tokens[comma_idx + 1..])
    } else {
        Vec::new()
    };
    if !trailing_tokens.is_empty() {
        return (tokens.to_vec(), None);
    }

    let mut prefix_tokens = tokens[..sentence_start].to_vec();
    while matches!(prefix_tokens.last(), Some(Token::Comma(_))) {
        prefix_tokens.pop();
    }

    let effect_predicate = match predicate {
        IfResultPredicate::Did => EffectPredicate::Happened,
        IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
        IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
    };
    let predicate_words = words(predicate_tokens);
    let remove_mode_only = predicate_words.len() >= 2
        && matches!(predicate_words[0], "you" | "they")
        && matches!(predicate_words[1], "remove" | "removed");

    (
        prefix_tokens,
        Some(ModalGate {
            predicate: effect_predicate,
            remove_mode_only,
        }),
    )
}

fn try_merge_modal_into_remove_mode(
    effects: &mut Vec<Effect>,
    modal_effect: Effect,
    predicate: EffectPredicate,
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

    let Some(remove_mode_idx) = choose_mode
        .modes
        .iter()
        .position(|mode| mode.description.to_ascii_lowercase().starts_with("remove "))
    else {
        effects.push(last_effect);
        return false;
    };

    let mut modes = choose_mode.modes.clone();
    let remove_mode = &mut modes[remove_mode_idx];
    let gate_id = EffectId(1_000_000_000);
    if let Some(last_remove_effect) = remove_mode.effects.pop() {
        remove_mode
            .effects
            .push(Effect::with_id(gate_id.0, last_remove_effect));
        remove_mode
            .effects
            .push(Effect::if_then(gate_id, predicate, vec![modal_effect]));
    } else {
        remove_mode.effects.push(modal_effect);
    }

    effects.push(Effect::new(crate::effects::ChooseModeEffect {
        modes,
        choose_count: choose_mode.choose_count.clone(),
        min_choose_count: choose_mode.min_choose_count.clone(),
        allow_repeated_modes: choose_mode.allow_repeated_modes,
        disallow_previously_chosen_modes: choose_mode.disallow_previously_chosen_modes,
        disallow_previously_chosen_modes_this_turn: choose_mode
            .disallow_previously_chosen_modes_this_turn,
    }));
    true
}

fn finalize_pending_modal(
    mut builder: CardDefinitionBuilder,
    pending_modal: &mut Option<PendingModal>,
) -> CardDefinitionBuilder {
    let Some(pending) = pending_modal.take() else {
        return builder;
    };

    let PendingModal { header, modes } = pending;
    let ModalHeader {
        min: header_min,
        max: header_max,
        same_mode_more_than_once,
        mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn,
        commander_allows_both,
        trigger,
        activated,
        x_replacement: _,
        prefix_effects,
        prefix_choices,
        modal_gate,
        line_text,
    } = header;

    if modes.is_empty() {
        return builder;
    }

    let mode_count = modes.len() as u32;
    let max = header_max.unwrap_or(mode_count).min(mode_count);
    let min = header_min.min(max);
    let with_unchosen_requirement = |effect: Effect| {
        if !mode_must_be_unchosen {
            return effect;
        }
        if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
            let choose_mode = choose_mode.clone();
            let choose_mode = if mode_must_be_unchosen_this_turn {
                choose_mode.with_previously_unchosen_modes_only_this_turn()
            } else {
                choose_mode.with_previously_unchosen_modes_only()
            };
            return Effect::new(choose_mode);
        }
        effect
    };

    let modal_effect = if commander_allows_both {
        let max_both = mode_count.min(2).max(1);
        let choose_both = if max_both == 1 {
            with_unchosen_requirement(Effect::choose_one(modes.clone()))
        } else {
            with_unchosen_requirement(Effect::choose_up_to(max_both, 1, modes.clone()))
        };
        let choose_one = with_unchosen_requirement(Effect::choose_one(modes.clone()));
        Effect::conditional(
            Condition::YouControlCommander,
            vec![choose_both],
            vec![choose_one],
        )
    } else if same_mode_more_than_once && min == max {
        with_unchosen_requirement(Effect::choose_exactly_allow_repeated_modes(max, modes))
    } else if min == 1 && max == 1 {
        with_unchosen_requirement(Effect::choose_one(modes))
    } else if min == max {
        with_unchosen_requirement(Effect::choose_exactly(max, modes))
    } else {
        with_unchosen_requirement(Effect::choose_up_to(max, min, modes))
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
            // Modal branch fused directly into the remove mode.
        } else if let Some(last_effect) = combined_effects.pop() {
            // Use an out-of-band effect id so modal gating does not collide with parser-assigned ids.
            let gate_id = EffectId(1_000_000_000);
            combined_effects.push(Effect::with_id(gate_id.0, last_effect));
            combined_effects.push(Effect::if_then(
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

    if let Some(trigger) = trigger {
        let compiled_trigger = compile_trigger_spec(trigger);
        builder = builder.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: compiled_trigger,
                effects: combined_effects,
                choices: prefix_choices,
                intervening_if: None,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some(line_text),
        });
    } else if let Some(activated) = activated {
        builder = builder.with_ability(Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: activated.mana_cost,
                effects: combined_effects,
                choices: prefix_choices,
                timing: activated.timing,
                additional_restrictions: activated.additional_restrictions,
            }),
            functional_zones: activated.functional_zones,
            text: Some(line_text),
        });
    } else if let Some(ref mut existing) = builder.spell_effect {
        existing.extend(combined_effects);
    } else {
        builder.spell_effect = Some(combined_effects);
    }

    builder
}

fn is_bullet_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('•') || trimmed.starts_with('*') || trimmed.starts_with('-')
}

fn split_sentences_for_parse(line: &str, _line_index: usize) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut paren_depth = 0u32;
    let mut quote_depth = 0u32;

    for ch in line.chars() {
        if ch == '(' {
            paren_depth = paren_depth.saturating_add(1);
            current.push(ch);
            continue;
        }
        if ch == ')' {
            if paren_depth > 0 {
                paren_depth -= 1;
            }
            current.push(ch);
            continue;
        }
        if ch == '"' || ch == '“' || ch == '”' {
            quote_depth = if quote_depth == 0 { 1 } else { 0 };
            current.push(ch);
            continue;
        }
        if ch == '.' && paren_depth == 0 && quote_depth == 0 {
            let sentence = current.trim();
            if !sentence.is_empty() {
                sentences.push(sentence.to_string());
            }
            current.clear();
            continue;
        }
        current.push(ch);
    }

    let sentence = current.trim();
    if !sentence.is_empty() {
        sentences.push(sentence.to_string());
    }

    sentences
}

fn sentence_starts_with_trigger_intro(sentence: &str, line_index: usize) -> bool {
    let tokens = tokenize_line(sentence, line_index);
    // "At the beginning of the next end step, ..." is almost always a delayed trigger created
    // by a prior effect clause, not a new printed triggered ability. Avoid splitting such
    // sentences into their own parse chunk so they can be parsed as delayed effects.
    if looks_like_delayed_next_end_step_intro(&tokens) {
        return false;
    }
    tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
        || is_at_trigger_intro(&tokens, 0)
}

fn looks_like_delayed_next_end_step_intro(tokens: &[Token]) -> bool {
    let mut idx = 0usize;
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return false;
    }
    idx += 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }
    if !tokens
        .get(idx)
        .is_some_and(|token| token.is_word("beginning"))
    {
        return false;
    }
    idx += 1;
    if !tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        return false;
    }
    idx += 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("your")) {
        idx += 1;
    }

    if !tokens.get(idx).is_some_and(|token| token.is_word("next")) {
        return false;
    }
    tokens
        .get(idx + 1)
        .is_some_and(|token| token.is_word("end"))
        && tokens
            .get(idx + 2)
            .is_some_and(|token| token.is_word("step"))
}

fn split_trigger_sentence_chunks(sentences: &[String], line_index: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_starts_with_trigger = false;

    for sentence in sentences {
        let sentence_starts_with_trigger = sentence_starts_with_trigger_intro(sentence, line_index);
        if !current.is_empty() && current_starts_with_trigger && sentence_starts_with_trigger {
            chunks.push(current.join(". "));
            current.clear();
            current_starts_with_trigger = false;
        }
        if current.is_empty() {
            current_starts_with_trigger = sentence_starts_with_trigger;
        }
        current.push(sentence.clone());
    }

    if !current.is_empty() {
        chunks.push(current.join(". "));
    }

    chunks
}

fn queue_restriction(
    restriction: &str,
    line_index: usize,
    pending: &mut PendingRestrictions,
) -> bool {
    let normalized = normalize_restriction_text(restriction);
    if normalized.is_empty() {
        return false;
    }

    let tokens = tokenize_line(&normalized, line_index);
    if is_activate_only_restriction_sentence(&tokens) {
        pending.activation.push(normalized);
        true
    } else if is_trigger_only_restriction_sentence(&tokens) {
        pending.trigger.push(normalized);
        true
    } else {
        false
    }
}

fn extract_parenthetical_restrictions(line: &str) -> Vec<String> {
    let mut restrictions = Vec::new();
    let mut paren_depth = 0u32;
    let mut start = None::<usize>;

    for (byte_idx, ch) in line.char_indices() {
        match ch {
            '(' => {
                if paren_depth == 0 {
                    start = Some(byte_idx + ch.len_utf8());
                }
                paren_depth = paren_depth.saturating_add(1);
            }
            ')' => {
                if paren_depth == 1 {
                    if let Some(start_idx) = start.take() {
                        let inside = &line[start_idx..byte_idx];
                        for sentence in split_sentences_for_parse(inside, 0) {
                            restrictions.push(sentence);
                        }
                    }
                }
                paren_depth = paren_depth.saturating_sub(1);
            }
            _ => {}
        }
    }

    restrictions
        .into_iter()
        .map(|restriction| normalize_restriction_text(&restriction))
        .filter(|restriction| !restriction.is_empty())
        .collect()
}

fn apply_pending_restrictions_to_ability(ability: &mut Ability, pending: &mut PendingRestrictions) {
    let activation_restrictions = std::mem::take(&mut pending.activation);
    let trigger_restrictions = std::mem::take(&mut pending.trigger);

    match &mut ability.kind {
        AbilityKind::Activated(ability) => {
            if activation_restrictions.is_empty() {
                return;
            }
            for restriction in activation_restrictions.iter() {
                apply_pending_activation_restriction(ability, &restriction);
            }
        }
        AbilityKind::Mana(mana_ability) => {
            if activation_restrictions.is_empty() {
                return;
            }
            for restriction in activation_restrictions.iter() {
                apply_pending_mana_restriction(mana_ability, &restriction);
            }
        }
        AbilityKind::Triggered(ability) => {
            if trigger_restrictions.is_empty() {
                return;
            }
            for restriction in trigger_restrictions.iter() {
                apply_pending_trigger_restriction(ability, &restriction);
            }
        }
        _ => {}
    }

    if !activation_restrictions.is_empty() {
        pending.activation.extend(activation_restrictions);
    }
    if !trigger_restrictions.is_empty() {
        pending.trigger.extend(trigger_restrictions);
    }
}

fn apply_instead_followup_statement_to_last_ability(
    builder: &mut CardDefinitionBuilder,
    last_restrictable_ability: Option<usize>,
    effects: &[EffectAst],
    info: &LineInfo,
    allow_unsupported: bool,
    annotations: &mut ParseAnnotations,
) -> Result<bool, CardTextError> {
    let Some(index) = last_restrictable_ability else {
        return Ok(false);
    };
    if index >= builder.abilities.len() {
        return Ok(false);
    }

    let normalized = info.normalized.normalized.as_str().to_ascii_lowercase();
    if !normalized.starts_with("if ") || !normalized.contains(" instead") {
        return Ok(false);
    }

    let compiled = match compile_statement_effects(effects) {
        Ok(compiled) => compiled,
        Err(err) if allow_unsupported => {
            return Err(err);
        }
        Err(_) => return Ok(false),
    };

    if compiled.len() != 1 {
        return Ok(false);
    }

    let Some(replacement) = compiled[0].downcast_ref::<crate::effects::ConditionalEffect>() else {
        return Ok(false);
    };

    if !replacement.if_false.is_empty() {
        return Ok(false);
    }

    collect_tag_spans_from_effects_with_context(effects, annotations, &info.normalized);

    let conditional = replacement.clone();
    match &mut builder.abilities[index].kind {
        AbilityKind::Triggered(ability) => {
            let original = std::mem::take(&mut ability.effects);
            if original.is_empty() {
                return Ok(false);
            }
            ability.effects = vec![Effect::new(crate::effects::ConditionalEffect::new(
                conditional.condition,
                conditional.if_true,
                original,
            ))];
        }
        AbilityKind::Activated(ability) => {
            let original = std::mem::take(&mut ability.effects);
            if original.is_empty() {
                return Ok(false);
            }
            ability.effects = vec![Effect::new(crate::effects::ConditionalEffect::new(
                conditional.condition,
                conditional.if_true,
                original,
            ))];
        }
        AbilityKind::Mana(ability) => {
            let Some(effects) = ability.effects.as_mut() else {
                return Ok(false);
            };

            let original = std::mem::take(effects);
            if original.is_empty() {
                return Ok(false);
            }
            *effects = vec![Effect::new(crate::effects::ConditionalEffect::new(
                conditional.condition,
                conditional.if_true,
                original,
            ))];
        }
        _ => return Ok(false),
    }

    Ok(true)
}

fn is_restrictable_ability(ability: &Ability) -> bool {
    matches!(
        ability.kind,
        AbilityKind::Activated(_) | AbilityKind::Triggered(_) | AbilityKind::Mana(_)
    )
}

fn apply_pending_activation_restriction(
    ability: &mut crate::ability::ActivatedAbility,
    restriction: &str,
) {
    let tokens = tokenize_line(restriction, 0);
    let parsed_timing = parse_activate_only_timing(&tokens);
    let mut timing_applied = false;
    if let Some(parsed_timing) = parsed_timing.as_ref() {
        let merged_timing = merge_activation_timing(&ability.timing, parsed_timing.clone());
        timing_applied = &merged_timing == parsed_timing;
        ability.timing = merged_timing;
    }
    // If timing cannot encode the new restriction (e.g. Equip's built-in sorcery timing
    // combined with "once each turn"), preserve the clause text as an extra restriction.
    let restriction = if parsed_timing.is_some() && !timing_applied {
        Some(normalize_restriction_text(restriction))
    } else {
        normalize_activation_restriction(restriction, parsed_timing.as_ref())
    };
    if let Some(restriction) = restriction {
        ability.additional_restrictions.push(restriction);
    }
}

fn apply_pending_trigger_restriction(ability: &mut TriggeredAbility, restriction: &str) {
    let tokens = tokenize_line(restriction, 0);
    let count = parse_triggered_times_each_turn_from_words(&words(&tokens));
    if let Some(parsed_count) = count {
        ability.intervening_if = Some(match ability.intervening_if.take() {
            Some(crate::ability::InterveningIfCondition::MaxTimesEachTurn(existing)) => {
                crate::ability::InterveningIfCondition::MaxTimesEachTurn(existing.min(parsed_count))
            }
            _ => crate::ability::InterveningIfCondition::MaxTimesEachTurn(parsed_count),
        });
    }
}

fn apply_pending_mana_restriction(ability: &mut crate::ability::ManaAbility, restriction: &str) {
    let normalized_restriction = normalize_restriction_text(restriction);
    if normalized_restriction.is_empty() {
        return;
    }
    let tokens = tokenize_line(&normalized_restriction, 0);
    let parsed_timing = parse_activate_only_timing(&tokens).unwrap_or_default();
    let parsed_condition = parse_activation_condition(&tokens).or_else(|| {
        if parsed_timing == ActivationTiming::AnyTime {
            Some(ManaAbilityCondition::Unmodeled(
                normalized_restriction.clone(),
            ))
        } else {
            None
        }
    });

    if parsed_condition.is_none() && parsed_timing == ActivationTiming::AnyTime {
        return;
    }

    let condition_with_timing = parsed_condition
        .map(|condition| combine_mana_activation_condition(Some(condition), parsed_timing.clone()))
        .unwrap_or_else(|| combine_mana_activation_condition(None, parsed_timing));

    let existing = ability.activation_condition.take();
    ability.activation_condition =
        merge_mana_activation_conditions(existing, condition_with_timing);
}

fn merge_activation_timing(
    existing: &crate::ability::ActivationTiming,
    next: crate::ability::ActivationTiming,
) -> ActivationTiming {
    match (existing, &next) {
        (current, crate::ability::ActivationTiming::AnyTime) => current.clone(),
        (crate::ability::ActivationTiming::AnyTime, _) => next,
        (current, next_timing) if current == next_timing => current.clone(),
        (current, _) => current.clone(),
    }
}

fn normalize_restriction_text(text: &str) -> String {
    text.trim().trim_end_matches('.').trim().to_string()
}

fn normalize_activation_restriction(
    restriction: &str,
    timing: Option<&ActivationTiming>,
) -> Option<String> {
    if timing != Some(&ActivationTiming::OncePerTurn) {
        return Some(restriction.to_string());
    }
    let mut normalized = restriction.to_ascii_lowercase();
    if normalized == "activate only once each turn" {
        return None;
    }
    let prefix = "activate only once each turn and ";
    if normalized.starts_with(prefix) {
        normalized = normalized[prefix.len()..].trim_start().to_string();
    }
    let suffix = " and only once each turn";
    if normalized.ends_with(suffix) {
        normalized = normalized[..normalized.len() - suffix.len()]
            .trim_end()
            .to_string();
    }
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn merge_mana_activation_conditions(
    existing: Option<ManaAbilityCondition>,
    additional: Option<ManaAbilityCondition>,
) -> Option<ManaAbilityCondition> {
    match (existing, additional) {
        (None, None) => None,
        (Some(condition), None) => Some(condition),
        (None, Some(condition)) => Some(condition),
        (Some(left), Some(right)) => Some(ManaAbilityCondition::All(
            flatten_mana_activation_conditions(left)
                .into_iter()
                .chain(flatten_mana_activation_conditions(right))
                .collect(),
        )),
    }
}

fn flatten_mana_activation_conditions(
    condition: ManaAbilityCondition,
) -> Vec<ManaAbilityCondition> {
    match condition {
        ManaAbilityCondition::All(conditions) => conditions,
        condition => vec![condition],
    }
}
