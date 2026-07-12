use super::super::grammar::keyword_special_lines;
use super::super::grammar::line_families as line_grammar;
use super::super::grammar::line_family_rewrites as line_rewrite_grammar;
use super::line_dispatch::{LineDispatchContext, LineDispatchResult};
use super::*;

fn push_synthetic_words(tokens: &mut Vec<OwnedLexToken>, words: &[&str]) {
    let mut cursor = tokens
        .iter()
        .map(|token| token.span.end)
        .max()
        .unwrap_or_default()
        .saturating_add(1);
    for word in words {
        let end = cursor.saturating_add(word.len());
        tokens.push(OwnedLexToken::word(
            *word,
            TextSpan {
                line: 0,
                start: cursor,
                end,
            },
        ));
        cursor = end.saturating_add(1);
    }
}

fn synthetic_word_tokens(words: &[&str]) -> Vec<OwnedLexToken> {
    let mut tokens = Vec::with_capacity(words.len());
    push_synthetic_words(&mut tokens, words);
    tokens
}

fn synthetic_sentence_tokens(words: &[&str]) -> Vec<OwnedLexToken> {
    let mut tokens = synthetic_word_tokens(words);
    tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    tokens
}

fn parse_static_line_from_tokens(
    line: &PreprocessedLine,
    parse_tokens: Vec<OwnedLexToken>,
) -> Result<Option<StaticLineCst>, CardTextError> {
    parse_static_line_cst(&rewrite_line_tokens(line, &parse_tokens))
}

pub(super) fn run_trailing_keyword_activation_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    try_parse_trailing_keyword_activation_dispatch(&ctx.preprocessed.builder, ctx.idx, ctx.line)
}

pub(super) fn run_labeled_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if is_sticker_sheet_ticket_marker_line(ctx) {
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Static(sticker_sheet_ticket_marker_static_line(ctx)),
            ctx.idx + 1,
        )));
    }

    try_parse_labeled_line_dispatch(ctx.preprocessed, ctx.idx, ctx.line, ctx.allow_unsupported)
}

fn sticker_sheet_ticket_marker_static_line(ctx: &LineDispatchContext<'_>) -> StaticLineCst {
    StaticLineCst {
        info: ctx.line.info.clone(),
        text: ctx.line.info.normalized.normalized.clone(),
        parse_tokens: ctx.line.tokens.clone(),
        chosen_option: None,
        parsed: None,
    }
}

fn is_sticker_sheet_ticket_marker_line(ctx: &LineDispatchContext<'_>) -> bool {
    let is_sticker_sheet = ctx.preprocessed.items.iter().any(|item| {
        matches!(
            item,
            PreprocessedItem::Metadata(metadata)
                if matches!(
                    &metadata.value,
                    crate::runtime_backend::MetadataLine::TypeLine(value)
                        if value.eq_ignore_ascii_case("Stickers")
                )
        )
    });
    if !is_sticker_sheet {
        return false;
    }
    line_grammar::parse_sticker_ticket_marker(&ctx.line.tokens).is_some()
}

pub(super) fn run_triggered_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    try_parse_triggered_line_dispatch(ctx.preprocessed, ctx.idx, ctx.line, ctx.allow_unsupported)
}

pub(super) fn run_championed_with_this_trigger_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(shape) = line_grammar::parse_championed_with_this_trigger(&ctx.line.tokens) else {
        return Ok(None);
    };
    let mut triggered_tokens = synthetic_word_tokens(&["When", "this", "creature", "enters"]);
    triggered_tokens.push(OwnedLexToken::comma(TextSpan::synthetic()));
    triggered_tokens
        .extend_from_slice(line_grammar::parse_visible_line_tokens(shape.effect_tokens));
    let triggered_line = rewrite_line_tokens(ctx.line, &triggered_tokens);
    let triggered = parse_triggered_line_cst(&triggered_line)?;
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Triggered(triggered),
        ctx.idx + 1,
    )))
}

pub(super) fn run_max_speed_labeled_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(shape) = line_grammar::parse_max_speed_line(&ctx.line.tokens) else {
        return Ok(None);
    };

    let body_tokens = shape.body_tokens;
    if body_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "max-speed label missing ability body: '{}'",
            ctx.line.info.raw_line
        )));
    }

    let body_line = rewrite_line_tokens(ctx.line, body_tokens);
    if shape.trigger_intro.is_some() {
        let triggered_tokens = max_speed_intervening_if_tokens(&body_line.tokens);
        let triggered_line = rewrite_line_tokens(ctx.line, &triggered_tokens);
        let triggered = parse_triggered_line_cst(&triggered_line)?;
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Triggered(triggered),
            ctx.idx + 1,
        )));
    }

    let mut activation_tokens = tokens_without_terminal_period(&body_line.tokens).to_vec();
    activation_tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    push_synthetic_words(
        &mut activation_tokens,
        &["activate", "only", "if", "you", "have", "max", "speed"],
    );
    activation_tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    let activation_line = rewrite_line_tokens(ctx.line, &activation_tokens);
    if let Some((cost_tokens, effect_parse_tokens)) =
        split_activation_text_tokens_lexed(&activation_line.tokens)
    {
        let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
        let normalized_cost_tokens = normalize_activation_cost_tokens_for_builder(
            &ctx.preprocessed.builder,
            ctx.line,
            cost_tokens.clone(),
        )?;
        match parse_activation_cost_tokens_rewrite(&normalized_cost_tokens) {
            Ok(cost) => {
                return Ok(Some(LineDispatchResult::single(
                    RewriteLineCst::Activated(ActivatedLineCst {
                        info: ctx.line.info.clone(),
                        cost,
                        cost_parse_tokens: normalized_cost_tokens,
                        effect_text,
                        effect_parse_tokens,
                        presentation_label: None,
                        chosen_option: None,
                    }),
                    ctx.idx + 1,
                )));
            }
            Err(err) if looks_like_activation_cost_prefix(&cost_tokens) => {
                return Err(err);
            }
            Err(_) => {}
        }
    }

    let Some(static_cst) = parse_static_line_cst(&body_line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower max-speed labeled line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(StaticLineCst {
            chosen_option: Some(ChosenOptionContext::MaxSpeed),
            ..static_cst
        }),
        ctx.idx + 1,
    )))
}

fn tokens_without_terminal_period(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        &tokens[..tokens.len().saturating_sub(1)]
    } else {
        tokens
    }
}

fn tokens_before_reminder_or_terminal_period(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    line_grammar::parse_visible_line_tokens(tokens)
}

fn max_speed_intervening_if_tokens(body_tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let visible = line_grammar::parse_visible_max_speed_tokens(body_tokens);
    let Some(shape) = line_grammar::parse_max_speed_trigger_split(body_tokens) else {
        return visible.to_vec();
    };
    let mut tokens = shape.before.to_vec();
    tokens.push(OwnedLexToken::comma(TextSpan::synthetic()));
    push_synthetic_words(&mut tokens, &["if", "you", "have", "max", "speed"]);
    tokens.push(OwnedLexToken::comma(TextSpan::synthetic()));
    tokens.extend_from_slice(shape.after);
    tokens
}

pub(super) fn run_start_your_engines_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if line_grammar::parse_simple_document_line(&ctx.line.tokens)
        != Some(line_grammar::SimpleDocumentLineShape::StartYourEngines)
    {
        return Ok(None);
    }

    let start_tokens = synthetic_word_tokens(&["start", "your", "engines"]);
    let Some(start_static) = parse_static_line_from_tokens(ctx.line, start_tokens)? else {
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

pub(super) fn run_draft_rule_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if line_grammar::parse_draft_rule_line(&ctx.line.tokens).is_none() {
        return Ok(None);
    }

    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(StaticLineCst {
            info: ctx.line.info.clone(),
            text: ctx.line.info.normalized.normalized.clone(),
            parse_tokens: ctx.line.tokens.clone(),
            chosen_option: None,
            parsed: None,
        }),
        ctx.idx + 1,
    )))
}

pub(super) fn run_learn_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if line_grammar::parse_simple_document_line(&ctx.line.tokens)
        != Some(line_grammar::SimpleDocumentLineShape::Learn)
    {
        return Ok(None);
    }

    let learn_tokens = line_grammar::parse_visible_line_tokens(&ctx.line.tokens).to_vec();
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Statement(StatementLineCst {
            info: ctx.line.info.clone(),
            text: "learn".to_string(),
            parse_tokens: learn_tokens.clone(),
            parse_groups: vec![learn_tokens],
        }),
        ctx.idx + 1,
    )))
}

pub(super) fn run_split_top_and_face_down_look_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if line_grammar::parse_simple_document_line(&ctx.line.tokens)
        != Some(line_grammar::SimpleDocumentLineShape::SplitTopAndFaceDownLook)
    {
        return Ok(None);
    }

    let top_card_tokens = synthetic_sentence_tokens(&[
        "you", "may", "look", "at", "the", "top", "card", "of", "your", "library", "any", "time",
    ]);
    let face_down_tokens = synthetic_sentence_tokens(&[
        "you",
        "may",
        "look",
        "at",
        "face-down",
        "creatures",
        "you",
        "don't",
        "control",
        "any",
        "time",
    ]);

    let Some(top_card_static) = parse_static_line_from_tokens(ctx.line, top_card_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower split top-card line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    let Some(face_down_static) = parse_static_line_from_tokens(ctx.line, face_down_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower split face-down line: '{}'",
            ctx.line.info.raw_line
        )));
    };

    Ok(Some(LineDispatchResult {
        lines: vec![
            RewriteLineCst::Static(top_card_static),
            RewriteLineCst::Static(face_down_static),
        ],
        next_idx: ctx.idx + 1,
    }))
}

pub(super) fn run_split_top_look_and_top_land_play_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if line_grammar::parse_special_line(&ctx.line.tokens)
        != Some(line_grammar::SpecialLineShape::SplitTopLookAndLandPlay)
    {
        return Ok(None);
    }

    let top_card_tokens = synthetic_sentence_tokens(&[
        "you", "may", "look", "at", "the", "top", "card", "of", "your", "library", "any", "time",
    ]);
    let play_lands_tokens = synthetic_sentence_tokens(&[
        "you", "may", "play", "lands", "from", "the", "top", "of", "your", "library",
    ]);

    let Some(top_card_static) = parse_static_line_from_tokens(ctx.line, top_card_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower split top-card look line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    let Some(play_lands_static) = parse_static_line_from_tokens(ctx.line, play_lands_tokens)?
    else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower split top-library land-play line: '{}'",
            ctx.line.info.raw_line
        )));
    };

    Ok(Some(LineDispatchResult {
        lines: vec![
            RewriteLineCst::Static(top_card_static),
            RewriteLineCst::Static(play_lands_static),
        ],
        next_idx: ctx.idx + 1,
    }))
}

pub(super) fn run_assign_damage_as_unblocked_enchanted_creature_controller_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if line_grammar::parse_special_line(&ctx.line.tokens)
        != Some(line_grammar::SpecialLineShape::AssignDamageAsUnblockedEnchanted)
    {
        return Ok(None);
    }

    let mut rewritten_tokens = synthetic_word_tokens(&["enchanted", "creature", "has"]);
    rewritten_tokens.push(OwnedLexToken::quote(TextSpan::synthetic()));
    push_synthetic_words(
        &mut rewritten_tokens,
        &[
            "you", "may", "have", "this", "creature", "assign", "its", "combat", "damage", "as",
            "though", "it", "weren't", "blocked",
        ],
    );
    rewritten_tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    rewritten_tokens.push(OwnedLexToken::quote(TextSpan::synthetic()));
    rewritten_tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    let Some(static_cst) = parse_static_line_from_tokens(ctx.line, rewritten_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower enchanted-creature assign-damage-as-unblocked line: '{}'",
            ctx.line.info.raw_line
        )));
    };

    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(static_cst),
        ctx.idx + 1,
    )))
}

pub(super) fn run_graveyard_cast_control_condition_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(condition) =
        line_rewrite_grammar::parse_graveyard_cast_control_condition_tokens(&ctx.line.tokens)
    else {
        return Ok(None);
    };

    let permission_tokens = synthetic_sentence_tokens(&[
        "you",
        "may",
        "cast",
        "this",
        "card",
        "from",
        "your",
        "graveyard",
    ]);
    let Some(mut static_cst) = parse_static_line_from_tokens(ctx.line, permission_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower graveyard-cast control condition line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    static_cst.chosen_option = Some(match condition {
        line_rewrite_grammar::GraveyardCastControlCondition::Subtype(subtype) => {
            ChosenOptionContext::ControlsSubtypePermanent(subtype)
        }
        line_rewrite_grammar::GraveyardCastControlCondition::ColorPair(left, right) => {
            ChosenOptionContext::ControlsEitherColorPermanent { left, right }
        }
    });

    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(static_cst),
        ctx.idx + 1,
    )))
}

pub(super) fn run_graveyard_or_exile_cast_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if line_grammar::parse_special_line(&ctx.line.tokens)
        != Some(line_grammar::SpecialLineShape::GraveyardOrExileCast)
    {
        return Ok(None);
    }

    let graveyard_tokens = synthetic_sentence_tokens(&[
        "you",
        "may",
        "cast",
        "this",
        "card",
        "from",
        "your",
        "graveyard",
    ]);
    let exile_tokens =
        synthetic_sentence_tokens(&["you", "may", "cast", "this", "card", "from", "exile"]);

    let Some(graveyard_static) = parse_static_line_from_tokens(ctx.line, graveyard_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower graveyard-or-exile cast line graveyard half: '{}'",
            ctx.line.info.raw_line
        )));
    };
    let Some(exile_static) = parse_static_line_from_tokens(ctx.line, exile_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower graveyard-or-exile cast line exile half: '{}'",
            ctx.line.info.raw_line
        )));
    };

    Ok(Some(LineDispatchResult {
        lines: vec![
            RewriteLineCst::Static(graveyard_static),
            RewriteLineCst::Static(exile_static),
        ],
        next_idx: ctx.idx + 1,
    }))
}

pub(super) fn run_champion_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(shape) = line_grammar::parse_champion_line(&ctx.line.tokens) else {
        return Ok(None);
    };
    let filter_tokens = shape.filter_tokens;
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "champion keyword missing object filter: '{}'",
            ctx.line.info.raw_line
        )));
    }

    let mut triggered_tokens = synthetic_word_tokens(&["When", "this", "permanent", "enters"]);
    triggered_tokens.push(OwnedLexToken::comma(TextSpan::synthetic()));
    push_synthetic_words(
        &mut triggered_tokens,
        &["sacrifice", "it", "unless", "you", "exile", "another"],
    );
    triggered_tokens.extend_from_slice(filter_tokens);
    push_synthetic_words(
        &mut triggered_tokens,
        &[
            "you",
            "control",
            "until",
            "this",
            "permanent",
            "leaves",
            "the",
            "battlefield",
        ],
    );
    triggered_tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    let triggered_line = rewrite_line_tokens(ctx.line, &triggered_tokens);
    let triggered = parse_triggered_line_cst(&triggered_line)?;
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Triggered(triggered),
        ctx.idx + 1,
    )))
}

pub(super) fn run_station_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(station_shape) =
        line_grammar::parse_station_keyword_line(&ctx.line.tokens, &ctx.line.info.source_tokens)
    else {
        return Ok(None);
    };

    let mut activation_tokens =
        synthetic_word_tokens(&["tap", "another", "untapped", "creature", "you", "control"]);
    activation_tokens.push(OwnedLexToken::colon(TextSpan::synthetic()));
    push_synthetic_words(
        &mut activation_tokens,
        &["put", "x", "charge", "counters", "on", "this", "artifact"],
    );
    activation_tokens.push(OwnedLexToken::comma(TextSpan::synthetic()));
    push_synthetic_words(
        &mut activation_tokens,
        &[
            "where", "x", "is", "the", "power", "of", "the", "creature", "tapped", "this", "way",
        ],
    );
    activation_tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    push_synthetic_words(
        &mut activation_tokens,
        &["activate", "only", "as", "a", "sorcery"],
    );
    activation_tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    let activation_line = rewrite_line_tokens(ctx.line, &activation_tokens);
    let Some((cost_tokens, effect_parse_tokens)) =
        split_activation_text_tokens_lexed(&activation_line.tokens)
    else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower station keyword line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    let normalized_cost_tokens = normalize_activation_cost_tokens_for_builder(
        &ctx.preprocessed.builder,
        ctx.line,
        cost_tokens.clone(),
    )?;
    let cost = parse_activation_cost_tokens_rewrite(&normalized_cost_tokens)?;
    let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();

    let mut lines = vec![RewriteLineCst::Activated(ActivatedLineCst {
        info: ctx.line.info.clone(),
        cost,
        cost_parse_tokens: normalized_cost_tokens,
        effect_text,
        effect_parse_tokens,
        presentation_label: None,
        chosen_option: None,
    })];

    let has_explicit_station_threshold_rows = ctx
        .preprocessed
        .items
        .iter()
        .filter_map(|item| match item {
            PreprocessedItem::Line(line) => Some(line),
            PreprocessedItem::Metadata(_) => None,
        })
        .any(|line| line_grammar::parse_station_threshold_line(&line.tokens).is_some());
    if !has_explicit_station_threshold_rows
        && let Some(threshold) = station_shape.creature_threshold
        && let Some(pt) = ctx.preprocessed.builder.card_builder.power_toughness_ref()
    {
        let chosen_option = ChosenOptionContext::StationThreshold(threshold);
        let power = pt.power.base_value();
        let toughness = pt.toughness.base_value();
        for parse_tokens in station_creature_support_parse_tokens(power, toughness) {
            let Some(static_cst) = parse_static_line_from_tokens(ctx.line, parse_tokens)? else {
                return Err(CardTextError::ParseError(format!(
                    "parser could not lower station reminder threshold support: '{}'",
                    ctx.line.info.raw_line
                )));
            };
            lines.push(RewriteLineCst::Static(StaticLineCst {
                chosen_option: Some(chosen_option.clone()),
                ..static_cst
            }));
        }
    }

    Ok(Some(LineDispatchResult {
        lines,
        next_idx: ctx.idx + 1,
    }))
}

pub(super) fn run_station_threshold_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(shape) = line_grammar::parse_station_threshold_line(&ctx.line.tokens) else {
        return Ok(None);
    };
    let threshold = shape.threshold;
    let mut body_tokens = shape.body_tokens.to_vec();
    if shape.needs_terminal_punctuation {
        body_tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    }

    let chosen_option = ChosenOptionContext::StationThreshold(threshold);
    let mut lines = Vec::new();
    if station_threshold_is_creature_pt_threshold(ctx, threshold)
        && let Some(pt) = ctx.preprocessed.builder.card_builder.power_toughness_ref()
    {
        let power = pt.power.base_value();
        let toughness = pt.toughness.base_value();
        for parse_tokens in station_creature_support_parse_tokens(power, toughness) {
            let Some(static_cst) = parse_static_line_from_tokens(ctx.line, parse_tokens)? else {
                return Err(CardTextError::ParseError(format!(
                    "parser could not lower station creature threshold support: '{}'",
                    ctx.line.info.raw_line
                )));
            };
            lines.push(RewriteLineCst::Static(StaticLineCst {
                chosen_option: Some(chosen_option.clone()),
                ..static_cst
            }));
        }
    }

    let body_line = rewrite_line_tokens(ctx.line, &body_tokens);
    if shape.trigger_intro.is_some() {
        let mut triggered = parse_triggered_line_cst(&body_line)?;
        triggered.chosen_option = Some(chosen_option);
        lines.push(RewriteLineCst::Triggered(triggered));
        return Ok(Some(LineDispatchResult {
            lines,
            next_idx: ctx.idx + 1,
        }));
    }

    if let Some((cost_tokens, effect_parse_tokens)) =
        split_activation_text_tokens_lexed(&body_line.tokens)
    {
        let normalized_cost_tokens = normalize_activation_cost_tokens_for_builder(
            &ctx.preprocessed.builder,
            ctx.line,
            cost_tokens.clone(),
        )?;
        let cost = parse_activation_cost_tokens_rewrite(&normalized_cost_tokens)?;
        let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
        lines.push(RewriteLineCst::Activated(ActivatedLineCst {
            info: ctx.line.info.clone(),
            cost,
            cost_parse_tokens: normalized_cost_tokens,
            effect_text,
            effect_parse_tokens,
            presentation_label: None,
            chosen_option: Some(chosen_option),
        }));
        return Ok(Some(LineDispatchResult {
            lines,
            next_idx: ctx.idx + 1,
        }));
    }

    let Some(static_cst) = parse_static_line_cst(&body_line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower station threshold line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    lines.push(RewriteLineCst::Static(StaticLineCst {
        chosen_option: Some(chosen_option),
        ..static_cst
    }));
    Ok(Some(LineDispatchResult {
        lines,
        next_idx: ctx.idx + 1,
    }))
}

fn station_creature_support_parse_tokens(power: i32, toughness: i32) -> [Vec<OwnedLexToken>; 2] {
    let type_line = synthetic_sentence_tokens(&[
        "this", "artifact", "is", "a", "creature", "in", "addition", "to", "its", "other", "types",
    ]);
    let mut pt_line = synthetic_word_tokens(&[
        "this",
        "artifact",
        "has",
        "base",
        "power",
        "and",
        "toughness",
    ]);
    pt_line.push(OwnedLexToken::word(
        format!("{power}/{toughness}"),
        TextSpan::synthetic(),
    ));
    pt_line.push(OwnedLexToken::period(TextSpan::synthetic()));
    [type_line, pt_line]
}

fn station_threshold_is_creature_pt_threshold(
    ctx: &LineDispatchContext<'_>,
    threshold: i32,
) -> bool {
    if ctx
        .preprocessed
        .builder
        .card_builder
        .power_toughness_ref()
        .is_none()
    {
        return false;
    }
    ctx.preprocessed.items.iter().any(|item| {
        let PreprocessedItem::Line(line) = item else {
            return false;
        };
        line_grammar::parse_station_keyword_line(&line.tokens, &line.info.source_tokens)
            .and_then(|shape| shape.creature_threshold)
            == Some(threshold)
    })
}

pub(super) fn run_partner_with_keyword_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(partner_name) = partner_with_name_from_line(ctx.line) else {
        return Ok(None);
    };

    let partner_static_text = format!("partner with {partner_name}");
    let partner_static = StaticLineCst {
        info: ctx.line.info.clone(),
        text: partner_static_text,
        parse_tokens: tokens_before_reminder_or_terminal_period(&ctx.line.tokens).to_vec(),
        chosen_option: None,
        parsed: None,
    };

    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(partner_static),
        ctx.idx + 1,
    )))
}

pub(super) fn run_partner_variant_keyword_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let raw = ctx.line.info.raw_line.trim();
    if line_grammar::parse_partner_variant(&ctx.line.tokens).is_none() {
        return Ok(None);
    }

    let visible_label = keyword_special_lines::parse_partner_visible_label_tokens(&ctx.line.tokens)
        .unwrap_or_else(|| raw.to_string());
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(StaticLineCst {
            info: ctx.line.info.clone(),
            text: visible_label,
            parse_tokens: ctx.line.tokens.clone(),
            chosen_option: None,
            parsed: None,
        }),
        ctx.idx + 1,
    )))
}

pub(super) fn run_escape_enters_with_counter_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if line_grammar::parse_escape_enters_with_line(&ctx.line.tokens).is_none() {
        return Ok(None);
    }
    Ok(parse_static_line_cst(ctx.line)?.map(|static_cst| {
        LineDispatchResult::single(RewriteLineCst::Static(static_cst), ctx.idx + 1)
    }))
}

pub(super) fn run_surge_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(shape) = line_grammar::parse_surge_line(&ctx.line.tokens) else {
        return Ok(None);
    };
    let cost_tokens = shape.cost_tokens;
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "surge keyword missing cost: '{}'",
            ctx.line.info.raw_line
        )));
    }

    let parse_tokens = alternative_cost_parse_tokens(
        &["If", "you've", "cast", "another", "spell", "this", "turn"],
        cost_tokens,
    );
    let alternative_line = rewrite_line_tokens(ctx.line, &parse_tokens);
    let Some(mut keyword) = parse_keyword_line_cst(&alternative_line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower surge keyword line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    keyword.text = ctx.line.info.raw_line.clone();

    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Keyword(keyword),
        ctx.idx + 1,
    )))
}

pub(super) fn run_freerunning_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(shape) = line_grammar::parse_freerunning_line(&ctx.line.tokens) else {
        return Ok(None);
    };
    let cost_tokens = shape.cost_tokens;
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "freerunning keyword missing cost: '{}'",
            ctx.line.info.raw_line
        )));
    }

    let parse_tokens = alternative_cost_parse_tokens(
        &[
            "If",
            "you",
            "dealt",
            "combat",
            "damage",
            "to",
            "a",
            "player",
            "this",
            "turn",
            "with",
            "an",
            "Assassin",
            "or",
            "commander",
        ],
        cost_tokens,
    );
    let alternative_line = rewrite_line_tokens(ctx.line, &parse_tokens);
    let Some(mut keyword) = parse_keyword_line_cst(&alternative_line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower freerunning keyword line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    keyword.text = ctx.line.info.raw_line.clone();

    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Keyword(keyword),
        ctx.idx + 1,
    )))
}

fn alternative_cost_parse_tokens(
    condition_words: &[&str],
    cost_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let mut tokens = synthetic_word_tokens(condition_words);
    tokens.push(OwnedLexToken::comma(TextSpan::synthetic()));
    push_synthetic_words(&mut tokens, &["you", "may", "pay"]);
    tokens.extend(cost_tokens.iter().map(|token| {
        if token.kind == TokenKind::ManaGroup {
            OwnedLexToken::new(token.kind, token.slice.to_ascii_uppercase(), token.span)
        } else {
            token.clone()
        }
    }));
    push_synthetic_words(
        &mut tokens,
        &["rather", "than", "pay", "this", "spell's", "mana", "cost"],
    );
    tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    tokens
}

pub(super) fn run_keyword_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if matches!(
        parse_ability_line_lexed(&ctx.line.tokens).as_deref(),
        Some([crate::cards::builders::KeywordAction::CumulativeUpkeep { .. }])
    ) {
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Static(StaticLineCst {
                info: ctx.line.info.clone(),
                text: ctx.line.info.normalized.normalized.clone(),
                parse_tokens: ctx.line.tokens.clone(),
                chosen_option: None,
                parsed: None,
            }),
            ctx.idx + 1,
        )));
    }
    if let Some(split_lines) = split_same_line_and_or_kicker_keywords(ctx.line)? {
        return Ok(Some(LineDispatchResult {
            lines: split_lines,
            next_idx: ctx.idx + 1,
        }));
    }

    Ok(parse_keyword_line_cst(ctx.line)?.map(|keyword_line| {
        LineDispatchResult::single(RewriteLineCst::Keyword(keyword_line), ctx.idx + 1)
    }))
}

fn split_same_line_and_or_kicker_keywords(
    line: &PreprocessedLine,
) -> Result<Option<Vec<RewriteLineCst>>, CardTextError> {
    let Some(shape) = line_grammar::parse_kicker_branches(&line.tokens) else {
        return Ok(None);
    };

    let branches = [shape.first_cost, shape.second_cost];

    let mut lines = Vec::new();
    for branch in branches {
        let parsed_cost = parse_activation_cost_tokens_rewrite(branch)?;
        let lowered_cost = lower_activation_cost_cst(&parsed_cost)?;
        let cost_text = lowered_cost
            .mana_cost()
            .map(|cost| cost.to_oracle())
            .unwrap_or_else(|| lowered_cost.display());
        let label = format!("Kicker {cost_text}");
        let raw = label.clone();
        let mut tokens = Vec::with_capacity(branch.len() + 1);
        tokens.push(OwnedLexToken::word("kicker", TextSpan::synthetic()));
        tokens.extend_from_slice(branch);
        let rewritten = rewrite_line_tokens(line, &tokens);
        let mut keyword = parse_keyword_line_cst(&rewritten)?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "parser could not split same-line kicker cost '{raw}'"
            ))
        })?;
        keyword
            .payload
            .set_kicker_label(label)
            .map_err(CardTextError::InvariantViolation)?;
        lines.push(RewriteLineCst::Keyword(keyword));
    }

    Ok(Some(lines))
}

pub(super) fn run_additional_combat_after_this_phase_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(shape) =
        line_rewrite_grammar::parse_additional_combat_rewrite_tokens(&ctx.line.tokens)
    else {
        return Ok(None);
    };
    let rewritten_tokens = match shape.kind {
        line_rewrite_grammar::AdditionalCombatRewriteKind::AlreadyCanonical => {
            ctx.line.tokens.clone()
        }
        line_rewrite_grammar::AdditionalCombatRewriteKind::ConditionalAfterThisPhase
        | line_rewrite_grammar::AdditionalCombatRewriteKind::AfterThisPhase => {
            let mut tokens = shape.before_tokens.to_vec();
            push_synthetic_words(&mut tokens, &["After", "this", "main", "phase"]);
            tokens.push(OwnedLexToken::comma(TextSpan::synthetic()));
            push_synthetic_words(
                &mut tokens,
                &[
                    "there",
                    "is",
                    "an",
                    "additional",
                    "combat",
                    "phase",
                    "followed",
                    "by",
                    "an",
                    "additional",
                    "main",
                    "phase",
                ],
            );
            tokens.extend_from_slice(shape.after_tokens);
            tokens
        }
    };
    let rewritten_line = rewrite_line_tokens(ctx.line, &rewritten_tokens);
    let Some(statement_line) = parse_statement_line_cst(&rewritten_line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower additional-combat-after-this-phase line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Statement(statement_line),
        ctx.idx + 1,
    )))
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
                    chosen_option: None,
                    parsed: None,
                }),
                ctx.idx + 1,
            )
        }),
    )
}

pub(super) fn run_activation_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if (!line_starts_with_lparen_token(ctx.line) || is_fully_parenthetical_line(ctx.line))
        && let Some((mut presentation_label, cost_tokens, effect_parse_tokens)) =
            split_label_prefix_lexed(&ctx.line.tokens)
                .filter(|(label, _, _)| is_named_ability_label(label.as_str()))
                .and_then(|(label, _, body_tokens)| {
                    split_activation_text_tokens_lexed(body_tokens).map(
                        |(cost_tokens, effect_tokens)| (Some(label), cost_tokens, effect_tokens),
                    )
                })
                .or_else(|| {
                    split_activation_text_tokens_lexed(&ctx.line.tokens)
                        .map(|(cost_tokens, effect_tokens)| (None, cost_tokens, effect_tokens))
                })
    {
        if presentation_label.is_none() {
            presentation_label = original_activation_presentation_label(
                ctx.line,
                &cost_tokens,
                &effect_parse_tokens,
            );
        }
        let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
        let normalized_cost_tokens = normalize_activation_cost_tokens_for_builder(
            &ctx.preprocessed.builder,
            ctx.line,
            cost_tokens.clone(),
        )?;
        match parse_activation_cost_tokens_rewrite(&normalized_cost_tokens) {
            Ok(cost) => {
                let activated = ActivatedLineCst {
                    info: ctx.line.info.clone(),
                    cost,
                    cost_parse_tokens: normalized_cost_tokens,
                    effect_text,
                    effect_parse_tokens,
                    presentation_label,
                    chosen_option: None,
                };
                let (activated, next_idx) = extend_activated_line_with_result_followups(
                    &ctx.preprocessed.items,
                    ctx.idx,
                    activated,
                );
                return Ok(Some(LineDispatchResult::single(
                    RewriteLineCst::Activated(activated),
                    next_idx,
                )));
            }
            Err(err) if looks_like_activation_cost_prefix(&cost_tokens) => {
                return Err(err);
            }
            Err(_) => {}
        }
    }

    Ok(None)
}

fn original_activation_presentation_label(
    line: &PreprocessedLine,
    cost_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
) -> Option<String> {
    let (label, _, body_tokens) = split_label_prefix_lexed(&line.info.source_tokens)?;
    if !is_named_ability_label(label.as_str()) {
        return None;
    }
    let (original_cost_tokens, original_effect_tokens) =
        split_activation_text_tokens_lexed(body_tokens)?;
    let original_effect_tokens = tokens_before_reminder_or_terminal_period(&original_effect_tokens);
    let effect_tokens = tokens_before_reminder_or_terminal_period(effect_tokens);
    let original_cost_text = render_token_slice(&original_cost_tokens);
    let cost_text = render_token_slice(cost_tokens);
    let original_effect_text = render_token_slice(original_effect_tokens);
    let effect_text = render_token_slice(effect_tokens);
    let costs_match = original_cost_text
        .trim()
        .eq_ignore_ascii_case(cost_text.trim());
    let effects_match = original_effect_text
        .trim()
        .eq_ignore_ascii_case(effect_text.trim());
    (costs_match && effects_match).then(|| label.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn partner_variant_separator_detection_uses_tokens() {
        for line in [
            "Partner—Character select",
            "Partner - Character select",
            "Partner–Character select",
            "Partner-Friends forever",
        ] {
            let tokens = lex_line(line, 0).expect("partner variant line should lex");
            assert!(
                line_grammar::parse_partner_variant(&tokens).is_some(),
                "{line} should be recognized as a partner variant"
            );
        }

        let tokens = lex_line("Partner with Proud Mentor", 0).unwrap();
        assert!(line_grammar::parse_partner_variant(&tokens).is_none());
    }

    #[test]
    fn partner_with_name_and_variant_label_trim_on_lexed_reminder_tokens() {
        fn line(text: &str) -> PreprocessedLine {
            preprocess_document(
                CardDefinitionBuilder::new(crate::CardId::new(), "Partner Test"),
                text,
            )
            .expect("partner line should preprocess")
            .items
            .into_iter()
            .find_map(|item| match item {
                PreprocessedItem::Line(line) => Some(line),
                PreprocessedItem::Metadata(_) => None,
            })
            .expect("partner line should yield a preprocessed line")
        }

        let partner_with_line =
            line("Partner with Toothy, Imaginary Friend (When this creature enters...)");
        assert_eq!(
            partner_with_name_from_line(&partner_with_line).as_deref(),
            Some("Toothy, Imaginary Friend")
        );

        let partner_variant_line = "Partner - Friends forever (You can have two commanders.)";
        let partner_variant_tokens = lex_line(partner_variant_line, 0).expect("line should lex");
        assert_eq!(
            keyword_special_lines::parse_partner_visible_label_tokens(&partner_variant_tokens)
                .as_deref(),
            Some("Partner - Friends forever")
        );
    }

    #[test]
    fn typed_line_family_migration_routes_simple_and_unless_shapes_into_cst() {
        let learn = preprocess_document(
            CardDefinitionBuilder::new(crate::CardId::new(), "Learn Test"),
            "Learn.",
        )
        .expect("learn should preprocess");
        let learn_cst = parse_document_cst(&learn, false).expect("learn cst");
        assert!(matches!(
            learn_cst.lines.as_slice(),
            [RewriteLineCst::Statement(_)]
        ));

        let unless = preprocess_document(
            CardDefinitionBuilder::new(crate::CardId::new(), "Unless Test"),
            "Unless you pay {2}, sacrifice this permanent.",
        )
        .expect("unless should preprocess");
        let unless_cst = parse_document_cst(&unless, false).expect("unless cst");
        let [RewriteLineCst::Statement(line)] = unless_cst.lines.as_slice() else {
            panic!("expected a statement CST for a leading-unless line");
        };
        assert_eq!(
            render_token_slice(&line.parse_tokens),
            "unless you pay {2}, sacrifice this permanent."
        );
    }

    #[test]
    fn station_threshold_line_uses_pipe_and_plus_tokens() {
        fn line(text: &str) -> PreprocessedLine {
            preprocess_document(
                CardDefinitionBuilder::new(crate::CardId::new(), "Station Threshold Test")
                    .card_types(vec![crate::types::CardType::Artifact]),
                text,
            )
            .expect("station threshold line should preprocess")
            .items
            .into_iter()
            .find_map(|item| match item {
                PreprocessedItem::Line(line) => Some(line),
                PreprocessedItem::Metadata(_) => None,
            })
            .expect("expected station threshold preprocessed line")
        }

        let station_line = line("6+ | This artifact is a creature in addition to its other types.");
        let shape = line_grammar::parse_station_threshold_line(&station_line.tokens)
            .expect("station threshold shape");
        assert_eq!(shape.threshold, 6);
        assert_eq!(
            render_token_slice(shape.body_tokens),
            "this artifact is a creature in addition to its other types."
        );

        let missing_plus = line("6 | This artifact is a creature.");
        assert_eq!(
            line_grammar::parse_station_threshold_line(&missing_plus.tokens),
            None
        );
    }

    #[test]
    fn max_speed_trigger_inserts_intervening_condition_without_relexing() {
        let tokens = lex_line("Whenever you attack, draw a card.", 0).expect("lex");
        let rewritten = max_speed_intervening_if_tokens(&tokens);
        assert_eq!(
            render_token_slice(&rewritten),
            "Whenever you attack, if you have max speed, draw a card"
        );
        assert!(
            rewritten
                .iter()
                .any(|token| token.span.line == 0 && token.span.start < 100),
            "the trigger and effect token slices should be carried from the source"
        );
    }

    #[test]
    fn max_speed_trigger_keeps_followup_sentences() {
        let tokens = lex_line(
            "At the beginning of your upkeep, exile the top card of your library. You may play that card this turn.",
            0,
        )
        .expect("lex");
        let rewritten = max_speed_intervening_if_tokens(&tokens);
        assert_eq!(
            render_token_slice(&rewritten),
            "At the beginning of your upkeep, if you have max speed, exile the top card of your library. You may play that card this turn."
        );
    }

    #[test]
    fn alternative_cost_plan_carries_mana_tokens() {
        let cost_tokens = lex_line("{2}{R}", 0).expect("lex");
        let rewritten = alternative_cost_parse_tokens(
            &[
                "If", "you", "dealt", "combat", "damage", "to", "a", "player", "this", "turn",
            ],
            &cost_tokens,
        );
        assert_eq!(
            render_token_slice(&rewritten),
            "If you dealt combat damage to a player this turn, you may pay {2}{R} rather than pay this spell's mana cost."
        );
        assert_eq!(
            rewritten
                .iter()
                .filter(|token| token.kind == TokenKind::ManaGroup)
                .count(),
            2
        );
    }

    #[test]
    fn non_turn_untap_split_returns_both_source_token_slices() {
        let source =
            "Creatures you control get +1/+1. If it's not your turn, untap those creatures.";
        let line = preprocess_document(
            CardDefinitionBuilder::new(crate::CardId::new(), "Untap Split Test"),
            source,
        )
        .expect("line should preprocess")
        .items
        .into_iter()
        .find_map(|item| match item {
            PreprocessedItem::Line(line) => Some(line),
            PreprocessedItem::Metadata(_) => None,
        })
        .expect("expected a preprocessed line");
        let shape = line_rewrite_grammar::parse_non_turn_conditional_untap_tokens(&line.tokens)
            .expect("split sentences");
        assert_eq!(
            render_token_slice(shape.first_sentence_tokens),
            "creatures you control get +1/+1"
        );
        assert_eq!(
            render_token_slice(shape.untap_sentence_tokens),
            "if it's not your turn, untap those creatures."
        );
    }

    #[test]
    fn graveyard_cast_conditions_carry_typed_labels_into_cst() {
        let subtype_document = preprocess_document(
            CardDefinitionBuilder::new(crate::CardId::new(), "Gravecrawler Test")
                .card_types(vec![crate::types::CardType::Creature]),
            "You may cast this card from your graveyard as long as you control a Zombie.",
        )
        .expect("subtype condition should preprocess");
        let subtype_cst = parse_document_cst(&subtype_document, false).expect("subtype cst");
        let [RewriteLineCst::Static(subtype_line)] = subtype_cst.lines.as_slice() else {
            panic!("expected one static subtype-permission line");
        };
        assert_eq!(
            subtype_line.chosen_option,
            Some(ChosenOptionContext::ControlsSubtypePermanent(
                crate::types::Subtype::Zombie
            ))
        );

        let color_document = preprocess_document(
            CardDefinitionBuilder::new(crate::CardId::new(), "Color Pair Test")
                .card_types(vec![crate::types::CardType::Creature]),
            "You may cast this card from your graveyard as long as you control a black or red permanent.",
        )
        .expect("color condition should preprocess");
        let color_cst = parse_document_cst(&color_document, false).expect("color cst");
        let [RewriteLineCst::Static(color_line)] = color_cst.lines.as_slice() else {
            panic!("expected one static color-permission line");
        };
        assert_eq!(
            color_line.chosen_option,
            Some(ChosenOptionContext::ControlsEitherColorPermanent {
                left: crate::Color::Black,
                right: crate::Color::Red,
            })
        );
    }

    #[test]
    fn additional_combat_rewrite_splices_typed_token_span() {
        let document = preprocess_document(
            CardDefinitionBuilder::new(crate::CardId::new(), "Additional Combat Test")
                .card_types(vec![crate::types::CardType::Sorcery]),
            "If it's your main phase, there is an additional combat phase after this phase, followed by an additional main phase.",
        )
        .expect("additional-combat line should preprocess");
        let cst = parse_document_cst(&document, false).expect("additional-combat cst");
        let [RewriteLineCst::Statement(line)] = cst.lines.as_slice() else {
            panic!("expected one additional-combat statement");
        };
        assert_eq!(
            render_token_slice(&line.parse_tokens),
            "After this main phase, there is an additional combat phase followed by an additional main phase."
        );
    }
}

fn partner_with_name_from_line(line: &PreprocessedLine) -> Option<String> {
    let shape = keyword_special_lines::parse_partner_with_name_shape_tokens(&line.tokens)?;
    let name = render_original_text_for_token_slice(line, shape.name_tokens)
        .unwrap_or_else(|| render_token_slice(shape.name_tokens))
        .trim()
        .replace('"', "");
    (!name.is_empty()).then_some(name)
}

pub(super) fn run_combined_static_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(PreprocessedItem::Line(next_line)) = ctx.preprocessed.items.get(ctx.idx + 1) else {
        return Ok(None);
    };
    if !should_try_combined_static_tokens(&ctx.line.tokens, &next_line.tokens) {
        return Ok(None);
    }

    let mut combined_tokens = tokens_without_terminal_period(&ctx.line.tokens).to_vec();
    combined_tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    combined_tokens.extend_from_slice(tokens_without_terminal_period(&next_line.tokens));
    let combined_line = rewrite_line_tokens(ctx.line, &combined_tokens);
    Ok(parse_static_line_cst(&combined_line)?.map(|static_line| {
        LineDispatchResult::single(RewriteLineCst::Static(static_line), ctx.idx + 2)
    }))
}

pub(super) fn run_non_turn_conditional_untap_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(shape) =
        line_rewrite_grammar::parse_non_turn_conditional_untap_tokens(&ctx.line.tokens)
    else {
        return Ok(None);
    };
    let first_line = rewrite_line_tokens(ctx.line, shape.first_sentence_tokens);
    let Some(first_statement) = parse_statement_line_cst(&first_line)? else {
        return Ok(None);
    };

    let second_line = rewrite_line_tokens(ctx.line, shape.untap_sentence_tokens);
    let Some(second_statement) = parse_statement_line_cst(&second_line)? else {
        return Ok(None);
    };

    Ok(Some(LineDispatchResult {
        lines: vec![
            RewriteLineCst::Statement(first_statement),
            RewriteLineCst::Statement(second_statement),
        ],
        next_idx: ctx.idx + 1,
    }))
}

fn is_keyword_action_replacement_static_line(tokens: &[OwnedLexToken]) -> bool {
    parse_static_ability_ast_line_lexed(tokens)
        .ok()
        .flatten()
        .is_some_and(|abilities| {
            abilities.iter().any(|ability| {
                matches!(
                    ability,
                    crate::cards::builders::StaticAbilityAst::Static(static_ability)
                        if static_ability.id()
                            == crate::static_abilities::StaticAbilityId::KeywordActionReplacement
                )
            })
        })
}

pub(super) fn run_statement_probe_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if crate::runtime_backend::families::keyword_static::parse_double_counters_replacement_line(
        &ctx.line.tokens,
    )?
    .is_some()
    {
        return Ok(None);
    }
    if let Some(split_result) =
        parse_labeled_conditional_replacement_sentence_split(ctx.line, ctx.idx)?
    {
        return Ok(Some(split_result));
    }

    let linked_preference = line_grammar::parse_linked_statement_preference(&ctx.line.tokens);
    let static_preference = line_grammar::parse_statement_static_preference(&ctx.line.tokens);

    if (matches!(
        crate::runtime_backend::grammar::structure::classify_statement_line_family_lexed(
            &ctx.line.tokens
        ),
        Some(
            crate::runtime_backend::grammar::structure::StatementLineFamily::Divvy
                | crate::runtime_backend::grammar::structure::StatementLineFamily::PactNextUpkeep
                | crate::runtime_backend::grammar::structure::StatementLineFamily::ExilePlayCostsMore
        )
    ) || linked_preference.is_some()
        || looks_like_statement_line_lexed(ctx.line)
        || should_prefer_statement_before_static_for_nonpermanent_spell(
            ctx.preprocessed,
            &ctx.line.tokens,
        ))
        && !matches!(
            static_preference,
            Some(
                line_grammar::StatementStaticPreference::BlocksAdditionalCreatures
                    | line_grammar::StatementStaticPreference::DrawReplacement
                    | line_grammar::StatementStaticPreference::TokenCreationReplacement
                    | line_grammar::StatementStaticPreference::DiscardOrRedirectReplacement
                    | line_grammar::StatementStaticPreference::FirstEquipCostAlternative
                    | line_grammar::StatementStaticPreference::ConditionalKeywordTypeAddition
            )
        )
        && !is_keyword_action_replacement_static_line(&ctx.line.tokens)
        && let Some(statement_line) = parse_statement_line_cst(ctx.line)?
    {
        let (statement_line, next_idx) = extend_statement_line_with_result_followups(
            &ctx.preprocessed.items,
            ctx.idx,
            statement_line,
        );
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Statement(statement_line),
            next_idx,
        )));
    }
    Ok(None)
}

pub(super) fn run_static_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    match parse_static_line_cst(ctx.line) {
        Ok(static_line) => Ok(static_line.map(|static_line| {
            LineDispatchResult::single(RewriteLineCst::Static(static_line), ctx.idx + 1)
        })),
        Err(err)
            if looks_like_statement_line_lexed(ctx.line)
                && !super::super::grammar::anthem_grants::parse_anthem_modifier_head(
                    &ctx.line.tokens,
                )
                .is_some_and(|head| !head.has_target && !head.temporary) =>
        {
            crate::parse_trace::event(format!(
                "line-family: static-line yielded to statement-like line after error: {err:?}"
            ));
            Ok(None)
        }
        Err(err) => Err(err),
    }
}

pub(super) fn run_statement_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    Ok(parse_statement_line_cst(ctx.line)?.map(|statement_line| {
        let (statement_line, next_idx) = extend_statement_line_with_result_followups(
            &ctx.preprocessed.items,
            ctx.idx,
            statement_line,
        );
        LineDispatchResult::single(RewriteLineCst::Statement(statement_line), next_idx)
    }))
}

pub(super) fn run_leading_unless_statement_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(shape) = line_grammar::parse_leading_unless_line(&ctx.line.tokens) else {
        return Ok(None);
    };
    debug_assert!(shape.condition_tokens.len() >= 2 && !shape.effect_tokens.is_empty());

    let parse_tokens = ctx.line.tokens.clone();
    let parse_groups = vec![parse_tokens.clone()];
    let statement_line = StatementLineCst {
        info: ctx.line.info.clone(),
        text: ctx.line.info.normalized.normalized.clone(),
        parse_tokens,
        parse_groups,
    };
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Statement(statement_line),
        ctx.idx + 1,
    )))
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
    let Some((label, _, body_tokens)) = split_label_prefix_lexed(&suffix_line.tokens) else {
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
    let normalized_cost_tokens =
        normalize_activation_cost_tokens_for_builder(builder, line, cost_tokens.clone())?;
    let cost = parse_activation_cost_tokens_rewrite(&normalized_cost_tokens)?;
    let activated = RewriteLineCst::Activated(ActivatedLineCst {
        info: suffix_line.info.clone(),
        cost,
        cost_parse_tokens: normalized_cost_tokens,
        effect_text,
        effect_parse_tokens,
        presentation_label: Some(label.trim().to_string()),
        chosen_option: None,
    });

    Ok(Some(LineDispatchResult {
        lines: vec![prefix_cst, activated],
        next_idx: idx + 1,
    }))
}

fn parse_keyword_activation_prefix_static_or_rewrite(
    _builder: &CardDefinitionBuilder,
    line: &PreprocessedLine,
    prefix_line: &PreprocessedLine,
    statement_error: Option<CardTextError>,
) -> Result<RewriteLineCst, CardTextError> {
    let static_error = match parse_static_line_cst(prefix_line) {
        Ok(Some(static_line)) => return Ok(RewriteLineCst::Static(static_line)),
        Ok(None) => None,
        Err(err) => Some(err),
    };

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
