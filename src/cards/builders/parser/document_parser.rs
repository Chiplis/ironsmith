use crate::PtValue;
use crate::ability::ActivationTiming;
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, LineAst, ParseAnnotations, ParsedLevelAbilityItemAst,
};
use crate::cards::builders::{
    find_index as find_token_index, str_contains, str_ends_with, str_ends_with_char,
    str_split_once, str_split_once_char, str_starts_with, str_starts_with_char, str_strip_prefix,
    str_strip_suffix,
};

use super::activation_and_restrictions::{
    parse_channel_line_lexed, parse_cycling_line_lexed, parse_equip_line_lexed,
};
use super::clause_support::{
    parse_ability_line_lexed, parse_effect_sentences_lexed, parse_static_ability_ast_line_lexed,
    parse_trigger_clause_lexed, parse_triggered_line_lexed,
};
use super::cst::{
    ActivatedLineCst, KeywordLineCst, KeywordLineKindCst, LevelHeaderCst, LevelItemCst,
    LevelItemKindCst, MetadataLineCst, ModalBlockCst, ModalModeCst, RewriteDocumentCst,
    RewriteLineCst, SagaChapterLineCst, StatementLineCst, StaticLineCst, TriggerIntroCst,
    TriggeredLineCst, UnsupportedLineCst,
};
use super::grammar::primitives as grammar;
use super::grammar::structure::split_lexed_sentences;
use super::ir::{
    RewriteActivatedLine, RewriteKeywordLine, RewriteKeywordLineKind, RewriteLevelHeader,
    RewriteLevelItem, RewriteLevelItemKind, RewriteModalBlock, RewriteModalMode,
    RewriteSagaChapterLine, RewriteSemanticDocument, RewriteSemanticItem, RewriteStatementLine,
    RewriteStaticLine, RewriteTriggeredLine, RewriteUnsupportedLine,
};
use super::keyword_static::parse_if_this_spell_costs_less_to_cast_line_lexed;
use super::leaf::{lower_activation_cost_cst, parse_activation_cost_tokens_rewrite};
use super::lexer::{
    OwnedLexToken, TokenKind, lex_line, render_token_slice, token_word_refs, trim_lexed_commas,
};
use super::lower::{
    lower_rewrite_activated_to_chunk, lower_rewrite_keyword_to_chunk,
    lower_rewrite_statement_to_chunks, lower_rewrite_static_to_chunk,
    lower_rewrite_triggered_to_chunk,
};
use super::preprocess::{
    PreprocessedDocument, PreprocessedItem, PreprocessedLine, preprocess_document,
};
use super::util::{
    parse_additional_cost_choice_options_lexed, parse_bargain_line_lexed, parse_bestow_line_lexed,
    parse_buyback_line_lexed, parse_cast_this_spell_only_line_lexed, parse_entwine_line_lexed,
    parse_escape_line_lexed, parse_flashback_line_lexed, parse_harmonize_line_lexed,
    parse_if_conditional_alternative_cost_line_lexed, parse_kicker_line_lexed, parse_level_header,
    parse_level_up_line_lexed, parse_madness_line_lexed, parse_morph_keyword_line_lexed,
    parse_multikicker_line_lexed, parse_offspring_line_lexed, parse_power_toughness,
    parse_reinforce_line_lexed, parse_saga_chapter_prefix,
    parse_self_free_cast_alternative_cost_line_lexed, parse_squad_line_lexed,
    parse_transmute_line_lexed, parse_warp_line_lexed,
    parse_you_may_rather_than_spell_cost_line_lexed, preserve_keyword_prefix_for_parse,
};

fn lexed_tokens(text: &str, line_index: usize) -> Result<Vec<OwnedLexToken>, CardTextError> {
    lex_line(text, line_index)
}

fn normalized_token_words(tokens: &[OwnedLexToken]) -> Vec<String> {
    grammar::CompatWordIndex::new(tokens).owned_words()
}

fn is_bullet_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if str_starts_with_char(trimmed, '•') || str_starts_with_char(trimmed, '*') {
        return true;
    }
    if let Some(rest) = str_strip_prefix(trimmed, "-") {
        let next = rest.chars().next();
        if next.is_some_and(|ch| ch.is_ascii_digit()) {
            return false;
        }
        return true;
    }
    false
}

fn parse_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> Option<TriggerIntroCst> {
    if grammar::parse_prefix(tokens, grammar::phrase(&["when"])).is_some() {
        Some(TriggerIntroCst::When)
    } else if grammar::parse_prefix(tokens, grammar::phrase(&["whenever"])).is_some() {
        Some(TriggerIntroCst::Whenever)
    } else if super::parser_support::is_at_trigger_intro_lexed(tokens, 0) {
        Some(TriggerIntroCst::At)
    } else {
        None
    }
}

fn strip_trigger_frequency_suffix_tokens(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<u32>) {
    if let Some((_, rest)) = grammar::strip_lexed_suffix_phrases(
        tokens,
        &[
            &["for", "the", "first", "time", "each", "turn"][..],
            &["for", "the", "first", "time", "this", "turn"][..],
        ],
    ) {
        return (rest, Some(1));
    }

    (tokens, None)
}

fn strip_trailing_trigger_cap_suffix_tokens(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<u32>) {
    let cap_suffixes = [
        &[
            "this", "ability", "triggers", "only", "once", "each", "turn",
        ][..],
        &[
            "this", "ability", "triggers", "only", "twice", "each", "turn",
        ][..],
    ];
    let Some((phrase, head)) = grammar::strip_lexed_suffix_phrases(tokens, &cap_suffixes) else {
        return (tokens, None);
    };
    let count = if phrase
        == [
            "this", "ability", "triggers", "only", "once", "each", "turn",
        ] {
        1
    } else {
        2
    };
    if !head.last().is_some_and(|token| token.is_period()) {
        return (tokens, None);
    }
    (&head[..head.len() - 1], Some(count))
}

fn line_starts_with_trigger_intro_rewrite(text: &str, line_index: usize) -> bool {
    let Ok(tokens) = lexed_tokens(text, line_index) else {
        return false;
    };
    line_starts_with_trigger_intro_tokens(&tokens)
}

fn line_starts_with_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> bool {
    if super::parser_support::looks_like_reflexive_followup_intro_lexed(tokens) {
        return false;
    }
    parse_trigger_intro_tokens(tokens).is_some()
}

fn render_trimmed_lexed_sentence(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(tokens).trim().to_string()
}

fn render_trimmed_lexed_sentences(tokens: &[OwnedLexToken]) -> Vec<String> {
    split_lexed_sentences(tokens)
        .into_iter()
        .map(render_trimmed_lexed_sentence)
        .filter(|sentence| !sentence.is_empty())
        .collect()
}

fn nested_combat_whenever_clause_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, after_intro) = grammar::parse_prefix(
        tokens,
        grammar::phrase(&["at", "the", "beginning", "of", "each", "combat"]),
    )?;
    let after_unless = trim_lexed_commas(after_intro);
    let (_, _after_pay) =
        grammar::parse_prefix(after_unless, grammar::phrase(&["unless", "you", "pay"]))?;

    tokens.iter().enumerate().find_map(|(idx, token)| {
        (token.kind == TokenKind::Comma
            && tokens
                .get(idx + 1)
                .is_some_and(|next| next.is_word("whenever")))
        .then_some(&tokens[idx + 1..])
    })
}

fn is_activate_only_once_each_turn_tokens(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, rest)) = grammar::parse_prefix(
        tokens,
        grammar::phrase(&["activate", "only", "once", "each", "turn"]),
    ) else {
        return false;
    };
    grammar::parse_prefix(rest, grammar::end_of_sentence_or_block())
        .is_some_and(|(_, remainder)| remainder.is_empty())
}

fn is_doesnt_untap_during_your_untap_step_tokens(tokens: &[OwnedLexToken]) -> bool {
    let words = normalized_token_words(tokens);
    words.len() >= 5
        && words[words.len() - 5..]
            .iter()
            .zip(["untap", "during", "your", "untap", "step"])
            .all(|(actual, expected)| actual == expected)
        && words
            .iter()
            .any(|word| matches!(word.as_str(), "dont" | "doesnt"))
}

fn looks_like_untap_all_during_each_other_players_untap_step_tokens(
    tokens: &[OwnedLexToken],
) -> bool {
    super::grammar::structure::looks_like_untap_all_during_each_other_players_untap_step_line_lexed(
        tokens,
    )
}

fn looks_like_pact_next_upkeep_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    super::grammar::structure::looks_like_pact_next_upkeep_line_lexed(tokens)
}

fn looks_like_next_turn_cant_cast_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    super::grammar::structure::looks_like_next_turn_cant_cast_line_lexed(tokens)
}

fn looks_like_divvy_statement_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    super::grammar::structure::looks_like_divvy_statement_line_lexed(tokens)
}

fn looks_like_vote_statement_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    super::grammar::structure::looks_like_vote_statement_line_lexed(tokens)
}

fn looks_like_generic_statement_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    super::grammar::structure::looks_like_generic_statement_line_lexed(tokens)
}

fn looks_like_generic_static_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    super::grammar::structure::looks_like_generic_static_line_lexed(tokens)
}

fn parse_triggered_line_cst(line: &PreprocessedLine) -> Result<TriggeredLineCst, CardTextError> {
    let Some(first_token) = line.tokens.first() else {
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
    if let Some(err) = diagnose_known_unsupported_rewrite_line(normalized.as_str()) {
        return Err(err);
    }

    if let Some(nested_trigger_tokens) = nested_combat_whenever_clause_tokens(tokens_without_cap) {
        let nested_text = render_token_slice(nested_trigger_tokens);
        let nested_line = rewrite_line_normalized(line, nested_text.as_str())?;
        if let Ok(parsed) = parse_triggered_line_cst(&nested_line) {
            return Ok(parsed);
        }
    }

    let mut best_supported_split = None;
    let mut best_fallback_split = None;

    for (separator_idx, separator) in tokens_without_cap.iter().enumerate() {
        if separator.kind != TokenKind::Comma || separator_idx <= 1 {
            continue;
        }

        let trigger_candidate_tokens = trim_lexed_commas(&tokens_without_cap[1..separator_idx]);
        let effect_candidate_tokens = trim_lexed_commas(&tokens_without_cap[separator_idx + 1..]);
        if trigger_candidate_tokens.is_empty() || effect_candidate_tokens.is_empty() {
            continue;
        }

        let (trigger_tokens, max_triggers_per_turn) =
            strip_trigger_frequency_suffix_tokens(trigger_candidate_tokens);
        let max_triggers_per_turn = max_triggers_per_turn.or(trailing_cap);
        let trigger_text = render_token_slice(trigger_tokens).trim().to_string();
        let effect_candidate = render_token_slice(effect_candidate_tokens)
            .trim()
            .to_string();
        if trigger_text.is_empty() {
            continue;
        }

        let trigger_probe = parse_trigger_clause_lexed(trigger_tokens);
        let effect_probe = parse_effect_sentences_lexed(effect_candidate_tokens);
        let trigger_is_supported = trigger_probe.is_ok();
        if trigger_is_supported && effect_probe.is_ok() {
            if best_supported_split.is_none() {
                best_supported_split = Some(TriggeredLineCst {
                    info: line.info.clone(),
                    full_text: format!(
                        "{} {}, {}",
                        first_token.slice.as_str(),
                        trigger_text,
                        effect_candidate
                    ),
                    trigger_text,
                    effect_text: effect_candidate.to_string(),
                    max_triggers_per_turn,
                    chosen_option_label: None,
                });
            }
            continue;
        }

        let full_text = format!(
            "{} {}, {}",
            first_token.slice.as_str(),
            trigger_text,
            effect_candidate.as_str()
        );
        if parse_triggered_line_lexed(tokens_without_cap).is_ok() && best_fallback_split.is_none() {
            best_fallback_split = Some(TriggeredLineCst {
                info: line.info.clone(),
                full_text,
                trigger_text,
                effect_text: effect_candidate,
                max_triggers_per_turn,
                chosen_option_label: None,
            });
        }
    }

    if let Some(split) = best_supported_split.or(best_fallback_split) {
        return Ok(split);
    }

    match parse_triggered_line_lexed(tokens_without_cap) {
        Ok(_) => Ok(TriggeredLineCst {
            info: line.info.clone(),
            full_text: normalized.to_string(),
            trigger_text: render_token_slice(condition_tokens).trim().to_string(),
            effect_text: String::new(),
            max_triggers_per_turn: trailing_cap,
            chosen_option_label: None,
        }),
        Err(err) => Err(err),
    }
}

fn parse_static_line_cst(line: &PreprocessedLine) -> Result<Option<StaticLineCst>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();
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
        return Ok(Some(StaticLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            chosen_option_label: None,
        }));
    }

    let rewritten_parse_text = rewrite_keyword_dash_parse_text(normalized);
    let lexed = lexed_tokens(rewritten_parse_text.as_str(), line.info.line_index)?;
    let mut deferred_error = None;

    if grammar::parse_prefix(&lexed, grammar::phrase(&["level", "up"])).is_some() {
        if parse_level_up_line_lexed(&lexed)?.is_some() {
            return Ok(Some(StaticLineCst {
                info: line.info.clone(),
                text: normalized.to_string(),
                chosen_option_label: None,
            }));
        }
    }
    if is_doesnt_untap_during_your_untap_step_tokens(&lexed) {
        return Ok(Some(StaticLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            chosen_option_label: None,
        }));
    }
    if looks_like_untap_all_during_each_other_players_untap_step_tokens(&lexed) {
        return Ok(Some(StaticLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            chosen_option_label: None,
        }));
    }

    if parse_if_this_spell_costs_less_to_cast_line_lexed(&lexed)?.is_some() {
        return Ok(Some(StaticLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            chosen_option_label: None,
        }));
    }

    if is_activate_only_once_each_turn_tokens(&lexed) {
        return Ok(Some(StaticLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            chosen_option_label: None,
        }));
    }

    if split_compound_buff_and_unblockable_sentence(normalized).is_some() {
        return Ok(Some(StaticLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            chosen_option_label: None,
        }));
    }

    if !should_skip_keyword_action_static_probe(normalized)
        && let Some(_actions) = parse_ability_line_lexed(&lexed)
    {
        return Ok(Some(StaticLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            chosen_option_label: None,
        }));
    }

    match parse_static_ability_ast_line_lexed(&lexed) {
        Ok(Some(_abilities)) => {
            return Ok(Some(StaticLineCst {
                info: line.info.clone(),
                text: normalized.to_string(),
                chosen_option_label: None,
            }));
        }
        Ok(None) => {}
        Err(err) => deferred_error = Some(err),
    }

    if parse_split_static_item_count(&lexed)?.is_some() {
        return Ok(Some(StaticLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            chosen_option_label: None,
        }));
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

pub(crate) fn split_compound_buff_and_unblockable_sentence(text: &str) -> Option<(String, String)> {
    let trimmed = text.trim();
    let (subject, buff_tail) = str_split_once(trimmed, " gets ")?;
    if subject.trim().is_empty() || !str_contains(buff_tail, " and can't be blocked") {
        return None;
    }
    let (buff_clause, _) = str_split_once(buff_tail, " and can't be blocked")?;
    let left = format!("{} gets {}.", subject.trim(), buff_clause.trim());
    let right = format!("{} can't be blocked.", subject.trim());
    Some((left, right))
}

fn parse_keyword_line_cst(
    line: &PreprocessedLine,
) -> Result<Option<KeywordLineCst>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();
    let rewritten_parse_text = rewrite_keyword_dash_parse_text(normalized);
    let tokens = lexed_tokens(rewritten_parse_text.as_str(), line.info.line_index)?;

    let kind = if parse_additional_cost_kind(&tokens, normalized)? {
        Some(KeywordLineKindCst::AdditionalCostChoice)
    } else if str_starts_with(normalized, "as an additional cost to cast this spell")
        && additional_cost_tail_tokens(&tokens).is_some()
    {
        Some(KeywordLineKindCst::AdditionalCost)
    } else if parse_alternative_cast_kind(&tokens, normalized)? {
        Some(KeywordLineKindCst::AlternativeCast)
    } else if parse_bestow_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Bestow)
    } else if parse_bargain_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Bargain)
    } else if parse_buyback_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Buyback)
    } else if parse_channel_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Channel)
    } else if parse_cycling_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Cycling)
    } else if parse_reinforce_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Reinforce)
    } else if parse_equip_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Equip)
    } else if parse_kicker_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Kicker)
    } else if parse_flashback_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Flashback)
    } else if parse_harmonize_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Harmonize)
    } else if parse_multikicker_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Multikicker)
    } else if parse_entwine_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Entwine)
    } else if parse_offspring_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Offspring)
    } else if parse_madness_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Madness)
    } else if parse_escape_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Escape)
    } else if str_starts_with(normalized, "morph—") || str_starts_with(normalized, "megamorph—")
    {
        None
    } else if parse_morph_keyword_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Morph)
    } else if parse_squad_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Squad)
    } else if parse_transmute_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Transmute)
    } else if parse_cast_this_spell_only_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::CastThisSpellOnly)
    } else if is_standard_gift_keyword_line(line.info.raw_line.as_str()) {
        Some(KeywordLineKindCst::Gift)
    } else if parse_warp_line_lexed(&tokens)?.is_some() {
        Some(KeywordLineKindCst::Warp)
    } else if is_exert_attack_keyword_line(normalized) {
        Some(KeywordLineKindCst::ExertAttack)
    } else {
        None
    };

    Ok(kind.map(|kind| KeywordLineCst {
        info: line.info.clone(),
        text: normalized.to_string(),
        kind,
    }))
}

fn is_exert_attack_keyword_line(text: &str) -> bool {
    let trimmed = strip_exert_reminder_suffix(text);
    str_starts_with(trimmed, "you may exert ")
        || str_starts_with(
            trimmed,
            "if this creature hasn't been exerted this turn, you may exert ",
        )
}

fn strip_exert_reminder_suffix(text: &str) -> &str {
    let trimmed = text.trim();
    for suffix in [
        " (an exerted creature won't untap during your next untap step.)",
        " (an exerted permanent won't untap during your next untap step.)",
        " (it won't untap during your next untap step.)",
    ] {
        if let Some(stripped) = str_strip_suffix(trimmed, suffix) {
            return stripped.trim_end();
        }
    }
    trimmed
}

fn is_standard_gift_keyword_line(text: &str) -> bool {
    let trimmed = text.trim().to_ascii_lowercase();
    if !str_starts_with(trimmed.as_str(), "gift ") {
        return false;
    }
    if !str_contains(
        trimmed.as_str(),
        "you may promise an opponent a gift as you cast this spell",
    ) || !str_contains(trimmed.as_str(), "if you do")
    {
        return false;
    }

    let head = str_split_once_char(trimmed.as_str(), '(')
        .map(|(head, _)| head.trim())
        .unwrap_or(trimmed.as_str());
    matches!(
        head,
        "gift a card"
            | "gift a treasure"
            | "gift a food"
            | "gift a tapped fish"
            | "gift an extra turn"
            | "gift an octopus"
    )
}

fn additional_cost_tail_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let comma_idx = tokens
        .iter()
        .enumerate()
        .find_map(|(idx, token)| (token.kind == TokenKind::Comma).then_some(idx));
    let effect_start = if let Some(idx) = comma_idx {
        idx + 1
    } else if let Some(idx) = find_token_index(tokens, |token| token.is_word("spell")) {
        idx + 1
    } else {
        tokens.len()
    };
    let effect_tokens = tokens.get(effect_start..).unwrap_or_default();
    (!effect_tokens.is_empty()).then_some(effect_tokens)
}

fn parse_additional_cost_kind(
    tokens: &[OwnedLexToken],
    normalized: &str,
) -> Result<bool, CardTextError> {
    if !str_starts_with(normalized, "as an additional cost to cast this spell") {
        return Ok(false);
    }
    let Some(effect_tokens) = additional_cost_tail_tokens(tokens) else {
        return Ok(false);
    };
    Ok(parse_additional_cost_choice_options_lexed(effect_tokens)?.is_some())
}

fn parse_alternative_cast_kind(
    tokens: &[OwnedLexToken],
    normalized: &str,
) -> Result<bool, CardTextError> {
    Ok(
        parse_self_free_cast_alternative_cost_line_lexed(tokens).is_some()
            || parse_you_may_rather_than_spell_cost_line_lexed(tokens, normalized)?.is_some()
            || parse_if_conditional_alternative_cost_line_lexed(tokens, normalized)?.is_some(),
    )
}

fn parse_level_item_cst(line: &PreprocessedLine) -> Result<Option<LevelItemCst>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();
    let lexed = lexed_tokens(normalized, line.info.line_index)?;

    if !should_skip_keyword_action_static_probe(normalized)
        && let Some(_actions) = parse_ability_line_lexed(&lexed)
    {
        return Ok(Some(LevelItemCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            kind: LevelItemKindCst::KeywordActions,
        }));
    }

    if let Some(_abilities) = parse_static_ability_ast_line_lexed(&lexed)? {
        return Ok(Some(LevelItemCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            kind: LevelItemKindCst::StaticAbilities,
        }));
    }

    Ok(None)
}

fn parse_statement_line_cst(
    line: &PreprocessedLine,
) -> Result<Option<StatementLineCst>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();
    if looks_like_divvy_statement_line_tokens(&line.tokens) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
        }));
    }
    if str_contains(
        normalized,
        "ask a person outside the game to rate its new art on a scale from 1 to 5",
    ) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
        }));
    }
    if looks_like_pact_next_upkeep_line_tokens(&line.tokens) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
        }));
    }
    if normalized
        == "exile target nonland permanent. for as long as that card remains exiled, its owner may play it. a spell cast by an opponent this way costs {2} more to cast."
    {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
        }));
    }
    if looks_like_untap_all_during_each_other_players_untap_step_tokens(&line.tokens) {
        return Ok(None);
    }
    let parse_text = rewrite_statement_parse_text_lexed(&line.tokens, normalized);
    let lexed = lexed_tokens(parse_text.as_str(), line.info.line_index)?;
    let effects = match parse_effect_sentences_lexed(&lexed) {
        Ok(effects) => effects,
        Err(err)
            if looks_like_statement_line(normalized)
                || str_starts_with(normalized, "choose ")
                || str_starts_with(normalized, "if ")
                || str_starts_with(normalized, "reveal ") =>
        {
            return Err(err);
        }
        Err(_) => return Ok(None),
    };
    if effects.is_empty() {
        return Ok(None);
    }

    Ok(Some(StatementLineCst {
        info: line.info.clone(),
        text: normalized.to_string(),
    }))
}

fn parse_former_section9_unsupported_line_cst(
    line: &PreprocessedLine,
) -> Option<UnsupportedLineCst> {
    let normalized = line.info.normalized.normalized.as_str();
    let canonical = normalized.replace(['\'', '’'], "");
    let canonical_without_hyphens = canonical.replace('-', "");
    let matches_former_section9_gap = canonical
        == "destroy target creature if its white. a creature destroyed this way cant be regenerated."
        || canonical
            == "create two 1/1 white kithkin soldier creature tokens if {w} was spent to cast this spell. counter up to one target creature spell if {u} was spent to cast this spell."
        || canonical == "{t}, sacrifice x goats: add x mana of any one color. you gain x life."
        || canonical
            == "shuffle your library, then exile the top four cards. you may cast any number of spells with mana value 5 or less from among them without paying their mana costs. lands you control dont untap during your next untap step."
        || canonical_without_hyphens
            == "exile target nontoken creature you own and the top two cards of your library in a facedown pile, shuffle that pile, then cloak those cards. they enter tapped."
        || canonical
            == "destroy target creature unless its controller pays life equal to its toughness. a creature destroyed this way cant be regenerated."
        || canonical
            == "destroy all lands or all creatures. creatures destroyed this way cant be regenerated."
        || canonical
            == "destroy two target nonblack creatures unless either one is a color the other isnt. they cant be regenerated.";
    matches_former_section9_gap.then(|| UnsupportedLineCst {
        info: line.info.clone(),
        reason_code: "former-section9-line-not-yet-supported",
    })
}

fn looks_like_statement_line(normalized: &str) -> bool {
    if let Some((_, body)) = split_label_prefix(normalized) {
        return looks_like_statement_line(body);
    }

    if let Ok(tokens) = lex_line(normalized, 0) {
        if looks_like_untap_all_during_each_other_players_untap_step_tokens(&tokens) {
            return false;
        }
        if looks_like_next_turn_cant_cast_line_tokens(&tokens)
            || looks_like_vote_statement_line_tokens(&tokens)
            || looks_like_generic_statement_line_tokens(&tokens)
        {
            return true;
        }
    }
    false
}

fn should_skip_keyword_action_static_probe(normalized: &str) -> bool {
    let normalized = normalized.trim();
    (str_ends_with(normalized, "can't be blocked.")
        || str_ends_with(normalized, "can't be blocked"))
        && !str_starts_with(normalized, "this ")
        && !str_starts_with(normalized, "it ")
}

fn rewrite_statement_parse_text_lexed(tokens: &[OwnedLexToken], fallback_text: &str) -> String {
    let sentences = render_trimmed_lexed_sentences(tokens)
        .into_iter()
        .map(|sentence| {
            strip_non_keyword_label_prefix(sentence.as_str())
                .trim()
                .to_string()
        })
        .map(|sentence| rewrite_statement_followup_intro(sentence.as_str()))
        .map(rewrite_copy_exception_type_removal)
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentences.is_empty() {
        fallback_text.trim().to_string()
    } else {
        format!("{}.", sentences.join(". "))
    }
}

fn rewrite_copy_exception_type_removal(sentence: String) -> String {
    sentence
        .replace(
            "except it's an artifact and it loses all other card types",
            "except it's an artifact",
        )
        .replace(
            "except its an artifact and it loses all other card types",
            "except its an artifact",
        )
        .replace(
            "except it's an enchantment and it loses all other card types",
            "except it's an enchantment",
        )
        .replace(
            "except its an enchantment and it loses all other card types",
            "except its an enchantment",
        )
        .replace(
            "except it's an enchantment and loses all other card types",
            "except it's an enchantment",
        )
        .replace(
            "except its an enchantment and loses all other card types",
            "except its an enchantment",
        )
}

fn rewrite_statement_followup_intro(sentence: &str) -> String {
    let trimmed = sentence.trim();
    if let Some(rest) = str_strip_prefix(trimmed, "when you do,") {
        return format!("if you do, {}", rest.trim_start());
    }
    if let Some(rest) = str_strip_prefix(trimmed, "whenever you do,") {
        return format!("if you do, {}", rest.trim_start());
    }
    trimmed.to_string()
}

fn looks_like_activation_cost_prefix(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    if str_starts_with_char(trimmed, '{')
        || str_starts_with(trimmed, "+")
        || str_starts_with(trimmed, "-")
        || str_starts_with_char(trimmed, '−')
    {
        return true;
    }
    let first = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '\'')
        .to_ascii_lowercase();
    matches!(
        first.as_str(),
        "tap"
            | "untap"
            | "pay"
            | "discard"
            | "sacrifice"
            | "exile"
            | "mill"
            | "remove"
            | "put"
            | "return"
    )
}

fn looks_like_static_line(normalized: &str) -> bool {
    lex_line(normalized, 0).ok().is_some_and(|tokens| {
        looks_like_untap_all_during_each_other_players_untap_step_tokens(&tokens)
            || looks_like_generic_static_line_tokens(&tokens)
    })
}

fn rewrite_keyword_dash_parse_text(text: &str) -> String {
    let trimmed = text.trim();
    if let Some((label, body)) = split_label_prefix(trimmed) {
        let normalized_label = label.to_ascii_lowercase();
        if matches!(
            normalized_label.as_str(),
            "will of the council" | "council's dilemma" | "councils dilemma" | "secret council"
        ) {
            return body.trim().to_string();
        }
    }
    if let Some((label, body)) = split_label_prefix(trimmed)
        && preserve_keyword_prefix_for_parse(label)
    {
        return format!("{label} {}", body.trim());
    }
    trimmed.to_string()
}

fn split_once_outside_quotes(text: &str, needle: char) -> Option<(&str, &str)> {
    let mut in_quotes = false;
    for (idx, ch) in text.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if ch == needle && !in_quotes {
            let needle_len = ch.len_utf8();
            return Some((&text[..idx], &text[idx + needle_len..]));
        }
    }
    None
}

fn split_label_prefix(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim();
    let (label, body) = str_split_once_char(trimmed, '—')?;
    let label = label.trim();
    let body = body.trim();
    (!label.is_empty() && !body.is_empty() && !str_contains(label, ".")).then_some((label, body))
}

fn is_nonkeyword_choice_labeled_line(line: &PreprocessedLine) -> bool {
    let normalized = line.info.normalized.normalized.as_str();
    split_label_prefix(normalized)
        .is_some_and(|(label, _)| !preserve_keyword_prefix_for_parse(label))
}

fn labeled_choice_block_has_peer(items: &[PreprocessedItem], idx: usize) -> bool {
    let mut probe = idx;
    while probe > 0 {
        probe -= 1;
        match items.get(probe) {
            Some(PreprocessedItem::Line(line)) if is_nonkeyword_choice_labeled_line(line) => {
                return true;
            }
            Some(PreprocessedItem::Line(_)) => break,
            Some(PreprocessedItem::Metadata(_)) => continue,
            None => break,
        }
    }

    let mut probe = idx + 1;
    while let Some(item) = items.get(probe) {
        match item {
            PreprocessedItem::Line(line) if is_nonkeyword_choice_labeled_line(line) => {
                return true;
            }
            PreprocessedItem::Line(_) => break,
            PreprocessedItem::Metadata(_) => {
                probe += 1;
                continue;
            }
        }
    }

    false
}

fn split_trailing_keyword_activation_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(String, String)> {
    let sentences = render_trimmed_lexed_sentences(tokens);
    if sentences.len() <= 1 {
        return None;
    }

    for split_idx in 1..sentences.len() {
        let prefix = sentences[..split_idx].join(". ");
        let suffix = sentences[split_idx..].join(". ");
        let Some((label, body)) = split_label_prefix(suffix.as_str()) else {
            continue;
        };
        if !preserve_keyword_prefix_for_parse(label)
            || split_once_outside_quotes(body, ':').is_none()
        {
            continue;
        }
        return Some((prefix, suffix));
    }

    None
}

fn preflight_known_strict_unsupported(text: &str) -> Option<CardTextError> {
    let normalized = text.to_ascii_lowercase();
    if str_contains(
        normalized.as_str(),
        "if your life total is less than or equal to half your starting life total plus one",
    ) {
        return Some(CardTextError::ParseError(
            "unsupported predicate".to_string(),
        ));
    }
    None
}

fn rewrite_named_source_sentence_for_builder(
    builder: &CardDefinitionBuilder,
    text: &str,
) -> Option<String> {
    let trimmed = text.trim();
    let subject = if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Creature)
    {
        "this creature"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Land)
    {
        "this land"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Artifact)
    {
        "this artifact"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Enchantment)
    {
        "this enchantment"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Planeswalker)
    {
        "this planeswalker"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Battle)
    {
        "this battle"
    } else {
        "this permanent"
    };
    let lower = trimmed.to_ascii_lowercase();

    let name = builder.card_builder.name_ref();
    if !name.is_empty() {
        let name_lower = name.to_ascii_lowercase();
        if let Some(remainder) = str_strip_prefix(lower.as_str(), &(name_lower + " ")) {
            return Some(format!("{subject} {remainder}"));
        }
    }

    let (_, rest) = str_split_once(lower.as_str(), " enters ")?;
    Some(format!("{subject} enters {rest}"))
}

fn rewrite_named_source_trigger_for_builder(
    builder: &CardDefinitionBuilder,
    text: &str,
) -> Option<String> {
    let trimmed = text.trim();
    let subject = if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Creature)
    {
        "this creature"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Land)
    {
        "this land"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Artifact)
    {
        "this artifact"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Enchantment)
    {
        "this enchantment"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Planeswalker)
    {
        "this planeswalker"
    } else if builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| *card_type == crate::types::CardType::Battle)
    {
        "this battle"
    } else {
        "this permanent"
    };

    let name = builder.card_builder.name_ref();
    if name.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    let name_lower = name.to_ascii_lowercase();
    for intro in ["whenever", "when", "at"] {
        let prefix = format!("{intro} {name_lower} ");
        if let Some(rest) = str_strip_prefix(lower.as_str(), prefix.as_str()) {
            return Some(format!("{intro} {subject} {rest}"));
        }
    }

    None
}

fn strip_non_keyword_label_prefix(text: &str) -> &str {
    let mut current = text.trim();
    while let Some((label, body)) = split_label_prefix(current) {
        if preserve_keyword_prefix_for_parse(label) {
            break;
        }
        current = body.trim();
    }
    current
}

#[cfg(test)]
mod tests {
    use super::{
        is_doesnt_untap_during_your_untap_step_tokens, lex_line,
        looks_like_divvy_statement_line_tokens, looks_like_generic_statement_line_tokens,
        looks_like_generic_static_line_tokens, looks_like_next_turn_cant_cast_line_tokens,
        looks_like_pact_next_upkeep_line_tokens, looks_like_statement_line, looks_like_static_line,
        looks_like_untap_all_during_each_other_players_untap_step_tokens,
        looks_like_vote_statement_line_tokens, split_label_prefix, strip_non_keyword_label_prefix,
    };

    #[test]
    fn strip_non_keyword_label_prefix_removes_chained_mode_name_and_cost() {
        assert_eq!(
            strip_non_keyword_label_prefix(
                "Meteor Strikes — {2} — Double target creature's power and toughness until end of turn."
            ),
            "Double target creature's power and toughness until end of turn."
        );
        assert_eq!(
            strip_non_keyword_label_prefix(
                "Final Heaven — {6}{G} — Triple target creature's power and toughness until end of turn."
            ),
            "Triple target creature's power and toughness until end of turn."
        );
    }

    #[test]
    fn looks_like_statement_line_recognizes_vote_leads() {
        for text in [
            "Starting with you, each player votes for death or torture. If death gets more votes, each opponent sacrifices a creature of their choice. If torture gets more votes or the vote is tied, each opponent loses 4 life.",
            "Secret council — Each player secretly votes for truth or consequences, then those votes are revealed. For each truth vote, draw a card. Then choose an opponent at random. For each consequences vote, Truth or Consequences deals 3 damage to that player.",
        ] {
            let helper_text = split_label_prefix(text)
                .map(|(_, body)| body.trim())
                .unwrap_or(text);
            let tokens =
                lex_line(helper_text, 0).expect("rewrite lexer should classify vote line body");
            assert!(looks_like_vote_statement_line_tokens(&tokens));
            assert!(
                looks_like_statement_line(text.to_ascii_lowercase().as_str()),
                "expected vote line to classify as a statement: {text}"
            );
        }
    }

    #[test]
    fn looks_like_statement_line_recognizes_next_turn_cast_lock() {
        let text =
            "Each opponent can't cast instant or sorcery spells during that player's next turn.";
        let tokens =
            lex_line(text, 0).expect("rewrite lexer should classify next-turn cast-lock line");
        assert!(looks_like_next_turn_cant_cast_line_tokens(&tokens));
        assert!(looks_like_statement_line(
            text.to_ascii_lowercase().as_str()
        ));
    }

    #[test]
    fn looks_like_statement_line_recognizes_generic_heads() {
        for text in [
            "Draw a card.",
            "Each player discards a card.",
            "That target player sacrifices a creature.",
            "This spell deals 3 damage to any target.",
            "Target creature gets +2/+2 until end of turn.",
        ] {
            let tokens =
                lex_line(text, 0).expect("rewrite lexer should classify generic statement head");
            assert!(looks_like_generic_statement_line_tokens(&tokens));
            assert!(looks_like_statement_line(
                text.to_ascii_lowercase().as_str()
            ));
        }
    }

    #[test]
    fn looks_like_static_line_recognizes_generic_heads() {
        for text in [
            "This creature has flying.",
            "Enchanted creature gets +1/+1.",
            "As long as you control an artifact, this creature has hexproof.",
            "Your maximum hand size is reduced by four.",
        ] {
            let tokens =
                lex_line(text, 0).expect("rewrite lexer should classify generic static head");
            assert!(looks_like_generic_static_line_tokens(&tokens));
            assert!(looks_like_static_line(text.to_ascii_lowercase().as_str()));
        }
    }

    #[test]
    fn looks_like_divvy_statement_probe_recognizes_pile_lines() {
        let text = "Separate all creatures target player controls into two piles. Destroy all creatures in the pile of your choice.";
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify divvy pile line");
        assert!(looks_like_divvy_statement_line_tokens(&tokens));
    }

    #[test]
    fn untap_shape_probes_recognize_expected_token_patterns() {
        let your_step = lex_line("Lands you control don't untap during your untap step.", 0)
            .expect("rewrite lexer should classify your-untap-step probe");
        assert!(is_doesnt_untap_during_your_untap_step_tokens(&your_step));

        let other_players_text =
            "Untap all permanents you control during each other player's untap step.";
        let other_players = lex_line(other_players_text, 0)
            .expect("rewrite lexer should classify other-players untap-step probe");
        assert!(looks_like_untap_all_during_each_other_players_untap_step_tokens(&other_players));
        assert!(!looks_like_statement_line(
            other_players_text.to_ascii_lowercase().as_str()
        ));
        assert!(looks_like_static_line(
            other_players_text.to_ascii_lowercase().as_str()
        ));
    }

    #[test]
    fn pact_shape_probe_recognizes_next_upkeep_lose_game_line() {
        let tokens = lex_line(
            "At the beginning of your next upkeep, pay {2}{U}{U}. If you don't, you lose the game.",
            0,
        )
        .expect("rewrite lexer should classify pact next-upkeep statement line");
        assert!(looks_like_pact_next_upkeep_line_tokens(&tokens));
    }
}

fn is_named_ability_label(label: &str) -> bool {
    matches!(
        label.to_ascii_lowercase().as_str(),
        "alliance"
            | "astral projection"
            | "bigby's hand"
            | "body-print"
            | "boast"
            | "cohort"
            | "devouring monster"
            | "diana"
            | "exhaust"
            | "gooooaaaalll!"
            | "hero's sundering"
            | "hunt for heresy"
            | "machina"
            | "mage hand"
            | "megamorph"
            | "morph"
            | "psychic blades"
            | "raid"
            | "renew"
            | "rope dart"
            | "scorching ray"
            | "share"
            | "shieldwall"
            | "sleight of hand"
            | "smear campaign"
            | "stunning strike"
            | "teleport"
            | "trance"
            | "valiant"
            | "waterbend"
    )
}

fn rewrite_line_normalized(
    line: &PreprocessedLine,
    normalized: &str,
) -> Result<PreprocessedLine, CardTextError> {
    let mut rewritten = line.clone();
    rewritten.info.normalized.original = normalized.to_string();
    rewritten.info.normalized.normalized = normalized.to_string();
    rewritten.info.normalized.char_map = (0..normalized.len()).collect();
    rewritten.tokens = super::lexer::lex_line(normalized, line.info.line_index)?;
    Ok(rewritten)
}

fn try_parse_triggered_line_with_named_source_rewrite(
    builder: &CardDefinitionBuilder,
    line: &PreprocessedLine,
    text: &str,
) -> Result<Option<TriggeredLineCst>, CardTextError> {
    let Some(rewritten) = rewrite_named_source_trigger_for_builder(builder, text) else {
        return Ok(None);
    };
    let rewritten_line = rewrite_line_normalized(line, rewritten.as_str())?;
    match parse_triggered_line_cst(&rewritten_line) {
        Ok(triggered) => Ok(Some(triggered)),
        Err(_) => Ok(None),
    }
}

fn is_fully_parenthetical_line(text: &str) -> bool {
    let trimmed = text.trim();
    str_starts_with_char(trimmed, '(') && str_ends_with_char(trimmed, ')')
}

fn split_trigger_sentence_chunks_rewrite_lexed(tokens: &[OwnedLexToken]) -> Vec<String> {
    let sentence_tokens = split_lexed_sentences(tokens);
    let sentences = sentence_tokens
        .iter()
        .map(|sentence| render_trimmed_lexed_sentence(sentence))
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentences.len() <= 1 {
        return sentences;
    }

    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_starts_with_trigger = false;

    for (sentence, sentence_tokens) in sentences.into_iter().zip(sentence_tokens.into_iter()) {
        let sentence_starts_with_trigger = line_starts_with_trigger_intro_tokens(sentence_tokens);
        if !current.is_empty() && current_starts_with_trigger && sentence_starts_with_trigger {
            chunks.push(current.join(". "));
            current.clear();
            current_starts_with_trigger = false;
        }
        if current.is_empty() {
            current_starts_with_trigger = sentence_starts_with_trigger;
        }
        current.push(sentence);
    }

    if !current.is_empty() {
        chunks.push(current.join(". "));
    }

    chunks
}

fn split_reveal_first_draw_line_rewrite_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<String>> {
    let sentences = render_trimmed_lexed_sentences(tokens);
    if sentences.len() <= 1 {
        return None;
    }

    let first = sentences.first()?.to_ascii_lowercase();
    let first_is_reveal_first_draw = matches!(
        first.as_str(),
        "reveal the first card you draw each turn"
            | "reveal the first card you draw on each of your turns"
            | "you may reveal the first card you draw each turn as you draw it"
            | "you may reveal the first card you draw on each of your turns as you draw it"
    );
    if !first_is_reveal_first_draw {
        return None;
    }

    let tail = sentences[1..].join(". ");
    let tail_lower = tail.to_ascii_lowercase();
    if !str_starts_with(tail_lower.as_str(), "whenever you reveal ") {
        return None;
    }

    Some(vec![sentences[0].clone(), tail])
}

fn classify_unsupported_line_reason(line: &PreprocessedLine) -> &'static str {
    let normalized = line.info.normalized.normalized.as_str();

    if is_bullet_line(line.info.raw_line.as_str()) {
        return "bullet-line-without-modal-header";
    }
    if line_starts_with_trigger_intro_rewrite(normalized, line.info.line_index) {
        return "triggered-line-not-yet-supported";
    }
    if split_once_outside_quotes(normalized, ':').is_some() {
        return "activated-line-not-yet-supported";
    }
    if str_starts_with(normalized, "choose ") {
        return "modal-header-not-yet-supported";
    }
    if looks_like_statement_line(normalized) {
        return "statement-line-not-yet-supported";
    }
    if looks_like_static_line(normalized) {
        return "static-line-not-yet-supported";
    }
    "unclassified-line-family"
}

fn diagnose_known_unsupported_rewrite_line(normalized: &str) -> Option<CardTextError> {
    let spent_to_cast_count = normalized.match_indices("spent to cast this spell").count();
    let message = if str_starts_with(normalized, "choose target land")
        && str_contains(normalized, "create three tokens that are copies of it")
    {
        "unsupported choose-leading spell clause"
    } else if str_contains(normalized, "same name as another card in their hand") {
        "unsupported same-name-as-another-in-hand discard clause"
    } else if str_starts_with(normalized, "partner with ") {
        "unsupported partner-with keyword line [rule=partner-with-keyword-line]"
    } else if str_starts_with(
        normalized,
        "the first creature spell you cast each turn costs ",
    ) {
        "unsupported first-spell cost modifier mechanic"
    } else if str_contains(normalized, "loses all abilities and becomes") {
        if str_starts_with(normalized, "until end of turn,") {
            "unsupported loses-all-abilities with becomes clause"
        } else {
            "unsupported lose-all-abilities static becomes clause"
        }
    } else if str_contains(
        normalized,
        "enters tapped and doesn't untap during your untap step",
    ) {
        "unsupported mixed enters-tapped and negated-untap clause"
    } else if str_starts_with(normalized, "once each turn, you may play a card from exile") {
        "unsupported static clause"
    } else if str_contains(
        normalized,
        "prevent all combat damage that would be dealt this turn by creatures with power",
    ) {
        "unsupported prevent-all-combat-damage clause tail"
    } else if str_starts_with(
        normalized,
        "prevent the next 1 damage that would be dealt to any target this turn by red sources",
    ) {
        "unsupported trailing prevent-next damage clause"
    } else if str_contains(
        normalized,
        "put one of them into your hand and the rest into your graveyard",
    ) {
        "unsupported multi-destination put clause"
    } else if str_contains(
        normalized,
        "assigns no combat damage this turn and defending player loses",
    ) {
        "unsupported assigns-no-combat-damage clause"
    } else if str_contains(normalized, "of defending player's choice") {
        "unsupported defending-players-choice clause"
    } else if str_starts_with(normalized, "ninjutsu abilities you activate cost ") {
        "unsupported marker keyword with non-keyword tail"
    } else if spent_to_cast_count >= 2
        && str_contains(normalized, "spent to cast this spell")
        && str_contains(normalized, " if ")
        && !str_starts_with(normalized, "if ")
        && !str_starts_with(normalized, "unless ")
        && !str_starts_with(normalized, "when ")
        && !str_starts_with(normalized, "as ")
    {
        "unsupported spent-to-cast conditional clause"
    } else if str_contains(normalized, "if you sacrifice an island this way") {
        "unsupported if-you-sacrifice-an-island-this-way clause"
    } else if str_contains(
        normalized,
        "create a token that's a copy of that aura attached to that creature",
    ) {
        "unsupported aura-copy attachment fanout clause"
    } else if str_contains(normalized, "target face-down creature") {
        "unsupported face-down clause"
    } else if normalized == "creatures you control have haste and attack each combat if able." {
        "unsupported anthem subject"
    } else if str_contains(
        normalized,
        "with islandwalk can be blocked as though they didn't have islandwalk",
    ) {
        "unsupported landwalk override clause"
    } else if normalized == "you may play any number of lands on each of your turns." {
        "unsupported additional-land-play permission clause"
    } else if normalized == "target creature can block any number of creatures this turn." {
        "unsupported target-only restriction clause"
    } else if normalized == "equip costs you pay cost {1} less." {
        "unsupported activation cost modifier clause"
    } else if normalized == "unleash while" {
        "unsupported line"
    } else if str_contains(normalized, "for each odd result")
        && str_contains(normalized, "for each even result")
    {
        "unsupported odd-or-even die-result clause"
    } else if str_contains(
        normalized,
        "for as long as that card remains exiled, its owner may play it",
    ) && !str_contains(normalized, "a spell cast by an opponent this way costs")
        && !str_contains(normalized, "a spell cast this way costs")
    {
        "unsupported for-as-long-as play/cast permission clause"
    } else if str_contains(
        normalized,
        "with power or toughness 1 or less can't be blocked",
    ) {
        "unsupported power-or-toughness cant-be-blocked subject"
    } else if str_contains(
        normalized,
        "discard up to two permanents, then draw that many cards",
    ) {
        "unsupported discard qualifier clause"
    } else if str_contains(
        normalized,
        "if your life total is less than or equal to half your starting life total plus one",
    ) {
        "unsupported predicate"
    } else if str_contains(
        normalized,
        "then sacrifices all creatures they control, then puts all cards they exiled this way onto the battlefield",
    ) {
        "unsupported each-player exile/sacrifice/return-this-way clause"
    } else if str_contains(
        normalized,
        "each player loses x life, discards x cards, sacrifices x creatures",
    ) && str_contains(normalized, "then sacrifices x lands")
    {
        "unsupported multi-step each-player clause with 'then'"
    } else if str_contains(normalized, "if this creature isn't saddled this turn") {
        "unsupported saddled conditional tail"
    } else if str_contains(
        normalized,
        "put a card from among them into your hand this turn",
    ) {
        "unsupported looked-card fallback tail"
    } else if str_contains(
        normalized,
        "if the sacrificed creature was a hamster this turn",
    ) {
        "unsupported predicate"
    } else {
        return None;
    };

    Some(CardTextError::ParseError(message.to_string()))
}

fn parse_colon_nonactivation_statement_fallback(
    line: &PreprocessedLine,
    text: &str,
) -> Result<Option<StatementLineCst>, CardTextError> {
    let Some((left, right)) = split_once_outside_quotes(text, ':') else {
        return Ok(None);
    };

    let trimmed_left = left.trim();
    let trimmed_right = right.trim();

    if trimmed_left.eq_ignore_ascii_case("reveal this card from your hand") {
        let left_line = rewrite_line_normalized(line, trimmed_left)?;
        if let Some(statement) = parse_statement_line_cst(&left_line)? {
            return Ok(Some(statement));
        }
    }

    if !str_contains(trimmed_left, "{") && str_contains(trimmed_left, ",") {
        let right_line = rewrite_line_normalized(line, trimmed_right)?;
        if let Some(statement) = parse_statement_line_cst(&right_line)? {
            return Ok(Some(statement));
        }
    }

    Ok(None)
}

fn split_activation_text_parts(
    text: &str,
    line_index: usize,
) -> Result<Option<(Vec<OwnedLexToken>, String)>, CardTextError> {
    let tokens = lexed_tokens(text, line_index)?;
    let mut quote_depth = 0u32;

    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            quote_depth = if quote_depth == 0 { 1 } else { 0 };
            continue;
        }
        if token.kind != TokenKind::Colon || quote_depth != 0 {
            continue;
        }

        let cost_tokens = trim_lexed_commas(&tokens[..idx]);
        let effect_tokens = trim_lexed_commas(&tokens[idx + 1..]);
        if cost_tokens.is_empty() || effect_tokens.is_empty() {
            return Ok(None);
        }

        return Ok(Some((
            cost_tokens.to_vec(),
            render_token_slice(effect_tokens).trim().to_string(),
        )));
    }

    Ok(None)
}

pub(crate) fn parse_text_to_semantic_document(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(RewriteSemanticDocument, ParseAnnotations), CardTextError> {
    if !allow_unsupported && let Some(err) = preflight_known_strict_unsupported(text.as_str()) {
        return Err(err);
    }
    let preprocessed = preprocess_document(builder, text.as_str())?;
    let cst = parse_document_cst(&preprocessed, allow_unsupported)?;
    let semantic = lower_document_cst(preprocessed, cst, allow_unsupported)?;
    let annotations = semantic.annotations.clone();
    Ok((semantic, annotations))
}

pub(crate) fn parse_document_cst(
    preprocessed: &PreprocessedDocument,
    allow_unsupported: bool,
) -> Result<RewriteDocumentCst, CardTextError> {
    let mut lines = Vec::with_capacity(preprocessed.items.len());
    let mut idx = 0usize;
    while idx < preprocessed.items.len() {
        let item = &preprocessed.items[idx];
        match item {
            PreprocessedItem::Metadata(meta) => {
                lines.push(RewriteLineCst::Metadata(metadata_line_cst(
                    meta.info.clone(),
                    meta.value.clone(),
                )?));
                idx += 1;
            }
            PreprocessedItem::Line(line) => {
                if let Some((min_level, max_level)) =
                    parse_level_header(&line.info.normalized.normalized)
                {
                    let mut pt = None;
                    let mut items = Vec::new();
                    let mut probe_idx = idx + 1;
                    while let Some(PreprocessedItem::Line(next_line)) =
                        preprocessed.items.get(probe_idx)
                    {
                        if parse_level_header(&next_line.info.normalized.normalized).is_some() {
                            break;
                        }
                        if parse_saga_chapter_prefix(&next_line.info.normalized.normalized)
                            .is_some()
                        {
                            break;
                        }
                        if let Some(parsed_pt) =
                            parse_power_toughness(&next_line.info.normalized.normalized)
                            && let (PtValue::Fixed(power), PtValue::Fixed(toughness)) =
                                (parsed_pt.power, parsed_pt.toughness)
                        {
                            pt = Some((power, toughness));
                            probe_idx += 1;
                            continue;
                        }
                        match parse_level_item_cst(next_line) {
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
                    if pt.is_none() && items.is_empty() && preprocessed.items.get(idx + 1).is_some()
                    {
                        if allow_unsupported {
                            lines.push(RewriteLineCst::Unsupported(UnsupportedLineCst {
                                info: line.info.clone(),
                                reason_code: "level-header-not-yet-supported",
                            }));
                            idx += 1;
                            continue;
                        }
                        return Err(CardTextError::ParseError(format!(
                            "parser does not yet support level header: '{}'",
                            line.info.raw_line
                        )));
                    }
                    lines.push(RewriteLineCst::LevelHeader(LevelHeaderCst {
                        min_level,
                        max_level,
                        pt,
                        items,
                    }));
                    idx = probe_idx;
                    continue;
                }

                if let Some((chapters, text)) =
                    parse_saga_chapter_prefix(&line.info.normalized.normalized)
                {
                    lines.push(RewriteLineCst::SagaChapter(SagaChapterLineCst {
                        info: line.info.clone(),
                        chapters,
                        text: text.to_string(),
                    }));
                    idx += 1;
                    continue;
                }

                let mut bullet_modes = Vec::new();
                let mut probe_idx = idx + 1;
                while let Some(PreprocessedItem::Line(next_line)) =
                    preprocessed.items.get(probe_idx)
                {
                    if !is_bullet_line(next_line.info.raw_line.as_str()) {
                        break;
                    }
                    let raw_mode = next_line
                        .info
                        .raw_line
                        .trim_start()
                        .trim_start_matches(|c: char| c == '•' || c == '*' || c == '-')
                        .trim();
                    let mode_text = strip_non_keyword_label_prefix(raw_mode).trim().to_string();
                    bullet_modes.push(ModalModeCst {
                        info: next_line.info.clone(),
                        text: mode_text,
                    });
                    probe_idx += 1;
                }
                if !bullet_modes.is_empty() {
                    lines.push(RewriteLineCst::Modal(ModalBlockCst {
                        header: line.info.clone(),
                        modes: bullet_modes,
                    }));
                    idx = probe_idx;
                    continue;
                }

                let normalized = line.info.normalized.normalized.as_str();
                if let Some(chunks) = split_reveal_first_draw_line_rewrite_lexed(&line.tokens) {
                    for chunk in chunks {
                        let chunk_line = rewrite_line_normalized(line, chunk.as_str())?;
                        if line_starts_with_trigger_intro_tokens(&chunk_line.tokens) {
                            for trigger_chunk in
                                split_trigger_sentence_chunks_rewrite_lexed(&chunk_line.tokens)
                            {
                                let trigger_line =
                                    rewrite_line_normalized(&chunk_line, trigger_chunk.as_str())?;
                                lines.push(RewriteLineCst::Triggered(parse_triggered_line_cst(
                                    &trigger_line,
                                )?));
                            }
                        } else if let Some(static_line) = parse_static_line_cst(&chunk_line)? {
                            lines.push(RewriteLineCst::Static(static_line));
                        } else {
                            return Err(CardTextError::ParseError(format!(
                                "parser could not split reveal-first-draw line family: '{}'",
                                line.info.raw_line
                            )));
                        }
                    }
                    idx += 1;
                    continue;
                }
                if let Some(unsupported) = parse_former_section9_unsupported_line_cst(line) {
                    lines.push(RewriteLineCst::Unsupported(unsupported));
                    idx += 1;
                    continue;
                }
                if let Some((prefix, suffix)) =
                    split_trailing_keyword_activation_sentence_lexed(&line.tokens)
                {
                    let prefix_line = rewrite_line_normalized(line, prefix.as_str())?;
                    if let Some(statement_line) = parse_statement_line_cst(&prefix_line)? {
                        lines.push(RewriteLineCst::Statement(statement_line));
                    } else if let Some(rewritten_prefix) = rewrite_named_source_sentence_for_builder(
                        &preprocessed.builder,
                        prefix.as_str(),
                    ) {
                        let rewritten_prefix_line =
                            rewrite_line_normalized(line, rewritten_prefix.as_str())?;
                        if let Some(statement_line) =
                            parse_statement_line_cst(&rewritten_prefix_line)?
                        {
                            lines.push(RewriteLineCst::Statement(statement_line));
                        } else if let Some(static_line) =
                            parse_static_line_cst(&rewritten_prefix_line)?
                        {
                            lines.push(RewriteLineCst::Static(static_line));
                        } else {
                            return Err(CardTextError::ParseError(format!(
                                "parser could not split leading sentence before keyword ability: '{}'",
                                line.info.raw_line
                            )));
                        }
                    } else if let Some(static_line) = parse_static_line_cst(&prefix_line)? {
                        lines.push(RewriteLineCst::Static(static_line));
                    } else {
                        return Err(CardTextError::ParseError(format!(
                            "parser could not split leading sentence before keyword ability: '{}'",
                            line.info.raw_line
                        )));
                    }

                    let suffix_line = rewrite_line_normalized(line, suffix.as_str())?;
                    let Some((_label, body)) = split_label_prefix(suffix.as_str()) else {
                        return Err(CardTextError::ParseError(format!(
                            "parser could not recover keyword activation suffix: '{}'",
                            line.info.raw_line
                        )));
                    };
                    let Some((cost_tokens, effect_text)) =
                        split_activation_text_parts(body, line.info.line_index)?
                    else {
                        return Err(CardTextError::ParseError(format!(
                            "parser could not recover activation suffix: '{}'",
                            line.info.raw_line
                        )));
                    };
                    let cost = parse_activation_cost_tokens_rewrite(&cost_tokens)?;
                    lines.push(RewriteLineCst::Activated(ActivatedLineCst {
                        info: suffix_line.info.clone(),
                        cost,
                        effect_text,
                        chosen_option_label: None,
                    }));
                    idx += 1;
                    continue;
                }
                if normalized
                    == "this effect can't reduce the mana in that cost to less than one mana."
                {
                    idx += 1;
                    continue;
                }
                if !allow_unsupported
                    && let Some(err) = diagnose_known_unsupported_rewrite_line(normalized)
                {
                    return Err(err);
                }

                if let Some((label, body)) = split_label_prefix(normalized) {
                    let is_named_label = is_named_ability_label(label);
                    let preserve_as_choice_label =
                        labeled_choice_block_has_peer(&preprocessed.items, idx);
                    if !preserve_keyword_prefix_for_parse(label) {
                        let body_line = rewrite_line_normalized(line, body)?;
                        if line_starts_with_trigger_intro_tokens(&body_line.tokens) {
                            if let Ok(mut triggered) = parse_triggered_line_cst(&body_line) {
                                if preserve_as_choice_label {
                                    triggered.chosen_option_label =
                                        Some(label.to_ascii_lowercase());
                                }
                                lines.push(RewriteLineCst::Triggered(triggered));
                                idx += 1;
                                continue;
                            }
                            if let Some(mut triggered) =
                                try_parse_triggered_line_with_named_source_rewrite(
                                    &preprocessed.builder,
                                    line,
                                    body,
                                )?
                            {
                                if preserve_as_choice_label {
                                    triggered.chosen_option_label =
                                        Some(label.to_ascii_lowercase());
                                }
                                lines.push(RewriteLineCst::Triggered(triggered));
                                idx += 1;
                                continue;
                            }
                            if allow_unsupported && is_named_label {
                                lines.push(RewriteLineCst::Unsupported(UnsupportedLineCst {
                                    info: line.info.clone(),
                                    reason_code: "triggered-line-not-yet-supported",
                                }));
                                idx += 1;
                                continue;
                            }
                            if is_named_label {
                                return Err(parse_triggered_line_cst(&body_line)
                                    .err()
                                    .unwrap_or_else(|| {
                                        CardTextError::ParseError(format!(
                                            "unsupported triggered line: '{}'",
                                            body
                                        ))
                                    }));
                            }
                        }

                        if is_named_label
                            && let Some(keyword_line) = parse_keyword_line_cst(&body_line)?
                        {
                            lines.push(RewriteLineCst::Keyword(keyword_line));
                            idx += 1;
                            continue;
                        }

                        if (!str_starts_with_char(line.info.raw_line.trim_start(), '(')
                            || is_fully_parenthetical_line(line.info.raw_line.as_str()))
                            && let Some((cost_tokens, effect_text)) =
                                split_activation_text_parts(body, line.info.line_index)?
                        {
                            let cost_text = render_token_slice(&cost_tokens);
                            match parse_activation_cost_tokens_rewrite(&cost_tokens) {
                                Ok(cost) => {
                                    lines.push(RewriteLineCst::Activated(ActivatedLineCst {
                                        info: line.info.clone(),
                                        cost,
                                        effect_text,
                                        chosen_option_label: preserve_as_choice_label
                                            .then(|| label.to_ascii_lowercase()),
                                    }));
                                    idx += 1;
                                    continue;
                                }
                                Err(err)
                                    if looks_like_activation_cost_prefix(cost_text.as_str()) =>
                                {
                                    return Err(err);
                                }
                                Err(_) => {}
                            }
                        }

                        if let Some(mut static_line) = parse_static_line_cst(&body_line)? {
                            if preserve_as_choice_label {
                                static_line.chosen_option_label = Some(label.to_ascii_lowercase());
                            }
                            lines.push(RewriteLineCst::Static(static_line));
                            idx += 1;
                            continue;
                        }

                        if let Some(statement_line) = parse_statement_line_cst(&body_line)? {
                            lines.push(RewriteLineCst::Statement(statement_line));
                            idx += 1;
                            continue;
                        }
                    }
                }

                if line_starts_with_trigger_intro_tokens(&line.tokens) {
                    let trigger_chunks = split_trigger_sentence_chunks_rewrite_lexed(&line.tokens);
                    if trigger_chunks.len() > 1 {
                        for chunk in trigger_chunks {
                            let chunk_line = rewrite_line_normalized(line, chunk.as_str())?;
                            match parse_triggered_line_cst(&chunk_line) {
                                Ok(triggered) => lines.push(RewriteLineCst::Triggered(triggered)),
                                Err(_) => {
                                    if let Some(triggered) =
                                        try_parse_triggered_line_with_named_source_rewrite(
                                            &preprocessed.builder,
                                            line,
                                            chunk.as_str(),
                                        )?
                                    {
                                        lines.push(RewriteLineCst::Triggered(triggered));
                                        continue;
                                    }
                                    if allow_unsupported {
                                        lines.push(RewriteLineCst::Unsupported(
                                            UnsupportedLineCst {
                                                info: line.info.clone(),
                                                reason_code: "triggered-line-not-yet-supported",
                                            },
                                        ))
                                    } else {
                                        return Err(parse_triggered_line_cst(&chunk_line)
                                            .err()
                                            .unwrap_or_else(|| {
                                                CardTextError::ParseError(format!(
                                                    "unsupported triggered line: '{}'",
                                                    chunk
                                                ))
                                            }));
                                    }
                                }
                            }
                        }
                        idx += 1;
                        continue;
                    }

                    match parse_triggered_line_cst(line) {
                        Ok(triggered) => {
                            lines.push(RewriteLineCst::Triggered(triggered));
                            idx += 1;
                            continue;
                        }
                        Err(_) => {
                            if let Some(triggered) =
                                try_parse_triggered_line_with_named_source_rewrite(
                                    &preprocessed.builder,
                                    line,
                                    &line.info.normalized.normalized,
                                )?
                            {
                                lines.push(RewriteLineCst::Triggered(triggered));
                                idx += 1;
                                continue;
                            }
                            if allow_unsupported {
                                lines.push(RewriteLineCst::Unsupported(UnsupportedLineCst {
                                    info: line.info.clone(),
                                    reason_code: "triggered-line-not-yet-supported",
                                }));
                                idx += 1;
                                continue;
                            }
                            return Err(parse_triggered_line_cst(line).err().unwrap_or_else(
                                || {
                                    CardTextError::ParseError(format!(
                                        "unsupported triggered line: '{}'",
                                        line.info.raw_line
                                    ))
                                },
                            ));
                        }
                    }
                }

                if let Some(keyword_line) = parse_keyword_line_cst(line)? {
                    lines.push(RewriteLineCst::Keyword(keyword_line));
                    idx += 1;
                    continue;
                }

                if str_starts_with(normalized, "ward—")
                    || str_starts_with(normalized, "ward ")
                    || str_starts_with(normalized, "echo—")
                    || str_starts_with(normalized, "echo ")
                {
                    lines.push(RewriteLineCst::Static(StaticLineCst {
                        info: line.info.clone(),
                        text: normalized.to_string(),
                        chosen_option_label: None,
                    }));
                    idx += 1;
                    continue;
                }

                let activation_text = split_label_prefix(normalized)
                    .filter(|(label, _)| is_named_ability_label(label))
                    .map(|(_, body)| body)
                    .unwrap_or(normalized);
                if (!str_starts_with_char(line.info.raw_line.trim_start(), '(')
                    || is_fully_parenthetical_line(line.info.raw_line.as_str()))
                    && let Some((cost_tokens, effect_text)) =
                        split_activation_text_parts(activation_text, line.info.line_index)?
                {
                    let cost_text = render_token_slice(&cost_tokens);
                    match parse_activation_cost_tokens_rewrite(&cost_tokens) {
                        Ok(cost) => {
                            lines.push(RewriteLineCst::Activated(ActivatedLineCst {
                                info: line.info.clone(),
                                cost,
                                effect_text,
                                chosen_option_label: None,
                            }));
                            idx += 1;
                            continue;
                        }
                        Err(err) if looks_like_activation_cost_prefix(cost_text.as_str()) => {
                            return Err(err);
                        }
                        Err(_) => {}
                    }
                }

                if let Some(PreprocessedItem::Line(next_line)) = preprocessed.items.get(idx + 1) {
                    let normalized_next = next_line.info.normalized.normalized.as_str();
                    let should_try_combined_static =
                        (str_starts_with(normalized, "as this land enters")
                            && str_contains(normalized, "you may reveal")
                            && str_contains(normalized, "from your hand")
                            && (str_starts_with(
                                normalized_next,
                                "if you dont, this land enters tapped",
                            ) || str_starts_with(
                                normalized_next,
                                "if you don't, this land enters tapped",
                            ) || str_starts_with(
                                normalized_next,
                                "if you dont, it enters tapped",
                            ) || str_starts_with(
                                normalized_next,
                                "if you don't, it enters tapped",
                            )))
                            || (str_starts_with(
                                normalized,
                                "if this card is in your opening hand",
                            ) && str_contains(normalized, "you may begin the game with")
                                && str_contains(normalized, "on the battlefield")
                                && (str_starts_with(normalized_next, "if you do, exile ")
                                    || str_starts_with(normalized_next, "if you do exile ")));

                    if should_try_combined_static {
                        let combined_text = format!(
                            "{}. {}",
                            normalized.trim_end_matches('.'),
                            normalized_next.trim_end_matches('.')
                        );
                        let combined_line = rewrite_line_normalized(line, combined_text.as_str())?;
                        if let Some(static_line) = parse_static_line_cst(&combined_line)? {
                            lines.push(RewriteLineCst::Static(static_line));
                            idx += 2;
                            continue;
                        }
                    }
                }

                if looks_like_pact_next_upkeep_line_tokens(&line.tokens)
                    || looks_like_statement_line(normalized)
                {
                    if let Some(statement_line) = parse_statement_line_cst(line)? {
                        lines.push(RewriteLineCst::Statement(statement_line));
                        idx += 1;
                        continue;
                    }
                }

                if let Some(static_line) = parse_static_line_cst(line)? {
                    lines.push(RewriteLineCst::Static(static_line));
                    idx += 1;
                    continue;
                }

                if let Some(statement_line) = parse_statement_line_cst(line)? {
                    lines.push(RewriteLineCst::Statement(statement_line));
                    idx += 1;
                    continue;
                }

                if let Some(statement_line) =
                    parse_colon_nonactivation_statement_fallback(line, normalized)?
                {
                    lines.push(RewriteLineCst::Statement(statement_line));
                    idx += 1;
                    continue;
                }

                if allow_unsupported {
                    lines.push(RewriteLineCst::Unsupported(UnsupportedLineCst {
                        info: line.info.clone(),
                        reason_code: if looks_like_pact_next_upkeep_line_tokens(&line.tokens) {
                            "statement-line-not-yet-supported"
                        } else {
                            classify_unsupported_line_reason(line)
                        },
                    }));
                    idx += 1;
                    continue;
                }

                return Err(CardTextError::ParseError(format!(
                    "parser does not yet support line family: '{}'",
                    line.info.raw_line
                )));
            }
        }
    }

    Ok(RewriteDocumentCst { lines })
}

fn lower_document_cst(
    preprocessed: PreprocessedDocument,
    cst: RewriteDocumentCst,
    allow_unsupported: bool,
) -> Result<RewriteSemanticDocument, CardTextError> {
    let mut builder = preprocessed.builder;
    let mut items = Vec::with_capacity(cst.lines.len());

    for line in cst.lines {
        match line {
            RewriteLineCst::Metadata(MetadataLineCst { value }) => {
                builder = builder.apply_metadata(value.clone())?;
                items.push(RewriteSemanticItem::Metadata);
            }
            RewriteLineCst::Keyword(keyword) => {
                let kind = match keyword.kind {
                    KeywordLineKindCst::AdditionalCost => RewriteKeywordLineKind::AdditionalCost,
                    KeywordLineKindCst::AdditionalCostChoice => {
                        RewriteKeywordLineKind::AdditionalCostChoice
                    }
                    KeywordLineKindCst::AlternativeCast => RewriteKeywordLineKind::AlternativeCast,
                    KeywordLineKindCst::Bestow => RewriteKeywordLineKind::Bestow,
                    KeywordLineKindCst::Bargain => RewriteKeywordLineKind::Bargain,
                    KeywordLineKindCst::Buyback => RewriteKeywordLineKind::Buyback,
                    KeywordLineKindCst::Channel => RewriteKeywordLineKind::Channel,
                    KeywordLineKindCst::Cycling => RewriteKeywordLineKind::Cycling,
                    KeywordLineKindCst::Equip => RewriteKeywordLineKind::Equip,
                    KeywordLineKindCst::Escape => RewriteKeywordLineKind::Escape,
                    KeywordLineKindCst::Flashback => RewriteKeywordLineKind::Flashback,
                    KeywordLineKindCst::Harmonize => RewriteKeywordLineKind::Harmonize,
                    KeywordLineKindCst::Kicker => RewriteKeywordLineKind::Kicker,
                    KeywordLineKindCst::Madness => RewriteKeywordLineKind::Madness,
                    KeywordLineKindCst::Morph => RewriteKeywordLineKind::Morph,
                    KeywordLineKindCst::Multikicker => RewriteKeywordLineKind::Multikicker,
                    KeywordLineKindCst::Offspring => RewriteKeywordLineKind::Offspring,
                    KeywordLineKindCst::Reinforce => RewriteKeywordLineKind::Reinforce,
                    KeywordLineKindCst::Squad => RewriteKeywordLineKind::Squad,
                    KeywordLineKindCst::Transmute => RewriteKeywordLineKind::Transmute,
                    KeywordLineKindCst::Entwine => RewriteKeywordLineKind::Entwine,
                    KeywordLineKindCst::CastThisSpellOnly => {
                        RewriteKeywordLineKind::CastThisSpellOnly
                    }
                    KeywordLineKindCst::Gift => RewriteKeywordLineKind::Gift,
                    KeywordLineKindCst::Warp => RewriteKeywordLineKind::Warp,
                    KeywordLineKindCst::ExertAttack => RewriteKeywordLineKind::ExertAttack,
                };
                let parsed =
                    lower_rewrite_keyword_to_chunk(keyword.info.clone(), &keyword.text, kind)?;
                items.push(RewriteSemanticItem::Keyword(RewriteKeywordLine {
                    info: keyword.info,
                    text: keyword.text,
                    kind,
                    parsed,
                }));
            }
            RewriteLineCst::Activated(activated) => {
                let cost = match lower_activation_cost_cst(&activated.cost) {
                    Ok(cost) => cost,
                    Err(err) => {
                        if allow_unsupported {
                            items.push(RewriteSemanticItem::Unsupported(RewriteUnsupportedLine {
                                info: activated.info,
                                reason_code: "activated-cost-not-yet-supported",
                            }));
                            continue;
                        }
                        return Err(err);
                    }
                };
                let lowered = lower_rewrite_activated_to_chunk(
                    activated.info.clone(),
                    cost.clone(),
                    activated.effect_text.clone(),
                    ActivationTiming::AnyTime,
                    activated.chosen_option_label.clone(),
                )?;
                items.push(RewriteSemanticItem::Activated(RewriteActivatedLine {
                    info: activated.info,
                    cost,
                    effect_text: activated.effect_text,
                    timing_hint: ActivationTiming::AnyTime,
                    chosen_option_label: activated.chosen_option_label,
                    parsed: lowered.chunk,
                    restrictions: lowered.restrictions,
                }));
            }
            RewriteLineCst::Triggered(triggered) => {
                let parsed = lower_rewrite_triggered_to_chunk(
                    triggered.info.clone(),
                    &triggered.full_text,
                    &triggered.trigger_text,
                    &triggered.effect_text,
                    triggered.max_triggers_per_turn,
                    triggered.chosen_option_label.as_deref(),
                )?;
                items.push(RewriteSemanticItem::Triggered(RewriteTriggeredLine {
                    info: triggered.info,
                    full_text: triggered.full_text,
                    trigger_text: triggered.trigger_text,
                    effect_text: triggered.effect_text,
                    max_triggers_per_turn: triggered.max_triggers_per_turn,
                    chosen_option_label: triggered.chosen_option_label,
                    parsed,
                }));
            }
            RewriteLineCst::Static(static_line) => {
                let parsed = if static_line.text == "activate only once each turn." {
                    LineAst::Statement {
                        effects: Vec::new(),
                    }
                } else {
                    lower_rewrite_static_to_chunk(
                        static_line.info.clone(),
                        &static_line.text,
                        static_line.chosen_option_label.as_deref(),
                    )?
                };
                items.push(RewriteSemanticItem::Static(RewriteStaticLine {
                    info: static_line.info,
                    text: static_line.text,
                    chosen_option_label: static_line.chosen_option_label,
                    parsed,
                }));
            }
            RewriteLineCst::Statement(statement_line) => {
                let parsed_chunks = lower_rewrite_statement_to_chunks(
                    statement_line.info.clone(),
                    &statement_line.text,
                )?;
                items.push(RewriteSemanticItem::Statement(RewriteStatementLine {
                    info: statement_line.info,
                    text: statement_line.text,
                    parsed_chunks,
                }));
            }
            RewriteLineCst::Modal(modal) => {
                items.push(RewriteSemanticItem::Modal(RewriteModalBlock {
                    header: modal.header,
                    modes: modal
                        .modes
                        .into_iter()
                        .map(|mode| {
                            let parse_text = strip_non_keyword_label_prefix(
                                mode.info.normalized.normalized.as_str(),
                            )
                            .trim();
                            let effects_ast = lexed_tokens(parse_text, mode.info.line_index)
                                .and_then(|tokens| parse_effect_sentences_lexed(&tokens))?;
                            Ok(RewriteModalMode {
                                info: mode.info,
                                text: mode.text,
                                effects_ast,
                            })
                        })
                        .collect::<Result<Vec<_>, CardTextError>>()?,
                }));
            }
            RewriteLineCst::LevelHeader(level) => {
                items.push(RewriteSemanticItem::LevelHeader(RewriteLevelHeader {
                    min_level: level.min_level,
                    max_level: level.max_level,
                    pt: level.pt,
                    items: level
                        .items
                        .into_iter()
                        .map(|item| {
                            let parsed = match item.kind {
                                LevelItemKindCst::KeywordActions => {
                                    let tokens = lexed_tokens(item.text.as_str(), item.info.line_index)?;
                                    let actions = parse_ability_line_lexed(&tokens).ok_or_else(|| {
                                        CardTextError::ParseError(format!(
                                            "rewrite level lowering could not parse keyword line '{}'",
                                            item.info.raw_line
                                        ))
                                    })?;
                                    ParsedLevelAbilityItemAst::KeywordActions(actions)
                                }
                                LevelItemKindCst::StaticAbilities => {
                                    let tokens =
                                        lexed_tokens(item.text.as_str(), item.info.line_index)?;
                                    let abilities =
                                        parse_static_ability_ast_line_lexed(&tokens)?
                                            .ok_or_else(|| {
                                                CardTextError::ParseError(format!(
                                                    "rewrite level lowering could not parse static line '{}'",
                                                    item.info.raw_line
                                                ))
                                            })?;
                                    ParsedLevelAbilityItemAst::StaticAbilities(abilities)
                                }
                            };
                            Ok(RewriteLevelItem {
                                info: item.info,
                                text: item.text,
                                kind: match item.kind {
                                    LevelItemKindCst::KeywordActions => {
                                        RewriteLevelItemKind::KeywordActions
                                    }
                                    LevelItemKindCst::StaticAbilities => {
                                        RewriteLevelItemKind::StaticAbilities
                                    }
                                },
                                parsed,
                            })
                        })
                        .collect::<Result<Vec<_>, CardTextError>>()?,
                }));
            }
            RewriteLineCst::SagaChapter(saga) => {
                let effects_ast = lexed_tokens(saga.text.as_str(), saga.info.line_index)
                    .and_then(|tokens| parse_effect_sentences_lexed(&tokens))?;
                items.push(RewriteSemanticItem::SagaChapter(RewriteSagaChapterLine {
                    info: saga.info,
                    chapters: saga.chapters,
                    text: saga.text,
                    effects_ast,
                }));
            }
            RewriteLineCst::Unsupported(unsupported) => {
                items.push(RewriteSemanticItem::Unsupported(RewriteUnsupportedLine {
                    info: unsupported.info,
                    reason_code: unsupported.reason_code,
                }));
            }
        }
    }

    Ok(RewriteSemanticDocument {
        builder,
        annotations: preprocessed.annotations,
        items,
        allow_unsupported,
    })
}

pub(crate) fn metadata_line_cst(
    info: crate::cards::builders::LineInfo,
    value: crate::cards::builders::MetadataLine,
) -> Result<MetadataLineCst, CardTextError> {
    let _ = info;
    Ok(MetadataLineCst { value })
}
