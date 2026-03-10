use crate::cards::builders::effect_ast_traversal::try_for_each_nested_effects_mut;
use crate::cards::builders::parse_parsing::{
    infer_activated_functional_zones, is_activate_only_restriction_sentence, is_at_trigger_intro,
    is_ignorable_unparsed_line, is_trigger_only_restriction_sentence, parse_ability_line,
    parse_activation_cost, parse_effect_sentences, parse_loyalty_shorthand_activation_cost,
    parse_metadata_line, parse_single_word_keyword_action, parse_static_ability_ast_line,
    parse_trigger_clause, reject_unimplemented_keyword_actions, split_on_period,
    starts_with_activation_cost, token_index_for_word_index, tokenize_line, trim_commas, words,
};
use crate::cards::builders::{
    ActivationTiming, CardDefinitionBuilder, CardTextError, EffectAst, EffectPredicate,
    IfResultPredicate, LineAst, LineInfo, ParseAnnotations, ParsedCardAst, ParsedCardItem,
    ParsedLevelAbilityAst, ParsedLevelAbilityItemAst, ParsedLineAst, ParsedModalActivatedHeader,
    ParsedModalAst, ParsedModalGate, ParsedModalHeader, ParsedModalModeAst, ParsedRestrictions,
    TextSpan, Token, collect_tag_spans_from_effects_with_context, collect_tag_spans_from_line,
    find_activation_cost_start, normalize_line_for_parse, parse_if_result_predicate,
    parse_level_header, parse_line, parse_number_or_x_value, parse_power_toughness,
    parse_where_x_value_clause, replace_unbound_x_with_value, value_contains_unbound_x,
};
use crate::effect::Value;
use crate::static_abilities::StaticAbility;
use crate::{PlayerFilter, PtValue};

type ModalHeader = ParsedModalHeader;
type ModalActivatedHeader = ParsedModalActivatedHeader;
type ModalGate = ParsedModalGate;
type PendingRestrictions = ParsedRestrictions;

pub(super) fn parse_card_ast_with_annotations(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<ParsedCardAst, CardTextError> {
    let (builder, mut annotations, line_infos) = collect_line_infos(builder, text.as_str())?;
    let mut items = Vec::new();
    let mut pending_modal: Option<ParsedModalAst> = None;

    let mut idx = 0usize;
    while idx < line_infos.len() {
        let info = &line_infos[idx];

        if let Some(reason) = strict_unsupported_line_reason(
            info.raw_line.as_str(),
            info.normalized.normalized.as_str(),
        ) {
            let err =
                CardTextError::ParseError(format!("{reason} (line: '{}')", info.raw_line.trim()));
            if allow_unsupported {
                items.push(ParsedCardItem::Line(ParsedLineAst {
                    info: info.clone(),
                    chunks: vec![unsupported_line_ast(info, reason.to_string())],
                    restrictions: ParsedRestrictions::default(),
                }));
                idx += 1;
                continue;
            }
            return Err(err);
        }

        if !allow_unsupported
            && let Some((min_level, max_level)) = parse_level_header(&info.normalized.normalized)
        {
            let (level, next_idx) =
                parse_level_ability_ast(&line_infos, idx, info, min_level, max_level)?;
            items.push(ParsedCardItem::LevelAbility(level));
            idx = next_idx;
            continue;
        }

        if let Some(pending) = pending_modal.as_mut() {
            if is_bullet_line(info.raw_line.as_str()) {
                let tokens = tokenize_line(&info.normalized.normalized, info.line_index);
                let mut effects_ast = match parse_effect_sentences(&tokens) {
                    Ok(effects_ast) => effects_ast,
                    Err(err) if allow_unsupported => {
                        items.push(ParsedCardItem::Line(ParsedLineAst {
                            info: info.clone(),
                            chunks: vec![unsupported_line_ast(info, format!("{err:?}"))],
                            restrictions: ParsedRestrictions::default(),
                        }));
                        idx += 1;
                        continue;
                    }
                    Err(err) => return Err(err),
                };
                if effects_ast.is_empty() {
                    if allow_unsupported {
                        items.push(ParsedCardItem::Line(ParsedLineAst {
                            info: info.clone(),
                            chunks: vec![unsupported_line_ast(
                                info,
                                "modal bullet line produced no effects".to_string(),
                            )],
                            restrictions: ParsedRestrictions::default(),
                        }));
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
                        items.push(ParsedCardItem::Line(ParsedLineAst {
                            info: info.clone(),
                            chunks: vec![unsupported_line_ast(info, format!("{err:?}"))],
                            restrictions: ParsedRestrictions::default(),
                        }));
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
                let description = info
                    .raw_line
                    .trim_start()
                    .trim_start_matches(|c: char| c == '•' || c == '*' || c == '-')
                    .trim()
                    .to_string();
                pending.modes.push(ParsedModalModeAst {
                    info: info.clone(),
                    description,
                    effects_ast,
                });
                idx += 1;
                continue;
            }

            items.push(ParsedCardItem::Modal(
                pending_modal
                    .take()
                    .expect("pending modal must exist while parsing bullet block"),
            ));
            continue;
        }

        let next_is_bullet = line_infos
            .get(idx + 1)
            .is_some_and(|next| is_bullet_line(next.raw_line.as_str()));
        if next_is_bullet {
            match parse_modal_header(info) {
                Ok(Some(header)) => {
                    pending_modal = Some(ParsedModalAst {
                        header,
                        modes: Vec::new(),
                    });
                    idx += 1;
                    continue;
                }
                Ok(None) => {}
                Err(err) if allow_unsupported => {
                    items.push(ParsedCardItem::Line(ParsedLineAst {
                        info: info.clone(),
                        chunks: vec![unsupported_line_ast(info, format!("{err:?}"))],
                        restrictions: ParsedRestrictions::default(),
                    }));
                    idx += 1;
                    continue;
                }
                Err(err) => return Err(err),
            }
        }

        let line_sentences =
            split_sentences_for_parse(&info.normalized.normalized, info.line_index);
        let mut restrictions = ParsedRestrictions::default();
        let mut parsed_portion = Vec::new();
        for sentence in line_sentences {
            if sentence.is_empty() {
                continue;
            }

            if queue_restriction(&sentence, info.line_index, &mut restrictions) {
                continue;
            }

            parsed_portion.push(sentence);
        }

        for restriction in extract_parenthetical_restrictions(&info.raw_line) {
            let _ = queue_restriction(&restriction, info.line_index, &mut restrictions);
        }

        let mut chunks = Vec::new();
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
                        unsupported_line_ast(info, short_reason)
                    }
                    Err(err) => return Err(err),
                };

                collect_tag_spans_from_line(&parsed, &mut annotations, &info.normalized);
                chunks.push(parsed);
            }
        }

        if !chunks.is_empty()
            || !restrictions.activation.is_empty()
            || !restrictions.trigger.is_empty()
        {
            items.push(ParsedCardItem::Line(ParsedLineAst {
                info: info.clone(),
                chunks,
                restrictions,
            }));
        }

        idx += 1;
    }

    if let Some(pending) = pending_modal.take() {
        items.push(ParsedCardItem::Modal(pending));
    }

    Ok(ParsedCardAst {
        builder,
        annotations,
        items,
        allow_unsupported,
    })
}

fn strict_unsupported_line_reason<'a>(
    raw_line: &'a str,
    normalized_line: &'a str,
) -> Option<&'static str> {
    let raw_lower = raw_line.to_ascii_lowercase();

    if normalized_line.contains("put one of them into your hand and the rest into your graveyard") {
        return Some("unsupported multi-destination put clause");
    }

    if normalized_line.contains("destroy target face-down creature")
        || normalized_line.contains("destroy target facedown creature")
    {
        return Some("unsupported face-down clause");
    }

    if normalized_line.contains("defending player's choice")
        || normalized_line.contains("defending players choice")
        || normalized_line.contains("defending player s choice")
    {
        return Some("unsupported defending-players-choice clause");
    }

    if normalized_line.contains("isn't saddled this turn")
        || normalized_line.contains("isnt saddled this turn")
    {
        return Some("unsupported saddled-state tail");
    }

    if normalized_line.contains("into your hand this turn") {
        return Some("unsupported looked-card fallback timing tail");
    }

    if normalized_line.contains("was a hamster this turn") {
        return Some("unsupported sacrificed-creature predicate tail");
    }

    if normalized_line.contains("it's not a creature")
        || normalized_line.contains("its not a creature")
        || raw_lower.contains("(it's not a creature.)")
        || raw_lower.contains("(its not a creature.)")
    {
        return Some("unsupported type-removal clause");
    }

    None
}

fn parse_level_ability_ast(
    line_infos: &[LineInfo],
    header_idx: usize,
    _header_info: &LineInfo,
    min_level: u32,
    max_level: Option<u32>,
) -> Result<(ParsedLevelAbilityAst, usize), CardTextError> {
    let mut level = ParsedLevelAbilityAst {
        min_level,
        max_level,
        pt: None,
        items: Vec::new(),
    };
    let mut idx = header_idx + 1;

    while idx < line_infos.len() {
        let next = &line_infos[idx];
        if parse_level_header(&next.normalized.normalized).is_some() {
            break;
        }

        let normalized_line = next.normalized.normalized.as_str();
        if let Some(pt) = parse_power_toughness(normalized_line) {
            if let (PtValue::Fixed(power), PtValue::Fixed(toughness)) = (pt.power, pt.toughness) {
                level.pt = Some((power, toughness));
            }
            idx += 1;
            continue;
        }

        let tokens = tokenize_line(normalized_line, next.line_index);
        if let Some(actions) = parse_ability_line(&tokens) {
            reject_unimplemented_keyword_actions(&actions, next.raw_line.as_str())?;
            level
                .items
                .push(ParsedLevelAbilityItemAst::KeywordActions(actions));
            idx += 1;
            continue;
        }

        if let Some(abilities) = parse_static_ability_ast_line(&tokens)? {
            level
                .items
                .push(ParsedLevelAbilityItemAst::StaticAbilities(abilities));
            idx += 1;
            continue;
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported level ability line: '{}'",
            next.raw_line
        )));
    }

    Ok((level, idx))
}

fn unsupported_line_ast(info: &LineInfo, reason: String) -> LineAst {
    let marker = StaticAbility::unsupported_parser_line(info.raw_line.trim(), reason);
    LineAst::StaticAbility(marker.into())
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

    fn short_name_for_self_reference(name: &str) -> String {
        fn is_reserved_short_alias(alias_lower: &str) -> bool {
            matches!(
                alias_lower,
                "a" | "an"
                    | "the"
                    | "one"
                    | "two"
                    | "three"
                    | "four"
                    | "five"
                    | "six"
                    | "seven"
                    | "eight"
                    | "nine"
                    | "ten"
                    | "x"
                    | "this"
                    | "that"
                    | "these"
                    | "those"
                    | "you"
                    | "your"
                    | "when"
                    | "whenever"
                    | "if"
                    | "at"
                    | "add"
                    | "move"
                    | "deal"
                    | "draw"
                    | "counter"
                    | "destroy"
                    | "exile"
                    | "untap"
                    | "scry"
                    | "discard"
                    | "transform"
                    | "regenerate"
                    | "mill"
                    | "get"
                    | "reveal"
                    | "look"
                    | "lose"
                    | "gain"
                    | "put"
                    | "sacrifice"
                    | "create"
                    | "investigate"
                    | "attach"
                    | "remove"
                    | "return"
                    | "exchange"
                    | "become"
                    | "switch"
                    | "skip"
                    | "surveil"
                    | "shuffle"
                    | "reorder"
                    | "pay"
                    | "goad"
                    | "power"
                    | "toughness"
                    | "mana"
                    | "life"
                    | "commander"
                    | "player"
                    | "opponent"
                    | "creature"
                    | "artifact"
                    | "enchantment"
                    | "land"
                    | "spell"
                    | "card"
                    | "token"
                    | "permanent"
                    | "library"
                    | "graveyard"
                    | "hand"
                    | "battlefield"
                    | "controller"
                    | "owner"
                    | "planeswalker"
                    | "battle"
                    | "equipment"
                    | "aura"
            ) || parse_single_word_keyword_action(alias_lower).is_some()
        }

        let trimmed = name.trim();
        let comma_short = trimmed.split(',').next().unwrap_or(trimmed).trim();
        if comma_short != trimmed {
            return comma_short.to_string();
        }

        let mut words = trimmed.split_whitespace();
        let Some(first_word) = words.next() else {
            return trimmed.to_string();
        };
        if words.next().is_none() {
            return trimmed.to_string();
        }

        let alias = first_word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-');
        if alias.len() <= 2 {
            return trimmed.to_string();
        }

        let alias_lower = alias.to_ascii_lowercase();
        if is_reserved_short_alias(alias_lower.as_str()) {
            return trimmed.to_string();
        }

        alias.to_string()
    }

    let card_name = builder.card_builder.name_ref().to_string();
    let front_face_name = card_name
        .split("//")
        .next()
        .unwrap_or(card_name.as_str())
        .trim()
        .to_string();
    let short_name = short_name_for_self_reference(front_face_name.as_str());
    let full_lower = normalize_card_name_for_self_reference(front_face_name.as_str());
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
        ". this ability costs ",
        ".this ability costs ",
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

    for marker in [". if ", ".if "] {
        if let Some(idx) = lower.find(marker) {
            let first = line[..=idx].trim();
            let first_lower = first.to_ascii_lowercase();
            let first_is_self_etb_counter_clause =
                first_lower.contains(" enters with ") && first_lower.contains(" counter");
            if !first_is_self_etb_counter_clause {
                continue;
            }

            let second = line[idx + 1..].trim();
            let second_lower = second.to_ascii_lowercase();
            if second_lower.starts_with("if ")
                && second_lower.contains(" enters with an additional ")
            {
                if !first.is_empty() && !second.is_empty() {
                    if let Some(comma_idx) = second.find(',')
                        && comma_idx > 3
                    {
                        let condition = second[3..comma_idx].trim();
                        let rest = second[comma_idx + 1..].trim().trim_end_matches('.').trim();
                        if !condition.is_empty() && !rest.is_empty() {
                            return vec![first.to_string(), format!("{rest} if {condition}.")];
                        }
                    }
                    return vec![first.to_string(), second.to_string()];
                }
            }
        }
    }
    vec![line.to_string()]
}

fn parse_modal_header(info: &LineInfo) -> Result<Option<ModalHeader>, CardTextError> {
    let tokens = tokenize_line(&info.normalized.normalized, info.line_index);
    let token_words = words(&tokens);
    let Some(choose_idx) = tokens.iter().position(|token| token.is_word("choose")) else {
        return Ok(None);
    };

    let mut min: Option<Value> = None;
    let mut max: Option<Value> = None;
    let choose_tokens = &tokens[choose_idx + 1..];
    if choose_tokens.len() >= 3
        && choose_tokens[0].is_word("one")
        && choose_tokens[1].is_word("or")
        && choose_tokens[2].is_word("more")
    {
        min = Some(Value::Fixed(1));
        max = None;
    } else if choose_tokens.len() >= 3
        && choose_tokens[0].is_word("one")
        && choose_tokens[1].is_word("or")
        && choose_tokens[2].is_word("both")
    {
        min = Some(Value::Fixed(1));
        max = Some(Value::Fixed(2));
    } else if choose_tokens.len() >= 2
        && choose_tokens[0].is_word("up")
        && choose_tokens[1].is_word("to")
    {
        if let Some((value, _)) = parse_number_or_x_value(&choose_tokens[2..]) {
            min = Some(Value::Fixed(0));
            max = Some(value);
        }
    } else if let Some((value, _)) = parse_number_or_x_value(choose_tokens) {
        min = Some(value.clone());
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
        let loyalty_shorthand_cost =
            parse_loyalty_shorthand_activation_cost(cost_region, Some(info.raw_line.as_str()));
        if let Some(cost_start) = find_activation_cost_start(cost_region)
            .or_else(|| loyalty_shorthand_cost.as_ref().map(|_| 0))
        {
            let cost_tokens = &cost_region[cost_start..];
            if !cost_tokens.is_empty()
                && (starts_with_activation_cost(cost_tokens) || loyalty_shorthand_cost.is_some())
            {
                let (mana_cost, cost_effects) = if let Some(cost) = &loyalty_shorthand_cost {
                    (cost.clone(), Vec::new())
                } else {
                    parse_activation_cost(cost_tokens)?
                };
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
                    timing: if loyalty_shorthand_cost.is_some() {
                        ActivationTiming::SorcerySpeed
                    } else {
                        ActivationTiming::AnyTime
                    },
                    additional_restrictions: if loyalty_shorthand_cost.is_some() {
                        vec!["Activate only once each turn.".to_string()]
                    } else {
                        Vec::new()
                    },
                    activation_restrictions: Vec::new(),
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
        prefix_effects_ast,
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
        | EffectAst::Monstrosity { amount, .. } => {
            replace_modal_header_x_in_value(amount, replacement, clause)?;
        }
        EffectAst::CreateTokenWithMods {
            count: amount,
            dynamic_power_toughness,
            ..
        } => {
            replace_modal_header_x_in_value(amount, replacement, clause)?;
            if let Some((power, toughness)) = dynamic_power_toughness {
                replace_modal_header_x_in_value(power, replacement, clause)?;
                replace_modal_header_x_in_value(toughness, replacement, clause)?;
            }
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
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_modal_header_x_in_effects_ast(nested, replacement, clause)
            })?;
        }
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
    // "At the beginning of the next end step, ..." and
    // "At the beginning of your next upkeep, ..." are almost always delayed
    // triggers created by a prior effect clause, not new printed triggered
    // abilities. Avoid splitting such sentences into their own parse chunk so
    // they can be parsed as delayed effects.
    if looks_like_delayed_next_turn_intro(&tokens) {
        return false;
    }
    // "When one or more ... this way, ..." is usually a follow-up gate tied to the
    // previous sentence's effect result, not a new printed triggered ability.
    if looks_like_when_one_or_more_this_way_followup(&tokens) {
        return false;
    }
    // "When you do, ..." is usually a follow-up to the immediately previous optional/conditional
    // action in the same ability sentence, not a standalone trigger.
    if looks_like_when_you_do_followup(&tokens) {
        return false;
    }
    tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
        || is_at_trigger_intro(&tokens, 0)
}

fn looks_like_delayed_next_turn_intro(tokens: &[Token]) -> bool {
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

    if tokens
        .get(idx + 1)
        .is_some_and(|token| token.is_word("end"))
        && tokens
            .get(idx + 2)
            .is_some_and(|token| token.is_word("step"))
    {
        return true;
    }

    tokens
        .get(idx + 1)
        .is_some_and(|token| token.is_word("upkeep"))
}

fn looks_like_when_one_or_more_this_way_followup(tokens: &[Token]) -> bool {
    let clause_words = words(tokens);
    (clause_words.starts_with(&["when", "one", "or", "more"])
        || clause_words.starts_with(&["whenever", "one", "or", "more"]))
        && clause_words
            .windows(2)
            .any(|window| window == ["this", "way"])
}

fn looks_like_when_you_do_followup(tokens: &[Token]) -> bool {
    let clause_words = words(tokens);
    clause_words.starts_with(&["when", "you", "do"])
        || clause_words.starts_with(&["whenever", "you", "do"])
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

fn normalize_restriction_text(text: &str) -> String {
    text.trim().trim_end_matches('.').trim().to_string()
}
