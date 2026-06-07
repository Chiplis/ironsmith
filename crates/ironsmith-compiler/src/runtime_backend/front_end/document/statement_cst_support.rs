use super::super::grammar::structure;
use super::super::lex_patterns::LexPattern;
use super::*;

const REVEAL_THIS_CARD_FROM_HAND_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["reveal", "this", "card", "from", "your", "hand"]),
]);
const DIE_ROLL_RESULT_ADJUSTMENT_PREFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["after", "you", "roll", "a", "die"])]);

fn is_die_roll_result_adjustment_statement(tokens: &[OwnedLexToken]) -> bool {
    DIE_ROLL_RESULT_ADJUSTMENT_PREFIX_PATTERN.matches_prefix(LexedClause::new(tokens))
        && contains_token_word_sequence(tokens, &["you", "may", "pay"])
        && contains_token_word_sequence(tokens, &["if", "you", "do"])
        && contains_token_word_sequence(
            tokens,
            &["increase", "or", "decrease", "the", "result", "by"],
        )
        && contains_token_word_sequence(tokens, &["do", "this", "only", "once", "each", "turn"])
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
    let line_family = structure::classify_statement_line_family_lexed(&line.tokens);
    let static_probe = parse_static_ability_ast_line_lexed(&line.tokens)
        .ok()
        .flatten();
    let force_statement = matches!(line_family, Some(structure::StatementLineFamily::Divvy))
        || matches!(
            line_family,
            Some(
                structure::StatementLineFamily::PactNextUpkeep
                    | structure::StatementLineFamily::ExilePlayCostsMore
                    | structure::StatementLineFamily::BidLife
            )
        )
        || (contains_token_word_sequence(
            &line.tokens,
            &["chooses", "two", "of", "those", "cards"],
        ) && contains_token_word_sequence(
            &line.tokens,
            &["shuffle", "the", "chosen", "cards"],
        ) && contains_token_word_sequence(
            &line.tokens,
            &["put", "the", "rest", "onto", "the", "battlefield"],
        ))
        || (contains_token_word_sequence(
            &line.tokens,
            &[
                "for", "as", "long", "as", "that", "card", "remains", "exiled",
            ],
        ) && contains_token_word_sequence(&line.tokens, &["more", "to", "cast"]))
        || (token_slice_starts_with_any(&line.tokens, &[&["if"]])
            && contains_token_word_sequence(&line.tokens, &["instead"])
            && static_probe.is_none())
        || (token_slice_starts_with_any(&line.tokens, &[&["each"], &["all"]])
            && contains_token_word_sequence(&line.tokens, &["until", "end", "of", "turn"]))
        || looks_like_statement_line_lexed(line);
    if !force_statement
        && static_probe.is_some()
    {
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
    if token_slice_starts_with_any(&line.tokens, &[&["the", "next", "time"]])
        && contains_token_word_sequence(&line.tokens, &["source", "of", "your", "choice"])
        && contains_token_word_sequence(&line.tokens, &["prevent", "that", "damage"])
        && contains_token_word_sequence(&line.tokens, &["damage", "is", "prevented", "this", "way"])
    {
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
        let effects = match parse_effect_sentences_lexed(group_tokens) {
            Ok(effects) => effects,
            Err(_)
                if matches!(
                    parse_static_ability_ast_line_lexed(group_tokens),
                    Ok(Some(_))
                ) =>
            {
                continue;
            }
            Err(err)
                if looks_like_statement_line_lexed(line)
                    || token_slice_starts_with_any(
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

fn looks_like_day_night_starts_day_as_enters_static_line(tokens: &[OwnedLexToken]) -> bool {
    token_slice_starts_with_any(tokens, &[&["if"]])
        && contains_token_word_sequence(tokens, &["neither", "day", "nor", "night"])
        && contains_token_word_sequence(tokens, &["it", "becomes", "day"])
        && (contains_token_word_sequence(tokens, &["as", "this", "creature", "enters"])
            || contains_token_word_sequence(tokens, &["as", "this", "permanent", "enters"])
            || contains_token_word_sequence(tokens, &["as", "this", "object", "enters"]))
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

        let followup_text = render_token_slice(&line.tokens).trim().to_string();
        if !activated.effect_text.is_empty() {
            activated.effect_text.push('\n');
        }
        activated.effect_text.push_str(followup_text.as_str());
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
    if matches!(
        structure::classify_static_line_family_lexed(tokens),
        Some(
            structure::StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep
                | structure::StaticLineFamily::GrantedQuotedAbility
        )
    ) {
        return false;
    }
    let effect_sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if !effect_sentences.is_empty()
        && effect_sentences.into_iter().all(|sentence| {
            parse_effect_sentences_lexed(sentence).is_ok_and(|effects| !effects.is_empty())
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

fn first_trailing_static_sentence_idx(sentence_tokens: &[Vec<OwnedLexToken>]) -> Option<usize> {
    let first_static_idx =
        sentence_tokens
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(idx, sentence)| {
                matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_))).then_some(idx)
            })?;

    if !sentence_tokens[..first_static_idx]
        .iter()
        .all(|sentence| parse_effect_sentences_lexed(sentence).is_ok())
    {
        return None;
    }
    if !sentence_tokens[first_static_idx..]
        .iter()
        .all(|sentence| matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_))))
    {
        return None;
    }

    Some(first_static_idx)
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

    if REVEAL_THIS_CARD_FROM_HAND_PATTERN.matches_clause(LexedClause::new(left_tokens)) {
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
