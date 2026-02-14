use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};

use ironsmith::cards::CardDefinitionBuilder;
use ironsmith::compiled_text::compiled_lines;
use ironsmith::ids::CardId;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug)]
struct Args {
    cards_path: String,
    limit: Option<usize>,
    min_cluster_size: usize,
    top_clusters: usize,
    examples_per_cluster: usize,
    json_out: Option<String>,
    parser_trace: bool,
    trace_name: Option<String>,
    allow_unsupported: bool,
    use_embeddings: bool,
    embedding_dims: usize,
    embedding_threshold: f32,
    mismatch_names_out: Option<String>,
    false_positive_names: Option<String>,
    failures_out: Option<String>,
}

#[derive(Debug, Clone)]
struct CardInput {
    name: String,
    oracle_text: String,
    parse_input: String,
}

#[derive(Debug, Clone)]
struct CardAudit {
    name: String,
    oracle_text: String,
    cluster_key: String,
    parse_error: Option<String>,
    compiled_lines: Vec<String>,
    oracle_coverage: f32,
    compiled_coverage: f32,
    similarity_score: f32,
    line_delta: isize,
    semantic_mismatch: bool,
    semantic_false_positive: bool,
}

#[derive(Debug, Serialize)]
struct JsonReport {
    cards_processed: usize,
    parse_failures: usize,
    semantic_mismatches: usize,
    semantic_false_positives: usize,
    clusters_total: usize,
    clusters_reported: usize,
    clusters: Vec<JsonCluster>,
}

#[derive(Debug, Serialize)]
struct JsonCluster {
    signature: String,
    size: usize,
    parse_failures: usize,
    semantic_mismatches: usize,
    semantic_false_positives: usize,
    parse_failure_rate: f32,
    semantic_mismatch_rate: f32,
    top_errors: Vec<JsonErrorCount>,
    examples: Vec<JsonExample>,
}

#[derive(Debug, Serialize)]
struct JsonErrorCount {
    error: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct JsonExample {
    name: String,
    parse_error: Option<String>,
    oracle_coverage: f32,
    compiled_coverage: f32,
    similarity_score: f32,
    line_delta: isize,
    oracle_excerpt: String,
    compiled_excerpt: String,
    oracle_text: String,
    compiled_lines: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonFailureReport {
    threshold: f32,
    cards_processed: usize,
    failures: usize,
    entries: Vec<JsonFailureEntry>,
}

#[derive(Debug, Serialize)]
struct JsonFailureEntry {
    name: String,
    parse_error: Option<String>,
    oracle_coverage: f32,
    compiled_coverage: f32,
    similarity_score: f32,
    line_delta: isize,
    oracle_text: String,
    compiled_text: String,
    compiled_lines: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = "cards.json".to_string();
    let mut limit = None;
    let mut min_cluster_size = 8usize;
    let mut top_clusters = 30usize;
    let mut examples_per_cluster = 3usize;
    let mut json_out = None;
    let mut parser_trace = false;
    let mut trace_name = None;
    let mut allow_unsupported = false;
    let mut use_embeddings = false;
    let mut embedding_dims = 384usize;
    let mut embedding_threshold = 0.17f32;
    let mut mismatch_names_out = None;
    let mut false_positive_names = None;
    let mut failures_out = None;

    let mut iter = env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--cards" => {
                cards_path = iter
                    .next()
                    .ok_or_else(|| "--cards requires a path".to_string())?;
            }
            "--limit" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--limit requires a number".to_string())?;
                limit = Some(
                    raw.parse::<usize>()
                        .map_err(|e| format!("invalid --limit value '{raw}': {e}"))?,
                );
            }
            "--min-cluster-size" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--min-cluster-size requires a number".to_string())?;
                min_cluster_size = raw
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --min-cluster-size value '{raw}': {e}"))?;
            }
            "--top-clusters" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--top-clusters requires a number".to_string())?;
                top_clusters = raw
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --top-clusters value '{raw}': {e}"))?;
            }
            "--examples" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--examples requires a number".to_string())?;
                examples_per_cluster = raw
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --examples value '{raw}': {e}"))?;
            }
            "--json-out" => {
                json_out = Some(
                    iter.next()
                        .ok_or_else(|| "--json-out requires a path".to_string())?,
                );
            }
            "--parser-trace" => {
                parser_trace = true;
            }
            "--trace-name" => {
                trace_name = Some(
                    iter.next()
                        .ok_or_else(|| "--trace-name requires a card-name substring".to_string())?
                        .to_ascii_lowercase(),
                );
            }
            "--allow-unsupported" => {
                allow_unsupported = true;
            }
            "--use-embeddings" => {
                use_embeddings = true;
            }
            "--embedding-dims" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--embedding-dims requires a number".to_string())?;
                embedding_dims = raw
                    .parse::<usize>()
                    .map_err(|e| format!("invalid --embedding-dims value '{raw}': {e}"))?;
            }
            "--embedding-threshold" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--embedding-threshold requires a float".to_string())?;
                embedding_threshold = raw
                    .parse::<f32>()
                    .map_err(|e| format!("invalid --embedding-threshold value '{raw}': {e}"))?;
            }
            "--mismatch-names-out" => {
                mismatch_names_out = Some(
                    iter.next()
                        .ok_or_else(|| "--mismatch-names-out requires a path".to_string())?,
                );
            }
            "--false-positive-names" => {
                false_positive_names = Some(
                    iter.next()
                        .ok_or_else(|| "--false-positive-names requires a path".to_string())?,
                );
            }
            "--failures-out" => {
                failures_out = Some(
                    iter.next()
                        .ok_or_else(|| "--failures-out requires a path".to_string())?,
                );
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. supported: --cards <path> --limit <n> --min-cluster-size <n> --top-clusters <n> --examples <n> --json-out <path> --parser-trace --trace-name <substring> --allow-unsupported --use-embeddings --embedding-dims <n> --embedding-threshold <f32> --mismatch-names-out <path> --false-positive-names <path> --failures-out <path>"
                ));
            }
        }
    }

    Ok(Args {
        cards_path,
        limit,
        min_cluster_size,
        top_clusters,
        examples_per_cluster,
        json_out,
        parser_trace,
        trace_name,
        allow_unsupported,
        use_embeddings,
        embedding_dims,
        embedding_threshold,
        mismatch_names_out,
        false_positive_names,
        failures_out,
    })
}

fn strip_parenthetical(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    for ch in text.chars() {
        if ch == '(' {
            depth += 1;
            continue;
        }
        if ch == ')' {
            depth = depth.saturating_sub(1);
            continue;
        }
        if depth == 0 {
            out.push(ch);
        }
    }
    out
}

fn looks_like_reminder_quote(content: &str) -> bool {
    let lower = content
        .trim()
        .trim_matches('"')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    lower.starts_with("{t}, sacrifice this artifact: add one mana of any color")
        || lower.starts_with("sacrifice this token: add {c}")
        || lower.starts_with("sacrifice this creature: add {c}")
        || lower.starts_with("{2}, {t}, sacrifice this token: draw a card")
        || lower.starts_with("{2}, sacrifice this token: you gain 3 life")
        || lower.starts_with("when this token dies")
        || lower.starts_with("when this token leaves the battlefield")
}

fn strip_reminder_like_quotes(text: &str) -> String {
    let mut out = String::new();
    let mut in_quote = false;
    let mut quoted = String::new();

    for ch in text.chars() {
        if ch == '"' {
            if in_quote {
                if !looks_like_reminder_quote(&quoted) {
                    out.push('"');
                    out.push_str(&quoted);
                    out.push('"');
                } else {
                    for prefix in ["It has ", "it has ", "They have ", "they have ", "with ", "With "] {
                        if out.ends_with(prefix) {
                            let keep = out.len().saturating_sub(prefix.len());
                            if out.is_char_boundary(keep) {
                                out.truncate(keep);
                            }
                            break;
                        }
                    }
                }
                quoted.clear();
                in_quote = false;
            } else {
                in_quote = true;
            }
            continue;
        }
        if in_quote {
            quoted.push(ch);
        } else {
            out.push(ch);
        }
    }

    if in_quote {
        out.push('"');
        out.push_str(&quoted);
    }

    out
}

fn strip_inline_token_reminders(text: &str) -> String {
    text.replace(
        " with Sacrifice this creature: Add {C}. under your control",
        "",
    )
    .replace(
        " with Sacrifice this token: Add {C}. under your control",
        "",
    )
    .replace(
        " with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control",
        "",
    )
    .replace(
        " with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped",
        "",
    )
    .replace(
        " It has \"{T}, Sacrifice this artifact: Add one mana of any color.\"",
        "",
    )
    .replace(" It has \"Sacrifice this token: Add {C}.\"", "")
    .replace(" It has \"Sacrifice this creature: Add {C}.\"", "")
}

fn parenthetical_segments(text: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut depth = 0u32;
    let mut current = String::new();

    for ch in text.chars() {
        if ch == '(' {
            if depth > 0 {
                current.push(ch);
            }
            depth += 1;
            continue;
        }
        if ch == ')' {
            if depth == 0 {
                continue;
            }
            depth -= 1;
            if depth == 0 {
                let segment = current.trim();
                if !segment.is_empty() {
                    segments.push(segment.to_string());
                }
                current.clear();
            } else {
                current.push(ch);
            }
            continue;
        }
        if depth > 0 {
            current.push(ch);
        }
    }

    segments
}

fn push_semantic_clauses(line: &str, clauses: &mut Vec<String>) {
    let mut current = String::new();
    for ch in line.chars() {
        if matches!(ch, '.' | ';' | '\n') {
            let trimmed = current.trim();
            if !trimmed.is_empty() && trimmed.chars().any(|ch| ch.is_ascii_alphanumeric()) {
                clauses.push(strip_ability_word_prefix(trimmed));
            }
            current.clear();
        } else {
            current.push(ch);
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() && trimmed.chars().any(|ch| ch.is_ascii_alphanumeric()) {
        clauses.push(strip_ability_word_prefix(trimmed));
    }
}

fn normalize_trigger_subject_for_compare(line: &str) -> String {
    let trimmed = line.trim();
    for prefix in ["When ", "Whenever "] {
        if !trimmed.starts_with(prefix) {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        let marker = [
            " becomes ",
            " become ",
            " enters",
            " attacks",
            " blocks",
            " dies",
            " deals ",
            " is turned face up",
        ]
        .iter()
        .filter_map(|needle| lower.find(needle))
        .min();
        let Some(idx) = marker else {
            continue;
        };
        if idx <= prefix.len() {
            continue;
        }
        let subject = trimmed[prefix.len()..idx].trim();
        if subject.is_empty() {
            continue;
        }
        let subject_lower = subject.to_ascii_lowercase();
        let starts_with_generic = [
            "a ",
            "an ",
            "the ",
            "this ",
            "that ",
            "target ",
            "another ",
            "other ",
            "each ",
            "all ",
            "any ",
            "up to ",
            "one or more ",
        ]
        .iter()
        .any(|start| subject_lower.starts_with(start));
        if starts_with_generic {
            continue;
        }
        let contains_generic_noun = [
            " creature",
            " permanent",
            " land",
            " artifact",
            " enchantment",
            " planeswalker",
            " battle",
            " token",
            " player",
            " opponent",
            " spell",
            " card",
        ]
        .iter()
        .any(|needle| subject_lower.contains(needle));
        if contains_generic_noun {
            continue;
        }
        if !subject
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            continue;
        }
        let tail = &trimmed[idx..];
        let replacement_subject = if tail.starts_with(" enters") {
            "this permanent"
        } else {
            "this creature"
        };
        return format!("{prefix}{replacement_subject}{tail}");
    }
    trimmed.to_string()
}

fn split_common_semantic_conjunctions(line: &str) -> String {
    let mut normalized = line.to_string();
    normalized = normalized.replace(
        "Whenever another creature enters under your control",
        "Whenever another creature you control enters",
    );
    normalized = normalized.replace(
        "whenever another creature enters under your control",
        "whenever another creature you control enters",
    );
    if normalized.starts_with("You draw ") {
        normalized = normalized.replace(" and you lose ", " and lose ");
        normalized = normalized.replace(" and you gain ", " and gain ");
    }
    if let Some(rest) = normalized.strip_prefix("Opponent's creatures get ") {
        normalized = format!("Creatures your opponents control get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("opponent's creatures get ") {
        normalized = format!("creatures your opponents control get {rest}");
    }
    for (from, to) in [
        (": you draw ", ": draw "),
        (": You draw ", ": Draw "),
        (", you draw ", ", draw "),
        (", You draw ", ", Draw "),
        (": you mill ", ": mill "),
        (": You mill ", ": Mill "),
        (", you mill ", ", mill "),
        (", You mill ", ", Mill "),
        (": you scry ", ": scry "),
        (", you scry ", ", scry "),
        (": you surveil ", ": surveil "),
        (", you surveil ", ", surveil "),
    ] {
        normalized = normalized.replace(from, to);
    }
    for (from, to) in [
        (
            ". until this enchantment leaves the battlefield",
            " until this enchantment leaves the battlefield",
        ),
        (
            ". until this artifact leaves the battlefield",
            " until this artifact leaves the battlefield",
        ),
        (
            ". until this permanent leaves the battlefield",
            " until this permanent leaves the battlefield",
        ),
        (
            ". until this creature leaves the battlefield",
            " until this creature leaves the battlefield",
        ),
    ] {
        normalized = normalized.replace(from, to);
    }
    if normalized.starts_with("Can't attack unless defending player controls ") {
        normalized = format!("This creature {normalized}");
    }
    if let Some((draw_part, lose_part)) = normalized.split_once(". target player loses ")
        && (draw_part.starts_with("Target player draws ")
            || draw_part.starts_with("target player draws "))
    {
        let draw_tail = draw_part
            .trim_start_matches("Target player draws ")
            .trim_start_matches("target player draws ")
            .trim();
        normalized = format!("Target player draws {draw_tail} and loses {}", lose_part.trim());
    }
    if let Some((left, right)) = normalized.split_once(". Deal ") {
        let right = right.trim().trim_end_matches('.').trim();
        if left.to_ascii_lowercase().contains(" deals ") && !right.is_empty() {
            normalized = format!("{} and {}", left.trim_end_matches('.'), right);
        }
    }
    normalized = normalized.replace(
        "that an opponent's land could produce",
        "that a land an opponent controls could produce",
    );
    normalized = normalized.replace(
        "that an opponent's lands could produce",
        "that lands an opponent controls could produce",
    );
    normalized = normalized
        .replace(
            "target opponent's artifact or enchantment",
            "target artifact or enchantment an opponent controls",
        )
        .replace(": It deals ", ": This creature deals ")
        .replace(": it deals ", ": this creature deals ")
        .replace(
            "have t add one mana of any color",
            "have {T}: add one mana of any color",
        )
        .replace(
            "target creature an opponent controls",
            "target creature you don't control",
        );
    if normalized.starts_with("Surveil ") || normalized.starts_with("surveil ") {
        normalized = normalized
            .replace(", then draw ", ". Draw ")
            .replace(", then you draw ", ". Draw ")
            .replace(", then you draw", ". Draw");
    }
    if normalized.starts_with("Draw ") || normalized.starts_with("draw ") {
        normalized = normalized
            .replace(" and create ", ". Create ")
            .replace(" and create", ". Create");
    }
    for (from, to) in [
        (
            "Search your library for up to one basic land you own, put it onto the battlefield tapped, then shuffle",
            "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle",
        ),
        (
            "Search your library for up to one basic land you own, put it onto the battlefield, then shuffle",
            "Search your library for a basic land card, put it onto the battlefield, then shuffle",
        ),
        (
            "Search your library for basic land you own, reveal it, then shuffle and put the card on top",
            "Search your library for a basic land card, reveal it, then shuffle and put that card on top",
        ),
    ] {
        normalized = normalized.replace(from, to);
    }
    if let Some((prefix, rest)) = normalized.split_once("Search your library for ")
        && let Some((tribe, tail)) = rest.split_once(" with mana value ")
        && !tribe.trim().is_empty()
        && !tribe.contains(' ')
    {
        for suffix in [
            " you own, put it onto the battlefield, then shuffle.",
            " you own, put it onto the battlefield, then shuffle",
        ] {
            if let Some(mv_clause) = tail.strip_suffix(suffix) {
                normalized = format!(
                    "{prefix}Search your library for a {tribe} permanent card with mana value {mv_clause}, put it onto the battlefield, then shuffle"
                );
                break;
            }
        }
    }
    if normalized.contains(", you draw ") && normalized.contains(" and lose ") {
        normalized = normalized.replace(" and lose ", " and you lose ");
    }
    if normalized.contains(", you draw ") && normalized.contains(" and gain ") {
        normalized = normalized.replace(" and gain ", " and you gain ");
    }
    let lower = normalized.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("for each player, you may that player ")
        && let Some((first, second)) = rest.split_once(". if you don't, that player ")
    {
        normalized = format!(
            "Each player may {}. Each player who doesn't {}",
            first.trim_end_matches('.'),
            second.trim_end_matches('.')
        );
    } else if let Some(rest) = lower.strip_prefix("for each opponent, you may that player ")
        && let Some((first, second)) = rest.split_once(". if you don't, that player ")
    {
        normalized = format!(
            "Each opponent may {}. Each opponent who doesn't {}",
            first.trim_end_matches('.'),
            second.trim_end_matches('.')
        );
    } else if let Some(amount) = lower
        .strip_prefix("for each opponent, deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that player"))
    {
        normalized = format!("This spell deals {amount} damage to each opponent");
    } else if let Some(rest) = lower.strip_prefix("for each opponent, that player ") {
        normalized = format!("Each opponent {rest}");
    } else if let Some(rest) = lower.strip_prefix("for each player, that player ") {
        normalized = format!("Each player {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Choose one — ") {
        normalized = format!("Choose one —. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Choose one or both — ") {
        normalized = format!("Choose one or both —. {rest}");
    }

    normalized
        .replace(" • ", ". ")
        .replace("• ", ". ")
        .replace(" and untap it", ". Untap it")
        .replace(" and untap that creature", ". Untap it")
        .replace(" and untap that permanent", ". Untap it")
        .replace(" and untap them", ". Untap them")
        .replace(" and investigate", ". Investigate")
        .replace(" and draw a card", ". Draw a card")
        .replace(" and discard a card", ". Discard a card")
        .replace(" and this creature deals ", ". Deal ")
        .replace(" and this permanent deals ", ". Deal ")
        .replace(" and this spell deals ", ". Deal ")
        .replace(" and it deals ", ". Deal ")
        .replace(" and you gain ", ". You gain ")
        .replace(" and you lose ", ". You lose ")
        .replace(" and you draw ", ". You draw ")
        .replace(" and you discard ", ". You discard ")
        .replace(" and each opponent loses ", ". Each opponent loses ")
        .replace(" and each opponent discards ", ". Each opponent discards ")
        .replace(" and each player loses ", ". Each player loses ")
        .replace(" and each player discards ", ". Each player discards ")
        .replace(" and target opponent loses ", ". Target opponent loses ")
        .replace(" and target opponent discards ", ". Target opponent discards ")
}

fn normalize_for_each_player_conditional_for_compare(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let player_prefix = "for each player, if that player ";
    if lower.starts_with(player_prefix)
        && let Some((condition, action)) = line[player_prefix.len()..].split_once(", that player ")
    {
        return format!("Each player who {} {}", condition.trim(), action.trim());
    }

    let opponent_prefix = "for each opponent, if that player ";
    if lower.starts_with(opponent_prefix)
        && let Some((condition, action)) = line[opponent_prefix.len()..].split_once(", that player ")
    {
        return format!("Each opponent who {} {}", condition.trim(), action.trim());
    }

    line.to_string()
}

fn normalize_explicit_damage_source_for_compare(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    for prefix in [
        "this creature deals ",
        "this permanent deals ",
        "this spell deals ",
        "this enchantment deals ",
        "this artifact deals ",
        "this land deals ",
        "this token deals ",
        "that creature deals ",
        "that permanent deals ",
        "it deals ",
    ] {
        if lower.starts_with(prefix) {
            let rest = line[prefix.len()..].trim_start();
            return format!("Deal {rest}");
        }
    }
    line.to_string()
}

fn semantic_clauses(text: &str) -> Vec<String> {
    let mut clauses = Vec::new();
    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let line = if trimmed.starts_with('(') && trimmed.ends_with(')') {
            // Oracle parentheticals are reminder/rules-help text for audit purposes.
            // Exclude them from semantic similarity scoring.
            continue;
        } else {
            let no_parenthetical = strip_parenthetical(raw_line);
            let no_inline_reminders = strip_inline_token_reminders(&no_parenthetical);
            strip_reminder_like_quotes(&no_inline_reminders)
        };
        let line = normalize_trigger_subject_for_compare(&line);
        let line = split_common_semantic_conjunctions(&line);
        let line = normalize_for_each_player_conditional_for_compare(&line);
        let line = normalize_explicit_damage_source_for_compare(&line);
        push_semantic_clauses(&line, &mut clauses);
    }
    clauses
}

fn reminder_clauses(text: &str) -> Vec<String> {
    let mut clauses = Vec::new();
    for segment in parenthetical_segments(text) {
        push_semantic_clauses(&segment, &mut clauses);
    }
    clauses
}

fn is_chapter_style_prefix(prefix: &str) -> bool {
    let lower = prefix.trim().to_ascii_lowercase();
    if lower.contains("chapter") || lower.contains("chapters") {
        return true;
    }
    let has_roman = lower.chars().any(|ch| matches!(ch, 'i' | 'v' | 'x'));
    has_roman
        && lower
            .chars()
            .all(|ch| matches!(ch, 'i' | 'v' | 'x' | ',' | ' '))
}

fn strip_ability_word_prefix(clause: &str) -> String {
    let trimmed = clause.trim();
    let Some((prefix, tail)) = trimmed.split_once('—') else {
        return trimmed.to_string();
    };
    if is_chapter_style_prefix(prefix) {
        return trimmed.to_string();
    }
    let tail = tail.trim();
    if tail.is_empty() {
        return trimmed.to_string();
    }
    let tail_lower = tail.to_ascii_lowercase();
    let semantic_tail = tail_lower.starts_with("when ")
        || tail_lower.starts_with("whenever ")
        || tail_lower.starts_with("at the beginning ")
        || tail_lower.starts_with("until ")
        || tail_lower.starts_with("each ")
        || tail_lower.starts_with("target ")
        || tail_lower.starts_with("draw ")
        || tail_lower.starts_with("destroy ")
        || tail_lower.starts_with("create ")
        || tail_lower.starts_with("put ")
        || tail_lower.starts_with("exile ")
        || tail_lower.starts_with("return ")
        || tail_lower.starts_with("you ")
        || tail_lower.starts_with("this ")
        || tail_lower.starts_with("counter ");
    if semantic_tail {
        tail.to_string()
    } else {
        trimmed.to_string()
    }
}

fn is_internal_compiled_scaffolding_clause(clause: &str) -> bool {
    let lower = clause.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }

    if lower.contains("tag the object") || lower.contains("tags it as '") {
        return true;
    }

    if lower.starts_with("you choose ")
        && (lower.contains(" in the battlefield")
            || lower.contains(" in your graveyard")
            || lower.contains(" in exile")
            || lower.contains(" and tag")
            || lower.contains(" and tags "))
    {
        return true;
    }
    if lower.starts_with("choose ")
        && lower.contains("target attacking creature")
        && !lower.contains(" and ")
    {
        return true;
    }

    false
}

fn tokenize_text(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_braces = false;

    for ch in lower.chars() {
        if in_braces {
            current.push(ch);
            if ch == '}' {
                tokens.push(current.clone());
                current.clear();
                in_braces = false;
            }
            continue;
        }

        if ch == '{' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            current.push(ch);
            in_braces = true;
            continue;
        }

        if ch.is_ascii_alphanumeric() || matches!(ch, '/' | '+' | '-' | '\'') {
            current.push(ch);
            continue;
        }

        if !current.is_empty() {
            tokens.push(current.clone());
            current.clear();
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn is_number_token(token: &str) -> bool {
    token == "x" || token.parse::<i64>().is_ok()
}

fn is_pt_component(value: &str) -> bool {
    let stripped = value.trim_matches(|c| matches!(c, '+' | '-'));
    stripped == "x" || stripped == "*" || stripped.parse::<i32>().is_ok()
}

fn is_pt_token(token: &str) -> bool {
    let Some((left, right)) = token.split_once('/') else {
        return false;
    };
    is_pt_component(left) && is_pt_component(right)
}

fn normalize_word(token: &str) -> Option<String> {
    if token.is_empty() {
        return None;
    }
    if matches!(
        token,
        "zero"
            | "one"
            | "two"
            | "three"
            | "four"
            | "five"
            | "six"
            | "seven"
            | "eight"
            | "nine"
            | "ten"
    ) {
        return Some("<num>".to_string());
    }
    if token == "plusoneplusone" || token == "minusoneminusone" {
        return Some("<pt>".to_string());
    }
    if token.starts_with('{') && token.ends_with('}') {
        return Some("<mana>".to_string());
    }
    if is_pt_token(token) {
        return Some("<pt>".to_string());
    }
    if is_number_token(token) {
        return Some("<num>".to_string());
    }

    let mut base = token.trim_matches('\'').to_string();
    if base == "can't" || base == "cannot" {
        base = "cant".to_string();
    }
    if base.ends_with("'s") {
        base.truncate(base.len().saturating_sub(2));
    }
    if base.ends_with("ies") && base.len() > 4 {
        base.truncate(base.len().saturating_sub(3));
        base.push('y');
    } else if base.ends_with("ing") && base.len() > 5 {
        base.truncate(base.len().saturating_sub(3));
    } else if base.ends_with("ed") && base.len() > 4 {
        base.truncate(base.len().saturating_sub(2));
    }
    if base.len() > 4 && base.ends_with('s') {
        base.pop();
    }
    base = match base.as_str() {
        "whenever" => "when".to_string(),
        "enters" | "entering" | "entered" => "enter".to_string(),
        "becomes" | "becoming" | "became" => "become".to_string(),
        "dies" | "died" | "dying" => "die".to_string(),
        "casts" | "casting" | "casted" => "cast".to_string(),
        "controls" | "controlled" | "controlling" => "control".to_string(),
        "sacrifices" | "sacrificed" | "sacrificing" => "sacrifice".to_string(),
        "draws" | "drawing" | "drew" => "draw".to_string(),
        "discards" | "discarded" | "discarding" => "discard".to_string(),
        "gains" | "gaining" | "gained" => "gain".to_string(),
        "loses" | "losing" | "lost" => "lose".to_string(),
        "deals" | "dealing" | "dealt" => "deal".to_string(),
        "matches" | "matched" | "matching" => "match".to_string(),
        _ => base,
    };
    if matches!(
        base.as_str(),
        "tag" | "tagged" | "object" | "attached" | "match" | "otherwise" | "appropriate"
    ) {
        return None;
    }
    if base.is_empty() { None } else { Some(base) }
}

fn is_stopword(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "the"
            | "this"
            | "that"
            | "those"
            | "these"
            | "it"
            | "its"
            | "them"
            | "their"
            | "they"
            | "you"
            | "your"
            | "to"
            | "of"
            | "and"
            | "or"
            | "for"
            | "from"
            | "in"
            | "on"
            | "at"
            | "with"
            | "into"
            | "onto"
            | "up"
            | "down"
            | "as"
            | "by"
            | "during"
            | "while"
            | "through"
            | "under"
            | "then"
            | "though"
            | "t"
    )
}

fn comparison_tokens(clause: &str) -> Vec<String> {
    tokenize_text(clause)
        .into_iter()
        .filter_map(|token| normalize_word(&token))
        .filter(|token| !is_stopword(token))
        .collect()
}

fn tokens_match_subsetish(tokens: &[String], reference: &[String]) -> bool {
    if tokens.is_empty() || reference.is_empty() {
        return false;
    }
    let reference_set: HashSet<&str> = reference.iter().map(String::as_str).collect();
    let overlap = tokens
        .iter()
        .filter(|token| reference_set.contains(token.as_str()))
        .count();
    (overlap as f32 / tokens.len() as f32) >= 0.80
}

fn is_cluster_keyword(token: &str) -> bool {
    matches!(
        token,
        "<mana>"
            | "<num>"
            | "<pt>"
            | "target"
            | "each"
            | "any"
            | "all"
            | "you"
            | "opponent"
            | "player"
            | "creature"
            | "artifact"
            | "enchantment"
            | "planeswalker"
            | "battle"
            | "permanent"
            | "spell"
            | "card"
            | "token"
            | "counter"
            | "draw"
            | "discard"
            | "gain"
            | "lose"
            | "destroy"
            | "exile"
            | "return"
            | "search"
            | "shuffle"
            | "reveal"
            | "mill"
            | "surveil"
            | "scry"
            | "look"
            | "choose"
            | "sacrifice"
            | "add"
            | "pay"
            | "cast"
            | "countered"
            | "put"
            | "remove"
            | "move"
            | "deal"
            | "damage"
            | "tap"
            | "untap"
            | "attack"
            | "block"
            | "regenerate"
            | "copy"
            | "transform"
            | "investigate"
            | "proliferate"
            | "vote"
            | "if"
            | "unless"
            | "when"
            | "whenever"
            | "at"
            | "beginning"
            | "end"
            | "step"
            | "turn"
            | "until"
            | "while"
            | "as"
            | "instead"
            | "where"
            | "for"
            | "this"
            | "that"
            | "it"
            | "those"
            | "their"
            | "your"
            | "can"
            | "cant"
            | "not"
            | "becomes"
            | "become"
            | "has"
            | "have"
            | "get"
            | "gets"
            | "enters"
            | "dies"
            | "graveyard"
            | "library"
            | "hand"
            | "battlefield"
            | "zone"
            | "owner"
            | "controller"
            | "life"
            | "power"
            | "toughness"
    )
}

fn clause_signature(clause: &str) -> String {
    let tokens = comparison_tokens(clause);
    if tokens.len() <= 3 {
        return if tokens.is_empty() {
            "<empty>".to_string()
        } else {
            tokens.join(" ")
        };
    }

    let mut out = Vec::new();
    for token in tokens {
        let mapped = if is_cluster_keyword(&token) {
            token
        } else if token.len() > 14 {
            "<arg>".to_string()
        } else {
            token
        };
        if out.last().is_some_and(|last| last == &mapped) {
            continue;
        }
        out.push(mapped);
    }
    if out.is_empty() {
        "<empty>".to_string()
    } else {
        out.join(" ")
    }
}

#[derive(Debug, Clone, Copy)]
struct EmbeddingConfig {
    dims: usize,
    mismatch_threshold: f32,
}

fn embedding_tokens(clause: &str) -> Vec<String> {
    tokenize_text(clause)
        .into_iter()
        .filter_map(|token| normalize_word(&token))
        .collect()
}

fn hash_index(feature: &str, dims: usize) -> usize {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    feature.hash(&mut hasher);
    (hasher.finish() as usize) % dims.max(1)
}

fn hash_sign(feature: &str) -> f32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    ("sign", feature).hash(&mut hasher);
    if hasher.finish() & 1 == 0 { 1.0 } else { -1.0 }
}

fn add_feature(vec: &mut [f32], feature: &str, weight: f32) {
    let idx = hash_index(feature, vec.len());
    vec[idx] += hash_sign(feature) * weight;
}

fn l2_normalize(vec: &mut [f32]) {
    let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in vec {
            *v /= norm;
        }
    }
}

fn embed_clause(clause: &str, dims: usize) -> Vec<f32> {
    let mut vec = vec![0.0f32; dims.max(1)];
    let tokens = embedding_tokens(clause);

    for token in &tokens {
        add_feature(&mut vec, &format!("u:{token}"), 1.0);
    }
    for window in tokens.windows(2) {
        add_feature(&mut vec, &format!("b:{}|{}", window[0], window[1]), 0.85);
    }
    for window in tokens.windows(3) {
        add_feature(
            &mut vec,
            &format!("t:{}|{}|{}", window[0], window[1], window[2]),
            1.0,
        );
    }

    // Structural anchors for common semantic clauses.
    let lower = clause.to_ascii_lowercase();
    for marker in ["where", "plus", "minus", "for each", "as long as", "unless"] {
        if lower.contains(marker) {
            add_feature(&mut vec, &format!("m:{marker}"), 1.8);
        }
    }

    // Lightweight character n-grams help when token sets are similar but syntax differs.
    let compact = lower
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == ' ')
        .collect::<String>();
    let chars: Vec<char> = compact.chars().collect();
    for ngram in chars.windows(4).take(200) {
        let key = ngram.iter().collect::<String>();
        add_feature(&mut vec, &format!("c:{key}"), 0.2);
    }

    l2_normalize(&mut vec);
    vec
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0f32;
    for i in 0..len {
        dot += a[i] * b[i];
    }
    dot.clamp(-1.0, 1.0)
}

fn directional_embedding_coverage(from: &[Vec<f32>], to: &[Vec<f32>]) -> f32 {
    if from.is_empty() {
        return if to.is_empty() { 1.0 } else { 0.0 };
    }

    let mut total = 0.0f32;
    for source in from {
        let mut best = -1.0f32;
        for target in to {
            let score = cosine_similarity(source, target);
            if score > best {
                best = score;
            }
        }
        total += best.max(0.0);
    }
    total / from.len() as f32
}

fn cluster_key(oracle_text: &str) -> String {
    let mut parts: Vec<String> = semantic_clauses(oracle_text)
        .into_iter()
        .map(|clause| clause_signature(&clause))
        .filter(|part| part != "<empty>")
        .collect();
    parts.sort();
    if parts.is_empty() {
        "0|<empty>".to_string()
    } else {
        format!("{}|{}", parts.len(), parts.join(" || "))
    }
}

fn jaccard_similarity(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_set: HashSet<&str> = a.iter().map(String::as_str).collect();
    let b_set: HashSet<&str> = b.iter().map(String::as_str).collect();
    let inter = a_set.intersection(&b_set).count() as f32;
    let union = a_set.union(&b_set).count() as f32;
    if union == 0.0 { 0.0 } else { inter / union }
}

fn directional_coverage(from: &[Vec<String>], to: &[Vec<String>]) -> f32 {
    if from.is_empty() {
        return if to.is_empty() { 1.0 } else { 0.0 };
    }

    let mut total = 0.0f32;
    for source in from {
        let mut best = 0.0f32;
        for target in to {
            let score = jaccard_similarity(source, target);
            if score > best {
                best = score;
            }
        }
        total += best;
    }
    total / from.len() as f32
}

fn is_compiled_heading_prefix(prefix: &str) -> bool {
    let prefix = prefix.trim().to_ascii_lowercase();
    prefix == "spell effects"
        || prefix.starts_with("activated ability ")
        || prefix.starts_with("triggered ability ")
        || prefix.starts_with("static ability ")
        || prefix.starts_with("keyword ability ")
        || prefix.starts_with("mana ability ")
        || prefix.starts_with("ability ")
        || prefix.starts_with("alternative cast ")
}

fn strip_compiled_prefix(line: &str) -> &str {
    let Some((prefix, rest)) = line.split_once(':') else {
        return line;
    };
    if is_compiled_heading_prefix(prefix) {
        rest.trim()
    } else {
        line
    }
}

fn normalize_card_self_references(text: &str, card_name: &str) -> String {
    let full_name = card_name.trim();
    if full_name.is_empty() {
        return text.to_string();
    }

    let left_half = full_name
        .split("//")
        .next()
        .map(str::trim)
        .unwrap_or(full_name);
    let short_name = left_half
        .split(',')
        .next()
        .map(str::trim)
        .unwrap_or(left_half);

    let mut names = vec![full_name, left_half, short_name];
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    names.dedup();

    let mut normalized = text.to_string();
    for name in names {
        if name.len() < 3 {
            continue;
        }
        normalized = normalized.replace(name, "this");
    }
    normalized
}

fn compare_semantics(
    card_name: &str,
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, f32, isize, bool) {
    let normalized_oracle = normalize_card_self_references(oracle_text, card_name);
    let normalized_compiled_lines = compiled_lines
        .iter()
        .map(|line| normalize_card_self_references(line, card_name))
        .collect::<Vec<_>>();

    let oracle_clauses = semantic_clauses(&normalized_oracle);
    let reminder_clauses = reminder_clauses(&normalized_oracle);
    let raw_compiled_clauses = normalized_compiled_lines
        .iter()
        .flat_map(|line| semantic_clauses(strip_compiled_prefix(line)))
        .collect::<Vec<_>>();

    let oracle_tokens: Vec<Vec<String>> = oracle_clauses
        .iter()
        .map(|clause| comparison_tokens(clause))
        .filter(|tokens| !tokens.is_empty())
        .collect();
    let reminder_tokens: Vec<Vec<String>> = reminder_clauses
        .iter()
        .map(|clause| comparison_tokens(clause))
        .filter(|tokens| !tokens.is_empty())
        .collect();

    let mut compiled_pairs = raw_compiled_clauses
        .iter()
        .map(|clause| (clause.clone(), comparison_tokens(clause)))
        .filter(|(_, tokens)| !tokens.is_empty())
        .collect::<Vec<_>>();

    compiled_pairs.retain(|(clause, _)| !is_internal_compiled_scaffolding_clause(clause));

    // If oracle reminder text is excluded, also exclude compiled-only clauses
    // that are clearly just reminder-surface equivalents.
    compiled_pairs.retain(|(_, tokens)| {
        let matches_oracle = oracle_tokens
            .iter()
            .any(|oracle| tokens_match_subsetish(tokens, oracle));
        let matches_reminder = reminder_tokens
            .iter()
            .any(|reminder| tokens_match_subsetish(tokens, reminder));
        !(matches_reminder && !matches_oracle)
    });

    let compiled_clauses = compiled_pairs
        .iter()
        .map(|(clause, _)| clause.clone())
        .collect::<Vec<_>>();
    let compiled_tokens: Vec<Vec<String>> = compiled_pairs
        .into_iter()
        .map(|(_, tokens)| tokens)
        .collect();

    // Parenthetical-only oracle text (typically reminder text) carries no
    // semantic clauses after normalization, so don't flag as mismatch.
    if oracle_tokens.is_empty() {
        return (1.0, 1.0, 1.0, 0, false);
    }

    let oracle_coverage = directional_coverage(&oracle_tokens, &compiled_tokens);
    let compiled_coverage = directional_coverage(&compiled_tokens, &oracle_tokens);
    let line_delta = compiled_clauses.len() as isize - oracle_clauses.len() as isize;

    let min_coverage = oracle_coverage.min(compiled_coverage);
    let semantic_gap = min_coverage < 0.25;
    let line_gap = line_delta.abs() >= 3 && min_coverage < 0.50;
    let empty_gap = !oracle_tokens.is_empty() && compiled_tokens.is_empty();

    let mut similarity_score = min_coverage;
    let mut mismatch = semantic_gap || line_gap || empty_gap;

    if let Some(cfg) = embedding {
        let oracle_emb = oracle_clauses
            .iter()
            .map(|clause| embed_clause(clause, cfg.dims))
            .collect::<Vec<_>>();
        let compiled_emb = compiled_clauses
            .iter()
            .map(|clause| embed_clause(clause, cfg.dims))
            .collect::<Vec<_>>();
        let emb_oracle = directional_embedding_coverage(&oracle_emb, &compiled_emb);
        let emb_compiled = directional_embedding_coverage(&compiled_emb, &oracle_emb);
        let emb_min = emb_oracle.min(emb_compiled);
        similarity_score = emb_min;
        if emb_min < cfg.mismatch_threshold {
            mismatch = true;
        }
    }

    (
        oracle_coverage,
        compiled_coverage,
        similarity_score,
        line_delta,
        mismatch,
    )
}

fn normalize_parse_error(error: &str) -> String {
    let mut normalized = error
        .trim()
        .trim_start_matches("ParseError(\"")
        .trim_start_matches("UnsupportedLine(\"")
        .trim_end_matches("\")")
        .to_string();
    for marker in [" (clause:", " (line:", " (source:"] {
        if let Some(idx) = normalized.find(marker) {
            normalized = normalized[..idx].trim().to_string();
        }
    }
    normalized
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let mut out = String::new();
    for ch in text.chars().take(keep) {
        out.push(ch);
    }
    out.push_str("...");
    out
}

fn first_oracle_excerpt(text: &str) -> String {
    semantic_clauses(text)
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn first_compiled_excerpt(lines: &[String]) -> String {
    lines
        .first()
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| "<none>".to_string())
}

fn read_name_set(path: &str) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    let raw = fs::read_to_string(path)?;
    Ok(raw
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            if let Some(rest) = line.strip_prefix("Name:") {
                let name = rest.trim();
                if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                }
            } else if line.contains(':') {
                None
            } else {
                Some(line.to_string())
            }
        })
        .collect())
}

fn pick_field<'a>(card: &'a Value, face: Option<&'a Value>, key: &str) -> Option<&'a str> {
    card.get(key)
        .and_then(Value::as_str)
        .or_else(|| face.and_then(|f| f.get(key)).and_then(Value::as_str))
}

fn has_acorn(card: &Value) -> bool {
    card.get("has_acorn")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn legalities_all_not_legal(card: &Value) -> bool {
    let Some(legalities) = card.get("legalities").and_then(Value::as_object) else {
        return false;
    };
    !legalities.is_empty()
        && legalities
            .values()
            .all(|value| value.as_str().is_some_and(|item| item == "not_legal"))
}

fn is_non_playable(type_line: Option<&str>, oracle_text: Option<&str>, card: &Value) -> bool {
    if card
        .get("border_color")
        .and_then(Value::as_str)
        .is_some_and(|value| value.eq_ignore_ascii_case("silver"))
    {
        return true;
    }
    if has_acorn(card) {
        return true;
    }
    if legalities_all_not_legal(card) {
        return true;
    }
    if card
        .get("layout")
        .and_then(Value::as_str)
        .is_some_and(|layout| {
            matches!(
                layout.to_ascii_lowercase().as_str(),
                "token"
                    | "double_faced_token"
                    | "emblem"
                    | "planar"
                    | "scheme"
                    | "vanguard"
                    | "art_series"
                    | "reversible_card"
            )
        })
    {
        return true;
    }
    if let Some(type_line) = type_line {
        let disallowed = [
            "Token",
            "Emblem",
            "Plane",
            "Scheme",
            "Vanguard",
            "Phenomenon",
            "Conspiracy",
            "Dungeon",
            "Attraction",
            "Contraption",
        ];
        if disallowed.iter().any(|needle| type_line.contains(needle)) {
            return true;
        }
        if type_line.trim() == "Card" {
            return true;
        }
    }
    if oracle_text.is_some_and(|oracle| oracle.contains("Theme color")) {
        return true;
    }
    false
}

fn build_parse_input(card: &Value) -> Option<CardInput> {
    let face = card
        .get("card_faces")
        .and_then(Value::as_array)
        .and_then(|faces| faces.first());

    let name = pick_field(card, face, "name")?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let mana_cost = pick_field(card, face, "mana_cost");
    let type_line = pick_field(card, face, "type_line");
    let oracle_text = pick_field(card, face, "oracle_text");
    if is_non_playable(type_line, oracle_text, card) {
        return None;
    }
    let oracle_text = oracle_text?.trim().to_string();
    if oracle_text.is_empty() {
        return None;
    }

    let power = pick_field(card, face, "power");
    let toughness = pick_field(card, face, "toughness");
    let loyalty = pick_field(card, face, "loyalty");
    let defense = pick_field(card, face, "defense");

    let mut lines = Vec::new();
    if let Some(mana_cost) = mana_cost.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("Mana cost: {}", mana_cost.trim()));
    }
    if let Some(type_line) = type_line.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("Type: {}", type_line.trim()));
    }
    if let (Some(power), Some(toughness)) = (power, toughness) {
        if !power.trim().is_empty() && !toughness.trim().is_empty() {
            lines.push(format!(
                "Power/Toughness: {}/{}",
                power.trim(),
                toughness.trim()
            ));
        }
    }
    if let Some(loyalty) = loyalty.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("Loyalty: {}", loyalty.trim()));
    }
    if let Some(defense) = defense.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("Defense: {}", defense.trim()));
    }
    lines.push(oracle_text.clone());

    Some(CardInput {
        name,
        oracle_text,
        parse_input: lines.join("\n"),
    })
}

fn set_parser_trace(enabled: bool) {
    unsafe {
        if enabled {
            env::set_var("IRONSMITH_PARSER_TRACE", "1");
        } else {
            env::remove_var("IRONSMITH_PARSER_TRACE");
        }
    }
}

fn set_allow_unsupported(enabled: bool) {
    unsafe {
        if enabled {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", "1");
        } else {
            env::remove_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED");
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let file_contents = fs::read_to_string(&args.cards_path)?;
    let cards_json: Value = serde_json::from_str(&file_contents)?;
    let cards = cards_json
        .as_array()
        .ok_or_else(|| std::io::Error::other("cards json must be an array"))?;

    let original_trace = env::var("IRONSMITH_PARSER_TRACE").ok();
    let original_allow_unsupported = env::var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED").ok();
    let original_semantic_guard = env::var("IRONSMITH_PARSER_SEMANTIC_GUARD").ok();
    let original_semantic_dims = env::var("IRONSMITH_PARSER_SEMANTIC_DIMS").ok();
    let original_semantic_threshold = env::var("IRONSMITH_PARSER_SEMANTIC_THRESHOLD").ok();
    if args.parser_trace {
        set_parser_trace(true);
    }
    if args.allow_unsupported {
        set_allow_unsupported(true);
    } else {
        set_allow_unsupported(false);
    }

    let embedding_cfg = if args.use_embeddings {
        Some(EmbeddingConfig {
            dims: args.embedding_dims,
            mismatch_threshold: args.embedding_threshold,
        })
    } else {
        None
    };
    let false_positive_names = match args.false_positive_names.as_deref() {
        Some(path) => read_name_set(path)?,
        None => HashSet::new(),
    };

    let mut audits = Vec::new();
    for card in cards {
        if let Some(limit) = args.limit
            && audits.len() >= limit
        {
            break;
        }
        let Some(card_input) = build_parse_input(card) else {
            continue;
        };

        let trace_for_card = if args.parser_trace {
            true
        } else if let Some(filter) = args.trace_name.as_ref() {
            card_input.name.to_ascii_lowercase().contains(filter)
        } else {
            false
        };
        if !args.parser_trace {
            set_parser_trace(trace_for_card);
        }

        let cluster_key = cluster_key(&card_input.oracle_text);
        let parse_result = CardDefinitionBuilder::new(CardId::new(), &card_input.name)
            .parse_text(card_input.parse_input.clone());

        let audit = match parse_result {
            Ok(definition) => {
                let compiled = compiled_lines(&definition);
                let (
                    oracle_coverage,
                    compiled_coverage,
                    similarity_score,
                    line_delta,
                    semantic_mismatch,
                ) = compare_semantics(
                    &card_input.name,
                    &card_input.oracle_text,
                    &compiled,
                    embedding_cfg,
                );
                CardAudit {
                    name: card_input.name,
                    oracle_text: card_input.oracle_text,
                    cluster_key,
                    parse_error: None,
                    compiled_lines: compiled,
                    oracle_coverage,
                    compiled_coverage,
                    similarity_score,
                    line_delta,
                    semantic_mismatch,
                    semantic_false_positive: false,
                }
            }
            Err(err) => CardAudit {
                name: card_input.name,
                oracle_text: card_input.oracle_text,
                cluster_key,
                parse_error: Some(format!("{err:?}")),
                compiled_lines: Vec::new(),
                oracle_coverage: 0.0,
                compiled_coverage: 0.0,
                similarity_score: 0.0,
                line_delta: 0,
                semantic_mismatch: false,
                semantic_false_positive: false,
            },
        };
        audits.push(audit);
    }

    for audit in &mut audits {
        if audit.parse_error.is_none()
            && audit.semantic_mismatch
            && false_positive_names.contains(&audit.name)
        {
            audit.semantic_mismatch = false;
            audit.semantic_false_positive = true;
        }
    }

    match original_trace {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_TRACE", value);
        },
        None => set_parser_trace(false),
    }
    match original_allow_unsupported {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", value);
        },
        None => set_allow_unsupported(false),
    }
    match original_semantic_guard {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_SEMANTIC_GUARD", value);
        },
        None => unsafe {
            env::remove_var("IRONSMITH_PARSER_SEMANTIC_GUARD");
        },
    }
    match original_semantic_dims {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_SEMANTIC_DIMS", value);
        },
        None => unsafe {
            env::remove_var("IRONSMITH_PARSER_SEMANTIC_DIMS");
        },
    }
    match original_semantic_threshold {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_SEMANTIC_THRESHOLD", value);
        },
        None => unsafe {
            env::remove_var("IRONSMITH_PARSER_SEMANTIC_THRESHOLD");
        },
    }

    let mut clusters: HashMap<String, Vec<CardAudit>> = HashMap::new();
    for audit in audits {
        clusters
            .entry(audit.cluster_key.clone())
            .or_default()
            .push(audit);
    }

    let cards_processed = clusters.values().map(Vec::len).sum::<usize>();
    let parse_failures = clusters
        .values()
        .flatten()
        .filter(|audit| audit.parse_error.is_some())
        .count();
    let semantic_mismatches = clusters
        .values()
        .flatten()
        .filter(|audit| audit.parse_error.is_none() && audit.semantic_mismatch)
        .count();
    let semantic_false_positives = clusters
        .values()
        .flatten()
        .filter(|audit| audit.parse_error.is_none() && audit.semantic_false_positive)
        .count();

    if let Some(path) = args.mismatch_names_out.as_ref() {
        let mut names = clusters
            .values()
            .flatten()
            .filter(|audit| audit.parse_error.is_none() && audit.semantic_mismatch)
            .map(|audit| audit.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        fs::write(path, names.join("\n"))?;
        println!("Wrote mismatch names to {path} ({} names)", names.len());
    }

    if let Some(path) = args.failures_out.as_ref() {
        let mut entries = clusters
            .values()
            .flatten()
            .filter(|audit| audit.parse_error.is_none() && audit.semantic_mismatch)
            .map(|audit| JsonFailureEntry {
                name: audit.name.clone(),
                parse_error: audit.parse_error.clone(),
                oracle_coverage: audit.oracle_coverage,
                compiled_coverage: audit.compiled_coverage,
                similarity_score: audit.similarity_score,
                line_delta: audit.line_delta,
                oracle_text: audit.oracle_text.clone(),
                compiled_text: audit.compiled_lines.join("\n"),
                compiled_lines: audit.compiled_lines.clone(),
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| {
            a.similarity_score
                .partial_cmp(&b.similarity_score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.name.cmp(&b.name))
        });
        let report = JsonFailureReport {
            threshold: args.embedding_threshold,
            cards_processed,
            failures: entries.len(),
            entries,
        };
        let payload = serde_json::to_string_pretty(&report)?;
        fs::write(path, payload)?;
        println!(
            "Wrote threshold failure report to {path} ({} cards)",
            report.failures
        );
    }

    let mut ranked: Vec<(String, Vec<CardAudit>)> = clusters
        .into_iter()
        .filter(|(_, entries)| entries.len() >= args.min_cluster_size)
        .collect();

    ranked.sort_by(|(a_key, a_entries), (b_key, b_entries)| {
        let a_fail = a_entries
            .iter()
            .filter(|entry| entry.parse_error.is_some())
            .count();
        let b_fail = b_entries
            .iter()
            .filter(|entry| entry.parse_error.is_some())
            .count();
        let a_mismatch = a_entries
            .iter()
            .filter(|entry| entry.parse_error.is_none() && entry.semantic_mismatch)
            .count();
        let b_mismatch = b_entries
            .iter()
            .filter(|entry| entry.parse_error.is_none() && entry.semantic_mismatch)
            .count();
        let a_total = a_entries.len().max(1) as f32;
        let b_total = b_entries.len().max(1) as f32;
        let a_problem = (a_fail + a_mismatch) as f32 / a_total;
        let b_problem = (b_fail + b_mismatch) as f32 / b_total;
        b_problem
            .partial_cmp(&a_problem)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b_entries.len().cmp(&a_entries.len()))
            .then_with(|| a_key.cmp(b_key))
    });

    println!("Oracle cluster audit complete");
    println!("- Cards processed: {cards_processed}");
    println!("- Parse failures: {parse_failures}");
    println!("- Semantic mismatches: {semantic_mismatches}");
    if !false_positive_names.is_empty() {
        println!("- Marked semantic false positives: {semantic_false_positives}");
    }
    if let Some(cfg) = embedding_cfg {
        println!(
            "- Embedding audit: enabled (dims={}, threshold={:.2})",
            cfg.dims, cfg.mismatch_threshold
        );
    } else {
        println!("- Embedding audit: disabled");
    }
    println!("- Total clusters: {}", ranked.len());
    println!("- Reporting up to {} clusters", args.top_clusters);
    println!();

    let mut parse_failure_hotspots = ranked
        .iter()
        .map(|(signature, entries)| {
            let parse_failures_count = entries
                .iter()
                .filter(|entry| entry.parse_error.is_some())
                .count();
            (signature, entries.len(), parse_failures_count)
        })
        .filter(|(_, _, parse_failures_count)| *parse_failures_count > 0)
        .collect::<Vec<_>>();
    parse_failure_hotspots.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.1.cmp(&a.1)));
    if !parse_failure_hotspots.is_empty() {
        println!("Top parse-failure clusters (absolute count):");
        for (signature, size, parse_failures_count) in parse_failure_hotspots.iter().take(8) {
            println!(
                "  - {} failures in cluster size {} :: {}",
                parse_failures_count,
                size,
                truncate_text(signature, 110)
            );
        }
        println!();
    }

    let clusters_total = ranked.len();
    let mut json_clusters = Vec::new();

    for (index, (signature, mut entries)) in ranked.into_iter().take(args.top_clusters).enumerate()
    {
        entries.sort_by(|a, b| {
            let a_key = (
                a.parse_error.is_none(),
                a.oracle_coverage.min(a.compiled_coverage),
                -(a.line_delta.abs()),
            );
            let b_key = (
                b.parse_error.is_none(),
                b.oracle_coverage.min(b.compiled_coverage),
                -(b.line_delta.abs()),
            );
            a_key
                .0
                .cmp(&b_key.0)
                .then_with(|| a_key.1.partial_cmp(&b_key.1).unwrap_or(Ordering::Equal))
                .then_with(|| a_key.2.cmp(&b_key.2))
                .then_with(|| a.name.cmp(&b.name))
        });

        let size = entries.len();
        let parse_failures_count = entries
            .iter()
            .filter(|entry| entry.parse_error.is_some())
            .count();
        let semantic_mismatch_count = entries
            .iter()
            .filter(|entry| entry.parse_error.is_none() && entry.semantic_mismatch)
            .count();
        let semantic_false_positive_count = entries
            .iter()
            .filter(|entry| entry.parse_error.is_none() && entry.semantic_false_positive)
            .count();
        let parse_failure_rate = parse_failures_count as f32 / size.max(1) as f32;
        let semantic_mismatch_rate = semantic_mismatch_count as f32 / size.max(1) as f32;

        let mut error_counts: HashMap<String, usize> = HashMap::new();
        for entry in &entries {
            if let Some(error) = entry.parse_error.as_ref() {
                *error_counts
                    .entry(normalize_parse_error(error))
                    .or_insert(0usize) += 1;
            }
        }
        let mut error_counts_vec: Vec<(String, usize)> = error_counts.into_iter().collect();
        error_counts_vec.sort_by(|(a_error, a_count), (b_error, b_count)| {
            b_count.cmp(a_count).then_with(|| a_error.cmp(b_error))
        });

        println!(
            "[{}] size={} parse_failures={} ({:.1}%) semantic_mismatches={} ({:.1}%)",
            index + 1,
            size,
            parse_failures_count,
            parse_failure_rate * 100.0,
            semantic_mismatch_count,
            semantic_mismatch_rate * 100.0
        );
        if semantic_false_positive_count > 0 {
            println!("marked false positives: {semantic_false_positive_count}");
        }
        println!("signature: {signature}");

        if !error_counts_vec.is_empty() {
            println!("top parse errors:");
            for (error, count) in error_counts_vec.iter().take(3) {
                println!("  - {count}x {error}");
            }
        }

        println!("examples:");
        for entry in entries.iter().take(args.examples_per_cluster) {
            if let Some(error) = &entry.parse_error {
                println!(
                    "  - {} [parse-failed] {}",
                    entry.name,
                    normalize_parse_error(error)
                );
            } else {
                println!(
                    "  - {} [score={:.2}, coverage o->{:.2}, c->{:.2}, delta={}]",
                    entry.name,
                    entry.similarity_score,
                    entry.oracle_coverage,
                    entry.compiled_coverage,
                    entry.line_delta
                );
            }
            println!("    oracle: {}", first_oracle_excerpt(&entry.oracle_text));
            println!(
                "    compiled: {}",
                first_compiled_excerpt(&entry.compiled_lines)
            );
        }
        println!();

        let top_errors = error_counts_vec
            .into_iter()
            .take(5)
            .map(|(error, count)| JsonErrorCount { error, count })
            .collect::<Vec<_>>();
        let examples = entries
            .iter()
            .take(args.examples_per_cluster)
            .map(|entry| JsonExample {
                name: entry.name.clone(),
                parse_error: entry.parse_error.clone(),
                oracle_coverage: entry.oracle_coverage,
                compiled_coverage: entry.compiled_coverage,
                similarity_score: entry.similarity_score,
                line_delta: entry.line_delta,
                oracle_excerpt: first_oracle_excerpt(&entry.oracle_text),
                compiled_excerpt: first_compiled_excerpt(&entry.compiled_lines),
                oracle_text: entry.oracle_text.clone(),
                compiled_lines: entry.compiled_lines.clone(),
            })
            .collect::<Vec<_>>();

        json_clusters.push(JsonCluster {
            signature,
            size,
            parse_failures: parse_failures_count,
            semantic_mismatches: semantic_mismatch_count,
            semantic_false_positives: semantic_false_positive_count,
            parse_failure_rate,
            semantic_mismatch_rate,
            top_errors,
            examples,
        });
    }

    if let Some(path) = args.json_out {
        let report = JsonReport {
            cards_processed,
            parse_failures,
            semantic_mismatches,
            semantic_false_positives,
            clusters_total,
            clusters_reported: json_clusters.len(),
            clusters: json_clusters,
        };
        let payload = serde_json::to_string_pretty(&report)?;
        fs::write(&path, payload)?;
        println!("Wrote JSON report to {path}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_signature_generalizes_numbers_and_mana() {
        let first = "Target creature gets +3/+3 until end of turn.";
        let second = "Target creature gets +5/+5 until end of turn.";
        assert_eq!(clause_signature(first), clause_signature(second));
    }

    #[test]
    fn test_compare_semantics_detects_drop() {
        let oracle = "Draw a card. Target player discards a card.";
        let compiled = vec!["you gain 2 life".to_string()];
        let (oracle_coverage, compiled_coverage, _similarity_score, line_delta, mismatch) =
            compare_semantics("", oracle, &compiled, None);
        assert!(mismatch);
        assert!(oracle_coverage < 0.75);
        assert!(compiled_coverage <= 0.25);
        assert!(line_delta < 0);
    }

    #[test]
    fn test_compare_semantics_ignores_parenthetical_only_oracle() {
        let oracle = "({T}: Add {B} or {R}.)";
        let compiled = vec!["Mana ability 1: Tap this source, Add {B} or {R}".to_string()];
        let (_oracle_coverage, _compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics("", oracle, &compiled, None);
        assert!(
            !mismatch,
            "parenthetical-only oracle reminder text should not count as semantic mismatch"
        );
    }

    #[test]
    fn test_semantic_clauses_ignore_parenthetical_lines_with_colon() {
        let oracle = "Create a Treasure token.\n(It's an artifact with \"{T}, Sacrifice this token: Add one mana of any color.\")";
        let clauses = semantic_clauses(oracle);
        assert_eq!(clauses, vec!["Create a Treasure token".to_string()]);
    }

    #[test]
    fn test_compare_semantics_excludes_compiled_clause_matched_only_by_reminder() {
        let oracle = "({T}: Add {R} or {W}.) This land enters tapped.";
        let compiled = vec![
            "Mana ability 1: {T}: Add {R}.".to_string(),
            "Mana ability 2: {T}: Add {W}.".to_string(),
            "Static ability 3: This land enters tapped.".to_string(),
        ];
        let (oracle_coverage, compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics("", oracle, &compiled, None);
        assert!(!mismatch);
        assert_eq!(oracle_coverage, 1.0);
        assert_eq!(compiled_coverage, 1.0);
    }

    #[test]
    fn test_embedding_mode_catches_dropped_where_plus_semantics() {
        let oracle = "Hobbit's Sting deals X damage to target creature, where X is the number of creatures you control plus the number of Foods you control.";
        let compiled = vec!["Deal X damage to target creature Food".to_string()];
        let (_oracle_coverage, _compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.55,
                }),
            );
        assert!(
            mismatch,
            "embedding mode should flag lost where/plus value semantics"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_named_trigger_subject() {
        let clauses = semantic_clauses("Whenever Tui and La become tapped, draw a card.");
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("whenever this creature"));
        assert!(clause.contains("tapp"));
        assert!(clause.contains("draw a card"));
    }

    #[test]
    fn test_semantic_clauses_split_and_you_gain_chain() {
        let clauses = semantic_clauses("Destroy target creature and you lose 2 life.");
        assert_eq!(
            clauses,
            vec![
                "Destroy target creature".to_string(),
                "You lose 2 life".to_string()
            ]
        );
    }

    #[test]
    fn test_semantic_clauses_split_you_draw_and_lose_chain() {
        let clauses =
            semantic_clauses("Whenever this creature attacks, you draw a card and lose 1 life.");
        assert_eq!(
            clauses,
            vec![
                "Whenever this creature attacks, you draw a card".to_string(),
                "You lose 1 life".to_string()
            ]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_explicit_damage_source_prefix() {
        let clauses = semantic_clauses(
            "When this creature leaves the battlefield, this creature deals 1 damage to you.",
        );
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("when this creature leaves the battlefield"));
        assert!(clause.contains("deal 1 damage to you") || clause.contains("deals 1 damage to you"));
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_player_conditional_phrase() {
        let clauses = semantic_clauses(
            "For each player, if that player controls a multicolored creature, that player draws a card.",
        );
        assert_eq!(
            clauses,
            vec!["Each player who controls a multicolored creature draws a card".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_repair_until_tail_split() {
        let clauses = semantic_clauses(
            "When this enchantment enters, exile target nonland permanent an opponent controls. until this enchantment leaves the battlefield.",
        );
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("exile target nonland permanent"));
        assert!(clause.contains("until this enchantment leaves the battlefield"));
    }

    #[test]
    fn test_semantic_clauses_add_missing_attack_subject() {
        let clauses = semantic_clauses("Can't attack unless defending player controls island.");
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.starts_with("this creature can't attack unless"));
    }

    #[test]
    fn test_semantic_clauses_merge_split_damage_followup() {
        let clauses = semantic_clauses("This creature deals 1 damage to any target. Deal 1 damage to you.");
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("deal 1 damage to any target"));
        assert!(clause.contains("and 1 damage to you"));
    }

    #[test]
    fn test_comparison_tokens_drop_internal_tag_scaffolding() {
        let tokens = comparison_tokens("Tag the object attached to this Aura as 'enchanted'.");
        assert!(!tokens.iter().any(|token| token == "tag"));
        assert!(!tokens.iter().any(|token| token == "object"));
        assert!(!tokens.iter().any(|token| token == "attached"));
    }

    #[test]
    fn test_compare_semantics_normalizes_card_name_self_reference() {
        let oracle = "{2}, Sacrifice Five Hundred Year Diary: Draw a card.";
        let compiled = vec!["Activated ability 1: {2}, Sacrifice this artifact: Draw a card.".to_string()];
        let (oracle_coverage, compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics("Five Hundred Year Diary", oracle, &compiled, None);
        assert!(!mismatch);
        assert!(oracle_coverage >= 0.5);
        assert!(compiled_coverage >= 0.5);
    }
}
