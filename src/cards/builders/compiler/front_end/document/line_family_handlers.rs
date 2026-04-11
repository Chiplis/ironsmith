use super::line_dispatch::{LineDispatchContext, LineDispatchResult};
use super::*;

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
        is_ward_or_echo_static_prefix_tokens(&ctx.line.tokens).then(|| {
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
        crate::cards::builders::compiler::grammar::structure::classify_statement_line_family_lexed(
            &ctx.line.tokens
        ),
        Some(crate::cards::builders::compiler::grammar::structure::StatementLineFamily::PactNextUpkeep)
    ) || looks_like_statement_line_lexed(ctx.line))
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
                    crate::cards::builders::compiler::grammar::structure::classify_statement_line_family_lexed(
                        &ctx.line.tokens
                    ),
                    Some(crate::cards::builders::compiler::grammar::structure::StatementLineFamily::PactNextUpkeep)
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
    let prefix_cst = if let Some(statement_line) = parse_statement_line_cst(&prefix_line)? {
        RewriteLineCst::Statement(statement_line)
    } else if let Some(rewritten_prefix) = normalize_named_source_sentence_for_builder(
        builder,
        prefix_line.info.normalized.normalized.as_str(),
    ) {
        let rewritten_prefix_line = rewrite_line_normalized(line, rewritten_prefix.as_str())?;
        if let Some(statement_line) = parse_statement_line_cst(&rewritten_prefix_line)? {
            RewriteLineCst::Statement(statement_line)
        } else if let Some(static_line) = parse_static_line_cst(&rewritten_prefix_line)? {
            RewriteLineCst::Static(static_line)
        } else {
            return Err(CardTextError::ParseError(format!(
                "parser could not split leading sentence before keyword ability: '{}'",
                line.info.raw_line
            )));
        }
    } else if let Some(static_line) = parse_static_line_cst(&prefix_line)? {
        RewriteLineCst::Static(static_line)
    } else {
        return Err(CardTextError::ParseError(format!(
            "parser could not split leading sentence before keyword ability: '{}'",
            line.info.raw_line
        )));
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
