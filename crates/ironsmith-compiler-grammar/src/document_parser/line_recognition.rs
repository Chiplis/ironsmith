use super::super::grammar::{
    clause_support, keyword_static_lines, line_families, line_family_rewrites, semantic_lowering,
    structure,
};
use super::*;
use crate::parse_trace;

fn line_starts_with_effect_statement_sentence(tokens: &[OwnedLexToken]) -> bool {
    line_families::parse_multi_sentence_effect_head(tokens).is_some()
}

fn comma_split_tail_starts_with_filter_list_continuation(tokens: &[OwnedLexToken]) -> bool {
    line_families::parse_filter_list_continuation(tokens).is_some()
}

pub(super) fn contains_reflexive_conditional_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    line_families::parse_reflexive_conditional_followup(tokens).is_some()
}

/// A delayed action can establish the exact object consumed by a later
/// delayed result check in the same triggered ability:
/// `... at end of combat. At the beginning of the next end step, if that
/// creature was destroyed this way, ...`. The later `if` comma is internal to
/// the resolution program and must not become the outer trigger/effect split.
fn has_end_of_combat_action_then_next_end_step_result_followup(tokens: &[OwnedLexToken]) -> bool {
    line_families::parse_end_combat_next_end_step_followup(tokens).is_some()
}

fn rewrite_count_that_number_life_total_trigger_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let parsed = line_family_rewrites::parse_count_that_number_life_total_rewrite_tokens(tokens)?;

    let mut rewritten = Vec::with_capacity(
        parsed.trigger_tokens.len()
            + parsed.subject_tokens.len()
            + parsed.count_value_tokens.len()
            + 3,
    );
    rewritten.extend_from_slice(parsed.trigger_tokens);
    rewritten.push(OwnedLexToken::comma(TextSpan::synthetic()));
    rewritten.extend_from_slice(parsed.subject_tokens);
    rewritten.push(OwnedLexToken::word("becomes", TextSpan::synthetic()));
    rewritten.extend_from_slice(parsed.count_value_tokens);
    rewritten.push(OwnedLexToken::period(TextSpan::synthetic()));
    Some(rewritten)
}

fn trigger_has_moved_or_cast_origin_condition(trigger: &crate::model::ast::TriggerSpec) -> bool {
    crate::activation_and_restrictions::trigger_clause_core::trigger_spec_has_moved_or_cast_origin_condition(trigger)
}

fn moved_or_cast_origin_trigger_split_index(
    tokens: &[OwnedLexToken],
    trigger_start: usize,
) -> Option<usize> {
    let mut inside_quotes = false;
    for (separator_idx, separator) in tokens.iter().enumerate() {
        if separator.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes
            || separator.kind != TokenKind::Comma
            || separator_idx <= trigger_start
            || separator_idx + 1 >= tokens.len()
        {
            continue;
        }

        let trigger_tokens = trim_lexed_commas(&tokens[trigger_start..separator_idx]);
        let Ok(trigger) = parse_trigger_clause_lexed(trigger_tokens) else {
            continue;
        };
        if trigger_has_moved_or_cast_origin_condition(&trigger) {
            return Some(separator_idx);
        }
    }
    None
}

pub(super) fn recognize_triggered_line(
    line: &PreprocessedLine,
) -> Result<RecognizedTriggeredLine, CardTextError> {
    if let Some(recognized) = recognize_correlated_triggered_line(line) {
        return Ok(recognized);
    }
    recognize_triggered_line_inner(line)
}

fn recognize_correlated_triggered_line(line: &PreprocessedLine) -> Option<RecognizedTriggeredLine> {
    let (tokens_without_cap, trailing_cap) = strip_trailing_trigger_cap_suffix_tokens(&line.tokens);
    let (source_tokens_without_cap, _) =
        strip_trailing_trigger_cap_suffix_tokens(&line.info.source_tokens);
    let correlated_tail = |effect_tokens: &[OwnedLexToken]| {
        let words = crate::lexer::parser_token_word_refs(effect_tokens);
        crate::semantic_line_parsing::has_created_token_reciprocal_lifecycle_surface(effect_tokens)
            || crate::semantic_line_parsing::has_linked_created_token_next_turn_sacrifice_surface(
                effect_tokens,
            )
            || has_end_of_combat_action_then_next_end_step_result_followup(effect_tokens)
            || crate::semantic_line_parsing::is_authored_dynamic_exile_permission_bundle(
                effect_tokens,
            )
            || crate::semantic_line_parsing::is_authored_look_hand_optional_cast_bundle(
                effect_tokens,
            )
            || (crate::word_primitives::contains_word(&words, "return")
                && crate::word_primitives::contains_word(&words, "aura")
                && crate::semantic_line_parsing::is_exact_correlated_trigger_effect_bundle(
                    effect_tokens,
                ))
    };

    if parse_trigger_intro_tokens(source_tokens_without_cap).is_some()
        && let Some((leading_tokens, effect_tokens)) =
            grammar::split_lexed_once_on_comma(source_tokens_without_cap)
        && leading_tokens.len() > 1
        && correlated_tail(effect_tokens)
        && let Some(candidate) = render_triggered_split_candidate(
            &leading_tokens[1..],
            effect_tokens,
            None,
            trailing_cap,
        )
    {
        parse_trace::event("trigger split: authored correlated multi-sentence body");
        return Some(candidate.into_recognized_line(line, source_tokens_without_cap));
    }

    let (leading_tokens, effect_tokens) = grammar::split_lexed_once_on_comma(tokens_without_cap)?;
    if leading_tokens.len() <= 1 || !correlated_tail(effect_tokens) {
        return None;
    }
    let candidate =
        render_triggered_split_candidate(&leading_tokens[1..], effect_tokens, None, trailing_cap)?;
    parse_trace::event("trigger split: exact correlated multi-sentence body");
    Some(candidate.into_recognized_line(line, tokens_without_cap))
}

fn recognize_triggered_line_inner(
    line: &PreprocessedLine,
) -> Result<RecognizedTriggeredLine, CardTextError> {
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

    if let Some(rewritten_tokens) =
        rewrite_count_that_number_life_total_trigger_tokens(tokens_without_cap)
    {
        let rewritten_line = rewrite_line_tokens(line, &rewritten_tokens);
        if let Ok(mut parsed) = recognize_triggered_line(&rewritten_line) {
            parsed.full_text = normalized.clone();
            parsed.full_parse_tokens = tokens_without_cap.to_vec();
            return Ok(parsed);
        }
    }

    if let Some(nested_trigger_tokens) =
        split_nested_combat_whenever_clause_lexed(tokens_without_cap)
    {
        let nested_line = rewrite_line_tokens(line, nested_trigger_tokens);
        if let Ok(mut parsed) = recognize_triggered_line(&nested_line) {
            // The semantic lowering pass needs the complete outer beginning-
            // of-combat/payment envelope in order to register the nested
            // trigger only when the payment is declined. Keep the nested
            // trigger/effect token split, but do not discard the authored
            // full-line tokens that prove that envelope.
            parsed.full_text = normalized.clone();
            parsed.full_parse_tokens = tokens_without_cap.to_vec();
            return Ok(parsed);
        }
    }

    let mut best_probe_error = None;
    let mut typed_conditional_fallback = None;
    let moved_or_cast_origin_split =
        moved_or_cast_origin_trigger_split_index(tokens_without_cap, 1);

    // A later reflexive sentence can itself begin with an `if` condition:
    // "At ..., mill a card. When you do, if ..., ...". The broad triggered
    // conditional splitter otherwise selects the comma after "When you do"
    // and absorbs the producer into the trigger header. Commit the grammar-
    // proven first trigger comma when the complete tail parses as one effect
    // program, preserving both the producer and its reflexive result.
    if moved_or_cast_origin_split.is_none()
        && let Some((leading_tokens, effect_tokens)) =
            grammar::split_lexed_once_on_comma(tokens_without_cap)
        && leading_tokens.len() > 1
        && contains_reflexive_conditional_followup_sentence(effect_tokens)
        && parse_trigger_clause_lexed(trim_lexed_commas(&leading_tokens[1..])).is_ok()
        && let Some(candidate) = render_triggered_split_candidate(
            &leading_tokens[1..],
            effect_tokens,
            None,
            trailing_cap,
        )
    {
        parse_trace::event("trigger split: producer with reflexive followup sentence");
        return Ok(candidate.into_recognized_line(line, tokens_without_cap));
    }

    // Once a triggered ability explicitly starts its post-trigger clause with
    // `if`, that condition is semantic: it is an intervening-if check, not
    // expendable punctuation. Keep track of the surface shape independently
    // of whether the typed predicate grammar can model it so the generic
    // comma splitter below cannot silently turn the ability unconditional.
    let has_explicit_intervening_if = moved_or_cast_origin_split.is_none()
        && grammar::split_lexed_once_on_comma(tokens_without_cap).is_some_and(
            |(_, after_trigger)| {
                after_trigger.first().is_some_and(|token| {
                    token.kind == TokenKind::Word && token.parser_text() == "if"
                })
            },
        );

    if moved_or_cast_origin_split.is_none()
        && let Some(spec) =
            structure::split_triggered_conditional_clause_lexed(tokens_without_cap, 1)
    {
        let probe = probe_triggered_split(
            spec.trigger_tokens,
            spec.effects_tokens,
            Some(spec.predicate.clone()),
            trailing_cap,
        );
        if let Some(mut parsed) = probe.supported_recognized(line, tokens_without_cap) {
            parsed.full_text = normalized.clone();
            parse_trace::event(format!(
                "trigger split: conditional trigger=\"{}\" effects=\"{}\"",
                render_token_slice(&parsed.trigger_parse_tokens),
                render_token_slice(&parsed.effect_parse_tokens)
            ));
            return Ok(parsed);
        }
        if best_probe_error.is_none() {
            best_probe_error = probe.preferred_error();
        }
        // The split probe is deliberately conservative and can reject a
        // trigger/effect pair that the semantic document pass can still
        // lower with its richer context. Preserve that existing fallback
        // behavior, but keep the successfully parsed predicate attached.
        // This is the conditional counterpart to the generic fallback below.
        typed_conditional_fallback = probe.fallback_recognized(line, tokens_without_cap);
    }

    // The combined X-cost trigger has a typed semantic lowering, but its
    // intervening-if predicate is intentionally not represented as an
    // ordinary standalone predicate. Build the recognized form directly so the semantic
    // pass can recognize the complete authored shape.
    if let Some(effect_start) =
        clause_support::parse_combined_x_cost_trigger_tokens(tokens_without_cap)
        && let Some(comma_index) =
            crate::slice_primitives::select_position(tokens_without_cap, |token| {
                token.kind == TokenKind::Comma
            })
        && let Some(trigger_tokens) = tokens_without_cap.get(1..comma_index)
        && let Some(effect_tokens) = tokens_without_cap.get(effect_start..)
        && let Some(candidate) =
            render_triggered_split_candidate(trigger_tokens, effect_tokens, None, trailing_cap)
    {
        return Ok(candidate.into_recognized_line(line, tokens_without_cap));
    }

    // A same-object exile/return program is one atomic effect even though it
    // contains a later comma-then boundary. Once the grammar proves the full
    // tail, commit the first trigger comma so speculative later splits cannot
    // absorb the exile action into the trigger and retain only the return.
    if let Some((leading_tokens, effect_tokens)) =
        grammar::split_lexed_once_on_comma(tokens_without_cap)
        && leading_tokens.len() > 1
        && parse_trigger_clause_lexed(trim_lexed_commas(&leading_tokens[1..])).is_ok()
        && crate::grammar::effects::parse_exile_return_same_shape(
            crate::util::trim_edge_punctuation_tokens(effect_tokens),
        )
        .is_some()
        && let Some(candidate) = render_triggered_split_candidate(
            &leading_tokens[1..],
            effect_tokens,
            None,
            trailing_cap,
        )
    {
        return Ok(candidate.into_recognized_line(line, tokens_without_cap));
    }

    if has_explicit_intervening_if {
        if let Some(parsed) = typed_conditional_fallback {
            parse_trace::event(format!(
                "trigger split: typed conditional fallback trigger=\"{}\" effects=\"{}\"",
                render_token_slice(&parsed.trigger_parse_tokens),
                render_token_slice(&parsed.effect_parse_tokens)
            ));
            return Ok(parsed);
        }
        return Err(best_probe_error.unwrap_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported intervening-if predicate in triggered line: '{}'",
                line.info.raw_line
            ))
        }));
    }

    let preferred_comma_split = moved_or_cast_origin_split
        .map(|separator_idx| {
            (
                &tokens_without_cap[..separator_idx],
                &tokens_without_cap[separator_idx + 1..],
            )
        })
        .or_else(|| grammar::split_lexed_once_on_comma(tokens_without_cap));
    if let Some((leading_tokens, effect_tokens)) = preferred_comma_split
        && leading_tokens.len() > 1
        && !comma_split_tail_starts_with_filter_list_continuation(effect_tokens)
    {
        let probe = probe_triggered_split(&leading_tokens[1..], effect_tokens, None, trailing_cap);
        if let Some(parsed) = probe.supported_recognized(line, tokens_without_cap) {
            parse_trace::event(format!(
                "trigger split: comma trigger=\"{}\" effects=\"{}\"",
                render_token_slice(&parsed.trigger_parse_tokens),
                render_token_slice(&parsed.effect_parse_tokens)
            ));
            return Ok(parsed);
        }
        if best_probe_error.is_none() {
            best_probe_error = probe.preferred_error();
        }
    }

    let whole_line_parse = parse_triggered_line_lexed(tokens_without_cap);
    let mut best_supported_split = None;
    let mut best_fallback_split = None;
    let mut inside_quotes = false;

    for (separator_idx, separator) in tokens_without_cap.iter().enumerate() {
        if separator.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes || separator.kind != TokenKind::Comma || separator_idx <= 1 {
            continue;
        }
        if moved_or_cast_origin_split.is_some_and(|owned_split| separator_idx < owned_split) {
            continue;
        }
        if tokens_without_cap[1..separator_idx]
            .iter()
            .any(|token| token.kind == TokenKind::Period)
        {
            continue;
        }
        if comma_split_tail_starts_with_filter_list_continuation(
            &tokens_without_cap[separator_idx + 1..],
        ) {
            continue;
        }

        let probe = probe_triggered_split(
            &tokens_without_cap[1..separator_idx],
            &tokens_without_cap[separator_idx + 1..],
            None,
            trailing_cap,
        );

        if let Some(parsed) = probe.supported_recognized(line, tokens_without_cap) {
            // Prefer the split with the most effect tokens (latest separator
            // = largest effects portion). This prevents silent truncation
            // where an early split absorbs most content into the trigger.
            let effect_len = parsed.effect_parse_tokens.len();
            if best_supported_split.as_ref().is_none_or(
                |(_, prev): &(usize, RecognizedTriggeredLine)| {
                    effect_len > prev.effect_parse_tokens.len()
                },
            ) {
                best_supported_split = Some((separator_idx, parsed));
            }
            continue;
        }

        if best_probe_error.is_none() {
            best_probe_error = probe.preferred_error();
        }

        if whole_line_parse.is_ok() && best_fallback_split.is_none() {
            best_fallback_split = probe.fallback_recognized(line, tokens_without_cap);
        }
    }

    if let Some(split) = best_supported_split
        .map(|(_, recognized)| recognized)
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
        parse_trace::event(format!(
            "trigger split: selected trigger=\"{}\" effects=\"{}\"",
            render_token_slice(&split.trigger_parse_tokens),
            render_token_slice(&split.effect_parse_tokens)
        ));
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
                LineAst::Multiple(chunks)
                    if chunks
                        .iter()
                        .any(|chunk| matches!(chunk, LineAst::Triggered { .. })) =>
                {
                    tokens_without_cap.len() / 2
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
            parse_trace::event("trigger parse: whole-line parser matched");
            Ok(RecognizedTriggeredLine {
                info: line.info.clone(),
                full_text: normalized.to_string(),
                full_parse_tokens: tokens_without_cap.to_vec(),
                trigger_parse_tokens: condition_tokens.to_vec(),
                effect_parse_tokens: Vec::new(),
                max_triggers_per_turn: trailing_cap,
                intervening_if: None,
                presentation: None,
                chosen_option: None,
            })
        }
        Err(err) => Err(best_probe_error.unwrap_or(err)),
    }
}

pub(super) fn recognize_static_line(
    line: &PreprocessedLine,
) -> Result<Option<RecognizedStaticLine>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();
    let parse_tokens = rewrite_keyword_dash_parse_tokens(&line.tokens);
    if super::super::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(
        &parse_tokens,
    )
    .is_some()
    {
        return Ok(None);
    }
    let is_flash_with_cleanup_sacrifice = super::super::grammar::abilities::is_cast_as_though_flash_with_next_cleanup_sacrifice_line_lexed(
        &parse_tokens,
    );
    if line_starts_with_effect_statement_sentence(&parse_tokens)
        && !is_flash_with_cleanup_sacrifice
        && !matches!(
            parse_static_ability_ast_line_lexed(&parse_tokens),
            Ok(Some(_))
        )
    {
        return Ok(None);
    }
    let make_static = |chosen_option: Option<ChosenOptionContext>| RecognizedStaticLine {
        info: line.info.clone(),
        parse_tokens: parse_tokens.clone(),
        chosen_option,
        parsed: None,
    };
    if is_flash_with_cleanup_sacrifice {
        return Ok(Some(make_static(None)));
    }
    let lexed = &parse_tokens;
    if crate::keyword_static::parse_double_counters_replacement_line(lexed)?.is_some() {
        return Ok(Some(make_static(None)));
    }
    if matches!(
        normalized,
        "for each {B} in a cost, you may pay 2 life rather than pay that mana."
            | "for each {b} in a cost, you may pay 2 life rather than pay that mana."
            | "as long as trinisphere is untapped, each spell that would cost less than three mana to cast costs three mana to cast."
            | "as long as this is untapped, each spell that would cost less than three mana to cast costs three mana to cast."
            | "players can't pay life or sacrifice nonland permanents to cast spells or activate abilities."
            | "creatures you control can boast twice during each of your turns rather than once."
            | "you may activate equip abilities any time you could cast an instant."
    ) || keyword_static_lines::parse_additional_vote_tokens(lexed).is_some()
        || is_first_equip_cost_alternative_line(lexed)
        || is_additional_land_play_static_line(lexed)
        || is_can_block_additional_creatures_static_line(lexed)
        || is_any_number_named_deck_construction_line(lexed)
        || matches!(
            semantic_lowering::parse_static_special_line_tokens(lexed),
            Some(
                semantic_lowering::StaticSpecialLineShape::HiddenAgenda
                    | semantic_lowering::StaticSpecialLineShape::DoubleAgenda
            )
        )
    {
        return Ok(Some(make_static(None)));
    }

    let mut deferred_error = None;

    if grammar::parse_prefix(lexed, grammar::phrase(&["level", "up"])).is_some()
        && parse_level_up_line_lexed(lexed)?.is_some()
    {
        return Ok(Some(make_static(None)));
    }
    if is_doesnt_untap_during_your_untap_step_line_lexed(lexed) {
        return Ok(Some(make_static(None)));
    }
    if matches!(
        super::super::grammar::structure::classify_static_line_family_lexed(lexed),
        Some(super::super::grammar::structure::StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep)
    ) {
        return Ok(Some(make_static(None)));
    }

    if parse_if_this_spell_costs_less_to_cast_line_lexed(lexed)?.is_some() {
        return Ok(Some(make_static(None)));
    }
    if parse_spell_additional_life_cost_per_target_line(lexed)?.is_some() {
        return Ok(Some(make_static(None)));
    }
    if parse_spell_cost_increase_per_target_beyond_first_line(lexed)?.is_some() {
        return Ok(Some(make_static(None)));
    }
    if parse_spell_and_player_activated_ability_cost_modifier_line(lexed)?.is_some() {
        return Ok(Some(make_static(None)));
    }
    if parse_spells_cost_modifier_line(lexed)?.is_some() {
        return Ok(Some(make_static(None)));
    }

    if is_activate_only_once_each_turn_line_lexed(lexed) {
        return Ok(Some(make_static(None)));
    }

    if effect_grammar::parse_compound_buff_unblockable_tokens(lexed).is_some() {
        return Ok(Some(make_static(None)));
    }

    match parse_static_ability_ast_line_lexed(lexed) {
        Ok(Some(_)) => return Ok(Some(make_static(None))),
        Ok(None) => {}
        Err(err) => deferred_error = Some(err),
    }

    if !should_skip_keyword_action_static_probe(lexed)
        && let Some(_actions) = parse_ability_line_lexed(lexed)
    {
        return Ok(Some(make_static(None)));
    }

    if parse_split_static_item_count(lexed)?.is_some() {
        return Ok(Some(make_static(None)));
    }

    if let Some(err) = deferred_error {
        return Err(err);
    }

    Ok(None)
}

fn is_any_number_named_deck_construction_line(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        semantic_lowering::parse_static_special_line_tokens(tokens),
        Some(semantic_lowering::StaticSpecialLineShape::AnyNumberNamedDeckConstruction)
    )
}

/// Recognizes "you may pay {COST} rather than pay the equip cost of the first
/// equip ability you activate each turn." and the variant "during each of your turns."
fn is_first_equip_cost_alternative_line(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        semantic_lowering::parse_static_special_line_tokens(tokens),
        Some(semantic_lowering::StaticSpecialLineShape::FirstEquipCostAlternative)
    )
}

fn is_additional_land_play_static_line(tokens: &[OwnedLexToken]) -> bool {
    semantic_lowering::parse_additional_land_play_count_tokens(tokens).is_some()
}

fn is_can_block_additional_creatures_static_line(tokens: &[OwnedLexToken]) -> bool {
    keyword_static_lines::parse_can_block_additional_creature_tokens(tokens).is_some()
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
        if parse_spell_additional_life_cost_per_target_line(sentence)?.is_some() {
            item_count += 1;
            continue;
        }
        if parse_spell_cost_increase_per_target_beyond_first_line(sentence)?.is_some() {
            item_count += 1;
            continue;
        }
        if let Some(abilities) =
            parse_spell_and_player_activated_ability_cost_modifier_line(sentence)?
        {
            item_count += abilities.len();
            continue;
        }
        if parse_spells_cost_modifier_line(sentence)?.is_some() {
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
            if message.contains("unsupported trigger clause") =>
        {
            CardTextError::ParseError(format!("unsupported triggered line: '{raw_line}'"))
        }
        Some(err) => err,
        None => CardTextError::ParseError(format!("unsupported triggered line: '{raw_line}'")),
    }
}

pub(super) fn recognize_level_item(
    builder: &CardDefinitionBuilder,
    line: &PreprocessedLine,
) -> Result<Option<RecognizedLevelItem>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();

    if let Some((cost_tokens, effect_parse_tokens)) =
        split_activation_text_tokens_lexed(&line.tokens)
    {
        let normalized_cost_tokens =
            normalize_activation_cost_tokens_for_builder(builder, line, cost_tokens.clone())?;
        match parse_activation_cost_tokens_rewrite(&normalized_cost_tokens) {
            Ok(cost_recognized) => {
                let compiler_cost =
                    crate::semantic_assembly::assemble_activation_cost(&cost_recognized)?;
                let parsed = super::super::semantic_line_parsing::parse_activated_line(
                    line.info.clone(),
                    compiler_cost,
                    normalized_cost_tokens,
                    effect_parse_tokens,
                    ActivationTiming::AnyTime,
                    false,
                    None,
                    None,
                )?;
                return Ok(Some(RecognizedLevelItem {
                    info: line.info.clone(),
                    text: normalized.to_string(),
                    kind: LevelItemKind::ActivatedAbility,
                    parsed: ParsedLevelAbilityItemAst::ActivatedAbility(
                        ParsedLevelActivatedAbilityAst {
                            info: line.info.semantic_info(),
                            chunk: parsed.chunk,
                            restrictions: parsed.restrictions,
                            semantic_facts: line.info.semantic_facts.clone(),
                        },
                    ),
                }));
            }
            Err(err) if looks_like_activation_cost_prefix(&cost_tokens) => return Err(err),
            Err(_) => {}
        }
    }

    if !should_skip_keyword_action_static_probe(&line.tokens)
        && let Some(actions) = parse_ability_line_lexed(&line.tokens)
    {
        return Ok(Some(RecognizedLevelItem {
            info: line.info.clone(),
            text: normalized.to_string(),
            kind: LevelItemKind::KeywordActions,
            parsed: ParsedLevelAbilityItemAst::KeywordActions(actions),
        }));
    }

    if let Some(abilities) = parse_static_ability_ast_line_lexed(&line.tokens)? {
        return Ok(Some(RecognizedLevelItem {
            info: line.info.clone(),
            text: normalized.to_string(),
            kind: LevelItemKind::StaticAbilities,
            parsed: ParsedLevelAbilityItemAst::StaticAbilities(abilities),
        }));
    }

    Ok(None)
}

pub(super) fn recognize_modal_mode(
    line: &PreprocessedLine,
    allow_bare_target: bool,
) -> Result<RecognizedModalMode, CardTextError> {
    let spree_prefix = parse_spree_mode_prefix(&line.tokens);
    let tiered_prefix = parse_tiered_mode_prefix(&line.tokens);
    let point_cost = spree_prefix
        .as_ref()
        .or(tiered_prefix.as_ref())
        .map_or_else(|| leading_modal_point_cost_tokens(&line.tokens), |_| None);
    let parse_tokens = if let Some((_, body_first)) = spree_prefix.as_ref() {
        line.tokens.get(*body_first..).unwrap_or_default()
    } else if let Some((_, body_first)) = tiered_prefix.as_ref() {
        line.tokens.get(*body_first..).unwrap_or_default()
    } else {
        strip_non_keyword_label_prefix_lexed(strip_modal_bullet_prefix_tokens(&line.tokens))
    };
    let surface_tokens = if tiered_prefix.is_some() {
        strip_modal_bullet_prefix_tokens(&line.tokens)
    } else {
        parse_tokens
    };
    let mode_text = render_original_text_for_token_slice(line, surface_tokens)
        .unwrap_or_else(|| render_token_slice(surface_tokens))
        .trim()
        .to_string();
    let effects_ast = match parse_effect_sentences_lexed(parse_tokens) {
        Ok(effects) => effects,
        Err(original_error) if allow_bare_target => {
            let target_tokens = crate::util::trim_edge_punctuation_tokens(parse_tokens);
            let target =
                crate::util::parse_target_phrase(target_tokens).map_err(|_| original_error)?;
            vec![crate::model::ast::EffectAst::subject_verb_explicit_target_only(target)]
        }
        Err(error) => return Err(error),
    };
    Ok(RecognizedModalMode {
        info: line.info.clone(),
        text: mode_text,
        point_cost,
        additional_mana_cost: spree_prefix.or(tiered_prefix).map(|(cost, _)| cost),
        effects_ast,
    })
}

fn parse_tiered_mode_prefix(tokens: &[OwnedLexToken]) -> Option<(crate::mana::ManaCost, usize)> {
    if !tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Bullet)
    {
        return None;
    }
    let label_delimiter = tokens.iter().enumerate().skip(2).find_map(|(idx, token)| {
        matches!(token.kind, TokenKind::Dash | TokenKind::EmDash).then_some(idx)
    })?;
    let mana = super::super::grammar::leaf::parse_leaf_mana_cost_prefix_tokens(
        tokens.get(label_delimiter + 1..)?,
    )?;
    let body_delimiter = label_delimiter + 1 + mana.consumed;
    if !tokens
        .get(body_delimiter)
        .is_some_and(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
    {
        return None;
    }
    Some((mana.cost, body_delimiter + 1))
}

fn parse_spree_mode_prefix(tokens: &[OwnedLexToken]) -> Option<(crate::mana::ManaCost, usize)> {
    if !tokens
        .first()
        .is_some_and(|token| token.kind == TokenKind::Plus)
    {
        return None;
    }
    let mana = super::super::grammar::leaf::parse_leaf_mana_cost_prefix_tokens(&tokens[1..])?;
    let delimiter = 1 + mana.consumed;
    if !tokens
        .get(delimiter)
        .is_some_and(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
    {
        return None;
    }
    Some((mana.cost, delimiter + 1))
}

fn leading_modal_point_cost_tokens(tokens: &[OwnedLexToken]) -> Option<u32> {
    super::super::grammar::modal::parse_modal_point_label_tokens(tokens).map(|shape| shape.count)
}

fn strip_modal_bullet_prefix_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if tokens
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Bullet | TokenKind::Dash))
    {
        tokens.get(1..).unwrap_or_default()
    } else {
        tokens
    }
}

pub(super) fn recognize_saga_chapter_line(
    line: &PreprocessedLine,
    chapters: Vec<u32>,
    presentation_label: Option<String>,
    display_text: &str,
    parse_text: &str,
) -> Result<RecognizedSagaChapterLine, CardTextError> {
    let parse_tokens = lexed_tokens(parse_text, line.info.line_index)?;
    let mut effects_ast = parse_effect_sentences_lexed(&parse_tokens)?;
    crate::util::reconcile_unique_named_source_exile_surface(
        &mut effects_ast,
        &line.info.source_tokens,
    );
    Ok(RecognizedSagaChapterLine {
        info: line.info.clone(),
        chapters,
        presentation_label,
        text: display_text.to_string(),
        effects_ast,
    })
}
