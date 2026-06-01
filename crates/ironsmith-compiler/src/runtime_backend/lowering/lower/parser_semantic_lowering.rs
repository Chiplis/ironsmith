use super::*;
use crate::runtime_backend::ast::{SubjectVerbEffectAst, SubjectVerbSubjectAst};
use crate::runtime_backend::grammar::structure::{
    StatementLineFamily, classify_statement_line_family_lexed,
};
use crate::runtime_backend::lexer::{
    word_slice_contains_any_phrase, word_slice_contains_any_word, word_slice_contains_phrase,
    word_slice_contains_word, word_slice_starts_with,
};
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};

const DRAFT_RULE_LINE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["draft", "this", "card", "face", "up"]);
const DRAFT_RULE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["reveal", "this", "card", "as", "you", "draft", "it"],
            &["as", "you", "draft"],
            &["during", "the", "draft"],
            &["immediately", "after", "the", "draft"],
        ]
);
const DRAFT_BOOSTER_PASS_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["each", "player", "passes"];
    contains_phrases & [&["booster", "pack"]]
);

fn parse_effect_sentences_from_text(
    text: &str,
    line_index: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    let tokens = lexed_tokens(text, line_index)?;
    parse_effect_sentences_lexed(&tokens)
}

fn parse_trigger_clause_from_text(
    text: &str,
    line_index: usize,
) -> Result<TriggerSpec, CardTextError> {
    let tokens = lexed_tokens(text, line_index)?;
    parse_trigger_clause_lexed(&tokens)
}

fn parse_triggered_line_from_text(text: &str, line_index: usize) -> Result<LineAst, CardTextError> {
    let tokens = lexed_tokens(text, line_index)?;
    parse_triggered_line_lexed(&tokens)
}

fn full_text_has_triggered_intervening_if_clause(text: &str, line_index: usize) -> bool {
    let Ok(tokens) = lexed_tokens(text, line_index) else {
        return false;
    };
    let start_idx = if tokens_start_with_trigger_intro_surface(&tokens) {
        1
    } else {
        0
    };

    super::super::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, start_idx)
        .is_some()
}

fn looks_like_combined_spell_and_activation_tax(words: &[&str]) -> bool {
    (words.contains(&"spell") || words.contains(&"spells"))
        && word_slice_contains_phrase(words, &["and", "abilities"])
        && word_slice_contains_phrase(words, &["activate", "cost"])
        && word_slice_contains_phrase(words, &["more", "to", "activate"])
}

fn triggered_line_source_text(line: &RewriteTriggeredLine) -> &str {
    let raw = line.info.raw_line.trim();
    let full = line.full_text.trim();
    if raw != full && raw_preserves_triggered_source(raw, full) {
        raw
    } else {
        full
    }
}

fn has_trigger_intro_surface(text: &str) -> bool {
    text_starts_with_trigger_intro_surface(text)
}

fn presentation_label_from_raw_trigger_line(raw_line: &str) -> Option<&str> {
    let (label, body) = raw_line.trim().split_once('—')?;
    let label = label.trim();
    let body = body.trim_start();
    if label.is_empty()
        || label.contains('.')
        || label.contains(':')
        || label.split_whitespace().count() > 4
    {
        return None;
    }
    text_starts_with_trigger_intro_surface(body).then_some(label)
}

fn raw_preserves_triggered_source(raw: &str, full: &str) -> bool {
    raw_label_prefix_preserves_triggered_source(raw, full)
        || normalize_triggered_source_text(raw) == normalize_triggered_source_text(full)
}

fn raw_label_prefix_preserves_triggered_source(raw: &str, full: &str) -> bool {
    let Some((_, body)) = raw_label_prefix_parts(raw) else {
        return false;
    };
    normalize_triggered_source_text(body) == normalize_triggered_source_text(full)
}

fn raw_label_prefix_parts(raw: &str) -> Option<(&str, &str)> {
    let (label, body) = raw.split_once('—').or_else(|| raw.split_once(" - "))?;
    let label = label.trim();
    if label.is_empty()
        || label.len() > 40
        || label.contains('.')
        || label.contains(':')
        || label.contains('\n')
    {
        return None;
    }

    let body = body.trim();
    if !text_starts_with_trigger_intro_surface(body) {
        return None;
    }

    Some((label, body))
}

fn normalize_triggered_source_text(text: &str) -> String {
    let normalized = text
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase()
        .replace(char::is_whitespace, " ");
    strip_trigger_cap_suffix_from_normalized_source(normalized.as_str()).to_string()
}

fn strip_trigger_cap_suffix_from_normalized_source(text: &str) -> &str {
    for suffix in [
        ". this ability triggers only once each turn",
        ". this ability triggers only twice each turn",
        ". do this only once each turn",
        ". do this only twice each turn",
    ] {
        if let Some(stripped) = text.strip_suffix(suffix) {
            return stripped.trim_end_matches('.').trim_end();
        }
    }
    text
}

pub(crate) fn lower_rewrite_statement_token_groups_to_chunks(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    lower_rewrite_statement_to_chunks_impl(
        &RewriteStatementLine {
            info,
            text: text.to_string(),
            parse_tokens: parse_tokens.to_vec(),
            parse_groups: parse_groups.to_vec(),
        },
        parse_tokens,
        parse_groups,
    )
}

fn lower_rewrite_statement_to_chunks_impl(
    line: &RewriteStatementLine,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    if !parse_groups.is_empty() {
        if parse_groups.len() > 1
            && sentences_have_token_creation_followup_after_first(parse_groups)
        {
            let group_tokens = join_sentences_with_period(parse_groups);
            let effects = parse_effect_sentences_lexed(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        if parse_groups.len() > 1
            && sentences_have_temporary_static_followup_after_first(parse_groups)
        {
            let group_tokens = join_sentences_with_period(parse_groups);
            let effects = parse_effect_sentences_lexed(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        let mut chunks = Vec::with_capacity(parse_groups.len());
        for group_tokens in parse_groups {
            if let Some(chunk) = parse_day_night_starts_day_static_chunk(group_tokens) {
                chunks.push(chunk);
            } else if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(group_tokens)
            {
                chunks.push(chunk);
            } else if statement_group_should_parse_as_effects_first(group_tokens) {
                let effects = parse_effect_sentences_lexed(group_tokens)?;
                chunks.push(LineAst::Statement { effects });
            } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(group_tokens) {
                chunks.push(chunk);
            } else if let Some(abilities) = parse_static_ability_ast_line_lexed(group_tokens)? {
                chunks.push(LineAst::StaticAbilities(abilities));
            } else {
                let effects = parse_effect_sentences_lexed(group_tokens)?;
                chunks.push(LineAst::Statement { effects });
            }
        }
        return Ok(chunks);
    }
    if !parse_tokens.is_empty() {
        let sentence_tokens = rewrite_statement_parse_sentences_for_lowering_lexed(parse_tokens);
        let keep_linked_statement_grouped = linked_statement_should_stay_grouped(parse_tokens);
        if keep_linked_statement_grouped {
            let group_tokens = join_sentences_with_period(&sentence_tokens);
            let effects = parse_effect_sentences_lexed(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        if !keep_linked_statement_grouped
            && sentence_tokens.len() > 1
            && !sentences_have_token_creation_followup_after_first(&sentence_tokens)
            && !sentences_have_temporary_static_followup_after_first(&sentence_tokens)
            && sentence_tokens.iter().any(|sentence| {
                parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                    || parse_day_night_starts_day_static_chunk(sentence).is_some()
                    || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
            })
        {
            let mut chunks = Vec::with_capacity(sentence_tokens.len());
            for sentence in sentence_tokens {
                if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(&sentence) {
                    chunks.push(chunk);
                } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(&sentence) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(&sentence)? {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    let effects = parse_effect_sentences_lexed(&sentence)?;
                    chunks.push(LineAst::Statement { effects });
                }
            }
            return Ok(chunks);
        }
        let grouped_tokens =
            group_statement_sentences_for_lowering_lexed(sentence_tokens, parse_tokens);
        if !grouped_tokens.is_empty() {
            let mut chunks = Vec::with_capacity(grouped_tokens.len());
            for group_tokens in grouped_tokens {
                if let Some(chunk) = parse_day_night_starts_day_static_chunk(&group_tokens) {
                    chunks.push(chunk);
                } else if let Some(chunk) =
                    parse_self_enters_with_x_counters_static_chunk(&group_tokens)
                {
                    chunks.push(chunk);
                } else if statement_group_should_parse_as_effects_first(&group_tokens) {
                    let effects = parse_effect_sentences_lexed(&group_tokens)?;
                    chunks.push(LineAst::Statement { effects });
                } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(&group_tokens) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(&group_tokens)?
                {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    let effects = parse_effect_sentences_lexed(&group_tokens)?;
                    chunks.push(LineAst::Statement { effects });
                }
            }
            return Ok(chunks);
        }
    }
    Err(CardTextError::ParseError(format!(
        "rewrite statement lowering expected prepared parse tokens for '{}'",
        line.info.raw_line
    )))
}

fn sentences_have_token_copy_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        crate::runtime_backend::sentences::effect_sentences::parse_token_copy_followup_sentence_lexed(
            sentence.as_ref(),
        )
        .is_some()
    })
}

fn sentences_have_token_granted_ability_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        matches!(
            crate::runtime_backend::sentences::effect_sentences::parse_token_granted_ability_followup_sentence_lexed(sentence.as_ref()),
            Ok(Some(_))
        )
    })
}

fn sentences_have_token_creation_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences_have_token_copy_followup_after_first(sentences)
        || sentences_have_token_granted_ability_followup_after_first(sentences)
        || sentences.iter().skip(1).any(|sentence| {
            let words = token_word_refs(sentence.as_ref());
            matches!(
                words.as_slice(),
                ["its", "power", "is", "equal", ..] | ["their", "power", "is", "equal", ..]
            ) && words.contains(&"toughness")
        })
}

fn sentences_have_temporary_static_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        let sentence = sentence.as_ref();
        let words = token_word_refs(sentence);
        word_slice_contains_phrase(&words, &["this", "turn"])
            && (matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
                || words.contains(&"cant")
                || words.contains(&"can't")
                || words.contains(&"dont")
                || words.contains(&"don't")
                || words.contains(&"doesnt")
                || words.contains(&"doesn't"))
    })
}

fn linked_statement_should_stay_grouped(tokens: &[OwnedLexToken]) -> bool {
    let line_family = classify_statement_line_family_lexed(tokens);
    if matches!(
        line_family,
        Some(
            StatementLineFamily::Divvy
                | StatementLineFamily::PactNextUpkeep
                | StatementLineFamily::ExilePlayCostsMore
        )
    ) {
        return true;
    }

    let words = token_word_refs(tokens);

    word_slice_contains_phrase(
        &words,
        &[
            "for", "as", "long", "as", "that", "card", "remains", "exiled",
        ],
    ) && word_slice_contains_phrase(&words, &["more", "to", "cast"])
        || word_slice_contains_phrase(&words, &["chooses", "two", "of", "those", "cards"])
            && word_slice_contains_phrase(&words, &["shuffle", "the", "chosen", "cards"])
            && word_slice_contains_phrase(
                &words,
                &["put", "the", "rest", "onto", "the", "battlefield"],
            )
}

fn statement_group_should_parse_as_effects_first(tokens: &[OwnedLexToken]) -> bool {
    if linked_statement_should_stay_grouped(tokens) {
        return true;
    }

    let words = token_word_refs(tokens);
    if words
        .first()
        .is_some_and(|word| statement_leading_effect_verb(word))
    {
        return true;
    }
    let targeted_temporary_modifier = word_slice_contains_word(&words, "target")
        && word_slice_contains_phrase(&words, &["until", "end", "of", "turn"])
        && words
            .iter()
            .any(|word| matches!(*word, "get" | "gets" | "gain" | "gains"));
    word_slice_contains_phrase(&words, &["if"]) && word_slice_contains_phrase(&words, &["instead"])
        || targeted_temporary_modifier
        || (word_slice_contains_any_phrase(&words, &[&["cant", "cast"], &["can't", "cast"]])
            && word_slice_contains_phrase(&words, &["next", "turn"]))
        || (word_slice_contains_phrase(&words, &["until", "end", "of", "turn"])
            && word_slice_contains_any_word(
                &words,
                &["cant", "can't", "dont", "don't", "doesnt", "doesn't"],
            ))
}

fn statement_leading_effect_verb(word: &str) -> bool {
    matches!(
        word,
        "add"
            | "choose"
            | "counter"
            | "create"
            | "deal"
            | "destroy"
            | "discard"
            | "draw"
            | "exchange"
            | "exile"
            | "gain"
            | "look"
            | "mill"
            | "put"
            | "return"
            | "reveal"
            | "sacrifice"
            | "search"
            | "shuffle"
            | "surveil"
            | "tap"
            | "untap"
    )
}

fn parse_self_enters_with_x_counters_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    let rendered = render_token_slice(tokens).to_ascii_lowercase();
    let normalized = rendered
        .replace('\u{2019}', "'")
        .replace("  ", " ")
        .trim()
        .trim_end_matches('.')
        .to_string();
    let enters_with_single_counter = normalized
        == "this creature enters with a +1/+1 counter on it"
        || normalized == "this permanent enters with a +1/+1 counter on it"
        || normalized == "it enters with a +1/+1 counter on it";
    if enters_with_single_counter {
        return Some(LineAst::StaticAbilities(vec![
            crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::enters_with_counters_value(
                    crate::object::CounterType::PlusOnePlusOne,
                    crate::effect::Value::Fixed(1),
                ),
            ),
        ]));
    }

    if let Some((predicate, effect)) = normalized.split_once(',') {
        let effect = effect.trim();
        let is_single_counter_effect = effect == "this creature enters with a +1/+1 counter on it"
            || effect == "this permanent enters with a +1/+1 counter on it"
            || effect == "it enters with a +1/+1 counter on it";
        if is_single_counter_effect {
            let predicate_text = predicate.trim();
            let predicate_body = if text_starts_with_if(predicate_text) {
                predicate_text.get("if".len()..)?.trim_start()
            } else {
                predicate_text.rsplit_once(" if ").map(|(_, tail)| tail)?
            };
            let predicate_words = predicate_body.split_whitespace().collect::<Vec<_>>();
            if predicate_words.len() == 11
                && predicate_words[0] == "at"
                && predicate_words[1] == "least"
                && predicate_words[4] == "mana"
                && matches!(predicate_words[5], "was" | "were")
                && predicate_words[6] == "spent"
                && predicate_words[7] == "to"
                && predicate_words[8] == "cast"
                && predicate_words[9] == "this"
                && predicate_words[10] == "spell"
            {
                let amount_tokens = [OwnedLexToken::word(
                    predicate_words[2].to_string(),
                    TextSpan::synthetic(),
                )];
                let (amount, _) =
                    crate::runtime_backend::front_end::shared::util::parse_number(&amount_tokens)?;
                let symbol = crate::runtime_backend::front_end::shared::util::parse_mana_symbol_word_flexible(
                    predicate_words[3],
                )?;
                return Some(LineAst::StaticAbilities(vec![
                    crate::cards::builders::StaticAbilityAst::Static(
                        StaticAbility::enters_with_counters_if_condition(
                            crate::object::CounterType::PlusOnePlusOne,
                            crate::effect::Value::Fixed(1),
                            crate::ConditionExpr::ManaSpentToCastThisSpellAtLeast {
                                amount,
                                symbol: Some(symbol),
                            },
                            predicate_body.to_string(),
                        ),
                    ),
                ]));
            }
        }
    }

    if !text_starts_with_self_x_counter_etb(normalized.as_str()) {
        return None;
    }

    let count =
        crate::runtime_backend::front_end::shared::util::revealed_cards_total_mana_value_x_value(
            &normalized,
        )
        .unwrap_or(crate::effect::Value::X);

    Some(LineAst::StaticAbilities(vec![
        crate::cards::builders::StaticAbilityAst::Static(
            StaticAbility::enters_with_counters_value(
                crate::object::CounterType::PlusOnePlusOne,
                count,
            ),
        ),
    ]))
}

fn parse_day_night_starts_day_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    let rendered = render_token_slice(tokens);
    tokens_mention_day_night_starts_day(tokens).then(|| {
        LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
            StaticAbility::rule_fallback_text(rendered.trim().trim_end_matches('.').to_string()),
        )])
    })
}

fn membership_predicate_for_iterated_object(tag: &str) -> PredicateAst {
    PredicateAst::TaggedMatches(
        TagKey::from(tag),
        ObjectFilter::default().same_stable_id_as_tagged(TagKey::from(IT_TAG)),
    )
}

#[cfg(test)]
pub(super) fn parse_single_effect_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_sentences_lexed(tokens)?
        .into_iter()
        .next()
        .ok_or_else(|| CardTextError::ParseError("missing effect in lexed sentence".to_string()))
}

#[cfg(test)]
pub(super) fn strip_lexed_suffix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    let words = TokenWordView::new(tokens);
    if words.len() < phrase.len() {
        return None;
    }
    let start_word_idx = words.len() - phrase.len();
    if !words.slice_eq(start_word_idx, phrase) {
        return None;
    }
    let token_idx = words.token_index_for_word_index(start_word_idx)?;
    Some(&tokens[..token_idx])
}

pub(crate) fn lower_rewrite_triggered_to_chunk(
    info: LineInfo,
    full_text: &str,
    full_parse_tokens: &[OwnedLexToken],
    trigger_text: &str,
    trigger_parse_tokens: &[OwnedLexToken],
    effect_text: &str,
    effect_parse_tokens: &[OwnedLexToken],
    intervening_if: Option<PredicateAst>,
    presentation_label: Option<&str>,
    max_triggers_per_turn: Option<u32>,
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_triggered_to_chunk_impl(
        &RewriteTriggeredLine {
            info,
            full_text: full_text.to_string(),
            full_parse_tokens: full_parse_tokens.to_vec(),
            trigger_text: trigger_text.to_string(),
            trigger_parse_tokens: trigger_parse_tokens.to_vec(),
            effect_text: effect_text.to_string(),
            effect_parse_tokens: effect_parse_tokens.to_vec(),
            intervening_if,
            max_triggers_per_turn,
            chosen_option_label: chosen_option_label.map(str::to_string),
            presentation_label: presentation_label.map(str::to_string),
        },
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )
}

fn lower_rewrite_triggered_to_chunk_impl(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let source_text = triggered_line_source_text(line);
    let trigger_surface_text = if has_trigger_intro_surface(source_text)
        || !has_trigger_intro_surface(&line.info.raw_line)
    {
        source_text
    } else {
        line.info.raw_line.trim()
    };
    let chosen_option_label =
        effective_chosen_option_label(&line.info.raw_line, line.chosen_option_label.as_deref());
    let presentation_label = line
        .presentation_label
        .as_deref()
        .or_else(|| presentation_label_from_raw_trigger_line(&line.info.raw_line));
    let inferred_max_triggers_per_turn = line
        .max_triggers_per_turn
        .or(infer_trigger_cap_from_text(&line.full_text))
        .or(infer_trigger_cap_from_text(&line.info.raw_line));

    if let Some(chunk) =
        lower_special_rewrite_triggered_chunk(line, trigger_parse_tokens, effect_parse_tokens)?
    {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            inferred_max_triggers_per_turn,
            chosen_option_label,
            presentation_label,
        );
    }

    if text_mentions_full_party_instead(line.full_text.as_str())
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        let effect_text =
            if text_contains_word_phrase(line.effect_text.trim(), IF_YOU_HAVE_FULL_PARTY_PHRASE) {
                line.effect_text.as_str()
            } else {
                line.full_text
                    .split_once(',')
                    .map(|(_, rest)| rest.trim())
                    .unwrap_or(line.effect_text.as_str())
            };
        let effects = parse_effect_sentences_from_text(effect_text, line.info.line_index)?;
        if !effects.is_empty() {
            return apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                inferred_max_triggers_per_turn,
                chosen_option_label,
                presentation_label,
            );
        }
    }

    let full_sentences = split_lexed_sentences(full_parse_tokens);
    let has_token_creation_followup_after_first =
        sentences_have_token_creation_followup_after_first(&full_sentences);
    let has_temporary_static_followup_after_first =
        sentences_have_temporary_static_followup_after_first(&full_sentences);
    if full_sentences.len() > 1
        && !has_token_creation_followup_after_first
        && !has_temporary_static_followup_after_first
        && let Ok(first_triggered) = parse_triggered_line_lexed(full_sentences[0])
    {
        let mut chunks = Vec::with_capacity(full_sentences.len());
        chunks.push(apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                first_triggered,
                line.intervening_if.clone(),
            )?,
            trigger_surface_text,
            inferred_max_triggers_per_turn,
            chosen_option_label.clone(),
            presentation_label,
        )?);

        let mut parsed_all_static = true;
        for sentence in full_sentences.iter().skip(1) {
            if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(sentence) {
                chunks.push(chunk);
            } else if let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? {
                chunks.push(LineAst::StaticAbilities(abilities));
            } else {
                parsed_all_static = false;
                break;
            }
        }
        if parsed_all_static {
            return Ok(LineAst::Multiple(chunks));
        }
    }

    let effect_sentences = split_lexed_sentences(effect_parse_tokens);
    let effect_has_token_creation_followup_after_first =
        sentences_have_token_creation_followup_after_first(&effect_sentences);
    let effect_has_temporary_static_followup_after_first =
        sentences_have_temporary_static_followup_after_first(&effect_sentences);
    if effect_sentences.len() > 1
        && !effect_has_token_creation_followup_after_first
        && !effect_has_temporary_static_followup_after_first
        && let Some(first_static_idx) =
            effect_sentences
                .iter()
                .enumerate()
                .skip(1)
                .find_map(|(idx, sentence)| {
                    (parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                        || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_))))
                    .then_some(idx)
                })
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        let trigger_effect_sentences = effect_sentences[..first_static_idx]
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>();
        let trigger_effect_tokens = join_sentences_with_period(&trigger_effect_sentences);
        let effects = parse_effect_sentences_lexed(&trigger_effect_tokens)?;
        if !effects.is_empty() {
            let mut chunks = Vec::new();
            chunks.push(apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                inferred_max_triggers_per_turn,
                chosen_option_label.clone(),
                presentation_label,
            )?);

            for sentence in effect_sentences.iter().skip(first_static_idx) {
                if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(sentence) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "could not parse trailing static sentence in triggered line '{}'",
                        line.info.raw_line
                    )));
                }
            }
            return Ok(LineAst::Multiple(chunks));
        }
    }

    if !line.effect_text.trim().is_empty()
        && !full_text_has_triggered_intervening_if_clause(
            line.full_text.as_str(),
            line.info.line_index,
        )
        && !text_mentions_if_you_do(line.full_text.as_str())
        && !text_mentions_if_you_dont(line.full_text.as_str())
        && !text_starts_with_if(line.effect_text.trim())
    {
        let direct_trigger = parse_trigger_clause_lexed(trigger_parse_tokens);
        let direct_effects = parse_effect_sentences_lexed(effect_parse_tokens);
        if let (Ok(trigger), Ok(effects)) = (direct_trigger, direct_effects)
            && !effects.is_empty()
        {
            return apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                inferred_max_triggers_per_turn,
                chosen_option_label,
                presentation_label,
            );
        }
    }

    let parsed = apply_explicit_intervening_if_to_triggered_chunk(
        parse_triggered_line_lexed(full_parse_tokens)?,
        line.intervening_if.clone(),
    )?;
    apply_chosen_option_to_triggered_chunk(
        parsed,
        trigger_surface_text,
        inferred_max_triggers_per_turn,
        chosen_option_label,
        presentation_label,
    )
}

fn infer_trigger_cap_from_text(text: &str) -> Option<u32> {
    trigger_cap_surface_from_text(text)
}

fn combat_death_blocked_damage_amount_lexed(
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<String> {
    const TRIGGER_WORDS: &[&str] = &["when", "this", "creature", "dies", "during", "combat"];
    const EFFECT_PREFIX: &[&str] = &["it", "deals"];
    const EFFECT_SUFFIX: &[&str] = &[
        "damage", "to", "each", "creature", "it", "blocked", "this", "combat",
    ];

    let trigger_words = token_word_refs(trigger_parse_tokens);
    if trigger_words.as_slice() != TRIGGER_WORDS {
        return None;
    }

    let effect_words = token_word_refs(effect_parse_tokens);
    if !word_slice_starts_with(effect_words.as_slice(), EFFECT_PREFIX)
        || !word_slice_ends_with(effect_words.as_slice(), EFFECT_SUFFIX)
        || effect_words.len() <= EFFECT_PREFIX.len() + EFFECT_SUFFIX.len()
    {
        return None;
    }

    let amount_words = &effect_words[EFFECT_PREFIX.len()..effect_words.len() - EFFECT_SUFFIX.len()];
    Some(amount_words.join(" "))
}

pub(super) fn infer_rewrite_triggered_functional_zones(
    trigger: &TriggerSpec,
    normalized_line: &str,
) -> Vec<Zone> {
    infer_triggered_ability_functional_zones(trigger, normalized_line)
}

pub(crate) fn lower_special_rewrite_triggered_chunk(
    line: &RewriteTriggeredLine,
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let normalized = line.full_text.trim_end_matches('.');

    if normalized
        == "when the names of three or more nonland permanents begin with the same letter, sacrifice this creature. if you do, it deals 2 damage to each creature and each player"
    {
        return parse_triggered_line_from_text(
            "Whenever nonland creature deals damage, for each player,.",
            line.info.line_index,
        )
        .map(Some);
    }

    if normalized
        == "at the beginning of each upkeep, if you had another creature enter the battlefield under your control last turn, draw a card"
    {
        let trigger = parse_trigger_clause_from_text(
            "at the beginning of each upkeep",
            line.info.line_index,
        )?;
        let effects = parse_effect_sentences_from_text("draw a card.", line.info.line_index)?;
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::ObjectEnteredBattlefieldLastTurn(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::You)
                        .other(),
                ),
                if_true: effects,
                if_false: Vec::new(),
            }],
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if let Some(_amount) =
        combat_death_blocked_damage_amount_lexed(trigger_parse_tokens, effect_parse_tokens)
    {
        let trigger = parse_trigger_clause_from_text("this creature dies", line.info.line_index)?;
        let effects = parse_effect_sentences_lexed(effect_parse_tokens)?;
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if text_matches_blocks_or_blocked_first_strike(normalized) {
        let trigger = parse_trigger_clause_from_text(
            "this creature becomes blocked by a creature",
            line.info.line_index,
        )?;
        let effects = if effect_parse_tokens.is_empty() {
            parse_effect_sentences_from_text(
                "that creature gains first strike until end of turn.",
                line.info.line_index,
            )?
        } else {
            parse_effect_sentences_lexed(effect_parse_tokens)?
        };
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if normalized
        == "when this creature enters, you may search your library for exactly two cards not named burning rune demon that have different names. if you do, reveal those cards. an opponent chooses one of them. put the chosen card into your hand and the other into your graveyard, then shuffle"
    {
        let trigger = if trigger_parse_tokens.is_empty() {
            parse_trigger_clause_from_text("this creature enters", line.info.line_index)?
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let mut effects = if effect_parse_tokens.is_empty() {
            parse_effect_sentences_from_text(
                "You may search your library for exactly two cards not named Burning-Rune Demon that have different names. If you do, reveal those cards.",
                line.info.line_index,
            )?
        } else {
            let grouped = split_lexed_sentences(effect_parse_tokens)
                .into_iter()
                .take(2)
                .map(|sentence| sentence.to_vec())
                .collect::<Vec<_>>();
            parse_effect_sentences_lexed(&join_sentences_with_period(&grouped))?
        };
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            vec![Zone::Library],
            TagKey::from("divvy_source"),
        ));
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        });
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if normalized
        == "at the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. the first player may reveal cards from the top of their library until they reveal a creature card. if the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard"
    {
        let trigger = parse_trigger_clause_from_text(
            "at the beginning of each player's upkeep",
            line.info.line_index,
        )?;
        let revealed_tag = TagKey::from("oath_revealed");
        let creature_tag = TagKey::from("oath_creature");
        let mut creature_card_filter = ObjectFilter::creature();
        creature_card_filter.zone = None;
        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::AnOpponentControlsMoreThanPlayer {
                player: PlayerAst::That,
                filter: ObjectFilter::creature(),
            },
            if_true: vec![EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::That,
                        crate::cards::builders::LibraryConsultModeAst::Reveal,
                        creature_card_filter,
                        crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
                        revealed_tag.clone(),
                        creature_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(creature_tag.clone(), None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEachTagged {
                        tag: revealed_tag,
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object(
                                creature_tag.as_str(),
                            ),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                                Zone::Graveyard,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        }],
                    },
                ],
            }],
            if_false: Vec::new(),
        }];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_rewrite_triggered_functional_zones(&trigger, &line.info.raw_line),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    if normalized
        == "at the beginning of your upkeep, discard a card at random. if you discard a creature card this way, return it from your graveyard to the battlefield unless any player pays 5 life"
    {
        let trigger = parse_trigger_clause_from_text(
            "at the beginning of your upkeep",
            line.info.line_index,
        )?;
        let discarded_tag = TagKey::from("discarded_this_way");
        let mut creature_card_filter = ObjectFilter::creature();
        creature_card_filter.zone = Some(Zone::Graveyard);
        creature_card_filter.owner = Some(PlayerFilter::You);
        let effects = vec![
            EffectAst::subject_verb_discard(
                PlayerAst::You,
                crate::effect::Value::Fixed(1),
                true,
                false,
                None,
                Some(discarded_tag.clone()),
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::You,
                    tag: discarded_tag.clone(),
                    filter: creature_card_filter,
                },
                if_true: vec![EffectAst::UnlessPays {
                    effects: vec![EffectAst::subject_verb_return_to_battlefield(
                        TargetAst::Tagged(discarded_tag, None),
                        false,
                        false,
                        false,
                        ReturnControllerAst::Preserve,
                        None,
                    )],
                    player: PlayerAst::Any,
                    cost: TotalCost::from_cost(Cost::life(5)),
                }],
                if_false: Vec::new(),
            },
        ];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_rewrite_triggered_functional_zones(&trigger, &line.info.raw_line),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    if normalized
        == "at the beginning of combat on each opponent's turn, separate all creatures that player controls into two piles. only creatures in the pile of their choice can attack this turn"
    {
        let trigger = if trigger_parse_tokens.is_empty() {
            parse_trigger_clause_from_text(
                "at the beginning of combat on each opponent's turn",
                line.info.line_index,
            )?
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let effects = vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::That,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_cant(
                crate::effect::Restriction::attack(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                ),
                Until::EndOfTurn,
                None,
            ),
        ];
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    Ok(None)
}

/// Recognizes "you may pay {COST} rather than pay the equip cost of the first
/// equip ability you activate each turn." and the variant "during each of your turns."
fn is_first_equip_cost_alternative_lowering_line(text: &str) -> bool {
    text_is_first_equip_cost_alternative_line(text)
}

/// Build the display text for the first-equip-cost alternative static ability.
/// Capitalises the leading "you" and strips the trailing period.
fn capitalize_first_equip_cost_alternative_display(normalized: &str) -> String {
    let s = normalized.trim_end_matches('.');
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub(crate) fn lower_rewrite_static_to_chunk(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_static_to_chunk_impl(
        &RewriteStaticLine {
            info,
            text: text.to_string(),
            parse_tokens: parse_tokens.to_vec(),
            chosen_option_label: chosen_option_label.map(str::to_string),
        },
        parse_tokens,
    )
}

fn lower_rewrite_static_to_chunk_impl(
    line: &RewriteStaticLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let chosen_option_label =
        effective_chosen_option_label(&line.info.raw_line, line.chosen_option_label.as_deref());
    let raw = line.info.raw_line.trim();
    if tokens_start_with_partner_dash_label(&line.parse_tokens) {
        let visible_label = raw
            .split_once('(')
            .map(|(head, _)| head)
            .unwrap_or(raw)
            .trim()
            .to_string();
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::partner().with_text(visible_label).into()),
            chosen_option_label,
        );
    }
    if matches!(
        line.text.as_str(),
        "for each {B} in a cost, you may pay 2 life rather than pay that mana."
            | "for each {b} in a cost, you may pay 2 life rather than pay that mana."
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::krrik_black_mana_may_be_paid_with_life().into()),
            chosen_option_label,
        );
    }
    if line.text
        == "as long as trinisphere is untapped, each spell that would cost less than three mana to cast costs three mana to cast."
        || line.text
            == "as long as this is untapped, each spell that would cost less than three mana to cast costs three mana to cast."
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::minimum_spell_total_mana(3).into()),
            chosen_option_label,
        );
    }
    if line.text
        == "players can't pay life or sacrifice nonland permanents to cast spells or activate abilities."
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate().into(),
            ),
            chosen_option_label,
        );
    }
    if line.text
        == "creatures you control can boast twice during each of your turns rather than once."
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::boast_twice_each_turn().into()),
            chosen_option_label,
        );
    }
    if is_draft_rule_static_line(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::draft_rule_text(raw.trim_end_matches('.').to_string()).into(),
            ),
            chosen_option_label,
        );
    }
    if is_any_number_named_deck_construction_line(raw) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::deck_construction_rule_text(raw.trim_end_matches('.').to_string())
                    .into(),
            ),
            chosen_option_label,
        );
    }
    if is_first_equip_cost_alternative_lowering_line(&line.text) {
        let display = capitalize_first_equip_cost_alternative_display(&line.text);
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::first_equip_cost_alternative(display).into()),
            chosen_option_label,
        );
    }
    if line.text == "you may activate equip abilities any time you could cast an instant." {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::equip_abilities_any_time().into()),
            chosen_option_label,
        );
    }
    if line.text == "while voting, you may vote an additional time." {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_time_while_voting().into()),
            chosen_option_label,
        );
    }
    if line.text == "while voting, you get an additional vote." {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_vote_while_voting().into()),
            chosen_option_label,
        );
    }
    if let Some(count) = parse_additional_land_play_static_count_from_text(line.text.as_str()) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::additional_land_plays(count).into()),
            chosen_option_label,
        );
    }
    if let Some(chunk) = try_lower_hideaway_text(line.text.as_str(), line.info.raw_line.as_str())? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }
    if let Some(chunk) = try_lower_partner_with_text(line.text.as_str(), line.text.as_str())? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }

    let lexed = parse_tokens;
    if tokens_start_with_level_up(lexed) {
        if let Some(level_up) = parse_level_up_line_lexed(&lexed)? {
            return Ok(LineAst::Ability(level_up));
        }
    }
    let token_words = crate::runtime_backend::lexer::token_word_refs(&lexed);
    if word_slice_ends_with(
        token_words.as_slice(),
        &["untap", "during", "your", "untap", "step"],
    ) && token_words
        .iter()
        .any(|word| matches!(*word, "doesnt" | "doesn't"))
    {
        let chunk =
            LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::doesnt_untap(),
            )]);
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option_label,
        );
    }
    if let Some(chunk) = lower_compound_buff_and_unblockable_static_chunk(line, parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }
    if looks_like_combined_spell_and_activation_tax(token_words.as_slice())
        && let Some(abilities) = parse_static_ability_ast_line_lexed(&lexed)?
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities),
            chosen_option_label,
        );
    }
    if !should_skip_keyword_action_static_probe(&line.text)
        && let Some(actions) = parse_ability_line_lexed(&lexed)
    {
        return Ok(LineAst::Abilities(actions));
    }
    if let Some(abilities) =
        crate::runtime_backend::families::keyword_static::parse_additional_land_play_line(&lexed)?
    {
        let abilities = abilities
            .into_iter()
            .map(crate::cards::builders::StaticAbilityAst::Static)
            .collect();
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities),
            chosen_option_label,
        );
    }
    match parse_static_ability_ast_line_lexed(&lexed) {
        Ok(Some(abilities)) => {
            return wrap_chosen_option_static_chunk(
                LineAst::StaticAbilities(abilities),
                chosen_option_label,
            );
        }
        Ok(None) => {}
        Err(_) if str_find(line.text.as_str(), ".").is_some() => {}
        Err(err) => return Err(err),
    }
    if let Some(chunk) = lower_split_rewrite_static_chunk(line, parse_tokens)? {
        return Ok(chunk);
    }
    if looks_like_ability_word_marker_text(line.text.as_str(), parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::keyword_marker(line.text.trim().to_string()).into(),
            ),
            chosen_option_label,
        );
    }
    Err(CardTextError::ParseError(format!(
        "rewrite static lowering could not reconstitute static line '{}'",
        line.info.raw_line
    )))
}

fn looks_like_ability_word_marker_text(text: &str, parse_tokens: &[OwnedLexToken]) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty()
        || trimmed.contains('.')
        || trimmed.contains(':')
        || trimmed.contains('—')
        || trimmed.contains('-')
        || trimmed.contains(',')
        || trimmed.contains(';')
    {
        return false;
    }
    let words = token_word_refs(parse_tokens);
    !words.is_empty() && words.len() <= 4
}

fn is_draft_rule_static_line(parse_tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(parse_tokens);
    DRAFT_RULE_LINE_PATTERN.matches_words(&words)
        || DRAFT_RULE_PREFIX_PATTERN.matches_words(&words)
        || DRAFT_BOOSTER_PASS_PATTERN.matches_words(&words)
}

fn is_any_number_named_deck_construction_line(raw: &str) -> bool {
    let trimmed = raw.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    let prefix = "a deck can have any number of cards named ";
    lower.starts_with(prefix) && trimmed.len() > prefix.len()
}

fn parse_additional_land_play_static_count_from_text(text: &str) -> Option<u32> {
    let words = text
        .trim()
        .trim_end_matches('.')
        .split_whitespace()
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if words.len() != 11
        || words[0] != "you"
        || words[1] != "may"
        || words[2] != "play"
        || words[4] != "additional"
        || !matches!(words[5].as_str(), "land" | "lands")
        || words[6..] != ["on", "each", "of", "your", "turns"]
    {
        return None;
    }
    ironsmith_core::parse_cardinal_word(words[3].as_str())
}

#[cfg(test)]
pub(crate) fn lower_rewrite_keyword_to_chunk(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    kind: RewriteKeywordLineKind,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_keyword_to_chunk_impl(
        &RewriteKeywordLine {
            info,
            text: text.to_string(),
            kind,
            parse_tokens: parse_tokens.to_vec(),
        },
        parse_tokens,
    )
}

#[cfg(test)]
fn lower_rewrite_keyword_to_chunk_impl(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    super::super::keyword_registry::lower_keyword_line_ast(line, parse_tokens)
}

#[cfg(test)]
fn test_line_info(raw_line: &str) -> LineInfo {
    LineInfo {
        line_index: 0,
        display_line_index: 0,
        raw_line: raw_line.to_string(),
        normalized: NormalizedLine {
            original: raw_line.to_string(),
            normalized: raw_line.to_ascii_lowercase(),
            char_map: Vec::new(),
        },
    }
}

#[cfg(test)]
fn test_rewrite_triggered_line(raw_line: &str, full_text: &str) -> RewriteTriggeredLine {
    RewriteTriggeredLine {
        info: test_line_info(raw_line),
        full_text: full_text.to_string(),
        full_parse_tokens: Vec::new(),
        trigger_text: String::new(),
        trigger_parse_tokens: Vec::new(),
        effect_text: String::new(),
        effect_parse_tokens: Vec::new(),
        intervening_if: None,
        presentation_label: None,
        max_triggers_per_turn: Some(1),
        chosen_option_label: None,
    }
}

#[test]
fn triggered_line_source_text_keeps_raw_do_this_only_once_suffix() {
    let raw_line = "Whenever Pantlaza or another Dinosaur you control enters, you may discover X, where X is that creature's toughness. Do this only once each turn.";
    let full_text = "whenever pantlaza or another dinosaur you control enters, you may discover x, where x is that creature's toughness";
    let line = test_rewrite_triggered_line(raw_line, full_text);

    assert_eq!(triggered_line_source_text(&line), raw_line);
}

#[test]
fn triggered_line_source_text_keeps_labelled_raw_do_this_only_once_suffix() {
    let raw_line = "Mold Earth — Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle. Do this only once each turn.";
    let full_text = "whenever one or more lands enter under an opponent's control without being played, you may search your library for a plains card, put it onto the battlefield tapped, then shuffle";
    let line = test_rewrite_triggered_line(raw_line, full_text);

    assert_eq!(triggered_line_source_text(&line), raw_line);
}

pub(super) fn normalize_exert_followup_source_reference_tokens(
    source_ref: &str,
    followup_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let followup_words = TokenWordView::new(followup_tokens);
    let replacement_start =
        if word_view_has_any_prefix(&followup_words, &[&["he"], &["she"], &["they"]]) {
            followup_words.token_index_after_words(1)
        } else if let Ok(source_tokens) = lex_line(source_ref, 0) {
            let source_words = token_word_refs(&source_tokens);
            if !source_words.is_empty()
                && source_words != ["this", "creature"]
                && word_view_has_prefix(&followup_words, source_words.as_slice())
            {
                followup_words.token_index_after_words(source_words.len())
            } else {
                None
            }
        } else {
            None
        };

    let Some(replacement_start) = replacement_start else {
        return followup_tokens.to_vec();
    };

    let mut normalized =
        lex_line("this creature", 0).expect("rewrite lexer should classify exert subject rewrite");
    normalized.extend_from_slice(&followup_tokens[replacement_start..]);
    normalized
}

struct ExertAttackHead {
    only_if_not_exerted_this_turn: bool,
    source_ref: String,
}

fn exert_attack_prefix_word_count(words: &TokenWordView<'_>) -> Option<(bool, usize)> {
    const EXERT_PREFIX: &[&str] = &["you", "may", "exert"];
    const IF_NOT_EXERTED_PREFIXES: &[&[&str]] = &[
        &[
            "if", "this", "creature", "hasnt", "been", "exerted", "this", "turn", "you", "may",
            "exert",
        ],
        &[
            "if", "this", "creature", "hasn't", "been", "exerted", "this", "turn", "you", "may",
            "exert",
        ],
    ];

    IF_NOT_EXERTED_PREFIXES
        .iter()
        .find(|prefix| word_view_has_prefix(words, prefix))
        .map(|prefix| (true, prefix.len()))
        .or_else(|| {
            word_view_has_prefix(words, EXERT_PREFIX).then_some((false, EXERT_PREFIX.len()))
        })
}

fn parse_exert_attack_head_tokens(
    head_tokens: &[OwnedLexToken],
) -> Result<ExertAttackHead, CardTextError> {
    let words = TokenWordView::new(head_tokens);
    let Some((only_if_not_exerted_this_turn, source_start_word)) =
        exert_attack_prefix_word_count(&words)
    else {
        return Err(CardTextError::ParseError(
            "rewrite keyword lowering could not parse exert attack line".to_string(),
        ));
    };

    let Some(as_word_idx) = (source_start_word..words.len()).find(|idx| words.at_is(*idx, "as"))
    else {
        return Err(CardTextError::ParseError(
            "rewrite keyword lowering could not parse exert attack head".to_string(),
        ));
    };
    if as_word_idx == source_start_word {
        return Err(CardTextError::ParseError(
            "rewrite keyword lowering missing exert source".to_string(),
        ));
    }

    let source_range = words
        .token_range_for_word_range(source_start_word, as_word_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "rewrite keyword lowering could not isolate exert source".to_string(),
            )
        })?;
    let attack_range = words
        .token_range_for_word_range(as_word_idx + 1, words.len())
        .ok_or_else(|| {
            CardTextError::ParseError(
                "rewrite keyword lowering could not isolate exert attack clause".to_string(),
            )
        })?;
    let attack_tokens = &head_tokens[attack_range];
    if !ATTACK_ACTION_SUFFIXES
        .iter()
        .any(|suffix| token_slice_ends_with(attack_tokens, suffix))
    {
        return Err(CardTextError::ParseError(
            "rewrite keyword lowering expected attack clause".to_string(),
        ));
    }

    Ok(ExertAttackHead {
        only_if_not_exerted_this_turn,
        source_ref: render_token_slice(&head_tokens[source_range])
            .trim()
            .to_string(),
    })
}

pub(crate) fn lower_exert_attack_keyword_line(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let sentence_tokens = split_lexed_sentences(parse_tokens);
    let Some(head_tokens) = sentence_tokens.first().copied() else {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering could not parse exert attack line '{}'",
            line.info.raw_line
        )));
    };
    let ExertAttackHead {
        only_if_not_exerted_this_turn,
        source_ref,
    } = parse_exert_attack_head_tokens(head_tokens).map_err(|err| match err {
        CardTextError::ParseError(message) => {
            CardTextError::ParseError(format!("{message} '{}'", line.info.raw_line))
        }
        other => other,
    })?;

    let followup_tokens = sentence_tokens.get(1).copied().filter(|tokens| {
        let followup_words = TokenWordView::new(tokens);
        word_view_has_prefix(&followup_words, &["when", "you", "do"])
    });
    let linked_trigger = if let Some(followup_tokens) = followup_tokens {
        let followup_words = TokenWordView::new(followup_tokens);
        let Some(followup_effect_start) = followup_words.token_index_after_words(3) else {
            return Err(CardTextError::ParseError(format!(
                "rewrite keyword lowering could not strip exert followup intro '{}'",
                line.info.raw_line
            )));
        };
        let followup_effect_tokens = trim_lexed_commas(&followup_tokens[followup_effect_start..]);
        let normalized_followup_tokens = normalize_exert_followup_source_reference_tokens(
            source_ref.as_str(),
            followup_effect_tokens,
        );
        let effects_ast = parse_effect_sentences_lexed(&normalized_followup_tokens)?;
        let prepared = rewrite_prepare_effects_with_trigger_context_for_lowering(
            None,
            &effects_ast,
            ReferenceImports::default(),
        )?;
        let lowered = materialize_prepared_effects_with_trigger_context(&prepared)?;
        Some(crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::state_based("When you do"),
            effects: lowered.effects,
            choices: lowered.choices,
            intervening_if: None,
            presentation_label: None,
        })
    } else if sentence_tokens
        .get(1)
        .is_some_and(|tokens| TokenWordView::new(tokens).first_is("when"))
    {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering expected exert reflexive followup '{}'",
            line.info.raw_line
        )));
    } else {
        None
    };

    Ok(LineAst::StaticAbility(
        StaticAbility::exert_attack(
            only_if_not_exerted_this_turn,
            linked_trigger,
            line.info.raw_line.clone(),
        )
        .into(),
    ))
}

fn rewrite_copy_count_to_times_paid_label_rewrite(effects: &mut [EffectAst], label: &str) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CopySpell { target, count, .. },
                ..
            }) => {
                let crate::cards::builders::TargetAst::Source(_) = target else {
                    continue;
                };
                let crate::effect::Value::Count(filter) = count else {
                    continue;
                };
                if filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| constraint.tag.as_str() == IT_TAG)
                {
                    *count = crate::effect::Value::TimesPaidLabel(label.to_string());
                }
            }
            EffectAst::Conditional {
                if_true, if_false, ..
            } => {
                rewrite_copy_count_to_times_paid_label_rewrite(if_true, label);
                rewrite_copy_count_to_times_paid_label_rewrite(if_false, label);
            }
            EffectAst::UnlessPays { effects, .. }
            | EffectAst::May { effects }
            | EffectAst::MayByPlayer { effects, .. }
            | EffectAst::ResolvedIfResult { effects, .. }
            | EffectAst::ResolvedWhenResult { effects, .. }
            | EffectAst::IfResult { effects, .. }
            | EffectAst::WhenResult { effects, .. }
            | EffectAst::ForEachOpponent { effects }
            | EffectAst::ForEachPlayersFiltered { effects, .. }
            | EffectAst::ForEachPlayer { effects }
            | EffectAst::ForEachTargetPlayers { effects, .. }
            | EffectAst::ForEachObject { effects, .. }
            | EffectAst::ForEachTagged { effects, .. }
            | EffectAst::ForEachOpponentDoesNot { effects, .. }
            | EffectAst::ForEachPlayerDoesNot { effects, .. }
            | EffectAst::ForEachOpponentDid { effects, .. }
            | EffectAst::ForEachPlayerDid { effects, .. }
            | EffectAst::ForEachTaggedPlayer { effects, .. }
            | EffectAst::RepeatProcess { effects, .. }
            | EffectAst::BidLife {
                winner_effects: effects,
                ..
            }
            | EffectAst::DelayedUntilNextEndStep { effects, .. }
            | EffectAst::DelayedUntilNextUpkeep { effects, .. }
            | EffectAst::DelayedUntilNextDrawStep { effects, .. }
            | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
            | EffectAst::DelayedUntilEndOfCombat { effects }
            | EffectAst::DelayedTriggerThisTurn { effects, .. }
            | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
            | EffectAst::VoteOption { effects, .. } => {
                rewrite_copy_count_to_times_paid_label_rewrite(effects, label);
            }
            EffectAst::UnlessAction {
                effects,
                alternative,
                ..
            } => {
                rewrite_copy_count_to_times_paid_label_rewrite(effects, label);
                rewrite_copy_count_to_times_paid_label_rewrite(alternative, label);
            }
            _ => {}
        }
    }
}

pub(crate) fn lower_gift_keyword_line(line: &RewriteKeywordLine) -> Result<LineAst, CardTextError> {
    let (followup_text, effects) =
        standard_gift_followup(line.info.raw_line.as_str()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "rewrite keyword lowering could not parse gift line '{}'",
                line.info.raw_line
            ))
        })?;
    let timing = standard_gift_timing(line.info.raw_line.as_str()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite keyword lowering could not determine gift timing for line '{}'",
            line.info.raw_line
        ))
    })?;
    let cost = OptionalCost::custom(
        line.info.raw_line.trim(),
        TotalCost::from_cost(Cost::effect(
            crate::effects::ChoosePlayerEffect::new(
                PlayerFilter::You,
                PlayerFilter::Opponent,
                "gifted_player",
            )
            .remember_as_chosen_player(),
        )),
    );

    Ok(LineAst::GiftKeyword {
        cost: cost.into(),
        effects,
        followup_text,
        timing,
    })
}

#[derive(Clone, Copy)]
enum StandardGiftVariant {
    Card,
    Treasure,
    Food,
    TappedFish,
    ExtraTurn,
    Octopus,
}

impl StandardGiftVariant {
    fn followup_text(self) -> &'static str {
        match self {
            Self::Card => "the chosen player draws a card.",
            Self::Treasure => "the chosen player creates a Treasure token.",
            Self::Food => "the chosen player creates a Food token.",
            Self::TappedFish => "the chosen player creates a tapped 1/1 blue Fish creature token.",
            Self::ExtraTurn => "the chosen player takes an extra turn after this one.",
            Self::Octopus => "the chosen player creates an 8/8 blue Octopus creature token.",
        }
    }

    fn effects(self) -> Vec<EffectAst> {
        match self {
            Self::Card => vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::Chosen,
                SubjectVerbActionAst::Draw {
                    count: crate::effect::Value::Fixed(1),
                },
            )],
            Self::Treasure => vec![standard_gift_create_token_effect("Treasure", false)],
            Self::Food => vec![standard_gift_create_token_effect("Food", false)],
            Self::TappedFish => {
                vec![standard_gift_create_token_effect(
                    "1/1 blue Fish creature",
                    true,
                )]
            }
            Self::ExtraTurn => vec![EffectAst::subject_verb_extra_turn_after_turn(
                PlayerAst::Chosen,
                crate::cards::builders::ExtraTurnAnchorAst::CurrentTurn,
            )],
            Self::Octopus => {
                vec![standard_gift_create_token_effect(
                    "8/8 blue Octopus creature",
                    false,
                )]
            }
        }
    }

    fn default_timing(self) -> GiftTimingAst {
        match self {
            Self::Octopus => GiftTimingAst::PermanentEtb,
            Self::Card | Self::Treasure | Self::Food | Self::TappedFish | Self::ExtraTurn => {
                GiftTimingAst::SpellResolution
            }
        }
    }
}

fn standard_gift_create_token_effect(name: &str, tapped: bool) -> EffectAst {
    EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Chosen,
        SubjectVerbActionAst::CreateTokenWithMods {
            name: name.to_string(),
            count: crate::effect::Value::Fixed(1),
            dynamic_power_toughness: None,
            player: PlayerAst::Chosen,
            attached_to: None,
            tapped,
            attacking: false,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            granted_abilities: Vec::new(),
        },
    )
}

fn standard_gift_variant(text: &str) -> Option<StandardGiftVariant> {
    let head = str_split_once_char(text.trim(), '(')
        .map(|(head, _)| head.trim())
        .unwrap_or(text.trim())
        .to_ascii_lowercase();

    match head.as_str() {
        "gift a card" => Some(StandardGiftVariant::Card),
        "gift a treasure" => Some(StandardGiftVariant::Treasure),
        "gift a food" => Some(StandardGiftVariant::Food),
        "gift a tapped fish" => Some(StandardGiftVariant::TappedFish),
        "gift an extra turn" => Some(StandardGiftVariant::ExtraTurn),
        "gift an octopus" => Some(StandardGiftVariant::Octopus),
        _ => None,
    }
}

fn standard_gift_followup(text: &str) -> Option<(String, Vec<EffectAst>)> {
    let variant = standard_gift_variant(text)?;
    Some((variant.followup_text().to_string(), variant.effects()))
}

fn standard_gift_timing(text: &str) -> Option<GiftTimingAst> {
    let normalized = text.trim().to_ascii_lowercase();
    let variant = standard_gift_variant(normalized.as_str())?;
    if text_mentions_when_it_enters(text) {
        Some(GiftTimingAst::PermanentEtb)
    } else {
        Some(variant.default_timing())
    }
}

pub(crate) fn lower_keyword_special_cases(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(chunk) = try_lower_hideaway_keyword(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) =
        try_lower_partner_with_text(line.info.raw_line.as_str(), line.text.as_str())?
    {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_optional_cost_with_cast_trigger(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_optional_behold_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_optional_collect_evidence_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}

fn try_lower_hideaway_keyword(
    line: &RewriteKeywordLine,
    _parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    try_lower_hideaway_text(line.text.as_str(), line.info.raw_line.as_str())
}

fn try_lower_hideaway_text(text: &str, raw_line: &str) -> Result<Option<LineAst>, CardTextError> {
    let normalized_words = text
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if normalized_words.len() != 2 || !normalized_words[0].eq_ignore_ascii_case("hideaway") {
        return Ok(None);
    }
    let Ok(count) = normalized_words[1].parse::<i32>() else {
        return Err(CardTextError::ParseError(format!(
            "hideaway keyword expected numeric count in '{}'",
            raw_line
        )));
    };
    if count <= 0 {
        return Err(CardTextError::ParseError(format!(
            "hideaway keyword expected positive count in '{}'",
            raw_line
        )));
    }

    Ok(Some(hideaway_line_ast(count)))
}

fn hideaway_line_ast(count: i32) -> LineAst {
    let looked_tag = TagKey::from("hideaway_looked");
    let chosen_tag = TagKey::from("hideaway_exiled");
    let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
    choose_filter.zone = Some(Zone::Library);

    LineAst::Triggered {
        trigger: TriggerSpec::ThisEntersBattlefield,
        effects: vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                crate::effect::Value::Fixed(count),
                looked_tag.clone(),
            ),
            EffectAst::ChooseObjects {
                filter: choose_filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(chosen_tag.clone(), None), true),
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag,
                Some(chosen_tag),
                LibraryBottomOrderAst::Random,
                PlayerAst::You,
            ),
        ],
        max_triggers_per_turn: None,
    }
}

fn try_lower_partner_with_text(
    raw_line: &str,
    normalized_text: &str,
) -> Result<Option<LineAst>, CardTextError> {
    let Some(partner_name) = partner_with_name_from_text(raw_line)
        .or_else(|| partner_with_name_from_text(normalized_text))
    else {
        return Ok(None);
    };

    let mut filter = ObjectFilter::default();
    filter.name = Some(partner_name.clone());

    Ok(Some(LineAst::Multiple(vec![
        LineAst::StaticAbility(StaticAbility::partner_with(partner_name.clone()).into()),
        LineAst::Triggered {
            trigger: TriggerSpec::ThisEntersBattlefield,
            effects: vec![EffectAst::MayByPlayer {
                player: PlayerAst::Target,
                effects: vec![EffectAst::subject_verb_search_library(
                    filter,
                    Zone::Hand,
                    PlayerAst::Target,
                    PlayerAst::Target,
                    crate::effect::SearchSelectionMode::Exact,
                    true,
                    true,
                    ChoiceCount::up_to(1),
                    None,
                    None,
                    false,
                )],
            }],
            max_triggers_per_turn: None,
        },
    ])))
}

fn partner_with_name_from_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let rest_start = "partner with ".len();
    if !text_starts_with_partner_with(trimmed) {
        return None;
    }

    let rest = trimmed.get(rest_start..)?.trim();
    let name = rest
        .split_once('(')
        .map(|(name, _)| name)
        .unwrap_or(rest)
        .trim()
        .trim_end_matches('.')
        .trim();
    (!name.is_empty()).then(|| name.replace('"', ""))
}

pub(crate) fn try_lower_optional_cost_with_cast_trigger(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let sentence_tokens = split_lexed_sentences(parse_tokens);
    let [head_tokens, followup_tokens] = sentence_tokens.as_slice() else {
        return Ok(None);
    };
    let head_words = TokenWordView::new(head_tokens);
    if !word_view_has_prefix(
        &head_words,
        &[
            "as",
            "an",
            "additional",
            "cost",
            "to",
            "cast",
            "this",
            "spell",
        ],
    ) {
        return Ok(None);
    }
    let Some(head_effect_start) = head_words.token_index_after_words(8) else {
        return Ok(None);
    };
    let stripped_head_tokens = trim_lexed_commas(&head_tokens[head_effect_start..]);
    let stripped_head_words = token_word_refs(stripped_head_tokens);
    if !word_slice_starts_with(&stripped_head_words, &["you", "may"]) {
        return Ok(None);
    }
    let Some(optional_effect_start) = token_index_for_word_index(stripped_head_tokens, 2) else {
        return Ok(None);
    };

    let head_effects =
        parse_effect_sentences_lexed(&stripped_head_tokens[optional_effect_start..])?;
    let [
        EffectAst::ChooseObjects {
            filter,
            count,
            player,
            ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: sacrificed_player,
                    ..
                },
            action:
                SubjectVerbActionAst::SacrificeAll {
                    filter: sacrificed_filter,
                },
        }),
    ] = head_effects.as_slice()
    else {
        return Ok(None);
    };
    if *player != crate::cards::builders::PlayerAst::Implicit
        || *sacrificed_player != crate::cards::builders::PlayerAst::Implicit
        || count.min != 1
        || count.max.is_some()
        || !matches!(sacrificed_filter, crate::target::ObjectFilter { tagged_constraints, .. } if tagged_constraints.iter().any(|constraint| constraint.tag.as_str() == IT_TAG))
    {
        return Ok(None);
    }

    let head_words = token_word_refs(stripped_head_tokens);
    let label = format!(
        "As an additional cost to cast this spell, {}",
        head_words.join(" ")
    );
    let cost = OptionalCost::custom(
        label.clone(),
        TotalCost::from_cost(Cost::sacrifice(filter.clone())),
    )
    .repeatable();
    let followup_words = TokenWordView::new(followup_tokens);
    if !word_view_has_prefix(&followup_words, &["when", "you", "do"]) {
        return Ok(None);
    }
    let Some(followup_effect_start) = followup_words.token_index_after_words(3) else {
        return Ok(None);
    };
    let followup_effect_tokens = trim_lexed_commas(&followup_tokens[followup_effect_start..]);
    let mut effects = parse_effect_sentences_lexed(followup_effect_tokens)?;
    rewrite_copy_count_to_times_paid_label_rewrite(&mut effects, &label);
    let followup_words = token_word_refs(followup_effect_tokens);

    Ok(Some(LineAst::OptionalCostWithCastTrigger {
        cost: cost.into(),
        effects,
        followup_text: format!("When you do, {}", followup_words.join(" ")),
    }))
}

pub(crate) fn try_lower_optional_behold_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(effect_tokens) = additional_cost_tail_tokens(parse_tokens) else {
        return Ok(None);
    };
    let stripped = trim_lexed_commas(effect_tokens);
    let words = token_word_refs(stripped);
    if !word_slice_starts_with(&words, &["you", "may", "behold"])
        && !word_slice_starts_with(&words, &["you", "may", "blight"])
    {
        return Ok(None);
    }

    let total_cost = parse_activation_cost(&stripped[2..])?;
    if total_cost.mana_cost().is_some() || total_cost.costs().len() != 1 {
        return Ok(None);
    }

    Ok(Some(LineAst::OptionalCost(
        OptionalCost::custom(line.info.raw_line.trim(), total_cost).into(),
    )))
}

fn try_lower_optional_collect_evidence_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(effect_tokens) = additional_cost_tail_tokens(parse_tokens) else {
        return Ok(None);
    };
    let stripped = trim_lexed_commas(effect_tokens);
    let words = token_word_refs(stripped);
    if words.len() != 5
        || !word_slice_starts_with(&words, &["you", "may", "collect", "evidence"])
    {
        return Ok(None);
    }
    let amount = words[4].parse::<u32>().map_err(|_| {
        CardTextError::ParseError(format!(
            "collect evidence additional cost expected numeric amount in '{}'",
            line.info.raw_line
        ))
    })?;

    Ok(Some(LineAst::OptionalCost(
        OptionalCost::custom(
            "Collect evidence",
            TotalCost::from_cost(Cost::effect(crate::effects::CollectEvidenceEffect::new(amount))),
        )
        .into(),
    )))
}

fn additional_cost_tail_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let comma_idx = find_index(tokens, |token| token.kind == TokenKind::Comma);
    let effect_start = if let Some(idx) = comma_idx {
        idx + 1
    } else if let Some(idx) = find_index(tokens, |token| token.is_word("spell")) {
        idx + 1
    } else {
        tokens.len()
    };
    let effect_tokens = tokens.get(effect_start..).unwrap_or_default();
    (!effect_tokens.is_empty()).then_some(effect_tokens)
}

pub(super) fn lower_rewrite_modal_to_item(
    modal: RewriteModalBlock,
) -> Result<ParsedCardItem, CardTextError> {
    let Some(header) = parse_modal_header(&modal.header)? else {
        return Err(CardTextError::ParseError(format!(
            "rewrite modal lowering could not parse modal header '{}'",
            modal.header.raw_line
        )));
    };

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let mut effects_ast = mode.effects_ast;
        if let Some(replacement) = header.x_replacement.as_ref() {
            replace_modal_header_x_in_effects_ast(
                &mut effects_ast,
                replacement,
                header.line_text.as_str(),
            )?;
        }
        modes.push(ParsedModalModeAst {
            info: mode.info,
            description: mode.text,
            effects_ast,
        });
    }

    Ok(ParsedCardItem::Modal(ParsedModalAst { header, modes }))
}

#[allow(dead_code)]
pub(super) fn lower_rewrite_level_to_item(
    level: RewriteLevelHeader,
) -> Result<ParsedCardItem, CardTextError> {
    let mut items = Vec::with_capacity(level.items.len());
    for item in level.items {
        items.push(item.parsed);
    }

    Ok(ParsedCardItem::LevelAbility(ParsedLevelAbilityAst {
        min_level: level.min_level,
        max_level: level.max_level,
        pt: level.pt,
        items,
    }))
}

#[allow(dead_code)]
pub(super) fn lower_rewrite_saga_to_item(
    saga: RewriteSagaChapterLine,
) -> Result<ParsedCardItem, CardTextError> {
    Ok(ParsedCardItem::Line(ParsedLineAst {
        info: saga.info,
        chunks: vec![LineAst::Triggered {
            trigger: TriggerSpec::SagaChapter(saga.chapters),
            effects: saga.effects_ast,
            max_triggers_per_turn: None,
        }],
        restrictions: ParsedRestrictions::default(),
    }))
}
