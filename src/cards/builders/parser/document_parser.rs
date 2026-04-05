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
use winnow::Parser;
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
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

    head_tokens.iter().enumerate().any(|(idx, _)| {
        grammar::parse_prefix(
            &head_tokens[idx..],
            winnow::combinator::alt((
                grammar::kw("don't").void(),
                grammar::kw("dont").void(),
                grammar::kw("doesn't").void(),
                grammar::kw("doesnt").void(),
                (grammar::kw("do"), grammar::kw("not")).void(),
                (grammar::kw("does"), grammar::kw("not")).void(),
            )),
        )
        .is_some()
    })
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

#[derive(Debug, Clone)]
struct TriggeredSplitCandidate {
    trigger_text: String,
    effect_text: String,
    max_triggers_per_turn: Option<u32>,
}

impl TriggeredSplitCandidate {
    fn into_cst(self, line: &PreprocessedLine, intro_token: &OwnedLexToken) -> TriggeredLineCst {
        TriggeredLineCst {
            info: line.info.clone(),
            full_text: format!(
                "{} {}, {}",
                intro_token.slice.as_str(),
                self.trigger_text,
                self.effect_text
            ),
            trigger_text: self.trigger_text,
            effect_text: self.effect_text,
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
        intro_token: &OwnedLexToken,
    ) -> Option<TriggeredLineCst> {
        match self {
            Self::Supported(candidate) => Some(candidate.clone().into_cst(line, intro_token)),
            _ => None,
        }
    }

    fn fallback_cst(
        &self,
        line: &PreprocessedLine,
        intro_token: &OwnedLexToken,
    ) -> Option<TriggeredLineCst> {
        match self {
            Self::Supported(candidate) => Some(candidate.clone().into_cst(line, intro_token)),
            Self::Unsupported { candidate, .. } => {
                Some(candidate.clone().into_cst(line, intro_token))
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
        effect_text,
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
    if let Some(err) = diagnose_known_unsupported_rewrite_line(tokens_without_cap) {
        return Err(err);
    }

    if let Some(nested_trigger_tokens) = nested_combat_whenever_clause_tokens(tokens_without_cap) {
        let nested_text = render_token_slice(nested_trigger_tokens);
        let nested_line = rewrite_line_normalized(line, nested_text.as_str())?;
        if let Ok(parsed) = parse_triggered_line_cst(&nested_line) {
            return Ok(parsed);
        }
    }

    let mut best_probe_error = None;

    if let Some(spec) =
        super::grammar::structure::split_triggered_conditional_clause_lexed(tokens_without_cap, 1)
    {
        let probe = probe_triggered_split(spec.trigger_tokens, spec.effects_tokens, trailing_cap);
        if let Some(mut parsed) = probe.supported_cst(line, first_token) {
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
            if let Some(parsed) = probe.supported_cst(line, first_token) {
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

        if let Some(parsed) = probe.supported_cst(line, first_token) {
            if best_supported_split.is_none() {
                best_supported_split = Some(parsed);
            }
            continue;
        }

        if best_probe_error.is_none() {
            best_probe_error = probe.preferred_error();
        }

        if whole_line_parse.is_ok() && best_fallback_split.is_none() {
            best_fallback_split = probe.fallback_cst(line, first_token);
        }
    }

    if let Some(split) = best_supported_split.or(best_fallback_split) {
        return Ok(split);
    }

    match whole_line_parse {
        Ok(_) => Ok(TriggeredLineCst {
            info: line.info.clone(),
            full_text: normalized.to_string(),
            trigger_text: render_token_slice(condition_tokens).trim().to_string(),
            effect_text: String::new(),
            max_triggers_per_turn: trailing_cap,
            chosen_option_label: None,
        }),
        Err(err) => Err(best_probe_error.unwrap_or(err)),
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
    let rewritten_parse_text = rewrite_keyword_dash_parse_text(normalized);
    let tokens = lexed_tokens(rewritten_parse_text.as_str(), line.info.line_index)?;

    let kind = match parse_keyword_dispatch_hint(&tokens) {
        Some(KeywordDispatchHint::AdditionalCostFamily) => {
            if parse_additional_cost_kind(&tokens, normalized)? {
                Some(KeywordLineKindCst::AdditionalCostChoice)
            } else if additional_cost_tail_tokens(&tokens).is_some() {
                Some(KeywordLineKindCst::AdditionalCost)
            } else {
                None
            }
        }
        Some(KeywordDispatchHint::AlternativeOrExertFamily) => {
            if parse_alternative_cast_kind(&tokens, normalized)? {
                Some(KeywordLineKindCst::AlternativeCast)
            } else if is_exert_attack_keyword_line(normalized) {
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
            if str_starts_with(normalized, "morph—") || str_starts_with(normalized, "megamorph—")
            {
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
            if looks_like_statement_line_lexed(line)
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

fn tokens_after_non_keyword_label_prefix(line: &PreprocessedLine) -> Option<Vec<OwnedLexToken>> {
    let normalized = line.info.normalized.normalized.as_str();
    let (label, body) = split_label_prefix(normalized)?;
    if preserve_keyword_prefix_for_parse(label) {
        return None;
    }
    lex_line(body, line.info.line_index).ok()
}

fn looks_like_statement_line_tokens(tokens: &[OwnedLexToken]) -> bool {
    if looks_like_untap_all_during_each_other_players_untap_step_tokens(tokens) {
        return false;
    }
    looks_like_next_turn_cant_cast_line_tokens(tokens)
        || looks_like_vote_statement_line_tokens(tokens)
        || looks_like_generic_statement_line_tokens(tokens)
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

    Err(ErrMode::Backtrack(ContextError::new()))
}

pub(crate) fn split_lexed_once_on_colon_outside_quotes(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let (left_len, rest) =
        grammar::parse_prefix(tokens, parse_segment_len_until_colon_outside_quotes)?;
    let (_, right_tokens) = grammar::parse_prefix(rest, grammar::colon())?;
    Some((&tokens[..left_len], right_tokens))
}

fn text_has_colon_outside_quotes(text: &str) -> bool {
    let Ok(tokens) = lexed_tokens(text, 0) else {
        return false;
    };
    split_lexed_once_on_colon_outside_quotes(&tokens).is_some()
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
        if !preserve_keyword_prefix_for_parse(label) || !text_has_colon_outside_quotes(body) {
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
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::ids::CardId;
    use crate::types::CardType;

    use super::{
        PreprocessedItem, TriggeredSplitProbe, diagnose_known_unsupported_rewrite_line,
        is_doesnt_untap_during_your_untap_step_tokens, lex_line,
        looks_like_divvy_statement_line_tokens, looks_like_generic_statement_line_tokens,
        looks_like_generic_static_line_tokens, looks_like_next_turn_cant_cast_line_tokens,
        looks_like_pact_next_upkeep_line_tokens, looks_like_statement_line,
        looks_like_statement_line_lexed, looks_like_static_line, looks_like_static_line_lexed,
        looks_like_untap_all_during_each_other_players_untap_step_tokens,
        looks_like_vote_statement_line_tokens, parse_triggered_line_cst, preprocess_document,
        probe_triggered_split, split_label_prefix, strip_non_keyword_label_prefix,
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
        let first_token = line.tokens.first().expect("expected intro token");
        let probe = probe_triggered_split(
            &line.tokens[1..comma_idx],
            &line.tokens[comma_idx + 1..],
            None,
        );
        let fallback = probe.fallback_cst(&line, first_token);

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
        assert_eq!(parsed.trigger_text, "the beginning of your second main phase");
        assert_eq!(
            parsed.effect_text,
            "reveal cards from the top of your library until you reveal a land card. put that card into your hand and the rest on the bottom of your library in a random order."
        );
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
    if line_starts_with_trigger_intro_tokens(&line.tokens) {
        return "triggered-line-not-yet-supported";
    }
    if split_lexed_once_on_colon_outside_quotes(&line.tokens).is_some() {
        return "activated-line-not-yet-supported";
    }
    if str_starts_with(normalized, "choose ") {
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
    text: &str,
) -> Result<Option<StatementLineCst>, CardTextError> {
    let tokens = lexed_tokens(text, line.info.line_index)?;
    let Some((left_tokens, right_tokens)) = split_lexed_once_on_colon_outside_quotes(&tokens)
    else {
        return Ok(None);
    };

    let left = render_token_slice(left_tokens);
    let right = render_token_slice(right_tokens);
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
    let Some((cost_tokens, effect_tokens)) = split_lexed_once_on_colon_outside_quotes(&tokens)
    else {
        return Ok(None);
    };

    let cost_tokens = trim_lexed_commas(cost_tokens);
    let effect_tokens = trim_lexed_commas(effect_tokens);
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return Ok(None);
    }

    Ok(Some((
        cost_tokens.to_vec(),
        render_token_slice(effect_tokens).trim().to_string(),
    )))
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
                    && let Some(err) = diagnose_known_unsupported_rewrite_line(&line.tokens)
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
                                        return Err(strict_unsupported_triggered_line_error(
                                            chunk.as_str(),
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
