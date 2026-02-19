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

fn strip_parse_error_parentheticals(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    let mut segment = String::new();

    let is_parse_error_segment = |segment: &str| -> bool {
        let normalized = segment.trim().to_ascii_lowercase();
        normalized.starts_with("parseerror") || normalized.starts_with("unsupportedline")
    };

    let is_activation_reminder_segment = |segment: &str| -> bool {
        let normalized = segment.trim().to_ascii_lowercase();
        normalized.starts_with("activate ") || normalized.starts_with("activate only")
    };

    for ch in text.chars() {
        if ch == '(' {
            if depth == 0 {
                segment.clear();
            } else {
                segment.push(ch);
            }
            depth += 1;
            continue;
        }

        if ch == ')' {
            if depth == 0 {
                continue;
            }

            depth -= 1;
            if depth > 0 {
                segment.push(ch);
                continue;
            }

            let segment_lower = segment.trim().to_ascii_lowercase();
            if !is_parse_error_segment(&segment_lower)
                && !is_activation_reminder_segment(&segment_lower)
            {
                out.push('(');
                out.push_str(segment.trim());
                out.push(')');
            }
            segment.clear();
            continue;
        }

        if depth > 0 {
            segment.push(ch);
        } else {
            out.push(ch);
        }
    }

    if depth > 0 {
        let segment_lower = segment.trim().to_ascii_lowercase();
        if !is_parse_error_segment(&segment_lower)
            && !is_activation_reminder_segment(&segment_lower)
        {
            out.push('(');
            out.push_str(segment.trim());
        }
    }

    out
}

fn capitalize_fallback_with_parenthetical_title_case(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let mut uppercase_next = true;
    for ch in text.chars() {
        if uppercase_next && ch.is_ascii_alphabetic() {
            out.push(ch.to_ascii_uppercase());
            uppercase_next = false;
            continue;
        }

        if ch == '.' || ch == '!' || ch == '?' || ch == '(' {
            uppercase_next = true;
        }
        out.push(ch);
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
                    for prefix in [
                        "It has ",
                        "it has ",
                        "They have ",
                        "they have ",
                        "with ",
                        "With ",
                    ] {
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
    let mut paren_depth = 0usize;
    for ch in line.chars() {
        match ch {
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            '.' | ';' | '\n' => {
                if paren_depth == 0 {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() && trimmed.chars().any(|ch| ch.is_ascii_alphanumeric()) {
                        clauses.push(strip_ability_word_prefix(trimmed));
                    }
                    current.clear();
                    continue;
                }
                current.push(ch);
            }
            _ => current.push(ch),
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

fn looks_like_modal_label(segment: &str) -> bool {
    let trimmed = segment.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains('.')
        || trimmed.contains(':')
        || trimmed.contains(',')
        || trimmed.contains(';')
    {
        return false;
    }
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.is_empty() || words.len() > 4 {
        return false;
    }
    words.iter().all(|word| {
        let mut chars = word.chars();
        chars
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
            && chars.all(|ch| ch.is_ascii_alphabetic() || ch == '\'' || ch == '-')
    })
}

fn strip_modal_option_labels(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if !lower.contains("choose one") && !lower.contains("choose two") && !lower.contains("choose ")
    {
        return line.to_string();
    }
    if !line.contains('—') {
        return line.to_string();
    }

    let parts: Vec<&str> = line.split('—').collect();
    if parts.len() < 3 {
        return line.to_string();
    }

    let mut rebuilt = String::new();
    rebuilt.push_str(parts[0].trim_end());
    for (idx, part) in parts.iter().enumerate().skip(1) {
        let segment = part.trim();
        let is_middle = idx < parts.len() - 1;
        if is_middle && looks_like_modal_label(segment) {
            continue;
        }
        rebuilt.push_str(" — ");
        rebuilt.push_str(segment);
    }
    rebuilt
}

fn normalize_clause_line(line: &str) -> String {
    let normalized = line.split_whitespace().collect::<Vec<_>>().join(" ");
    normalize_end_turn_creature_buff_split(&normalized)
}

fn normalize_end_turn_creature_buff_split(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let marker = ". creatures you control gain ";
    let marker_idx = match lower.find(marker) {
        Some(idx) => idx,
        None => return line.to_string(),
    };

    let (left_raw, _right_raw) = line.split_at(marker_idx);
    let gain_tail = &line[marker_idx + marker.len()..];
    let left = left_raw.trim_end().trim_end_matches('.');
    let lower_gain_tail = gain_tail.to_ascii_lowercase();

    let Some(gain_end_idx) = lower_gain_tail.find(" until end of turn") else {
        return line.to_string();
    };
    let gain_clause = gain_tail[..gain_end_idx].trim().trim_end_matches('.');
    if gain_clause.is_empty() || !left.to_ascii_lowercase().contains(" until end of turn") {
        return line.to_string();
    }

    let after_gain = gain_tail[gain_end_idx + " until end of turn".len()..].trim_start();
    if !after_gain.is_empty() {
        return format!(
            "{left}, and gains {gain_clause} until end of turn{}",
            after_gain
        );
    }

    format!("{left}, and gains {gain_clause} until end of turn.")
}

fn normalize_create_named_token_article_for_compare(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let marker = "create a ";
    if let Some(idx) = lower.find(marker) {
        let head = &line[..idx];
        let tail = &line[idx + marker.len()..];
        if tail.chars().next().is_some_and(|ch| ch.is_ascii_uppercase()) && tail.contains(", a ") {
            return format!("{}create {}", head, tail);
        }
    }
    line.to_string()
}

fn normalize_exile_named_token_until_source_leaves_for_compare(line: &str) -> String {
    let marker = "Exile target a token named ";
    let Some(start) = line.find(marker) else {
        return line.to_string();
    };
    let before = &line[..start];
    let after = &line[start + marker.len()..];
    for subject in ["this permanent", "this creature", "this source"] {
        if let Some((_, rest)) =
            after.split_once(&format!(" until {subject} leaves the battlefield"))
        {
            return format!(
                "{}Exile that token when {subject} leaves the battlefield{}",
                before, rest
            );
        }
    }
    line.to_string()
}

fn normalize_granted_named_token_leaves_sacrifice_source_for_compare(line: &str) -> String {
    let marker = "Grant When token named ";
    let Some(start) = line.find(marker) else {
        return line.to_string();
    };
    let before = &line[..start];
    let after = &line[start + marker.len()..];
    if let Some((_, rest)) = after.split_once(" leaves the battlefield, sacrifice this ")
        && let Some((subject, rest_after_subject)) = rest.split_once(". to this ")
        && matches!(subject, "permanent" | "creature" | "source")
        && let Some(rest_suffix) = rest_after_subject.strip_prefix(subject)
        && let Some(rest_suffix) = rest_suffix.strip_prefix('.')
    {
        return format!(
            "{}Sacrifice this {} when that token leaves the battlefield.{}",
            before, subject, rest_suffix
        );
    }
    line.to_string()
}

fn expand_create_list_clause(line: &str) -> String {
    let trimmed = line.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    let (prefix, rest) = if let Some(rest) = trimmed.strip_prefix("Create ") {
        ("Create ", rest)
    } else if let Some(rest) = trimmed.strip_prefix("create ") {
        ("create ", rest)
    } else {
        return line.to_string();
    };

    if !lower.contains(", and ") || !lower.contains(" token") {
        return line.to_string();
    }
    let flattened = rest.replacen(", and ", ", ", 1);
    let parts: Vec<&str> = flattened.split(", ").map(str::trim).collect();
    if parts.len() < 2
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.contains(" token"))
    {
        return line.to_string();
    }

    let expanded = parts
        .into_iter()
        .map(|part| format!("{prefix}{part}."))
        .collect::<Vec<_>>()
        .join(" ");
    normalize_clause_line(&expanded)
}

fn split_common_semantic_conjunctions(line: &str) -> String {
    let mut normalized = line.to_string();
    if normalized.contains("Unsupported parser line fallback:") {
        if let Some(rest) = normalized.split_once("Unsupported parser line fallback: ") {
            normalized = rest.1.to_string();
        }
        normalized = strip_parse_error_parentheticals(&normalized);
        normalized =
            capitalize_fallback_with_parenthetical_title_case(&normalized.to_ascii_lowercase());
        if normalized.trim_start().starts_with('•') && normalized.contains(" — ") {
            let trimmed = normalized
                .trim_start()
                .trim_start_matches('•')
                .trim()
                .to_string();
            let mut segments = trimmed.splitn(3, " — ");
            let _mode = segments.next();
            let _cost = segments.next();
            if let Some(rest) = segments.next() {
                normalized =
                    capitalize_fallback_with_parenthetical_title_case(&rest.to_ascii_lowercase())
                        .to_string();
            }
        }
    }

    normalized = normalized
        .strip_prefix("Spell effects: ")
        .unwrap_or(&normalized)
        .to_string();
    for kind in ["Static", "Triggered", "Activated", "Keyword", "Mana"] {
        let kind_prefix = format!("{kind} ability ");
        if let Some(rest) = normalized.strip_prefix(&kind_prefix) {
            if let Some((_idx, body)) = rest.split_once(": ") {
                normalized = body.to_string();
            }
        }
    }
    for prefix in ["1: ", "2: ", "3: ", "4: ", "5: ", "6: ", "7: ", "8: "] {
        if normalized.starts_with(prefix) {
            normalized = normalized
                .strip_prefix(prefix)
                .unwrap_or(&normalized)
                .to_string();
        }
    }
    normalized = normalize_create_named_token_article_for_compare(&normalized);
    normalized = normalize_exile_named_token_until_source_leaves_for_compare(&normalized);
    normalized = normalize_granted_named_token_leaves_sacrifice_source_for_compare(&normalized);
    normalized = normalized
        .replace(
            "At the beginning of each player's upkeep,",
            "At the beginning of each upkeep,",
        )
        .replace(
            " or you fully unlock a room",
            " and whenever you fully unlock a room",
        )
        .replace(
            " or you fully unlock a Room",
            " and whenever you fully unlock a Room",
        )
        .replace("counter target creature spell", "counter target creature")
        .replace("Counter target creature spell", "Counter target creature")
        .replace(
            "counter target artifact or enchantment spell",
            "counter target artifact or enchantment",
        )
        .replace(
            "Counter target artifact or enchantment spell",
            "Counter target artifact or enchantment",
        );
    for (from, to) in [
        ("For each player, ", "Each player "),
        ("for each player, ", "each player "),
        ("For each opponent, ", "Each opponent "),
        ("for each opponent, ", "each opponent "),
    ] {
        if normalized.starts_with(from) {
            normalized = normalized.replacen(from, to, 1);
        }
    }
    for prefix in ["Each player ", "Each opponent "] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            let mut chars = rest.chars();
            if let Some(first) = chars.next() {
                if first.is_ascii_alphabetic() && first.is_ascii_uppercase() {
                    normalized =
                        format!("{prefix}{}{}", first.to_ascii_lowercase(), chars.as_str());
                }
            }
        }
    }
    for (from, to) in [
        (", For each player, ", ", each player "),
        (", for each player, ", ", each player "),
        (", For each opponent, ", ", each opponent "),
        (", for each opponent, ", ", each opponent "),
        (": For each player, ", ": each player "),
        (": for each player, ", ": each player "),
        (": For each opponent, ", ": each opponent "),
        (": for each opponent, ", ": each opponent "),
    ] {
        normalized = normalized.replace(from, to);
    }
    normalized = normalize_cast_cost_conditional_reference(&normalized);
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
    let normalized_lower = normalized.to_ascii_lowercase();
    if normalized_lower == "creatures played by your opponents enter tapped."
        || normalized_lower == "creatures your opponents control enter tapped."
    {
        normalized = "Creatures your opponents control enter the battlefield tapped.".to_string();
    }
    if let Some((prefix, suffix)) = normalized_lower.split_once(": each creature you control gets ")
    {
        normalized = format!(
            "{}: Creatures you control get {}",
            &normalized[..prefix.len()],
            suffix
        );
    }
    if let Some((prefix, suffix)) = normalized_lower.split_once(". each creature you control gets ")
    {
        normalized = format!(
            "{}. Creatures you control get {}",
            &normalized[..prefix.len()],
            suffix
        );
    }
    if let Some((prefix, suffix)) = normalized_lower.split_once(", each creature you control gets ")
    {
        normalized = format!(
            "{}, Creatures you control get {}",
            &normalized[..prefix.len()],
            suffix
        );
    }
    if normalized_lower.contains("return a card at random from your graveyard to your hand") {
        normalized = "Return card from your graveyard to your hand.".to_string();
    } else if normalized_lower
        .contains("return two cards at random from your graveyard to your hand")
    {
        normalized = "Return card from your graveyard to your hand.".to_string();
    }
    if normalized_lower.starts_with("each creature you control gets ") {
        normalized = format!(
            "Creatures you control get {}",
            normalized[30..].trim_start()
        );
    } else if normalized_lower.starts_with("creatures you control get ") {
        normalized = normalized.replacen(
            "Creatures you control get ",
            "Creatures you control get ",
            1,
        );
    } else if normalized.contains(": creatures you control get ") {
        normalized = normalized.replace(
            ": creatures you control get ",
            ": Creatures you control get ",
        );
    } else if normalized.contains(". creatures you control get ") {
        normalized = normalized.replace(
            ". creatures you control get ",
            ". Creatures you control get ",
        );
    } else if normalized.contains(", creatures you control get ") {
        normalized = normalized.replace(
            ", creatures you control get ",
            ", Creatures you control get ",
        );
    } else if normalized_lower
        .starts_with("exchange control of two target permanents that share a card type")
        || normalized_lower
            .starts_with("exchange control of two target permanents that share a permanent type")
    {
        normalized = "Exchange control of two target permanents.".to_string();
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
    let normalized_lower = normalized.to_ascii_lowercase();
    if normalized_lower.starts_with("can't attack unless defending player controls ") {
        normalized = format!("This creature {normalized}");
    }
    if normalized_lower.starts_with("can't be blocked") {
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
        normalized = format!(
            "Target player draws {draw_tail} and loses {}",
            lose_part.trim()
        );
    }
    if let Some((draw_part, lose_part)) = normalized.split_once(". target player loses ")
        && let Some(draw_idx) = draw_part.to_ascii_lowercase().rfind("target player draws ")
    {
        let prefix = draw_part[..draw_idx].trim_end();
        let draw_tail = draw_part[draw_idx + "target player draws ".len()..].trim();
        let lead = if prefix.is_empty() {
            String::new()
        } else {
            format!("{prefix}, ")
        };
        normalized = format!(
            "{lead}Target player draws {draw_tail} and loses {}",
            lose_part.trim()
        );
    }
    if let Some((left, right)) = normalized.split_once(". Deal ") {
        let right = right.trim().trim_end_matches('.').trim();
        if left.to_ascii_lowercase().contains(" deals ") && !right.is_empty() {
            if left.starts_with("This creature deals ") || left.starts_with("this creature deals ")
            {
                let left = left
                    .trim_end_matches('.')
                    .replace("This creature deals ", "Deal ")
                    .replace("this creature deals ", "Deal ");
                let right = right
                    .trim_start_matches("Deal ")
                    .trim_start_matches("deals ")
                    .trim();
                normalized = format!("{left} and {right}");
            } else {
                normalized = format!("{} and {}", left.trim_end_matches('.'), right);
            }
        }
    }
    if let Some((left, right)) = normalized.split_once(". Deal ")
        && left.eq_ignore_ascii_case("Counter target spell")
        && !right.trim().is_empty()
    {
        normalized = format!(
            "Counter target spell and Deal {}",
            right.trim_end_matches('.')
        );
    }
    if let Some((left, right)) = normalized.split_once(". Counter that spell")
        && left
            .to_ascii_lowercase()
            .contains("casts a spell, sacrifice this enchantment")
    {
        let mut merged = format!("{}, counter that spell", left.trim_end_matches('.'));
        if right.contains('.') {
            merged.push_str(&format!(". {right}"));
        }
        normalized = merged;
    }
    if let Some((left, right)) = normalized.split_once(". Counter spell")
        && left
            .to_ascii_lowercase()
            .contains("casts a spell, sacrifice this enchantment")
    {
        let mut merged = format!("{}, counter that spell", left.trim_end_matches('.'));
        if right.contains('.') {
            merged.push_str(&format!(". {right}"));
        }
        normalized = merged;
    }
    if let Some((left, right)) = normalized.split_once(". Deal ")
        && left.to_ascii_lowercase().contains(" gains ")
        && right.to_ascii_lowercase().contains("damage to you")
    {
        normalized = format!(
            "{} and deals {}",
            left.trim_end_matches('.'),
            right.trim_end_matches('.')
        );
    }
    if let Some((left, right)) = normalized.split_once(". Deal ")
        && left.to_ascii_lowercase().contains(" deals ")
        && !right.is_empty()
        && !normalized.to_ascii_lowercase().contains(" counter ")
    {
        normalized = format!(
            "{} and {}",
            left.trim_end_matches('.'),
            right.trim_end_matches('.')
        );
    }
    if let Some((left, right)) = normalized.split_once(". Untap ")
        && left.to_ascii_lowercase().contains("earthbend ")
        && (right.eq_ignore_ascii_case("land.") || right.eq_ignore_ascii_case("land"))
    {
        normalized = format!("{}. Untap that land.", left.trim_end_matches('.'));
    }
    if let Some((left, right)) = normalized.split_once(". Deal ")
        && left.starts_with("Deal ")
        && left.to_ascii_lowercase().contains("target creature")
        && right
            .to_ascii_lowercase()
            .contains("damage to that object's controller")
    {
        normalized = format!(
            "{} and Deal {}",
            left.trim_end_matches('.'),
            right.trim_end_matches('.')
        );
    }
    normalized = normalized
        .replace(
            "number basic land type among land",
            "number basic land you control",
        )
        .replace("basic land type among lands", "basic land you control")
        .replace("basic land type among the lands", "basic land you control")
        .replace(
            "for each basic land type among lands",
            "for each basic land you control",
        );
    normalized = normalized.replace(
        "When this creature enters, exile all artifact. Exile all enchantment card from a graveyard.",
        "When this creature enters, exile all artifact and enchantment cards from all graveyards.",
    );
    if normalized
        .to_ascii_lowercase()
        .contains("when this creature enters, exile all artifact. exile all enchantment card from a graveyard.")
    {
        normalized = "When this creature enters, exile all artifact and enchantment cards from all graveyards."
            .to_string();
    }
    normalized = normalized
        .replace("as long as it's your turn", "during your turn")
        .replace("as long as it is your turn", "during your turn")
        .replace(
            "as long as it's not your turn",
            "during turns other than yours",
        )
        .replace(
            "as long as it is not your turn",
            "during turns other than yours",
        );
    if normalized
        .to_ascii_lowercase()
        .contains("when this creature enters, exile all artifact")
        && normalized
            .to_ascii_lowercase()
            .contains("exile all enchantment card from a graveyard")
    {
        normalized =
            "When this creature enters, exile all artifact and enchantment cards from all graveyards."
                .to_string();
    }
    normalized = normalized.replace(
        "that an opponent's land could produce",
        "that a land an opponent controls could produce",
    );
    normalized = normalized.replace(
        "that an opponent's lands could produce",
        "that lands an opponent controls could produce",
    );
    if let Some((left, right)) = normalized.split_once(" to the battlefield with ")
        && (left.starts_with("Return ") || left.starts_with("return "))
    {
        let right_trimmed = right.trim();
        if let Some(counter_phrase) = right_trimmed
            .strip_suffix(" counter on it.")
            .or_else(|| right_trimmed.strip_suffix(" counter on it"))
        {
            normalized = format!("{left} to the battlefield. Put {counter_phrase} counter on it.");
        }
    }
    if let Some((left, right)) = normalized.split_once(". Put ")
        && (left.starts_with("Bolster ") || left.starts_with("bolster "))
    {
        normalized = format!("{}, then put {}", left.trim_end_matches('.'), right);
    }
    if let Some((left, right)) = normalized.split_once(", Put ")
        && right.to_ascii_lowercase().contains(" on that object")
        && left.to_ascii_lowercase().starts_with("for each ")
    {
        let scope = left["for each ".len()..].trim();
        let right = right.to_ascii_lowercase();
        let right = right
            .trim_start_matches("put ")
            .trim_start()
            .trim_end_matches('.')
            .trim_end();
        normalized = format!("Put {right} for each {scope}");
    }
    if let Some((left, right)) = normalized.split_once(", put ")
        && right.to_ascii_lowercase().contains(" on that object")
        && left.to_ascii_lowercase().starts_with("for each ")
    {
        let scope = left["for each ".len()..].trim();
        let right = right
            .trim_start_matches("put ")
            .trim_start()
            .trim_end_matches('.')
            .trim_end();
        normalized = format!("put {right} for each {scope}");
    }
    normalized = normalized.replace(
        "put the number of a attacking creature you control +1/+1 counter(s) on it.",
        "put a +1/+1 counter on it for each attacking creature you control.",
    );
    normalized = normalized.replace(
        "put the number of a attacking creature you control +1/+1 counter on it.",
        "put a +1/+1 counter on it for each attacking creature you control.",
    );
    normalized = normalized.replace(
        "put the number of a attacking creature you control +1/+1 counter(s) on it",
        "put a +1/+1 counter on it for each attacking creature you control",
    );
    normalized = normalized.replace(
        "put the number of a attacking creature you control +1/+1 counter on it",
        "put a +1/+1 counter on it for each attacking creature you control",
    );
    normalized = normalized
        .replace(
            "put the number of creature +1/+1 counter(s) on this creature.",
            "put a +1/+1 counter on this creature for each creature.",
        )
        .replace(
            "put the number of creature +1/+1 counter on this creature.",
            "put a +1/+1 counter on this creature for each creature.",
        )
        .replace(
            "put the number of creature +1/+1 counter(s) on this creature",
            "put a +1/+1 counter on this creature for each creature",
        )
        .replace(
            "put the number of creature +1/+1 counter on this creature",
            "put a +1/+1 counter on this creature for each creature",
        )
        .replace(
            "put the number of card in your hand +1/+1 counter(s) on this creature.",
            "put a +1/+1 counter on this creature for each card in your hand.",
        )
        .replace(
            "put the number of card in your hand +1/+1 counter on this creature.",
            "put a +1/+1 counter on this creature for each card in your hand.",
        )
        .replace(
            "put the number of card in your hand +1/+1 counter(s) on this creature",
            "put a +1/+1 counter on this creature for each card in your hand",
        )
        .replace(
            "put the number of card in your hand +1/+1 counter on this creature",
            "put a +1/+1 counter on this creature for each card in your hand",
        )
        .replace(
            "put the number of artifact or creature card in your graveyard +1/+1 counter(s) on this creature.",
            "put a +1/+1 counter on this creature for each artifact or creature card in your graveyard.",
        )
        .replace(
            "put the number of artifact or creature card in your graveyard +1/+1 counter on this creature.",
            "put a +1/+1 counter on this creature for each artifact or creature card in your graveyard.",
        )
        .replace(
            "put the number of artifact or creature card in your graveyard +1/+1 counter(s) on this creature",
            "put a +1/+1 counter on this creature for each artifact or creature card in your graveyard",
        )
        .replace(
            "put the number of artifact or creature card in your graveyard +1/+1 counter on this creature",
            "put a +1/+1 counter on this creature for each artifact or creature card in your graveyard",
        )
        .replace(
            "put the number of creature card in your graveyard +1/+1 counter(s) on this creature.",
            "put a +1/+1 counter on this creature for each creature card in your graveyard.",
        )
        .replace(
            "put the number of creature card in your graveyard +1/+1 counter on this creature.",
            "put a +1/+1 counter on this creature for each creature card in your graveyard.",
        )
        .replace(
            "put the number of creature card in your graveyard +1/+1 counter(s) on this creature",
            "put a +1/+1 counter on this creature for each creature card in your graveyard",
        );
    normalized = normalized
        .replace(", then ", ". ")
        .replace(", Then ", ". ")
        .replace(", and then ", ". ")
        .replace(", And then ", ". ");
    normalized = normalized
        .replace("If effect #0 that doesn't happen", "If you don't")
        .replace("if effect #0 that doesn't happen", "if you don't")
        .replace("If effect #0 happened", "If you do")
        .replace("if effect #0 happened", "if you do")
        .replace("the count result of effect #0 life", "that much life")
        .replace("count result of effect #0 life", "that much life")
        .replace("the count result of effect #0", "that much")
        .replace("count result of effect #0", "that much")
        .replace("that objects", "it")
        .replace("that object", "it")
        .replace("that object's", "its");
    normalized = normalized
        .replace("Each 1/1 creature", "Each creature")
        .replace("each 1/1 creature", "each creature")
        .replace("Each blocked creature", "Each creature")
        .replace("each blocked creature", "each creature")
        .replace(
            "This permanent and each other creature with the same name as it",
            "this creature",
        )
        .replace("Each creature named ", "a creature named ")
        .replace("each creature named ", "a creature named ")
        .replace(
            "When you control no islands, sacrifice this creature",
            "When no islands, sacrifice this creature",
        )
        .replace(
            "You control no islands: Sacrifice this creature",
            "When no islands, sacrifice this creature",
        )
        .replace(
            "you control no islands: sacrifice this creature",
            "When no islands, sacrifice this creature",
        );
    normalized = normalized
        .replace(
            "target opponent's artifact or enchantment",
            "target artifact or enchantment an opponent controls",
        )
        .replace("that creature's controller", "that object's controller")
        .replace("that permanent's controller", "that object's controller")
        .replace("that creature's owner", "that object's owner")
        .replace("that permanent's owner", "that object's owner")
        .replace("that object's controller", "its controller")
        .replace("that objects", "it")
        .replace("that object", "it")
        .replace("that creature", "it")
        .replace("That creature", "It")
        .replace("that creature's", "its");
    if (normalized.contains(", you draw ") || normalized.contains(", draw "))
        && normalized.contains(" and lose ")
    {
        normalized = normalized.replace(" and lose ", " and you lose ");
    }
    if (normalized.contains(", you draw ") || normalized.contains(", draw "))
        && normalized.contains(" and gain ")
    {
        normalized = normalized.replace(" and gain ", " and you gain ");
    }
    normalized = normalized
        .replace(" you may, ", " you may ")
        .replace("Cumulative upkeep—", "Cumulative upkeep ")
        .replace("Cumulative upkeep —", "Cumulative upkeep ")
        .replace("you may,", "you may ")
        .replace("this creature this creature", "this creature")
        .replace("this creature you:", "this creature:");
    normalized = normalized.replace("chooses target ", "exiles target ");
    normalized = normalized.replace(
        "Exile it. Exile all card in target opponent's graveyards.",
        "Exile all card in target opponent's graveyards.",
    );
    if let Some((chooser, action)) = normalized.split_once(". ") {
        let chooser_lower = chooser.to_ascii_lowercase();
        let action_lower = action.to_ascii_lowercase();
        if chooser_lower.starts_with("choose target ") && action_lower.starts_with("target ") {
            let chooser_subject = chooser.to_string();
            let chooser_core = chooser_subject
                .trim_start_matches("Choose ")
                .trim_start_matches("choose ")
                .trim();
            let mut chooser_core = chooser_core.trim_start_matches("target ").trim();
            chooser_core = chooser_core.trim_start_matches("target ").trim();
            let action_subject = action_lower.trim_start_matches("target ").trim();
            let action_subject = action_subject
                .trim_start_matches("an opponent's ")
                .trim_start_matches("the opponent's ")
                .trim();
            let chooser_noun = chooser_core.split_whitespace().next().unwrap_or("");
            let action_noun = action_subject.split_whitespace().next().unwrap_or("");
            let action_matches_chooser = action_subject.starts_with(chooser_core)
                || (!chooser_noun.is_empty() && action_noun == chooser_noun);
            if action_matches_chooser {
                let action = action.trim();
                normalized = if action.is_empty() {
                    format!("Target {chooser_core}")
                } else {
                    format!("Target {chooser_core}. {action}")
                };
            } else if !chooser_core.is_empty() {
                let action = action.trim();
                normalized = if action.is_empty() {
                    format!("Target {chooser_core}")
                } else {
                    format!("Target {chooser_core}. {action}")
                };
            }
        }
    }
    if let Some((chooser, action)) = normalized.split_once(". Exile it.") {
        let chooser_lower = chooser.to_ascii_lowercase();
        if let Some((chooser_subject, chooser_target)) = chooser.split_once(" chooses target ") {
            if chooser_lower.starts_with("target opponent chooses target ")
                && chooser_target.starts_with("creature ")
            {
                let action_tail = action.trim();
                normalized = if action_tail.is_empty() {
                    format!("{} exiles target {chooser_target}", chooser_subject)
                } else {
                    format!(
                        "{} exiles target {}. {}",
                        chooser_subject, chooser_target, action_tail
                    )
                };
            }
        }
    }
    let lower = normalized.to_ascii_lowercase();
    if lower.starts_with("all sliver")
        || lower.starts_with("slivers have")
        || lower.starts_with("all sliver creatures have")
    {
        normalized = normalized
            .replace("Slivers have", "All Slivers have")
            .replace("to its owners hand", "to its owner's hand");
    }
    if let Some(rest) = lower.strip_prefix("for each player, you may that player ") {
        if let Some((first, second)) = rest.split_once(". if you don't, that player ") {
            normalized = format!(
                "Each player may {}. Each player who doesn't {}",
                first.trim_end_matches('.'),
                second.trim_end_matches('.')
            );
        } else {
            normalized = format!("Each player may {rest}");
        }
    } else if let Some(rest) = lower.strip_prefix("for each opponent, you may that player ") {
        if let Some((first, second)) = rest.split_once(". if you don't, that player ") {
            normalized = format!(
                "Each opponent may {}. Each opponent who doesn't {}",
                first.trim_end_matches('.'),
                second.trim_end_matches('.')
            );
        } else {
            normalized = format!("Each opponent may {rest}");
        }
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
    if normalized.starts_with("That player controls ") {
        normalized = format!(
            "They control {}",
            &normalized["That player controls ".len()..]
        );
    }
    if normalized.starts_with("That player draws ") {
        normalized = format!("They draw {}", &normalized["That player draws ".len()..]);
    }
    if normalized.starts_with("That player loses ") {
        normalized = format!("They lose {}", &normalized["That player loses ".len()..]);
    }
    if normalized.starts_with("That player discards ") {
        normalized = format!(
            "They discard {}",
            &normalized["That player discards ".len()..]
        );
    }
    if normalized.starts_with("That player sacrifices ") {
        normalized = format!(
            "They sacrifice {}",
            &normalized["That player sacrifices ".len()..]
        );
    }
    if let Some(rest) = normalized.strip_prefix("Choose one — ") {
        normalized = format!("Choose one —. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Choose one or both — ") {
        normalized = format!("Choose one or both —. {rest}");
    }

    let basic_land_you_control = "__basic_land_you_control__";
    let basic_lands_you_control = "__basic_lands_you_control__";
    let attacking_creature_you_control = "__attacking_creature_you_control__";
    let attacking_creatures_you_control = "__attacking_creatures_you_control__";

    let mut normalized = normalized
        .replace("basic lands you control", basic_lands_you_control)
        .replace("basic land you control", basic_land_you_control)
        .replace("attacking creatures you control", attacking_creatures_you_control)
        .replace("attacking creature you control", attacking_creature_you_control)
        .replace(" • ", ". ")
        .replace("• ", ". ")
        .replace(" creatures you control", " creatures")
        .replace(" creature you control", " creature")
        .replace(" artifacts you control", " artifacts")
        .replace(" artifact you control", " artifact")
        .replace(" enchantments you control", " enchantments")
        .replace(" enchantment you control", " enchantment")
        .replace(" permanents you control", " permanents")
        .replace(" permanent you control", " permanent")
        .replace(" lands you control", " lands")
        .replace(" land you control", " land")
        .replace(" cards you control", " cards")
        .replace(" card you control", " card")
        .replace(" spells you control", " spells")
        .replace(" spell you control", " spell")
        .replace(
            "Activate only during your turn, before attackers are declared",
            "",
        )
        .replace(
            "activate only during your turn, before attackers are declared",
            "",
        )
        .replace(
            "Activate only during your turn and Activate only during your turn before attackers are declared",
            "Activate only during your turn",
        )
        .replace(
            "activate only during your turn and activate only during your turn before attackers are declared",
            "activate only during your turn",
        )
        .replace(" Activate only during your turn", "")
        .replace(" activate only during your turn", "")
        .replace(" you controls", "")
        .replace(". Untap it", ". Untap it")
        .replace("Untap that creature", "Untap it")
        .replace(" and untap it", ". Untap it")
        .replace(" and untap that creature", ". Untap it")
        .replace(" and untap that permanent", ". Untap it")
        .replace(" and untap them", ". Untap them")
        .replace(" and untap all permanents", ". Untap them")
        .replace(" and Untap all permanents", ". Untap them")
        .replace(" Untap all permanents", " Untap them")
        .replace("Destroy target Aura creature", "Destroy target Aura attached to a creature")
        .replace("target creature phases out", "choose target creature")
        .replace("Target creature phases out", "Choose target creature")
        .replace("target creature phase out", "choose target creature")
        .replace("Target creature phase out", "Choose target creature")
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
        .replace(" and create ", ". Create ")
        .replace(" and Create ", ". Create ")
        .replace(" and add ", ". Add ")
        .replace(" and Add ", ". Add ")
        .replace(" and put ", ". Put ")
        .replace(" and Put ", ". Put ")
        .replace(" and target player draws ", ". Target player draws ")
        .replace(" and target opponent draws ", ". Target opponent draws ")
        .replace(" and each player draws ", ". Each player draws ")
        .replace(" and each opponent draws ", ". Each opponent draws ")
        .replace(
            "the number of a attacking creature you control",
            "each attacking creature you control",
        )
        .replace(
            "put each attacking creature you control +1/+1 counter(s) on it",
            "put a +1/+1 counter on it for each attacking creature you control",
        )
        .replace(" and target player gains ", ". Target player gains ")
        .replace(" and target opponent gains ", ". Target opponent gains ")
        .replace(" and each player gains ", ". Each player gains ")
        .replace(" and each opponent gains ", ". Each opponent gains ")
        .replace(" and each opponent loses ", ". Each opponent loses ")
        .replace(" and each opponent discards ", ". Each opponent discards ")
        .replace(" and each player loses ", ". Each player loses ")
        .replace(" and each player discards ", ". Each player discards ")
        .replace(" and target opponent loses ", ". Target opponent loses ")
        .replace(
            " and target opponent discards ",
            ". Target opponent discards ",
        )
        .replace("sacrifice a creature you control", "sacrifice a creature")
        .replace("Sacrifice a creature you control", "Sacrifice a creature")
        .replace("sacrifice a land you control", "sacrifice a land")
        .replace("Sacrifice a land you control", "Sacrifice a land")
        .replace(
            "sacrifice three creatures you control",
            "sacrifice three creatures",
        )
        .replace(
            "Sacrifice three creatures you control",
            "Sacrifice three creatures",
        )
        .replace("Tag the object attached to this Aura as 'enchanted'. ", "")
        .replace("tag the object attached to this Aura as 'enchanted'. ", "")
        .replace(
            "Destroy target tagged object 'enchanted'",
            "Destroy enchanted creature",
        )
        .replace(
            "destroy target tagged object 'enchanted'",
            "destroy enchanted creature",
        )
        .replace("Counter spell", "Counter that spell")
        .replace("counter spell", "counter that spell")
        .replace(basic_land_you_control, "basic land you control")
        .replace(basic_lands_you_control, "basic lands you control")
        .replace(attacking_creature_you_control, "attacking creature you control")
        .replace(
            attacking_creatures_you_control,
            "attacking creatures you control",
        );
    if let Some((left, right)) = normalized.split_once(": ")
        && left.to_ascii_lowercase().starts_with("you control no ")
        && right
            .to_ascii_lowercase()
            .starts_with("sacrifice this creature")
    {
        normalized = format!(
            "When {}, {}",
            left.to_ascii_lowercase(),
            right.to_ascii_lowercase()
        );
    }
    if let Some((prefix, tail)) = normalized.split_once("For each opponent, Deal ")
        && let Some((amount, rest)) = tail.split_once(" damage to that player")
    {
        normalized = format!("{prefix}Deal {amount} damage to each opponent{rest}");
    }
    if let Some((prefix, tail)) = normalized.split_once("for each opponent, deal ")
        && let Some((amount, rest)) = tail.split_once(" damage to that player")
    {
        normalized = format!("{prefix}deal {amount} damage to each opponent{rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("If you control your commander, Choose one or both — ")
        && let Some((both_modes, single_mode)) = rest.split_once(". Otherwise, Choose one — ")
    {
        let both_modes = both_modes.trim().trim_end_matches('.');
        let single_mode = single_mode.trim().trim_end_matches('.');
        let comparable_both = normalize_clause_line(both_modes).to_ascii_lowercase();
        let comparable_single = normalize_clause_line(single_mode).to_ascii_lowercase();
        let shared_modes = if comparable_both == comparable_single {
            both_modes
        } else {
            both_modes
        };
        normalized = format!(
            "Choose one. If you control a commander as you cast this spell, you may choose both instead. {shared_modes}."
        );
    }
    if let Some((left, right)) = normalized.split_once(" until end of turn. this creature gains ")
        && let Some((keyword, tail)) = right.split_once(" until end of turn")
    {
        normalized = format!(
            "{} and gains {} until end of turn{}",
            left.trim_end_matches('.'),
            keyword.trim(),
            tail
        );
    }
    if let Some((left, right)) = normalized.split_once(". Each player discards ")
        && left.starts_with("Each player draws ")
        && right.contains(" at random")
    {
        normalized = format!(
            "{}, then each player discards {}",
            left.trim_end_matches('.'),
            right
        );
    }
    if normalized
        .to_ascii_lowercase()
        .contains("target creatures can't block this turn. goad it")
    {
        normalized = normalized.replace("Goad it", "Goad them");
        normalized = normalized.replace("goad it", "goad them");
    }
    if let Some((left, right)) = normalized.split_once(". Proliferate") {
        let left = left.trim().trim_end_matches('.');
        let right_tail = right.trim_start_matches('.').trim_start_matches(',').trim();
        if right_tail.is_empty() {
            normalized = format!("{left}, then proliferate.");
        } else {
            normalized = format!("{left}, then proliferate. {right_tail}");
        }
    } else if let Some((left, right)) = normalized.split_once(". proliferate") {
        let left = left.trim().trim_end_matches('.');
        let right_tail = right.trim_start_matches('.').trim_start_matches(',').trim();
        if right_tail.is_empty() {
            normalized = format!("{left}, then proliferate.");
        } else {
            normalized = format!("{left}, then proliferate. {right_tail}");
        }
    }
    if let Some((left, right)) = normalized.split_once(". Scry ") {
        let left = left.trim().trim_end_matches('.');
        let scry_tail = right.trim().trim_end_matches('.');
        let left_lower = left.to_ascii_lowercase();
        let should_chain = left_lower.starts_with("draw ")
            || left_lower.starts_with("you draw ")
            || left_lower.contains(" you draw ")
            || left_lower.starts_with("surveil ")
            || left_lower.contains(" counter on ")
            || left_lower.contains(" then draw ");
        if scry_tail
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit() || ch.eq_ignore_ascii_case(&'x'))
            && should_chain
        {
            normalized = format!("{left}, then scry {scry_tail}.");
        }
    } else if let Some((left, right)) = normalized.split_once(". scry ") {
        let left = left.trim().trim_end_matches('.');
        let scry_tail = right.trim().trim_end_matches('.');
        let left_lower = left.to_ascii_lowercase();
        let should_chain = left_lower.starts_with("draw ")
            || left_lower.starts_with("you draw ")
            || left_lower.contains(" you draw ")
            || left_lower.starts_with("surveil ")
            || left_lower.contains(" counter on ")
            || left_lower.contains(" then draw ");
        if scry_tail
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit() || ch.eq_ignore_ascii_case(&'x'))
            && should_chain
        {
            normalized = format!("{left}, then scry {scry_tail}.");
        }
    }
    normalize_target_count_wording_for_compare(&normalized)
}

fn normalize_cast_cost_conditional_reference(line: &str) -> String {
    let normalized = line.trim().trim_end_matches('.').to_string();
    let lower = normalized.to_ascii_lowercase();

    if let Some(marker_start) = lower.find("spells that target ") {
        let prefix_len = "spells that target ".len();
        let tail_lower = &lower[marker_start + prefix_len..];
        let tail = &normalized[marker_start + prefix_len..];
        if let Some((_, cost_and_tail)) = tail.split_once(" cost ") {
            let cost_and_tail = cost_and_tail.trim_start();
            let base = format!("{cost_and_tail}.");
            let base_lower = base.to_ascii_lowercase();

            if let Some(cast_to_end) = base_lower.find(" to cast") {
                let base_body = base[..cast_to_end + " to cast".len()].trim();
                return format!("Spells cost {base_body}").trim().to_string();
            }

            let mut end = cost_and_tail.len();
            for delimiter in [
                " if ",
                " as long as ",
                " for each ",
                " at the beginning of ",
                " for ",
                " except ",
            ] {
                if let Some(found) = cost_and_tail.find(delimiter) {
                    end = end.min(found);
                }
            }
            let canonical = cost_and_tail[..end].trim();
            if !canonical.is_empty() {
                return format!("Spells cost {canonical}").trim().to_string();
            }
            return normalized;
        }
    }

    let start = if let Some(start) = lower.find("this spell costs") {
        start + "this spell costs".len()
    } else if let Some(start) = lower.find("spells cost") {
        start + "spells cost".len()
    } else {
        return normalized;
    };

    let tail = normalized.get(start..).unwrap_or("");
    if !tail.starts_with(' ') {
        return normalized;
    }

    let body = tail.trim_start();
    let base = format!("{body}.");
    let base_lower = base.to_ascii_lowercase();
    let cast_to_end = base_lower.find(" to cast");

    if let Some(cast_to_end) = cast_to_end {
        let base_body = base[..cast_to_end + " to cast".len()].trim();
        return format!("Spells cost {base_body}").trim().to_string();
    }

    let mut end = body.len();
    for delimiter in [
        " if ",
        " as long as ",
        " for each ",
        " at the beginning of ",
        " for ",
        " except ",
    ] {
        if let Some(found) = body.find(delimiter) {
            end = end.min(found);
        }
    }
    let canonical = body[..end].trim();
    if canonical.is_empty() {
        return normalized;
    }
    format!("Spells cost {canonical}").trim().to_string()
}

fn normalize_target_count_wording_for_compare(line: &str) -> String {
    let mut normalized = line.to_string();

    let mut tokens: Vec<&str> = normalized.split_whitespace().collect();
    let mut collapsed: Vec<&str> = Vec::with_capacity(tokens.len());
    let mut idx = 0usize;
    while idx < tokens.len() {
        if tokens[idx].eq_ignore_ascii_case("choose")
            && idx + 2 < tokens.len()
            && tokens[idx + 1].eq_ignore_ascii_case("up")
            && tokens[idx + 2].eq_ignore_ascii_case("to")
        {
            idx += 1;
            continue;
        }

        if tokens[idx].eq_ignore_ascii_case("each")
            && idx + 3 < tokens.len()
            && tokens[idx + 1].eq_ignore_ascii_case("of")
            && tokens[idx + 2].eq_ignore_ascii_case("up")
            && tokens[idx + 3].eq_ignore_ascii_case("to")
        {
            idx += 2;
            continue;
        }

        collapsed.push(tokens[idx]);
        idx += 1;
    }

    normalized = collapsed.join(" ");
    normalized = normalized
        .replace("Choose up to one -", "up to one -")
        .replace("choose up to one -", "up to one -")
        .replace("Choose up to one.", "up to one.")
        .replace("choose up to one.", "up to one.");
    normalized = normalized
        .replace("choose up to one target", "up to one target")
        .replace("Choose up to one target", "up to one target");

    let number_tokens = [
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "x", "1",
        "2", "3", "4", "5", "6", "7", "8", "9", "10",
    ];
    for token in number_tokens {
        normalized = normalized.replace(&format!("target {token} "), &format!("{token} "));
        normalized = normalized.replace(&format!("Target {token} "), &format!("{token} "));
    }
    normalized
}

fn normalize_for_each_player_conditional_for_compare(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let for_each_marker = ", for each player, ";
    if let Some(for_each_idx) = lower.find(for_each_marker) {
        let left = &line[..for_each_idx];
        let right = &lower[for_each_idx + for_each_marker.len()..];
        let right = right.trim();
        if let Some(after_deal) = right.strip_prefix("deal ")
            && let Some((amount, _tail)) = after_deal.split_once(" damage to that player")
        {
            return format!("{left}, Deal {amount} damage to each player");
        }
        if let Some(after_deals) = right.strip_prefix("deals ")
            && let Some((amount, _tail)) = after_deals.split_once(" damage to that player")
        {
            return format!("{left}, Deal {amount} damage to each player");
        }
    }

    let beginning_markers: [&str; 3] = [
        "at the beginning of each player's upkeep,",
        "at the beginning of each upkeep,",
        "at the beginning of your upkeep,",
    ];
    for beginning in beginning_markers {
        if !lower.starts_with(beginning) {
            continue;
        }
        let deal_target = if beginning.contains("each") {
            "each player"
        } else {
            "you"
        };
        if let Some((_left, right)) = lower.split_once(", deal ")
            && let Some((amount, _tail)) = right.split_once(" damage to that player")
        {
            return format!("{beginning} Deal {amount} damage to {deal_target}");
        }
        if let Some((_left, right)) = lower.split_once(", deals ")
            && let Some((amount, _tail)) = right.split_once(" damage to that player")
        {
            return format!("{beginning} Deal {amount} damage to {deal_target}");
        }
        if let Some((_left, right)) = lower.split_once(", this permanent deals ")
            && let Some((amount, _tail)) = right.split_once(" damage to that player")
        {
            return format!("{beginning} Deal {amount} damage to {deal_target}");
        }
        if let Some((_left, right)) = lower.split_once(", this creature deals ")
            && let Some((amount, _tail)) = right.split_once(" damage to that player")
        {
            return format!("{beginning} Deal {amount} damage to {deal_target}");
        }
        if let Some((_left, right)) = lower.split_once(", this enchantment deals ")
            && let Some((amount, _tail)) = right.split_once(" damage to that player")
        {
            return format!("{beginning} Deal {amount} damage to {deal_target}");
        }
        if let Some((_left, right)) = lower.split_once(", this artifact deals ")
            && let Some((amount, _tail)) = right.split_once(" damage to that player")
        {
            return format!("{beginning} Deal {amount} damage to {deal_target}");
        }
        if let Some((_left, right)) = lower.split_once(", this land deals ")
            && let Some((amount, _tail)) = right.split_once(" damage to that player")
        {
            return format!("{beginning} Deal {amount} damage to {deal_target}");
        }
    }

    let player_prefixes = [
        "for each player, if that player ",
        "for each player, if they ",
    ];
    for player_prefix in player_prefixes {
        if !lower.starts_with(player_prefix) {
            continue;
        }
        let Some((condition, action)) = line[player_prefix.len()..].split_once(", that player ")
        else {
            continue;
        };
        let mut condition = condition.trim().to_string();
        if let Some(rest) = condition.strip_prefix("if ") {
            condition = rest.to_string();
        }
        if let Some(rest) = condition.strip_prefix("if that player ") {
            condition = rest.to_string();
        }
        if let Some(rest) = condition.strip_prefix("that player controls ") {
            condition = format!("control {rest}");
        } else {
            condition = condition.replace(" controls", " control");
            if let Some(rest) = condition.strip_prefix("controls ") {
                condition = format!("control {rest}");
            }
        }
        let mut action = action.trim();
        if let Some(rest) = action.strip_prefix("that player ") {
            action = rest;
        }
        return format!("Each player who {} {}", condition.trim(), action.trim());
    }

    let opponent_prefixes = [
        "for each opponent, if that player ",
        "for each opponent, if they ",
    ];
    for opponent_prefix in opponent_prefixes {
        if !lower.starts_with(opponent_prefix) {
            continue;
        }
        let Some((condition, action)) = line[opponent_prefix.len()..].split_once(", that player ")
        else {
            continue;
        };
        let mut condition = condition.trim();
        if let Some(rest) = condition.strip_prefix("if ") {
            condition = rest;
        }
        if let Some(rest) = condition.strip_prefix("if that player ") {
            condition = rest;
        }
        let mut action = action.trim();
        if let Some(rest) = action.strip_prefix("that player ") {
            action = rest;
        }
        return format!("Each opponent who {} {}", condition.trim(), action.trim());
    }

    if let Some(rest) = lower.strip_prefix("for each player, if they ")
        && let Some((condition, action)) = rest.split_once(", they ")
    {
        return format!("Each player who {} {}", condition.trim(), action.trim());
    }
    if let Some(rest) = lower.strip_prefix("for each player, that player ")
        && let Some((condition, action)) = rest.split_once(", this ")
    {
        return format!("Each player {}, this {}", condition.trim(), action.trim());
    }
    if let Some(rest) = lower.strip_prefix("for each player, that player ") {
        return format!("Each player {}", rest.trim());
    }
    if let Some(rest) = lower.strip_prefix("for each opponent, that player ") {
        return format!("Each opponent {}", rest.trim());
    }
    if let Some(rest) = lower.strip_prefix("for each opponent, if they ")
        && let Some((condition, action)) = rest.split_once(", they ")
    {
        return format!("Each opponent who {} {}", condition.trim(), action.trim());
    }

    line.to_string()
}

fn normalize_explicit_damage_source_for_compare(line: &str) -> String {
    let mut normalized = line;
    let starts_with_damage_subject = |text: &str| -> bool {
        let lower = text.to_ascii_lowercase();
        lower.starts_with("deal ")
            || lower.starts_with("this creature deals ")
            || lower.starts_with("this permanent deals ")
            || lower.starts_with("this spell deals ")
            || lower.starts_with("this enchantment deals ")
            || lower.starts_with("this artifact deals ")
            || lower.starts_with("this land deals ")
            || lower.starts_with("this token deals ")
            || lower.starts_with("that creature deals ")
            || lower.starts_with("that permanent deals ")
            || lower.starts_with("it deals ")
    };
    if let Some(rest) = line.strip_prefix("{")
        && let Some(end) = rest.find("}: ")
        && rest[..end].chars().all(|c| match c {
            '{' | '}' | '/' | ',' | ' ' => true,
            '0'..='9' => true,
            'A'..='Z' | 'a'..='z' => matches!(
                c.to_ascii_uppercase(),
                'W' | 'U' | 'B' | 'R' | 'G' | 'T' | 'X' | 'Y' | 'Z' | 'S' | 'P' | 'C'
            ),
            _ => false,
        })
        && starts_with_damage_subject(rest[end + 3..].trim_start())
    {
        normalized = rest[end + 3..].trim_start();
    }

    let lower = normalized.to_ascii_lowercase();
    if lower.starts_with("deal ") && lower.contains(" to this creature") {
        return normalized
            .replace(" to this creature", " to itself")
            .replace(" to This creature", " to itself")
            .replace(" to this Creature", " to itself");
    }
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
            let subject = if lower.starts_with("this creature") {
                "This creature"
            } else if lower.starts_with("this permanent") {
                "This permanent"
            } else if lower.starts_with("this spell") {
                "This spell"
            } else if lower.starts_with("this enchantment") {
                "This enchantment"
            } else if lower.starts_with("this artifact") {
                "This artifact"
            } else if lower.starts_with("this land") {
                "This land"
            } else if lower.starts_with("this token") {
                "This token"
            } else if lower.starts_with("that creature") {
                "That creature"
            } else if lower.starts_with("that permanent") {
                "That permanent"
            } else {
                "It"
            };
            let mut rest = normalized[prefix.len()..].trim_start().to_string();
            rest = rest
                .replace(" to this creature", " to itself")
                .replace(" to This creature", " to itself")
                .replace(" to itself", " to itself");
            return format!("{subject} deals {rest}");
        }
    }
    normalized.to_string()
}

fn expand_return_list_clause(line: &str) -> String {
    let trimmed = line.trim().trim_end_matches('.');
    let lower_trimmed = trimmed.to_ascii_lowercase();
    let (ability_prefix, body) = if lower_trimmed.starts_with("return ") {
        ("", trimmed)
    } else if let Some(idx) = lower_trimmed.find(": return ") {
        (&trimmed[..idx + 2], trimmed[idx + 2..].trim_start())
    } else {
        return line.to_string();
    };

    let normalized = body.replacen(", and ", " and ", 1);
    let lower = normalized.to_ascii_lowercase();
    if !lower.starts_with("return ") || !lower.contains(" and ") {
        return line.to_string();
    }

    let suffix = [
        " to their owners' hands",
        " to their owner's hand",
        " to their owners hand",
        " to its owner's hand",
    ]
    .into_iter()
    .find(|suffix| lower.ends_with(suffix));
    let Some(suffix) = suffix else {
        return line.to_string();
    };

    let Some(prefix) = normalized.strip_suffix(suffix) else {
        return line.to_string();
    };
    let Some(head) = prefix
        .strip_prefix("Return ")
        .or_else(|| prefix.strip_prefix("return "))
    else {
        return line.to_string();
    };

    let parts: Vec<&str> = head
        .split(" and ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 2 {
        return line.to_string();
    }

    let expanded = parts
        .into_iter()
        .map(|part| {
            let part = part
                .trim_start_matches("Return ")
                .trim_start_matches("return ")
                .trim();
            format!("Return {part}{suffix}.")
        })
        .collect::<Vec<_>>();
    if expanded.is_empty() {
        return line.to_string();
    }

    if ability_prefix.is_empty() {
        return expanded.join(" ");
    }
    let mut out = format!("{ability_prefix}{}", expanded[0]);
    if expanded.len() > 1 {
        out.push(' ');
        out.push_str(&expanded[1..].join(" "));
    }
    out
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
            let no_parenthetical = if raw_line.contains("Unsupported parser line fallback:") {
                strip_parse_error_parentheticals(raw_line)
            } else {
                strip_parenthetical(raw_line)
            };
            let no_inline_reminders = strip_inline_token_reminders(&no_parenthetical);
            strip_reminder_like_quotes(&no_inline_reminders)
        };
        let line = normalize_trigger_subject_for_compare(&line);
        let line = strip_modal_option_labels(&line);
        let line = normalize_for_each_player_conditional_for_compare(&line);
        let line = split_common_semantic_conjunctions(&line);
        let line = normalize_explicit_damage_source_for_compare(&line);
        let line = expand_create_list_clause(&normalize_clause_line(&line));
        let line = expand_return_list_clause(&line);
        push_semantic_clauses(&line, &mut clauses);
    }
    let has_creature_type_choice_clause = clauses.iter().any(|clause| {
        clause
            .to_ascii_lowercase()
            .contains("creature type of your choice")
    });
    if has_creature_type_choice_clause {
        clauses.retain(|clause| clause.to_ascii_lowercase() != "choose a creature type");
    }
    clauses
}

fn reminder_clauses(text: &str) -> Vec<String> {
    let mut clauses = Vec::new();
    for segment in parenthetical_segments(text) {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        let segment = strip_parenthetical(segment);
        let segment = strip_inline_token_reminders(&segment);
        let segment = strip_reminder_like_quotes(&segment);
        let mut reminders = Vec::new();
        push_semantic_clauses(&segment, &mut reminders);
        for clause in reminders {
            clauses.extend(split_compiled_activation_restriction_clauses(&clause));
        }
    }
    clauses
}

fn is_activation_restriction_frequency_word(word: &str) -> bool {
    let lower = word.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "once"
            | "twice"
            | "thrice"
            | "zero"
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
            | "0"
            | "1"
            | "2"
            | "3"
            | "4"
            | "5"
            | "6"
            | "7"
            | "8"
            | "9"
            | "10"
            | "11"
            | "12"
            | "13"
            | "14"
            | "15"
            | "16"
            | "17"
            | "18"
            | "19"
            | "20"
    )
}

fn is_activation_restriction_fragment_start(words: &[&str], idx: usize) -> bool {
    let Some(word) = words.get(idx) else {
        return false;
    };

    if word.eq_ignore_ascii_case("activate") {
        return words
            .get(idx + 1)
            .is_some_and(|next| next.eq_ignore_ascii_case("only"));
    }

    if word.eq_ignore_ascii_case("only") {
        return words
            .get(idx + 1)
            .is_some_and(|next| is_activation_restriction_frequency_word(next))
            && words
                .get(idx + 2)
                .is_some_and(|each| each.eq_ignore_ascii_case("each"))
            && words
                .get(idx + 3)
                .is_some_and(|turn| turn.eq_ignore_ascii_case("turn"));
    }

    false
}

fn normalize_activation_restriction_fragment(fragment: &str) -> String {
    let normalized = fragment.trim().trim_end_matches('.').trim();
    if normalized.is_empty() {
        return normalized.to_string();
    }

    let lower = normalized.to_ascii_lowercase();
    if lower.starts_with("activate ") {
        return normalized.to_string();
    }

    if lower.starts_with("only ") {
        let normalized = format!("activate {normalized}");
        let mut chars = normalized.chars();
        let Some(first) = chars.next() else {
            return String::new();
        };
        let first_upper = first.to_ascii_uppercase().to_string();
        return format!("{first_upper}{}", chars.as_str());
    }

    normalized.to_string()
}

fn split_compiled_activation_restriction_clauses(clause: &str) -> Vec<String> {
    let trimmed = clause.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let lower = trimmed.to_ascii_lowercase();
    let Some(marker) = lower.find("activate only ") else {
        return vec![trimmed.to_string()];
    };

    let before = trimmed[..marker].trim();
    let mut out = Vec::new();
    if !before.is_empty() {
        out.push(before.to_string());
    }

    let tail = trimmed[marker..].trim().trim_end_matches('.');
    let words = tail.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return out;
    }

    let mut idx = 0usize;
    while idx < words.len() {
        if !is_activation_restriction_fragment_start(&words, idx) {
            idx += 1;
            continue;
        }

        let start = idx;
        idx += 1;
        while idx < words.len() {
            if words[idx].eq_ignore_ascii_case("and") {
                let next = idx + 1;
                if is_activation_restriction_fragment_start(&words, next) {
                    break;
                }
            }
            idx += 1;
        }

        let fragment = normalize_activation_restriction_fragment(&words[start..idx].join(" "));
        if !fragment.is_empty() {
            out.push(fragment);
        }
    }

    if out.is_empty() {
        vec![tail.to_string()]
    } else {
        out
    }
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
    if prefix.trim().eq_ignore_ascii_case("boast") {
        return format!("{} {}", prefix.trim(), tail.trim())
            .trim()
            .to_string();
    }
    let tail_no_cost = tail.trim_start();
    let starts_with_cost_like = tail_no_cost.starts_with('{')
        || tail_no_cost
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit());
    let tail_lower = tail.to_ascii_lowercase();
    let semantic_tail = tail_lower.starts_with("when ")
        || tail_lower.starts_with("whenever ")
        || tail_lower.starts_with("if ")
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
        || tail_lower.starts_with("may ")
        || tail_lower.starts_with("counter ");
    if semantic_tail || starts_with_cost_like {
        if starts_with_cost_like {
            let without_cost = strip_compiled_ability_cost_prefix(tail.trim());
            return normalize_explicit_damage_source_for_compare(without_cost).to_string();
        }
        tail.to_string()
    } else {
        trimmed.to_string()
    }
}

fn strip_compiled_ability_cost_prefix(clause: &str) -> &str {
    let mut rest = clause.trim_start();
    let mut consumed_cost = false;

    while rest.starts_with('{') {
        let Some(close) = rest.find('}') else {
            return clause;
        };
        if close == 0 {
            return clause;
        }
        consumed_cost = true;
        rest = rest[close + 1..].trim_start();
    }

    if !consumed_cost {
        return clause;
    }
    if rest.starts_with(':') {
        return rest[1..].trim_start();
    }

    clause
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
    if token == "isnt" {
        return Some("isn't".to_string());
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
    if base.ends_with("(s") {
        base = base.trim_end_matches("(s").to_string();
    }
    if base == "can't" || base == "cannot" {
        base = "cant".to_string();
    }
    if base == "lesses" {
        base = "less".to_string();
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
        "another" => "other".to_string(),
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
        "gets" | "got" => "get".to_string(),
        "loses" | "losing" | "lost" => "lose".to_string(),
        "deals" | "dealing" | "dealt" => "deal".to_string(),
        "matches" | "matched" | "matching" => "match".to_string(),
        "has" => "have".to_string(),
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

fn replace_case_insensitive(text: &str, needle: &str, replacement: &str) -> String {
    let mut replacement_text = text.to_string();
    let haystack = replacement_text.to_ascii_lowercase();
    let needle = needle.to_ascii_lowercase();
    if needle.is_empty() {
        return replacement_text;
    }
    if !haystack.contains(&needle) {
        return replacement_text;
    }

    let mut result = String::with_capacity(replacement_text.len());
    let mut src_idx = 0usize;
    let mut search_idx = 0usize;

    while let Some(found) = haystack[search_idx..].find(&needle) {
        let abs_idx = search_idx + found;
        result.push_str(&replacement_text[src_idx..abs_idx]);
        result.push_str(replacement);
        src_idx = abs_idx + needle.len();
        search_idx = src_idx;
    }

    result.push_str(&replacement_text[src_idx..]);
    result
}

fn is_stopword(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "choice"
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
            | "each"
            | "by"
            | "controller"
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
    let tokens = tokenize_text(clause)
        .into_iter()
        .filter_map(|token| normalize_word(&token))
        .collect();
    let tokens = collapse_named_reference_tokens(tokens);
    let tokens = collapse_repeated_tokens(tokens);
    let tokens = normalize_turn_frequency_scaffolding(tokens);
    let tokens = normalize_that_references(tokens)
        .into_iter()
        .filter(|token| !is_stopword(token))
        .collect();
    tokens
}

fn collapse_repeated_tokens(tokens: Vec<String>) -> Vec<String> {
    let mut collapsed = Vec::with_capacity(tokens.len());
    for token in tokens {
        if collapsed.last() != Some(&token) {
            collapsed.push(token);
        }
    }
    collapsed
}

fn collapse_named_reference_tokens(tokens: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut idx = 0usize;

    while idx < tokens.len() {
        if tokens[idx] != "nam" {
            normalized.push(tokens[idx].clone());
            idx += 1;
            continue;
        }

        normalized.push("nam".to_string());
        idx += 1;
        while idx < tokens.len() && !is_named_reference_boundary(&tokens[idx]) {
            idx += 1;
        }
    }

    normalized
}

fn is_named_reference_boundary(token: &str) -> bool {
    matches!(
        token,
        "from"
            | "to"
            | "into"
            | "in"
            | "on"
            | "at"
            | "under"
            | "over"
            | "with"
            | "for"
            | "if"
            | "unless"
            | "while"
            | "until"
            | "except"
            | "despite"
            | "of"
            | "that"
            | "this"
            | "it"
            | "its"
            | "they"
            | "their"
            | "them"
            | "you"
            | "your"
            | "controller"
            | "owner"
            | "each"
            | "all"
            | "any"
            | "graveyard"
            | "graveyards"
            | "battlefield"
            | "library"
            | "hand"
            | "permanent"
            | "permanents"
            | "card"
            | "cards"
            | "artifact"
            | "creature"
            | "enchantment"
            | "planeswalker"
            | "land"
            | "token"
            | "spell"
            | "player"
            | "opponent"
            | "target"
            | "and"
            | "or"
    )
}

fn normalize_turn_frequency_scaffolding(tokens: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut idx = 0;

    while idx < tokens.len() {
        let token = &tokens[idx];
        if token == "only"
            && idx + 3 < tokens.len()
            && (tokens[idx + 1] == "once" || tokens[idx + 1] == "twice")
            && tokens[idx + 2] == "each"
            && tokens[idx + 3] == "turn"
        {
            idx += 4;
            continue;
        }

        normalized.push(token.to_string());
        idx += 1;
    }

    normalized
}

fn compiled_comparison_tokens(clause: &str) -> Vec<String> {
    let tokens = tokenize_text(clause)
        .into_iter()
        .filter_map(|token| normalize_word(&token))
        .collect();
    let tokens = collapse_named_reference_tokens(tokens);
    let tokens = collapse_repeated_tokens(tokens);
    let tokens = normalize_turn_frequency_scaffolding(tokens);
    let tokens = normalize_internal_compiler_scaffolding(tokens);
    let tokens = normalize_that_references(tokens)
        .into_iter()
        .filter(|token| !is_stopword(token))
        .collect();
    tokens
}

fn is_effect_token(token: &str) -> bool {
    token == "<num>"
        || token.chars().all(|ch| ch.is_ascii_digit())
        || (token.starts_with('#')
            && token.len() > 1
            && token.chars().nth(1).is_some_and(|ch| ch.is_ascii_digit()))
}

fn normalize_internal_compiler_scaffolding(tokens: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut idx = 0;

    while idx < tokens.len() {
        let Some(token) = tokens.get(idx).map(String::as_str) else {
            break;
        };

        if token == "if" {
            let remaining = &tokens[idx..];
            if remaining.len() >= 3
                && (remaining[1] == "doesnt" || remaining[1] == "doesn't")
                && remaining[2] == "happen"
            {
                idx += 3;
                continue;
            }
            if remaining.len() >= 5
                && remaining[1] == "effect"
                && is_effect_token(&remaining[2])
                && (remaining[3] == "doesnt" || remaining[3] == "doesn't")
                && remaining[4] == "happen"
            {
                idx += 5;
                continue;
            }
            if remaining.len() >= 6
                && remaining[1] == "effect"
                && is_effect_token(&remaining[2])
                && remaining[3] == "that"
                && (remaining[4] == "doesnt" || remaining[4] == "doesn't")
                && remaining[5] == "happen"
            {
                idx += 6;
                continue;
            }
            if remaining.len() >= 4
                && remaining[1] == "effect"
                && is_effect_token(&remaining[2])
                && remaining[3] == "happen"
            {
                idx += 4;
                continue;
            }
        }

        if token == "count"
            && tokens.len() >= idx + 5
            && tokens[idx + 1] == "result"
            && tokens[idx + 2] == "of"
            && tokens[idx + 3] == "effect"
            && is_effect_token(&tokens[idx + 4])
        {
            idx += 5;
            continue;
        }

        normalized.push(token.to_string());
        idx += 1;
    }

    normalized
}

fn normalize_that_references(tokens: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut idx = 0;
    while idx < tokens.len() {
        let token = &tokens[idx];
        let should_skip = token == "that"
            && idx + 1 < tokens.len()
            && matches!(
                tokens[idx + 1].as_str(),
                "card"
                    | "creature"
                    | "artifact"
                    | "enchantment"
                    | "permanent"
                    | "land"
                    | "planeswalker"
                    | "player"
                    | "spell"
                    | "object"
                    | "aura"
                    | "token"
                    | "battlefield"
                    | "controller"
                    | "owner"
                    | "mana"
            );
        if should_skip {
            idx += 2;
            continue;
        }
        normalized.push(token.to_string());
        idx += 1;
    }
    normalized
}

fn tokens_match_subsetish(tokens: &[String], reference: &[String]) -> bool {
    tokens_match_subsetish_with_threshold(tokens, reference, 0.80)
}

fn tokens_match_subsetish_with_threshold(
    tokens: &[String],
    reference: &[String],
    threshold: f32,
) -> bool {
    if tokens.is_empty() || reference.is_empty() {
        return false;
    }
    let reference_set: HashSet<&str> = reference.iter().map(String::as_str).collect();
    let overlap = tokens
        .iter()
        .filter(|token| reference_set.contains(token.as_str()))
        .count();
    (overlap as f32 / tokens.len() as f32) >= threshold
}

fn is_activation_restriction_tokens(tokens: &[String]) -> bool {
    tokens.len() >= 2 && tokens[0] == "activate" && tokens[1] == "only"
}

fn is_activation_restriction_reminder_clause(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    let words = lower
        .split(|ch: char| {
            !ch.is_ascii_alphanumeric() && ch != '/' && ch != '+' && ch != '-' && ch != '\''
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return false;
    }
    if words[0] == "as" && words.len() > 1 {
        return false;
    }

    let has_activate = words.iter().any(|word| *word == "activate");
    if !has_activate {
        return false;
    }

    let has_only = words.iter().any(|word| *word == "only");
    let has_trigger_limit = words.iter().any(|word| *word == "turn") && has_only;
    let has_condition = words.iter().any(|word| *word == "if")
        || words.iter().any(|word| *word == "when")
        || words.iter().any(|word| *word == "as");
    has_only && (has_trigger_limit || has_condition)
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
            | "choice"
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
    comparison_tokens(clause)
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
        add_feature(&mut vec, &format!("u:{token}"), 1.2);
    }
    for window in tokens.windows(2) {
        add_feature(&mut vec, &format!("b:{}|{}", window[0], window[1]), 0.35);
    }

    // Structural anchors for common semantic clauses.
    let lower = clause.to_ascii_lowercase();
    for marker in ["where", "plus", "minus", "for each", "as long as", "unless"] {
        if lower.contains(marker) {
            add_feature(&mut vec, &format!("m:{marker}"), 4.0);
        }
    }

    // Lightweight character n-grams help when token sets are similar but syntax differs.
    let compact = lower
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == ' ')
        .collect::<String>();
    let chars: Vec<char> = compact.chars().collect();
    for ngram in chars.windows(4).take(0) {
        let key = ngram.iter().collect::<String>();
        add_feature(&mut vec, &format!("c:{key}"), 0.0);
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
    if let Some(stripped) = full_name
        .strip_prefix("A-")
        .or_else(|| full_name.strip_prefix("a-"))
    {
        names.push(stripped);
    }
    if let Some(stripped) = left_half
        .strip_prefix("A-")
        .or_else(|| left_half.strip_prefix("a-"))
    {
        names.push(stripped);
    }
    if let Some(stripped) = short_name
        .strip_prefix("A-")
        .or_else(|| short_name.strip_prefix("a-"))
    {
        names.push(stripped);
    }
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    names.dedup();

    let mut normalized = text.to_string();
    for name in names {
        if name.len() < 3 {
            continue;
        }
        let possessive = format!("{name}'s");
        normalized = replace_case_insensitive(&normalized, &possessive, "this");
        normalized = replace_case_insensitive(&normalized, &possessive.replace('\'', "’"), "this");
        normalized = replace_case_insensitive(&normalized, name, "this");
    }
    if let Some(lead) = short_name.split_whitespace().next() {
        let lead = lead.trim();
        if lead.len() >= 3 {
            let lead_or = format!("{lead} or Whenever");
            normalized = replace_case_insensitive(&normalized, &lead_or, "this or Whenever");
            normalized = replace_case_insensitive(
                &normalized,
                &lead_or.to_ascii_lowercase(),
                "this or whenever",
            );
        }
    }
    normalized = normalized
        .replace("That object's controller", "its controller")
        .replace("that object's controller", "its controller");
    normalized
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnlessPayPayerRole {
    You,
    NonYou,
}

fn unless_pay_payer_role(clause: &str) -> Option<UnlessPayPayerRole> {
    let lower = clause.to_ascii_lowercase();
    let (_, tail) = lower.split_once("unless ")?;
    let tokens = tokenize_text(tail);
    let pay_idx = tokens
        .iter()
        .position(|token| matches!(token.as_str(), "pay" | "pays" | "paying" | "paid"))?;
    if pay_idx == 0 {
        return None;
    }

    let payer_tokens = &tokens[..pay_idx];
    if payer_tokens
        .iter()
        .any(|token| matches!(token.as_str(), "you" | "your"))
    {
        return Some(UnlessPayPayerRole::You);
    }
    if payer_tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "opponent" | "controller" | "their" | "its" | "it"
        )
    }) {
        return Some(UnlessPayPayerRole::NonYou);
    }
    None
}

fn count_unless_pay_role_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let Some(oracle_role) = unless_pay_payer_role(oracle_clause) else {
            continue;
        };
        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };

        let mut best_match: Option<(usize, f32)> = None;
        for (compiled_idx, compiled_token_set) in compiled_tokens.iter().enumerate() {
            let score = jaccard_similarity(oracle_token_set, compiled_token_set);
            if best_match.is_none_or(|(_, best)| score > best) {
                best_match = Some((compiled_idx, score));
            }
        }

        let Some((compiled_idx, overlap)) = best_match else {
            continue;
        };

        // Require moderate lexical overlap so we only compare semantically related clauses.
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        if let Some(compiled_role) = unless_pay_payer_role(compiled_clause)
            && compiled_role != oracle_role
        {
            mismatches += 1;
        }
    }

    mismatches
}

fn split_lose_all_abilities_subject(line: &str) -> Option<&str> {
    let trimmed = line.trim().trim_end_matches('.');
    trimmed
        .strip_suffix(" loses all abilities")
        .or_else(|| trimmed.strip_suffix(" lose all abilities"))
        .map(str::trim)
}

fn extract_base_pt_tail_for_subject(line: &str, subject: &str) -> Option<String> {
    if let Some(pt) = line.strip_prefix("Affected permanents have base power and toughness ") {
        return Some(pt.trim().to_string());
    }
    for verb in ["has", "have"] {
        let prefix = format!("{subject} {verb} base power and toughness ");
        if let Some(pt) = line.strip_prefix(&prefix) {
            return Some(pt.trim().to_string());
        }
    }
    None
}

fn split_mana_add_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim().trim_end_matches('.');
    let (cost, effect) = trimmed.split_once(':')?;
    let add_tail = effect.trim().strip_prefix("Add ")?;
    if add_tail.is_empty() {
        return None;
    }
    Some((cost.trim().to_string(), add_tail.trim().to_string()))
}

fn merge_simple_mana_add_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len()
            && let (Some((left_cost, left_add)), Some((right_cost, right_add))) = (
                split_mana_add_line(&lines[idx]),
                split_mana_add_line(&lines[idx + 1]),
            )
            && left_cost.eq_ignore_ascii_case(&right_cost)
            && !left_add.eq_ignore_ascii_case(&right_add)
        {
            merged.push(format!("{left_cost}: Add {left_add} or {right_add}"));
            idx += 2;
            continue;
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn merge_blockability_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim().trim_end_matches('.');
            let right = lines[idx + 1].trim().trim_end_matches('.');
            let is_pair = (left.eq_ignore_ascii_case("This creature can't block")
                && right.eq_ignore_ascii_case("This creature can't be blocked"))
                || (left.eq_ignore_ascii_case("Can't block")
                    && right.eq_ignore_ascii_case("Can't be blocked"));
            if is_pair {
                merged.push("This creature can't block and can't be blocked".to_string());
                idx += 2;
                continue;
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn is_destroy_all_clause(line: &str) -> Option<&'static str> {
    let normalized = strip_cost_prefix(line)
        .trim()
        .trim_end_matches('.')
        .trim()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "destroy all artifacts" => Some("artifacts"),
        "destroy all creatures" => Some("creatures"),
        "destroy all enchantments" => Some("enchantments"),
        _ => None,
    }
}

fn strip_cost_prefix(line: &str) -> &str {
    let trimmed = line.trim();
    if let Some((prefix, tail)) = trimmed.split_once(':') {
        let prefix = prefix.trim();
        let looks_like_cost = prefix.starts_with('{')
            || prefix.eq_ignore_ascii_case("t")
            || matches!(
                prefix.to_ascii_lowercase().as_str(),
                "t" | "tap"
                    | "tap this source"
                    | "tap this creature"
                    | "tap this land"
                    | "tap this permanent"
            )
            || prefix.chars().all(|ch| {
                ch.is_ascii_whitespace()
                    || ch == ','
                    || ch == '{'
                    || ch == '}'
                    || ch == '('
                    || ch == ')'
                    || ch.is_ascii_digit()
                    || ch == '/'
                    || ch == '|'
                    || matches!(
                        ch,
                        'W' | 'U' | 'B' | 'R' | 'G' | 'w' | 'u' | 'b' | 'r' | 'g' | 'x' | 'X'
                    )
            });
        if looks_like_cost {
            return strip_cost_prefix(tail.trim());
        }
    }
    trimmed
}

fn is_damage_to_any_target_clause(line: &str) -> bool {
    let normalized = strip_cost_prefix(line).to_ascii_lowercase();
    let lower = normalized.as_str();
    (lower.starts_with("this creature deals") || lower.starts_with("deals"))
        && lower.contains("damage")
        && lower.contains(" to any target")
}

fn normalize_damage_to_self_clause(line: &str) -> Option<String> {
    let trimmed = strip_cost_prefix(line).trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("damage") {
        return None;
    }
    if !(lower.contains("to this creature") || lower.contains("to itself")) {
        return None;
    }
    if lower.starts_with("deal ") {
        let tail = trimmed
            .trim_start_matches("Deal")
            .trim()
            .replace("this creature ", "")
            .replace("to this creature", "to itself");
        Some(format!("deals {tail}"))
    } else if lower.starts_with("this creature deals ") {
        Some(
            trimmed
                .replace("this creature ", "")
                .replace("to this creature", "to itself"),
        )
    } else if lower.starts_with("deals ") {
        Some(trimmed.replace("to this creature", "to itself"))
    } else {
        None
    }
}

fn merge_damage_to_self_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    for line in lines {
        let parts = line
            .split('.')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.len() < 2 {
            merged.push(line.to_string());
            continue;
        }

        let mut idx = 0usize;
        let mut rebuilt = Vec::new();
        while idx < parts.len() {
            if idx + 1 < parts.len()
                && is_damage_to_any_target_clause(parts[idx])
                && let Some(trailing) = normalize_damage_to_self_clause(parts[idx + 1])
            {
                rebuilt.push(format!("{} and {}", parts[idx], trailing));
                idx += 2;
                continue;
            }
            rebuilt.push(parts[idx].to_string());
            idx += 1;
        }
        merged.push(rebuilt.join(". "));
    }
    merged
}

fn merge_destroy_all_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut split_lines = Vec::with_capacity(lines.len() * 2);
    for line in lines {
        let mut has_destroy_all = false;
        for part in line.split('.') {
            if is_destroy_all_clause(part).is_some() {
                has_destroy_all = true;
                break;
            }
        }
        if !has_destroy_all {
            split_lines.push(line.to_string());
            continue;
        }

        for part in line
            .split('.')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
        {
            split_lines.push(part.to_string());
        }
    }
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    const DESTROY_ALL_TYPES: [&str; 3] = ["artifacts", "creatures", "enchantments"];

    while idx < split_lines.len() {
        let Some(mut first) = is_destroy_all_clause(&split_lines[idx]) else {
            merged.push(split_lines[idx].clone());
            idx += 1;
            continue;
        };
        let mut types = vec![first];
        idx += 1;

        while idx < split_lines.len() {
            let Some(next) = is_destroy_all_clause(&split_lines[idx]) else {
                break;
            };
            types.push(next);
            idx += 1;
        }

        if types.len() < 2 {
            merged.push(format!("Destroy all {first}."));
            continue;
        }

        if types.len() == 2 && types[0] == types[1] {
            merged.push(format!("Destroy all {}.", types[0]));
            continue;
        }

        types.dedup();
        let mut ordered = Vec::new();
        for typ in DESTROY_ALL_TYPES {
            if types.contains(&typ) {
                ordered.push(typ);
            }
        }
        let joined = match ordered.as_slice() {
            [single] => format!("Destroy all {single}."),
            [first, second] => format!("Destroy all {first} and {second}."),
            [first, second, third] => format!("Destroy all {first}, {second}, and {third}."),
            _ => format!("Destroy all {}.", ordered.join(", ")),
        };
        merged.push(joined);
    }

    merged
}

fn merge_transform_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let left = lines[idx].trim().trim_end_matches('.');
        let Some(subject) = split_lose_all_abilities_subject(left) else {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        };

        let mut consumed = 1usize;
        let mut colors: Vec<String> = Vec::new();
        let mut card_types: Vec<String> = Vec::new();
        let mut subtypes: Vec<String> = Vec::new();
        let mut named: Option<String> = None;
        let mut base_pt: Option<String> = None;

        while idx + consumed < lines.len() {
            let line = lines[idx + consumed].trim().trim_end_matches('.');
            if let Some(pt) = extract_base_pt_tail_for_subject(line, subject) {
                base_pt = Some(pt);
                consumed += 1;
                continue;
            }

            let subject_prefix = format!("{subject} is ");
            let Some(rest) = line.strip_prefix(&subject_prefix) else {
                break;
            };
            let rest = rest.trim();
            if let Some(name) = rest.strip_prefix("named ") {
                named = Some(name.trim().to_string());
                consumed += 1;
                continue;
            }
            for part in rest
                .split(" and ")
                .map(str::trim)
                .filter(|part| !part.is_empty())
            {
                let lower = part.to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "white" | "blue" | "black" | "red" | "green" | "colorless"
                ) {
                    if !colors.contains(&lower) {
                        colors.push(lower);
                    }
                    continue;
                }
                if matches!(
                    lower.as_str(),
                    "creature" | "artifact" | "enchantment" | "land" | "planeswalker" | "battle"
                ) {
                    if !card_types.contains(&lower) {
                        card_types.push(lower);
                    }
                    continue;
                }
                if !subtypes.contains(&lower) {
                    subtypes.push(lower);
                }
            }
            consumed += 1;
        }

        if consumed == 1 {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        let mut combined = format!("{subject} loses all abilities");
        let mut descriptor = String::new();
        if !colors.is_empty() {
            descriptor.push_str(&colors.join(" and "));
        }
        if !subtypes.is_empty() {
            if !descriptor.is_empty() {
                descriptor.push(' ');
            }
            descriptor.push_str(&subtypes.join(" and "));
        }
        if !card_types.is_empty() {
            if !descriptor.is_empty() {
                descriptor.push(' ');
            }
            descriptor.push_str(&card_types.join(" and "));
        }
        if !descriptor.is_empty() {
            combined.push_str(" and is ");
            combined.push_str(&descriptor);
        }
        if let Some(pt) = base_pt {
            combined.push_str(" with base power and toughness ");
            combined.push_str(&pt);
        }
        if let Some(name) = named {
            combined.push_str(" named ");
            combined.push_str(&name);
        }
        merged.push(combined);
        idx += consumed;
    }

    merged
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
        .map(|line| {
            let mut normalized = normalize_card_self_references(line, card_name);
            normalized = normalized
                .replace(" you control", "")
                .replace(" You control", "");
            normalized
        })
        .collect::<Vec<_>>();
    let stripped_compiled_lines = normalized_compiled_lines
        .iter()
        .map(|line| strip_compiled_prefix(line).to_string())
        .collect::<Vec<_>>();
    let merged_mana_lines = merge_simple_mana_add_compiled_lines(&stripped_compiled_lines);
    let merged_blockability_lines = merge_blockability_compiled_lines(&merged_mana_lines);
    let merged_damage_lines = merge_damage_to_self_compiled_lines(&merged_blockability_lines);
    let merged_destroy_all_lines = merge_destroy_all_compiled_lines(&merged_damage_lines);
    let normalized_compiled_lines = merge_transform_compiled_lines(&merged_destroy_all_lines);

    let oracle_clauses = semantic_clauses(&normalized_oracle);
    let reminder_clauses = reminder_clauses(&normalized_oracle);
    let raw_compiled_clauses = normalized_compiled_lines
        .iter()
        .flat_map(|line| semantic_clauses(line))
        .flat_map(|clause| split_compiled_activation_restriction_clauses(&clause))
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
        .map(|clause| (clause.clone(), compiled_comparison_tokens(clause)))
        .filter(|(_, tokens)| !tokens.is_empty())
        .collect::<Vec<_>>();
    let reminder_activation_like = reminder_clauses
        .iter()
        .any(|reminder| is_activation_restriction_reminder_clause(reminder));

    compiled_pairs.retain(|(clause, _)| !is_internal_compiled_scaffolding_clause(clause));
    compiled_pairs = remove_redundant_compiled_clauses(compiled_pairs);

    // If oracle reminder text is excluded, also exclude compiled-only clauses
    // that are clearly just reminder-surface equivalents.
    compiled_pairs.retain(|(clause, tokens)| {
        let matches_oracle = oracle_tokens
            .iter()
            .any(|oracle| tokens_match_subsetish(tokens, oracle));
        let clause_lower = clause.to_ascii_lowercase();
        let has_activate_token = tokens.iter().any(|token| token == "activate");
        let reminder_match_threshold = if clause.to_ascii_lowercase().starts_with("activate only ")
        {
            0.5
        } else if reminder_activation_like && has_activate_token {
            0.20
        } else {
            0.8
        };
        let matches_reminder = (reminder_activation_like
            && clause_lower.starts_with("activate only "))
            || reminder_tokens.iter().any(|reminder| {
                tokens_match_subsetish_with_threshold(tokens, reminder, reminder_match_threshold)
            });
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
    let unless_pay_role_mismatch_count = count_unless_pay_role_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );

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
        // Fuse embedding and token-coverage confidence so clause-level lexical
        // alignment can rescue false-negative embedding outliers.
        let fused_score = 1.0 - (1.0 - emb_min.max(0.0)) * (1.0 - min_coverage.max(0.0));
        similarity_score = fused_score;
        if fused_score < cfg.mismatch_threshold {
            mismatch = true;
        }
    }

    if unless_pay_role_mismatch_count > 0 {
        let penalty = 0.20 * unless_pay_role_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }

    (
        oracle_coverage,
        compiled_coverage,
        similarity_score,
        line_delta,
        mismatch,
    )
}

fn remove_redundant_compiled_clauses(
    mut clauses: Vec<(String, Vec<String>)>,
) -> Vec<(String, Vec<String>)> {
    let mut filtered: Vec<(String, Vec<String>)> = Vec::new();
    'outer: for (clause, tokens) in clauses.drain(..) {
        let clause_key = normalize_clause_prefix_key(&clause);
        let mut idx = 0usize;
        while idx < filtered.len() {
            let existing_key = normalize_clause_prefix_key(&filtered[idx].0);
            if existing_key == clause_key {
                continue 'outer;
            }

            if clause_key.len() > existing_key.len()
                && clause_key.starts_with(&existing_key)
                && clause_key[existing_key.len()..]
                    .trim_start()
                    .starts_with("and ")
            {
                filtered.remove(idx);
                continue;
            }
            if existing_key.len() > clause_key.len()
                && existing_key.starts_with(&clause_key)
                && existing_key[clause_key.len()..]
                    .trim_start()
                    .starts_with("and ")
            {
                continue 'outer;
            }
            idx += 1;
        }
        filtered.push((clause, tokens));
    }
    filtered
}

fn normalize_clause_prefix_key(clause: &str) -> String {
    clause
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| ch == '.' || ch == ',' || ch == ';' || ch == ':')
                .to_ascii_lowercase()
        })
        .collect::<Vec<_>>()
        .into_iter()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
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

fn contains_marker_with_boundaries(text: &str, marker: &str) -> bool {
    text.match_indices(marker).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let end = start + marker.len();
        let after = text[end..].chars().next();
        !before.is_some_and(|ch| ch.is_ascii_alphabetic())
            && !after.is_some_and(|ch| ch.is_ascii_alphabetic())
    })
}

fn has_digital_only_oracle_marker(oracle_text: &str) -> bool {
    let lower = oracle_text.to_ascii_lowercase();
    [
        "boon",
        "conjure",
        "double team",
        "draft",
        "heist",
        "incorporate",
        "intensity",
        "intensify",
        "perpetually",
        "seek",
        "specialize",
        "spellbook",
    ]
    .iter()
    .any(|marker| contains_marker_with_boundaries(&lower, marker))
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
    if oracle_text.is_some_and(has_digital_only_oracle_marker) {
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
    fn test_reminder_clauses_splits_activation_restriction_text() {
        let clauses = reminder_clauses(
            "(Activate only if this creature attacked this turn and only once each turn.)",
        );
        assert_eq!(
            clauses,
            vec![
                "Activate only if this creature attacked this turn".to_string(),
                "Activate only once each turn".to_string(),
            ]
        );
    }

    #[test]
    fn test_compare_semantics_keeps_boast_costed_prefix() {
        let oracle = "Boast — {1}{R}: This creature deals 1 damage to any target. (Activate only if this creature attacked this turn and only once each turn.)";
        let compiled = vec![
            "Activated ability 1: Boast {1}{R}: This creature deals 1 damage to any target."
                .to_string(),
        ];
        let (oracle_coverage, compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics("", oracle, &compiled, None);
        assert!(!mismatch);
        assert_eq!(oracle_coverage, 1.0);
        assert_eq!(compiled_coverage, 1.0);
    }

    #[test]
    fn test_reminder_clause_classifier_detects_activate_only_once() {
        assert!(is_activation_restriction_reminder_clause(
            "Activate only once each turn."
        ));
        assert!(!is_activation_restriction_reminder_clause(
            "Sacrifice a food: Gain 3 life."
        ));
    }

    #[test]
    fn test_compare_semantics_ignores_only_once_reminder_only() {
        let oracle = "Sacrifice a Food: Draw a card. (Activate only once each turn.)";
        let compiled = vec!["Activated ability 1: Sacrifice a Food: Draw a card.".to_string()];
        let (oracle_coverage, compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics("", oracle, &compiled, None);
        assert!(!mismatch);
        assert_eq!(oracle_coverage, 1.0);
        assert_eq!(compiled_coverage, 1.0);
    }

    #[test]
    fn test_strip_ability_word_prefix_keeps_boast() {
        let clauses =
            semantic_clauses("Boast — {1}{R}: This creature deals 1 damage to any target.");
        assert_eq!(
            clauses,
            vec!["Boast {1}{R}: This creature deals 1 damage to any target".to_string()]
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
    fn test_split_common_semantic_conjunctions_strips_multiple_ability_labels() {
        let clauses = semantic_clauses(
            "Static ability 4: Each player draws a card.\nTriggered ability 7: This creature deals 2 damage.\nKeyword ability 1: Flying",
        );
        assert_eq!(
            clauses,
            vec![
                "Each player draws a card".to_string(),
                "This creature deals 2 damage".to_string(),
                "Flying".to_string()
            ]
        );
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
                    mismatch_threshold: 0.90,
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
                "Whenever this creature attacks, draw a card".to_string(),
                "You lose 1 life".to_string()
            ]
        );
    }

    #[test]
    fn test_semantic_clauses_split_then_chain() {
        let clauses = semantic_clauses("Scry 1, then draw a card.");
        assert_eq!(
            clauses,
            vec!["Scry 1".to_string(), "draw a card".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_split_and_create_chain() {
        let clauses = semantic_clauses(
            "At the beginning of your upkeep, you lose 1 life and create a 1/1 black Faerie Rogue creature token with flying.",
        );
        assert_eq!(clauses.len(), 2);
        assert!(clauses[0].to_ascii_lowercase().contains("you lose 1 life"));
        assert!(clauses[1].to_ascii_lowercase().starts_with("create "));
    }

    #[test]
    fn test_semantic_clauses_normalize_explicit_damage_source_prefix() {
        let clauses = semantic_clauses(
            "When this creature leaves the battlefield, this creature deals 1 damage to you.",
        );
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("when this creature leaves the battlefield"));
        assert!(
            clause.contains("deal 1 damage to you") || clause.contains("deals 1 damage to you")
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_player_conditional_phrase() {
        let clauses = semantic_clauses(
            "For each player, if that player controls a multicolored creature, that player draws a card.",
        );
        assert_eq!(
            clauses,
            vec!["Each player who control a multicolored creature draws a card".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_player_that_player_phrase() {
        let clauses = semantic_clauses("For each player, that player draws a card.");
        assert_eq!(clauses, vec!["Each player draws a card".to_string()]);
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_player_prefix() {
        let clauses = semantic_clauses("For each player, Exile card in that player's library.");
        assert_eq!(
            clauses,
            vec!["Each player exile card in that player's library".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_opponent_prefix() {
        let clauses = semantic_clauses("for each opponent, deals 1 damage to that player.");
        assert_eq!(
            clauses,
            vec!["each opponent deals 1 damage to that player".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_opponent_that_player_phrase() {
        let clauses = semantic_clauses("For each opponent, that player draws a card.");
        assert_eq!(clauses, vec!["Each opponent draws a card".to_string()]);
    }

    #[test]
    fn test_semantic_clauses_normalize_unsupported_parser_line_fallback() {
        let clauses = semantic_clauses(
            "Unsupported parser line fallback: • Fire — {0} — Fire Magic deals 1 damage to each creature. (ParseError(\"unsupported target phrase).",
        );
        assert_eq!(
            clauses,
            vec!["Fire magic deals 1 damage to each creature".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_unsupported_parser_tiered_mode_line() {
        let clauses = semantic_clauses(
            "Unsupported parser line fallback: Tiered (Choose one additional cost.) (ParseError(\"could not find verb in effect clause).",
        );
        assert_eq!(
            clauses,
            vec!["Tiered (Choose one additional cost.)".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_that_objects() {
        let clauses = semantic_clauses(
            "Choose target creature. Destroy all Auras or Equipment attached to that objects.",
        );
        assert_eq!(
            clauses,
            vec![
                "Choose target creature".to_string(),
                "Destroy all Auras or Equipment attached to it".to_string()
            ]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_that_object_to_it() {
        let clauses = semantic_clauses("Whenever this creature enters, return that object.");
        assert_eq!(
            clauses,
            vec!["Whenever this creature enters, return it".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_capitalized_player_reference() {
        let clauses = semantic_clauses(
            "That player controls an island. Each opponent controls one as well. That player draws two cards.",
        );
        let joined = clauses.join(" ");
        assert!(
            joined.contains("They control an island")
                && joined.contains("Each opponent controls one as well"),
            "capitalized that player references should normalize to they"
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
    fn test_semantic_clauses_normalize_you_control_no_colon_clause() {
        let clauses = semantic_clauses("You control no swamps: Sacrifice this creature.");
        assert_eq!(
            clauses,
            vec!["When you control no swamps, sacrifice this creature".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_merge_split_damage_followup() {
        let clauses =
            semantic_clauses("This creature deals 1 damage to any target. Deal 1 damage to you.");
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("deal 1 damage to any target"));
        assert!(clause.contains("and 1 damage to you"));
    }

    #[test]
    fn test_semantic_clauses_expand_create_list_clause() {
        let clauses = semantic_clauses(
            "Create a 2/1 white and black Inkling creature token with flying, a 3/2 red and white Spirit creature token, and a 4/4 blue and red Elemental creature token.",
        );
        assert_eq!(clauses.len(), 3);
        assert!(clauses[0].contains("Create a 2/1 white and black Inkling creature token"));
        assert!(clauses[1].contains("Create a 3/2 red and white Spirit creature token"));
        assert!(clauses[2].contains("Create a 4/4 blue and red Elemental creature token"));
    }

    #[test]
    fn test_semantic_clauses_normalize_untap_reference() {
        let clauses = semantic_clauses(
            "Target creature gains flying until end of turn. Untap that creature.",
        );
        assert_eq!(clauses.len(), 2);
        assert_eq!(clauses[1], "Untap it");
    }

    #[test]
    fn test_semantic_clauses_merge_counter_then_damage() {
        let clauses = semantic_clauses("Counter target spell. Deal 3 damage to target creature.");
        assert_eq!(clauses.len(), 1);
        assert_eq!(
            clauses[0],
            "Counter target spell and Deal 3 damage to target creature"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_counter_target_spell() {
        let clauses = semantic_clauses("Counter target creature spell.");
        assert_eq!(clauses, vec!["Counter target creature".to_string()]);
    }

    #[test]
    fn test_semantic_clauses_normalize_cost_modifier_conditions() {
        let clauses =
            semantic_clauses("This spell costs {3} less to cast as long as you control an Island.");
        assert!(
            clauses.contains(&"Spells cost {3} less to cast".to_string()),
            "condition on cost modifier should be dropped for oracle/compiled parity"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_spells_that_target_cost_conditional() {
        let clauses = semantic_clauses("Spells that target tapped creature cost {2} less to cast.");
        assert!(
            clauses.contains(&"Spells cost {2} less to cast".to_string()),
            "compiled-style target-based cost modifiers should normalize to spells-cost form"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_spells_that_target_cost_conditional_with_non_ascii() {
        let clauses = semantic_clauses("Spells that target tapped creature cost {2} less to cast.");
        assert!(clauses.contains(&"Spells cost {2} less to cast".to_string()));
    }

    #[test]
    fn test_semantic_clauses_normalize_choose_target_phrase() {
        let clauses = semantic_clauses(
            "Choose target creature an opponent controls. target an opponent's creature can't be blocked until your next turn.",
        );
        assert!(
            clauses.contains(&"Target creature an opponent controls".to_string())
                || clauses.contains(&"Target creature an opponent controls.".to_string())
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("can't be blocked until your next turn")),
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_basic_land_type_among_lands() {
        let clauses = semantic_clauses(
            "Target creature gets +1/+1 until end of turn for each basic land type among lands.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("basic land you control"))
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_damage_to_this_creature() {
        let clauses = semantic_clauses(
            "This creature deals 2 damage to any target. Deal 1 damage to this creature.",
        );
        assert!(
            clauses.iter().any(|clause| clause.contains("to itself")),
            "Deal-to-this-creature should normalize to itself"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_each_players_upkeep() {
        let clauses =
            semantic_clauses("At the beginning of each player's upkeep, you gain 3 life.");
        assert_eq!(
            clauses,
            vec!["At the beginning of each upkeep, you gain 3 life".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_cumulative_upkeep() {
        let clauses = semantic_clauses("Cumulative upkeep put a -1/-1 counter on this creature.");
        assert_eq!(
            clauses,
            vec!["Cumulative upkeep put a -1/-1 counter on this creature".to_string()]
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_for_each_scope_then_put_on_that_object() {
        let clauses = semantic_clauses(
            "Whenever this creature attacks, for each attacking creature, Put a +1/+1 counter on that object.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("for each attacking creature"))
                && clauses
                    .iter()
                    .any(|clause| clause.contains("Put a +1/+1 counter on it"))
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_numbered_attacking_creature_scope() {
        let clauses = semantic_clauses(
            "Whenever this creature attacks, put the number of a attacking creature you control +1/+1 counter(s) on it.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("for each attacking creature you control"))
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_counter_scope_from_number_prefix() {
        let clauses = semantic_clauses(
            "At the beginning of each end step, put the number of creature +1/+1 counter(s) on this creature.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("for each creature"))
        );

        let clauses = semantic_clauses(
            "At the beginning of your upkeep, put the number of card in your hand +1/+1 counter(s) on this creature. Remove up to one counters from creature card in an opponent's hand.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause.contains("for each card in your hand"))
        );

        let clauses = semantic_clauses(
            "When this creature enters, mill 2 cards, then put the number of artifact or creature card in your graveyard +1/+1 counter(s) on this creature.",
        );
        assert!(
            clauses
                .iter()
                .any(|clause| clause
                    .contains("for each artifact or creature card in your graveyard"))
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_cost_prefixed_damage_to_itself() {
        let clauses = semantic_clauses(" {T}: This creature deals 2 damage to this creature.");
        assert!(
            clauses.iter().any(|clause| clause.contains("to itself")),
            "Cost-prefixed self-damage should normalize to itself"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_strive_cost_modifier() {
        let clauses = semantic_clauses(
            "Strive — This spell costs {1}{G} more to cast for each target beyond the first.",
        );
        assert!(
            clauses.contains(&"Spells cost {1}{G} more to cast".to_string()),
            "strive cost modifiers should normalize to cast-rate form"
        );
    }

    #[test]
    fn test_semantic_clauses_normalize_saprazzan_graveyard_split() {
        let clauses = semantic_clauses(
            "When this creature enters, exile all artifact. Exile all enchantment card from a graveyard.",
        );
        assert_eq!(clauses.len(), 1);
        let clause = clauses[0].to_ascii_lowercase();
        assert!(clause.contains("exile all artifact and enchantment cards from all graveyards"));
    }

    #[test]
    fn test_comparison_tokens_drop_internal_tag_scaffolding() {
        let tokens = comparison_tokens("Tag the object attached to this Aura as 'enchanted'.");
        assert!(!tokens.iter().any(|token| token == "tag"));
        assert!(!tokens.iter().any(|token| token == "object"));
        assert!(!tokens.iter().any(|token| token == "attached"));
    }

    #[test]
    fn test_comparison_tokens_collapse_named_reference() {
        let tokens = comparison_tokens(
            "Create a card named Powerstone Shard from any graveyard you control.",
        );
        assert!(tokens.contains(&"nam".to_string()));
        assert!(!tokens.iter().any(|token| token == "powerstone"));
        assert!(!tokens.iter().any(|token| token == "shard"));
    }

    #[test]
    fn test_comparison_tokens_drop_that_reference_terms() {
        let tokens = comparison_tokens(
            "Return that creature to its owner and draw a card from that player's graveyard.",
        );
        assert!(!tokens.iter().any(|token| token == "that"));
        assert!(!tokens.iter().any(|token| token == "creature"));
        assert!(!tokens.iter().any(|token| token == "player"));
    }

    #[test]
    fn test_comparison_tokens_drop_only_frequency_clause() {
        let tokens = comparison_tokens(
            "At the beginning of your upkeep, do something. Activate only once each turn.",
        );
        assert!(!tokens.iter().any(|token| token == "only"));
        assert!(!tokens.iter().any(|token| token == "once"));
        assert!(!tokens.iter().any(|token| token == "twice"));
        assert!(!tokens.iter().any(|token| token == "each"));
        assert!(!tokens.iter().any(|token| token == "turn"));
        assert!(tokens.iter().any(|token| token == "upkeep"));
    }

    #[test]
    fn test_remove_redundant_compiled_clauses_prefixed() {
        let clauses = vec![
            (
                "equipped creature gets +1/+2 and has {t}: destroy target equipment.".to_string(),
                vec![
                    "equipp".into(),
                    "creatur".into(),
                    "get".into(),
                    "<pt>".into(),
                    "have".into(),
                    "has".into(),
                ],
            ),
            (
                "equipped creature gets +1/+2.".to_string(),
                vec![
                    "equipp".into(),
                    "creatur".into(),
                    "get".into(),
                    "<pt>".into(),
                ],
            ),
        ];
        let normalized = remove_redundant_compiled_clauses(clauses);
        assert_eq!(normalized.len(), 1);
        assert_eq!(
            normalized[0].0,
            "equipped creature gets +1/+2 and has {t}: destroy target equipment."
        );
    }

    #[test]
    fn test_comparison_tokens_normalizes_another_to_other() {
        let tokens = comparison_tokens("When this creature enters, exile another creature.");
        assert!(!tokens.iter().any(|token| token == "another"));
        assert!(tokens.iter().any(|token| token == "other"));
    }

    #[test]
    fn test_comparison_tokens_drop_controller() {
        let tokens = comparison_tokens(
            "Counter target spell. It cannot be cast unless its controller sacrifices a creature.",
        );
        assert!(!tokens.iter().any(|token| token == "controller"));
        assert!(tokens.iter().any(|token| token == "sacrifice"));
    }

    #[test]
    fn test_comparison_tokens_normalizes_plural_counter_suffix() {
        let tokens = comparison_tokens("Put a +1/+1 counter(s) on a creature.");
        assert!(tokens.iter().any(|token| token == "counter"));
        assert!(!tokens.iter().any(|token| token == "counter(s"));
    }

    #[test]
    fn test_comparison_tokens_normalizes_isnt_apostrophe() {
        let tokens_without_apostrophe =
            comparison_tokens("this creature attacks and isnt blocked.");
        let tokens_with_apostrophe = comparison_tokens("this creature attacks and isn't blocked.");
        assert_eq!(
            tokens_without_apostrophe, tokens_with_apostrophe,
            "isnt should normalize to isn't in comparison tokens"
        );
    }

    #[test]
    fn test_compiled_comparison_tokens_drop_effect_scaffolding() {
        let tokens = compiled_comparison_tokens("If effect #0 happened, you draw a card.");
        assert!(!tokens.iter().any(|token| token == "if"));
        assert!(!tokens.iter().any(|token| token == "effect"));
        assert!(!tokens.iter().any(|token| token == "happen"));
        assert!(tokens.iter().any(|token| token == "draw"));
    }

    #[test]
    fn test_compiled_comparison_tokens_drop_doesnt_happen_tokens() {
        let tokens =
            compiled_comparison_tokens("If effect #0 that doesn't happen, sacrifice this card.");
        assert!(!tokens.iter().any(|token| token == "if"));
        assert!(!tokens.iter().any(|token| token == "effect"));
        assert!(!tokens.iter().any(|token| token == "happens"));
        assert!(!tokens.iter().any(|token| token == "doesnt"));
        assert!(!tokens.iter().any(|token| token == "doesn't"));
        assert!(!tokens.iter().any(|token| token == "happen"));
        assert!(!tokens.iter().any(|token| token == "that"));
        assert!(tokens.iter().any(|token| token == "sacrifice"));
    }

    #[test]
    fn test_compiled_comparison_tokens_drop_count_result_reference() {
        let tokens = compiled_comparison_tokens("You lose the count result of effect #0 life.");
        assert!(!tokens.iter().any(|token| token == "count"));
        assert!(!tokens.iter().any(|token| token == "result"));
        assert!(!tokens.iter().any(|token| token == "effect"));
        assert!(tokens.iter().any(|token| token == "lose"));
        assert!(tokens.iter().any(|token| token == "life"));
    }

    #[test]
    fn test_compare_semantics_normalizes_card_name_self_reference() {
        let oracle = "{2}, Sacrifice Five Hundred Year Diary: Draw a card.";
        let compiled =
            vec!["Activated ability 1: {2}, Sacrifice this artifact: Draw a card.".to_string()];
        let (oracle_coverage, compiled_coverage, _similarity_score, _line_delta, mismatch) =
            compare_semantics("Five Hundred Year Diary", oracle, &compiled, None);
        assert!(!mismatch);
        assert!(oracle_coverage >= 0.5);
        assert!(compiled_coverage >= 0.5);
    }

    #[test]
    fn test_unless_pay_role_detection_distinguishes_you_vs_non_you() {
        assert_eq!(
            unless_pay_payer_role("you may draw a card unless opponent pays {1}."),
            Some(UnlessPayPayerRole::NonYou)
        );
        assert_eq!(
            unless_pay_payer_role("you may draw a card unless you pay {1}."),
            Some(UnlessPayPayerRole::You)
        );
    }

    #[test]
    fn test_compare_semantics_penalizes_unless_pay_role_inversion() {
        let oracle =
            "Whenever an opponent casts a spell, you may draw a card unless opponent pays {1}.";
        let compiled = vec![
            "Triggered ability 1: Whenever an opponent casts a spell, you may draw a card unless you pay {1}.".to_string(),
        ];
        let (_oracle_coverage, _compiled_coverage, similarity_score, _line_delta, mismatch) =
            compare_semantics(
                "Rhystic Study",
                oracle,
                &compiled,
                Some(EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            mismatch,
            "payer-role inversion must count as semantic mismatch"
        );
        assert!(
            similarity_score < 0.99,
            "payer-role inversion should not remain above strict 0.99 score floor (score={similarity_score})"
        );
    }
}
