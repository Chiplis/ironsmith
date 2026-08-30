use super::super::grammar::keyword_special_lines;
use super::super::grammar::line_families as line_grammar;
use super::super::grammar::line_family_rewrites as line_rewrite_grammar;
use super::line_dispatch::{LineDispatchContext, LineDispatchResult};
use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId};

fn line_family_match(
    ctx: &LineDispatchContext<'_>,
    result: LineDispatchResult,
) -> ParseOutcome<LineDispatchResult> {
    ParseOutcome::matched(result, span_from_tokens(&ctx.line.tokens))
}

fn line_family_error(
    ctx: &LineDispatchContext<'_>,
    rule: RuleId,
    error: CardTextError,
) -> ParseOutcome<LineDispatchResult> {
    ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
        rule,
        span_from_tokens(&ctx.line.tokens),
        error,
    ))
}

macro_rules! line_family_try {
    ($ctx:expr, $rule:expr, $result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return line_family_error($ctx, $rule, error),
        }
    };
}

macro_rules! line_family_claimed {
    ($rule:expr, $outcome:expr) => {
        match $outcome {
            ParseOutcome::Match(_) => true,
            ParseOutcome::NoMatch => false,
            ParseOutcome::Error(diagnostic) => {
                return ParseOutcome::Error(diagnostic.within($rule));
            }
        }
    };
}

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

fn is_direct_alternative_cost_keyword_line(line: &PreprocessedLine) -> Result<bool, CardTextError> {
    Ok(super::super::grammar::shared_util::alternative_cost_lines::
        parse_you_may_rather_than_spell_cost(&line.tokens, &line.info.raw_line)?
        .is_some())
}

fn parse_static_line_from_tokens(
    line: &PreprocessedLine,
    parse_tokens: Vec<OwnedLexToken>,
) -> Result<Option<RecognizedStaticLine>, CardTextError> {
    recognize_static_line(&rewrite_line_tokens(line, &parse_tokens))
}

pub(super) fn run_trailing_keyword_activation_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("trailing-keyword-activation");
    if let Some(result) = sticker_sheet_ticket_marker_result(ctx) {
        return line_family_match(ctx, result);
    }
    match try_parse_trailing_keyword_activation_dispatch(
        &ctx.preprocessed.builder,
        ctx.idx,
        ctx.line,
    ) {
        Ok(Some(result)) => line_family_match(ctx, result),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => line_family_error(ctx, rule, error),
    }
}

pub(super) fn run_labeled_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("labeled-line");
    if let Some(result) = sticker_sheet_ticket_marker_result(ctx) {
        return line_family_match(ctx, result);
    }
    if split_label_prefix_lexed(&ctx.line.info.source_tokens).is_none()
        && split_label_prefix_lexed(&ctx.line.tokens).is_none()
    {
        return ParseOutcome::NoMatch;
    }

    match try_parse_labeled_line_dispatch(
        ctx.preprocessed,
        ctx.idx,
        ctx.line,
        ctx.allow_unsupported,
    ) {
        Ok(Some(result)) => line_family_match(ctx, result),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => line_family_error(ctx, rule, error),
    }
}

pub(super) fn sticker_sheet_ticket_marker_result(
    ctx: &LineDispatchContext<'_>,
) -> Option<LineDispatchResult> {
    is_sticker_sheet_ticket_marker_line(ctx).then(|| {
        LineDispatchResult::single(
            RecognizedLine::Static(sticker_sheet_ticket_marker_static_line(ctx)),
            ctx.idx + 1,
        )
    })
}

fn sticker_sheet_ticket_marker_static_line(ctx: &LineDispatchContext<'_>) -> RecognizedStaticLine {
    let marker = render_token_slice(&ctx.line.tokens).trim().to_string();
    RecognizedStaticLine {
        info: ctx.line.info.clone(),
        parse_tokens: ctx.line.tokens.clone(),
        chosen_option: None,
        // Ticket-threshold rows on a sticker sheet are presentation entries,
        // not abilities of the sticker-sheet card itself. Lower the complete
        // row directly so trigger-shaped and keyword-shaped bodies cannot be
        // reclassified after the `Stickers` metadata line has identified it.
        parsed: Some(Box::new(LineAst::StaticAbility(
            crate::cards::builders::StaticAbilityAst::KeywordAction(
                crate::payload::KeywordAction::StaticMarkerText(marker),
            ),
        ))),
    }
}

fn is_sticker_sheet_ticket_marker_line(ctx: &LineDispatchContext<'_>) -> bool {
    let is_sticker_sheet = ctx.preprocessed.items.iter().any(|item| {
        matches!(
            item,
            PreprocessedItem::Metadata(metadata)
                if matches!(
                    &metadata.value,
                    crate::model::facts::MetadataLine::TypeLine(value)
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
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("triggered-line");
    if let Some(result) = sticker_sheet_ticket_marker_result(ctx) {
        return line_family_match(ctx, result);
    }
    if !line_starts_with_trigger_intro_tokens(&ctx.line.tokens) {
        return ParseOutcome::NoMatch;
    }
    if let Some(mut triggered) = recognize_simple_source_entry_face_down_exile_trigger(ctx.line) {
        if let Err(error) = restore_authored_named_source_trigger_subject(
            &ctx.preprocessed.builder,
            ctx.line,
            &ctx.line.info.raw_line,
            &mut triggered,
        ) {
            return line_family_error(ctx, rule, error);
        }
        let (triggered, next_idx) = extend_triggered_line_with_result_followups(
            &ctx.preprocessed.items,
            ctx.idx,
            triggered,
        );
        return line_family_match(
            ctx,
            LineDispatchResult::single(RecognizedLine::Triggered(triggered), next_idx),
        );
    }
    match try_parse_triggered_line_dispatch(
        ctx.preprocessed,
        ctx.idx,
        ctx.line,
        ctx.allow_unsupported,
    ) {
        Ok(Some(result)) => line_family_match(ctx, result),
        Ok(None) => ParseOutcome::NoMatch,
        Err(error) => line_family_error(ctx, rule, error),
    }
}

fn recognize_simple_source_entry_face_down_exile_trigger(
    line: &PreprocessedLine,
) -> Option<RecognizedTriggeredLine> {
    let (trigger_tokens, effect_tokens) = grammar::split_lexed_once_on_comma(&line.tokens)?;
    let trigger_words = crate::lexer::parser_token_word_refs(trigger_tokens);
    if !matches!(
        trigger_words.as_slice(),
        ["when", "this", _, "enters"] | ["when", "this", _, "enters", "the", "battlefield"]
    ) {
        return None;
    }
    let effect_words = crate::lexer::parser_token_word_refs(effect_tokens);
    let complete_face_down_exile = match effect_words.as_slice() {
        [
            "exile",
            "the",
            "top",
            "card",
            "of",
            "your",
            "library",
            "face",
            "down",
        ] => true,
        [
            "exile",
            "the",
            "top",
            count,
            "cards",
            "of",
            "your",
            "library",
            "face",
            "down",
        ] => crate::util::parse_number_word_i32(count).is_some_and(|count| count > 0),
        _ => false,
    };
    if !complete_face_down_exile || trigger_tokens.len() <= 1 {
        return None;
    }
    render_triggered_split_candidate(&trigger_tokens[1..], effect_tokens, None, None)
        .map(|candidate| candidate.into_recognized_line(line, &line.tokens))
}

pub(super) fn run_championed_with_this_trigger_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("championed-with-this-trigger-line");
    let Some(shape) = line_grammar::parse_championed_with_this_trigger(&ctx.line.tokens) else {
        return ParseOutcome::NoMatch;
    };
    let mut triggered_tokens = synthetic_word_tokens(&["When", "this", "creature", "enters"]);
    triggered_tokens.push(OwnedLexToken::comma(TextSpan::synthetic()));
    triggered_tokens
        .extend_from_slice(line_grammar::parse_visible_line_tokens(shape.effect_tokens));
    let triggered_line = rewrite_line_tokens(ctx.line, &triggered_tokens);
    let triggered = line_family_try!(ctx, rule, recognize_triggered_line(&triggered_line));
    line_family_match(
        ctx,
        LineDispatchResult::single(RecognizedLine::Triggered(triggered), ctx.idx + 1),
    )
}

pub(super) fn run_max_speed_labeled_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("max-speed-labeled-line");
    let Some(shape) = line_grammar::parse_max_speed_line(&ctx.line.tokens) else {
        return ParseOutcome::NoMatch;
    };

    let body_tokens = shape.body_tokens;
    if body_tokens.is_empty() {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "max-speed label missing ability body: '{}'",
                ctx.line.info.raw_line
            )),
        );
    }

    let body_line = rewrite_line_tokens(ctx.line, body_tokens);
    if shape.trigger_intro.is_some() {
        let triggered_tokens = max_speed_intervening_if_tokens(&body_line.tokens);
        let triggered_line = rewrite_line_tokens(ctx.line, &triggered_tokens);
        let triggered = line_family_try!(ctx, rule, recognize_triggered_line(&triggered_line));
        return line_family_match(
            ctx,
            LineDispatchResult::single(RecognizedLine::Triggered(triggered), ctx.idx + 1),
        );
    }

    if let Some((cost_tokens, effect_parse_tokens)) =
        split_activation_text_tokens_lexed(&body_line.tokens)
    {
        let normalized_cost_tokens = line_family_try!(
            ctx,
            rule,
            normalize_activation_cost_tokens_for_builder(
                &ctx.preprocessed.builder,
                ctx.line,
                cost_tokens.clone(),
            )
        );
        match parse_activation_cost_tokens_rewrite(&normalized_cost_tokens) {
            Ok(cost) => {
                let effect_parse_tokens = line_family_try!(
                    ctx,
                    rule,
                    normalize_activation_effect_tokens_for_builder(
                        &ctx.preprocessed.builder,
                        ctx.line,
                        &effect_parse_tokens,
                    )
                );
                return line_family_match(
                    ctx,
                    LineDispatchResult::single(
                        RecognizedLine::Activated(RecognizedActivatedLine {
                            info: ctx.line.info.clone(),
                            cost,
                            cost_parse_tokens: normalized_cost_tokens,
                            effect_parse_tokens,
                            presentation: activated_presentation_from_preprocessed_line(ctx.line),
                            chosen_option: Some(ChosenOptionContext::MaxSpeed),
                        }),
                        ctx.idx + 1,
                    ),
                );
            }
            Err(err) if looks_like_activation_cost_prefix(&cost_tokens) => {
                return line_family_error(ctx, rule, err);
            }
            Err(_) => {}
        }
    }

    let Some(static_recognized) = line_family_try!(ctx, rule, recognize_static_line(&body_line))
    else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower max-speed labeled line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };
    line_family_match(
        ctx,
        LineDispatchResult::single(
            RecognizedLine::Static(RecognizedStaticLine {
                chosen_option: Some(ChosenOptionContext::MaxSpeed),
                ..static_recognized
            }),
            ctx.idx + 1,
        ),
    )
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
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("start-your-engines-line");
    if line_grammar::parse_simple_document_line(&ctx.line.tokens)
        != Some(line_grammar::SimpleDocumentLineShape::StartYourEngines)
    {
        return ParseOutcome::NoMatch;
    }

    let start_tokens = synthetic_word_tokens(&["start", "your", "engines"]);
    let Some(start_static) = line_family_try!(
        ctx,
        rule,
        parse_static_line_from_tokens(ctx.line, start_tokens)
    ) else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower start-your-engines keyword line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };

    line_family_match(
        ctx,
        LineDispatchResult::single(RecognizedLine::Static(start_static), ctx.idx + 1),
    )
}

pub(super) fn run_draft_rule_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    if line_grammar::parse_draft_rule_line(&ctx.line.tokens).is_none() {
        return ParseOutcome::NoMatch;
    }

    line_family_match(
        ctx,
        LineDispatchResult::single(
            RecognizedLine::Static(RecognizedStaticLine {
                info: ctx.line.info.clone(),
                parse_tokens: ctx.line.tokens.clone(),
                chosen_option: None,
                parsed: None,
            }),
            ctx.idx + 1,
        ),
    )
}

pub(super) fn run_learn_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    if line_grammar::parse_simple_document_line(&ctx.line.tokens)
        != Some(line_grammar::SimpleDocumentLineShape::Learn)
    {
        return ParseOutcome::NoMatch;
    }

    let learn_tokens = line_grammar::parse_visible_line_tokens(&ctx.line.tokens).to_vec();
    line_family_match(
        ctx,
        LineDispatchResult::single(
            RecognizedLine::Statement(RecognizedStatementLine {
                info: ctx.line.info.clone(),
                text: "learn".to_string(),
                parse_tokens: learn_tokens.clone(),
                parse_groups: vec![learn_tokens],
                parsed_effects: None,
            }),
            ctx.idx + 1,
        ),
    )
}

pub(super) fn run_split_top_and_face_down_look_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("split-top-and-face-down-look-line");
    if line_grammar::parse_simple_document_line(&ctx.line.tokens)
        != Some(line_grammar::SimpleDocumentLineShape::SplitTopAndFaceDownLook)
    {
        return ParseOutcome::NoMatch;
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

    let Some(top_card_static) = line_family_try!(
        ctx,
        rule,
        parse_static_line_from_tokens(ctx.line, top_card_tokens)
    ) else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower split top-card line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };
    let Some(face_down_static) = line_family_try!(
        ctx,
        rule,
        parse_static_line_from_tokens(ctx.line, face_down_tokens)
    ) else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower split face-down line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };

    line_family_match(
        ctx,
        LineDispatchResult {
            lines: vec![
                RecognizedLine::Static(top_card_static),
                RecognizedLine::Static(face_down_static),
            ],
            next_idx: ctx.idx + 1,
        },
    )
}

pub(super) fn run_split_top_look_and_top_land_play_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("split-top-look-and-top-land-play-line");
    if line_grammar::parse_special_line(&ctx.line.tokens)
        != Some(line_grammar::SpecialLineShape::SplitTopLookAndLandPlay)
    {
        return ParseOutcome::NoMatch;
    }

    let top_card_tokens = synthetic_sentence_tokens(&[
        "you", "may", "look", "at", "the", "top", "card", "of", "your", "library", "any", "time",
    ]);
    let play_lands_tokens = synthetic_sentence_tokens(&[
        "you", "may", "play", "lands", "from", "the", "top", "of", "your", "library",
    ]);

    let Some(top_card_static) = line_family_try!(
        ctx,
        rule,
        parse_static_line_from_tokens(ctx.line, top_card_tokens)
    ) else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower split top-card look line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };
    let Some(play_lands_static) = line_family_try!(
        ctx,
        rule,
        parse_static_line_from_tokens(ctx.line, play_lands_tokens)
    ) else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower split top-library land-play line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };

    line_family_match(
        ctx,
        LineDispatchResult {
            lines: vec![
                RecognizedLine::Static(top_card_static),
                RecognizedLine::Static(play_lands_static),
            ],
            next_idx: ctx.idx + 1,
        },
    )
}

pub(super) fn run_assign_damage_as_unblocked_enchanted_creature_controller_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    if line_grammar::parse_special_line(&ctx.line.tokens)
        != Some(line_grammar::SpecialLineShape::AssignDamageAsUnblockedEnchanted)
    {
        return ParseOutcome::NoMatch;
    }

    let static_recognized = RecognizedStaticLine {
        info: ctx.line.info.clone(),
        parse_tokens: ctx.line.tokens.clone(),
        chosen_option: None,
        parsed: Some(Box::new(LineAst::StaticAbility(
            crate::cards::builders::StaticAbilityAst::AttachedStaticAbilityGrant {
                ability: Box::new(crate::cards::builders::StaticAbilityAst::Static(
                    crate::model::CompilerStaticAbilityCore::may_assign_damage_as_unblocked(),
                )),
                display: "enchanted creature has \"You may have this creature assign its combat damage as though it weren't blocked.\"".to_string(),
                condition: None,
            },
        ))),
    };

    line_family_match(
        ctx,
        LineDispatchResult::single(RecognizedLine::Static(static_recognized), ctx.idx + 1),
    )
}

pub(super) fn run_graveyard_cast_control_condition_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("graveyard-cast-control-condition");
    let Some(condition) =
        line_rewrite_grammar::parse_graveyard_cast_control_condition_tokens(&ctx.line.tokens)
    else {
        return ParseOutcome::NoMatch;
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
    let Some(mut static_recognized) = line_family_try!(
        ctx,
        rule,
        parse_static_line_from_tokens(ctx.line, permission_tokens)
    ) else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower graveyard-cast control condition line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };
    static_recognized.chosen_option = Some(match condition {
        line_rewrite_grammar::GraveyardCastControlCondition::Subtype(subtype) => {
            ChosenOptionContext::ControlsSubtypePermanent(subtype)
        }
        line_rewrite_grammar::GraveyardCastControlCondition::ColorPair(left, right) => {
            ChosenOptionContext::ControlsEitherColorPermanent { left, right }
        }
    });

    line_family_match(
        ctx,
        LineDispatchResult::single(RecognizedLine::Static(static_recognized), ctx.idx + 1),
    )
}

pub(super) fn run_graveyard_or_exile_cast_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("graveyard-or-exile-cast");
    if line_grammar::parse_special_line(&ctx.line.tokens)
        != Some(line_grammar::SpecialLineShape::GraveyardOrExileCast)
    {
        return ParseOutcome::NoMatch;
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

    let Some(graveyard_static) = line_family_try!(
        ctx,
        rule,
        parse_static_line_from_tokens(ctx.line, graveyard_tokens)
    ) else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower graveyard-or-exile cast line graveyard half: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };
    let Some(exile_static) = line_family_try!(
        ctx,
        rule,
        parse_static_line_from_tokens(ctx.line, exile_tokens)
    ) else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower graveyard-or-exile cast line exile half: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };

    line_family_match(
        ctx,
        LineDispatchResult {
            lines: vec![
                RecognizedLine::Static(graveyard_static),
                RecognizedLine::Static(exile_static),
            ],
            next_idx: ctx.idx + 1,
        },
    )
}

pub(super) fn run_champion_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("champion-line");
    let Some(shape) = line_grammar::parse_champion_line(&ctx.line.tokens) else {
        return ParseOutcome::NoMatch;
    };
    let filter_tokens = shape.filter_tokens;
    if filter_tokens.is_empty() {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "champion keyword missing object filter: '{}'",
                ctx.line.info.raw_line
            )),
        );
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
    let triggered = line_family_try!(ctx, rule, recognize_triggered_line(&triggered_line));
    line_family_match(
        ctx,
        LineDispatchResult::single(RecognizedLine::Triggered(triggered), ctx.idx + 1),
    )
}

pub(super) fn run_station_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("station-line");
    let Some(station_shape) =
        line_grammar::parse_station_keyword_line(&ctx.line.tokens, &ctx.line.info.source_tokens)
    else {
        return ParseOutcome::NoMatch;
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
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower station keyword line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };
    let normalized_cost_tokens = line_family_try!(
        ctx,
        rule,
        normalize_activation_cost_tokens_for_builder(
            &ctx.preprocessed.builder,
            ctx.line,
            cost_tokens.clone(),
        )
    );
    let cost = line_family_try!(
        ctx,
        rule,
        parse_activation_cost_tokens_rewrite(&normalized_cost_tokens)
    );
    let effect_parse_tokens = line_family_try!(
        ctx,
        rule,
        normalize_activation_effect_tokens_for_builder(
            &ctx.preprocessed.builder,
            ctx.line,
            &effect_parse_tokens,
        )
    );
    let mut lines = vec![RecognizedLine::Activated(RecognizedActivatedLine {
        info: ctx.line.info.clone(),
        cost,
        cost_parse_tokens: normalized_cost_tokens,
        effect_parse_tokens,
        presentation: None,
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
        let chosen_option = ChosenOptionContext::StationThresholdSupport(threshold);
        let power = pt.power.base_value();
        let toughness = pt.toughness.base_value();
        for parse_tokens in station_creature_support_parse_tokens(power, toughness) {
            let Some(static_recognized) = line_family_try!(
                ctx,
                rule,
                parse_static_line_from_tokens(ctx.line, parse_tokens)
            ) else {
                return line_family_error(
                    ctx,
                    rule,
                    CardTextError::ParseError(format!(
                        "parser could not lower station reminder threshold support: '{}'",
                        ctx.line.info.raw_line
                    )),
                );
            };
            lines.push(RecognizedLine::Static(RecognizedStaticLine {
                chosen_option: Some(chosen_option.clone()),
                ..static_recognized
            }));
        }
    }

    line_family_match(
        ctx,
        LineDispatchResult {
            lines,
            next_idx: ctx.idx + 1,
        },
    )
}

pub(super) fn run_station_threshold_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("station-threshold-line");
    let Some(shape) = line_grammar::parse_station_threshold_line(&ctx.line.tokens) else {
        return ParseOutcome::NoMatch;
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
            let Some(static_recognized) = line_family_try!(
                ctx,
                rule,
                parse_static_line_from_tokens(ctx.line, parse_tokens)
            ) else {
                return line_family_error(
                    ctx,
                    rule,
                    CardTextError::ParseError(format!(
                        "parser could not lower station creature threshold support: '{}'",
                        ctx.line.info.raw_line
                    )),
                );
            };
            lines.push(RecognizedLine::Static(RecognizedStaticLine {
                chosen_option: Some(ChosenOptionContext::StationThresholdSupport(threshold)),
                ..static_recognized
            }));
        }
    }

    let body_line = rewrite_line_tokens(ctx.line, &body_tokens);
    if shape.trigger_intro.is_some() {
        let mut triggered = line_family_try!(ctx, rule, recognize_triggered_line(&body_line));
        triggered.presentation = Some(PresentationLabel::AbilityWord(format!(
            "{}{threshold}",
            ironsmith_core::static_ability_model::STATION_THRESHOLD_STATIC_LABEL_PREFIX
        )));
        triggered.chosen_option = Some(chosen_option);
        lines.push(RecognizedLine::Triggered(triggered));
        return line_family_match(
            ctx,
            LineDispatchResult {
                lines,
                next_idx: ctx.idx + 1,
            },
        );
    }

    if let Some((cost_tokens, effect_parse_tokens)) =
        split_activation_text_tokens_lexed(&body_line.tokens)
    {
        let normalized_cost_tokens = line_family_try!(
            ctx,
            rule,
            normalize_activation_cost_tokens_for_builder(
                &ctx.preprocessed.builder,
                ctx.line,
                cost_tokens.clone(),
            )
        );
        let cost = line_family_try!(
            ctx,
            rule,
            parse_activation_cost_tokens_rewrite(&normalized_cost_tokens)
        );
        let effect_parse_tokens = line_family_try!(
            ctx,
            rule,
            normalize_activation_effect_tokens_for_builder(
                &ctx.preprocessed.builder,
                ctx.line,
                &effect_parse_tokens,
            )
        );
        lines.push(RecognizedLine::Activated(RecognizedActivatedLine {
            info: ctx.line.info.clone(),
            cost,
            cost_parse_tokens: normalized_cost_tokens,
            effect_parse_tokens,
            presentation: None,
            chosen_option: Some(chosen_option),
        }));
        return line_family_match(
            ctx,
            LineDispatchResult {
                lines,
                next_idx: ctx.idx + 1,
            },
        );
    }

    let Some(static_recognized) = line_family_try!(ctx, rule, recognize_static_line(&body_line))
    else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower station threshold line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };
    lines.push(RecognizedLine::Static(RecognizedStaticLine {
        chosen_option: Some(chosen_option),
        ..static_recognized
    }));
    line_family_match(
        ctx,
        LineDispatchResult {
            lines,
            next_idx: ctx.idx + 1,
        },
    )
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
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("partner-with-keyword-line");
    let Some(partner_name) = partner_with_name_from_line(ctx.line) else {
        return ParseOutcome::NoMatch;
    };

    let partner_static = RecognizedStaticLine {
        info: ctx.line.info.clone(),
        // Source-name normalization can rewrite a shared leading word in the
        // partner's proper name (for example, Soulblade Corrupter on
        // Soulblade Renewer) to "this creature". Recover the authored name
        // from the retained source tokens before lowering the keyword line.
        parse_tokens: line_family_try!(
            ctx,
            rule,
            lex_line(
                &format!("Partner with {partner_name}"),
                ctx.line.info.line_index,
            )
        ),
        chosen_option: None,
        parsed: None,
    };

    line_family_match(
        ctx,
        LineDispatchResult::single(RecognizedLine::Static(partner_static), ctx.idx + 1),
    )
}

pub(super) fn run_partner_variant_keyword_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    if line_grammar::parse_partner_variant(&ctx.line.tokens).is_none() {
        return ParseOutcome::NoMatch;
    }

    line_family_match(
        ctx,
        LineDispatchResult::single(
            RecognizedLine::Static(RecognizedStaticLine {
                info: ctx.line.info.clone(),
                parse_tokens: ctx.line.tokens.clone(),
                chosen_option: None,
                parsed: None,
            }),
            ctx.idx + 1,
        ),
    )
}

pub(super) fn run_escape_enters_with_counter_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("escape-enters-with-counter-line");
    if line_grammar::parse_escape_enters_with_line(&ctx.line.tokens).is_none() {
        return ParseOutcome::NoMatch;
    }
    match line_family_try!(ctx, rule, recognize_static_line(ctx.line)) {
        Some(static_recognized) => line_family_match(
            ctx,
            LineDispatchResult::single(RecognizedLine::Static(static_recognized), ctx.idx + 1),
        ),
        None => ParseOutcome::NoMatch,
    }
}

pub(super) fn run_surge_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("surge-line");
    let Some(shape) = line_grammar::parse_surge_line(&ctx.line.tokens) else {
        return ParseOutcome::NoMatch;
    };
    let cost_tokens = shape.cost_tokens;
    if cost_tokens.is_empty() {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "surge keyword missing cost: '{}'",
                ctx.line.info.raw_line
            )),
        );
    }

    let parse_tokens = alternative_cost_parse_tokens(
        &["If", "you've", "cast", "another", "spell", "this", "turn"],
        cost_tokens,
    );
    let alternative_line = rewrite_line_tokens(ctx.line, &parse_tokens);
    let Some(keyword) = line_family_try!(ctx, rule, recognize_keyword_line(&alternative_line))
    else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower surge keyword line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };
    line_family_match(
        ctx,
        LineDispatchResult::single(RecognizedLine::Keyword(keyword), ctx.idx + 1),
    )
}

pub(super) fn run_freerunning_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("freerunning-line");
    let Some(shape) = line_grammar::parse_freerunning_line(&ctx.line.tokens) else {
        return ParseOutcome::NoMatch;
    };
    let cost_tokens = shape.cost_tokens;
    if cost_tokens.is_empty() {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "freerunning keyword missing cost: '{}'",
                ctx.line.info.raw_line
            )),
        );
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
    let Some(keyword) = line_family_try!(ctx, rule, recognize_keyword_line(&alternative_line))
    else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower freerunning keyword line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };
    line_family_match(
        ctx,
        LineDispatchResult::single(RecognizedLine::Keyword(keyword), ctx.idx + 1),
    )
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
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("keyword-line");
    if let Some(result) = sticker_sheet_ticket_marker_result(ctx) {
        return line_family_match(ctx, result);
    }
    if super::super::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(
        &ctx.line.tokens,
    )
    .is_some()
    {
        return ParseOutcome::NoMatch;
    }

    if let Some(action) = crate::keyword_static::parse_dynamic_firebending_with_source(
        &ctx.line.tokens,
        Some(ctx.parse.source().card_name.as_str()),
    ) {
        return line_family_match(
            ctx,
            LineDispatchResult::single(
                RecognizedLine::Static(RecognizedStaticLine {
                    info: ctx.line.info.clone(),
                    parse_tokens: ctx.line.tokens.clone(),
                    chosen_option: None,
                    parsed: Some(Box::new(LineAst::Abilities(vec![action]))),
                }),
                ctx.idx + 1,
            ),
        );
    }

    if matches!(
        parse_ability_line_lexed(&ctx.line.tokens).as_deref(),
        Some([crate::cards::builders::KeywordAction::CumulativeUpkeep { .. }])
    ) {
        return line_family_match(
            ctx,
            LineDispatchResult::single(
                RecognizedLine::Static(RecognizedStaticLine {
                    info: ctx.line.info.clone(),
                    parse_tokens: ctx.line.tokens.clone(),
                    chosen_option: None,
                    parsed: None,
                }),
                ctx.idx + 1,
            ),
        );
    }
    if let Some(split_lines) =
        line_family_try!(ctx, rule, split_same_line_and_or_kicker_keywords(ctx.line))
    {
        return line_family_match(
            ctx,
            LineDispatchResult {
                lines: split_lines,
                next_idx: ctx.idx + 1,
            },
        );
    }
    if let Some(split_lines) = line_family_try!(ctx, rule, split_kicker_x_minimum_line(ctx.line)) {
        return line_family_match(
            ctx,
            LineDispatchResult {
                lines: split_lines,
                next_idx: ctx.idx + 1,
            },
        );
    }

    match line_family_try!(ctx, rule, recognize_keyword_line(ctx.line)) {
        Some(keyword_line) => line_family_match(
            ctx,
            LineDispatchResult::single(RecognizedLine::Keyword(keyword_line), ctx.idx + 1),
        ),
        None => ParseOutcome::NoMatch,
    }
}

fn split_kicker_x_minimum_line(
    line: &PreprocessedLine,
) -> Result<Option<Vec<RecognizedLine>>, CardTextError> {
    let Some(first_period) = crate::slice_primitives::select_position(&line.tokens, |token| {
        token.kind == TokenKind::Period
    }) else {
        return Ok(None);
    };
    let kicker_tokens = &line.tokens[..=first_period];
    if !kicker_tokens
        .first()
        .is_some_and(|token| token.is_word("kicker"))
        || !kicker_tokens.iter().any(|token| {
            token.kind == TokenKind::ManaGroup && token.slice.eq_ignore_ascii_case("{X}")
        })
    {
        return Ok(None);
    }

    let suffix_end =
        crate::slice_primitives::select_position(&line.tokens[first_period + 1..], |token| {
            token.kind == TokenKind::LParen
        })
        .map_or(line.tokens.len(), |offset| first_period + 1 + offset);
    let minimum_tokens = trim_lexed_commas(&line.tokens[first_period + 1..suffix_end]);
    let minimum_words = token_word_refs(minimum_tokens);
    if !crate::word_primitives::parse_choice_sequence_complete(
        &minimum_words,
        &[&["x"], &["cant", "can't"], &["be"], &["0"]],
    ) {
        return Ok(None);
    }

    let kicker_line = rewrite_line_tokens(line, kicker_tokens);
    let Some(keyword) = recognize_keyword_line(&kicker_line)? else {
        return Ok(None);
    };
    let Some(static_line) = parse_static_line_from_tokens(line, minimum_tokens.to_vec())? else {
        return Ok(None);
    };
    Ok(Some(vec![
        RecognizedLine::Keyword(keyword),
        RecognizedLine::Static(static_line),
    ]))
}

fn split_same_line_and_or_kicker_keywords(
    line: &PreprocessedLine,
) -> Result<Option<Vec<RecognizedLine>>, CardTextError> {
    let Some(shape) = line_grammar::parse_kicker_branches(&line.tokens) else {
        return Ok(None);
    };

    let branches = [shape.first_cost, shape.second_cost];

    let mut lines = Vec::new();
    for branch in branches {
        let parsed_cost = parse_activation_cost_tokens_rewrite(branch)?;
        let compiler_cost = crate::semantic_assembly::assemble_activation_cost(&parsed_cost)?;
        let compiler_cost = compiler_cost.to_core_total_cost();
        let cost_text = compiler_cost
            .mana_cost()
            .map(|cost| cost.to_oracle())
            .unwrap_or_else(|| compiler_cost.display());
        let label = format!("Kicker {cost_text}");
        let raw = label.clone();
        let mut tokens = Vec::with_capacity(branch.len() + 1);
        tokens.push(OwnedLexToken::word("kicker", TextSpan::synthetic()));
        tokens.extend_from_slice(branch);
        let rewritten = rewrite_line_tokens(line, &tokens);
        let mut keyword = recognize_keyword_line(&rewritten)?.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "parser could not split same-line kicker cost '{raw}'"
            ))
        })?;
        keyword
            .payload
            .set_kicker_label(label)
            .map_err(CardTextError::InvariantViolation)?;
        lines.push(RecognizedLine::Keyword(keyword));
    }

    Ok(Some(lines))
}

pub(super) fn run_additional_combat_after_this_phase_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("additional-combat-after-this-phase");
    let Some(shape) =
        line_rewrite_grammar::parse_additional_combat_rewrite_tokens(&ctx.line.tokens)
    else {
        return ParseOutcome::NoMatch;
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
    let Some(statement_line) =
        line_family_try!(ctx, rule, recognize_statement_line(&rewritten_line))
    else {
        return line_family_error(
            ctx,
            rule,
            CardTextError::ParseError(format!(
                "parser could not lower additional-combat-after-this-phase line: '{}'",
                ctx.line.info.raw_line
            )),
        );
    };
    line_family_match(
        ctx,
        LineDispatchResult::single(RecognizedLine::Statement(statement_line), ctx.idx + 1),
    )
}

pub(super) fn run_ward_or_echo_static_prefix_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    if !is_ward_or_echo_static_prefix_line_lexed(&ctx.line.tokens) {
        return ParseOutcome::NoMatch;
    }
    line_family_match(
        ctx,
        LineDispatchResult::single(
            RecognizedLine::Static(RecognizedStaticLine {
                info: ctx.line.info.clone(),
                parse_tokens: rewrite_keyword_dash_parse_tokens(&ctx.line.tokens),
                chosen_option: None,
                parsed: None,
            }),
            ctx.idx + 1,
        ),
    )
}

pub(super) fn run_activation_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("activated-line");
    if (!line_starts_with_lparen_token(ctx.line) || is_fully_parenthetical_line(ctx.line))
        && let Some((cost_tokens, effect_parse_tokens)) = split_label_prefix_lexed(&ctx.line.tokens)
            .filter(|(label, _, _)| is_named_ability_label(label.as_str()))
            .and_then(|(_, _, body_tokens)| split_activation_text_tokens_lexed(body_tokens))
            .or_else(|| split_activation_text_tokens_lexed(&ctx.line.tokens))
    {
        let normalized_cost_tokens = line_family_try!(
            ctx,
            rule,
            normalize_activation_cost_tokens_for_builder(
                &ctx.preprocessed.builder,
                ctx.line,
                cost_tokens.clone(),
            )
        );
        match parse_activation_cost_tokens_rewrite(&normalized_cost_tokens) {
            Ok(cost) => {
                let effect_parse_tokens = line_family_try!(
                    ctx,
                    rule,
                    normalize_activation_effect_tokens_for_builder(
                        &ctx.preprocessed.builder,
                        ctx.line,
                        &effect_parse_tokens,
                    )
                );
                let activated = RecognizedActivatedLine {
                    info: ctx.line.info.clone(),
                    cost,
                    cost_parse_tokens: normalized_cost_tokens,
                    effect_parse_tokens,
                    presentation: activated_presentation_from_preprocessed_line(ctx.line),
                    chosen_option: None,
                };
                let (activated, next_idx) = extend_activated_line_with_result_followups(
                    &ctx.preprocessed.items,
                    ctx.idx,
                    activated,
                );
                return line_family_match(
                    ctx,
                    LineDispatchResult::single(RecognizedLine::Activated(activated), next_idx),
                );
            }
            Err(err) if looks_like_activation_cost_prefix(&cost_tokens) => {
                return line_family_error(ctx, rule, err);
            }
            Err(_) => {}
        }
    }

    ParseOutcome::NoMatch
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

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

        let shared_prefix_document = preprocess_document(
            CardDefinitionBuilder::new(crate::CardId::new(), "Soulblade Renewer"),
            "Partner with Soulblade Corrupter (When this creature enters, target player may put Soulblade Corrupter into their hand from their library, then shuffle.)",
        )
        .expect("shared-prefix partner line should preprocess");
        let shared_prefix_recognized = recognize_document(&shared_prefix_document, false)
            .expect("shared-prefix partner recognized");
        let [RecognizedLine::Static(shared_prefix_line)] =
            shared_prefix_recognized.lines.as_slice()
        else {
            panic!("expected one static partner-with line");
        };
        assert_eq!(
            render_token_slice(&shared_prefix_line.parse_tokens),
            "Partner with Soulblade Corrupter"
        );
    }

    #[test]
    fn typed_line_family_migration_routes_simple_and_unless_shapes_into_recognized() {
        let learn = preprocess_document(
            CardDefinitionBuilder::new(crate::CardId::new(), "Learn Test"),
            "Learn.",
        )
        .expect("learn should preprocess");
        let learn_recognized = recognize_document(&learn, false).expect("learn recognized");
        assert!(matches!(
            learn_recognized.lines.as_slice(),
            [RecognizedLine::Statement(_)]
        ));

        let unless = preprocess_document(
            CardDefinitionBuilder::new(crate::CardId::new(), "Unless Test"),
            "Unless you pay {2}, sacrifice this permanent.",
        )
        .expect("unless should preprocess");
        let unless_recognized = recognize_document(&unless, false).expect("unless recognized");
        let [RecognizedLine::Statement(line)] = unless_recognized.lines.as_slice() else {
            panic!("expected a statement recognized form for a leading-unless line");
        };
        assert_eq!(
            render_token_slice(&line.parse_tokens),
            "unless you pay {2}, sacrifice this permanent."
        );
    }

    #[test]
    fn expanded_removed_draft_ladder_yields_to_typed_static_dispatch() {
        let oracle = "If you removed a creature card with flying from the draft with cards named Animus of Predation, this creature has flying. The same is true for first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, reach, and vigilance.";
        let builder = CardDefinitionBuilder::new(crate::CardId::new(), "Animus of Predation")
            .card_types(vec![crate::types::CardType::Creature]);
        let document = preprocess_document(builder, oracle)
            .expect("the removed-from-draft ladder should preprocess");
        let recognized = recognize_document(&document, false)
            .expect("the removed-from-draft ladder should classify");
        assert!(
            matches!(recognized.lines.as_slice(), [RecognizedLine::Static(_)]),
            "keyword discovery must not claim a typed conditional static ladder: {recognized:#?}"
        );

        let compiled = crate::compile_card_text(
            CardDefinitionBuilder::new(crate::CardId::new(), "Animus of Predation")
                .card_types(vec![crate::types::CardType::Creature]),
            oracle,
            false,
        )
        .expect("the typed draft ladder should lower");
        let debug = format!("{:#?}", compiled.definition.abilities);
        assert_eq!(
            debug.matches("PlayerRemovedDraftCardMatching").count(),
            11,
            "{debug}"
        );
        assert!(
            !debug.contains("animus of predation this creature"),
            "{debug}"
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
    fn graveyard_cast_conditions_carry_typed_labels_into_recognized() {
        let subtype_document = preprocess_document(
            CardDefinitionBuilder::new(crate::CardId::new(), "Gravecrawler Test")
                .card_types(vec![crate::types::CardType::Creature]),
            "You may cast this card from your graveyard as long as you control a Zombie.",
        )
        .expect("subtype condition should preprocess");
        let subtype_recognized =
            recognize_document(&subtype_document, false).expect("subtype recognized");
        let [RecognizedLine::Static(subtype_line)] = subtype_recognized.lines.as_slice() else {
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
        let color_recognized =
            recognize_document(&color_document, false).expect("color recognized");
        let [RecognizedLine::Static(color_line)] = color_recognized.lines.as_slice() else {
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
        let recognized =
            recognize_document(&document, false).expect("additional-combat recognized");
        let [RecognizedLine::Statement(line)] = recognized.lines.as_slice() else {
            panic!("expected one additional-combat statement");
        };
        assert_eq!(
            render_token_slice(&line.parse_tokens),
            "After this main phase, there is an additional combat phase followed by an additional main phase."
        );
    }

    #[test]
    fn attached_ignore_permission_wins_full_document_static_classification() {
        let restriction = "Enchanted creature can't attack or block, and its activated abilities can't be activated. That creature's controller may sacrifice a permanent of their choice for that player to ignore this effect until end of turn.";
        let document = preprocess_document(
            CardDefinitionBuilder::new(crate::CardId::new(), "Attached Restriction Test")
                .card_types(vec![crate::types::CardType::Enchantment])
                .subtypes(vec![crate::types::Subtype::Aura]),
            restriction,
        )
        .expect("attached restriction should preprocess");
        let recognized =
            recognize_document(&document, false).expect("attached restriction recognized");
        let [RecognizedLine::Static(line)] = recognized.lines.as_slice() else {
            panic!("the two-sentence attached restriction must be classified as one static line");
        };
        let routed = parse_static_ability_ast_line_lexed(&line.parse_tokens)
            .expect("classified static line should parse")
            .expect("classified static line should retain typed abilities");
        let debug = format!("{routed:#?}");
        assert_eq!(routed.len(), 3, "{debug}");
        assert!(debug.contains("AttackOrBlock"), "{debug}");
        assert!(debug.contains("ActivateAbilitiesOf"), "{debug}");
        assert!(
            debug.contains(
                "AttachedControllerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn"
            ),
            "{debug}"
        );

        let compiled = crate::compile_card_text(
            CardDefinitionBuilder::new(crate::CardId::new(), "Attached Restriction Test")
                .card_types(vec![crate::types::CardType::Enchantment])
                .subtypes(vec![crate::types::Subtype::Aura]),
            format!(
                "Enchant creature\n{restriction}\n{{1}}{{U}}: Return this Aura to its owner's hand."
            ),
            false,
        )
        .expect("the complete Aura text should compile");
        let abilities_debug = format!("{:#?}", compiled.definition.abilities);
        let spell_debug = format!("{:#?}", compiled.definition.spell_effect);
        assert!(
            abilities_debug.contains(
                "AttachedControllerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn"
            ) && abilities_debug.matches("RuleRestriction").count() >= 2,
            "all three static abilities must survive final definition assembly: {abilities_debug}"
        );
        assert!(
            spell_debug.contains("AttachToEffect")
                && !spell_debug.contains("MayEffect")
                && !spell_debug.contains("RuleRestriction"),
            "the spell program must contain only Aura attachment semantics: {spell_debug}"
        );
    }
}

fn partner_with_name_from_line(line: &PreprocessedLine) -> Option<String> {
    let name =
        keyword_special_lines::parse_partner_with_name_shape_tokens(&line.info.source_tokens)
            .map(|shape| render_token_slice(shape.name_tokens))
            .or_else(|| {
                let shape =
                    keyword_special_lines::parse_partner_with_name_shape_tokens(&line.tokens)?;
                Some(
                    render_original_text_for_token_slice(line, shape.name_tokens)
                        .unwrap_or_else(|| render_token_slice(shape.name_tokens)),
                )
            })?
            .trim()
            .replace('"', "");
    (!name.is_empty()).then_some(name)
}

pub(super) fn run_combined_static_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("combined-static-pair");
    let Some(PreprocessedItem::Line(next_line)) = ctx.preprocessed.items.get(ctx.idx + 1) else {
        return ParseOutcome::NoMatch;
    };
    if !should_try_combined_static_tokens(&ctx.line.tokens, &next_line.tokens) {
        return ParseOutcome::NoMatch;
    }

    let mut combined_tokens = tokens_without_terminal_period(&ctx.line.tokens).to_vec();
    combined_tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    combined_tokens.extend_from_slice(tokens_without_terminal_period(&next_line.tokens));
    let combined_line = rewrite_line_tokens(ctx.line, &combined_tokens);
    match line_family_try!(ctx, rule, recognize_static_line(&combined_line)) {
        Some(static_line) => line_family_match(
            ctx,
            LineDispatchResult::single(RecognizedLine::Static(static_line), ctx.idx + 2),
        ),
        None => ParseOutcome::NoMatch,
    }
}

pub(super) fn run_non_turn_conditional_untap_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("non-turn-conditional-untap");
    let Some(shape) =
        line_rewrite_grammar::parse_non_turn_conditional_untap_tokens(&ctx.line.tokens)
    else {
        return ParseOutcome::NoMatch;
    };
    let first_line = rewrite_line_tokens(ctx.line, shape.first_sentence_tokens);
    let Some(first_statement) = line_family_try!(ctx, rule, recognize_statement_line(&first_line))
    else {
        return ParseOutcome::NoMatch;
    };

    let second_line = rewrite_line_tokens(ctx.line, shape.untap_sentence_tokens);
    let Some(second_statement) =
        line_family_try!(ctx, rule, recognize_statement_line(&second_line))
    else {
        return ParseOutcome::NoMatch;
    };

    line_family_match(
        ctx,
        LineDispatchResult {
            lines: vec![
                RecognizedLine::Statement(first_statement),
                RecognizedLine::Statement(second_statement),
            ],
            next_idx: ctx.idx + 1,
        },
    )
}

fn is_keyword_action_replacement_static_line(tokens: &[OwnedLexToken]) -> bool {
    super::super::grammar::keyword_static_lines::parse_keyword_action_replacement_tokens(tokens)
        .is_some()
}

fn is_lose_game_replacement_static_line(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens);
    words.parses_prefix(&["if", "you", "would", "lose"])
        && words.parse_any_word_position_from(&["game"], 4).is_some()
        && words
            .parse_any_word_position_from(&["instead"], 4)
            .is_some()
}

fn has_specialized_document_line_shape(ctx: &LineDispatchContext<'_>) -> bool {
    let tokens = &ctx.line.tokens;
    sticker_sheet_ticket_marker_result(ctx).is_some()
        || normalize_trailing_keyword_activation_sentence_lexed(tokens).is_some()
        || line_grammar::parse_max_speed_line(tokens).is_some()
        || split_label_prefix_lexed(tokens).is_some()
        || line_grammar::parse_championed_with_this_trigger(tokens).is_some()
        || partner_with_name_from_line(ctx.line).is_some()
        || line_grammar::parse_partner_variant(tokens).is_some()
        || line_grammar::parse_simple_document_line(tokens).is_some()
        || line_grammar::parse_draft_rule_line(tokens).is_some()
        || line_grammar::parse_special_line(tokens).is_some()
        || line_grammar::parse_champion_line(tokens).is_some()
        || line_grammar::parse_station_keyword_line(tokens, &ctx.line.info.source_tokens).is_some()
        || line_grammar::parse_station_threshold_line(tokens).is_some()
        || line_grammar::parse_escape_enters_with_line(tokens).is_some()
        || line_grammar::parse_surge_line(tokens).is_some()
        || line_grammar::parse_freerunning_line(tokens).is_some()
        || is_ward_or_echo_static_prefix_line_lexed(tokens)
        || line_rewrite_grammar::parse_non_turn_conditional_untap_tokens(tokens).is_some()
        || line_rewrite_grammar::parse_graveyard_cast_control_condition_tokens(tokens).is_some()
        || line_rewrite_grammar::parse_additional_combat_rewrite_tokens(tokens).is_some()
        || line_grammar::parse_leading_unless_line(tokens).is_some()
        || (split_lexed_once_on_colon_outside_quotes(tokens).is_some()
            && split_activation_text_tokens_lexed(tokens).is_none())
        || ctx
            .preprocessed
            .items
            .get(ctx.idx + 1)
            .and_then(|item| match item {
                PreprocessedItem::Line(next) => Some(next),
                PreprocessedItem::Metadata(_) => None,
            })
            .is_some_and(|next| should_try_combined_static_tokens(tokens, &next.tokens))
}

pub(super) fn run_statement_probe_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("statement-probe");
    if line_family_claimed!(rule, run_keyword_line_family(ctx))
        || line_family_claimed!(rule, run_start_your_engines_line_family(ctx))
    {
        crate::parse_trace::event("statement-probe: declined for keyword/start-your-engines owner");
        return ParseOutcome::NoMatch;
    }
    // Dedicated trigger and activation families own their complete source
    // lines. The statement grammar intentionally understands many of their
    // effect bodies, but must not advertise those partial interpretations as
    // competing top-level registry matches.
    if line_family_claimed!(rule, run_triggered_line_family(ctx))
        || line_family_claimed!(rule, run_activation_line_family(ctx))
    {
        crate::parse_trace::event("statement-probe: declined for trigger/activation owner");
        return ParseOutcome::NoMatch;
    }
    if has_specialized_document_line_shape(ctx) {
        return ParseOutcome::NoMatch;
    }
    // A direct alternative-cast-cost sentence is a keyword line.  The broad
    // statement parsers can also interpret its leading `you may pay` as an
    // effect, but that interpretation drops the casting-method semantics and
    // must not become a second registry candidate.
    if line_family_try!(ctx, rule, is_direct_alternative_cost_keyword_line(ctx.line)) {
        return ParseOutcome::NoMatch;
    }
    // A token creation can contain quoted static-looking abilities.  Claim
    // the complete create sentence before the ability/static probes classify
    // one of those quoted rules as the host card's own ability.
    if ctx
        .line
        .tokens
        .first()
        .is_some_and(|token| token.is_word("create"))
        && let Some(mut statement_line) =
            line_family_try!(ctx, rule, recognize_statement_line(ctx.line))
    {
        statement_line
            .info
            .semantic_facts
            .statement
            .presentation_label = activated_presentation_from_preprocessed_line(ctx.line);
        let (statement_line, next_idx) = extend_statement_line_with_result_followups(
            &ctx.preprocessed.items,
            ctx.idx,
            statement_line,
        );
        return line_family_match(
            ctx,
            LineDispatchResult::single(RecognizedLine::Statement(statement_line), next_idx),
        );
    }
    // Bare keyword lines are claimed by the static/keyword line families.
    // Keep the broad statement probe from consuming them as empty effect
    // programs (notably Daybound, Nightbound, and Fuse).
    if parse_ability_line_lexed(&ctx.line.tokens).is_some()
        && super::super::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(
            &ctx.line.tokens,
        )
        .is_none()
    {
        return ParseOutcome::NoMatch;
    }
    let prefer_statement_before_static =
        super::super::grammar::structure::classify_statement_line_family_lexed(&ctx.line.tokens)
            .is_some_and(|family| {
                family != super::super::grammar::structure::StatementLineFamily::Generic
            })
            || should_prefer_statement_before_static_for_nonpermanent_spell(
                ctx.preprocessed,
                &ctx.line.tokens,
        ) || super::super::grammar::effects::parse_persistent_no_maximum_hand_size_player_lexed(
            &ctx.line.tokens,
        )
        .is_some()
            || super::super::grammar::effects::gain_ability_shapes::parse_source_gain_ability_shape(
                &ctx.line.tokens,
            )
            .is_some();
    let has_effect_prefix_before_static =
        has_effect_prefix_before_trailing_static_sentence(&ctx.line.tokens);

    // Typed static-line parsers must win over the broad statement probe. In
    // particular, `As this enters, choose ...` and `As this enters, note ...`
    // otherwise become generic AsEntersEffectProgram abilities, losing the
    // runtime ETB choice/note metadata. A typed statement on an instant or
    // sorcery is the exception: trailing restriction language must not turn
    // the spell's complete action sequence into a battlefield static ability.
    if !prefer_statement_before_static
        && !has_effect_prefix_before_static
        && matches!(
            parse_static_ability_ast_line_lexed(&ctx.line.tokens),
            Ok(Some(_))
        )
    {
        return ParseOutcome::NoMatch;
    }
    if line_family_try!(
        ctx,
        rule,
        crate::keyword_static::parse_double_counters_replacement_line(&ctx.line.tokens)
    )
    .is_some()
    {
        return ParseOutcome::NoMatch;
    }
    // "As this artifact enters, you may have it become a copy of ..." has a
    // dedicated as-enters copy-replacement static; let the static line family
    // claim it instead of the generic as-enters effect program.
    if super::super::grammar::keyword_static_lines::parse_enter_as_copy_tokens(&ctx.line.tokens)
        .is_some()
    {
        return ParseOutcome::NoMatch;
    }
    // Pay-life enter-the-battlefield replacements are expressed as two
    // sentences, so the generic statement parser can otherwise consume the
    // line before the typed static replacement family sees it.  Probe the
    // typed shape here as well so malformed variants fail instead of being
    // accepted as a partial "enters tapped" statement.
    if line_family_try!(
        ctx,
        rule,
        crate::keyword_static::parse_pay_life_or_enter_tapped_line(&ctx.line.tokens)
    )
    .is_some()
    {
        return ParseOutcome::NoMatch;
    }
    let replacement_sentences = split_lexed_sentences(&ctx.line.tokens);
    let replacement_split_candidate = matches!(
        line_grammar::parse_statement_static_preference(&ctx.line.tokens),
        Some(
            line_grammar::StatementStaticPreference::DrawReplacement
                | line_grammar::StatementStaticPreference::DiscardOrRedirectReplacement
        )
    )
        || line_grammar::parse_remove_counter_prevention_then_trigger(&ctx.line.tokens).is_some()
        || replacement_sentences.first().is_some_and(|sentence| {
            document_grammar::parse_conditional_replacement_surface(sentence).is_some()
        })
        || replacement_sentences
            .get(1)
            .is_some_and(|sentence| line_starts_with_trigger_intro_tokens(sentence));
    if replacement_split_candidate
        && let Some(split_result) = line_family_try!(
            ctx,
            rule,
            parse_labeled_conditional_replacement_sentence_split(ctx.line, ctx.idx)
        )
    {
        return line_family_match(ctx, split_result);
    }

    let linked_preference = line_grammar::parse_linked_statement_preference(&ctx.line.tokens);
    let static_preference = line_grammar::parse_statement_static_preference(&ctx.line.tokens);
    let is_keyword_action_replacement = is_keyword_action_replacement_static_line(&ctx.line.tokens);
    let is_lose_game_replacement = is_lose_game_replacement_static_line(&ctx.line.tokens);
    if (prefer_statement_before_static
        || matches!(
            crate::grammar::structure::classify_statement_line_family_lexed(&ctx.line.tokens),
            Some(
                crate::grammar::structure::StatementLineFamily::Divvy
                    | crate::grammar::structure::StatementLineFamily::PactNextUpkeep
                    | crate::grammar::structure::StatementLineFamily::ExilePlayCostsMore
            )
        )
        || linked_preference.is_some()
        || looks_like_statement_line_lexed(ctx.line))
        && (!matches!(
            static_preference,
            Some(
                line_grammar::StatementStaticPreference::BlocksAdditionalCreatures
                    | line_grammar::StatementStaticPreference::DrawReplacement
                    | line_grammar::StatementStaticPreference::TokenCreationReplacement
                    | line_grammar::StatementStaticPreference::DiscardOrRedirectReplacement
                    | line_grammar::StatementStaticPreference::FirstEquipCostAlternative
                    | line_grammar::StatementStaticPreference::ConditionalKeywordTypeAddition
            )
        ) || prefer_statement_before_static)
        && !is_keyword_action_replacement
        && !is_lose_game_replacement
        && let Some(mut statement_line) =
            line_family_try!(ctx, rule, recognize_statement_line(ctx.line))
    {
        statement_line
            .info
            .semantic_facts
            .statement
            .presentation_label = activated_presentation_from_preprocessed_line(ctx.line);
        let (statement_line, next_idx) = extend_statement_line_with_result_followups(
            &ctx.preprocessed.items,
            ctx.idx,
            statement_line,
        );
        return line_family_match(
            ctx,
            LineDispatchResult::single(RecognizedLine::Statement(statement_line), next_idx),
        );
    }
    ParseOutcome::NoMatch
}

pub(super) fn run_static_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("static-line");
    if line_family_claimed!(rule, run_keyword_line_family(ctx))
        || line_family_claimed!(rule, run_start_your_engines_line_family(ctx))
    {
        return ParseOutcome::NoMatch;
    }
    // Activated and triggered source lines may contain text that the broad
    // static grammar can parse after the cost/trigger prefix.  Their dedicated
    // families own the complete line, so that partial static interpretation is
    // never an independent registry candidate.
    if line_family_claimed!(rule, run_triggered_line_family(ctx))
        || line_family_claimed!(rule, run_activation_line_family(ctx))
    {
        return ParseOutcome::NoMatch;
    }
    // The statement probe already resolves static-vs-statement ownership from
    // typed line facts. If it accepts the complete line, a permissive static
    // parse is the same overlapping candidate rather than a second semantic
    // interpretation.
    if line_family_claimed!(rule, run_statement_probe_line_family(ctx)) {
        crate::parse_trace::event("static-line: declined for statement-probe owner");
        return ParseOutcome::NoMatch;
    }
    if has_specialized_document_line_shape(ctx) {
        crate::parse_trace::event("static-line: declined for specialized document shape");
        return ParseOutcome::NoMatch;
    }
    match recognize_static_line(ctx.line) {
        Ok(Some(static_line)) => line_family_match(
            ctx,
            LineDispatchResult::single(RecognizedLine::Static(static_line), ctx.idx + 1),
        ),
        Ok(None) => ParseOutcome::NoMatch,
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
            ParseOutcome::NoMatch
        }
        Err(err) => line_family_error(ctx, rule, err),
    }
}

pub(super) fn run_statement_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("statement-line");
    if line_family_claimed!(rule, run_keyword_line_family(ctx))
        || line_family_claimed!(rule, run_start_your_engines_line_family(ctx))
    {
        return ParseOutcome::NoMatch;
    }
    if line_family_claimed!(rule, run_triggered_line_family(ctx))
        || line_family_claimed!(rule, run_activation_line_family(ctx))
    {
        return ParseOutcome::NoMatch;
    }
    if line_family_try!(ctx, rule, is_direct_alternative_cost_keyword_line(ctx.line)) {
        return ParseOutcome::NoMatch;
    }
    // This is the permissive fallback behind the guarded statement probe.
    // If the probe already owns the line, returning the same statement a
    // second time creates a false registry ambiguity; specialized probe
    // results (including linked followups) must also keep their precedence.
    if line_family_claimed!(rule, run_statement_probe_line_family(ctx)) {
        return ParseOutcome::NoMatch;
    }
    if has_specialized_document_line_shape(ctx) {
        return ParseOutcome::NoMatch;
    }
    // A successful typed static parse owns permanent rules text.  The final
    // statement family is intentionally permissive and can often reinterpret
    // the same words as a one-shot effect; do not advertise that fallback as
    // a second registry meaning.  Instants and sorceries retain the explicit
    // statement-before-static exception used by the guarded probe above.
    if !should_prefer_statement_before_static_for_nonpermanent_spell(
        ctx.preprocessed,
        &ctx.line.tokens,
    ) && matches!(recognize_static_line(ctx.line), Ok(Some(_)))
    {
        return ParseOutcome::NoMatch;
    }
    match line_family_try!(ctx, rule, recognize_statement_line(ctx.line)) {
        Some(mut statement_line) => {
            statement_line
                .info
                .semantic_facts
                .statement
                .presentation_label = activated_presentation_from_preprocessed_line(ctx.line);
            let (statement_line, next_idx) = extend_statement_line_with_result_followups(
                &ctx.preprocessed.items,
                ctx.idx,
                statement_line,
            );
            line_family_match(
                ctx,
                LineDispatchResult::single(RecognizedLine::Statement(statement_line), next_idx),
            )
        }
        None => ParseOutcome::NoMatch,
    }
}

pub(super) fn run_leading_unless_statement_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let Some(shape) = line_grammar::parse_leading_unless_line(&ctx.line.tokens) else {
        return ParseOutcome::NoMatch;
    };
    debug_assert!(shape.condition_tokens.len() >= 2 && !shape.effect_tokens.is_empty());

    let parse_tokens = ctx.line.tokens.clone();
    let parse_groups = vec![parse_tokens.clone()];
    let statement_line = RecognizedStatementLine {
        info: ctx.line.info.clone(),
        text: ctx.line.info.normalized.normalized.clone(),
        parse_tokens,
        parse_groups,
        parsed_effects: None,
    };
    line_family_match(
        ctx,
        LineDispatchResult::single(RecognizedLine::Statement(statement_line), ctx.idx + 1),
    )
}

pub(super) fn run_colon_nonactivation_statement_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    let rule = RuleId::new("colon-nonactivation-statement");
    if line_family_claimed!(rule, run_activation_line_family(ctx)) {
        return ParseOutcome::NoMatch;
    }

    match line_family_try!(
        ctx,
        rule,
        parse_colon_nonactivation_statement_fallback(ctx.line)
    ) {
        Some(statement_line) => line_family_match(
            ctx,
            LineDispatchResult::single(RecognizedLine::Statement(statement_line), ctx.idx + 1),
        ),
        None => ParseOutcome::NoMatch,
    }
}

pub(super) fn run_unsupported_line_family(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    if ctx.allow_unsupported {
        return line_family_match(
            ctx,
            LineDispatchResult::single(
                RecognizedLine::Unsupported(RecognizedUnsupportedLine {
                    info: ctx.line.info.clone(),
                    reason_code: if matches!(
                        crate::grammar::structure::classify_statement_line_family_lexed(
                            &ctx.line.tokens
                        ),
                        Some(crate::grammar::structure::StatementLineFamily::PactNextUpkeep)
                    ) {
                        "statement-line-not-yet-supported"
                    } else {
                        classify_unsupported_line_reason(ctx.line)
                    },
                }),
                ctx.idx + 1,
            ),
        );
    }

    line_family_error(
        ctx,
        RuleId::new("unsupported-line-family"),
        CardTextError::ParseError(format!(
            "parser does not yet support line family: '{}'",
            ctx.line.info.raw_line
        )),
    )
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
    let (prefix_statement, prefix_statement_error) = match recognize_statement_line(&prefix_line) {
        Ok(statement) => (statement, None),
        Err(err) => (None, Some(err)),
    };
    let prefix_recognized = if let Some(statement_line) = prefix_statement {
        RecognizedLine::Statement(statement_line)
    } else {
        parse_keyword_activation_prefix_static_or_rewrite(
            builder,
            line,
            &prefix_line,
            prefix_statement_error,
        )?
    };

    let suffix_line = rewrite_line_tokens(line, &suffix_tokens);
    let Some((_, _, body_tokens)) = split_label_prefix_lexed(&suffix_line.tokens) else {
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
    let normalized_cost_tokens =
        normalize_activation_cost_tokens_for_builder(builder, line, cost_tokens.clone())?;
    let cost = parse_activation_cost_tokens_rewrite(&normalized_cost_tokens)?;
    let effect_parse_tokens =
        normalize_activation_effect_tokens_for_builder(builder, line, &effect_parse_tokens)?;
    let activated = RecognizedLine::Activated(RecognizedActivatedLine {
        info: suffix_line.info.clone(),
        cost,
        cost_parse_tokens: normalized_cost_tokens,
        effect_parse_tokens,
        presentation: None,
        chosen_option: None,
    });

    Ok(Some(LineDispatchResult {
        lines: vec![prefix_recognized, activated],
        next_idx: idx + 1,
    }))
}

fn parse_keyword_activation_prefix_static_or_rewrite(
    _builder: &CardDefinitionBuilder,
    line: &PreprocessedLine,
    prefix_line: &PreprocessedLine,
    statement_error: Option<CardTextError>,
) -> Result<RecognizedLine, CardTextError> {
    let static_error = match recognize_static_line(prefix_line) {
        Ok(Some(static_line)) => return Ok(RecognizedLine::Static(static_line)),
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

#[cfg(test)]
mod ticket_marker_tests {
    use super::*;

    fn compiled_sticker_marker_labels(name: &str, text: &str) -> Vec<String> {
        let compiled = crate::compile_card_text(
            CardDefinitionBuilder::new(crate::ids::CardId::new(), name),
            text,
            false,
        )
        .unwrap_or_else(|err| panic!("{name} should compile as a sticker sheet: {err:?}"));

        assert!(
            compiled.definition.spell_effect.is_none(),
            "sticker-sheet rows must not become spell effects: {:#?}",
            compiled.definition
        );
        compiled
            .definition
            .abilities
            .iter()
            .map(|ability| match &ability.kind {
                crate::ability::AbilityKind::Static(static_ability) => {
                    static_ability.display().to_ascii_lowercase()
                }
                other => panic!("sticker-sheet row became a runtime ability: {other:#?}"),
            })
            .collect()
    }

    #[test]
    fn sticker_ticket_keyword_rows_keep_their_threshold_header() {
        let labels = compiled_sticker_marker_labels(
            "Trendy Circus Pirate",
            "Type: Stickers\n\
             {TK}{TK} — Deathtouch\n\
             {TK}{TK}{TK}{TK}{TK} — Whenever this creature deals combat damage to a player, create that many 1/1 green Squirrel creature tokens.\n\
             {TK}{TK} — 5/1\n\
             {TK}{TK}{TK} — 3/6",
        );

        assert_eq!(labels.len(), 4, "{labels:#?}");
        assert_eq!(labels[0], "{tk}{tk} — deathtouch");
        assert!(labels[1].starts_with("{tk}{tk}{tk}{tk}{tk} — whenever "));
    }

    #[test]
    fn sticker_ticket_double_labeled_trigger_stays_one_presentation_row() {
        let labels = compiled_sticker_marker_labels(
            "Werewolf Lightning Mage",
            "Type: Stickers\n\
             {TK}{TK} — Landfall — Whenever a land enters under your control, put a +1/+1 counter on this permanent.\n\
             {TK}{TK}{TK}{TK} — Whenever a creature blocks this creature, that creature gets -4/-4 until end of turn.\n\
             {TK}{TK} — 4/1\n\
             {TK}{TK}{TK} — 3/5",
        );

        assert_eq!(labels.len(), 4, "{labels:#?}");
        assert!(labels[0].starts_with("{tk}{tk} — landfall — whenever "));
        assert!(labels[0].ends_with("put a +1/+1 counter on this permanent."));
        assert!(labels[1].starts_with("{tk}{tk}{tk}{tk} — whenever "));
    }
}
