use super::*;

pub(super) fn try_parse_level_header_block(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
) -> Result<Option<(RecognizedLine, usize)>, CardTextError> {
    let Some((min_level, max_level)) = parse_level_header_tokens(&line.tokens) else {
        return Ok(None);
    };

    let mut pt = None;
    let mut items = Vec::new();
    let mut probe_idx = idx + 1;
    while let Some(PreprocessedItem::Line(next_line)) = preprocessed.items.get(probe_idx) {
        if parse_level_header_tokens(&next_line.tokens).is_some() {
            break;
        }
        if parse_saga_chapter_prefix_tokens(&next_line.tokens).is_some() {
            break;
        }
        if let Some(parsed_pt) = parse_power_toughness(&next_line.info.normalized.normalized)
            && let (PtValue::Fixed(power), PtValue::Fixed(toughness)) =
                (parsed_pt.power, parsed_pt.toughness)
        {
            pt = Some((power, toughness));
            probe_idx += 1;
            continue;
        }
        match recognize_level_item(&preprocessed.card, next_line) {
            Ok(Some(item)) => {
                items.push(item);
                probe_idx += 1;
            }
            Ok(None) => {
                if allow_unsupported {
                    break;
                }
                return Err(CardTextError::ParseError(format!(
                    "unsupported level ability line: '{}'",
                    next_line.info.raw_line
                )));
            }
            Err(_) if allow_unsupported => break,
            Err(err) => return Err(err),
        }
    }

    if pt.is_none() && items.is_empty() && preprocessed.items.get(idx + 1).is_some() {
        if allow_unsupported {
            return Ok(Some((
                RecognizedLine::Unsupported(RecognizedUnsupportedLine {
                    info: line.info.clone(),
                    reason_code: "level-header-not-yet-supported",
                }),
                idx + 1,
            )));
        }
        return Err(CardTextError::ParseError(format!(
            "parser does not yet support level header: '{}'",
            line.info.raw_line
        )));
    }

    Ok(Some((
        RecognizedLine::LevelHeader(RecognizedLevelHeader {
            min_level,
            max_level,
            pt,
            items,
        }),
        probe_idx,
    )))
}

/// "Tiered (Choose one additional cost.)" whose shared instruction sits on its
/// own line before the tier bullets, each bullet naming a cost and only the
/// characteristic the instruction leaves open ("Until end of turn, target
/// creature you control ... has the chosen base power and toughness." /
/// "• Galian Beast — {0} — 3/2."; Vincent's Limit Break, CR 702.183): each
/// tier is the shared instruction with its own base power and toughness, and
/// carries its cost as the tier's additional mana cost.
fn try_parse_tiered_shared_body_block(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    normalized_header: &str,
) -> Result<Option<(RecognizedLine, usize)>, CardTextError> {
    if !crate::string_primitives::starts_with(normalized_header, "tiered") {
        return Ok(None);
    }
    let (Some(PreprocessedItem::Line(body)), Some(PreprocessedItem::Line(first_tier))) =
        (preprocessed.items.get(idx + 1), preprocessed.items.get(idx + 2))
    else {
        return Ok(None);
    };
    if is_bullet_line(body) || !is_bullet_line(first_tier) {
        return Ok(None);
    }
    const OPEN: [&str; 6] = ["the", "chosen", "base", "power", "and", "toughness"];
    let Some(open_at) = body.tokens.windows(OPEN.len()).position(|window| {
        window.iter().zip(OPEN).all(|(token, word)| token.is_word(word))
    }) else {
        return Ok(None);
    };
    let mut modes = Vec::new();
    let mut probe_idx = idx + 2;
    while let Some(PreprocessedItem::Line(tier)) = preprocessed.items.get(probe_idx) {
        if !is_bullet_line(tier) {
            break;
        }
        let Some((cost, body_first)) = parse_tiered_mode_prefix_tokens(&tier.tokens) else {
            return Ok(None);
        };
        let pt_words = crate::lexer::parser_token_word_refs(
            crate::util::trim_edge_punctuation_tokens(tier.tokens.get(body_first..).unwrap_or_default()),
        );
        let [pt] = pt_words.as_slice() else {
            return Ok(None);
        };
        if !pt.contains('/') {
            return Ok(None);
        }
        let mut tokens = body.tokens[..open_at].to_vec();
        tokens.extend(crate::lexer::synthetic_word_tokens(["base", "power", "and", "toughness"]));
        tokens.extend_from_slice(crate::util::trim_edge_punctuation_tokens(&tier.tokens[body_first..]));
        tokens.extend_from_slice(&body.tokens[open_at + OPEN.len()..]);
        let effects_ast = parse_effect_sentences_lexed(&tokens)?;
        if effects_ast.is_empty() {
            return Ok(None);
        }
        let surface = strip_modal_bullet_prefix_tokens(&tier.tokens);
        let text = render_original_text_for_token_slice(tier, surface)
            .unwrap_or_else(|| render_token_slice(surface))
            .trim()
            .to_string();
        modes.push(RecognizedModalMode {
            info: tier.info.clone(),
            text,
            point_cost: None,
            additional_mana_cost: Some(cost),
            effects_ast,
        });
        probe_idx += 1;
    }
    if modes.is_empty() {
        return Ok(None);
    }
    Ok(Some((
        RecognizedLine::Modal(RecognizedModalBlock {
            header: line.info.clone(),
            header_tokens: line.tokens.clone(),
            modes,
            saga_chapters: None,
        }),
        probe_idx,
    )))
}

/// "{2}, {T}, Choose flying, first strike, trample, or shadow: Until end of
/// turn, target creature with the chosen ability loses it and another target
/// creature gains it." (Phyrexian Splicer): the keyword is chosen as the
/// ability is activated, before its targets (CR 602.2b), and the targets
/// depend on it. That is exactly a modal activation whose modes are the
/// instruction read once per listed keyword (CR 700.2), so each mode
/// substitutes its keyword for "the chosen ability" and for "it" after
/// "loses"/"gains".
fn try_parse_chosen_keyword_cost_block(
    line: &PreprocessedLine,
) -> Result<Option<(RecognizedLine, usize)>, CardTextError> {
    let tokens = &line.tokens;
    let Some(colon) = tokens.iter().position(|token| token.kind == TokenKind::Colon) else {
        return Ok(None);
    };
    if tokens.iter().any(|token| token.kind == TokenKind::Quote) {
        return Ok(None);
    }
    let cost = &tokens[..colon];
    let Some(choose_idx) = cost.iter().rposition(|token| token.is_word("choose")) else {
        return Ok(None);
    };
    if choose_idx == 0 || !cost[choose_idx - 1].is_comma() {
        return Ok(None);
    }
    let options = &cost[choose_idx + 1..];
    let mut keywords: Vec<&[OwnedLexToken]> = Vec::new();
    for option in options.split(|token| token.is_comma() || token.is_word("or")) {
        if option.is_empty() {
            continue;
        }
        if crate::keyword_static::parse_ability_line(option).is_none_or(|actions| actions.len() != 1)
        {
            return Ok(None);
        }
        keywords.push(option);
    }
    if keywords.len() < 2 {
        return Ok(None);
    }
    let body = &tokens[colon + 1..];
    let chosen_at = body.windows(3).position(|window| {
        window[0].is_word("the") && window[1].is_word("chosen") && window[2].is_word("ability")
    });
    if chosen_at.is_none() {
        return Ok(None);
    }
    let mut modes = Vec::with_capacity(keywords.len());
    for keyword in &keywords {
        let mut substituted = Vec::with_capacity(body.len() + keyword.len() * 3);
        let mut index = 0;
        while index < body.len() {
            if index + 2 < body.len()
                && body[index].is_word("the")
                && body[index + 1].is_word("chosen")
                && body[index + 2].is_word("ability")
            {
                substituted.extend_from_slice(keyword);
                index += 3;
                continue;
            }
            if body[index].is_word("it")
                && index > 0
                && body[index - 1].is_any_word(&["loses", "lose", "gains", "gain"])
            {
                substituted.extend_from_slice(keyword);
                index += 1;
                continue;
            }
            substituted.push(body[index].clone());
            index += 1;
        }
        let effects_ast = parse_effect_sentences_lexed(&substituted)?;
        if effects_ast.is_empty() {
            return Ok(None);
        }
        let text = render_token_slice(&substituted).trim().to_string();
        let mut chars = text.chars();
        let text = match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => text,
        };
        modes.push(RecognizedModalMode {
            info: line.info.clone(),
            text,
            point_cost: None,
            additional_mana_cost: None,
            effects_ast,
        });
    }
    let mut header_tokens = cost[..choose_idx - 1].to_vec();
    header_tokens.push(tokens[colon].clone());
    header_tokens.extend(crate::lexer::synthetic_word_tokens(["choose", "one"]));
    header_tokens.push(OwnedLexToken::new(TokenKind::EmDash, "—", tokens[colon].span()));
    Ok(Some((
        RecognizedLine::Modal(RecognizedModalBlock {
            header: line.info.clone(),
            header_tokens,
            modes,
            saga_chapters: None,
        }),
        1,
    )))
}

pub(super) fn try_parse_modal_bullet_block(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
) -> Result<Option<(RecognizedLine, usize)>, CardTextError> {
    if is_bullet_line(line) {
        return Ok(None);
    }
    if let Some((block, consumed)) = try_parse_chosen_keyword_cost_block(line)? {
        return Ok(Some((block, idx + consumed)));
    }
    if is_named_option_as_enters_choice_header(line) {
        return Ok(None);
    }

    let normalized_header = line.info.raw_line.trim_start().to_ascii_lowercase();
    if let Some(block) = try_parse_tiered_shared_body_block(preprocessed, idx, line, &normalized_header)? {
        return Ok(Some(block));
    }
    let spree_header = crate::string_primitives::starts_with(&normalized_header, "spree");
    let next_line_is_mode = preprocessed
        .items
        .get(idx + 1)
        .is_some_and(|item| match item {
            PreprocessedItem::Line(next_line) => {
                is_bullet_line(next_line)
                    || (spree_header
                        && next_line
                            .tokens
                            .first()
                            .is_some_and(|token| token.kind == TokenKind::Plus))
            }
            PreprocessedItem::Metadata(_) => false,
        });
    if !next_line_is_mode {
        return Ok(None);
    }

    // Parsing a modal header can probe a common action suffix. Only perform
    // that work after the following physical line proves this is a modal
    // block; ordinary multi-sentence abilities may contain the same choice
    // words and internal commas but have no bullet modes to inherit a suffix.
    // "When Venser enters, choose one —": a triggered modal header names its
    // source the same way an ordinary triggered line does. Rewrite the
    // source alias to the typed self subject here too, or a short name that
    // is also a subtype ("Venser") is read as a subtype-filtered trigger on
    // other objects.
    let header_tokens = if line
        .tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        super::named_source_tokens::normalize_named_source_trigger_tokens(
            &preprocessed.card,
            &line.tokens,
        )
        .unwrap_or_else(|| line.tokens.clone())
    } else if let Some(colon) = line.tokens.iter().position(|token| token.kind == TokenKind::Colon)
        && !line.tokens[..colon].iter().any(|token| token.kind == TokenKind::Quote)
    {
        // Direct modal activation costs have the same self-reference scope as
        // ordinary activation costs. Normalize only the cost prefix: names in
        // effect/target clauses and quoted granted abilities have other scopes.
        let mut tokens = super::normalize_named_source_tokens_for_builder(
            &preprocessed.card,
            &line.tokens[..colon],
        )
        .unwrap_or_else(|| line.tokens[..colon].to_vec());
        tokens.extend_from_slice(&line.tokens[colon..]);
        tokens
    } else {
        line.tokens.clone()
    };
    let header_has_common_target_suffix =
        super::super::modal_support::parse_modal_header(&line.info, &header_tokens)?
            .is_some_and(|header| !header.common_suffix_effects_ast.is_empty());
    let characteristics_template =
        super::characteristic_modes::characteristics_template(&header_tokens);
    let mut bullet_modes = Vec::new();
    let mut probe_idx = idx + 1;
    while let Some(PreprocessedItem::Line(next_line)) = preprocessed.items.get(probe_idx) {
        let is_spree_mode = spree_header
            && next_line
                .tokens
                .first()
                .is_some_and(|token| token.kind == TokenKind::Plus);
        if !is_bullet_line(next_line) && !is_spree_mode {
            break;
        }
        if let Some(template) = characteristics_template.as_ref() {
            bullet_modes.push(super::characteristic_modes::recognize_characteristics_mode(
                next_line, template,
            )?);
            probe_idx += 1;
            continue;
        }
        bullet_modes.push(recognize_modal_mode(
            &preprocessed.card,
            next_line,
            header_has_common_target_suffix,
        )?);
        probe_idx += 1;
    }

    if bullet_modes.is_empty() {
        return Ok(None);
    }

    Ok(Some((
        RecognizedLine::Modal(RecognizedModalBlock {
            header: line.info.clone(),
            header_tokens,
            modes: bullet_modes,
            saga_chapters: None,
        }),
        probe_idx,
    )))
}

/// "I, II — Choose one —" followed by bullet modes: a modal Saga chapter
/// ability (CR 714.2b). Its mode is chosen as the chapter ability is put on
/// the stack, exactly as for any other modal triggered ability (CR 700.2b).
pub(super) fn try_parse_saga_modal_chapter_block(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
) -> Result<Option<(RecognizedLine, usize)>, CardTextError> {
    let Some(authored) = parse_saga_chapter_prefix_tokens(&line.info.source_tokens) else {
        return Ok(None);
    };
    if authored.presentation_label.is_some() {
        return Ok(None);
    }
    let next_line_is_mode = matches!(
        preprocessed.items.get(idx + 1),
        Some(PreprocessedItem::Line(next_line)) if is_bullet_line(next_line)
    );
    if !next_line_is_mode {
        return Ok(None);
    }
    let chapters = authored.chapters.clone();
    let body_tokens = parse_saga_chapter_prefix_tokens(&line.tokens)
        .filter(|normalized| normalized.chapters == chapters)
        .map(|normalized| normalized.body_tokens.to_vec())
        .unwrap_or_else(|| authored.body_tokens.to_vec());
    let Some(header) = super::super::modal_support::parse_modal_header(&line.info, &body_tokens)?
    else {
        return Ok(None);
    };
    // The chapter number is the only trigger; a body with its own trigger or
    // activation cost is not a chapter-level choice.
    if header.trigger.is_some() || header.activated.is_some() {
        return Ok(None);
    }
    let header_has_common_target_suffix = !header.common_suffix_effects_ast.is_empty();
    let mut bullet_modes = Vec::new();
    let mut probe_idx = idx + 1;
    while let Some(PreprocessedItem::Line(next_line)) = preprocessed.items.get(probe_idx) {
        if !is_bullet_line(next_line) {
            break;
        }
        bullet_modes.push(recognize_modal_mode(
            &preprocessed.card,
            next_line,
            header_has_common_target_suffix,
        )?);
        probe_idx += 1;
    }
    if bullet_modes.is_empty() {
        return Ok(None);
    }
    Ok(Some((
        RecognizedLine::Modal(RecognizedModalBlock {
            header: line.info.clone(),
            header_tokens: body_tokens,
            modes: bullet_modes,
            saga_chapters: Some(chapters),
        }),
        probe_idx,
    )))
}
