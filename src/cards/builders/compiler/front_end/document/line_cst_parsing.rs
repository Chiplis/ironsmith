use super::super::grammar::structure;
use super::*;

pub(super) fn parse_triggered_line_cst(
    line: &PreprocessedLine,
) -> Result<TriggeredLineCst, CardTextError> {
    let Some(_first_token) = line.tokens.first() else {
        return Err(CardTextError::ParseError(format!(
            "rewrite triggered parser received empty token stream for '{}'",
            line.info.raw_line
        )));
    };
    let Some(_intro) = parse_trigger_intro_tokens(&line.tokens) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite triggered parser expected trigger intro for '{}'",
            line.info.raw_line
        )));
    };
    let (tokens_without_cap, trailing_cap) = strip_trailing_trigger_cap_suffix_tokens(&line.tokens);
    let Some(condition_tokens) = tokens_without_cap.get(1..) else {
        return Err(CardTextError::ParseError(format!(
            "rewrite triggered line is missing trigger body: '{}'",
            line.info.raw_line
        )));
    };
    if condition_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite triggered line is missing trigger body: '{}'",
            line.info.raw_line
        )));
    }
    let normalized = render_token_slice(tokens_without_cap).trim().to_string();
    if let Some(err) = diagnose_known_unsupported_rewrite_line(tokens_without_cap) {
        return Err(err);
    }

    if let Some(nested_trigger_tokens) =
        split_nested_combat_whenever_clause_lexed(tokens_without_cap)
    {
        let nested_line = rewrite_line_tokens(line, nested_trigger_tokens);
        if let Ok(parsed) = parse_triggered_line_cst(&nested_line) {
            return Ok(parsed);
        }
    }

    let mut best_probe_error = None;

    if let Some(spec) = structure::split_triggered_conditional_clause_lexed(tokens_without_cap, 1) {
        let probe = probe_triggered_split(
            spec.trigger_tokens,
            spec.effects_tokens,
            Some(spec.predicate.clone()),
            trailing_cap,
        );
        if let Some(mut parsed) = probe.supported_cst(line, tokens_without_cap) {
            parsed.full_text = normalized.clone();
            return Ok(parsed);
        }
        if best_probe_error.is_none() {
            best_probe_error = probe.preferred_error();
        }
    }

    if let Some((leading_tokens, effect_tokens)) =
        grammar::split_lexed_once_on_comma(tokens_without_cap)
    {
        if leading_tokens.len() > 1 {
            let probe =
                probe_triggered_split(&leading_tokens[1..], effect_tokens, None, trailing_cap);
            if let Some(parsed) = probe.supported_cst(line, tokens_without_cap) {
                return Ok(parsed);
            }
            if best_probe_error.is_none() {
                best_probe_error = probe.preferred_error();
            }
        }
    }

    let whole_line_parse = parse_triggered_line_lexed(tokens_without_cap);
    let mut best_supported_split = None;
    let mut best_fallback_split = None;

    for (separator_idx, separator) in tokens_without_cap.iter().enumerate() {
        if separator.kind != TokenKind::Comma || separator_idx <= 1 {
            continue;
        }

        let probe = probe_triggered_split(
            &tokens_without_cap[1..separator_idx],
            &tokens_without_cap[separator_idx + 1..],
            None,
            trailing_cap,
        );

        if let Some(parsed) = probe.supported_cst(line, tokens_without_cap) {
            // Prefer the split with the most effect tokens (latest separator
            // = largest effects portion). This prevents silent truncation
            // where an early split absorbs most content into the trigger.
            let effect_len = parsed.effect_parse_tokens.len();
            if best_supported_split
                .as_ref()
                .map_or(true, |(_, prev): &(usize, TriggeredLineCst)| {
                    effect_len > prev.effect_parse_tokens.len()
                })
            {
                best_supported_split = Some((separator_idx, parsed));
            }
            continue;
        }

        if best_probe_error.is_none() {
            best_probe_error = probe.preferred_error();
        }

        if whole_line_parse.is_ok() && best_fallback_split.is_none() {
            best_fallback_split = probe.fallback_cst(line, tokens_without_cap);
        }
    }

    if let Some(split) = best_supported_split
        .map(|(_, cst)| cst)
        .or(best_fallback_split)
    {
        // Reject splits where effects cover too little of a multi-sentence
        // line - this catches silent truncation where voting, conditional, or
        // other unsupported clauses are absorbed into the trigger.
        let total_tokens = tokens_without_cap.len();
        let effect_tokens = split.effect_parse_tokens.len();
        let period_count = tokens_without_cap
            .iter()
            .filter(|t| t.kind == TokenKind::Period)
            .count();
        if period_count >= 2 && total_tokens > 15 && effect_tokens * 4 < total_tokens {
            return Err(CardTextError::ParseError(format!(
                "unsupported triggered line: effects cover too few tokens ({effect_tokens}/{total_tokens}), \
                 likely missing unsupported clauses (line: '{}')",
                line.info.raw_line
            )));
        }
        return Ok(split);
    }

    match whole_line_parse {
        Ok(line_ast) => {
            // The whole-line parser found a valid split internally.
            // Apply the same coverage validation: reject if the line has
            // multiple sentences and the effects from the internal split
            // are too small relative to the total.
            let effect_token_count = match &line_ast {
                LineAst::Triggered { effects, .. } => {
                    if effects.is_empty() {
                        0
                    } else {
                        tokens_without_cap.len() / 2
                    }
                }
                _ => tokens_without_cap.len(),
            };
            let period_count = tokens_without_cap
                .iter()
                .filter(|t| t.kind == TokenKind::Period)
                .count();
            let total_tokens = tokens_without_cap.len();
            if period_count >= 2 && total_tokens > 15 && effect_token_count * 4 < total_tokens {
                return Err(CardTextError::ParseError(format!(
                    "unsupported triggered line: whole-line parse covers too little of multi-sentence \
                     ability (line: '{}')",
                    line.info.raw_line
                )));
            }
            Ok(TriggeredLineCst {
                info: line.info.clone(),
                full_text: normalized.to_string(),
                full_parse_tokens: tokens_without_cap.to_vec(),
                trigger_text: render_token_slice(condition_tokens).trim().to_string(),
                trigger_parse_tokens: condition_tokens.to_vec(),
                effect_text: String::new(),
                effect_parse_tokens: Vec::new(),
                max_triggers_per_turn: trailing_cap,
                intervening_if: None,
                chosen_option_label: None,
            })
        }
        Err(err) => Err(best_probe_error.unwrap_or(err)),
    }
}

pub(super) fn parse_static_line_cst(
    line: &PreprocessedLine,
) -> Result<Option<StaticLineCst>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();
    let parse_tokens = rewrite_keyword_dash_parse_tokens(&line.tokens);
    let make_static = |chosen_option_label: Option<String>| StaticLineCst {
        info: line.info.clone(),
        text: normalized.to_string(),
        parse_tokens: parse_tokens.clone(),
        chosen_option_label,
    };
    if matches!(
        normalized,
        "for each {B} in a cost, you may pay 2 life rather than pay that mana."
            | "for each {b} in a cost, you may pay 2 life rather than pay that mana."
            | "as long as trinisphere is untapped, each spell that would cost less than three mana to cast costs three mana to cast."
            | "as long as this is untapped, each spell that would cost less than three mana to cast costs three mana to cast."
            | "players can't pay life or sacrifice nonland permanents to cast spells or activate abilities."
            | "creatures you control can boast twice during each of your turns rather than once."
            | "while voting, you may vote an additional time."
            | "while voting, you get an additional vote."
    ) {
        return Ok(Some(make_static(None)));
    }

    let lexed = &parse_tokens;
    let mut deferred_error = None;

    if grammar::parse_prefix(&lexed, grammar::phrase(&["level", "up"])).is_some() {
        if parse_level_up_line_lexed(&lexed)?.is_some() {
            return Ok(Some(make_static(None)));
        }
    }
    if is_doesnt_untap_during_your_untap_step_line_lexed(&lexed) {
        return Ok(Some(make_static(None)));
    }
    if matches!(
        super::super::grammar::structure::classify_static_line_family_lexed(&lexed),
        Some(super::super::grammar::structure::StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep)
    ) {
        return Ok(Some(make_static(None)));
    }

    if parse_if_this_spell_costs_less_to_cast_line_lexed(&lexed)?.is_some() {
        return Ok(Some(make_static(None)));
    }

    if is_activate_only_once_each_turn_line_lexed(&lexed) {
        return Ok(Some(make_static(None)));
    }

    if split_compound_buff_and_unblockable_sentence(&lexed).is_some() {
        return Ok(Some(make_static(None)));
    }

    if !should_skip_keyword_action_static_probe(&lexed)
        && let Some(_actions) = parse_ability_line_lexed(&lexed)
    {
        return Ok(Some(make_static(None)));
    }

    match parse_static_ability_ast_line_lexed(&lexed) {
        Ok(Some(_abilities)) => {
            return Ok(Some(make_static(None)));
        }
        Ok(None) => {}
        Err(err) => deferred_error = Some(err),
    }

    if parse_split_static_item_count(&lexed)?.is_some() {
        return Ok(Some(make_static(None)));
    }

    if let Some(err) = deferred_error {
        return Err(err);
    }

    Ok(None)
}

fn parse_split_static_item_count(tokens: &[OwnedLexToken]) -> Result<Option<usize>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() <= 1 {
        return Ok(None);
    }

    let mut item_count = 0usize;
    for sentence in sentences {
        if parse_if_this_spell_costs_less_to_cast_line_lexed(sentence)?.is_some() {
            item_count += 1;
            continue;
        }
        if let Some(actions) = parse_ability_line_lexed(sentence) {
            item_count += actions.len();
            continue;
        }
        let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? else {
            return Ok(None);
        };
        item_count += abilities.len();
    }

    Ok(Some(item_count))
}

pub(super) fn strict_unsupported_triggered_line_error(
    raw_line: &str,
    err: Option<CardTextError>,
) -> CardTextError {
    match err {
        Some(CardTextError::ParseError(message))
            if str_contains(message.as_str(), "unsupported trigger clause") =>
        {
            CardTextError::ParseError(format!("unsupported triggered line: '{raw_line}'"))
        }
        Some(err) => err,
        None => CardTextError::ParseError(format!("unsupported triggered line: '{raw_line}'")),
    }
}

pub(super) fn parse_level_item_cst(
    line: &PreprocessedLine,
) -> Result<Option<LevelItemCst>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();

    if !should_skip_keyword_action_static_probe(&line.tokens)
        && let Some(actions) = parse_ability_line_lexed(&line.tokens)
    {
        return Ok(Some(LevelItemCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            kind: LevelItemKindCst::KeywordActions,
            parsed: ParsedLevelAbilityItemAst::KeywordActions(actions),
        }));
    }

    if let Some(abilities) = parse_static_ability_ast_line_lexed(&line.tokens)? {
        return Ok(Some(LevelItemCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            kind: LevelItemKindCst::StaticAbilities,
            parsed: ParsedLevelAbilityItemAst::StaticAbilities(abilities),
        }));
    }

    Ok(None)
}

pub(super) fn parse_modal_mode_cst(line: &PreprocessedLine) -> Result<ModalModeCst, CardTextError> {
    let raw_mode = line
        .info
        .raw_line
        .trim_start()
        .trim_start_matches(|c: char| c == '•' || c == '*' || c == '-')
        .trim();
    let parse_tokens = strip_non_keyword_label_prefix_lexed(&line.tokens);
    let mode_text = strip_non_keyword_label_prefix(raw_mode).trim().to_string();
    let effects_ast = parse_effect_sentences_lexed(parse_tokens)?;
    Ok(ModalModeCst {
        info: line.info.clone(),
        text: mode_text,
        effects_ast,
    })
}

pub(super) fn parse_saga_chapter_line_cst(
    line: &PreprocessedLine,
    chapters: Vec<u32>,
    text: &str,
) -> Result<SagaChapterLineCst, CardTextError> {
    let parse_tokens = lexed_tokens(text, line.info.line_index)?;
    let effects_ast = parse_effect_sentences_lexed(&parse_tokens)?;
    Ok(SagaChapterLineCst {
        info: line.info.clone(),
        chapters,
        text: text.to_string(),
        effects_ast,
    })
}
