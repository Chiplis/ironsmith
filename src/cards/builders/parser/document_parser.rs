use crate::PtValue;
use crate::ability::ActivationTiming;
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, LineAst, ParseAnnotations, ParsedLevelAbilityItemAst,
    TextSpan,
};
use winnow::Parser;
use winnow::error::ModalResult as WResult;
use winnow::stream::Stream;
use winnow::token::any;

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
    LexStream, OwnedLexToken, TokenKind, TokenWordView, lex_line, render_token_slice,
    token_word_refs, trim_lexed_commas,
};
use super::lower::{
    lower_rewrite_activated_to_chunk, lower_rewrite_keyword_to_chunk,
    lower_rewrite_statement_token_groups_to_chunks, lower_rewrite_static_to_chunk,
    lower_rewrite_triggered_to_chunk,
};
use super::preprocess::{
    PreprocessedDocument, PreprocessedItem, PreprocessedLine, preprocess_document,
};
use super::token_primitives::{
    clone_sentence_chunk_tokens, find_index as find_token_index,
    lexed_tokens_contain_non_prefix_instead, remove_copy_exception_type_removal_lexed,
    rewrite_followup_intro_to_if_lexed, split_em_dash_label_prefix,
    split_em_dash_label_prefix_tokens, str_contains, str_ends_with, str_ends_with_char,
    str_split_once, str_split_once_char, str_starts_with, str_starts_with_char, str_strip_prefix,
    str_strip_suffix,
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
    if let Some((intro, _)) = grammar::parse_prefix(
        tokens,
        winnow::combinator::alt((
            grammar::kw("when").value(TriggerIntroCst::When),
            grammar::kw("whenever").value(TriggerIntroCst::Whenever),
        )),
    ) {
        return Some(intro);
    }

    super::parser_support::is_at_trigger_intro_lexed(tokens, 0).then_some(TriggerIntroCst::At)
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

fn line_starts_with_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> bool {
    if super::parser_support::looks_like_reflexive_followup_intro_lexed(tokens) {
        return false;
    }
    parse_trigger_intro_tokens(tokens).is_some()
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

fn nested_combat_whenever_clause_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, after_intro) = grammar::parse_prefix(
        tokens,
        grammar::phrase(&["at", "the", "beginning", "of", "each", "combat"]),
    )?;
    let after_unless = trim_lexed_commas(after_intro);
    let (_, after_pay) =
        grammar::parse_prefix(after_unless, grammar::phrase(&["unless", "you", "pay"]))?;
    let (_, nested_trigger_tokens) = grammar::split_lexed_once_on_comma(after_pay)?;
    nested_trigger_tokens
        .first()
        .is_some_and(|token| token.is_word("whenever"))
        .then_some(nested_trigger_tokens)
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
    let Some((_, head_tokens)) = grammar::strip_lexed_suffix_phrases(
        tokens,
        &[&["untap", "during", "your", "untap", "step"]],
    ) else {
        return false;
    };

    let head_tokens = trim_lexed_commas(head_tokens);
    if head_tokens.is_empty() {
        return false;
    }

    grammar::find_prefix(head_tokens, || {
        winnow::combinator::alt((
            grammar::kw("don't").void(),
            grammar::kw("dont").void(),
            grammar::kw("doesn't").void(),
            grammar::kw("doesnt").void(),
            (grammar::kw("do"), grammar::kw("not")).void(),
            (grammar::kw("does"), grammar::kw("not")).void(),
        ))
    })
    .is_some()
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

fn parse_dont_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    winnow::combinator::alt((grammar::kw("don't"), grammar::kw("dont")))
        .void()
        .parse_next(input)
}

fn is_ward_or_echo_static_prefix_tokens(tokens: &[OwnedLexToken]) -> bool {
    grammar::parse_prefix(
        tokens,
        winnow::combinator::alt((grammar::kw("ward"), grammar::kw("echo"))),
    )
    .is_some()
}

fn is_land_reveal_enters_static_tokens(tokens: &[OwnedLexToken]) -> bool {
    grammar::parse_prefix(tokens, grammar::phrase(&["as", "this", "land", "enters"])).is_some()
        && grammar::contains_phrase(tokens, &["you", "may", "reveal"])
        && grammar::contains_phrase(tokens, &["from", "your", "hand"])
}

fn is_land_reveal_enters_tapped_followup_tokens(tokens: &[OwnedLexToken]) -> bool {
    grammar::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        (
            grammar::phrase(&["if", "you"]),
            parse_dont_word,
            winnow::combinator::opt(grammar::comma()),
            winnow::combinator::alt((
                grammar::phrase(&["this", "land", "enters", "tapped"]),
                grammar::phrase(&["it", "enters", "tapped"]),
            )),
        )
            .void()
            .parse_next(input)
    })
    .is_some()
}

fn is_opening_hand_begin_game_static_tokens(tokens: &[OwnedLexToken]) -> bool {
    grammar::parse_prefix(
        tokens,
        grammar::phrase(&["if", "this", "card", "is", "in", "your", "opening", "hand"]),
    )
    .is_some()
        && grammar::contains_phrase(tokens, &["you", "may", "begin", "the", "game", "with"])
        && grammar::contains_phrase(tokens, &["on", "the", "battlefield"])
}

fn is_if_you_do_exile_followup_tokens(tokens: &[OwnedLexToken]) -> bool {
    grammar::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        (
            grammar::phrase(&["if", "you", "do"]),
            winnow::combinator::opt(grammar::comma()),
            grammar::kw("exile"),
        )
            .void()
            .parse_next(input)
    })
    .is_some()
}

fn should_try_combined_static_tokens(
    line_tokens: &[OwnedLexToken],
    next_line_tokens: &[OwnedLexToken],
) -> bool {
    (is_land_reveal_enters_static_tokens(line_tokens)
        && is_land_reveal_enters_tapped_followup_tokens(next_line_tokens))
        || (is_opening_hand_begin_game_static_tokens(line_tokens)
            && is_if_you_do_exile_followup_tokens(next_line_tokens))
}

#[derive(Debug, Clone)]
struct TriggeredSplitCandidate {
    trigger_text: String,
    trigger_parse_tokens: Vec<OwnedLexToken>,
    effect_text: String,
    effect_parse_tokens: Vec<OwnedLexToken>,
    max_triggers_per_turn: Option<u32>,
}

impl TriggeredSplitCandidate {
    fn into_cst(
        self,
        line: &PreprocessedLine,
        full_parse_tokens: &[OwnedLexToken],
    ) -> TriggeredLineCst {
        TriggeredLineCst {
            info: line.info.clone(),
            full_text: render_token_slice(full_parse_tokens).trim().to_string(),
            full_parse_tokens: full_parse_tokens.to_vec(),
            trigger_text: self.trigger_text,
            trigger_parse_tokens: self.trigger_parse_tokens,
            effect_text: self.effect_text,
            effect_parse_tokens: self.effect_parse_tokens,
            max_triggers_per_turn: self.max_triggers_per_turn,
            chosen_option_label: None,
        }
    }
}

#[derive(Debug, Clone)]
enum TriggeredSplitProbe {
    Empty,
    Supported(TriggeredSplitCandidate),
    Unsupported {
        candidate: TriggeredSplitCandidate,
        trigger_error: Option<CardTextError>,
        effect_error: Option<CardTextError>,
    },
}

impl TriggeredSplitProbe {
    fn supported_cst(
        &self,
        line: &PreprocessedLine,
        full_parse_tokens: &[OwnedLexToken],
    ) -> Option<TriggeredLineCst> {
        match self {
            Self::Supported(candidate) => Some(candidate.clone().into_cst(line, full_parse_tokens)),
            _ => None,
        }
    }

    fn fallback_cst(
        &self,
        line: &PreprocessedLine,
        full_parse_tokens: &[OwnedLexToken],
    ) -> Option<TriggeredLineCst> {
        match self {
            Self::Supported(candidate) => Some(candidate.clone().into_cst(line, full_parse_tokens)),
            Self::Unsupported { candidate, .. } => {
                Some(candidate.clone().into_cst(line, full_parse_tokens))
            }
            Self::Empty => None,
        }
    }

    fn preferred_error(&self) -> Option<CardTextError> {
        match self {
            Self::Unsupported {
                trigger_error,
                effect_error,
                ..
            } => effect_error.clone().or_else(|| trigger_error.clone()),
            Self::Empty | Self::Supported(_) => None,
        }
    }
}

fn render_triggered_split_candidate(
    trigger_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
    trailing_cap: Option<u32>,
) -> Option<TriggeredSplitCandidate> {
    let trigger_candidate_tokens = trim_lexed_commas(trigger_tokens);
    let effect_candidate_tokens = trim_lexed_commas(effect_tokens);
    if trigger_candidate_tokens.is_empty() || effect_candidate_tokens.is_empty() {
        return None;
    }

    let (trigger_tokens, max_triggers_per_turn) =
        strip_trigger_frequency_suffix_tokens(trigger_candidate_tokens);
    let trigger_text = render_token_slice(trigger_tokens).trim().to_string();
    let effect_text = render_token_slice(effect_candidate_tokens)
        .trim()
        .to_string();
    if trigger_text.is_empty() || effect_text.is_empty() {
        return None;
    }

    Some(TriggeredSplitCandidate {
        trigger_text,
        trigger_parse_tokens: trigger_tokens.to_vec(),
        effect_text,
        effect_parse_tokens: effect_candidate_tokens.to_vec(),
        max_triggers_per_turn: max_triggers_per_turn.or(trailing_cap),
    })
}

fn probe_triggered_split(
    trigger_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
    trailing_cap: Option<u32>,
) -> TriggeredSplitProbe {
    let trigger_candidate_tokens = trim_lexed_commas(trigger_tokens);
    let effect_candidate_tokens = trim_lexed_commas(effect_tokens);
    let Some(candidate) =
        render_triggered_split_candidate(trigger_tokens, effect_tokens, trailing_cap)
    else {
        return TriggeredSplitProbe::Empty;
    };
    let (trigger_tokens, _) = strip_trigger_frequency_suffix_tokens(trigger_candidate_tokens);

    let trigger_error = parse_trigger_clause_lexed(trigger_tokens).err();
    let effect_error = parse_effect_sentences_lexed(effect_candidate_tokens).err();
    if trigger_error.is_none() && effect_error.is_none() {
        TriggeredSplitProbe::Supported(candidate)
    } else {
        TriggeredSplitProbe::Unsupported {
            candidate,
            trigger_error,
            effect_error,
        }
    }
}

fn parse_triggered_line_cst(line: &PreprocessedLine) -> Result<TriggeredLineCst, CardTextError> {
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

    if let Some(nested_trigger_tokens) = nested_combat_whenever_clause_tokens(tokens_without_cap) {
        let nested_line = rewrite_line_tokens(line, nested_trigger_tokens);
        if let Ok(parsed) = parse_triggered_line_cst(&nested_line) {
            return Ok(parsed);
        }
    }

    let mut best_probe_error = None;

    if let Some(spec) =
        super::grammar::structure::split_triggered_conditional_clause_lexed(tokens_without_cap, 1)
    {
        let probe = probe_triggered_split(spec.trigger_tokens, spec.effects_tokens, trailing_cap);
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
            let probe = probe_triggered_split(&leading_tokens[1..], effect_tokens, trailing_cap);
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
            trailing_cap,
        );

        if let Some(parsed) = probe.supported_cst(line, tokens_without_cap) {
            // Prefer the split with the most effect tokens (latest separator
            // = largest effects portion).  This prevents silent truncation
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
        // line — this catches silent truncation where voting, conditional, or
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
                chosen_option_label: None,
            })
        }
        Err(err) => Err(best_probe_error.unwrap_or(err)),
    }
}

fn parse_static_line_cst(line: &PreprocessedLine) -> Result<Option<StaticLineCst>, CardTextError> {
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
    if is_doesnt_untap_during_your_untap_step_tokens(&lexed) {
        return Ok(Some(make_static(None)));
    }
    if looks_like_untap_all_during_each_other_players_untap_step_tokens(&lexed) {
        return Ok(Some(make_static(None)));
    }

    if parse_if_this_spell_costs_less_to_cast_line_lexed(&lexed)?.is_some() {
        return Ok(Some(make_static(None)));
    }

    if is_activate_only_once_each_turn_tokens(&lexed) {
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

fn token_words_match(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let words = TokenWordView::new(tokens);
    words.len() == expected.len() && words.slice_eq(0, expected)
}

fn token_words_match_any(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|phrase| token_words_match(tokens, phrase))
}

fn token_words_have_prefix(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    TokenWordView::new(tokens).starts_with(expected)
}

fn token_words_have_any_prefix(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|phrase| token_words_have_prefix(tokens, phrase))
}

fn token_words_have_suffix(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let words = TokenWordView::new(tokens);
    words.len() >= expected.len() && words.slice_eq(words.len() - expected.len(), expected)
}

fn tokens_before_kind(tokens: &[OwnedLexToken], kind: TokenKind) -> &[OwnedLexToken] {
    let end = grammar::find_token_index(tokens, |token| token.kind == kind).unwrap_or(tokens.len());
    &tokens[..end]
}

pub(crate) fn split_compound_buff_and_unblockable_sentence(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let words = TokenWordView::new(tokens);
    let gets_idx = words.find_word("gets")?;
    let and_idx = words.find_phrase_start(&["and", "cant", "be", "blocked"])?;
    if and_idx + 4 != words.len() {
        return None;
    }

    let subject_token_end = words.token_index_for_word_index(gets_idx)?;
    let and_token_idx = words.token_index_for_word_index(and_idx)?;
    let cant_token_idx = words.token_index_for_word_index(and_idx + 1)?;
    if subject_token_end == 0
        || subject_token_end >= and_token_idx
        || cant_token_idx <= and_token_idx
    {
        return None;
    }

    let left_tokens = tokens[..and_token_idx].to_vec();
    let mut right_tokens =
        Vec::with_capacity(subject_token_end + tokens.len().saturating_sub(cant_token_idx));
    right_tokens.extend_from_slice(&tokens[..subject_token_end]);
    right_tokens.extend_from_slice(&tokens[cant_token_idx..]);
    Some((left_tokens, right_tokens))
}

#[derive(Clone, Copy)]
enum KeywordDispatchHint {
    AdditionalCostFamily,
    AlternativeOrExertFamily,
    Bestow,
    Bargain,
    Buyback,
    Channel,
    Cycling,
    Reinforce,
    Equip,
    Kicker,
    Flashback,
    Harmonize,
    Multikicker,
    Entwine,
    Offspring,
    Madness,
    Escape,
    MorphFamily,
    Squad,
    Transmute,
    CastThisSpellOnly,
    Gift,
    Warp,
}

fn parse_keyword_dispatch_hint(tokens: &[OwnedLexToken]) -> Option<KeywordDispatchHint> {
    let hinted = grammar::parse_prefix(
        tokens,
        winnow::combinator::alt((
            winnow::combinator::alt((
                grammar::phrase(&[
                    "as",
                    "an",
                    "additional",
                    "cost",
                    "to",
                    "cast",
                    "this",
                    "spell",
                ])
                .value(KeywordDispatchHint::AdditionalCostFamily),
                grammar::kw("you").value(KeywordDispatchHint::AlternativeOrExertFamily),
                grammar::kw("if").value(KeywordDispatchHint::AlternativeOrExertFamily),
                grammar::kw("bestow").value(KeywordDispatchHint::Bestow),
                grammar::kw("bargain").value(KeywordDispatchHint::Bargain),
                grammar::kw("buyback").value(KeywordDispatchHint::Buyback),
                grammar::kw("channel").value(KeywordDispatchHint::Channel),
                grammar::kw("cycling").value(KeywordDispatchHint::Cycling),
            )),
            winnow::combinator::alt((
                grammar::kw("reinforce").value(KeywordDispatchHint::Reinforce),
                grammar::kw("equip").value(KeywordDispatchHint::Equip),
                grammar::kw("kicker").value(KeywordDispatchHint::Kicker),
                grammar::kw("flashback").value(KeywordDispatchHint::Flashback),
                grammar::kw("harmonize").value(KeywordDispatchHint::Harmonize),
                grammar::kw("multikicker").value(KeywordDispatchHint::Multikicker),
                grammar::kw("entwine").value(KeywordDispatchHint::Entwine),
                grammar::kw("offspring").value(KeywordDispatchHint::Offspring),
            )),
            winnow::combinator::alt((
                grammar::kw("madness").value(KeywordDispatchHint::Madness),
                grammar::kw("escape").value(KeywordDispatchHint::Escape),
                grammar::kw("morph").value(KeywordDispatchHint::MorphFamily),
                grammar::kw("megamorph").value(KeywordDispatchHint::MorphFamily),
                grammar::kw("squad").value(KeywordDispatchHint::Squad),
                grammar::kw("transmute").value(KeywordDispatchHint::Transmute),
                grammar::phrase(&["cast", "this", "spell", "only"])
                    .value(KeywordDispatchHint::CastThisSpellOnly),
                grammar::kw("gift").value(KeywordDispatchHint::Gift),
                grammar::kw("warp").value(KeywordDispatchHint::Warp),
            )),
        )),
    )
    .map(|(hint, _)| hint);
    if hinted.is_some() {
        return hinted;
    }

    let word_view = TokenWordView::new(tokens);
    let first = word_view.get(0)?;
    if first == "basic" {
        if word_view.get(1) == Some("landcycling") {
            return Some(KeywordDispatchHint::Cycling);
        }
        return None;
    }
    if str_strip_suffix(first, "cycling").is_some() {
        return Some(KeywordDispatchHint::Cycling);
    }

    None
}

fn strict_unsupported_triggered_line_error(
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

fn parse_keyword_line_cst(
    line: &PreprocessedLine,
) -> Result<Option<KeywordLineCst>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();
    let tokens = rewrite_keyword_dash_parse_tokens(&line.tokens);

    let kind = match parse_keyword_dispatch_hint(&tokens) {
        Some(KeywordDispatchHint::AdditionalCostFamily) => {
            if parse_additional_cost_kind(&tokens)? {
                Some(KeywordLineKindCst::AdditionalCostChoice)
            } else if additional_cost_tail_tokens(&tokens).is_some() {
                Some(KeywordLineKindCst::AdditionalCost)
            } else {
                None
            }
        }
        Some(KeywordDispatchHint::AlternativeOrExertFamily) => {
            if parse_alternative_cast_kind(&tokens)? {
                Some(KeywordLineKindCst::AlternativeCast)
            } else if is_exert_attack_keyword_line(&tokens) {
                Some(KeywordLineKindCst::ExertAttack)
            } else {
                None
            }
        }
        Some(KeywordDispatchHint::Bestow) => {
            parse_bestow_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Bestow)
        }
        Some(KeywordDispatchHint::Bargain) => {
            parse_bargain_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Bargain)
        }
        Some(KeywordDispatchHint::Buyback) => {
            parse_buyback_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Buyback)
        }
        Some(KeywordDispatchHint::Channel) => {
            parse_channel_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Channel)
        }
        Some(KeywordDispatchHint::Cycling) => {
            parse_cycling_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Cycling)
        }
        Some(KeywordDispatchHint::Reinforce) => {
            parse_reinforce_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Reinforce)
        }
        Some(KeywordDispatchHint::Equip) => {
            parse_equip_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Equip)
        }
        Some(KeywordDispatchHint::Kicker) => {
            parse_kicker_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Kicker)
        }
        Some(KeywordDispatchHint::Flashback) => {
            parse_flashback_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Flashback)
        }
        Some(KeywordDispatchHint::Harmonize) => {
            parse_harmonize_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Harmonize)
        }
        Some(KeywordDispatchHint::Multikicker) => {
            parse_multikicker_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Multikicker)
        }
        Some(KeywordDispatchHint::Entwine) => {
            parse_entwine_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Entwine)
        }
        Some(KeywordDispatchHint::Offspring) => {
            parse_offspring_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Offspring)
        }
        Some(KeywordDispatchHint::Madness) => {
            parse_madness_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Madness)
        }
        Some(KeywordDispatchHint::Escape) => {
            parse_escape_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Escape)
        }
        Some(KeywordDispatchHint::MorphFamily) => {
            if is_morph_family_dash_keyword_line(&line.tokens) {
                None
            } else {
                parse_morph_keyword_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Morph)
            }
        }
        Some(KeywordDispatchHint::Squad) => {
            parse_squad_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Squad)
        }
        Some(KeywordDispatchHint::Transmute) => {
            parse_transmute_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Transmute)
        }
        Some(KeywordDispatchHint::CastThisSpellOnly) => {
            parse_cast_this_spell_only_line_lexed(&tokens)?
                .map(|_| KeywordLineKindCst::CastThisSpellOnly)
        }
        Some(KeywordDispatchHint::Gift) => {
            is_standard_gift_keyword_line(line.info.raw_line.as_str())
                .then_some(KeywordLineKindCst::Gift)
        }
        Some(KeywordDispatchHint::Warp) => {
            parse_warp_line_lexed(&tokens)?.map(|_| KeywordLineKindCst::Warp)
        }
        None => None,
    };

    Ok(kind.map(|kind| KeywordLineCst {
        info: line.info.clone(),
        text: normalized.to_string(),
        parse_tokens: tokens,
        kind,
    }))
}

fn is_morph_family_dash_keyword_line(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .first()
        .is_some_and(|token| token.is_word("morph") || token.is_word("megamorph"))
        && tokens
            .get(1)
            .is_some_and(|token| token.kind == TokenKind::EmDash)
}

fn is_exert_attack_keyword_line(tokens: &[OwnedLexToken]) -> bool {
    token_words_have_any_prefix(
        tokens,
        &[
            &["you", "may", "exert"],
            &[
                "if", "this", "creature", "hasnt", "been", "exerted", "this", "turn", "you", "may",
                "exert",
            ],
        ],
    )
}

fn is_standard_gift_keyword_line(raw_line: &str) -> bool {
    let Ok(tokens) = lexed_tokens(raw_line, 0) else {
        return false;
    };
    is_standard_gift_keyword_tokens(&tokens)
}

fn is_standard_gift_keyword_tokens(tokens: &[OwnedLexToken]) -> bool {
    let head_tokens = tokens_before_kind(tokens, TokenKind::LParen);
    if !token_words_have_prefix(head_tokens, &["gift"]) {
        return false;
    }
    if !grammar::contains_phrase(
        tokens,
        &[
            "you", "may", "promise", "an", "opponent", "a", "gift", "as", "you", "cast", "this",
            "spell",
        ],
    ) || !grammar::contains_phrase(tokens, &["if", "you", "do"])
    {
        return false;
    }

    token_words_have_any_prefix(
        head_tokens,
        &[
            &["gift", "a", "card"],
            &["gift", "a", "treasure"],
            &["gift", "a", "food"],
            &["gift", "a", "tapped", "fish"],
            &["gift", "an", "extra", "turn"],
            &["gift", "an", "octopus"],
        ],
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

fn parse_additional_cost_kind(tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
    if grammar::parse_prefix(
        tokens,
        grammar::phrase(&[
            "as",
            "an",
            "additional",
            "cost",
            "to",
            "cast",
            "this",
            "spell",
        ]),
    )
    .is_none()
    {
        return Ok(false);
    }
    let Some(effect_tokens) = additional_cost_tail_tokens(tokens) else {
        return Ok(false);
    };
    Ok(parse_additional_cost_choice_options_lexed(effect_tokens)?.is_some())
}

fn parse_alternative_cast_kind(tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
    let rendered = render_token_slice(tokens).trim().to_ascii_lowercase();
    Ok(
        parse_self_free_cast_alternative_cost_line_lexed(tokens).is_some()
            || parse_you_may_rather_than_spell_cost_line_lexed(tokens, rendered.as_str())?
                .is_some()
            || parse_if_conditional_alternative_cost_line_lexed(tokens, rendered.as_str())?
                .is_some(),
    )
}

fn parse_level_item_cst(line: &PreprocessedLine) -> Result<Option<LevelItemCst>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();

    if !should_skip_keyword_action_static_probe(&line.tokens)
        && let Some(_actions) = parse_ability_line_lexed(&line.tokens)
    {
        return Ok(Some(LevelItemCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            kind: LevelItemKindCst::KeywordActions,
        }));
    }

    if let Some(_abilities) = parse_static_ability_ast_line_lexed(&line.tokens)? {
        return Ok(Some(LevelItemCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            kind: LevelItemKindCst::StaticAbilities,
        }));
    }

    Ok(None)
}

fn parse_statement_line_cst(
    line: &PreprocessedLine,
) -> Result<Option<StatementLineCst>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();
    let force_statement = looks_like_divvy_statement_line_tokens(&line.tokens)
        || grammar::contains_phrase(
            &line.tokens,
            &[
                "ask", "a", "person", "outside", "the", "game", "to", "rate", "its", "new",
                "art", "on", "a", "scale", "from", "1", "to", "5",
            ],
        )
        || looks_like_pact_next_upkeep_line_tokens(&line.tokens)
        || is_exile_then_owner_may_play_costs_more_statement_line(&line.tokens)
        || looks_like_statement_line_lexed(line);
    if !force_statement
        && parse_static_ability_ast_line_lexed(&line.tokens)
            .ok()
            .flatten()
            .is_some()
    {
        return Ok(None);
    }
    if looks_like_divvy_statement_line_tokens(&line.tokens) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: Vec::new(),
        }));
    }
    if grammar::contains_phrase(
        &line.tokens,
        &[
            "ask", "a", "person", "outside", "the", "game", "to", "rate", "its", "new", "art",
            "on", "a", "scale", "from", "1", "to", "5",
        ],
    ) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: Vec::new(),
        }));
    }
    if looks_like_pact_next_upkeep_line_tokens(&line.tokens) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: Vec::new(),
        }));
    }
    if is_exile_then_owner_may_play_costs_more_statement_line(&line.tokens) {
        return Ok(Some(StatementLineCst {
            info: line.info.clone(),
            text: normalized.to_string(),
            parse_tokens: line.tokens.clone(),
            parse_groups: Vec::new(),
        }));
    }
    if looks_like_untap_all_during_each_other_players_untap_step_tokens(&line.tokens) {
        return Ok(None);
    }
    let parse_groups = statement_parse_groups_lexed(&line.tokens);
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

fn strip_non_keyword_label_prefix_lexed(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while let Some((label, body_tokens)) = split_label_prefix_lexed(tokens) {
        if preserve_keyword_prefix_for_parse(label.as_str()) {
            break;
        }
        tokens = body_tokens;
    }

    tokens
}

fn tokens_after_non_keyword_label_prefix(line: &PreprocessedLine) -> Option<&[OwnedLexToken]> {
    let stripped = strip_non_keyword_label_prefix_lexed(&line.tokens);
    (stripped.len() != line.tokens.len()).then_some(stripped)
}

fn looks_like_statement_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    if looks_like_untap_all_during_each_other_players_untap_step_tokens(tokens) {
        return false;
    }
    if looks_like_granted_quoted_static_line_tokens(tokens) {
        return false;
    }
    looks_like_next_turn_cant_cast_line_tokens(tokens)
        || looks_like_vote_statement_line_tokens(tokens)
        || looks_like_generic_statement_line_tokens(tokens)
}

fn looks_like_granted_quoted_static_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    let Some(quote_idx) = grammar::find_token_index(tokens, |token| token.is_quote()) else {
        return false;
    };
    let head = trim_lexed_commas(&tokens[..quote_idx]);
    if head.is_empty() || !token_words_have_any_prefix(head, &[&["this"], &["it"], &["all"], &["each"]]) {
        return false;
    }
    let words = TokenWordView::new(head);
    words.find_word("has").is_some() || words.find_word("have").is_some()
}

fn looks_like_statement_line_lexed(line: &PreprocessedLine) -> bool {
    if let Some(tokens) = tokens_after_non_keyword_label_prefix(line) {
        return looks_like_statement_line_tokens(&tokens);
    }
    looks_like_statement_line_tokens(&line.tokens)
}

#[cfg(test)]
fn looks_like_statement_line(normalized: &str) -> bool {
    if let Some((_, body)) = split_label_prefix(normalized) {
        return looks_like_statement_line(body);
    }

    lex_line(normalized, 0)
        .ok()
        .is_some_and(|tokens| looks_like_statement_line_tokens(&tokens))
}

fn should_skip_keyword_action_static_probe(tokens: &[OwnedLexToken]) -> bool {
    token_words_have_suffix(tokens, &["cant", "be", "blocked"])
        && !token_words_have_any_prefix(tokens, &[&["this"], &["it"]])
}

fn rewrite_statement_followup_intro_lexed(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    rewrite_followup_intro_to_if_lexed(tokens)
}

fn rewrite_copy_exception_type_removal_lexed(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    remove_copy_exception_type_removal_lexed(tokens)
}

fn rewrite_statement_parse_sentences_lexed(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
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

fn group_statement_parse_sentences_lexed(
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

fn statement_parse_groups_lexed(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let sentence_tokens = rewrite_statement_parse_sentences_lexed(tokens);
    group_statement_parse_sentences_lexed(sentence_tokens, tokens)
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

#[cfg(test)]
fn looks_like_static_line(normalized: &str) -> bool {
    lex_line(normalized, 0)
        .ok()
        .is_some_and(|tokens| looks_like_static_line_tokens(&tokens))
}

fn looks_like_static_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    looks_like_untap_all_during_each_other_players_untap_step_tokens(tokens)
        || looks_like_generic_static_line_tokens(tokens)
}

fn looks_like_static_line_lexed(line: &PreprocessedLine) -> bool {
    if let Some(tokens) = tokens_after_non_keyword_label_prefix(line) {
        return looks_like_static_line_tokens(&tokens);
    }
    looks_like_static_line_tokens(&line.tokens)
}

fn parse_segment_len_until_colon_outside_quotes<'a>(input: &mut LexStream<'a>) -> WResult<usize> {
    let initial_len = input.len();
    let mut inside_quotes = false;

    while let Some(token) = input.peek_token() {
        if token.kind == TokenKind::Quote {
            grammar::quote().parse_next(input)?;
            inside_quotes = !inside_quotes;
            continue;
        }
        if token.kind == TokenKind::Colon && !inside_quotes {
            return Ok(initial_len - input.len());
        }

        any.parse_next(input)?;
    }

    Err(grammar::backtrack_err("colon", "colon outside quotes"))
}

pub(crate) fn split_lexed_once_on_colon_outside_quotes(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let (left_len, rest) =
        grammar::parse_prefix(tokens, parse_segment_len_until_colon_outside_quotes)?;
    let (_, right_tokens) = grammar::parse_prefix(rest, grammar::colon())?;
    Some((&tokens[..left_len], right_tokens))
}

fn split_label_prefix(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim();
    let (label, body) = str_split_once_char(trimmed, '—')?;
    let label = label.trim();
    let body = body.trim();
    (!label.is_empty() && !body.is_empty() && !str_contains(label, ".")).then_some((label, body))
}

fn split_label_prefix_token_slices(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    split_em_dash_label_prefix_tokens(tokens)
}

fn split_label_prefix_lexed(tokens: &[OwnedLexToken]) -> Option<(String, &[OwnedLexToken])> {
    split_em_dash_label_prefix(tokens)
}

fn rewrite_keyword_dash_parse_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let Some((label_tokens, body_tokens)) = split_label_prefix_token_slices(tokens) else {
        return tokens.to_vec();
    };

    let label = render_token_slice(label_tokens).trim().to_ascii_lowercase();
    if matches!(
        label.as_str(),
        "will of the council" | "council's dilemma" | "councils dilemma" | "secret council"
    ) {
        return body_tokens.to_vec();
    }
    if preserve_keyword_prefix_for_parse(label.as_str()) {
        let mut rewritten = Vec::with_capacity(label_tokens.len() + body_tokens.len());
        rewritten.extend(label_tokens.iter().cloned());
        rewritten.extend(body_tokens.iter().cloned());
        return rewritten;
    }

    tokens.to_vec()
}

fn is_nonkeyword_choice_labeled_line(line: &PreprocessedLine) -> bool {
    split_label_prefix_lexed(&line.tokens)
        .is_some_and(|(label, _)| !preserve_keyword_prefix_for_parse(label.as_str()))
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
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentences.len() <= 1 {
        return None;
    }

    for split_idx in 1..sentences.len() {
        let prefix = clone_sentence_chunk_tokens(tokens, &sentences[..split_idx])?;
        let suffix = clone_sentence_chunk_tokens(tokens, &sentences[split_idx..])?;
        let Some((label, body_tokens)) = split_label_prefix_lexed(&suffix) else {
            continue;
        };
        if !preserve_keyword_prefix_for_parse(label.as_str())
            || split_lexed_once_on_colon_outside_quotes(body_tokens).is_none()
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
    use crate::cards::builders::{CardDefinitionBuilder, CardTextError};
    use crate::ids::CardId;
    use crate::types::CardType;

    use super::{
        PreprocessedItem, TriggeredSplitProbe, classify_unsupported_line_reason,
        diagnose_known_unsupported_rewrite_line, is_doesnt_untap_during_your_untap_step_tokens,
        is_if_you_do_exile_followup_tokens, is_land_reveal_enters_static_tokens,
        is_land_reveal_enters_tapped_followup_tokens, is_opening_hand_begin_game_static_tokens,
        is_ward_or_echo_static_prefix_tokens, lex_line, looks_like_divvy_statement_line_tokens,
        looks_like_generic_statement_line_tokens, looks_like_generic_static_line_tokens,
        looks_like_next_turn_cant_cast_line_tokens, looks_like_pact_next_upkeep_line_tokens,
        looks_like_statement_line, looks_like_statement_line_lexed, looks_like_static_line,
        looks_like_static_line_lexed,
        looks_like_untap_all_during_each_other_players_untap_step_tokens,
        looks_like_vote_statement_line_tokens, parse_colon_nonactivation_statement_fallback,
        parse_keyword_line_cst, parse_level_item_cst, parse_statement_line_cst,
        parse_static_line_cst, parse_triggered_line_cst, preprocess_document,
        probe_triggered_split, render_token_slice, rewrite_keyword_dash_parse_tokens,
        split_activation_text_parts_lexed, split_label_prefix, split_label_prefix_lexed,
        split_reveal_first_draw_line_rewrite_lexed,
        split_trailing_keyword_activation_sentence_lexed,
        split_trigger_sentence_chunks_rewrite_lexed, statement_parse_groups_lexed,
        strip_non_keyword_label_prefix, tokens_after_non_keyword_label_prefix,
    };

    fn single_preprocessed_line(text: &str) -> super::PreprocessedLine {
        let document = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Document Parser Test")
                .card_types(vec![CardType::Creature]),
            text,
        )
        .expect("expected preprocess_document to keep test line");
        match document
            .items
            .into_iter()
            .next()
            .expect("expected one preprocessed item")
        {
            PreprocessedItem::Line(line) => line,
            other => panic!("expected preprocessed line, got {other:?}"),
        }
    }

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
    fn split_label_prefix_lexed_reuses_existing_body_tokens() {
        let tokens = lex_line(
            "Secret Council — Each player votes for death or torture.",
            0,
        )
        .expect("rewrite lexer should classify labeled line");

        let (label, body_tokens) =
            split_label_prefix_lexed(&tokens).expect("expected token label prefix split");

        assert_eq!(label, "Secret Council");
        assert_eq!(
            render_token_slice(body_tokens),
            "Each player votes for death or torture."
        );
    }

    #[test]
    fn tokens_after_non_keyword_label_prefix_reuses_chained_body_tokens() {
        let line = single_preprocessed_line(
            "Meteor Strikes — {2} — Double target creature's power and toughness until end of turn.",
        );

        let tokens = tokens_after_non_keyword_label_prefix(&line)
            .expect("expected chained non-keyword label prefix to strip");

        assert_eq!(
            render_token_slice(&tokens),
            "double target creature's power and toughness until end of turn."
        );
    }

    #[test]
    fn rewrite_keyword_dash_parse_tokens_drops_council_label_body_only() {
        let tokens = lex_line(
            "secret council — each player votes for death or torture.",
            0,
        )
        .expect("rewrite lexer should classify council label line");

        let rewritten = rewrite_keyword_dash_parse_tokens(&tokens);

        assert_eq!(
            render_token_slice(&rewritten),
            "each player votes for death or torture."
        );
    }

    #[test]
    fn rewrite_keyword_dash_parse_tokens_keeps_keyword_label_without_dash() {
        let tokens = lex_line("cycling — {2}, discard this card: draw a card.", 0)
            .expect("rewrite lexer should classify keyword label line");

        let rewritten = rewrite_keyword_dash_parse_tokens(&tokens);

        assert_eq!(
            render_token_slice(&rewritten),
            "cycling {2}, discard this card: draw a card."
        );
    }

    #[test]
    fn keyword_line_cst_stores_rewritten_parse_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line("Cycling — {2}, Discard this card: Draw a card.");

        let parsed =
            parse_keyword_line_cst(&line)?.expect("expected cycling line to parse as keyword");

        assert_eq!(
            render_token_slice(&parsed.parse_tokens),
            "cycling {2}, discard this card: draw a card."
        );

        Ok(())
    }

    #[test]
    fn ward_and_echo_static_prefixes_are_token_classified() {
        let ward = lex_line("Ward — Pay 3 life.", 0)
            .expect("rewrite lexer should classify ward static prefix");
        let echo =
            lex_line("Echo {2}{R}", 0).expect("rewrite lexer should classify echo static prefix");

        assert!(is_ward_or_echo_static_prefix_tokens(&ward));
        assert!(is_ward_or_echo_static_prefix_tokens(&echo));
    }

    #[test]
    fn land_reveal_combined_static_pair_is_token_classified() {
        let first = lex_line(
            "As this land enters, you may reveal an Island card from your hand.",
            0,
        )
        .expect("rewrite lexer should classify first static line");
        let second = lex_line("If you don't, it enters tapped.", 0)
            .expect("rewrite lexer should classify followup static line");

        assert!(is_land_reveal_enters_static_tokens(&first));
        assert!(is_land_reveal_enters_tapped_followup_tokens(&second));
    }

    #[test]
    fn opening_hand_begin_game_combined_static_pair_is_token_classified() {
        let first = lex_line(
            "If this card is in your opening hand, you may begin the game with it on the battlefield.",
            0,
        )
        .expect("rewrite lexer should classify opening-hand static line");
        let second = lex_line("If you do exile a card from your hand.", 0)
            .expect("rewrite lexer should classify if-you-do followup line");

        assert!(is_opening_hand_begin_game_static_tokens(&first));
        assert!(is_if_you_do_exile_followup_tokens(&second));
    }

    #[test]
    fn keyword_line_cst_recognizes_gift_family_from_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Gift a card (You may promise an opponent a gift as you cast this spell. If you do, they draw a card before its other effects.)",
        );

        let parsed =
            parse_keyword_line_cst(&line)?.expect("expected gift line to parse as keyword");

        assert!(matches!(parsed.kind, super::KeywordLineKindCst::Gift));

        Ok(())
    }

    #[test]
    fn keyword_line_cst_recognizes_exert_attack_from_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "You may exert this creature as it attacks. (An exerted creature won't untap during your next untap step.)",
        );

        let parsed =
            parse_keyword_line_cst(&line)?.expect("expected exert line to parse as keyword");

        assert!(matches!(
            parsed.kind,
            super::KeywordLineKindCst::ExertAttack
        ));

        Ok(())
    }

    #[test]
    fn static_line_cst_recognizes_compound_unblockable_from_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line("Enchanted creature gets +2/+2 and can't be blocked.");

        assert!(parse_static_line_cst(&line)?.is_some());

        Ok(())
    }

    #[test]
    fn statement_line_cst_recognizes_exile_then_play_costs_more_from_tokens()
    -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Exile target nonland permanent. For as long as that card remains exiled, its owner may play it. A spell cast by an opponent this way costs {2} more to cast.",
        );

        assert!(parse_statement_line_cst(&line)?.is_some());

        Ok(())
    }

    #[test]
    fn unsupported_line_reason_recognizes_modal_header_from_tokens() {
        let line = single_preprocessed_line("Choose one —");

        assert_eq!(
            classify_unsupported_line_reason(&line),
            "modal-header-not-yet-supported"
        );
    }

    #[test]
    fn level_item_cst_stores_parse_tokens() -> Result<(), CardTextError> {
        let line = single_preprocessed_line("Flying");

        let parsed = parse_level_item_cst(&line)?.expect("expected flying to parse as level item");

        assert_eq!(parsed.text, "flying");
        assert_eq!(render_token_slice(&parsed.parse_tokens), parsed.text);

        Ok(())
    }

    #[test]
    fn saga_chapter_cst_stores_parse_tokens() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Saga Parse Tokens Test")
                .card_types(vec![CardType::Enchantment]),
            "I, II — Draw a card.",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;

        match cst.lines.as_slice() {
            [super::RewriteLineCst::SagaChapter(saga)] => {
                assert_eq!(saga.text, "draw a card.");
                assert_eq!(render_token_slice(&saga.parse_tokens), saga.text);
            }
            other => panic!("expected one saga chapter line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn level_header_lowering_keeps_parsed_level_items() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Level Lowering Parse Tokens Test")
                .card_types(vec![CardType::Creature]),
            "Level up {1}\nLEVEL 1-2\nFlying\n3/3",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;
        let semantic = super::lower_document_cst(preprocessed, cst, false)?;

        let level = semantic
            .items
            .iter()
            .find_map(|item| match item {
                super::RewriteSemanticItem::LevelHeader(level) => Some(level),
                _ => None,
            })
            .expect("expected lowered semantic document to contain a level header");

        match level.items.as_slice() {
            [item] => match &item.parsed {
                crate::cards::builders::ParsedLevelAbilityItemAst::KeywordActions(actions) => {
                    assert!(!actions.is_empty());
                }
                other => panic!("expected keyword-actions level item, got {other:?}"),
            },
            other => panic!("expected one lowered level item, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn saga_chapter_lowering_keeps_effects_ast() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Saga Lowering Parse Tokens Test")
                .card_types(vec![CardType::Enchantment]),
            "I, II — Draw a card.",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;
        let semantic = super::lower_document_cst(preprocessed, cst, false)?;

        let saga = semantic
            .items
            .iter()
            .find_map(|item| match item {
                super::RewriteSemanticItem::SagaChapter(saga) => Some(saga),
                _ => None,
            })
            .expect("expected lowered semantic document to contain a saga chapter");

        assert_eq!(saga.text, "draw a card.");
        assert!(!saga.effects_ast.is_empty());

        Ok(())
    }

    #[test]
    fn statement_parse_groups_lexed_strip_labels_and_rewrite_followups() {
        let line = single_preprocessed_line(
            "Meteor Strikes — Exile target artifact. When you do, draw a card.",
        );

        let groups = statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec!["exile target artifact. if you do, draw a card.".to_string()]
        );
    }

    #[test]
    fn statement_parse_groups_lexed_split_instead_followup_into_separate_chunks() {
        let line = single_preprocessed_line(
            "Exile target creature. Return that card to the battlefield under its owner's control instead, then scry 1.",
        );

        let groups = statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec![
                "exile target creature.".to_string(),
                "return that card to the battlefield under its owner's control instead, then scry 1."
                    .to_string(),
            ]
        );
    }

    #[test]
    fn statement_parse_groups_lexed_rewrites_copy_exception_without_relex() {
        let line = single_preprocessed_line(
            "Target artifact becomes a copy of target enchantment, except it's an artifact and it loses all other card types.",
        );

        let groups = statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec![
                "target artifact becomes a copy of target enchantment, except it's an artifact."
                    .to_string()
            ]
        );
    }

    #[test]
    fn statement_parse_groups_lexed_keep_broken_visage_followups_in_one_effect_group() {
        let line = single_preprocessed_line(
            "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.",
        );

        let groups = statement_parse_groups_lexed(&line.tokens)
            .into_iter()
            .map(|group| render_token_slice(&group))
            .collect::<Vec<_>>();

        assert_eq!(
            groups,
            vec![
                "destroy target nonartifact attacking creature. it can't be regenerated. create a black spirit creature token. its power is equal to that creature's power and its toughness is equal to that creature's toughness. sacrifice the token at the beginning of the next end step.".to_string()
            ]
        );
    }

    #[test]
    fn parse_statement_line_cst_does_not_abort_on_broken_visage_static_probe_error(
    ) -> Result<(), CardTextError> {
        let line = single_preprocessed_line(
            "Destroy target nonartifact attacking creature. It can't be regenerated. Create a black Spirit creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness. Sacrifice the token at the beginning of the next end step.",
        );

        assert!(parse_statement_line_cst(&line)?.is_some());

        Ok(())
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
    fn looks_like_lexed_line_family_helpers_handle_nonkeyword_labels() {
        let statement = single_preprocessed_line("Battle Plan — Each player discards a card.");
        let static_line = single_preprocessed_line("Mystic Aura — Enchanted creature gets +1/+1.");

        assert!(looks_like_statement_line_lexed(&statement));
        assert!(looks_like_static_line_lexed(&static_line));
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
    fn triggered_split_probe_preserves_failed_effect_parse_details() {
        let line = single_preprocessed_line(
            "Whenever this creature attacks, search your library for artifact card named.",
        );
        let comma_idx = line
            .tokens
            .iter()
            .enumerate()
            .find_map(|(idx, token)| token.is_comma().then_some(idx))
            .expect("expected triggered probe line to contain a comma");
        let probe = probe_triggered_split(
            &line.tokens[1..comma_idx],
            &line.tokens[comma_idx + 1..],
            None,
        );
        let fallback = probe.fallback_cst(&line, &line.tokens);

        match probe {
            TriggeredSplitProbe::Unsupported {
                trigger_error,
                effect_error,
                ..
            } => {
                assert!(trigger_error.is_none());
                assert!(effect_error.is_some());
                assert!(fallback.is_some());
            }
            other => panic!("expected unsupported triggered split probe, got {other:?}"),
        }
    }

    #[test]
    fn triggered_conditional_split_preserves_full_text_for_lowering() {
        let line = single_preprocessed_line(
            "At the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. Put that card into your hand and the rest on the bottom of your library in a random order.",
        );

        let parsed =
            parse_triggered_line_cst(&line).expect("expected triggered conditional line to parse");

        assert_eq!(
            parsed.full_text,
            "at the beginning of your second main phase, if this creature is tapped, reveal cards from the top of your library until you reveal a land card. put that card into your hand and the rest on the bottom of your library in a random order."
        );
        assert_eq!(
            parsed.trigger_text,
            "the beginning of your second main phase"
        );
        assert_eq!(
            parsed.effect_text,
            "reveal cards from the top of your library until you reveal a land card. put that card into your hand and the rest on the bottom of your library in a random order."
        );
        assert_eq!(
            render_token_slice(&parsed.full_parse_tokens),
            parsed.full_text
        );
        assert_eq!(
            render_token_slice(&parsed.trigger_parse_tokens),
            parsed.trigger_text
        );
        assert_eq!(
            render_token_slice(&parsed.effect_parse_tokens),
            parsed.effect_text
        );
    }

    #[test]
    fn colon_nonactivation_statement_fallback_reuses_split_token_slice() {
        let line = single_preprocessed_line("Reveal this card from your hand: Draw a card.");

        let parsed = parse_colon_nonactivation_statement_fallback(&line)
            .expect("expected fallback parse to succeed")
            .expect("expected reveal-prefix fallback to produce a statement");

        assert_eq!(parsed.text, "reveal this card from your hand");
        assert_eq!(
            parsed.info.normalized.normalized,
            "reveal this card from your hand"
        );
    }

    #[test]
    fn trigger_sentence_chunk_splitter_reuses_token_ranges() {
        let tokens = lex_line(
            "Whenever this creature attacks, draw a card. Whenever it deals combat damage to a player, create a Treasure token.",
            0,
        )
        .expect("rewrite lexer should classify trigger chunk line");

        let chunks = split_trigger_sentence_chunks_rewrite_lexed(&tokens)
            .into_iter()
            .map(|chunk| render_token_slice(&chunk))
            .collect::<Vec<_>>();

        assert_eq!(
            chunks,
            vec![
                "Whenever this creature attacks, draw a card".to_string(),
                "Whenever it deals combat damage to a player, create a Treasure token".to_string(),
            ]
        );
    }

    #[test]
    fn reveal_first_draw_splitter_reuses_token_ranges() {
        let tokens = lex_line(
            "Reveal the first card you draw each turn. Whenever you reveal an instant card this way, draw a card.",
            0,
        )
        .expect("rewrite lexer should classify reveal-first-draw line");

        let chunks = split_reveal_first_draw_line_rewrite_lexed(&tokens)
            .expect("expected reveal-first-draw splitter to match")
            .into_iter()
            .map(|chunk| render_token_slice(&chunk))
            .collect::<Vec<_>>();

        assert_eq!(
            chunks,
            vec![
                "Reveal the first card you draw each turn".to_string(),
                "Whenever you reveal an instant card this way, draw a card".to_string(),
            ]
        );
    }

    #[test]
    fn trailing_keyword_activation_splitter_reuses_token_ranges() {
        let tokens = lex_line(
            "draw a card. cycling — {2}, discard this card: draw a card.",
            0,
        )
        .expect("rewrite lexer should classify trailing keyword activation line");

        let (prefix, suffix) = split_trailing_keyword_activation_sentence_lexed(&tokens)
            .expect("expected trailing keyword activation split");

        assert_eq!(render_token_slice(&prefix), "draw a card");
        assert_eq!(
            render_token_slice(&suffix),
            "cycling — {2}, discard this card: draw a card"
        );
    }

    #[test]
    fn activation_text_parts_lexed_reuse_existing_token_split() {
        let tokens = lex_line("{2}, discard this card: draw a card.", 0)
            .expect("rewrite lexer should classify activation text");

        let (cost_tokens, effect_text) =
            split_activation_text_parts_lexed(&tokens).expect("expected activation text split");

        assert_eq!(render_token_slice(&cost_tokens), "{2}, discard this card");
        assert_eq!(effect_text, "draw a card.");
    }

    #[test]
    fn activated_line_cst_stores_cost_and_effect_parse_tokens() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Activated Parse Tokens Test")
                .card_types(vec![CardType::Artifact]),
            "{T}: Draw a card.",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;

        match cst.lines.as_slice() {
            [super::RewriteLineCst::Activated(activated)] => {
                assert_eq!(render_token_slice(&activated.cost_parse_tokens), "{t}");
                assert_eq!(activated.effect_text, "draw a card.");
                assert_eq!(
                    render_token_slice(&activated.effect_parse_tokens),
                    activated.effect_text
                );
            }
            other => panic!("expected one activated line, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn reveal_first_draw_line_family_parses_through_document_cst() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Reveal First Draw Split Test")
                .card_types(vec![CardType::Enchantment]),
            "Reveal the first card you draw each turn. Whenever you reveal an instant card this way, draw a card.",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;

        match cst.lines.as_slice() {
            [
                super::RewriteLineCst::Static(static_line),
                super::RewriteLineCst::Triggered(triggered),
            ] => {
                assert_eq!(static_line.text, "reveal the first card you draw each turn");
                assert_eq!(
                    triggered.trigger_text,
                    "you reveal an instant card this way"
                );
                assert_eq!(triggered.effect_text, "draw a card");
            }
            other => {
                panic!("expected static plus triggered reveal-first-draw split, got {other:?}")
            }
        }

        Ok(())
    }

    #[test]
    fn modal_mode_cst_stores_stripped_parse_tokens() -> Result<(), CardTextError> {
        let preprocessed = preprocess_document(
            CardDefinitionBuilder::new(CardId::new(), "Modal Parse Tokens Test")
                .card_types(vec![CardType::Instant]),
            "Choose one —\n• Meteor Strikes — Draw a card.\n• Final Heaven — Gain 3 life.",
        )?;
        let cst = super::parse_document_cst(&preprocessed, false)?;

        match cst.lines.as_slice() {
            [super::RewriteLineCst::Modal(modal)] => {
                assert_eq!(modal.modes.len(), 2);
                assert_eq!(modal.modes[0].text, "Draw a card.");
                assert_eq!(modal.modes[1].text, "Gain 3 life.");
                assert_eq!(
                    render_token_slice(&modal.modes[0].parse_tokens),
                    "draw a card."
                );
                assert_eq!(
                    render_token_slice(&modal.modes[1].parse_tokens),
                    "gain 3 life."
                );
            }
            other => panic!("expected one modal block, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn unsupported_line_diagnostics_use_token_normalization() {
        let landwalk = single_preprocessed_line(
            "Creatures with islandwalk can be blocked as though they didn’t have islandwalk.",
        );
        let aura_copy = single_preprocessed_line(
            "Create a token that’s a copy of that Aura attached to that creature.",
        );
        let face_down = single_preprocessed_line("Target face-down creature can block this turn.");

        let landwalk_error = diagnose_known_unsupported_rewrite_line(&landwalk.tokens)
            .expect("expected landwalk override diagnostic");
        let aura_copy_error = diagnose_known_unsupported_rewrite_line(&aura_copy.tokens)
            .expect("expected aura-copy diagnostic");
        let face_down_error = diagnose_known_unsupported_rewrite_line(&face_down.tokens)
            .expect("expected face-down diagnostic");

        assert_eq!(
            landwalk_error.to_string(),
            "unsupported landwalk override clause"
        );
        assert_eq!(
            aura_copy_error.to_string(),
            "unsupported aura-copy attachment fanout clause"
        );
        assert_eq!(face_down_error.to_string(), "unsupported face-down clause");
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

        let your_step_do_not = lex_line(
            "Artifacts you control do not untap during your untap step.",
            0,
        )
        .expect("rewrite lexer should classify do-not untap-step probe");
        assert!(is_doesnt_untap_during_your_untap_step_tokens(
            &your_step_do_not
        ));

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

fn rewrite_line_tokens(line: &PreprocessedLine, tokens: &[OwnedLexToken]) -> PreprocessedLine {
    let normalized = render_token_slice(tokens);
    let mut rewritten = line.clone();
    rewritten.info.normalized.original = normalized.clone();
    rewritten.info.normalized.normalized = normalized.clone();
    rewritten.info.normalized.char_map = (0..normalized.len()).collect();
    rewritten.tokens = tokens.to_vec();
    rewritten
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

fn split_trigger_sentence_chunks_rewrite_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<Vec<OwnedLexToken>> {
    let sentence_tokens = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentence_tokens.len() <= 1 {
        return sentence_tokens
            .into_iter()
            .map(|sentence| sentence.to_vec())
            .collect();
    }

    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_starts_with_trigger = false;

    for sentence_tokens in sentence_tokens {
        let sentence_starts_with_trigger = line_starts_with_trigger_intro_tokens(sentence_tokens);
        if !current.is_empty() && current_starts_with_trigger && sentence_starts_with_trigger {
            if let Some(chunk) = clone_sentence_chunk_tokens(tokens, &current) {
                chunks.push(chunk);
            }
            current.clear();
            current_starts_with_trigger = false;
        }
        if current.is_empty() {
            current_starts_with_trigger = sentence_starts_with_trigger;
        }
        current.push(sentence_tokens);
    }

    if !current.is_empty() {
        if let Some(chunk) = clone_sentence_chunk_tokens(tokens, &current) {
            chunks.push(chunk);
        }
    }

    chunks
}

fn split_reveal_first_draw_line_rewrite_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<Vec<OwnedLexToken>>> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentences.len() <= 1 {
        return None;
    }

    let first_tokens = *sentences.first()?;
    let first_is_reveal_first_draw = token_words_match_any(
        first_tokens,
        &[
            &[
                "reveal", "the", "first", "card", "you", "draw", "each", "turn",
            ],
            &[
                "reveal", "the", "first", "card", "you", "draw", "on", "each", "of", "your",
                "turns",
            ],
            &[
                "you", "may", "reveal", "the", "first", "card", "you", "draw", "each", "turn",
                "as", "you", "draw", "it",
            ],
            &[
                "you", "may", "reveal", "the", "first", "card", "you", "draw", "on", "each", "of",
                "your", "turns", "as", "you", "draw", "it",
            ],
        ],
    );
    if !first_is_reveal_first_draw {
        return None;
    }

    let tail_tokens = clone_sentence_chunk_tokens(tokens, &sentences[1..])?;
    if !token_words_have_prefix(&tail_tokens, &["whenever", "you", "reveal"]) {
        return None;
    }

    Some(vec![first_tokens.to_vec(), tail_tokens])
}

fn classify_unsupported_line_reason(line: &PreprocessedLine) -> &'static str {
    let classification_tokens = tokens_after_non_keyword_label_prefix(line).unwrap_or(&line.tokens);

    if is_bullet_line(line.info.raw_line.as_str()) {
        return "bullet-line-without-modal-header";
    }
    if line_starts_with_trigger_intro_tokens(&line.tokens) {
        return "triggered-line-not-yet-supported";
    }
    if split_lexed_once_on_colon_outside_quotes(&line.tokens).is_some() {
        return "activated-line-not-yet-supported";
    }
    if token_words_have_prefix(classification_tokens, &["choose"]) {
        return "modal-header-not-yet-supported";
    }
    if looks_like_statement_line_lexed(line) {
        return "statement-line-not-yet-supported";
    }
    if looks_like_static_line_lexed(line) {
        return "static-line-not-yet-supported";
    }
    "unclassified-line-family"
}

fn is_exile_then_owner_may_play_costs_more_statement_line(tokens: &[OwnedLexToken]) -> bool {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    matches!(
        sentences.as_slice(),
        [first, second, third]
            if token_words_match(first, &["exile", "target", "nonland", "permanent"])
                && token_words_match(
                    second,
                    &[
                        "for", "as", "long", "as", "that", "card", "remains", "exiled", "its",
                        "owner", "may", "play", "it",
                    ],
                )
                && token_words_match(
                    third,
                    &[
                        "a", "spell", "cast", "by", "an", "opponent", "this", "way", "costs",
                        "2", "more", "to", "cast",
                    ],
                )
    )
}

struct UnsupportedWordRule {
    phrase: &'static [&'static str],
    message: &'static str,
}

const UNSUPPORTED_STARTS_WITH_RULES: &[UnsupportedWordRule] = &[
    UnsupportedWordRule {
        phrase: &["partner", "with"],
        message: "unsupported partner-with keyword line [rule=partner-with-keyword-line]",
    },
    UnsupportedWordRule {
        phrase: &[
            "the", "first", "creature", "spell", "you", "cast", "each", "turn", "costs",
        ],
        message: "unsupported first-spell cost modifier mechanic",
    },
    UnsupportedWordRule {
        phrase: &[
            "once", "each", "turn", "you", "may", "play", "a", "card", "from", "exile",
        ],
        message: "unsupported static clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "prevent", "the", "next", "1", "damage", "that", "would", "be", "dealt", "to", "any",
            "target", "this", "turn", "by", "red", "sources",
        ],
        message: "unsupported trailing prevent-next damage clause",
    },
    UnsupportedWordRule {
        phrase: &["ninjutsu", "abilities", "you", "activate", "cost"],
        message: "unsupported marker keyword with non-keyword tail",
    },
];

const UNSUPPORTED_CONTAINS_RULES: &[UnsupportedWordRule] = &[
    UnsupportedWordRule {
        phrase: &[
            "same", "name", "as", "another", "card", "in", "their", "hand",
        ],
        message: "unsupported same-name-as-another-in-hand discard clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "enters", "tapped", "and", "doesnt", "untap", "during", "your", "untap", "step",
        ],
        message: "unsupported mixed enters-tapped and negated-untap clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "prevent",
            "all",
            "combat",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "this",
            "turn",
            "by",
            "creatures",
            "with",
            "power",
        ],
        message: "unsupported prevent-all-combat-damage clause tail",
    },
    UnsupportedWordRule {
        phrase: &[
            "put",
            "one",
            "of",
            "them",
            "into",
            "your",
            "hand",
            "and",
            "the",
            "rest",
            "into",
            "your",
            "graveyard",
        ],
        message: "unsupported multi-destination put clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "assigns",
            "no",
            "combat",
            "damage",
            "this",
            "turn",
            "and",
            "defending",
            "player",
            "loses",
        ],
        message: "unsupported assigns-no-combat-damage clause",
    },
    UnsupportedWordRule {
        phrase: &["of", "defending", "players", "choice"],
        message: "unsupported defending-players-choice clause",
    },
    UnsupportedWordRule {
        phrase: &["if", "you", "sacrifice", "an", "island", "this", "way"],
        message: "unsupported if-you-sacrifice-an-island-this-way clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "create", "a", "token", "thats", "a", "copy", "of", "that", "aura", "attached", "to",
            "that", "creature",
        ],
        message: "unsupported aura-copy attachment fanout clause",
    },
    UnsupportedWordRule {
        phrase: &["target", "face", "down", "creature"],
        message: "unsupported face-down clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "with",
            "islandwalk",
            "can",
            "be",
            "blocked",
            "as",
            "though",
            "they",
            "didnt",
            "have",
            "islandwalk",
        ],
        message: "unsupported landwalk override clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "with",
            "power",
            "or",
            "toughness",
            "1",
            "or",
            "less",
            "cant",
            "be",
            "blocked",
        ],
        message: "unsupported power-or-toughness cant-be-blocked subject",
    },
    UnsupportedWordRule {
        phrase: &[
            "discard",
            "up",
            "to",
            "two",
            "permanents",
            "then",
            "draw",
            "that",
            "many",
            "cards",
        ],
        message: "unsupported discard qualifier clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "if", "your", "life", "total", "is", "less", "than", "or", "equal", "to", "half",
            "your", "starting", "life", "total", "plus", "one",
        ],
        message: "unsupported predicate",
    },
    UnsupportedWordRule {
        phrase: &[
            "then",
            "sacrifices",
            "all",
            "creatures",
            "they",
            "control",
            "then",
            "puts",
            "all",
            "cards",
            "they",
            "exiled",
            "this",
            "way",
            "onto",
            "the",
            "battlefield",
        ],
        message: "unsupported each-player exile/sacrifice/return-this-way clause",
    },
    UnsupportedWordRule {
        phrase: &["if", "this", "creature", "isnt", "saddled", "this", "turn"],
        message: "unsupported saddled conditional tail",
    },
    UnsupportedWordRule {
        phrase: &[
            "put", "a", "card", "from", "among", "them", "into", "your", "hand", "this", "turn",
        ],
        message: "unsupported looked-card fallback tail",
    },
    UnsupportedWordRule {
        phrase: &[
            "if",
            "the",
            "sacrificed",
            "creature",
            "was",
            "a",
            "hamster",
            "this",
            "turn",
        ],
        message: "unsupported predicate",
    },
];

const UNSUPPORTED_EQUALS_RULES: &[UnsupportedWordRule] = &[
    UnsupportedWordRule {
        phrase: &[
            "creatures",
            "you",
            "control",
            "have",
            "haste",
            "and",
            "attack",
            "each",
            "combat",
            "if",
            "able",
        ],
        message: "unsupported anthem subject",
    },
    UnsupportedWordRule {
        phrase: &[
            "you", "may", "play", "any", "number", "of", "lands", "on", "each", "of", "your",
            "turns",
        ],
        message: "unsupported additional-land-play permission clause",
    },
    UnsupportedWordRule {
        phrase: &[
            "target",
            "creature",
            "can",
            "block",
            "any",
            "number",
            "of",
            "creatures",
            "this",
            "turn",
        ],
        message: "unsupported target-only restriction clause",
    },
    UnsupportedWordRule {
        phrase: &["equip", "costs", "you", "pay", "cost", "1", "less"],
        message: "unsupported activation cost modifier clause",
    },
    UnsupportedWordRule {
        phrase: &["unleash", "while"],
        message: "unsupported line",
    },
];

struct UnsupportedRewriteLineContext {
    words: Vec<String>,
}

impl UnsupportedRewriteLineContext {
    fn new(tokens: &[OwnedLexToken]) -> Self {
        Self {
            words: TokenWordView::new(tokens).owned_words(),
        }
    }

    fn has_prefix(&self, expected: &[&str]) -> bool {
        self.words.len() >= expected.len()
            && self
                .words
                .iter()
                .take(expected.len())
                .map(String::as_str)
                .zip(expected.iter().copied())
                .all(|(actual, expected)| actual == expected)
    }

    fn contains_phrase(&self, expected: &[&str]) -> bool {
        self.phrase_count(expected) > 0
    }

    fn phrase_count(&self, expected: &[&str]) -> usize {
        if expected.is_empty() || self.words.len() < expected.len() {
            return 0;
        }

        let mut count = 0usize;
        let last_start = self.words.len() - expected.len();
        let mut start = 0usize;
        while start <= last_start {
            let matches = self.words[start..start + expected.len()]
                .iter()
                .map(String::as_str)
                .zip(expected.iter().copied())
                .all(|(actual, expected)| actual == expected);
            if matches {
                count += 1;
            }
            start += 1;
        }
        count
    }

    fn equals_words(&self, expected: &[&str]) -> bool {
        self.words.len() == expected.len() && self.has_prefix(expected)
    }

    fn contains_word(&self, expected: &str) -> bool {
        self.words.iter().any(|word| word == expected)
    }

    fn first_word(&self) -> Option<&str> {
        self.words.first().map(String::as_str)
    }
}

fn diagnose_known_unsupported_rewrite_line(tokens: &[OwnedLexToken]) -> Option<CardTextError> {
    let ctx = UnsupportedRewriteLineContext::new(tokens);

    for rule in UNSUPPORTED_STARTS_WITH_RULES {
        if ctx.has_prefix(rule.phrase) {
            return Some(CardTextError::ParseError(rule.message.to_string()));
        }
    }

    for rule in UNSUPPORTED_EQUALS_RULES {
        if ctx.equals_words(rule.phrase) {
            return Some(CardTextError::ParseError(rule.message.to_string()));
        }
    }

    for rule in UNSUPPORTED_CONTAINS_RULES {
        if ctx.contains_phrase(rule.phrase) {
            return Some(CardTextError::ParseError(rule.message.to_string()));
        }
    }

    let message = if ctx.has_prefix(&["choose", "target", "land"])
        && ctx.contains_phrase(&[
            "create", "three", "tokens", "that", "are", "copies", "of", "it",
        ]) {
        "unsupported choose-leading spell clause"
    } else if ctx.contains_phrase(&["loses", "all", "abilities", "and", "becomes"]) {
        if ctx.has_prefix(&["until", "end", "of", "turn"]) {
            "unsupported loses-all-abilities with becomes clause"
        } else {
            "unsupported lose-all-abilities static becomes clause"
        }
    } else if ctx.phrase_count(&["spent", "to", "cast", "this", "spell"]) >= 2
        && ctx.contains_word("if")
        && !matches!(ctx.first_word(), Some("if" | "unless" | "when" | "as"))
    {
        "unsupported spent-to-cast conditional clause"
    } else if ctx.contains_phrase(&["for", "each", "odd", "result"])
        && ctx.contains_phrase(&["for", "each", "even", "result"])
    {
        "unsupported odd-or-even die-result clause"
    } else if ctx.contains_phrase(&[
        "for", "as", "long", "as", "that", "card", "remains", "exiled", "its", "owner", "may",
        "play", "it",
    ]) && !ctx.contains_phrase(&[
        "a", "spell", "cast", "by", "an", "opponent", "this", "way", "costs",
    ]) && !ctx.contains_phrase(&["a", "spell", "cast", "this", "way", "costs"])
    {
        "unsupported for-as-long-as play/cast permission clause"
    } else if ctx.contains_phrase(&[
        "each",
        "player",
        "loses",
        "x",
        "life",
        "discards",
        "x",
        "cards",
        "sacrifices",
        "x",
        "creatures",
    ]) && ctx.contains_phrase(&["then", "sacrifices", "x", "lands"])
    {
        "unsupported multi-step each-player clause with 'then'"
    } else {
        return None;
    };

    Some(CardTextError::ParseError(message.to_string()))
}

fn parse_colon_nonactivation_statement_fallback(
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

#[cfg(test)]
fn split_activation_text_parts_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, String)> {
    let (cost_tokens, effect_tokens) = split_activation_text_tokens_lexed(tokens)?;
    Some((
        cost_tokens,
        render_token_slice(&effect_tokens).trim().to_string(),
    ))
}

fn split_activation_text_tokens_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let Some((cost_tokens, effect_tokens)) = split_lexed_once_on_colon_outside_quotes(tokens)
    else {
        return None;
    };

    let cost_tokens = trim_lexed_commas(cost_tokens);
    let effect_tokens = trim_lexed_commas(effect_tokens);
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return None;
    }

    Some((cost_tokens.to_vec(), effect_tokens.to_vec()))
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
                    let parse_tokens = lexed_tokens(text, line.info.line_index)?;
                    lines.push(RewriteLineCst::SagaChapter(SagaChapterLineCst {
                        info: line.info.clone(),
                        chapters,
                        text: text.to_string(),
                        parse_tokens,
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
                    let parse_tokens = strip_non_keyword_label_prefix_lexed(&next_line.tokens);
                    let mode_text = strip_non_keyword_label_prefix(raw_mode).trim().to_string();
                    bullet_modes.push(ModalModeCst {
                        info: next_line.info.clone(),
                        text: mode_text,
                        parse_tokens: parse_tokens.to_vec(),
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
                    for chunk_tokens in chunks {
                        let chunk_line = rewrite_line_tokens(line, &chunk_tokens);
                        if line_starts_with_trigger_intro_tokens(&chunk_line.tokens) {
                            for trigger_chunk in
                                split_trigger_sentence_chunks_rewrite_lexed(&chunk_line.tokens)
                            {
                                let trigger_line = rewrite_line_tokens(&chunk_line, &trigger_chunk);
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
                if let Some((prefix_tokens, suffix_tokens)) =
                    split_trailing_keyword_activation_sentence_lexed(&line.tokens)
                {
                    let prefix_line = rewrite_line_tokens(line, &prefix_tokens);
                    if let Some(statement_line) = parse_statement_line_cst(&prefix_line)? {
                        lines.push(RewriteLineCst::Statement(statement_line));
                    } else if let Some(rewritten_prefix) = rewrite_named_source_sentence_for_builder(
                        &preprocessed.builder,
                        prefix_line.info.normalized.normalized.as_str(),
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

                    let suffix_line = rewrite_line_tokens(line, &suffix_tokens);
                    let Some((_label, body_tokens)) = split_label_prefix_lexed(&suffix_line.tokens)
                    else {
                        return Err(CardTextError::ParseError(format!(
                            "parser could not recover keyword activation suffix: '{}'",
                            line.info.raw_line
                        )));
                    };
                    let Some((cost_tokens, effect_parse_tokens)) =
                        split_activation_text_tokens_lexed(body_tokens)
                    else {
                        return Err(CardTextError::ParseError(format!(
                            "parser could not recover activation suffix: '{}'",
                            line.info.raw_line
                        )));
                    };
                    let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
                    let cost = parse_activation_cost_tokens_rewrite(&cost_tokens)?;
                    lines.push(RewriteLineCst::Activated(ActivatedLineCst {
                        info: suffix_line.info.clone(),
                        cost,
                        cost_parse_tokens: cost_tokens,
                        effect_text,
                        effect_parse_tokens,
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
                    && let Some(err) = diagnose_known_unsupported_rewrite_line(&line.tokens)
                {
                    return Err(err);
                }

                if let Some((label, body_tokens)) = split_label_prefix_lexed(&line.tokens) {
                    let is_named_label = is_named_ability_label(label.as_str());
                    let preserve_as_choice_label =
                        labeled_choice_block_has_peer(&preprocessed.items, idx);
                    if !preserve_keyword_prefix_for_parse(label.as_str()) {
                        let body_line = rewrite_line_tokens(line, body_tokens);
                        let labeled_activation = if (!str_starts_with_char(
                            line.info.raw_line.trim_start(),
                            '(',
                        ) || is_fully_parenthetical_line(line.info.raw_line.as_str()))
                            && let Some((cost_tokens, effect_parse_tokens)) =
                                split_activation_text_tokens_lexed(&body_line.tokens)
                        {
                            let cost_text = render_token_slice(&cost_tokens);
                            let effect_text =
                                render_token_slice(&effect_parse_tokens).trim().to_string();
                            Some((cost_tokens, effect_parse_tokens, cost_text, effect_text))
                        } else {
                            None
                        };
                        let prefer_activation = labeled_activation
                            .as_ref()
                            .is_some_and(|(_, _, cost_text, _)| {
                                looks_like_activation_cost_prefix(cost_text.as_str())
                            });
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
                                    body_line.info.normalized.normalized.as_str(),
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
                                            body_line.info.normalized.normalized.as_str()
                                        ))
                                    }));
                            }
                        }

                        if prefer_activation
                            && let Some((cost_tokens, effect_parse_tokens, cost_text, effect_text)) =
                                labeled_activation.clone()
                        {
                            match parse_activation_cost_tokens_rewrite(&cost_tokens) {
                                Ok(cost) => {
                                    lines.push(RewriteLineCst::Activated(ActivatedLineCst {
                                        info: line.info.clone(),
                                        cost,
                                        cost_parse_tokens: cost_tokens,
                                        effect_text,
                                        effect_parse_tokens,
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

                        if is_named_label
                            && let Some(keyword_line) = parse_keyword_line_cst(&body_line)?
                        {
                            lines.push(RewriteLineCst::Keyword(keyword_line));
                            idx += 1;
                            continue;
                        }

                        if let Some(mut static_line) = parse_static_line_cst(&body_line)? {
                            if preserve_as_choice_label {
                                static_line.chosen_option_label = Some(label.to_ascii_lowercase());
                            }
                            lines.push(RewriteLineCst::Static(static_line));
                            idx += 1;
                            continue;
                        }

                        if let Some(rewritten_body) = rewrite_named_source_sentence_for_builder(
                            &preprocessed.builder,
                            body_line.info.normalized.normalized.as_str(),
                        ) {
                            let rewritten_body_line =
                                rewrite_line_normalized(line, rewritten_body.as_str())?;
                            if let Some(mut static_line) =
                                parse_static_line_cst(&rewritten_body_line)?
                            {
                                if preserve_as_choice_label {
                                    static_line.chosen_option_label =
                                        Some(label.to_ascii_lowercase());
                                }
                                lines.push(RewriteLineCst::Static(static_line));
                                idx += 1;
                                continue;
                            }
                        }

                        if let Some((cost_tokens, effect_parse_tokens, cost_text, effect_text)) =
                            labeled_activation
                        {
                            match parse_activation_cost_tokens_rewrite(&cost_tokens) {
                                Ok(cost) => {
                                    lines.push(RewriteLineCst::Activated(ActivatedLineCst {
                                        info: line.info.clone(),
                                        cost,
                                        cost_parse_tokens: cost_tokens,
                                        effect_text,
                                        effect_parse_tokens,
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
                        for chunk_tokens in trigger_chunks {
                            let chunk_line = rewrite_line_tokens(line, &chunk_tokens);
                            match parse_triggered_line_cst(&chunk_line) {
                                Ok(triggered) => lines.push(RewriteLineCst::Triggered(triggered)),
                                Err(_) => {
                                    if let Some(triggered) =
                                        try_parse_triggered_line_with_named_source_rewrite(
                                            &preprocessed.builder,
                                            line,
                                            chunk_line.info.normalized.normalized.as_str(),
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
                                        return Err(strict_unsupported_triggered_line_error(
                                            chunk_line.info.normalized.normalized.as_str(),
                                            parse_triggered_line_cst(&chunk_line).err(),
                                        ));
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
                            return Err(strict_unsupported_triggered_line_error(
                                &line.info.raw_line,
                                parse_triggered_line_cst(line).err(),
                            ));
                        }
                    }
                }

                if let Some(keyword_line) = parse_keyword_line_cst(line)? {
                    lines.push(RewriteLineCst::Keyword(keyword_line));
                    idx += 1;
                    continue;
                }

                if is_ward_or_echo_static_prefix_tokens(&line.tokens) {
                    lines.push(RewriteLineCst::Static(StaticLineCst {
                        info: line.info.clone(),
                        text: normalized.to_string(),
                        parse_tokens: rewrite_keyword_dash_parse_tokens(&line.tokens),
                        chosen_option_label: None,
                    }));
                    idx += 1;
                    continue;
                }

                if (!str_starts_with_char(line.info.raw_line.trim_start(), '(')
                    || is_fully_parenthetical_line(line.info.raw_line.as_str()))
                    && let Some((cost_tokens, effect_parse_tokens)) =
                        split_label_prefix_lexed(&line.tokens)
                            .filter(|(label, _)| is_named_ability_label(label.as_str()))
                            .and_then(|(_, body_tokens)| {
                                split_activation_text_tokens_lexed(body_tokens)
                            })
                            .or_else(|| split_activation_text_tokens_lexed(&line.tokens))
                {
                    let cost_text = render_token_slice(&cost_tokens);
                    let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
                    match parse_activation_cost_tokens_rewrite(&cost_tokens) {
                        Ok(cost) => {
                            lines.push(RewriteLineCst::Activated(ActivatedLineCst {
                                info: line.info.clone(),
                                cost,
                                cost_parse_tokens: cost_tokens,
                                effect_text,
                                effect_parse_tokens,
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
                    if should_try_combined_static_tokens(&line.tokens, &next_line.tokens) {
                        let combined_text = format!(
                            "{}. {}",
                            normalized.trim_end_matches('.'),
                            next_line.info.normalized.normalized.trim_end_matches('.')
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
                    || looks_like_statement_line_lexed(line)
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

                if let Some(statement_line) = parse_colon_nonactivation_statement_fallback(line)? {
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
                let parsed = lower_rewrite_keyword_to_chunk(
                    keyword.info.clone(),
                    &keyword.text,
                    &keyword.parse_tokens,
                    kind,
                )?;
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
                    activated.cost_parse_tokens.clone(),
                    activated.effect_text.clone(),
                    activated.effect_parse_tokens.clone(),
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
                    &triggered.full_parse_tokens,
                    &triggered.trigger_text,
                    &triggered.trigger_parse_tokens,
                    &triggered.effect_text,
                    &triggered.effect_parse_tokens,
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
                        &static_line.parse_tokens,
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
                let parsed_chunks = lower_rewrite_statement_token_groups_to_chunks(
                    statement_line.info.clone(),
                    &statement_line.text,
                    &statement_line.parse_tokens,
                    &statement_line.parse_groups,
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
                            let effects_ast = parse_effect_sentences_lexed(&mode.parse_tokens)?;
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
                                    let actions = parse_ability_line_lexed(&item.parse_tokens)
                                        .ok_or_else(|| {
                                        CardTextError::ParseError(format!(
                                            "rewrite level lowering could not parse keyword line '{}'",
                                            item.info.raw_line
                                        ))
                                    })?;
                                    ParsedLevelAbilityItemAst::KeywordActions(actions)
                                }
                                LevelItemKindCst::StaticAbilities => {
                                    let abilities =
                                        parse_static_ability_ast_line_lexed(&item.parse_tokens)?
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
                let effects_ast = parse_effect_sentences_lexed(&saga.parse_tokens)?;
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
