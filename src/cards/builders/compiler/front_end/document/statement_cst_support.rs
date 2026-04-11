use super::super::grammar::structure;
use super::*;

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
    let line_family = structure::classify_statement_line_family_lexed(&line.tokens);
    if matches!(line_family, Some(structure::StatementLineFamily::ArtRating)) {
        return Ok(None);
    }
    let force_statement = matches!(line_family, Some(structure::StatementLineFamily::Divvy))
        || matches!(
            line_family,
            Some(
                structure::StatementLineFamily::PactNextUpkeep
                    | structure::StatementLineFamily::ExilePlayCostsMore
            )
        )
        || looks_like_statement_line_lexed(line);
    if !force_statement
        && parse_static_ability_ast_line_lexed(&line.tokens)
            .ok()
            .flatten()
            .is_some()
    {
        return Ok(None);
    }
    if matches!(line_family, Some(structure::StatementLineFamily::Divvy)) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: Vec::new(),
        }));
    }
    if matches!(
        line_family,
        Some(
            structure::StatementLineFamily::PactNextUpkeep
                | structure::StatementLineFamily::ExilePlayCostsMore
        )
    ) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: Vec::new(),
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
    let parse_groups = normalize_statement_parse_groups_lexed(&line.tokens);
    let mut found_effects = false;
    for group_tokens in &parse_groups {
        let effects = match parse_effect_sentences_lexed(group_tokens) {
            Ok(effects) => effects,
            Err(err)
                if looks_like_statement_line_lexed(line)
                    || token_words_have_any_prefix(
                        group_tokens,
                        &[&["choose"], &["if"], &["reveal"]],
                    ) =>
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
        if !is_trigger_result_followup_line(line) {
            break;
        }

        let followup_text = render_token_slice(&line.tokens).trim().to_string();
        if !triggered.effect_text.is_empty() {
            triggered.effect_text.push('\n');
        }
        triggered.effect_text.push_str(followup_text.as_str());
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

fn looks_like_statement_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    if matches!(
        structure::classify_static_line_family_lexed(tokens),
        Some(
            structure::StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep
                | structure::StaticLineFamily::GrantedQuotedAbility
        )
    ) {
        return false;
    }
    matches!(
        structure::classify_statement_line_family_lexed(tokens),
        Some(
            structure::StatementLineFamily::PactNextUpkeep
                | structure::StatementLineFamily::NextTurnCantCast
                | structure::StatementLineFamily::Divvy
                | structure::StatementLineFamily::ArtRating
                | structure::StatementLineFamily::ExilePlayCostsMore
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

fn rewrite_statement_followup_intro_lexed(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    rewrite_followup_intro_to_if_lexed(tokens)
}

fn rewrite_copy_exception_type_removal_lexed(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    remove_copy_exception_type_removal_lexed(tokens)
}

fn normalize_statement_parse_sentences_lexed(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence_tokens| !sentence_tokens.is_empty())
        .map(strip_non_keyword_label_prefix_lexed)
        .map(rewrite_statement_followup_intro_lexed)
        .map(|tokens| rewrite_copy_exception_type_removal_lexed(&tokens))
        .filter(|tokens| !tokens.is_empty())
        .collect()
}

fn sentence_rewrite_contains_instead_split(tokens: &[OwnedLexToken]) -> bool {
    lexed_tokens_contain_non_prefix_instead(tokens)
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
                let fallback = strip_non_keyword_label_prefix_lexed(fallback_tokens);
                (!fallback.is_empty()).then(|| {
                    rewrite_copy_exception_type_removal_lexed(
                        &rewrite_statement_followup_intro_lexed(fallback),
                    )
                })
            })
            .unwrap_or_default();
        return (!only_sentence.is_empty())
            .then(|| join_statement_parse_sentence_group(&[only_sentence]))
            .into_iter()
            .collect();
    }

    let split_idx = sentence_tokens
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(idx, sentence)| {
            sentence_rewrite_contains_instead_split(sentence).then_some(idx)
        });

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

    let left = render_token_slice(left_tokens);
    let trimmed_left = left.trim();

    if trimmed_left.eq_ignore_ascii_case("reveal this card from your hand") {
        let left_line = rewrite_line_tokens(line, left_tokens);
        if let Some(statement) = parse_statement_line_cst(&left_line)? {
            return Ok(Some(statement));
        }
    }

    if !str_contains(trimmed_left, "{") && str_contains(trimmed_left, ",") {
        let right_line = rewrite_line_tokens(line, right_tokens);
        if let Some(statement) = parse_statement_line_cst(&right_line)? {
            return Ok(Some(statement));
        }
    }

    Ok(None)
}
