use super::super::grammar::effects::{
    clause_dispatch_shapes::{self, DirectClauseShape},
    followup_shapes, parse_create_head_tokens,
};
use super::super::grammar::statement_shapes::{self, StatementForceShape};
use super::super::grammar::structure;
use super::*;
use crate::runtime_backend::ast::{EffectAst, StaticAbilityAst};

fn probe_static_ability_ast_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    crate::parse_loss::capture(|| parse_static_ability_ast_line_lexed(tokens)).0
}

fn probe_effect_sentences_lexed(tokens: &[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError> {
    crate::parse_loss::capture(|| parse_effect_sentences_lexed(tokens)).0
}

fn parse_effect_sentences_committing_loss_on_success(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let (result, loss) = crate::parse_loss::capture(|| parse_effect_sentences_lexed(tokens));
    if result.is_ok() {
        for diagnostic in loss.diagnostics() {
            crate::parse_loss::record(diagnostic.code.clone(), diagnostic.message.clone());
        }
    }
    result
}

fn is_die_roll_result_adjustment_statement(tokens: &[OwnedLexToken]) -> bool {
    statement_shapes::parse_die_roll_adjustment_tokens(tokens).is_some()
}

fn parse_any_player_no_one_does_statement(
    line: &PreprocessedLine,
) -> Result<Option<StatementLineCst>, CardTextError> {
    let sentences = normalize_statement_parse_sentences_lexed(&line.tokens);
    if statement_shapes::parse_any_player_no_one_does_sentences(&sentences).is_none() {
        return Ok(None);
    }

    let parse_groups = vec![join_statement_parse_sentence_group(&sentences)];
    for group_tokens in &parse_groups {
        parse_effect_sentences_committing_loss_on_success(group_tokens)?;
    }

    Ok(Some(StatementLineCst {
        info: line.info.clone(),
        text: line.info.normalized.normalized.clone(),
        parse_tokens: line.tokens.clone(),
        parse_groups,
    }))
}

fn is_each_player_choose_unselected_bounce_then_draw_statement(tokens: &[OwnedLexToken]) -> bool {
    statement_shapes::parse_each_player_choose_bounce_draw_tokens(tokens).is_some()
}

fn join_statement_parse_sentence_group(sentences: &[Vec<OwnedLexToken>]) -> Vec<OwnedLexToken> {
    let mut joined = Vec::new();
    for sentence in sentences {
        if sentence.is_empty() {
            continue;
        }
        if !joined.is_empty() {
            joined.push(OwnedLexToken::period(TextSpan::synthetic()));
        }
        joined.extend(sentence.clone());
    }
    if !joined.is_empty() {
        joined.push(OwnedLexToken::period(TextSpan::synthetic()));
    }
    joined
}

pub(super) fn parse_statement_line_cst(
    line: &PreprocessedLine,
) -> Result<Option<StatementLineCst>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();
    if looks_like_day_night_starts_day_as_enters_static_line(&line.tokens) {
        return Ok(None);
    }
    if is_die_roll_result_adjustment_statement(&line.tokens) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: vec![line.tokens.clone()],
        }));
    }
    if let Some(statement) = parse_any_player_no_one_does_statement(line)? {
        return Ok(Some(statement));
    }
    if is_each_player_choose_unselected_bounce_then_draw_statement(&line.tokens) {
        parse_effect_sentences_committing_loss_on_success(&line.tokens)?;
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: vec![line.tokens.clone()],
        }));
    }
    let line_family = structure::classify_statement_line_family_lexed(&line.tokens);
    let static_probe = probe_static_ability_ast_line_lexed(&line.tokens)
        .ok()
        .flatten();
    let typed_effect_prefix_before_static =
        has_effect_prefix_before_trailing_static_sentence(&line.tokens);
    let typed_create_statement = line
        .tokens
        .first()
        .is_some_and(|token| token.is_word("create"))
        || parse_create_head_tokens(&line.tokens).is_some();
    let typed_energy_payment_threshold =
        super::super::grammar::effects::parse_energy_pay_any_destroy_tokens(&line.tokens).is_some();
    let typed_counter_linked_land_subtype = super::super::grammar::effects::followup_shapes::parse_counter_linked_land_subtype_followup(&line.tokens)
        .is_some();
    let typed_persistent_player_rule =
        super::super::grammar::effects::parse_persistent_no_maximum_hand_size_player_lexed(
            &line.tokens,
        )
        .is_some();
    if typed_counter_linked_land_subtype {
        // This follow-up is intentionally close to a static sentence, but it
        // is an effect-backed continuation of the preceding tagged land.
        // Route it through the statement parser before the generic static
        // probe can discard it as a static-only line.
        parse_effect_sentences_committing_loss_on_success(&line.tokens)?;
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: vec![line.tokens.clone()],
        }));
    }
    let force_surface = statement_shapes::parse_statement_force_shape(&line.tokens);
    let persistent_static_modifier = !typed_create_statement
        && !typed_energy_payment_threshold
        && !typed_counter_linked_land_subtype
        && !typed_persistent_player_rule
        && !typed_effect_prefix_before_static
        && force_surface != Some(StatementForceShape::PlayerGetsCounters)
        && !matches!(
            line_family,
            Some(structure::StatementLineFamily::Emblem | structure::StatementLineFamily::Vote)
        )
        && super::super::grammar::anthem_grants::parse_anthem_modifier_head(&line.tokens)
            .is_some_and(|head| !head.has_target && !head.temporary);
    if persistent_static_modifier {
        return Ok(None);
    }
    let force_statement = typed_create_statement
        || typed_energy_payment_threshold
        || typed_counter_linked_land_subtype
        || typed_persistent_player_rule
        || typed_effect_prefix_before_static
        || matches!(
            line_family,
            Some(structure::StatementLineFamily::Divvy | structure::StatementLineFamily::Emblem)
        )
        || matches!(
            line_family,
            Some(
                structure::StatementLineFamily::PactNextUpkeep
                    | structure::StatementLineFamily::ExilePlayCostsMore
                    | structure::StatementLineFamily::BidLife
            )
        )
        || matches!(
            force_surface,
            Some(
                StatementForceShape::DivvySelection
                    | StatementForceShape::ExilePlayCost
                    | StatementForceShape::GroupTurnDuration
                    | StatementForceShape::PlayerGetsCounters
            )
        )
        || (force_surface == Some(StatementForceShape::ConditionalInstead)
            && static_probe.is_none())
        || looks_like_statement_line_lexed(line);
    if !force_statement && static_probe.is_some() {
        return Ok(None);
    }
    if matches!(line_family, Some(structure::StatementLineFamily::Divvy)) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: vec![join_statement_parse_sentence_group(
                &normalize_statement_parse_sentences_lexed(&line.tokens),
            )],
        }));
    }
    if matches!(
        line_family,
        Some(
            structure::StatementLineFamily::PactNextUpkeep
                | structure::StatementLineFamily::ExilePlayCostsMore
                | structure::StatementLineFamily::BidLife
        )
    ) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: vec![join_statement_parse_sentence_group(
                &normalize_statement_parse_sentences_lexed(&line.tokens),
            )],
        }));
    }
    if matches!(
        structure::classify_static_line_family_lexed(&line.tokens),
        Some(
            structure::StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep
                | structure::StaticLineFamily::GrantedQuotedAbility
        )
    ) {
        return Ok(None);
    }
    if statement_shapes::parse_next_damage_prevention_tokens(&line.tokens).is_some() {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: vec![line.tokens.clone()],
        }));
    }
    let parse_groups = normalize_statement_parse_groups_lexed(&line.tokens);
    let mut found_effects = false;
    for group_tokens in &parse_groups {
        let effects = match parse_effect_sentences_committing_loss_on_success(group_tokens) {
            Ok(effects) => effects,
            Err(_)
                if matches!(
                    probe_static_ability_ast_line_lexed(group_tokens),
                    Ok(Some(_))
                ) =>
            {
                continue;
            }
            Err(err)
                if looks_like_statement_line_lexed(line)
                    || statement_shapes::has_statement_error_prefix(group_tokens) =>
            {
                return Err(err);
            }
            Err(_) => return Ok(None),
        };
        found_effects |= !effects.is_empty();
    }
    if !found_effects {
        return Ok(None);
    }

    Ok(Some(StatementLineCst {
        info: line.info.clone(),
        text: normalized.to_string(),
        parse_tokens: line.tokens.clone(),
        parse_groups,
    }))
}

fn looks_like_day_night_starts_day_as_enters_static_line(tokens: &[OwnedLexToken]) -> bool {
    statement_shapes::parse_day_night_enters_tokens(tokens).is_some()
}

fn is_trigger_result_followup_line(line: &PreprocessedLine) -> bool {
    structure::split_leading_result_prefix_lexed(&line.tokens).is_some()
}

fn append_joined_line_tokens(target: &mut Vec<OwnedLexToken>, extra: &[OwnedLexToken]) {
    if extra.is_empty() {
        return;
    }
    if target
        .last()
        .is_some_and(|token| token.kind != TokenKind::Period)
    {
        target.push(OwnedLexToken::period(TextSpan::synthetic()));
    }
    target.extend(extra.iter().cloned());
}

pub(super) fn extend_triggered_line_with_result_followups(
    items: &[PreprocessedItem],
    idx: usize,
    mut triggered: TriggeredLineCst,
) -> (TriggeredLineCst, usize) {
    let mut next_idx = idx + 1;

    while let Some(PreprocessedItem::Line(line)) = items.get(next_idx) {
        if super::is_nonkeyword_choice_labeled_line(line) {
            break;
        }
        if !is_trigger_result_followup_line(line) {
            break;
        }

        let followup_text = render_token_slice(&line.tokens).trim().to_string();
        if !triggered.full_text.is_empty() {
            triggered.full_text.push('\n');
        }
        triggered.full_text.push_str(followup_text.as_str());
        append_joined_line_tokens(&mut triggered.effect_parse_tokens, &line.tokens);
        append_joined_line_tokens(&mut triggered.full_parse_tokens, &line.tokens);

        next_idx += 1;
    }

    (triggered, next_idx)
}

pub(super) fn extend_activated_line_with_result_followups(
    items: &[PreprocessedItem],
    idx: usize,
    mut activated: ActivatedLineCst,
) -> (ActivatedLineCst, usize) {
    let mut next_idx = idx + 1;

    while let Some(PreprocessedItem::Line(line)) = items.get(next_idx) {
        if super::is_nonkeyword_choice_labeled_line(line) {
            break;
        }
        if !is_trigger_result_followup_line(line) {
            break;
        }

        append_joined_line_tokens(&mut activated.effect_parse_tokens, &line.tokens);

        next_idx += 1;
    }

    (activated, next_idx)
}

pub(super) fn extend_statement_line_with_result_followups(
    items: &[PreprocessedItem],
    idx: usize,
    mut statement: StatementLineCst,
) -> (StatementLineCst, usize) {
    let mut next_idx = idx + 1;

    while let Some(PreprocessedItem::Line(line)) = items.get(next_idx) {
        if super::is_nonkeyword_choice_labeled_line(line) {
            break;
        }
        if !is_trigger_result_followup_line(line) {
            break;
        }

        let followup_text = render_token_slice(&line.tokens).trim().to_string();
        if !statement.text.is_empty() {
            statement.text.push('\n');
        }
        statement.text.push_str(followup_text.as_str());
        append_joined_line_tokens(&mut statement.parse_tokens, &line.tokens);
        if let Some(parse_group) = statement.parse_groups.last_mut() {
            append_joined_line_tokens(parse_group, &line.tokens);
        } else {
            statement.parse_groups.push(line.tokens.clone());
        }

        next_idx += 1;
    }

    (statement, next_idx)
}

fn looks_like_statement_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    crate::parse_loss::capture(|| looks_like_statement_line_tokens_inner(tokens)).0
}

fn looks_like_statement_line_tokens_inner(tokens: &[OwnedLexToken]) -> bool {
    if matches!(
        structure::classify_static_line_family_lexed(tokens),
        Some(
            structure::StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep
                | structure::StaticLineFamily::GrantedQuotedAbility
        )
    ) {
        return false;
    }
    // A global phase-in prohibition is also superficially a valid phase-in
    // effect sentence. Prefer its complete typed static parse, while leaving
    // targeted or explicitly temporary prohibitions on the effect path.
    let words = crate::runtime_backend::token_word_refs(tokens);
    let is_phase_in_prohibition = words.windows(3).any(|window| {
        matches!(window[0], "can't" | "cant" | "cannot")
            && window[1] == "phase"
            && window[2] == "in"
    }) || words
        .windows(4)
        .any(|window| window == ["can", "not", "phase", "in"]);
    let is_timeless_phase_in_prohibition =
        is_phase_in_prohibition && !words.windows(2).any(|window| window == ["this", "turn"]);
    if is_timeless_phase_in_prohibition
        && structure::classify_static_line_family_lexed(tokens).is_some()
        && matches!(probe_static_ability_ast_line_lexed(tokens), Ok(Some(_)))
    {
        return false;
    }
    if is_each_player_choose_unselected_bounce_then_draw_statement(tokens) {
        return true;
    }
    let effect_sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if !effect_sentences.is_empty()
        && effect_sentences.into_iter().all(|sentence| {
            probe_effect_sentences_lexed(sentence).is_ok_and(|effects| !effects.is_empty())
        })
    {
        return true;
    }
    matches!(
        structure::classify_statement_line_family_lexed(tokens),
        Some(
            structure::StatementLineFamily::PactNextUpkeep
                | structure::StatementLineFamily::NextTurnCantCast
                | structure::StatementLineFamily::Divvy
                | structure::StatementLineFamily::Emblem
                | structure::StatementLineFamily::ArtRating
                | structure::StatementLineFamily::ExilePlayCostsMore
                | structure::StatementLineFamily::BidLife
                | structure::StatementLineFamily::Vote
                | structure::StatementLineFamily::Generic
        )
    )
}

pub(super) fn looks_like_statement_line_lexed(line: &PreprocessedLine) -> bool {
    if let Some(tokens) = tokens_after_non_keyword_label_prefix(line) {
        return looks_like_statement_line_tokens(tokens);
    }
    looks_like_statement_line_tokens(&line.tokens)
}

#[cfg(test)]
pub(super) fn looks_like_statement_line(normalized: &str) -> bool {
    if let Some((_, body)) = split_label_prefix(normalized) {
        return looks_like_statement_line(body);
    }

    lex_line(normalized, 0)
        .ok()
        .is_some_and(|tokens| looks_like_statement_line_tokens(&tokens))
}

fn normalize_statement_parse_sentences_lexed(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut sentences =
        super::super::grammar::statement_grouping::parse_statement_sentences_tokens(tokens)
            .sentences;
    if let Some(first) = sentences.first_mut()
        && first.first().is_some_and(|token| token.is_word("as"))
        && first.get(1).is_some_and(|token| token.is_word("this"))
        && let Some(timing_idx) = first
            .iter()
            .position(|token| token.is_word("enters") || token.is_word("transforms"))
        && (first[timing_idx].is_word("enters")
            || first
                .get(timing_idx + 1)
                .is_some_and(|token| token.is_word("into")))
        && let Some(comma_idx) = first
            .iter()
            .enumerate()
            .skip(timing_idx + 1)
            .find_map(|(idx, token)| token.is_comma().then_some(idx))
        && comma_idx + 1 < first.len()
    {
        first.drain(..=comma_idx);
    }
    sentences
}

fn first_trailing_static_sentence_idx(sentence_tokens: &[Vec<OwnedLexToken>]) -> Option<usize> {
    crate::parse_loss::capture(|| first_trailing_static_sentence_idx_inner(sentence_tokens)).0
}

fn first_trailing_static_sentence_idx_inner(
    sentence_tokens: &[Vec<OwnedLexToken>],
) -> Option<usize> {
    let first_static_idx =
        sentence_tokens
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(idx, sentence)| {
                (!followup_shapes::is_if_did_untap_source_followup(sentence)
                    && followup_shapes::parse_cant_be_regenerated_followup(sentence).is_none()
                    && clause_dispatch_shapes::parse_direct_clause_shape(sentence)
                        != Some(DirectClauseShape::DamageCantBePrevented)
                    && matches!(probe_static_ability_ast_line_lexed(sentence), Ok(Some(_))))
                .then_some(idx)
            })?;

    let effect_prefix = join_statement_parse_sentence_group(&sentence_tokens[..first_static_idx]);
    if probe_effect_sentences_lexed(&effect_prefix).is_err() {
        return None;
    }
    if !sentence_tokens[first_static_idx..]
        .iter()
        .all(|sentence| matches!(probe_static_ability_ast_line_lexed(sentence), Ok(Some(_))))
    {
        return None;
    }

    Some(first_static_idx)
}

pub(super) fn has_effect_prefix_before_trailing_static_sentence(tokens: &[OwnedLexToken]) -> bool {
    let sentences = normalize_statement_parse_sentences_lexed(tokens);
    first_trailing_static_sentence_idx(&sentences).is_some()
}

fn normalize_statement_parse_groups_from_sentences_lexed(
    sentence_tokens: Vec<Vec<OwnedLexToken>>,
    fallback_tokens: &[OwnedLexToken],
) -> Vec<Vec<OwnedLexToken>> {
    if sentence_tokens.len() <= 1 {
        let only_sentence = sentence_tokens
            .into_iter()
            .next()
            .or_else(|| {
                super::super::grammar::statement_grouping::parse_statement_grouping_tokens(
                    fallback_tokens,
                )
                .groups
                .into_iter()
                .next()
            })
            .unwrap_or_default();
        return (!only_sentence.is_empty())
            .then(|| join_statement_parse_sentence_group(&[only_sentence]))
            .into_iter()
            .collect();
    }

    let split_idx =
        super::super::grammar::statement_grouping::parse_statement_group_boundary(&sentence_tokens)
            .map(|boundary| boundary.sentence_index);

    let split_idx = split_idx.or_else(|| first_trailing_static_sentence_idx(&sentence_tokens));

    let Some(split_idx) = split_idx else {
        return vec![join_statement_parse_sentence_group(&sentence_tokens)];
    };

    let mut groups = Vec::new();
    if !sentence_tokens[..split_idx].is_empty() {
        groups.push(join_statement_parse_sentence_group(
            &sentence_tokens[..split_idx],
        ));
    }
    if !sentence_tokens[split_idx..].is_empty() {
        groups.push(join_statement_parse_sentence_group(
            &sentence_tokens[split_idx..],
        ));
    }
    groups
}

pub(super) fn normalize_statement_parse_groups_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<Vec<OwnedLexToken>> {
    // This typed bundle has a cross-sentence effect metric: the destroy
    // threshold refers to the amount of energy paid by the preceding effect.
    // Keep it as one semantic parse group so generic statement grouping cannot
    // sever that typed relationship.
    if super::super::grammar::effects::parse_energy_pay_any_destroy_tokens(tokens).is_some() {
        return vec![tokens.to_vec()];
    }
    let sentence_tokens = normalize_statement_parse_sentences_lexed(tokens);
    normalize_statement_parse_groups_from_sentences_lexed(sentence_tokens, tokens)
}

pub(super) fn parse_colon_nonactivation_statement_fallback(
    line: &PreprocessedLine,
) -> Result<Option<StatementLineCst>, CardTextError> {
    let Some((left_tokens, right_tokens)) = split_lexed_once_on_colon_outside_quotes(&line.tokens)
    else {
        return Ok(None);
    };

    if statement_shapes::parse_reveal_from_hand_tokens(left_tokens).is_some() {
        let left_line = rewrite_line_tokens(line, left_tokens);
        if let Some(statement) = parse_statement_line_cst(&left_line)? {
            return Ok(Some(statement));
        }
    }

    let left_has_mana_group = left_tokens
        .iter()
        .any(|token| matches!(token.kind, TokenKind::ManaGroup));
    let left_has_comma = left_tokens
        .iter()
        .any(|token| token.kind == TokenKind::Comma);

    if !left_has_mana_group && left_has_comma {
        let right_line = rewrite_line_tokens(line, right_tokens);
        if let Some(statement) = parse_statement_line_cst(&right_line)? {
            return Ok(Some(statement));
        }
    }

    Ok(None)
}
