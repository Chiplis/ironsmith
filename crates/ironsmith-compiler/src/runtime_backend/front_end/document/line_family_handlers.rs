use super::super::token_primitives::{
    str_contains, str_ends_with, str_ends_with_any_char, str_find, str_find_char, str_rfind,
    str_split_once, str_split_once_char, str_starts_with, str_starts_with_char, str_strip_prefix,
    str_strip_suffix,
};
use super::line_dispatch::{LineDispatchContext, LineDispatchResult};
use super::*;

const MAX_SPEED_CONDITION_LABEL: &str = "__max_speed_condition";
const CONTROL_COLOR_PAIR_PERMANENT_CONDITION_PREFIX: &str = "__control_color_pair_permanent_";
const STATION_THRESHOLD_CONDITION_PREFIX: &str = "__station_threshold_";

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

    let Some((cost, body)) = str_split_once_char(ctx.line.info.raw_line.as_str(), '—') else {
        return false;
    };
    let mut remainder = cost.trim().to_ascii_lowercase();
    let mut saw_ticket_symbol = false;
    while let Some(next) = str_strip_prefix(remainder.as_str(), "{tk}") {
        saw_ticket_symbol = true;
        remainder = next.trim_start().to_string();
    }

    saw_ticket_symbol && remainder.is_empty() && !body.trim().is_empty()
}

pub(super) fn run_triggered_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    try_parse_triggered_line_dispatch(ctx.preprocessed, ctx.idx, ctx.line, ctx.allow_unsupported)
}

pub(super) fn run_championed_with_this_trigger_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let raw = ctx.line.info.raw_line.trim();
    let lower = raw.to_ascii_lowercase();
    if !str_starts_with(lower.as_str(), "when ")
        || !str_contains(lower.as_str(), " is championed with this ")
    {
        return Ok(None);
    }
    let Some((_, effect_text)) = str_split_once_char(raw, ',') else {
        return Ok(None);
    };
    let triggered_text = format!(
        "When this creature enters, {}",
        effect_text.trim_start().trim_end_matches('.')
    );
    let triggered_line = rewrite_line_normalized(ctx.line, triggered_text.as_str())?;
    let triggered = parse_triggered_line_cst(&triggered_line)?;
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Triggered(triggered),
        ctx.idx + 1,
    )))
}

pub(super) fn run_max_speed_labeled_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let raw = ctx.line.info.raw_line.trim_start();
    if !str_starts_with(raw.to_ascii_lowercase().as_str(), "max speed") {
        return Ok(None);
    };

    let body_text = str_find_char(raw, '\u{2014}')
        .and_then(|idx| raw.get(idx + '\u{2014}'.len_utf8()..))
        .or_else(|| str_split_once_char(raw, '-').map(|(_, body)| body))
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
    if str_starts_with(body_lower.as_str(), "when ")
        || str_starts_with(body_lower.as_str(), "whenever ")
        || str_starts_with(body_lower.as_str(), "at ")
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
    let Some((trigger, effects)) = str_split_once_char(trimmed, ',') else {
        return trimmed.to_string();
    };
    format!("{trigger}, if you have max speed,{effects}")
}

pub(super) fn run_start_your_engines_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let lower = ctx.line.info.raw_line.trim_start().to_ascii_lowercase();
    if !str_starts_with(lower.as_str(), "start your engines!")
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

pub(super) fn run_case_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let raw = ctx.line.info.raw_line.trim();
    let lower = raw.to_ascii_lowercase();

    if str_starts_with(lower.as_str(), "to solve") {
        return parse_case_to_solve_line(ctx, raw);
    }
    if str_starts_with(lower.as_str(), "solved") {
        return parse_case_solved_line(ctx, raw);
    }

    Ok(None)
}

fn parse_case_to_solve_line(
    ctx: &LineDispatchContext<'_>,
    raw: &str,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some((label, body)) = str_split_once_char(raw, '\u{2014}') else {
        return Ok(None);
    };
    if label.trim().to_ascii_lowercase() != "to solve" {
        return Ok(None);
    }

    let condition = body
        .split("(If unsolved")
        .next()
        .unwrap_or(body)
        .trim()
        .trim_end_matches('.')
        .trim();
    if condition.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "case solve line missing condition: '{}'",
            ctx.line.info.raw_line
        )));
    }

    let triggered_text = format!("At the beginning of your end step, if {condition}, solve.");
    let triggered_line = rewrite_line_normalized(ctx.line, triggered_text.as_str())?;
    let mut triggered = parse_triggered_line_cst(&triggered_line)?;
    let unsolved_condition = PredicateAst::Not(Box::new(PredicateAst::SourceChosenOption(
        "solved".to_string(),
    )));
    triggered.intervening_if = Some(match triggered.intervening_if.take() {
        Some(condition) => PredicateAst::And(Box::new(condition), Box::new(unsolved_condition)),
        None => unsolved_condition,
    });
    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Triggered(triggered),
        ctx.idx + 1,
    )))
}

fn parse_case_solved_line(
    ctx: &LineDispatchContext<'_>,
    raw: &str,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some((label, body)) = str_split_once_char(raw, '\u{2014}') else {
        return Ok(None);
    };
    if label.trim().to_ascii_lowercase() != "solved" {
        return Ok(None);
    }

    let body = body.trim();
    if body.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "case solved line missing ability body: '{}'",
            ctx.line.info.raw_line
        )));
    }

    let mut lines = Vec::new();
    for ability_text in split_case_solved_static_body(body) {
        let ability_line = rewrite_line_normalized(ctx.line, ability_text.as_str())?;
        let Some(mut static_line) = parse_static_line_cst(&ability_line)? else {
            return Err(CardTextError::ParseError(format!(
                "parser could not lower case solved line: '{}'",
                ctx.line.info.raw_line
            )));
        };
        static_line.chosen_option_label = Some("solved".to_string());
        lines.push(RewriteLineCst::Static(static_line));
    }

    Ok(Some(LineDispatchResult {
        lines,
        next_idx: ctx.idx + 1,
    }))
}

fn split_case_solved_static_body(body: &str) -> Vec<String> {
    let trimmed = body.trim().trim_end_matches('.').trim();
    let lower = trimmed.to_ascii_lowercase();
    let prefix = "you may look at the top card of your library any time, and you may ";
    if let Some(rest) = lower.strip_prefix(prefix)
        && !rest.is_empty()
        && let Some(original_rest) = trimmed.get(prefix.len()..)
    {
        return vec![
            "You may look at the top card of your library any time.".to_string(),
            format!("You may {}.", original_rest.trim()),
        ];
    }

    vec![format!("{}.", trimmed)]
}

pub(super) fn run_draft_rule_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    if !is_draft_rule_line(ctx.line.info.raw_line.as_str()) {
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

fn is_draft_rule_line(raw_line: &str) -> bool {
    let lower = raw_line.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }

    lower.trim_end_matches('.') == "draft this card face up"
        || lower.starts_with("reveal this card as you draft it")
        || lower.starts_with("as you draft ")
        || lower.starts_with("during the draft, ")
        || lower.starts_with("immediately after the draft, ")
        || lower.starts_with("each player passes ") && lower.contains("booster pack")
}

pub(super) fn run_learn_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let lower = ctx.line.info.raw_line.trim().to_ascii_lowercase();
    if !str_starts_with(lower.as_str(), "learn.") && lower.trim_end_matches('.') != "learn" {
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
    let lower = ctx.line.info.raw_line.trim().to_ascii_lowercase();
    let phrase = "you may look at the top card of your library and at face-down creatures you don't control any time";
    if lower.trim_end_matches('.') != phrase {
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
    let lower = ctx.line.info.raw_line.trim().to_ascii_lowercase();
    let phrase = "you may look at the top card of your library any time, and you may play lands from the top of your library";
    if lower.trim_end_matches('.') != phrase {
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
    let raw = ctx.line.info.raw_line.trim();
    let lower = raw.to_ascii_lowercase();
    let phrase = "enchanted creature's controller may have it assign its combat damage as though it weren't blocked";
    if lower.trim_end_matches('.') != phrase {
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
    let raw = ctx.line.info.raw_line.trim();
    let raw_no_period = raw.trim_end_matches('.');
    let lower = raw_no_period.to_ascii_lowercase();
    let prefix = "you may cast this card from your graveyard as long as you control a ";
    if !str_starts_with(lower.as_str(), prefix) || !str_ends_with(lower.as_str(), " permanent") {
        return Ok(None);
    }

    let Some(condition_text) = raw_no_period.get(prefix.len()..) else {
        return Ok(None);
    };
    let condition_text = condition_text.trim();
    let Some(color_pair_text) = str_strip_suffix(condition_text, " permanent") else {
        return Ok(None);
    };
    let color_pair_text = color_pair_text.trim();
    let Some((left, right)) = str_split_once(color_pair_text, " or ") else {
        return Ok(None);
    };

    let left = left.trim();
    let right = right.trim();
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

pub(super) fn run_champion_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let raw = ctx.line.info.raw_line.trim();
    let lower = raw.to_ascii_lowercase();
    if !str_starts_with(lower.as_str(), "champion ") {
        return Ok(None);
    }

    let without_reminder = str_split_once_char(raw, '(')
        .map(|(prefix, _)| prefix)
        .unwrap_or(raw)
        .trim()
        .trim_end_matches('.');
    let Some(filter_text) = str_strip_prefix(without_reminder, "Champion ") else {
        return Ok(None);
    };
    let filter_text = str_strip_prefix(filter_text, "a ")
        .or_else(|| str_strip_prefix(filter_text, "an "))
        .unwrap_or(filter_text)
        .trim();
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
    let lower = ctx.line.info.raw_line.trim().to_ascii_lowercase();
    if !str_starts_with(lower.as_str(), "station")
        || (!str_starts_with(lower.as_str(), "station ")
            && !str_starts_with(lower.as_str(), "station(")
            && lower.trim_end_matches('.') != "station")
    {
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
    let cost = parse_activation_cost_tokens_rewrite(&cost_tokens)?;
    let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();

    let mut lines = vec![RewriteLineCst::Activated(ActivatedLineCst {
        info: ctx.line.info.clone(),
        cost,
        cost_parse_tokens: cost_tokens,
        effect_text,
        effect_parse_tokens,
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
        .any(|line| parse_station_threshold_line(line.info.raw_line.as_str()).is_some());
    if !has_explicit_station_threshold_rows
        && let Some(threshold) = parse_station_keyword_creature_threshold(&lower)
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
    let Some((threshold, mut body_text)) =
        parse_station_threshold_line(ctx.line.info.raw_line.as_str())
    else {
        return Ok(None);
    };
    if let Some(rewritten) =
        normalize_named_source_sentence_for_builder(&ctx.preprocessed.builder, body_text.as_str())
    {
        body_text = rewritten;
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
    let body_lower = body_text.to_ascii_lowercase();
    if str_starts_with(body_lower.as_str(), "when ")
        || str_starts_with(body_lower.as_str(), "whenever ")
        || str_starts_with(body_lower.as_str(), "at ")
    {
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
        let cost = parse_activation_cost_tokens_rewrite(&cost_tokens)?;
        let effect_text = render_token_slice(&effect_parse_tokens).trim().to_string();
        lines.push(RewriteLineCst::Activated(ActivatedLineCst {
            info: ctx.line.info.clone(),
            cost,
            cost_parse_tokens: cost_tokens,
            effect_text,
            effect_parse_tokens,
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

fn parse_station_threshold_line(raw_line: &str) -> Option<(i32, String)> {
    let (prefix, body) = str_split_once_char(raw_line, '|')?;
    let threshold = prefix
        .trim()
        .strip_suffix('+')?
        .trim()
        .parse::<i32>()
        .ok()?;
    let body = body.trim();
    (!body.is_empty()).then(|| (threshold, body.to_string()))
}

fn parse_station_keyword_creature_threshold(lower: &str) -> Option<i32> {
    let marker = "artifact creature at ";
    let start = str_find(lower, marker)? + marker.len();
    let tail = &lower[start..];
    let digits = tail
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() || !str_starts_with_char(&tail[digits.len()..], '+') {
        return None;
    }
    digits.parse().ok()
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
    let needle = format!("artifact creature at {threshold}+");
    ctx.preprocessed.items.iter().any(|item| {
        let PreprocessedItem::Line(line) = item else {
            return false;
        };
        let lower = line.info.raw_line.to_ascii_lowercase();
        str_contains(lower.as_str(), needle.as_str())
    })
}

pub(super) fn run_partner_with_keyword_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let Some(partner_name) = partner_with_name_from_line(ctx.line.info.raw_line.as_str()) else {
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
    let lower = raw.to_ascii_lowercase();
    if !str_starts_with(lower.as_str(), "partner") {
        return Ok(None);
    }

    let Some(rest) = raw.get("Partner".len()..) else {
        return Ok(None);
    };
    let rest = rest.trim_start();
    if !(str_starts_with_char(rest, '\u{2014}')
        || str_starts_with(rest, "-")
        || str_starts_with_char(rest, '\u{2013}'))
    {
        return Ok(None);
    }

    let partner_line = rewrite_line_normalized(ctx.line, "Partner")?;
    if let Some(mut keyword_line) = parse_keyword_line_cst(&partner_line)? {
        let visible_label = str_split_once_char(raw, '(')
            .map(|(head, _)| head)
            .unwrap_or(raw)
            .trim()
            .to_string();
        keyword_line.text = visible_label;
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Keyword(keyword_line),
            ctx.idx + 1,
        )));
    }

    Ok(Some(LineDispatchResult::single(
        RewriteLineCst::Static(StaticLineCst {
            info: partner_line.info.clone(),
            text: partner_line.info.normalized.normalized.clone(),
            parse_tokens: partner_line.tokens.clone(),
            chosen_option_label: None,
        }),
        ctx.idx + 1,
    )))
}

pub(super) fn run_escape_enters_with_counter_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let raw = ctx.line.info.raw_line.trim();
    let lower = raw.to_ascii_lowercase();
    if !str_contains(lower.as_str(), " escapes with ") {
        return Ok(None);
    }
    Ok(parse_static_line_cst(ctx.line)?.map(|static_cst| {
        LineDispatchResult::single(RewriteLineCst::Static(static_cst), ctx.idx + 1)
    }))
}

pub(super) fn run_surge_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let raw = ctx.line.info.raw_line.trim();
    let Some(rest) = str_strip_prefix(raw, "Surge ") else {
        return Ok(None);
    };
    let cost_text = str_split_once_char(rest, '(')
        .map(|(prefix, _)| prefix)
        .unwrap_or(rest)
        .trim()
        .trim_end_matches('.')
        .trim();
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
    let raw = ctx.line.info.raw_line.trim();
    let Some(rest) = str_strip_prefix(raw, "Freerunning ") else {
        return Ok(None);
    };
    let cost_text = str_split_once_char(rest, '(')
        .map(|(prefix, _)| prefix)
        .unwrap_or(rest)
        .trim()
        .trim_end_matches('.')
        .trim();
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
    let needle = "there is an additional combat phase after this phase, followed by an additional main phase";
    let raw = ctx.line.info.raw_line.trim();
    if !str_contains(raw.to_ascii_lowercase().as_str(), needle) {
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
    if !str_starts_with(lower.as_str(), "partner with ") {
        return None;
    }

    let rest = trimmed.get(rest_start..)?.trim();
    let name = str_split_once_char(rest, '(')
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

pub(super) fn run_non_turn_conditional_untap_line_family(
    ctx: &LineDispatchContext<'_>,
) -> Result<Option<LineDispatchResult>, CardTextError> {
    let raw = ctx.line.info.raw_line.trim();
    let lower = raw.to_ascii_lowercase();
    const SUFFIX: &str = "if it's not your turn, untap those creatures.";
    if !str_ends_with(lower.as_str(), SUFFIX) {
        return Ok(None);
    }

    let marker = format!(". {SUFFIX}");
    let Some(split_idx) = str_rfind(lower.as_str(), marker.as_str()) else {
        return Ok(None);
    };
    let Some(prefix) = raw.get(..split_idx) else {
        return Ok(None);
    };

    let first_sentence = prefix.trim();
    if first_sentence.is_empty() {
        return Ok(None);
    }

    let first_sentence_lower = first_sentence.to_ascii_lowercase();
    if !str_starts_with(first_sentence_lower.as_str(), "creatures you control get ") {
        return Ok(None);
    }

    let first_line = rewrite_line_normalized(ctx.line, first_sentence.trim_end_matches('.'))?;
    let Some(first_statement) = parse_statement_line_cst(&first_line)? else {
        return Ok(None);
    };

    let second_line = rewrite_line_normalized(ctx.line, "If it's not your turn, untap them")?;
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
    ) || (token_words_have_sequence(&ctx.line.tokens, &["chooses", "two", "of", "those", "cards"])
        && token_words_have_sequence(&ctx.line.tokens, &["shuffle", "the", "chosen", "cards"])
        && token_words_have_sequence(
            &ctx.line.tokens,
            &["put", "the", "rest", "onto", "the", "battlefield"],
        ))
        || (token_words_have_sequence(
            &ctx.line.tokens,
            &["for", "as", "long", "as", "that", "card", "remains", "exiled"],
        ) && token_words_have_sequence(&ctx.line.tokens, &["more", "to", "cast"]))
        || looks_like_statement_line_lexed(ctx.line)
        || should_prefer_statement_before_static_for_nonpermanent_spell(
            ctx.preprocessed,
            &ctx.line.tokens,
        ))
        && !is_can_block_additional_creatures_static_line(&ctx.line.tokens)
        && let Some(statement_line) = parse_statement_line_cst(ctx.line)?
    {
        return Ok(Some(LineDispatchResult::single(
            RewriteLineCst::Statement(statement_line),
            ctx.idx + 1,
        )));
    }
    Ok(None)
}

fn is_can_block_additional_creatures_static_line(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !word_slice_starts_with(&words, &["this", "creature", "can", "block"]) {
        return false;
    }

    let has_additional =
        crate::runtime_backend::lexer::word_slice_contains_word(&words, "additional");
    let has_creature_noun = words
        .iter()
        .any(|word| *word == "creature" || *word == "creatures");
    if !has_additional || !has_creature_noun {
        return false;
    }

    word_slice_ends_with_any(&words, &[&["each", "combat"], &["this", "turn"]])
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
