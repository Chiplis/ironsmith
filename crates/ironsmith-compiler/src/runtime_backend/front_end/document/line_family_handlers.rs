use super::super::token_primitives::str_ends_with_any_char;
use super::line_dispatch::{LineDispatchContext, LineDispatchResult};
use super::*;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};

const MAX_SPEED_CONDITION_LABEL: &str = "__max_speed_condition";
const CONTROL_COLOR_PAIR_PERMANENT_CONDITION_PREFIX: &str = "__control_color_pair_permanent_";
const STATION_THRESHOLD_CONDITION_PREFIX: &str = "__station_threshold_";
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
const CAN_BLOCK_ADDITIONAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "creature", "can", "block"]);
const ADDITIONAL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["additional"]);
const CREATURE_OR_CREATURES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["creature", "creatures"]]);
const BLOCK_DURATION_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["each", "combat"], &["this", "turn"]]);
const LINKED_EXILED_CARD_COST_MORE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [
            &[
                "for", "as", "long", "as", "that", "card", "remains", "exiled",
            ],
            &["more", "to", "cast"],
        ]
);
const LINKED_CHOOSE_TWO_SHUFFLE_REST_BATTLEFIELD_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [
            &["chooses", "two", "of", "those", "cards"],
            &["shuffle", "the", "chosen", "cards"],
            &["put", "the", "rest", "onto", "the", "battlefield"],
        ]
);
const START_YOUR_ENGINES_LINE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["start", "your", "engines"]);
const LEARN_LINE_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["learn"]);
const STATION_LINE_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["station"]);
const ARTIFACT_CREATURE_AT_PREFIX: &[&str] = &["artifact", "creature", "at"];
const CHAMPIONED_WITH_THIS_PHRASE: &[&str] = &["is", "championed", "with", "this"];
const MAX_SPEED_PREFIX: &[&str] = &["max", "speed"];
const GRAVEYARD_CAST_CONTROL_PREFIX: &[&str] = &[
    "you",
    "may",
    "cast",
    "this",
    "card",
    "from",
    "your",
    "graveyard",
    "as",
    "long",
    "as",
    "you",
    "control",
    "a",
];
const GRAVEYARD_OR_EXILE_CAST_LINE: &[&str] = &[
    "you",
    "may",
    "cast",
    "this",
    "card",
    "from",
    "your",
    "graveyard",
    "or",
    "from",
    "exile",
];
const CHAMPION_PREFIX: &[&str] = &["champion"];
const PARTNER_PREFIX: &[&str] = &["partner"];
const PARTNER_WITH_PREFIX: &[&str] = &["partner", "with"];
const ESCAPES_WITH_PHRASE: &[&str] = &["escapes", "with"];
const CREATURES_YOU_CONTROL_GET_PREFIX: &[&str] = &["creatures", "you", "control", "get"];
const CHARACTER_SELECT_PREFIX: &[&str] = &["character", "select"];
const PARTNER_WITH_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix PARTNER_WITH_PREFIX);
const CREATURES_YOU_CONTROL_GET_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix CREATURES_YOU_CONTROL_GET_PREFIX);
const NON_TURN_UNTAP_SUFFIX: &[&str] = &[
    "if",
    "it's",
    "not",
    "your",
    "turn",
    "untap",
    "those",
    "creatures",
];
const NON_TURN_UNTAP_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix NON_TURN_UNTAP_SUFFIX);
const INDEFINITE_ARTICLE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["a"], &["an"]]);
const TRIGGER_INTRO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["when"], &["whenever"], &["at"]]);
const SPLIT_TOP_AND_FACE_DOWN_LOOK_LINE: &[&str] = &[
    "you",
    "may",
    "look",
    "at",
    "the",
    "top",
    "card",
    "of",
    "your",
    "library",
    "and",
    "at",
    "face-down",
    "creatures",
    "you",
    "don't",
    "control",
    "any",
    "time",
];
const SPLIT_TOP_LOOK_AND_TOP_LAND_PLAY_LINE: &[&str] = &[
    "you", "may", "look", "at", "the", "top", "card", "of", "your", "library", "any", "time",
    "and", "you", "may", "play", "lands", "from", "the", "top", "of", "your", "library",
];
const ASSIGN_DAMAGE_AS_UNBLOCKED_ENCHANTED_LINE: &[&str] = &[
    "enchanted",
    "creatures",
    "controller",
    "may",
    "have",
    "it",
    "assign",
    "its",
    "combat",
    "damage",
    "as",
    "though",
    "it",
    "werent",
    "blocked",
];
const ADDITIONAL_COMBAT_AFTER_THIS_PHASE_PHRASE: &[&str] = &[
    "there",
    "is",
    "an",
    "additional",
    "combat",
    "phase",
    "after",
    "this",
    "phase",
    "followed",
    "by",
    "an",
    "additional",
    "main",
    "phase",
];
const ADDITIONAL_COMBAT_AFTER_THIS_MAIN_PHASE_LINE: &[&str] = &[
    "after",
    "this",
    "main",
    "phase",
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
];

fn line_starts_with_words(line: &PreprocessedLine, words: &[&str]) -> bool {
    ClauseShape::new()
        .prefix(words)
        .matches_words(&crate::runtime_backend::token_word_refs(&line.tokens))
}

fn line_contains_words(line: &PreprocessedLine, words: &[&str]) -> bool {
    ClauseShape::new()
        .contains_phrases(&[words])
        .matches_words(&crate::runtime_backend::token_word_refs(&line.tokens))
}

fn line_ends_with_words(line: &PreprocessedLine, words: &[&str]) -> bool {
    ClauseShape::new()
        .suffix(words)
        .matches_words(&crate::runtime_backend::token_word_refs(&line.tokens))
}

fn keyword_body_tokens_before_reminder<'a>(
    line: &'a PreprocessedLine,
    prefix: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    if !ClauseShape::new()
        .prefix(prefix)
        .matches_words(&crate::runtime_backend::token_word_refs(&line.tokens))
    {
        return None;
    }
    let body_start = prefix.len();
    let body_end = line.tokens[body_start..]
        .iter()
        .position(|token| matches!(token.kind, TokenKind::LParen | TokenKind::Period))
        .map(|offset| body_start + offset)
        .unwrap_or(line.tokens.len());
    Some(trim_lexed_commas(&line.tokens[body_start..body_end]))
}

fn strip_indefinite_article_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if INDEFINITE_ARTICLE_PREFIX_PATTERN
        .matches_words(&crate::runtime_backend::token_word_refs(tokens))
    {
        &tokens[1..]
    } else {
        tokens
    }
}

fn render_keyword_cost_tokens(tokens: &[OwnedLexToken]) -> String {
    let rendered = render_token_slice(tokens);
    let mut out = String::with_capacity(rendered.len());
    let mut in_mana_group = false;
    for ch in rendered.chars() {
        match ch {
            '{' => {
                in_mana_group = true;
                out.push(ch);
            }
            '}' => {
                in_mana_group = false;
                out.push(ch);
            }
            _ if in_mana_group => out.push(ch.to_ascii_uppercase()),
            _ => out.push(ch),
        }
    }
    out
}

fn line_starts_with_trigger_intro(line: &PreprocessedLine) -> bool {
    TRIGGER_INTRO_PREFIX_PATTERN
        .matches_words(&crate::runtime_backend::token_word_refs(&line.tokens))
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
        let Some(static_line) = parse_static_line_cst(ctx.line)? else {
            return Err(CardTextError::ParseError(format!(
                "parser could not lower sticker ticket marker line: '{}'",
                ctx.line.info.raw_line
            )));
        };
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Static(static_line),
            ctx.idx + 1,
        )));
    }

    try_parse_labeled_line_dispatch(ctx.preprocessed, ctx.idx, ctx.line, ctx.allow_unsupported)
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

    let Some(dash_idx) = ctx
        .line
        .tokens
        .iter()
        .position(|token| token.kind == TokenKind::EmDash)
    else {
        return false;
    };
    let cost_tokens = &ctx.line.tokens[..dash_idx];
    let body_tokens = &ctx.line.tokens[dash_idx + 1..];
    let saw_ticket_symbol = !cost_tokens.is_empty()
        && cost_tokens.iter().all(|token| {
            token.kind == TokenKind::ManaGroup && token.slice.eq_ignore_ascii_case("{tk}")
        });

    saw_ticket_symbol && !TokenWordView::new(body_tokens).is_empty()
}

pub(super) fn run_triggered_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    try_parse_triggered_line_dispatch(ctx.preprocessed, ctx.idx, ctx.line, ctx.allow_unsupported)
}

pub(super) fn run_championed_with_this_trigger_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if !line_starts_with_words(ctx.line, &["when"])
        || !line_contains_words(ctx.line, CHAMPIONED_WITH_THIS_PHRASE)
    {
        return Ok(None);
    }
    let Some((_, effect_tokens)) = split_once_on_comma_tokens(&ctx.line.tokens) else {
        return Ok(None);
    };
    let effect_text = render_token_slice(tokens_without_terminal_period(effect_tokens))
        .trim()
        .to_string();
    let triggered_text = format!("When this creature enters, {}", effect_text);
    let triggered_line = rewrite_line_normalized(ctx.line, triggered_text.as_str())?;
    let triggered = parse_triggered_line_cst(&triggered_line)?;
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Triggered(triggered),
        ctx.idx + 1,
    )))
}

fn split_once_on_comma_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let comma_idx = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Comma)?;
    Some((&tokens[..comma_idx], tokens.get(comma_idx + 1..)?))
}

pub(super) fn run_max_speed_labeled_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if !line_starts_with_words(ctx.line, MAX_SPEED_PREFIX) {
        return Ok(None);
    };

    let body_text = max_speed_body_text_from_tokens(ctx.line)
        .unwrap_or_else(|| ctx.line.info.normalized.normalized.trim().to_string());
    if body_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "max-speed label missing ability body: '{}'",
            ctx.line.info.raw_line
        )));
    }

    let body_line = rewrite_line_normalized(ctx.line, body_text.as_str())?;
    if line_starts_with_trigger_intro(&body_line) {
        let triggered_text = max_speed_intervening_if_text(&body_line.tokens);
        let triggered_line = rewrite_line_normalized(ctx.line, triggered_text.as_str())?;
        let triggered = parse_triggered_line_cst(&triggered_line)?;
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Triggered(triggered),
            ctx.idx + 1,
        )));
    }

    let activation_text = format!(
        "{}. Activate only if you have max speed.",
        render_tokens_without_terminal_period(&body_line.tokens)
    );
    let activation_line = rewrite_line_normalized(ctx.line, activation_text.as_str())?;
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
                        chosen_option_label: None,
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
            chosen_option_label: Some(MAX_SPEED_CONDITION_LABEL.to_string()),
            ..static_cst
        }),
        ctx.idx + 1,
    )))
}

fn max_speed_body_text_from_tokens(line: &PreprocessedLine) -> Option<String> {
    let dash_idx = line.tokens.iter().position(|token| {
        matches!(
            token.kind,
            TokenKind::Dash | TokenKind::EmDash | TokenKind::Colon
        )
    })?;
    let body_tokens = line.tokens.get(dash_idx + 1..)?;
    let body_text = render_token_slice(body_tokens).trim().to_string();
    (!body_text.is_empty()).then_some(body_text)
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
    let tokens = tokens_without_terminal_period(tokens);
    let end = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LParen)
        .unwrap_or(tokens.len());
    tokens_without_terminal_period(&tokens[..end])
}

fn render_tokens_without_terminal_period(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(tokens_without_terminal_period(tokens))
        .trim()
        .to_string()
}

fn max_speed_intervening_if_text(body_tokens: &[OwnedLexToken]) -> String {
    let tokens = tokens_without_terminal_period(body_tokens);
    let Some(comma_idx) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Comma)
    else {
        return render_token_slice(tokens).trim().to_string();
    };
    let trigger = render_token_slice(&tokens[..comma_idx]).trim().to_string();
    let effects = render_token_slice(&tokens[comma_idx + 1..])
        .trim_start()
        .to_string();
    format!("{trigger}, if you have max speed,{effects}")
}

pub(super) fn run_start_your_engines_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(&ctx.line.tokens);
    if !START_YOUR_ENGINES_LINE_PATTERN.matches_words(&words) {
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

pub(super) fn run_draft_rule_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if !is_draft_rule_line(&ctx.line.tokens) {
        return Ok(None);
    }

    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(StaticLineCst {
            info: ctx.line.info.clone(),
            text: ctx.line.info.normalized.normalized.clone(),
            parse_tokens: ctx.line.tokens.clone(),
            chosen_option_label: None,
        }),
        ctx.idx + 1,
    )))
}

fn is_draft_rule_line(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    DRAFT_RULE_LINE_PATTERN.matches_words(&words)
        || DRAFT_RULE_PREFIX_PATTERN.matches_words(&words)
        || DRAFT_BOOSTER_PASS_PATTERN.matches_words(&words)
}

pub(super) fn run_learn_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(&ctx.line.tokens);
    if !LEARN_LINE_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let learn_line = rewrite_line_normalized(ctx.line, "learn")?;
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Statement(StatementLineCst {
            info: learn_line.info,
            text: "learn".to_string(),
            parse_tokens: learn_line.tokens.clone(),
            parse_groups: vec![learn_line.tokens],
        }),
        ctx.idx + 1,
    )))
}

pub(super) fn run_split_top_and_face_down_look_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if crate::runtime_backend::token_word_refs(&ctx.line.tokens)
        != SPLIT_TOP_AND_FACE_DOWN_LOOK_LINE
    {
        return Ok(None);
    }

    let top_card_line = rewrite_line_normalized(
        ctx.line,
        "You may look at the top card of your library any time.",
    )?;
    let face_down_line = rewrite_line_normalized(
        ctx.line,
        "You may look at face-down creatures you don't control any time.",
    )?;

    let Some(top_card_static) = parse_static_line_cst(&top_card_line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower split top-card line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    let Some(face_down_static) = parse_static_line_cst(&face_down_line)? else {
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
    if !token_slice_words_eq(&ctx.line.tokens, SPLIT_TOP_LOOK_AND_TOP_LAND_PLAY_LINE) {
        return Ok(None);
    }

    let top_card_line = rewrite_line_normalized(
        ctx.line,
        "You may look at the top card of your library any time.",
    )?;
    let play_lands_line =
        rewrite_line_normalized(ctx.line, "You may play lands from the top of your library.")?;

    let Some(top_card_static) = parse_static_line_cst(&top_card_line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower split top-card look line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    let Some(play_lands_static) = parse_static_line_cst(&play_lands_line)? else {
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
    if !token_slice_words_eq(&ctx.line.tokens, ASSIGN_DAMAGE_AS_UNBLOCKED_ENCHANTED_LINE) {
        return Ok(None);
    }

    let rewritten = "Enchanted creature has \"You may have this creature assign its combat damage as though it weren't blocked.\".";
    let rewritten_line = rewrite_line_normalized(ctx.line, rewritten)?;
    let Some(static_cst) = parse_static_line_cst(&rewritten_line)? else {
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
    if !line_starts_with_words(ctx.line, GRAVEYARD_CAST_CONTROL_PREFIX)
        || !line_ends_with_words(ctx.line, &["permanent"])
    {
        return Ok(None);
    }

    let words = TokenWordView::new(&ctx.line.tokens);
    let prefix_len = GRAVEYARD_CAST_CONTROL_PREFIX.len();
    if words.len() != prefix_len + 4
        || !words.at_is(prefix_len + 1, "or")
        || !words.at_is(prefix_len + 3, "permanent")
    {
        return Ok(None);
    };
    let left = words.get(prefix_len).unwrap_or_default();
    let right = words.get(prefix_len + 2).unwrap_or_default();
    if left.is_empty() || right.is_empty() {
        return Ok(None);
    }

    let permission_line =
        rewrite_line_normalized(ctx.line, "You may cast this card from your graveyard.")?;
    let Some(mut static_cst) = parse_static_line_cst(&permission_line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower graveyard-cast control condition line: '{}'",
            ctx.line.info.raw_line
        )));
    };
    static_cst.chosen_option_label = Some(format!(
        "{CONTROL_COLOR_PAIR_PERMANENT_CONDITION_PREFIX}{}_{}",
        left.to_ascii_lowercase(),
        right.to_ascii_lowercase()
    ));

    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(static_cst),
        ctx.idx + 1,
    )))
}

pub(super) fn run_graveyard_or_exile_cast_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if !token_slice_words_eq(&ctx.line.tokens, GRAVEYARD_OR_EXILE_CAST_LINE) {
        return Ok(None);
    }

    let graveyard_line =
        rewrite_line_normalized(ctx.line, "You may cast this card from your graveyard.")?;
    let exile_line = rewrite_line_normalized(ctx.line, "You may cast this card from exile.")?;

    let Some(graveyard_static) = parse_static_line_cst(&graveyard_line)? else {
        return Err(CardTextError::ParseError(format!(
            "parser could not lower graveyard-or-exile cast line graveyard half: '{}'",
            ctx.line.info.raw_line
        )));
    };
    let Some(exile_static) = parse_static_line_cst(&exile_line)? else {
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
    let Some(filter_tokens) = keyword_body_tokens_before_reminder(ctx.line, CHAMPION_PREFIX) else {
        return Ok(None);
    };
    let filter_text = render_token_slice(strip_indefinite_article_tokens(filter_tokens))
        .trim()
        .to_string();
    if filter_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "champion keyword missing object filter: '{}'",
            ctx.line.info.raw_line
        )));
    }

    let triggered_text = format!(
        "When this creature enters, exile another {filter_text} you control until this creature leaves the battlefield."
    );
    let triggered_line = rewrite_line_normalized(ctx.line, triggered_text.as_str())?;
    let triggered = parse_triggered_line_cst(&triggered_line)?;
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Triggered(triggered),
        ctx.idx + 1,
    )))
}

pub(super) fn run_station_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(&ctx.line.tokens);
    if !STATION_LINE_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let activation_text = "Tap another untapped creature you control: Put X charge counters on this artifact, where X is the power of the creature tapped this way. Activate only as a sorcery.";
    let activation_line = rewrite_line_normalized(ctx.line, activation_text)?;
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
        chosen_option_label: None,
    })];

    let has_explicit_station_threshold_rows = ctx
        .preprocessed
        .items
        .iter()
        .filter_map(|item| match item {
            PreprocessedItem::Line(line) => Some(line),
            PreprocessedItem::Metadata(_) => None,
        })
        .any(|line| parse_station_threshold_line(line).is_some());
    if !has_explicit_station_threshold_rows
        && let Some(threshold) = parse_station_keyword_creature_threshold_for_line(ctx.line)
        && let Some(pt) = ctx.preprocessed.builder.card_builder.power_toughness_ref()
    {
        let label = station_threshold_condition_label(threshold);
        let power = pt.power.base_value();
        let toughness = pt.toughness.base_value();
        for static_text in [
            "This artifact is a creature in addition to its other types.".to_string(),
            format!("This artifact has base power and toughness {power}/{toughness}."),
        ] {
            let static_line = rewrite_line_normalized(ctx.line, static_text.as_str())?;
            let Some(static_cst) = parse_static_line_cst(&static_line)? else {
                return Err(CardTextError::ParseError(format!(
                    "parser could not lower station reminder threshold support: '{}'",
                    ctx.line.info.raw_line
                )));
            };
            lines.push(RewriteLineCst::Static(StaticLineCst {
                chosen_option_label: Some(label.clone()),
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
    let Some((threshold, mut body_text)) = parse_station_threshold_line(ctx.line) else {
        return Ok(None);
    };
    if let Some(rewritten) =
        normalize_named_source_sentence_for_builder(&ctx.preprocessed.builder, body_text.as_str())
    {
        body_text = rewritten;
    } else {
        let source_name = ctx.preprocessed.builder.card_builder.name_ref();
        if !source_name.is_empty() {
            let source_name_lower = source_name.to_ascii_lowercase();
            let rewritten = replace_named_source_aliases(
                &body_text,
                source_name_lower.as_str(),
                "this artifact",
            );
            if rewritten != body_text {
                body_text = rewritten;
            }
        }
    }
    if !str_ends_with_any_char(body_text.as_str(), &['.', '!', '?']) {
        body_text.push('.');
    }

    let label = station_threshold_condition_label(threshold);
    let mut lines = Vec::new();
    if station_threshold_is_creature_pt_threshold(ctx, threshold)
        && let Some(pt) = ctx.preprocessed.builder.card_builder.power_toughness_ref()
    {
        let power = pt.power.base_value();
        let toughness = pt.toughness.base_value();
        for static_text in [
            "This artifact is a creature in addition to its other types.".to_string(),
            format!("This artifact has base power and toughness {power}/{toughness}."),
        ] {
            let static_line = rewrite_line_normalized(ctx.line, static_text.as_str())?;
            let Some(static_cst) = parse_static_line_cst(&static_line)? else {
                return Err(CardTextError::ParseError(format!(
                    "parser could not lower station creature threshold support: '{}'",
                    ctx.line.info.raw_line
                )));
            };
            lines.push(RewriteLineCst::Static(StaticLineCst {
                chosen_option_label: Some(label.clone()),
                ..static_cst
            }));
        }
    }

    let body_line = rewrite_line_normalized(ctx.line, body_text.as_str())?;
    if line_starts_with_trigger_intro(&body_line) {
        let mut triggered = parse_triggered_line_cst(&body_line)?;
        triggered.chosen_option_label = Some(label);
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
            chosen_option_label: Some(label),
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
        chosen_option_label: Some(label),
        ..static_cst
    }));
    Ok(Some(LineDispatchResult {
        lines,
        next_idx: ctx.idx + 1,
    }))
}

fn parse_station_threshold_line(line: &PreprocessedLine) -> Option<(i32, String)> {
    let tokens = &line.tokens;
    let pipe_idx = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Pipe)?;
    let [threshold_token, plus_token] = tokens.get(..pipe_idx)? else {
        return None;
    };
    if !matches!(threshold_token.kind, TokenKind::Number | TokenKind::Word)
        || plus_token.kind != TokenKind::Plus
    {
        return None;
    }

    let threshold = threshold_token.parser_text.parse::<i32>().ok()?;
    let body_tokens = trim_lexed_commas(tokens.get(pipe_idx + 1..)?);
    let body = render_original_text_for_token_slice(line, body_tokens)
        .unwrap_or_else(|| render_token_slice(body_tokens))
        .trim()
        .to_string();
    (!body.is_empty()).then(|| (threshold, body.to_string()))
}

fn parse_station_keyword_creature_threshold(tokens: &[OwnedLexToken]) -> Option<i32> {
    let word_positions = crate::runtime_backend::lexer::parser_token_word_positions(tokens);
    for (word_idx, window) in word_positions.windows(4).enumerate() {
        if !window
            .iter()
            .take(3)
            .map(|(_, word)| *word)
            .eq(ARTIFACT_CREATURE_AT_PREFIX.iter().copied())
        {
            continue;
        }
        let (threshold_token_idx, threshold_word) = word_positions[word_idx + 3];
        let threshold = threshold_word.parse::<i32>().ok()?;
        if tokens
            .get(threshold_token_idx + 1)
            .is_some_and(|token| token.kind == TokenKind::Plus)
        {
            return Some(threshold);
        }
    }
    None
}

fn parse_station_keyword_creature_threshold_for_line(line: &PreprocessedLine) -> Option<i32> {
    parse_station_keyword_creature_threshold(&line.tokens).or_else(|| {
        crate::runtime_backend::lexer::lex_line(&line.info.raw_line, line.info.line_index)
            .ok()
            .and_then(|tokens| parse_station_keyword_creature_threshold(&tokens))
    })
}

fn station_threshold_condition_label(threshold: i32) -> String {
    format!("{STATION_THRESHOLD_CONDITION_PREFIX}{threshold}")
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
        parse_station_keyword_creature_threshold_for_line(line) == Some(threshold)
    })
}

pub(super) fn run_partner_with_keyword_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(partner_name) = partner_with_name_from_line(ctx.line) else {
        return Ok(None);
    };

    let partner_static_text = format!("partner with {partner_name}");
    let partner_static_line = rewrite_line_normalized(ctx.line, partner_static_text.as_str())?;
    let partner_static = StaticLineCst {
        info: partner_static_line.info.clone(),
        text: partner_static_line.info.normalized.normalized.clone(),
        parse_tokens: partner_static_line.tokens.clone(),
        chosen_option_label: None,
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
    if !tokens_start_with_partner_variant_separator(&ctx.line.tokens) {
        return Ok(None);
    }

    let visible_label = source_before_reminder_or_period(raw, &ctx.line.tokens)
        .unwrap_or(raw)
        .trim()
        .to_string();
    let partner_line = rewrite_line_normalized(ctx.line, "Partner")?;
    if let Some(mut keyword_line) = parse_keyword_line_cst(&partner_line)? {
        keyword_line.text = visible_label;
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Keyword(keyword_line),
            ctx.idx + 1,
        )));
    }

    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(StaticLineCst {
            info: ctx.line.info.clone(),
            text: visible_label,
            parse_tokens: ctx.line.tokens.clone(),
            chosen_option_label: None,
        }),
        ctx.idx + 1,
    )))
}

fn tokens_start_with_partner_variant_separator(tokens: &[OwnedLexToken]) -> bool {
    if tokens.first().is_some_and(first_token_is_partner_variant) {
        return true;
    }
    let words = TokenWordView::new(tokens);
    if words.starts_with(CHARACTER_SELECT_PREFIX) {
        return true;
    }
    if !words.starts_with(PARTNER_PREFIX) {
        return false;
    }
    if words.len() > PARTNER_PREFIX.len()
        && words.get(PARTNER_PREFIX.len()) != Some("with")
        && words.token_index_for_word_index(0) == words.token_index_for_word_index(1)
    {
        return true;
    }
    let Some(separator_start_idx) = words.token_index_after_words(PARTNER_PREFIX.len()) else {
        return false;
    };
    let Some(next_word_idx) = words.token_index_for_word_index(PARTNER_PREFIX.len()) else {
        return false;
    };
    tokens
        .get(separator_start_idx..next_word_idx)
        .is_some_and(|between| {
            between
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
        })
}

fn first_token_is_partner_variant(token: &OwnedLexToken) -> bool {
    let text = token.parser_text().to_ascii_lowercase();
    if text == "partnercharacter" || text == "partnercharacterselect" {
        return true;
    }
    ["-", "\u{2013}", "\u{2014}"].iter().any(|separator| {
        text.split_once(separator)
            .is_some_and(|(head, tail)| head == "partner" && !tail.trim().is_empty())
    })
}

pub(super) fn run_escape_enters_with_counter_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if !line_contains_words(ctx.line, ESCAPES_WITH_PHRASE) {
        return Ok(None);
    }
    Ok(parse_static_line_cst(ctx.line)?.map(|static_cst| {
        LineDispatchResult::single(RewriteLineCst::Static(static_cst), ctx.idx + 1)
    }))
}

pub(super) fn run_surge_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(cost_tokens) = keyword_body_tokens_before_reminder(ctx.line, &["surge"]) else {
        return Ok(None);
    };
    let cost_text = render_keyword_cost_tokens(cost_tokens).trim().to_string();
    if cost_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "surge keyword missing cost: '{}'",
            ctx.line.info.raw_line
        )));
    }

    let rewritten = format!(
        "If you've cast another spell this turn, you may pay {cost_text} rather than pay this spell's mana cost."
    );
    let alternative_line = rewrite_line_normalized(ctx.line, rewritten.as_str())?;
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
    let Some(cost_tokens) = keyword_body_tokens_before_reminder(ctx.line, &["freerunning"]) else {
        return Ok(None);
    };
    let cost_text = render_keyword_cost_tokens(cost_tokens).trim().to_string();
    if cost_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "freerunning keyword missing cost: '{}'",
            ctx.line.info.raw_line
        )));
    }

    let rewritten = format!(
        "If you dealt combat damage to a player this turn with an Assassin or commander, you may pay {cost_text} rather than pay this spell's mana cost."
    );
    let alternative_line = rewrite_line_normalized(ctx.line, rewritten.as_str())?;
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

pub(super) fn run_keyword_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    Ok(parse_keyword_line_cst(ctx.line)?.map(|keyword_line| {
        LineDispatchResult::single(RewriteLineCst::Keyword(keyword_line), ctx.idx + 1)
    }))
}

pub(super) fn run_additional_combat_after_this_phase_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let raw = ctx.line.info.raw_line.trim();
    if !line_contains_words(ctx.line, ADDITIONAL_COMBAT_AFTER_THIS_PHASE_PHRASE)
        && !token_slice_words_eq(
            &ctx.line.tokens,
            ADDITIONAL_COMBAT_AFTER_THIS_MAIN_PHASE_LINE,
        )
    {
        return Ok(None);
    }

    let rewritten = raw
        .replace(
            "If it's your main phase, there is an additional combat phase after this phase, followed by an additional main phase",
            "After this main phase, there is an additional combat phase followed by an additional main phase",
        )
        .replace(
            "if it's your main phase, there is an additional combat phase after this phase, followed by an additional main phase",
            "after this main phase, there is an additional combat phase followed by an additional main phase",
        )
        .replace(
            "there is an additional combat phase after this phase, followed by an additional main phase",
            "after this main phase, there is an additional combat phase followed by an additional main phase",
        );
    let rewritten_line = rewrite_line_normalized(ctx.line, rewritten.as_str())?;
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
                    chosen_option_label: None,
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
    let original_tokens =
        lex_line(line.info.normalized.original.as_str(), line.info.line_index).ok()?;
    let (label, _, body_tokens) = split_label_prefix_lexed(&original_tokens)?;
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
                tokens_start_with_partner_variant_separator(&tokens),
                "{line} should be recognized as a partner variant"
            );
        }

        let tokens = lex_line("Partner with Proud Mentor", 0).unwrap();
        assert!(!tokens_start_with_partner_variant_separator(&tokens));
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
            source_before_reminder_or_period(partner_variant_line, &partner_variant_tokens),
            Some("Partner - Friends forever")
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
        assert_eq!(
            parse_station_threshold_line(&station_line),
            Some((
                6,
                "This artifact is a creature in addition to its other types.".to_string()
            ))
        );

        let missing_plus = line("6 | This artifact is a creature.");
        assert_eq!(parse_station_threshold_line(&missing_plus), None);
    }
}

fn partner_with_name_from_line(line: &PreprocessedLine) -> Option<String> {
    let tokens = &line.tokens;
    if !PARTNER_WITH_PREFIX_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(tokens))
    {
        return None;
    }

    let words = TokenWordView::new(tokens);
    let name_start_idx = words.token_index_for_word_index(PARTNER_WITH_PREFIX.len())?;
    let name_end_idx = tokens[name_start_idx..]
        .iter()
        .position(|token| matches!(token.kind, TokenKind::LParen | TokenKind::Period))
        .map(|idx| name_start_idx + idx)
        .unwrap_or(tokens.len());
    let name_tokens = tokens.get(name_start_idx..name_end_idx)?;
    let name = render_original_text_for_token_slice(line, name_tokens)
        .unwrap_or_else(|| render_token_slice(name_tokens))
        .trim()
        .replace('"', "");
    (!name.is_empty()).then(|| name.to_string())
}

fn source_before_reminder_or_period<'a>(
    raw_line: &'a str,
    tokens: &[OwnedLexToken],
) -> Option<&'a str> {
    let end = tokens
        .iter()
        .find(|token| matches!(token.kind, TokenKind::LParen | TokenKind::Period))
        .map(|token| token.span.start)
        .unwrap_or(raw_line.len());
    let display = raw_line.get(..end)?.trim();
    (!display.is_empty()).then_some(display)
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

pub(super) fn run_non_turn_conditional_untap_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(first_sentence_tokens) = non_turn_conditional_untap_first_sentence_tokens(ctx.line)
    else {
        return Ok(None);
    };
    if !CREATURES_YOU_CONTROL_GET_PREFIX_PATTERN.matches_words(
        &crate::runtime_backend::token_word_refs(first_sentence_tokens),
    ) {
        return Ok(None);
    }
    let first_sentence = render_original_text_for_token_slice(ctx.line, first_sentence_tokens)
        .unwrap_or_else(|| render_tokens_without_terminal_period(first_sentence_tokens))
        .trim()
        .to_string();

    let first_line = rewrite_line_normalized(ctx.line, first_sentence.as_str())?;
    let Some(first_statement) = parse_statement_line_cst(&first_line)? else {
        return Ok(None);
    };

    let second_line =
        rewrite_line_normalized(ctx.line, "If it's not your turn, untap those creatures")?;
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

fn non_turn_conditional_untap_first_sentence_tokens(
    line: &PreprocessedLine,
) -> Option<&[OwnedLexToken]> {
    let words = TokenWordView::new(&line.tokens);
    if !NON_TURN_UNTAP_SUFFIX_PATTERN
        .matches_words(&crate::runtime_backend::token_word_refs(&line.tokens))
    {
        return None;
    }
    let suffix_word_idx = words.len().checked_sub(NON_TURN_UNTAP_SUFFIX.len())?;
    let suffix_token_idx = words.token_index_for_word_index(suffix_word_idx)?;
    let prefix_tokens = line.tokens.get(..suffix_token_idx)?;
    prefix_tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
        .then_some(tokens_without_terminal_period(prefix_tokens))
        .filter(|tokens| !tokens.is_empty())
}

fn statement_probe_shape_prefers_statement(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    LINKED_CHOOSE_TWO_SHUFFLE_REST_BATTLEFIELD_PATTERN.matches_words(&words)
        || LINKED_EXILED_CARD_COST_MORE_PATTERN.matches_words(&words)
}

pub(super) fn run_statement_probe_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if (matches!(
        crate::runtime_backend::grammar::structure::classify_statement_line_family_lexed(
            &ctx.line.tokens
        ),
        Some(
            crate::runtime_backend::grammar::structure::StatementLineFamily::Divvy
                | crate::runtime_backend::grammar::structure::StatementLineFamily::PactNextUpkeep
                | crate::runtime_backend::grammar::structure::StatementLineFamily::ExilePlayCostsMore
        )
    ) || statement_probe_shape_prefers_statement(&ctx.line.tokens)
        || looks_like_statement_line_lexed(ctx.line)
        || should_prefer_statement_before_static_for_nonpermanent_spell(
            ctx.preprocessed,
            &ctx.line.tokens,
        ))
        && !is_can_block_additional_creatures_static_line(&ctx.line.tokens)
        && !is_draw_replacement_reveal_top_static_line(&ctx.line.tokens)
        && let Some(statement_line) = parse_statement_line_cst(ctx.line)?
    {
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Statement(statement_line),
            ctx.idx + 1,
        )));
    }
    Ok(None)
}

fn is_draw_replacement_reveal_top_static_line(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    words.starts_with(&[
        "if", "you", "would", "draw", "a", "card", "instead", "reveal", "the", "top",
    ])
}

fn is_can_block_additional_creatures_static_line(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !CAN_BLOCK_ADDITIONAL_PREFIX_PATTERN.matches_words(&words) {
        return false;
    }

    let has_additional = ADDITIONAL_WORD_PATTERN.matches_words(&words);
    let has_creature_noun = CREATURE_OR_CREATURES_WORD_PATTERN.matches_words(&words);
    if !has_additional || !has_creature_noun {
        return false;
    }

    BLOCK_DURATION_TAIL_PATTERN.matches_words(&words)
}

pub(super) fn run_static_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    match parse_static_line_cst(ctx.line) {
        Ok(static_line) => Ok(static_line.map(|static_line| {
            LineDispatchResult::single(RewriteLineCst::Static(static_line), ctx.idx + 1)
        })),
        Err(err) if looks_like_statement_line_lexed(ctx.line) => {
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
