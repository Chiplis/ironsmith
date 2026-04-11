use crate::cards::builders::{CardTextError, LineAst};
use winnow::Parser;

use super::activation_and_restrictions::{
    parse_activation_cost, parse_channel_line_lexed, parse_cycling_line_lexed,
    parse_equip_line_lexed,
};
use super::clause_support::parse_effect_sentences_lexed;
use super::cst::{KeywordLineCst, KeywordLineKindCst};
use super::grammar::primitives::{self as grammar, TokenWordView};
use super::ir::RewriteKeywordLine;
use super::keyword_static::parse_if_this_spell_costs_less_to_cast_line_lexed;
use super::lexer::{OwnedLexToken, TokenKind, lex_line, render_token_slice, trim_lexed_commas};
use super::lower::{
    lower_exert_attack_keyword_line, lower_gift_keyword_line, lower_keyword_special_cases,
};
use super::preprocess::PreprocessedLine;
use super::token_primitives::{
    find_index as find_token_index, split_em_dash_label_prefix_tokens, str_contains,
    str_strip_suffix,
};
use super::util::{
    parse_additional_cost_choice_options_lexed, parse_bargain_line_lexed, parse_bestow_line_lexed,
    parse_buyback_line_lexed, parse_cast_this_spell_only_line_lexed, parse_entwine_line_lexed,
    parse_escape_line_lexed, parse_flashback_line_lexed, parse_harmonize_line_lexed,
    parse_if_conditional_alternative_cost_line_lexed, parse_kicker_line_lexed,
    parse_madness_line_lexed, parse_morph_keyword_line_lexed, parse_multikicker_line_lexed,
    parse_offspring_line_lexed, parse_reinforce_line_lexed,
    parse_self_free_cast_alternative_cost_line_lexed, parse_squad_line_lexed,
    parse_transmute_line_lexed, parse_warp_line_lexed,
    parse_you_may_rather_than_spell_cost_line_lexed, preserve_keyword_prefix_for_parse,
};

type KeywordRuleFn = fn(&PreprocessedLine, &[OwnedLexToken]) -> Result<bool, CardTextError>;
type KeywordLowerFn = fn(&RewriteKeywordLine, &[OwnedLexToken]) -> Result<LineAst, CardTextError>;

#[derive(Clone, Copy)]
struct KeywordLineRule {
    cst_kind: KeywordLineKindCst,
    hints: &'static [KeywordDispatchHint],
    matches: KeywordRuleFn,
    lower: KeywordLowerFn,
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

const KEYWORD_LINE_RULES: &[KeywordLineRule] = &[
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::AdditionalCostChoice,
        hints: &[KeywordDispatchHint::AdditionalCostFamily],
        matches: matches_additional_cost_choice,
        lower: lower_additional_cost_choice,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::AdditionalCost,
        hints: &[KeywordDispatchHint::AdditionalCostFamily],
        matches: matches_additional_cost,
        lower: lower_additional_cost,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::AlternativeCast,
        hints: &[KeywordDispatchHint::AlternativeOrExertFamily],
        matches: matches_alternative_cast,
        lower: lower_alternative_cast,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Bestow,
        hints: &[KeywordDispatchHint::Bestow],
        matches: matches_bestow,
        lower: lower_bestow,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Bargain,
        hints: &[KeywordDispatchHint::Bargain],
        matches: matches_bargain,
        lower: lower_bargain,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Buyback,
        hints: &[KeywordDispatchHint::Buyback],
        matches: matches_buyback,
        lower: lower_buyback,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Channel,
        hints: &[KeywordDispatchHint::Channel],
        matches: matches_channel,
        lower: lower_channel,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Cycling,
        hints: &[KeywordDispatchHint::Cycling],
        matches: matches_cycling,
        lower: lower_cycling,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Reinforce,
        hints: &[KeywordDispatchHint::Reinforce],
        matches: matches_reinforce,
        lower: lower_reinforce,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Equip,
        hints: &[KeywordDispatchHint::Equip],
        matches: matches_equip,
        lower: lower_equip,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Kicker,
        hints: &[KeywordDispatchHint::Kicker],
        matches: matches_kicker,
        lower: lower_kicker,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Flashback,
        hints: &[KeywordDispatchHint::Flashback],
        matches: matches_flashback,
        lower: lower_flashback,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Harmonize,
        hints: &[KeywordDispatchHint::Harmonize],
        matches: matches_harmonize,
        lower: lower_harmonize,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Multikicker,
        hints: &[KeywordDispatchHint::Multikicker],
        matches: matches_multikicker,
        lower: lower_multikicker,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Entwine,
        hints: &[KeywordDispatchHint::Entwine],
        matches: matches_entwine,
        lower: lower_entwine,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Offspring,
        hints: &[KeywordDispatchHint::Offspring],
        matches: matches_offspring,
        lower: lower_offspring,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Madness,
        hints: &[KeywordDispatchHint::Madness],
        matches: matches_madness,
        lower: lower_madness,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Escape,
        hints: &[KeywordDispatchHint::Escape],
        matches: matches_escape,
        lower: lower_escape,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Morph,
        hints: &[KeywordDispatchHint::MorphFamily],
        matches: matches_morph,
        lower: lower_morph,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Squad,
        hints: &[KeywordDispatchHint::Squad],
        matches: matches_squad,
        lower: lower_squad,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Transmute,
        hints: &[KeywordDispatchHint::Transmute],
        matches: matches_transmute,
        lower: lower_transmute,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::CastThisSpellOnly,
        hints: &[KeywordDispatchHint::CastThisSpellOnly],
        matches: matches_cast_this_spell_only,
        lower: lower_cast_this_spell_only,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Gift,
        hints: &[KeywordDispatchHint::Gift],
        matches: matches_gift,
        lower: lower_gift,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::Warp,
        hints: &[KeywordDispatchHint::Warp],
        matches: matches_warp,
        lower: lower_warp,
    },
    KeywordLineRule {
        cst_kind: KeywordLineKindCst::ExertAttack,
        hints: &[KeywordDispatchHint::AlternativeOrExertFamily],
        matches: matches_exert_attack,
        lower: lower_exert_attack,
    },
];

pub(crate) fn parse_keyword_line_cst(
    line: &PreprocessedLine,
) -> Result<Option<KeywordLineCst>, CardTextError> {
    let normalized = line.info.normalized.normalized.as_str();
    let tokens = rewrite_keyword_dash_parse_tokens(&line.tokens);
    let Some(hint) = parse_keyword_dispatch_hint(&tokens) else {
        return Ok(None);
    };

    for rule in KEYWORD_LINE_RULES {
        if !rule.hints.contains(&hint) {
            continue;
        }
        if (rule.matches)(line, &tokens)? {
            return Ok(Some(KeywordLineCst {
                info: line.info.clone(),
                text: normalized.to_string(),
                parse_tokens: tokens,
                kind: rule.cst_kind,
            }));
        }
    }

    Ok(None)
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

pub(crate) fn lower_keyword_line_cst(
    keyword: KeywordLineCst,
) -> Result<RewriteKeywordLine, CardTextError> {
    let mut line = RewriteKeywordLine {
        info: keyword.info,
        text: keyword.text,
        kind: keyword.kind,
        parsed: LineAst::Statement {
            effects: Vec::new(),
        },
    };
    line.parsed = lower_keyword_line_ast(&line, &keyword.parse_tokens)?;
    Ok(line)
}

pub(crate) fn lower_keyword_line_ast(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    if let Some(chunk) = lower_keyword_special_cases(line, parse_tokens)? {
        return Ok(chunk);
    }

    let rule = KEYWORD_LINE_RULES
        .iter()
        .find(|rule| rule.cst_kind == line.kind)
        .ok_or_else(|| {
            CardTextError::InvariantViolation(format!(
                "no keyword lowering rule registered for {:?}",
                line.kind
            ))
        })?;
    (rule.lower)(line, parse_tokens)
}

pub(crate) fn rewrite_keyword_dash_parse_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let Some((label_tokens, body_tokens)) = split_em_dash_label_prefix_tokens(tokens) else {
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

fn require_keyword_parse<T>(
    line: &RewriteKeywordLine,
    family: &str,
    parsed: Option<T>,
) -> Result<T, CardTextError> {
    parsed.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite keyword lowering could not parse {family} line '{}'",
            line.info.raw_line
        ))
    })
}

fn optional_cost_tail_effect_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let comma_idx = find_token_index(tokens, |token| token.kind == TokenKind::Comma)?;
    let effect_tokens = trim_lexed_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    (!effect_tokens.is_empty()).then_some(effect_tokens)
}

fn lower_additional_cost(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let effect_tokens = additional_cost_tail_tokens(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite keyword lowering could not find additional cost tail '{}'",
            line.info.raw_line
        ))
    })?;
    let effects = parse_effect_sentences_lexed(effect_tokens)?;
    Ok(LineAst::AdditionalCost { effects })
}

fn lower_additional_cost_choice(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let effect_tokens = additional_cost_tail_tokens(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite keyword lowering could not find additional cost-choice tail '{}'",
            line.info.raw_line
        ))
    })?;
    let options = require_keyword_parse(
        line,
        "additional cost-choice",
        parse_additional_cost_choice_options_lexed(effect_tokens)?,
    )?;
    Ok(LineAst::AdditionalCostChoice { options })
}

fn lower_alternative_cast(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    if let Some(method) = parse_self_free_cast_alternative_cost_line_lexed(tokens) {
        return Ok(LineAst::AlternativeCastingMethod(method));
    }
    if let Some(method) =
        parse_you_may_rather_than_spell_cost_line_lexed(tokens, line.text.as_str())?
    {
        return Ok(LineAst::AlternativeCastingMethod(method));
    }
    if let Some(method) =
        parse_if_conditional_alternative_cost_line_lexed(tokens, line.text.as_str())?
    {
        return Ok(LineAst::AlternativeCastingMethod(method));
    }
    Err(CardTextError::ParseError(format!(
        "rewrite keyword lowering could not parse alternative cost line '{}'",
        line.info.raw_line
    )))
}

fn lower_bestow(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::AlternativeCastingMethod(require_keyword_parse(
        line,
        "bestow",
        parse_bestow_line_lexed(tokens)?,
    )?))
}

fn lower_bargain(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::OptionalCost(require_keyword_parse(
        line,
        "bargain",
        parse_bargain_line_lexed(tokens)?,
    )?))
}

fn lower_buyback(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::OptionalCost(require_keyword_parse(
        line,
        "buyback",
        parse_buyback_line_lexed(tokens)?,
    )?))
}

fn lower_channel(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::Ability(require_keyword_parse(
        line,
        "channel",
        parse_channel_line_lexed(tokens)?,
    )?))
}

fn lower_cycling(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::Ability(require_keyword_parse(
        line,
        "cycling",
        parse_cycling_line_lexed(tokens)?,
    )?))
}

fn lower_equip(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::Ability(require_keyword_parse(
        line,
        "equip",
        parse_equip_line_lexed(tokens)?,
    )?))
}

fn lower_escape(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::AlternativeCastingMethod(require_keyword_parse(
        line,
        "escape",
        parse_escape_line_lexed(tokens)?,
    )?))
}

fn lower_flashback(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::AlternativeCastingMethod(require_keyword_parse(
        line,
        "flashback",
        parse_flashback_line_lexed(tokens)?,
    )?))
}

fn lower_harmonize(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::AlternativeCastingMethod(require_keyword_parse(
        line,
        "harmonize",
        parse_harmonize_line_lexed(tokens)?,
    )?))
}

fn lower_kicker(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::OptionalCost(require_keyword_parse(
        line,
        "kicker",
        parse_kicker_line_lexed(tokens)?,
    )?))
}

fn lower_madness(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::AlternativeCastingMethod(require_keyword_parse(
        line,
        "madness",
        parse_madness_line_lexed(tokens)?,
    )?))
}

fn lower_morph(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::Ability(require_keyword_parse(
        line,
        "morph",
        parse_morph_keyword_line_lexed(tokens)?,
    )?))
}

fn lower_multikicker(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::OptionalCost(require_keyword_parse(
        line,
        "multikicker",
        parse_multikicker_line_lexed(tokens)?,
    )?))
}

fn lower_offspring(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::OptionalCost(require_keyword_parse(
        line,
        "offspring",
        parse_offspring_line_lexed(tokens)?,
    )?))
}

fn lower_reinforce(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::Ability(require_keyword_parse(
        line,
        "reinforce",
        parse_reinforce_line_lexed(tokens)?,
    )?))
}

fn lower_squad(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    if let Some(effect_tokens) = optional_cost_tail_effect_tokens(tokens)
        && let Ok(effects) = parse_effect_sentences_lexed(effect_tokens)
        && !effects.is_empty()
    {
        return Ok(LineAst::Statement { effects });
    }

    Ok(LineAst::OptionalCost(require_keyword_parse(
        line,
        "squad",
        parse_squad_line_lexed(tokens)?,
    )?))
}

fn lower_transmute(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::Ability(require_keyword_parse(
        line,
        "transmute",
        parse_transmute_line_lexed(tokens)?,
    )?))
}

fn lower_entwine(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::OptionalCost(require_keyword_parse(
        line,
        "entwine",
        parse_entwine_line_lexed(tokens)?,
    )?))
}

fn lower_cast_this_spell_only(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::StaticAbility(
        require_keyword_parse(
            line,
            "cast restriction",
            parse_cast_this_spell_only_line_lexed(tokens)?,
        )?
        .into(),
    ))
}

fn lower_gift(
    line: &RewriteKeywordLine,
    _tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    lower_gift_keyword_line(line)
}

fn lower_warp(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    Ok(LineAst::AlternativeCastingMethod(require_keyword_parse(
        line,
        "warp",
        parse_warp_line_lexed(tokens)?,
    )?))
}

fn lower_exert_attack(
    line: &RewriteKeywordLine,
    tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    lower_exert_attack_keyword_line(line, tokens)
}

fn matches_additional_cost_choice(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    parse_additional_cost_kind(tokens)
}

fn matches_additional_cost(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(additional_cost_tail_tokens(tokens).is_some() && !parse_additional_cost_kind(tokens)?)
}

fn matches_alternative_cast(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    parse_alternative_cast_kind(tokens)
}

fn matches_bestow(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_bestow_line_lexed(tokens)?.is_some())
}

fn matches_bargain(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_bargain_line_lexed(tokens)?.is_some())
}

fn matches_buyback(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_buyback_line_lexed(tokens)?.is_some())
}

fn matches_channel(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_channel_line_lexed(tokens)?.is_some())
}

fn matches_cycling(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_cycling_line_lexed(tokens)?.is_some())
}

fn matches_reinforce(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_reinforce_line_lexed(tokens)?.is_some())
}

fn matches_equip(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_equip_line_lexed(tokens)?.is_some())
}

fn matches_kicker(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_kicker_line_lexed(tokens)?.is_some())
}

fn matches_flashback(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_flashback_line_lexed(tokens)?.is_some())
}

fn matches_harmonize(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_harmonize_line_lexed(tokens)?.is_some())
}

fn matches_multikicker(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_multikicker_line_lexed(tokens)?.is_some())
}

fn matches_entwine(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_entwine_line_lexed(tokens)?.is_some())
}

fn matches_offspring(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_offspring_line_lexed(tokens)?.is_some())
}

fn matches_madness(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_madness_line_lexed(tokens)?.is_some())
}

fn matches_escape(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_escape_line_lexed(tokens)?.is_some())
}

fn matches_morph(line: &PreprocessedLine, tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
    if is_morph_family_dash_keyword_line(&line.tokens) {
        return Ok(false);
    }
    Ok(parse_morph_keyword_line_lexed(tokens)?.is_some())
}

fn matches_squad(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_squad_line_lexed(tokens)?.is_some())
}

fn matches_transmute(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_transmute_line_lexed(tokens)?.is_some())
}

fn matches_cast_this_spell_only(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(parse_cast_this_spell_only_line_lexed(tokens)?.is_some())
}

fn matches_gift(line: &PreprocessedLine, _tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
    Ok(is_standard_gift_keyword_line(line.info.raw_line.as_str()))
}

fn matches_warp(_line: &PreprocessedLine, tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
    Ok(parse_warp_line_lexed(tokens)?.is_some())
}

fn matches_exert_attack(
    _line: &PreprocessedLine,
    tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    Ok(is_exert_attack_keyword_line(tokens))
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
    let Ok(tokens) = lex_line(raw_line, 0) else {
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
                .is_some()
            || parse_if_this_spell_costs_less_to_cast_line_lexed(tokens)?.is_some(),
    )
}

fn token_words_have_prefix(tokens: &[OwnedLexToken], expected: &[&str]) -> bool {
    let words = TokenWordView::new(tokens);
    if words.len() < expected.len() {
        return false;
    }

    expected
        .iter()
        .enumerate()
        .all(|(idx, expected_word)| words.get(idx) == Some(*expected_word))
}

fn token_words_have_any_prefix(tokens: &[OwnedLexToken], expected: &[&[&str]]) -> bool {
    expected
        .iter()
        .any(|phrase| token_words_have_prefix(tokens, phrase))
}

fn tokens_before_kind(tokens: &[OwnedLexToken], kind: TokenKind) -> &[OwnedLexToken] {
    let split_idx = tokens
        .iter()
        .position(|token| token.kind == kind)
        .unwrap_or(tokens.len());
    &tokens[..split_idx]
}
