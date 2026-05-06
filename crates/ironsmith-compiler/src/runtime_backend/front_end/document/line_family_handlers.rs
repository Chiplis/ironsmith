use super::line_dispatch::{LineDispatchContext, LineDispatchResult};
use super::*;

const MAX_SPEED_CONDITION_LABEL: &str = "__max_speed_condition";

pub(super) fn run_trailing_keyword_activation_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    try_parse_trailing_keyword_activation_dispatch(&ctx.preprocessed.builder, ctx.idx, ctx.line)
}

pub(super) fn run_labeled_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    try_parse_labeled_line_dispatch(ctx.preprocessed, ctx.idx, ctx.line, ctx.allow_unsupported)
}

pub(super) fn run_triggered_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    try_parse_triggered_line_dispatch(ctx.preprocessed, ctx.idx, ctx.line, ctx.allow_unsupported)
}

pub(super) fn run_max_speed_labeled_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let raw = ctx.line.info.raw_line.trim_start();
    if !raw.to_ascii_lowercase().starts_with("max speed") {
        return Ok(None);
    };

    let body_text = raw
        .find('\u{2014}')
        .and_then(|idx| raw.get(idx + '\u{2014}'.len_utf8()..))
        .or_else(|| raw.split_once('-').map(|(_, body)| body))
        .map(str::trim)
        .filter(|body| !body.is_empty())
        .unwrap_or(ctx.line.info.normalized.normalized.as_str())
        .trim()
        .to_string();
    if body_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "max-speed label missing ability body: '{}'",
            ctx.line.info.raw_line
        )));
    }

    let body_lower = body_text.to_ascii_lowercase();
    if body_lower.starts_with("when ")
        || body_lower.starts_with("whenever ")
        || body_lower.starts_with("at ")
    {
        let triggered_text = max_speed_intervening_if_text(body_text.as_str());
        let triggered_line = rewrite_line_normalized(ctx.line, triggered_text.as_str())?;
        let triggered = parse_triggered_line_cst(&triggered_line)?;
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Triggered(triggered),
            ctx.idx + 1,
        )));
    }

    let activation_text = format!(
        "{}. Activate only if you have max speed.",
        body_text.trim_end_matches('.')
    );
    let activation_line = rewrite_line_normalized(ctx.line, activation_text.as_str())?;
    if let Some((cost_tokens, effect_parse_tokens)) =
        split_activation_text_tokens_lexed(&activation_line.tokens)
    {
        let cost_text = render_token_slice(&cost_tokens);
        let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
        match parse_activation_cost_tokens_rewrite(&cost_tokens) {
            Ok(cost) => {
                return Ok(Some(LineDispatchResult::single(
                    RewriteLineCst::Activated(ActivatedLineCst {
                        info: ctx.line.info.clone(),
                        cost,
                        cost_parse_tokens: cost_tokens,
                        effect_text,
                        effect_parse_tokens,
                        chosen_option_label: None,
                    }),
                    ctx.idx + 1,
                )));
            }
            Err(err) if looks_like_activation_cost_prefix(cost_text.as_str()) => {
                return Err(err);
            }
            Err(_) => {}
        }
    }

    let Some(static_cst) = parse_static_line_cst(ctx.line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower max-speed labeled line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(StaticLineCst {
            chosen_option_label: Some(MAX_SPEED_CONDITION_LABEL.to_string()),
            ..static_cst
        }),
        ctx.idx + 1,
    )))
}

fn max_speed_intervening_if_text(body_text: &str) -> String {
    let trimmed = body_text.trim().trim_end_matches('.');
    let Some((trigger, effects)) = trimmed.split_once(',') else {
        return trimmed.to_string();
    };
    format!("{trigger}, if you have max speed,{effects}")
}

pub(super) fn run_start_your_engines_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let lower = ctx.line.info.raw_line.trim_start().to_ascii_lowercase();
    if !lower.starts_with("start your engines!")
        && lower.trim_end_matches('.').trim() != "start your engines"
    {
        return Ok(None);
    }

    let start_line = rewrite_line_normalized(ctx.line, "start your engines")?;
    let Some(start_static) = parse_static_line_cst(&start_line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower start-your-engines keyword line: '{}'",
            ctx.line.info.raw_line
        )));
    };

    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(start_static),
        ctx.idx + 1,
    )))
}

pub(super) fn run_partner_with_keyword_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(partner_name) = partner_with_name_from_line(ctx.line.info.raw_line.as_str()) else {
        return Ok(None);
    };

    let partner_line = rewrite_line_normalized(ctx.line, "partner")?;
    let Some(partner_static) = parse_static_line_cst(&partner_line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower partner-with keyword head: '{}'",
            ctx.line.info.raw_line
        )));
    };

    let trigger_text = format!(
        "when this creature enters, target player may search their library for a card named \"{}\", reveal it, put it into their hand, then shuffle",
        partner_name.replace('"', "")
    );
    let trigger_line = rewrite_line_normalized(ctx.line, trigger_text.as_str())?;
    let partner_trigger = parse_triggered_line_cst(&trigger_line)?;

    Ok(Some(LineDispatchResult {
        lines: vec![
            RewriteLineCst::Static(partner_static),
            RewriteLineCst::Triggered(partner_trigger),
        ],
        next_idx: ctx.idx + 1,
    }))
}

pub(super) fn run_keyword_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    Ok(parse_keyword_line_cst(ctx.line)?.map(|keyword_line| {
        LineDispatchResult::single(RewriteLineCst::Keyword(keyword_line), ctx.idx + 1)
    }))
}

pub(super) fn run_ward_or_echo_static_prefix_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let normalized = ctx.line.info.normalized.normalized.as_str();
    Ok(
        is_ward_or_echo_static_prefix_line_lexed(&ctx.line.tokens).then(|| {
            LineDispatchResult::single(
                RewriteLineCst::Static(StaticLineCst {
                    info: ctx.line.info.clone(),
                    text: normalized.to_string(),
                    parse_tokens: rewrite_keyword_dash_parse_tokens(&ctx.line.tokens),
                    chosen_option_label: None,
                }),
                ctx.idx + 1,
            )
        }),
    )
}

pub(super) fn run_activation_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if (!str_starts_with_char(ctx.line.info.raw_line.trim_start(), '(')
        || is_fully_parenthetical_line(ctx.line.info.raw_line.as_str()))
        && let Some((cost_tokens, effect_parse_tokens)) = split_label_prefix_lexed(&ctx.line.tokens)
            .filter(|(label, _)| is_named_ability_label(label.as_str()))
            .and_then(|(_, body_tokens)| split_activation_text_tokens_lexed(body_tokens))
            .or_else(|| split_activation_text_tokens_lexed(&ctx.line.tokens))
    {
        let cost_text = render_token_slice(&cost_tokens);
        let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
        match parse_activation_cost_tokens_rewrite(&cost_tokens) {
            Ok(cost) => {
                return Ok(Some(LineDispatchResult::single(
                    RewriteLineCst::Activated(ActivatedLineCst {
                        info: ctx.line.info.clone(),
                        cost,
                        cost_parse_tokens: cost_tokens,
                        effect_text,
                        effect_parse_tokens,
                        chosen_option_label: None,
                    }),
                    ctx.idx + 1,
                )));
            }
            Err(err) if looks_like_activation_cost_prefix(cost_text.as_str()) => {
                return Err(err);
            }
            Err(_) => {}
        }
    }

    Ok(None)
}

fn partner_with_name_from_line(raw_line: &str) -> Option<String> {
    let trimmed = raw_line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let rest_start = "partner with ".len();
    if !lower.starts_with("partner with ") {
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
    (!name.is_empty()).then(|| name.to_string())
}

pub(super) fn run_combined_static_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let normalized = ctx.line.info.normalized.normalized.as_str();
    let Some(PreprocessedItem::Line(next_line)) = ctx.preprocessed.items.get(ctx.idx + 1) else {
        return Ok(None);
    };
    if !should_try_combined_static_tokens(&ctx.line.tokens, &next_line.tokens) {
        return Ok(None);
    }

    let combined_text = format!(
        "{}. {}",
        normalized.trim_end_matches('.'),
        next_line.info.normalized.normalized.trim_end_matches('.')
    );
    let combined_line = rewrite_line_normalized(ctx.line, combined_text.as_str())?;
    Ok(parse_static_line_cst(&combined_line)?.map(|static_line| {
        LineDispatchResult::single(RewriteLineCst::Static(static_line), ctx.idx + 2)
    }))
}

pub(super) fn run_statement_probe_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if (matches!(
        crate::runtime_backend::grammar::structure::classify_statement_line_family_lexed(
            &ctx.line.tokens
        ),
        Some(crate::runtime_backend::grammar::structure::StatementLineFamily::PactNextUpkeep)
    ) || looks_like_statement_line_lexed(ctx.line)
        || should_prefer_statement_before_static_for_nonpermanent_spell(
            ctx.preprocessed,
            &ctx.line.tokens,
        ))
        && let Some(statement_line) = parse_statement_line_cst(ctx.line)?
    {
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Statement(statement_line),
            ctx.idx + 1,
        )));
    }
    Ok(None)
}

pub(super) fn run_static_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    Ok(parse_static_line_cst(ctx.line)?.map(|static_line| {
        LineDispatchResult::single(RewriteLineCst::Static(static_line), ctx.idx + 1)
    }))
}

pub(super) fn run_statement_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    Ok(parse_statement_line_cst(ctx.line)?.map(|statement_line| {
        LineDispatchResult::single(RewriteLineCst::Statement(statement_line), ctx.idx + 1)
    }))
}

pub(super) fn run_colon_nonactivation_statement_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    Ok(
        parse_colon_nonactivation_statement_fallback(ctx.line)?.map(|statement_line| {
            LineDispatchResult::single(RewriteLineCst::Statement(statement_line), ctx.idx + 1)
        }),
    )
}

pub(super) fn run_unsupported_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if ctx.allow_unsupported {
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Unsupported(UnsupportedLineCst {
                info: ctx.line.info.clone(),
                reason_code: if matches!(
                    crate::runtime_backend::grammar::structure::classify_statement_line_family_lexed(
                        &ctx.line.tokens
                    ),
                    Some(crate::runtime_backend::grammar::structure::StatementLineFamily::PactNextUpkeep)
                ) {
                    "statement-line-not-yet-supported"
                } else {
                    classify_unsupported_line_reason(ctx.line)
                },
            }),
            ctx.idx + 1,
        )));
    }

    Err(CardTextError::ParseError(format!(
        "parser does not yet support line family: '{}'",
        ctx.line.info.raw_line
    )))
}

fn try_parse_trailing_keyword_activation_dispatch(
    builder: &CardDefinitionBuilder,
    idx: usize,
    line: &PreprocessedLine,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some((prefix_tokens, suffix_tokens)) =
        normalize_trailing_keyword_activation_sentence_lexed(&line.tokens)
    else {
        return Ok(None);
    };

    let prefix_line = rewrite_line_tokens(line, &prefix_tokens);
    let (prefix_statement, prefix_statement_error) = match parse_statement_line_cst(&prefix_line) {
        Ok(statement) => (statement, None),
        Err(err) => (None, Some(err)),
    };
    let prefix_cst = if let Some(statement_line) = prefix_statement {
        RewriteLineCst::Statement(statement_line)
    } else {
        parse_keyword_activation_prefix_static_or_rewrite(
            builder,
            line,
            &prefix_line,
            prefix_statement_error,
        )?
    };

    let suffix_line = rewrite_line_tokens(line, &suffix_tokens);
    let Some((_label, body_tokens)) = split_label_prefix_lexed(&suffix_line.tokens) else {
        return Err(CardTextError::ParseError(format!(
            "parser could not recover keyword activation suffix: '{}'",
            line.info.raw_line
        )));
    };
    let Some((cost_tokens, effect_parse_tokens)) = split_activation_text_tokens_lexed(body_tokens)
    else {
        return Err(CardTextError::ParseError(format!(
            "parser could not recover activation suffix: '{}'",
            line.info.raw_line
        )));
    };
    let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
    let cost = parse_activation_cost_tokens_rewrite(&cost_tokens)?;
    let activated = RewriteLineCst::Activated(ActivatedLineCst {
        info: suffix_line.info.clone(),
        cost,
        cost_parse_tokens: cost_tokens,
        effect_text,
        effect_parse_tokens,
        chosen_option_label: None,
    });

    Ok(Some(LineDispatchResult {
        lines: vec![prefix_cst, activated],
        next_idx: idx + 1,
    }))
}

fn parse_keyword_activation_prefix_static_or_rewrite(
    builder: &CardDefinitionBuilder,
    line: &PreprocessedLine,
    prefix_line: &PreprocessedLine,
    statement_error: Option<CardTextError>,
) -> Result<RewriteLineCst, CardTextError> {
    let static_error = match parse_static_line_cst(prefix_line) {
        Ok(Some(static_line)) => return Ok(RewriteLineCst::Static(static_line)),
        Ok(None) => None,
        Err(err) => Some(err),
    };

    if let Some(rewritten_prefix) = normalize_named_source_sentence_for_builder(
        builder,
        prefix_line.info.normalized.normalized.as_str(),
    ) {
        let rewritten_prefix_line = rewrite_line_normalized(line, rewritten_prefix.as_str())?;
        if let Some(statement_line) = parse_statement_line_cst(&rewritten_prefix_line)? {
            return Ok(RewriteLineCst::Statement(statement_line));
        }
        if let Some(static_line) = parse_static_line_cst(&rewritten_prefix_line)? {
            return Ok(RewriteLineCst::Static(static_line));
        }
    }

    if let Some(err) = statement_error {
        return Err(err);
    }
    if let Some(err) = static_error {
        return Err(err);
    }

    Err(CardTextError::ParseError(format!(
        "parser could not split leading sentence before keyword ability: '{}'",
        line.info.raw_line
    )))
}
