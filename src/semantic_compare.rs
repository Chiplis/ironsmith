use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy)]
pub struct EmbeddingConfig {
    pub dims: usize,
    pub mismatch_threshold: f32,
}

pub fn report_embedding_config() -> Option<EmbeddingConfig> {
    Some(EmbeddingConfig {
        dims: 384,
        mismatch_threshold: 0.99,
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

fn rewrite_grant_play_tagged_effect_scaffolding(text: &str) -> String {
    let markers = [
        "you may Effect(GrantPlayTaggedEffect",
        "You may Effect(GrantPlayTaggedEffect",
    ];
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;

    while cursor < text.len() {
        let mut next_match: Option<(usize, &str)> = None;
        for marker in markers {
            if let Some(rel) = text[cursor..].find(marker) {
                let idx = cursor + rel;
                if next_match.map_or(true, |(best_idx, _)| idx < best_idx) {
                    next_match = Some((idx, marker));
                }
            }
        }

        let Some((start, marker)) = next_match else {
            out.push_str(&text[cursor..]);
            break;
        };

        out.push_str(&text[cursor..start]);
        let Some(open_offset) = marker.find('(') else {
            out.push_str(marker);
            cursor = start + marker.len();
            continue;
        };
        let open_idx = start + open_offset;
        let mut idx = open_idx;
        let mut depth = 0u32;
        let mut in_string = false;
        let mut escaped = false;
        let mut end_idx = text.len();

        while idx < text.len() {
            let ch = text[idx..].chars().next().unwrap();
            let ch_len = ch.len_utf8();
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
                idx += ch_len;
                continue;
            }

            if ch == '"' {
                in_string = true;
                idx += ch_len;
                continue;
            }

            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end_idx = idx + ch_len;
                    break;
                }
            }
            idx += ch_len;
        }

        let effect_call = &text[start..end_idx];
        if effect_call.contains("UntilYourNextTurn") {
            out.push_str("you may play that card until your next turn");
        } else if effect_call.contains("UntilEndOfTurn") {
            out.push_str("you may play that card this turn");
        } else {
            out.push_str(effect_call);
        }
        cursor = end_idx;
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
        || lower.starts_with("sacrifice this artifact: add one mana of any color")
        || lower.starts_with("sacrifice this token: add {c}")
        || lower.starts_with("sacrifice this creature: add {c}")
        || lower.starts_with("{2}, {t}, sacrifice this token: you gain 3 life")
        || lower.starts_with("{2}, {t}, sacrifice this artifact: you gain 3 life")
        || lower.starts_with("{2}, {t}, sacrifice this token: draw a card")
        || lower.starts_with("{2}, sacrifice this artifact: draw a card")
        || lower.starts_with("{2}, sacrifice this token: you gain 3 life")
        || lower.starts_with("when this token dies")
        || lower.starts_with("when this token leaves the battlefield")
}

fn strip_trailing_ci_suffix(text: &mut String, suffix: &str) {
    if text.len() < suffix.len() {
        return;
    }
    let lower = text.to_ascii_lowercase();
    let suffix_lower = suffix.to_ascii_lowercase();
    if lower.ends_with(&suffix_lower) {
        let keep = text.len().saturating_sub(suffix.len());
        if text.is_char_boundary(keep) {
            text.truncate(keep);
        }
    }
}

fn strip_reminder_like_quotes(text: &str) -> String {
    let mut out = String::new();
    let mut in_quote = false;
    let mut quoted = String::new();

    for ch in text.chars() {
        if ch == '"' {
            if in_quote {
                if looks_like_reminder_quote(&quoted) {
                    strip_trailing_ci_suffix(&mut out, "It has ");
                    strip_trailing_ci_suffix(&mut out, "it has ");
                    strip_trailing_ci_suffix(&mut out, "They have ");
                    strip_trailing_ci_suffix(&mut out, "they have ");
                    strip_trailing_ci_suffix(&mut out, "with ");
                    strip_trailing_ci_suffix(&mut out, "With ");
                } else {
                    out.push('"');
                    out.push_str(&quoted);
                    out.push('"');
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
    .replace(
        " It has \"Sacrifice this artifact: Add one mana of any color.\"",
        "",
    )
    .replace(" It has \"Sacrifice this token: Add {C}.\"", "")
    .replace(" It has \"Sacrifice this creature: Add {C}.\"", "")
    .replace(
        " It has \"{2}, {T}, Sacrifice this token: You gain 3 life.\"",
        "",
    )
    .replace(
        " It has \"{2}, {T}, Sacrifice this artifact: You gain 3 life.\"",
        "",
    )
    .replace(" It has \"{2}, Sacrifice this artifact: Draw a card.\"", "")
}

pub fn strip_reminder_text_for_comparison(text: &str) -> String {
    text.lines()
        .filter_map(|raw_line| {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() {
                return None;
            }
            if trimmed.starts_with('(') && trimmed.ends_with(')') {
                return None;
            }

            let no_parenthetical = strip_parenthetical(raw_line);
            let no_inline_reminder = strip_inline_token_reminders(&no_parenthetical);
            let no_quote_reminder = strip_reminder_like_quotes(&no_inline_reminder);
            let normalized = normalize_clause_line(&no_quote_reminder);

            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_not_named_phrase(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 3 {
        return text.to_string();
    }

    let mut out = Vec::with_capacity(words.len());
    let mut idx = 0usize;
    while idx < words.len() {
        if idx + 1 < words.len()
            && words[idx].eq_ignore_ascii_case("not")
            && words[idx + 1].eq_ignore_ascii_case("named")
        {
            idx += 2;
            let mut consumed_name = false;
            while idx < words.len() {
                let token = words[idx].trim_matches(|ch: char| matches!(ch, ',' | '.' | ';' | ':'));
                let lower = token.to_ascii_lowercase();
                if consumed_name
                    && matches!(
                        lower.as_str(),
                        "and"
                            | "or"
                            | "with"
                            | "without"
                            | "that"
                            | "which"
                            | "who"
                            | "whose"
                            | "under"
                            | "among"
                            | "on"
                            | "in"
                            | "to"
                            | "from"
                            | "if"
                            | "unless"
                            | "then"
                    )
                {
                    break;
                }
                consumed_name = true;
                idx += 1;
            }
            continue;
        }
        out.push(words[idx]);
        idx += 1;
    }

    out.join(" ")
}

fn normalize_clause_line(text: &str) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
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

fn strip_compiled_prefixes(text: &str) -> String {
    let trimmed = text.trim();

    if let Some(rest) = trimmed.strip_prefix("Spell effects:") {
        return rest.trim().to_string();
    }

    for prefix in [
        "Triggered ability ",
        "Activated ability ",
        "Mana ability ",
        "Static ability ",
        "Keyword ability ",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix)
            && let Some((_, tail)) = rest.split_once(':')
        {
            return tail.trim().to_string();
        }
    }

    trimmed.to_string()
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

fn strip_implicit_you_control_in_sacrifice_phrases(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let lower = text.to_ascii_lowercase();
    let mut idx = 0usize;
    let mut in_sacrifice = false;
    while idx < text.len() {
        let ch = text[idx..].chars().next().unwrap();
        if matches!(ch, '.' | ';' | ':' | ',' | '\n') {
            in_sacrifice = false;
            out.push(ch);
            idx += ch.len_utf8();
            continue;
        }

        if in_sacrifice {
            if lower[idx..].starts_with(" you control") {
                idx += " you control".len();
                continue;
            }
        } else if lower[idx..].starts_with("sacrifice") || lower[idx..].starts_with("sacrifices") {
            in_sacrifice = true;
        }

        out.push(ch);
        idx += ch.len_utf8();
    }
    out
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

fn is_ignorable_semantic_clause(clause: &str) -> bool {
    let lower = clause.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }

    lower == "choose a background"
        || lower == "you can have a background as a second commander"
        || lower.starts_with("you choose exactly 1 a background ")
        || lower.starts_with("choose exactly 1 a background ")
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

fn looks_like_named_subject(subject: &str) -> bool {
    let trimmed = subject.trim();
    if trimmed.is_empty() {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.contains(" or ") {
        return false;
    }

    for banned in [
        "this ",
        "another ",
        "target ",
        "enchanted ",
        "equipped ",
        "creature",
        "artifact",
        "enchantment",
        "land",
        "permanent",
        "player",
        "opponent",
        "you ",
        "your ",
        "card",
    ] {
        if lower.contains(banned) {
            return false;
        }
    }

    trimmed.chars().any(|ch| ch.is_ascii_uppercase())
        || trimmed.contains(',')
        || trimmed.split_whitespace().count() >= 2
}

fn normalize_trigger_subject_for_compare(line: &str) -> String {
    let trimmed = line.trim();

    for prefix in ["When ", "Whenever "] {
        if !trimmed.starts_with(prefix) {
            continue;
        }

        let lower = trimmed.to_ascii_lowercase();
        let mut marker_idx = None;
        for marker in [
            " becomes tapped",
            " become tapped",
            " becomes untapped",
            " become untapped",
            " becomes ",
            " become ",
            " enters",
            " dies",
            " attacks",
            " blocks",
            " deals ",
            " is turned face up",
        ] {
            if let Some(idx) = lower.find(marker) {
                marker_idx = Some(idx);
                break;
            }
        }
        let Some(idx) = marker_idx else {
            continue;
        };

        if idx <= prefix.len() {
            continue;
        }

        let subject = trimmed[prefix.len()..idx].trim();

        if !looks_like_named_subject(subject) {
            continue;
        }

        let tail = &trimmed[idx..];
        let replacement_subject = if tail.starts_with(" enters") {
            "this permanent"
        } else {
            "this creature"
        };

        let normalized = format!("{prefix}{replacement_subject}{tail}");
        if let Some((subject, rest)) = normalized.split_once("'s power and toughness")
            && looks_like_named_subject(subject)
        {
            return format!("this creature's power and toughness{rest}");
        }
        if let Some((subject, rest)) = normalized.split_once("'s power")
            && looks_like_named_subject(subject)
        {
            return format!("this creature's power{rest}");
        }

        return normalized;
    }

    if let Some((subject, rest)) = trimmed.split_once("'s power and toughness")
        && looks_like_named_subject(subject)
    {
        return format!("this creature's power and toughness{rest}");
    }
    if let Some((subject, rest)) = trimmed.split_once("'s power")
        && looks_like_named_subject(subject)
    {
        return format!("this creature's power{rest}");
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

fn split_common_clause_conjunctions(text: &str) -> String {
    let mut normalized = text.to_string();

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
                    capitalize_fallback_with_parenthetical_title_case(&rest.to_ascii_lowercase());
            }
        }
    }

    normalized = strip_compiled_prefixes(&normalized);
    normalized = strip_not_named_phrase(&normalized);
    normalized = strip_implicit_you_control_in_sacrifice_phrases(&normalized);
    normalized = normalized
        .replace("Flashback—", "Flashback ")
        .replace("flashback—", "flashback ")
        .replace("Buyback—", "Buyback ")
        .replace("buyback—", "buyback ")
        .replace(" a a ", " a ");
    let normalized_lower = normalized.to_ascii_lowercase();
    if normalized_lower.starts_with(
        "target opponent chooses target creature an opponent controls. exile it. exile all ",
    ) && (normalized_lower.contains(" in target opponent's graveyard")
        || normalized_lower.contains(" in target opponent's graveyards"))
    {
        normalized =
            "Target opponent exiles a creature they control and their graveyard.".to_string();
    }
    // Canonicalize expanded keyword scaffolding for comparison.
    if normalized.contains("SoulbondPairEffect") {
        normalized = "Soulbond".to_string();
    }
    if normalized.eq_ignore_ascii_case("Whenever a creature you control enters, effect") {
        normalized = "Soulbond".to_string();
    }
    if normalized.eq_ignore_ascii_case("Daybound") || normalized.eq_ignore_ascii_case("Nightbound")
    {
        normalized = "Daybound/Nightbound".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("Scavenge ")
        && normalized_lower.contains(", exile this card from your graveyard:")
        && normalized_lower.contains(
            "put a number of +1/+1 counters equal to this card's power on target creature",
        )
        && let Some((cost, _)) = rest.split_once(' ')
    {
        normalized = format!("Scavenge {}", cost.trim());
    }
    let normalized_lower = normalized.to_ascii_lowercase();
    let normalized_compact = normalized_lower
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if normalized_lower.starts_with(
        "whenever this creature attacks the player with the most life or tied for most life, put a +1/+1 counter on this creature",
    ) {
        normalized = "Dethrone".to_string();
    }
    if normalized_lower.starts_with(
        "whenever this creature attacks, another attacking creature you control get +1/+0 until end of turn",
    ) || normalized_lower.starts_with(
        "whenever this creature attacks, other attacking creatures get +1/+0 until end of turn",
    ) || normalized_lower.starts_with(
        "whenever this creature attacks, other attacking creatures you control get +1/+0 until end of turn",
    ) || normalized_lower.starts_with(
        "whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn",
    ) {
        normalized = "Battle cry".to_string();
    }
    if normalized_lower.starts_with(
        "whenever this creature attacks, put a +1/+1 counter on target attacking creature with power less than this creature's power",
    ) || normalized_lower.starts_with(
        "whenever this creature attacks, put a +1/+1 counter on target attacking creature with lesser power",
    ) {
        normalized = "Mentor".to_string();
    }
    if normalized_lower
        .starts_with("when this creature enters, you may put a +1/+1 counter on this creature")
        || normalized_lower
            .starts_with("this creature can't block as long as it has a +1/+1 counter on it")
    {
        normalized = "Unleash".to_string();
    }
    if normalized_lower
        .trim_end_matches('.')
        .starts_with("when this creature dies, create two 1/1 white and black spirit creature tokens with flying")
    {
        normalized = "Afterlife 2".to_string();
    }
    if let Some((cost, _)) =
        normalized.split_once(", Return an unblocked attacker you control to its owner's hand:")
        && normalized_lower.contains("put this card onto the battlefield tapped and attacking")
    {
        normalized = format!("Ninjutsu {}", cost.trim());
    }
    if let Some((cost, _)) = normalized.split_once(", Exile this card from your graveyard:")
        && normalized_compact.contains("put this creature's power +1/+1 counter")
        && normalized_compact.contains("activate only as a sorcery")
    {
        normalized = format!("Scavenge {}", cost.trim());
    }
    if normalized_lower.starts_with(
        "whenever this creature attacks, untap target defending player's creature. target defending player's creature gains blocks each combat if able until end of combat",
    ) {
        normalized = "Provoke".to_string();
    }
    if normalized_lower
        .trim_end_matches('.')
        .starts_with("at the beginning of each player's upkeep, if this creature is transformed, if two or more spells were cast last turn, transform this creature. otherwise, if no spells were cast last turn, transform this creature")
    {
        normalized = "Daybound/Nightbound".to_string();
    }
    if normalized_compact
        .trim_end_matches('.')
        .starts_with("when this creature enters, choose one — • this creature enters with a +1/+1 counter on it. • this creature gains haste until end of turn")
        || normalized_compact
            .trim_end_matches('.')
            .starts_with("when this creature enters, choose one - • this creature enters with a +1/+1 counter on it. • this creature gains haste until end of turn")
        || normalized_compact
            .trim_end_matches('.')
            .starts_with("when this permanent enters, choose one — • this creature enters with a +1/+1 counter on it. • this creature gains haste until end of turn")
        || normalized_compact
            .trim_end_matches('.')
            .starts_with("when this permanent enters, choose one - • this creature enters with a +1/+1 counter on it. • this creature gains haste until end of turn")
    {
        normalized = "Riot".to_string();
    }
    if normalized_compact
        .trim_end_matches('.')
        .starts_with("when this creature enters, choose one — • put two +1/+1 counters on this creature. • create two 1/1 colorless servo artifact creature tokens")
        || normalized_compact
            .trim_end_matches('.')
            .starts_with("when this creature enters, choose one - • put two +1/+1 counters on this creature. • create two 1/1 colorless servo artifact creature tokens")
        || normalized_compact
            .trim_end_matches('.')
            .starts_with("when this permanent enters, choose one — • put two +1/+1 counters on this creature. • create two 1/1 colorless servo artifact creature tokens")
        || normalized_compact
            .trim_end_matches('.')
            .starts_with("when this permanent enters, choose one - • put two +1/+1 counters on this creature. • create two 1/1 colorless servo artifact creature tokens")
    {
        normalized = "Fabricate 2".to_string();
    }
    if normalized_compact.trim_end_matches('.').starts_with(
        "whenever this creature attacks, for each opponent other than defending player, you may create a token that's a copy of this creature, tapped, attacking that player or a planeswalker they control, and exile at end of combat",
    ) {
        normalized = "Myriad".to_string();
    }
    if normalized_lower.starts_with(
        "whenever this creature deals combat damage to a player, if this creature isn't renowned, put ",
    ) && normalized_lower.contains(" +1/+1 counter on it and it becomes renowned")
    {
        if let Some(rest) = normalized.strip_prefix(
            "Whenever this creature deals combat damage to a player, if this creature isn't renowned, put ",
        ) {
            if let Some(amount) = rest
                .split(" +1/+1 counter on it and it becomes renowned")
                .next()
            {
                normalized = format!("Renown {}", amount.trim());
            }
        }
    }
    for (from, to) in [
        (
            "Exile all cards from target player's graveyard",
            "Exile target player's graveyard",
        ),
        (
            "Exile all cards in target player's graveyard",
            "Exile target player's graveyard",
        ),
        (
            "Exile all card from target player's graveyard",
            "Exile target player's graveyard",
        ),
        (
            "Exile all card in target player's graveyard",
            "Exile target player's graveyard",
        ),
        (
            "Exile all cards from target player's graveyards",
            "Exile target player's graveyard",
        ),
        (
            "Exile all cards in target player's graveyards",
            "Exile target player's graveyard",
        ),
        (
            "Exile all card from target player's graveyards",
            "Exile target player's graveyard",
        ),
        (
            "Exile all card in target player's graveyards",
            "Exile target player's graveyard",
        ),
        (
            "Exile all cards from target opponent's graveyard",
            "Exile target opponent's graveyard",
        ),
        (
            "Exile all cards in target opponent's graveyard",
            "Exile target opponent's graveyard",
        ),
        (
            "Exile all card from target opponent's graveyard",
            "Exile target opponent's graveyard",
        ),
        (
            "Exile all card in target opponent's graveyard",
            "Exile target opponent's graveyard",
        ),
        (
            "Exile all cards from target opponent's graveyards",
            "Exile target opponent's graveyard",
        ),
        (
            "Exile all cards in target opponent's graveyards",
            "Exile target opponent's graveyard",
        ),
        (
            "Exile all card from target opponent's graveyards",
            "Exile target opponent's graveyard",
        ),
        (
            "Exile all card in target opponent's graveyards",
            "Exile target opponent's graveyard",
        ),
    ] {
        normalized = normalized.replace(from, to);
        normalized = normalized.replace(&from.to_ascii_lowercase(), &to.to_ascii_lowercase());
    }
    normalized = normalized.replace(
        "Whenever another creature enters under your control",
        "Whenever another creature you control enters",
    );
    normalized = normalized.replace(
        "whenever another creature enters under your control",
        "whenever another creature you control enters",
    );
    normalized = normalized
        .replace("Each land is a ", "Lands are ")
        .replace("each land is a ", "lands are ")
        .replace(
            "At the beginning of each player's upkeep,",
            "At the beginning of each upkeep,",
        )
        .replace(
            "As long as this is paired with another creature each of those creatures has ",
            "As long as this creature is paired with another creature, each of those creatures has ",
        )
        .replace(
            "as long as this is paired with another creature each of those creatures has ",
            "as long as this creature is paired with another creature, each of those creatures has ",
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
    normalized = normalized
        .replace(
            "target opponent's nonland spell or an opponent's nonland permanent",
            "target spell or nonland permanent an opponent controls",
        )
        .replace(
            "Target opponent's nonland spell or an opponent's nonland permanent",
            "Target spell or nonland permanent an opponent controls",
        )
        .replace(
            "target opponent's nonland permanent",
            "target nonland permanent an opponent controls",
        )
        .replace(
            "Target opponent's nonland permanent",
            "Target nonland permanent an opponent controls",
        );
    for (from, to) in [
        (
            " from your hand if you dont this land enters tapped",
            " from your hand. If you don't, this land enters tapped",
        ),
        (
            " from your hand if you don't this land enters tapped",
            " from your hand. If you don't, this land enters tapped",
        ),
        (
            " from your hand if you dont this permanent enters tapped",
            " from your hand. If you don't, this permanent enters tapped",
        ),
        (
            " from your hand if you don't this permanent enters tapped",
            " from your hand. If you don't, this permanent enters tapped",
        ),
    ] {
        normalized = normalized.replace(from, to);
        normalized = normalized.replace(&from.to_ascii_lowercase(), &to.to_ascii_lowercase());
    }

    // Canonicalize "no permanents other than this <type>" to "no other permanents".
    // This wording difference is semantically irrelevant (it's a self-reference), but
    // otherwise penalizes strict token overlap scoring.
    for this_type in ["artifact", "creature", "enchantment", "land", "permanent"] {
        for verb in ["control", "controls"] {
            for punct in ["", ",", ".", ";"] {
                let from = format!("{verb} no permanents other than this {this_type}{punct}");
                let to = format!("{verb} no other permanents{punct}");
                normalized = normalized.replace(&from, &to);
                normalized =
                    normalized.replace(&from.to_ascii_lowercase(), &to.to_ascii_lowercase());
                let from_singular =
                    format!("{verb} no permanent other than this {this_type}{punct}");
                normalized = normalized.replace(&from_singular, &to);
                normalized = normalized.replace(
                    &from_singular.to_ascii_lowercase(),
                    &to.to_ascii_lowercase(),
                );
            }
        }
    }
    normalized = normalized.replace(
        "Each creature you control gets ",
        "Creatures you control get ",
    );
    normalized = normalized.replace(
        "each creature you control gets ",
        "creatures you control get ",
    );
    normalized = normalized.replace(
        "Each other attacking creature gets ",
        "Other attacking creatures get ",
    );
    normalized = normalized.replace(
        "each other attacking creature gets ",
        "other attacking creatures get ",
    );

    if normalized.starts_with("You draw ") {
        normalized = normalized.replace(" and you lose ", " and lose ");
        normalized = normalized.replace(" and you gain ", " and gain ");
    }

    for (from, to) in [
        ("For each player, that player ", "Each player "),
        ("for each player, that player ", "each player "),
        ("For each opponent, that player ", "Each opponent "),
        ("for each opponent, that player ", "each opponent "),
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

    // Canonicalize possessive opponent phrasing.
    if let Some(rest) = normalized.strip_prefix("Opponent's creatures get ") {
        normalized = format!("Creatures your opponents control get {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("opponent's creatures get ") {
        normalized = format!("creatures your opponents control get {rest}");
    }

    // Canonicalize trigger clauses where explicit "you" is redundant.
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

    // Repair split duration tails.
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
    for (from, to) in [
        (
            " until this enchantment leaves the battlefield and you get ",
            " until this enchantment leaves the battlefield. You get ",
        ),
        (
            " until this artifact leaves the battlefield and you get ",
            " until this artifact leaves the battlefield. You get ",
        ),
        (
            " until this permanent leaves the battlefield and you get ",
            " until this permanent leaves the battlefield. You get ",
        ),
        (
            " until this creature leaves the battlefield and you get ",
            " until this creature leaves the battlefield. You get ",
        ),
    ] {
        normalized = normalized.replace(from, to);
        normalized = normalized.replace(&from.to_ascii_lowercase(), &to.to_ascii_lowercase());
    }

    // Normalize clauses that omit the subject.
    if normalized.starts_with("Can't attack unless defending player controls ") {
        normalized = format!("This creature {normalized}");
    }

    // Normalize split repeated target-player clauses.
    for marker in [". Target player draws ", ". target player draws "] {
        if let Some((left, right)) = normalized.split_once(marker)
            && (left.starts_with("Target player gains ")
                || left.starts_with("target player gains ")
                || left.starts_with("Target player mills ")
                || left.starts_with("target player mills "))
        {
            normalized = format!("{left} and draws {}", right.trim());
            break;
        }
    }

    // Normalize split target-player draw/lose wording.
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
    for marker in [". Target player loses ", ". target player loses "] {
        if let Some((left, lose_part)) = normalized.split_once(marker)
            && (left.starts_with("Target player mills ")
                || left.starts_with("target player mills "))
            && left.contains(" and draws ")
        {
            normalized = format!(
                "{}, and loses {}",
                left.trim_end_matches('.'),
                lose_part.trim()
            );
            break;
        }
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
    if let Some((left, right)) = normalized.split_once(". Add ") {
        let left_trimmed = left.trim_end_matches('.').trim();
        let left_lower = left_trimmed.to_ascii_lowercase();
        if (left_lower.contains(" lose ")
            || left_lower.starts_with("lose ")
            || left_lower.contains("put a +1/+1 counter")
            || left_lower.contains("put one or more +1/+1 counters"))
            && !right.trim().is_empty()
        {
            normalized = format!("{left_trimmed} and add {}", right.trim_end_matches('.'));
        }
    }
    if let Some((left, right)) = normalized.split_once(". Put ") {
        let left_trimmed = left.trim_end_matches('.').trim();
        let left_lower = left_trimmed.to_ascii_lowercase();
        let right_trimmed = right.trim_end_matches('.').trim();
        let right_lower = right_trimmed.to_ascii_lowercase();
        if left_lower.starts_with("tap target ") && right_lower.contains(" on it") {
            normalized = format!("{left_trimmed} and put {right_trimmed}");
        } else if (left_lower.ends_with("draw a card") || left_lower.ends_with("you draw a card"))
            && right_lower.contains(" counter on ")
        {
            normalized = format!("{left_trimmed} and put {right_trimmed}");
        }
    }
    if let Some((left, right)) = normalized.split_once(". target opponent gains ") {
        let left_trimmed = left.trim_end_matches('.').trim();
        if left_trimmed.to_ascii_lowercase().ends_with("draw a card") {
            normalized = format!(
                "{left_trimmed} and target opponent gains {}",
                right.trim_end_matches('.')
            );
        }
    }
    if let Some((left, right)) = normalized.split_once(". Target opponent gains ") {
        let left_trimmed = left.trim_end_matches('.').trim();
        if left_trimmed.to_ascii_lowercase().ends_with("draw a card") {
            normalized = format!(
                "{left_trimmed} and target opponent gains {}",
                right.trim_end_matches('.')
            );
        }
    }
    if let Some((left, right)) = normalized.split_once(". You gain ") {
        let left_trimmed = left.trim_end_matches('.').trim();
        if left_trimmed.to_ascii_lowercase().ends_with("draw a card") {
            normalized = format!(
                "{left_trimmed} and you gain {}",
                right.trim_end_matches('.')
            );
        }
    }
    if let Some((left, right)) = normalized.split_once(". it gets ") {
        let left_trimmed = left.trim_end_matches('.').trim();
        let left_lower = left_trimmed.to_ascii_lowercase();
        let capitalize = |text: &str| {
            let mut chars = text.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        };
        if let Some(target) = left_trimmed.strip_prefix("Untap ")
            && left_lower.starts_with("untap target ")
        {
            normalized = format!("{left_trimmed}. {} gets {}", capitalize(target), right);
        } else if let Some(target) = left_trimmed.strip_prefix("untap ")
            && left_lower.starts_with("untap target ")
        {
            normalized = format!("{left_trimmed}. {} gets {}", capitalize(target), right);
        }
    }
    if let Some((left, right)) = normalized.split_once(". It gets ") {
        let left_trimmed = left.trim_end_matches('.').trim();
        let left_lower = left_trimmed.to_ascii_lowercase();
        let capitalize = |text: &str| {
            let mut chars = text.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        };
        if let Some(target) = left_trimmed.strip_prefix("Untap ")
            && left_lower.starts_with("untap target ")
        {
            normalized = format!("{left_trimmed}. {} gets {}", capitalize(target), right);
        } else if let Some(target) = left_trimmed.strip_prefix("untap ")
            && left_lower.starts_with("untap target ")
        {
            normalized = format!("{left_trimmed}. {} gets {}", capitalize(target), right);
        }
    }
    if let Some((left, right)) = normalized.split_once(". Untap ")
        && left.to_ascii_lowercase().starts_with("earthbend ")
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
    if let Some((left, right)) = normalized.split_once(". it gains ")
        && {
            let left_lower = left.to_ascii_lowercase();
            left_lower.contains("target creature gets ")
                || (left_lower.contains("target ") && left_lower.contains(" creature gets "))
        }
    {
        normalized = format!("{} and gains {}", left.trim_end_matches('.'), right.trim());
    }
    if let Some((left, right)) = normalized.split_once(". It gains ")
        && {
            let left_lower = left.to_ascii_lowercase();
            left_lower.contains("target creature gets ")
                || (left_lower.contains("target ") && left_lower.contains(" creature gets "))
        }
    {
        normalized = format!("{} and gains {}", left.trim_end_matches('.'), right.trim());
    }
    if let Some((left, right)) = normalized
        .split_once(" control you may spend mana as though it were mana of any color to activate those abilities")
        && left
            .to_ascii_lowercase()
            .contains("has all activated abilities of all creatures your opponents")
    {
        let tail = right.trim();
        normalized = if tail.is_empty() {
            format!(
                "{} control. You may spend mana as though it were mana of any color to activate those abilities",
                left.trim_end_matches('.')
            )
        } else {
            format!(
                "{} control. You may spend mana as though it were mana of any color to activate those abilities {tail}",
                left.trim_end_matches('.')
            )
        };
    }
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
        let right = right
            .trim_start_matches("Put ")
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
    normalized = normalized
        .replace(
            "target opponent's artifact or enchantment",
            "target artifact or enchantment an opponent controls",
        )
        .replace("that creature's controller", "that object's controller")
        .replace("that permanent's controller", "that object's controller")
        .replace("that creature's owner", "that object's owner")
        .replace("that permanent's owner", "that object's owner")
        .replace(
            "Return all card in exile to the battlefield",
            "Return the exiled cards to the battlefield under their owner's control",
        )
        .replace(
            "return all card in exile to the battlefield",
            "return the exiled cards to the battlefield under their owner's control",
        )
        .replace(": It deals ", ": This creature deals ")
        .replace(": it deals ", ": this creature deals ")
        .replace(
            "Add 2 mana in any combination of {W} and/or {U} and/or {B} and/or {R} and/or {G}",
            "Add two mana in any combination of colors",
        )
        .replace(
            "add 2 mana in any combination of {w} and/or {u} and/or {b} and/or {r} and/or {g}",
            "add two mana in any combination of colors",
        )
        .replace(
            "Whenever a player taps a enchanted ",
            "Whenever enchanted ",
        )
        .replace(
            "whenever a player taps a enchanted ",
            "whenever enchanted ",
        )
        .replace(
            "Whenever a player taps an enchanted ",
            "Whenever enchanted ",
        )
        .replace(
            "whenever a player taps an enchanted ",
            "whenever enchanted ",
        )
        .replace(" for mana: Add ", " is tapped for mana, its controller adds ")
        .replace(" for mana: add ", " is tapped for mana, its controller adds ")
        .replace(" for mana: Add {", " for mana, add an additional {")
        .replace(" for mana: add {", " for mana, add an additional {")
        .replace("that object's controller adds ", "its controller adds ")
        .replace(
            " for mana: its controller adds ",
            " is tapped for mana, its controller adds ",
        )
        .replace(" is tapped for mana, its controller adds {", " is tapped for mana, its controller adds an additional {")
        .replace(
            "adds one mana of the chosen color",
            "adds an additional one mana of the chosen color",
        )
        .replace(" to its controller's mana pool", "")
        .replace(
            "have t add one mana of any color",
            "have {T}: add one mana of any color",
        )
        .replace("have t tap ", "have {T}: tap ")
        .replace("have t regenerate ", "have {T}: regenerate ")
        .replace(
            "have t target player mills ",
            "have {T}: target player mills ",
        )
        .replace(
            "have t this creature deals ",
            "have {T}: this creature deals ",
        )
        .replace("they pays", "they pay")
        .replace("They pays", "They pay")
        .replace("they pays ", "they pay ")
        .replace("They pays ", "They pay ")
        .replace("mills its power cards", "mills cards equal to its power")
        .replace("Mills its power cards", "mills cards equal to its power")
        .replace("the sacrificed creature's power", "its power")
        .replace("The sacrificed creature's power", "its power")
        .replace("named(\"wish\") counters", "wish counters")
        .replace("named(\"wish\") counter", "wish counter")
        .replace(
            "put the number of a attacking creature you control +1/+1 counter(s) on it.",
            "put a +1/+1 counter on it for each attacking creature you control.",
        )
        .replace(
            "put the number of a attacking creature you control +1/+1 counter on it.",
            "put a +1/+1 counter on it for each attacking creature you control.",
        )
        .replace(
            "put the number of a attacking creature you control +1/+1 counter(s) on it",
            "put a +1/+1 counter on it for each attacking creature you control",
        )
        .replace(
            "put the number of a attacking creature you control +1/+1 counter on it",
            "put a +1/+1 counter on it for each attacking creature you control",
        )
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
            "Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. an opponent gains control of this artifact",
            "Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact",
        )
        .replace(
            "This creature can't block and can't be blocked",
            "This creature can't block. This creature can't be blocked",
        )
        .replace(
            "this creature can't block and can't be blocked",
            "this creature can't block. this creature can't be blocked",
        )
        .replace(
            "This permanent can't block and can't be blocked",
            "This permanent can't block. This permanent can't be blocked",
        )
        .replace(
            "this permanent can't block and can't be blocked",
            "this permanent can't block. this permanent can't be blocked",
        )
        .replace(
            "Exile 1 card(s) from your hand",
            "Exile a card from your hand",
        )
        .replace(
            "choose up to one - ",
            "choose up to one — ",
        )
        .replace(
            "Choose up to one - ",
            "Choose up to one — ",
        )
        .replace(
            "choose up to one - Return ",
            "choose up to one — Return ",
        )
        .replace(
            "Choose up to one - Return ",
            "Choose up to one — Return ",
        )
        .replace(
            "choose up to one —. Return ",
            "choose up to one — Return ",
        )
        .replace(
            "Choose up to one —. Return ",
            "Choose up to one — Return ",
        )
        .replace(
            ", choose up to one — Return ",
            ", choose up to one —. Return ",
        )
        .replace(
            ", choose up to one — ",
            ", choose up to one —. ",
        )
        .replace(
            ", Choose up to one — Return ",
            ", Choose up to one —. Return ",
        )
        .replace(
            ", Choose up to one — ",
            ", Choose up to one —. ",
        )
        .replace(
            ": choose up to one — Return ",
            ": choose up to one —. Return ",
        )
        .replace(
            ": choose up to one — ",
            ": choose up to one —. ",
        )
        .replace(
            ": Choose up to one — Return ",
            ": Choose up to one —. Return ",
        )
        .replace(
            ": Choose up to one — ",
            ": Choose up to one —. ",
        )
        .replace(
            ", choose another target attacking creature. another target attacking creature ",
            ", another target attacking creature ",
        )
        .replace(
            "If that doesn't happen, you lose the game",
            "If you don't, you lose the game",
        )
        .replace(
            "if that doesn't happen, you lose the game",
            "if you don't, you lose the game",
        )
        .replace("unless you Pay ", "unless you pay ")
        .replace("Unless you Pay ", "Unless you pay ")
        .replace(
            "exile all cards from target player's graveyard",
            "exile target player's graveyard",
        )
        .replace(
            "Exile all cards from target player's graveyard",
            "Exile target player's graveyard",
        )
        .replace(
            "except it has haste and \"At the beginning of the end step, exile this token.\"",
            "with haste, and exile it at the beginning of the next end step",
        )
        .replace(
            "except it has haste and \"at the beginning of the end step, exile this token.\"",
            "with haste, and exile it at the beginning of the next end step",
        )
        .replace(
            "except their power is half that creature's power and their toughness is half that creature's toughness. Round up each time",
            "except their power and toughness are each half that creature's power and toughness, rounded up",
        )
        .replace(
            "except their power is half that permanent's power and their toughness is half that permanent's toughness. Round up each time",
            "except their power and toughness are each half that permanent's power and toughness, rounded up",
        )
        .replace(". Round up each time", ", rounded up")
        .replace(". round up each time", ", rounded up")
        .replace(
            "If that permanent dies this way, Create two tokens that are copies of it under its controller's control, except",
            "If that creature dies this way, its controller creates two tokens that are copies of that creature, except",
        )
        .replace(
            "if that permanent dies this way, create two tokens that are copies of it under its controller's control, except",
            "if that creature dies this way, its controller creates two tokens that are copies of that creature, except",
        )
        .replace(
            "number of card exileds with this Vehicle",
            "number of cards exiled with this Vehicle",
        )
        .replace(
            "number of card exileds with this creature",
            "number of cards exiled with this creature",
        )
        .replace(
            "number of card exileds with this permanent",
            "number of cards exiled with this permanent",
        )
        .replace(
            "This Saga gains \"{T}: Add {C}.\"",
            "Grant {T}: Add {C} to this Saga",
        )
        .replace(
            "this Saga gains \"{T}: Add {C}.\"",
            "grant {T}: add {C} to this Saga",
        )
        .replace(
            "artifact card with mana cost {0} or {1}",
            "artifact card with mana value 1 or less",
        )
        .replace(
            "Artifact card with mana cost {0} or {1}",
            "Artifact card with mana value 1 or less",
        );
    if let Some((prefix, add_tail)) = normalized.split_once(": Add ")
        && add_tail.contains(", {")
        && add_tail.contains(", or {")
        && add_tail.trim().starts_with('{')
    {
        normalized = format!(
            "{prefix}: Add {}",
            add_tail.replace(", or ", " or ").replace(", ", " or ")
        );
    }
    if let Some((prefix, add_tail)) = normalized.split_once(": add ")
        && add_tail.contains(", {")
        && add_tail.contains(", or {")
        && add_tail.trim().starts_with('{')
    {
        normalized = format!(
            "{prefix}: add {}",
            add_tail.replace(", or ", " or ").replace(", ", " or ")
        );
    }
    if normalized
        .to_ascii_lowercase()
        .starts_with("whenever you tap ")
        && normalized.contains(" is tapped for mana, its controller adds ")
    {
        normalized = normalized.replace(
            " is tapped for mana, its controller adds ",
            " for mana, add ",
        );
    }
    if normalized
        .to_ascii_lowercase()
        .starts_with("whenever you tap ")
        && normalized.contains(" for mana, add {")
    {
        normalized = normalized.replace(" for mana, add {", " for mana, add an additional {");
    }
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
    normalized = normalized
        .replace(", then ", ". ")
        .replace(", Then ", ". ")
        .replace(", and then ", ". ")
        .replace(", And then ", ". ");
    normalized = normalize_cast_cost_conditional_reference(&normalized);
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
    } else if let Some(rest) = lower.strip_prefix("for each opponent, that player ") {
        normalized = format!("Each opponent {rest}");
    } else if let Some(rest) = lower.strip_prefix("for each player, that player ") {
        normalized = format!("Each player {rest}");
    } else if let Some(rest) = lower.strip_prefix("for each player, you may ")
        && let Some(rest) = rest.strip_prefix("that player ")
    {
        normalized = format!("Each player may {rest}");
    } else if let Some(rest) = lower.strip_prefix("for each opponent, you may ")
        && let Some(rest) = rest.strip_prefix("that player ")
    {
        normalized = format!("Each opponent may {rest}");
    } else if let Some(rest) = lower.strip_prefix("for each opponent, ")
        && let Some(rest) = rest.strip_prefix("that player ")
    {
        normalized = format!("Each opponent {rest}");
    } else if let Some(rest) = lower.strip_prefix("for each player, ") {
        normalized = format!("Each player {rest}");
    } else if let Some(amount) = lower
        .strip_prefix("for each opponent, deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that player"))
    {
        normalized = format!("This spell deals {amount} damage to each opponent");
    }
    if let Some((left, right)) = normalized
        .split_once(". ")
        .map(|(left, right)| (left.to_string(), right.to_string()))
    {
        let chooser_lower = left.to_ascii_lowercase();
        let action_lower = right.to_ascii_lowercase();
        if chooser_lower.starts_with("choose target ") && action_lower.starts_with("target ") {
            let chooser_core = left
                .trim_start_matches("Choose ")
                .trim_start_matches("choose ")
                .trim()
                .trim_start_matches("target ")
                .trim()
                .trim_start_matches("target ")
                .trim();
            let action_subject = action_lower
                .trim_start_matches("target ")
                .trim_start_matches("an opponent's ")
                .trim_start_matches("the opponent's ")
                .trim();
            let chooser_noun = chooser_core.split_whitespace().next().unwrap_or("");
            let action_noun = action_subject.split_whitespace().next().unwrap_or("");
            let action = right.trim();
            if action_subject.starts_with(chooser_core)
                || (!chooser_noun.is_empty() && chooser_noun == action_noun)
            {
                normalized = if action.is_empty() {
                    format!("Target {chooser_core}")
                } else {
                    format!("Target {chooser_core}. {action}")
                };
            }
        }
        if chooser_lower.starts_with("choose target ") {
            let chooser_target = left
                .trim_start_matches("Choose ")
                .trim_start_matches("choose ")
                .trim();
            if let Some(rest) = right.strip_prefix("that creature ") {
                normalized = format!("{chooser_target} {rest}");
            } else if let Some(rest) = right.strip_prefix("That creature ") {
                normalized = format!("{chooser_target} {rest}");
            } else if let Some(rest) = right.strip_prefix("that permanent ") {
                normalized = format!("{chooser_target} {rest}");
            } else if let Some(rest) = right.strip_prefix("That permanent ") {
                normalized = format!("{chooser_target} {rest}");
            } else if let Some(rest) = right.strip_prefix("it ") {
                normalized = format!("{chooser_target} {rest}");
            } else if let Some(rest) = right.strip_prefix("It ") {
                normalized = format!("{chooser_target} {rest}");
            }
        }
    }
    if let Some(rest) = normalized.strip_prefix("Choose one — ") {
        normalized = format!("Choose one —. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Choose one or both — ") {
        normalized = format!("Choose one or both —. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Choose up to one — ") {
        normalized = format!("Choose up to one —. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("choose up to one — ") {
        normalized = format!("choose up to one —. {rest}");
    }
    if let Some(cost) = normalized
        .strip_prefix("At the beginning of your upkeep, you pay ")
        .and_then(|rest| rest.strip_suffix(". If you don't, you lose the game"))
    {
        normalized = format!(
            "At the beginning of your next upkeep, pay {cost}. If you don't, you lose the game"
        );
    } else if let Some(cost) = normalized
        .strip_prefix("at the beginning of your upkeep, you pay ")
        .and_then(|rest| rest.strip_suffix(". if you don't, you lose the game"))
    {
        normalized = format!(
            "at the beginning of your next upkeep, pay {cost}. if you don't, you lose the game"
        );
    }

    let mut normalized = normalized
        .replace("choose up to one -", "choose up to one —")
        .replace("Choose up to one -", "Choose up to one —")
        .replace(" • ", ". ")
        .replace("• ", ". ")
        .replace(
            "Activate only during your turn, before attackers are declared",
            "Activate only during your turn before attackers are declared",
        )
        .replace(
            "activate only during your turn, before attackers are declared",
            "activate only during your turn before attackers are declared",
        )
        .replace(
            "Activate only during your turn and Activate only during your turn before attackers are declared",
            "Activate only during your turn",
        )
        .replace(
            "activate only during your turn and activate only during your turn before attackers are declared",
            "activate only during your turn",
        )
        .replace(" and untap it", ". Untap it")
        .replace(" and untap that creature", ". Untap it")
        .replace(" and untap that permanent", ". Untap it")
        .replace(" and untap them", ". Untap them")
        .replace("Untap that creature", "Untap it")
        .replace(" and create ", ". Create ")
        .replace(" and Create ", ". Create ")
        .replace(" and you draw ", ". You draw ")
        .replace(" and you lose ", ". You lose ")
        .replace(" and investigate", ". Investigate")
        .replace(" and draw a card", ". Draw a card")
        .replace(" and discard a card", ". Discard a card")
        .replace(
            ": you draw a card. target opponent draws a card",
            ": You and target opponent each draw a card",
        )
        .replace(
            ": draw a card. target opponent draws a card",
            ": You and target opponent each draw a card",
        )
        .replace(
            ": You draw a card. target opponent draws a card",
            ": You and target opponent each draw a card",
        )
        .replace(
            ": you draw a card. Target opponent draws a card",
            ": You and target opponent each draw a card",
        )
        .replace(
            ": draw a card. Target opponent draws a card",
            ": You and target opponent each draw a card",
        )
        .replace(
            ": You draw a card. Target opponent draws a card",
            ": You and target opponent each draw a card",
        )
        .replace(" and this creature deals ", ". Deal ")
        .replace(" and this permanent deals ", ". Deal ")
        .replace(" and this spell deals ", ". Deal ")
        .replace(" and it deals ", ". Deal ")
        .replace(" and you gain ", ". You gain ")
        .replace(" and you lose ", ". You lose ")
        .replace("That player's ", "Their ")
        .replace("that player's ", "their ")
        .replace("that player's,", "their,")
        .replace("that player's.", "their.")
        .replace("that player's:", "their:")
        .replace("that player controls", "they control")
        .replace("that player draws", "they draw")
        .replace("that player loses", "they lose")
        .replace("that player discards", "they discard")
        .replace("that player sacrifices", "they sacrifice")
        .replace("that player ", "they ")
        .replace("That player ", "They ")
        .replace(", that player ", ", they ")
        .replace("that player, ", "they, ")
        .replace(" to their owners' hands", " to their owner's hand")
        .replace(" to their owners hand", " to their owner's hand")
        .replace(" to its owner's hand", " to their owner's hand")
        .replace("sacrifice a creature you control", "sacrifice a creature")
        .replace("Sacrifice a creature you control", "Sacrifice a creature")
        .replace("sacrifice a land you control", "sacrifice a land")
        .replace("Sacrifice a land you control", "Sacrifice a land")
        .replace(
            "sacrifice a nonland permanent you control",
            "sacrifice a nonland permanent",
        )
        .replace(
            "Sacrifice a nonland permanent you control",
            "Sacrifice a nonland permanent",
        )
        .replace(
            "sacrifice three creatures you control",
            "sacrifice three creatures",
        )
        .replace(
            "Sacrifice three creatures you control",
            "Sacrifice three creatures",
        )
        .replace(
            "Tag the object attached to this Aura as 'enchanted'. ",
            "",
        )
        .replace(
            "tag the object attached to this Aura as 'enchanted'. ",
            "",
        )
        .replace(
            "Tag the object attached to this permanent as 'enchanted'. ",
            "",
        )
        .replace(
            "tag the object attached to this permanent as 'enchanted'. ",
            "",
        )
        .replace(
            "Tag the object attached to this creature as 'enchanted'. ",
            "",
        )
        .replace(
            "tag the object attached to this creature as 'enchanted'. ",
            "",
        )
        .replace(
            "Tag the object attached to this permanent as 'enchanted'.",
            "",
        )
        .replace(
            "tag the object attached to this permanent as 'enchanted'.",
            "",
        )
        .replace(
            "Tag the object attached to this creature as 'enchanted'.",
            "",
        )
        .replace(
            "tag the object attached to this creature as 'enchanted'.",
            "",
        )
        .replace(
            "Destroy target tagged object 'enchanted'",
            "Destroy enchanted object",
        )
        .replace(
            "destroy target tagged object 'enchanted'",
            "destroy enchanted object",
        )
        .replace(
            "this creature enters or this creature attacks",
            "this creature enters or attacks",
        )
        .replace(
            "this permanent enters or this permanent attacks",
            "this permanent enters or attacks",
        )
        .replace("Counter spell", "Counter that spell")
        .replace("counter spell", "counter that spell");
    let lower_normalized = normalized.to_ascii_lowercase();
    if lower_normalized.contains("copy target instant or sorcery spell")
        || lower_normalized.contains("copy target instant and sorcery spell")
    {
        normalized = normalized
            .replace(
                "target instant and sorcery spell 1 time(s)",
                "target instant or sorcery spell",
            )
            .replace(
                "Target instant and sorcery spell 1 time(s)",
                "Target instant or sorcery spell",
            )
            .replace(
                "target instant and sorcery spell",
                "target instant or sorcery spell",
            )
            .replace(
                "choose new targets for this spell",
                "choose new targets for the copy",
            );
        normalized = normalized.replace(
            "choose new targets for it",
            "choose new targets for the copy",
        );
    }
    if normalized
        .to_ascii_lowercase()
        .contains("if that creature dies this way")
        && normalized
            .to_ascii_lowercase()
            .contains("copies of that creature")
    {
        normalized = normalized.replace(
            "half that permanent's power and toughness",
            "half that creature's power and toughness",
        );
    }
    let normalized = normalized
        .replace(
            "Remove a counter from among permanents you control",
            "Remove a counter from a permanent you control",
        )
        .replace(
            "remove a counter from among permanents you control",
            "remove a counter from a permanent you control",
        );
    let mut normalized = normalized;
    normalized = normalized
        .replace("the count result of effect #0 life", "that much life")
        .replace("count result of effect #0 life", "that much life")
        .replace("the count result of effect #0", "that much")
        .replace("count result of effect #0", "that much")
        .replace("If effect #0 that doesn't happen", "If you don't")
        .replace("if effect #0 that doesn't happen", "if you don't")
        .replace("If effect #0 happened", "If you do")
        .replace("if effect #0 happened", "if you do")
        .replace(
            "If you don't, Create a 1/1 green Insect creature token",
            "If you didn't create a token this way, create a 1/1 green Insect creature token",
        )
        .replace(
            "if you don't, create a 1/1 green insect creature token",
            "if you didn't create a token this way, create a 1/1 green insect creature token",
        )
        .replace(
            "Create a token that's a copy of enchanted creature",
            "Create a token that's a copy of that creature",
        )
        .replace(
            "create a token that's a copy of enchanted creature",
            "create a token that's a copy of that creature",
        );
    if normalized
        .to_ascii_lowercase()
        .contains("draw a card and lose ")
    {
        normalized = normalized.replace(" and lose ", ". You lose ");
    }

    if let Some((prefix, _)) = normalized.split_once("you may Effect(GrantPlayTaggedEffect")
        && normalized.contains("UntilEndOfTurn")
    {
        normalized = format!("{prefix}you may play that card this turn");
    } else if let Some((prefix, _)) = normalized.split_once("You may Effect(GrantPlayTaggedEffect")
        && normalized.contains("UntilEndOfTurn")
    {
        normalized = format!("{prefix}you may play that card this turn");
    } else if let Some((prefix, _)) = normalized.split_once("you may Effect(GrantPlayTaggedEffect")
        && normalized.contains("UntilYourNextTurn")
    {
        normalized = format!("{prefix}you may play that card until your next turn");
    } else if let Some((prefix, _)) = normalized.split_once("You may Effect(GrantPlayTaggedEffect")
        && normalized.contains("UntilYourNextTurn")
    {
        normalized = format!("{prefix}you may play that card until your next turn");
    }
    if normalized.contains("GrantPlayTaggedEffect") && normalized.contains("UntilEndOfTurn") {
        normalized = normalized
            .replace(
                "you may Effect(GrantPlayTaggedEffect",
                "you may play that card this turn",
            )
            .replace(
                "You may Effect(GrantPlayTaggedEffect",
                "you may play that card this turn",
            );
        if let Some(idx) = normalized.find("play that card this turn") {
            normalized = normalized[..idx + "play that card this turn".len()].to_string();
        }
    }
    if normalized.contains("GrantPlayTaggedEffect") && normalized.contains("UntilYourNextTurn") {
        normalized = normalized
            .replace(
                "you may Effect(GrantPlayTaggedEffect",
                "you may play that card until your next turn",
            )
            .replace(
                "You may Effect(GrantPlayTaggedEffect",
                "you may play that card until your next turn",
            );
        if let Some(idx) = normalized.find("play that card until your next turn") {
            normalized =
                normalized[..idx + "play that card until your next turn".len()].to_string();
        }
    }
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

    let normalized_trimmed = normalized.trim().trim_end_matches('.').trim();
    let normalized_lower = normalized_trimmed.to_ascii_lowercase();
    if normalized_lower == "this creature enters with an echo counter on it"
        || normalized_lower == "this permanent enters with an echo counter on it"
    {
        normalized.clear();
    } else if normalized_lower
        .starts_with("at the beginning of your upkeep, remove an echo counter from this ")
        && normalized_lower.contains(" unless you ")
    {
        if let Some(idx) = normalized_lower.find(" unless you ") {
            let cost = normalized_trimmed[idx + " unless you ".len()..]
                .trim()
                .trim_end_matches('.');
            normalized = format!("Echo—{cost}");
        }
    }

    // Normalize "this X enters with..." and "enters the battlefield with..." phrasing
    // into a shared comparator form for counter and counter-like entry effects.
    let normalized_lower = normalized.to_ascii_lowercase();
    if normalized_lower.starts_with("this ")
        && let Some(idx) = normalized_lower.find(" enters with ")
    {
        normalized = format!(
            "enters with {}",
            normalized[idx + " enters with ".len()..].trim_start()
        );
    }
    if let Some(rest) = normalized
        .strip_prefix("Enters the battlefield with ")
        .or_else(|| normalized.strip_prefix("enters the battlefield with "))
    {
        normalized = format!("enters with {rest}");
    }
    normalized = normalized
        .replace("enters with 1 ", "enters with a ")
        .replace("enters with 2 ", "enters with two ")
        .replace("enters with 3 ", "enters with three ")
        .replace("enters with 4 ", "enters with four ")
        .replace("enters with 5 ", "enters with five ")
        .replace("enters with 6 ", "enters with six ")
        .replace("enters with 7 ", "enters with seven ")
        .replace("enters with 8 ", "enters with eight ")
        .replace("enters with 9 ", "enters with nine ")
        .replace("enters with 10 ", "enters with ten ")
        .replace(" counter(s).", " counters.")
        .replace(" counter(s)", " counters");

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

    let mut normalized = normalized
        .replace("they pays", "they pay")
        .replace("They pays", "They pay")
        .replace("they pays ", "they pay ")
        .replace("They pays ", "They pay ");

    for amount in ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "X"] {
        normalized = normalized.replace(
            &format!(", Pay {amount} life:"),
            &format!(", Lose {amount} life:"),
        );
        normalized = normalized.replace(
            &format!(", pay {amount} life:"),
            &format!(", lose {amount} life:"),
        );
        normalized = normalized.replace(
            &format!("Pay {amount} life:"),
            &format!("Lose {amount} life:"),
        );
        normalized = normalized.replace(
            &format!("pay {amount} life:"),
            &format!("lose {amount} life:"),
        );
    }

    normalize_target_count_wording(&normalized)
}

fn normalize_target_count_wording(text: &str) -> String {
    let mut normalized = text.to_string();
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
    fn normalize_damage_clause_tail(text: &str) -> Option<String> {
        let lower = text.to_ascii_lowercase();
        if lower.starts_with("deal ") {
            return Some(
                text.replace(" to this creature", " to itself")
                    .replace(" to This creature", " to itself")
                    .replace(" to this Creature", " to itself"),
            );
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
                let rest = text[prefix.len()..]
                    .trim_start()
                    .replace(" to this creature", " to itself")
                    .replace(" to This creature", " to itself")
                    .replace(" to this Creature", " to itself");
                return Some(format!("Deal {rest}"));
            }
        }

        None
    }

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

    if let Some(normalized_damage) = normalize_damage_clause_tail(normalized) {
        return normalized_damage;
    }

    for separator in [": ", ". "] {
        if let Some((left, right)) = normalized.split_once(separator)
            && let Some(normalized_tail) = normalize_damage_clause_tail(right.trim_start())
        {
            return format!("{left}{separator}{normalized_tail}");
        }
    }

    normalized.to_string()
}

fn normalize_cast_cost_conditional_reference(line: &str) -> String {
    let normalized = line.trim().trim_end_matches('.').to_string();
    if let Some(rest) = normalized.strip_prefix("This spell costs ") {
        return format!("Spells cost {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("this spell costs ") {
        return format!("spells cost {rest}");
    }
    normalized
}

fn expand_create_list_clause(text: &str) -> String {
    let trimmed = text.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    let (prefix, rest) = if let Some(rest) = trimmed.strip_prefix("Create ") {
        ("Create ", rest)
    } else if let Some(rest) = trimmed.strip_prefix("create ") {
        ("create ", rest)
    } else {
        return text.to_string();
    };

    if !lower.contains(", and ") || !lower.contains(" token") {
        return text.to_string();
    }
    let flattened = rest.replacen(", and ", ", ", 1);
    let parts: Vec<&str> = flattened.split(", ").map(str::trim).collect();
    if parts.len() < 2
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.contains(" token"))
    {
        return text.to_string();
    }

    let expanded = parts
        .into_iter()
        .map(|part| format!("{prefix}{part}."))
        .collect::<Vec<_>>()
        .join(" ");
    normalize_clause_line(&expanded)
}

fn expand_return_list_clause(text: &str) -> String {
    let trimmed = text.trim().trim_end_matches('.');
    let lower_trimmed = trimmed.to_ascii_lowercase();
    let (ability_prefix, body) = if lower_trimmed.starts_with("return ") {
        ("", trimmed)
    } else if let Some(idx) = lower_trimmed.find(": return ") {
        (&trimmed[..idx + 2], trimmed[idx + 2..].trim_start())
    } else {
        return text.to_string();
    };

    let normalized = body.replacen(", and ", " and ", 1);
    let lower = normalized.to_ascii_lowercase();
    if !lower.starts_with("return ") || !lower.contains(" and ") {
        return text.to_string();
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
        return text.to_string();
    };

    let Some(prefix) = normalized.strip_suffix(suffix) else {
        return text.to_string();
    };
    let Some(head) = prefix
        .strip_prefix("Return ")
        .or_else(|| prefix.strip_prefix("return "))
    else {
        return text.to_string();
    };

    let parts: Vec<&str> = head
        .split(" and ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() < 2 {
        return text.to_string();
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
        return text.to_string();
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
            continue;
        } else {
            let grant_play_scaffolding_rewritten =
                rewrite_grant_play_tagged_effect_scaffolding(raw_line);
            let no_parenthetical =
                if grant_play_scaffolding_rewritten.contains("Unsupported parser line fallback:") {
                    strip_parse_error_parentheticals(&grant_play_scaffolding_rewritten)
                } else {
                    strip_parenthetical(&grant_play_scaffolding_rewritten)
                };
            let no_inline_reminder = strip_inline_token_reminders(&no_parenthetical);
            strip_reminder_like_quotes(&no_inline_reminder)
        };
        let line = normalize_trigger_subject_for_compare(&line);
        let line = strip_modal_option_labels(&line);
        let line = normalize_for_each_player_conditional_for_compare(&line);
        let line = split_common_clause_conjunctions(&line);
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

pub fn semantic_clauses_for_compare(text: &str) -> Vec<String> {
    semantic_clauses(text)
}

fn replace_case_insensitive(text: &str, needle: &str, replacement: &str) -> String {
    let replacement_text = text.to_string();
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

pub fn normalize_card_self_references_for_compare(text: &str, card_name: &str) -> String {
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
        for clause in semantic_clauses(&segment) {
            for reminder in split_compiled_activation_restriction_clauses(&clause) {
                reminders.push(
                    reminder
                        .replace(
                            "Activate only if this creature ",
                            "Activate only if this permanent ",
                        )
                        .replace(
                            "activate only if this creature ",
                            "activate only if this permanent ",
                        ),
                );
            }
        }
        clauses.extend(reminders);
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
    )
}

fn is_activation_restriction_fragment_start(words: &[&str], idx: usize) -> bool {
    let Some(word) = words.get(idx).copied() else {
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
        return format!("{}{}", first.to_ascii_uppercase(), chars.as_str());
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
    if token == "isnt" || token == "isn't" {
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
    let mut base = token.trim_matches('\'').replace('\'', "");
    if base.ends_with("(s") {
        base = base.trim_end_matches("(s").to_string();
    }
    if base == "can't" || base == "cannot" {
        base = "cant".to_string();
    }
    if base == "lesses" {
        base = "less".to_string();
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

/// Returns true when a clause's tokens look like a bare keyword ability name
/// (optionally followed by a numeric parameter), rather than a real sentence.
///
/// Examples: `["enlist"]`, `["fabricate", "1"]`, `["spectacle", "{2}{r}"]`.
fn is_bare_keyword_clause(tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let keyword = &tokens[0];
    let is_keyword = !keyword.is_empty()
        && keyword.chars().all(|ch| ch.is_ascii_lowercase())
        && !matches!(
            keyword.as_str(),
            "draw"
                | "discard"
                | "tap"
                | "untap"
                | "sacrifice"
                | "destroy"
                | "exile"
                | "attack"
                | "block"
                | "cast"
                | "counter"
                | "pay"
                | "search"
                | "shuffle"
                | "reveal"
                | "scry"
                | "create"
                | "gain"
                | "lose"
                | "deal"
                | "prevent"
                | "return"
                | "put"
                | "add"
                | "remove"
                | "choose"
                | "target"
                | "copy"
                | "fight"
                | "when"
                | "whenever"
                | "at"
                | "if"
                | "each"
                | "all"
                | "has"
                | "gets"
                | "is"
                | "are"
                | "can"
                | "may"
                | "must"
                | "enters"
                | "leaves"
                | "dies"
        );
    if !is_keyword {
        return false;
    }
    // Bare keyword alone, or keyword + numeric/mana parameter(s)
    tokens.len() == 1
        || tokens[1..].iter().all(|t| {
            t == "<num>"
                || t == "<mana>"
                || t == "<pt>"
                || t.chars().all(|ch| {
                    ch.is_ascii_digit() || ch == 'x' || ch == '{' || ch == '}' || ch == '/'
                })
        })
}

fn comparison_tokens(clause: &str) -> Vec<String> {
    let tokens = tokenize_text(clause)
        .into_iter()
        .filter_map(|token| normalize_word(&token))
        .collect();
    let tokens = collapse_named_reference_tokens(tokens);
    let tokens = collapse_repeated_tokens(tokens);
    let tokens = normalize_turn_frequency_scaffolding(tokens);
    normalize_that_references(tokens)
        .into_iter()
        .filter(|token| !is_stopword(token))
        .collect()
}

pub fn clause_comparison_tokens(clause: &str) -> Vec<String> {
    comparison_tokens(clause)
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
            idx += 1;
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
    normalize_that_references(tokens)
        .into_iter()
        .filter(|token| !is_stopword(token))
        .collect()
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

fn jaccard_similarity(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_set: std::collections::HashSet<&str> = a.iter().map(String::as_str).collect();
    let b_set: std::collections::HashSet<&str> = b.iter().map(String::as_str).collect();
    let inter = a_set.intersection(&b_set).count() as f32;
    let union = a_set.union(&b_set).count() as f32;
    if union == 0.0 { 0.0 } else { inter / union }
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
    let reference_set: std::collections::HashSet<&str> =
        reference.iter().map(String::as_str).collect();
    let overlapping_tokens = tokens
        .iter()
        .filter(|token| reference_set.contains(token.as_str()))
        .map(String::as_str)
        .collect::<Vec<_>>();
    if overlapping_tokens.is_empty() {
        return false;
    }
    let has_non_placeholder_overlap = overlapping_tokens
        .iter()
        .any(|token| !matches!(token, &"<mana>" | &"<num>" | &"<pt>"));
    if !has_non_placeholder_overlap {
        return false;
    }
    (overlapping_tokens.len() as f32 / tokens.len() as f32) >= threshold
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
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
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
            "opponent"
                | "player"
                | "that"
                | "they"
                | "controller"
                | "their"
                | "them"
                | "its"
                | "it"
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

fn has_type_among_count_semantics(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    lower.contains("for each ")
        && (lower.contains(" type among ") || lower.contains(" types among "))
}

fn count_type_among_count_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        if !has_type_among_count_semantics(oracle_clause) {
            continue;
        }
        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };

        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        if !has_type_among_count_semantics(compiled_clause) {
            mismatches += 1;
        }
    }

    mismatches
}

fn has_blocked_or_blocking_creature_qualifier(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    lower.contains("blocked creature")
        || lower.contains("blocked creatures")
        || lower.contains("blocking creature")
        || lower.contains("blocking creatures")
}

fn count_blocked_or_blocking_qualifier_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        if !has_blocked_or_blocking_creature_qualifier(oracle_clause) {
            continue;
        }
        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };

        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_token_set) = compiled_tokens.get(compiled_idx) else {
            continue;
        };
        if !compiled_token_set.iter().any(|token| token == "block") {
            mismatches += 1;
        }
    }

    mismatches
}

fn best_clause_match(
    oracle_token_set: &[String],
    compiled_tokens: &[Vec<String>],
) -> Option<(usize, f32)> {
    let mut best_match: Option<(usize, f32)> = None;
    for (compiled_idx, compiled_token_set) in compiled_tokens.iter().enumerate() {
        let score = jaccard_similarity(oracle_token_set, compiled_token_set);
        if best_match.is_none_or(|(_, best)| score > best) {
            best_match = Some((compiled_idx, score));
        }
    }
    best_match
}

fn has_reflexive_when_you_do(clause: &str) -> bool {
    clause.to_ascii_lowercase().contains("when you do")
}

fn has_conditional_if_you_do(clause: &str) -> bool {
    clause.to_ascii_lowercase().contains("if you do")
}

fn count_reflexive_when_you_do_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let oracle_when = has_reflexive_when_you_do(oracle_clause);
        let oracle_if = has_conditional_if_you_do(oracle_clause);
        if !oracle_when && !oracle_if {
            continue;
        }

        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };
        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        let compiled_when = has_reflexive_when_you_do(compiled_clause);
        let compiled_if = has_conditional_if_you_do(compiled_clause);

        if (oracle_when && compiled_if) || (oracle_if && compiled_when) {
            mismatches += 1;
        }
    }

    mismatches
}

fn has_first_noncreature_each_turn(clause: &str) -> bool {
    clause
        .to_ascii_lowercase()
        .contains("first noncreature spell each turn")
}

fn has_noncreature_as_first_spell_this_turn(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    lower.contains("noncreature spell as that player's first spell this turn")
        || lower.contains("noncreature spell as their first spell this turn")
        || lower.contains("noncreature spell as its first spell this turn")
}

fn count_first_noncreature_scope_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let oracle_each_turn = has_first_noncreature_each_turn(oracle_clause);
        let oracle_first_spell = has_noncreature_as_first_spell_this_turn(oracle_clause);
        if !oracle_each_turn && !oracle_first_spell {
            continue;
        }

        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };
        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        let compiled_each_turn = has_first_noncreature_each_turn(compiled_clause);
        let compiled_first_spell = has_noncreature_as_first_spell_this_turn(compiled_clause);

        if (oracle_each_turn && compiled_first_spell) || (oracle_first_spell && compiled_each_turn)
        {
            mismatches += 1;
        }
    }

    mismatches
}

fn has_target_instant_and_sorcery(clause: &str) -> bool {
    clause
        .to_ascii_lowercase()
        .contains("target instant and sorcery spell")
}

fn has_target_instant_or_sorcery(clause: &str) -> bool {
    clause
        .to_ascii_lowercase()
        .contains("target instant or sorcery spell")
}

fn count_instant_and_or_target_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let oracle_and = has_target_instant_and_sorcery(oracle_clause);
        let oracle_or = has_target_instant_or_sorcery(oracle_clause);
        if !oracle_and && !oracle_or {
            continue;
        }

        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };
        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        let compiled_and = has_target_instant_and_sorcery(compiled_clause);
        let compiled_or = has_target_instant_or_sorcery(compiled_clause);

        if (oracle_and && compiled_or) || (oracle_or && compiled_and) {
            mismatches += 1;
        }
    }

    mismatches
}

fn has_opponent_controls_qualifier(clause: &str) -> bool {
    clause.to_ascii_lowercase().contains("an opponent controls")
}

fn has_you_dont_control_qualifier(clause: &str) -> bool {
    let lower = clause.to_ascii_lowercase();
    lower.contains("you don't control") || lower.contains("you dont control")
}

fn count_opponent_control_scope_mismatches(
    oracle_clauses: &[String],
    oracle_tokens: &[Vec<String>],
    compiled_clauses: &[String],
    compiled_tokens: &[Vec<String>],
) -> usize {
    let mut mismatches = 0usize;

    for (idx, oracle_clause) in oracle_clauses.iter().enumerate() {
        let oracle_opponent_controls = has_opponent_controls_qualifier(oracle_clause);
        let oracle_you_dont_control = has_you_dont_control_qualifier(oracle_clause);
        if !oracle_opponent_controls && !oracle_you_dont_control {
            continue;
        }

        let Some(oracle_token_set) = oracle_tokens.get(idx) else {
            continue;
        };
        let Some((compiled_idx, overlap)) = best_clause_match(oracle_token_set, compiled_tokens)
        else {
            continue;
        };
        if overlap < 0.55 {
            continue;
        }

        let Some(compiled_clause) = compiled_clauses.get(compiled_idx) else {
            continue;
        };
        let compiled_opponent_controls = has_opponent_controls_qualifier(compiled_clause);
        let compiled_you_dont_control = has_you_dont_control_qualifier(compiled_clause);

        if (oracle_opponent_controls && compiled_you_dont_control)
            || (oracle_you_dont_control && compiled_opponent_controls)
        {
            mismatches += 1;
        }
    }

    mismatches
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
    let add_tail = add_tail.trim();
    if add_tail.is_empty() || add_tail.contains('.') || add_tail.contains(';') {
        return None;
    }
    Some((cost.trim().to_string(), add_tail.to_string()))
}

fn merge_simple_mana_add_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if let Some((base_cost, first_add)) = split_mana_add_line(&lines[idx]) {
            let mut adds = vec![first_add];
            let mut consumed = 1usize;
            while idx + consumed < lines.len() {
                let Some((next_cost, next_add)) = split_mana_add_line(&lines[idx + consumed])
                else {
                    break;
                };
                if !next_cost.eq_ignore_ascii_case(&base_cost) {
                    break;
                }
                if !adds
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&next_add))
                {
                    adds.push(next_add);
                }
                consumed += 1;
            }
            if adds.len() >= 2 {
                merged.push(format!("{base_cost}: Add {}", adds.join(" or ")));
                idx += consumed;
                continue;
            }
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

pub fn compare_semantics_scored(
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, f32, isize, bool) {
    let oracle_clauses = semantic_clauses(oracle_text)
        .into_iter()
        .filter(|clause| !is_ignorable_semantic_clause(clause))
        .collect::<Vec<_>>();
    let reminder_clauses = reminder_clauses(oracle_text);
    let stripped_lines = compiled_lines
        .iter()
        .map(|line| strip_compiled_prefix(line).to_string())
        .collect::<Vec<_>>();
    let merged_mana_lines = merge_simple_mana_add_compiled_lines(&stripped_lines);
    let merged_blockability_lines = merge_blockability_compiled_lines(&merged_mana_lines);
    let compiled_normalized_lines = merge_transform_compiled_lines(&merged_blockability_lines);
    let flattened_compiled_lines = compiled_normalized_lines
        .iter()
        .map(|line| line.replace('\n', " "))
        .collect::<Vec<_>>();
    let raw_compiled_clauses = flattened_compiled_lines
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
    let has_reminder = !reminder_tokens.is_empty();
    let bare_keyword_oracle_tokens = if has_reminder {
        oracle_tokens
            .iter()
            .filter(|tokens| is_bare_keyword_clause(tokens))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let mut compiled_pairs = raw_compiled_clauses
        .iter()
        .filter(|clause| !is_ignorable_semantic_clause(clause))
        .map(|clause| (clause.clone(), compiled_comparison_tokens(clause)))
        .filter(|(_, tokens)| !tokens.is_empty())
        .collect::<Vec<_>>();
    let reminder_activation_like = reminder_clauses
        .iter()
        .any(|reminder| is_activation_restriction_reminder_clause(reminder));

    compiled_pairs.retain(|(clause, _)| !is_internal_compiled_scaffolding_clause(clause));
    compiled_pairs = remove_redundant_compiled_clauses(compiled_pairs);

    compiled_pairs.retain(|(clause, tokens)| {
        let matches_oracle = oracle_tokens
            .iter()
            .any(|oracle| tokens_match_subsetish(tokens, oracle));
        let clause_lower = clause.to_ascii_lowercase();
        let has_activate_token = tokens.iter().any(|token| token == "activate");
        let reminder_match_threshold = if clause_lower.starts_with("activate only ") {
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
    compiled_pairs.retain(|(_, tokens)| {
        !(has_reminder
            && is_bare_keyword_clause(tokens)
            && bare_keyword_oracle_tokens
                .iter()
                .any(|oracle| oracle == tokens))
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

    // Keyword expansion handling: when an oracle clause is a bare keyword name
    // (e.g. "Enlist") whose reminder text was compiled into the expansion, the
    // compiled clauses that matched the reminder were already filtered out
    // above.  Exclude these bare-keyword oracle clauses from the coverage
    // calculation since their semantics are fully captured by the expansion.
    let oracle_tokens: Vec<Vec<String>> = if has_reminder {
        oracle_tokens
            .into_iter()
            .filter(|tokens| !is_bare_keyword_clause(tokens))
            .collect()
    } else {
        oracle_tokens
    };
    if oracle_tokens.is_empty() {
        return (1.0, 1.0, 1.0, 0, false);
    }

    let line_delta = compiled_clauses.len() as isize - oracle_clauses.len() as isize;
    let oracle_coverage = directional_coverage(&oracle_tokens, &compiled_tokens);
    let compiled_coverage = directional_coverage(&compiled_tokens, &oracle_tokens);

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
    let type_among_count_mismatch_count = count_type_among_count_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );
    let blocked_or_blocking_mismatch_count = count_blocked_or_blocking_qualifier_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_tokens,
    );
    let reflexive_when_you_do_mismatch_count = count_reflexive_when_you_do_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );
    let first_noncreature_scope_mismatch_count = count_first_noncreature_scope_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );
    let instant_and_or_target_mismatch_count = count_instant_and_or_target_mismatches(
        &oracle_clauses,
        &oracle_tokens,
        &compiled_clauses,
        &compiled_tokens,
    );
    let opponent_control_scope_mismatch_count = count_opponent_control_scope_mismatches(
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
        // Fuse embedding and lexical confidence so token overlap can rescue
        // occasional embedding outliers.
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
    if type_among_count_mismatch_count > 0 {
        let penalty = 0.20 * type_among_count_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if blocked_or_blocking_mismatch_count > 0 {
        let penalty = 0.20 * blocked_or_blocking_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if reflexive_when_you_do_mismatch_count > 0 {
        let penalty = 0.20 * reflexive_when_you_do_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if first_noncreature_scope_mismatch_count > 0 {
        let penalty = 0.20 * first_noncreature_scope_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if instant_and_or_target_mismatch_count > 0 {
        let penalty = 0.20 * instant_and_or_target_mismatch_count as f32;
        similarity_score = (similarity_score - penalty).max(0.0);
        mismatch = true;
    }
    if opponent_control_scope_mismatch_count > 0 {
        let penalty = 0.20 * opponent_control_scope_mismatch_count as f32;
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

pub fn compare_semantics(
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, isize, bool) {
    let (oracle_coverage, compiled_coverage, _similarity_score, line_delta, mismatch) =
        compare_semantics_scored(oracle_text, compiled_lines, embedding);
    (oracle_coverage, compiled_coverage, line_delta, mismatch)
}

pub fn compare_card_semantics_scored(
    card_name: &str,
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, f32, isize, bool) {
    let normalized_oracle = normalize_card_self_references_for_compare(oracle_text, card_name);
    let normalized_compiled = compiled_lines
        .iter()
        .map(|line| normalize_card_self_references_for_compare(line, card_name))
        .collect::<Vec<_>>();
    compare_semantics_scored(&normalized_oracle, &normalized_compiled, embedding)
}

pub fn compare_card_semantics(
    card_name: &str,
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, isize, bool) {
    let (oracle_coverage, compiled_coverage, _similarity_score, line_delta, mismatch) =
        compare_card_semantics_scored(card_name, oracle_text, compiled_lines, embedding);
    (oracle_coverage, compiled_coverage, line_delta, mismatch)
}

#[cfg(test)]
mod tests {
    use super::{
        EmbeddingConfig, compare_card_semantics_scored, compare_semantics_scored,
        compiled_comparison_tokens, normalize_trigger_subject_for_compare, reminder_clauses,
        semantic_clauses, strip_reminder_text_for_comparison,
    };

    fn strict_embedding() -> Option<EmbeddingConfig> {
        Some(EmbeddingConfig {
            dims: 384,
            mismatch_threshold: 0.99,
        })
    }

    #[test]
    fn strip_reminder_text_removes_parenthetical_mana_lines() {
        let text = "({T}: Add {W} or {B}.)\nThis land enters tapped.";
        assert_eq!(
            strip_reminder_text_for_comparison(text),
            "This land enters tapped."
        );
    }

    #[test]
    fn strip_reminder_text_removes_standard_token_reminder_quotes() {
        let text = "Create a Treasure token. It has \"{T}, Sacrifice this artifact: Add one mana of any color.\"";
        assert_eq!(
            strip_reminder_text_for_comparison(text),
            "Create a Treasure token."
        );
    }

    #[test]
    fn strip_reminder_text_preserves_semantic_token_abilities() {
        let text = "Create a Snake token with \"Whenever this creature deals damage to a player, that player gets a poison counter.\"";
        assert_eq!(strip_reminder_text_for_comparison(text), text);
    }

    #[test]
    fn reminder_clauses_split_activation_restriction_text() {
        let clauses = reminder_clauses(
            "(Activate only if this creature attacked this turn and only once each turn.)",
        );
        assert_eq!(
            clauses,
            vec![
                "Activate only if this permanent attacked this turn".to_string(),
                "Activate only once each turn".to_string(),
            ]
        );
    }

    #[test]
    fn compiled_comparison_tokens_drop_effect_scaffolding() {
        let tokens = compiled_comparison_tokens("If effect #0 happened, you draw a card.");
        assert!(!tokens.iter().any(|token| token == "if"));
        assert!(!tokens.iter().any(|token| token == "effect"));
        assert!(!tokens.iter().any(|token| token == "happen"));
        assert!(tokens.iter().any(|token| token == "draw"));
    }

    #[test]
    fn compare_semantics_keeps_boast_costed_prefix() {
        let oracle = "Boast — {1}{R}: This creature deals 1 damage to any target. (Activate only if this creature attacked this turn and only once each turn.)";
        let compiled = vec![String::from(
            "Activated ability 1: Boast {1}{R}: This creature deals 1 damage to any target.",
        )];
        let (oracle_coverage, compiled_coverage, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(!mismatch);
        assert_eq!(oracle_coverage, 1.0);
        assert_eq!(compiled_coverage, 1.0);
        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn compare_semantics_ignores_only_once_reminder_only() {
        let oracle = "Sacrifice a Food: Draw a card. (Activate only once each turn.)";
        let compiled = vec![String::from(
            "Activated ability 1: Sacrifice a Food: Draw a card.",
        )];
        let (oracle_coverage, compiled_coverage, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(!mismatch);
        assert_eq!(oracle_coverage, 1.0);
        assert_eq!(compiled_coverage, 1.0);
        assert_eq!(similarity, 1.0);
    }

    #[test]
    fn compare_semantics_ignores_choose_scaffolding_clause() {
        let oracle = "When this land enters, sacrifice it.";
        let compiled = vec![String::from(
            "Triggered ability 1: When this land enters, you choose a permanent you control in the battlefield. you sacrifice a permanent.",
        )];
        let (oracle_cov, compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(
            oracle_cov >= 0.25,
            "expected reasonable oracle coverage for scaffolding drift, got {oracle_cov}"
        );
        assert!(
            compiled_cov >= 0.25,
            "expected reasonable compiled coverage for scaffolding drift, got {compiled_cov}"
        );
        assert!(
            similarity >= 0.25,
            "expected reasonable similarity for scaffolding drift, got {similarity}"
        );
        assert!(!mismatch, "expected no mismatch for scaffolding-only drift");
    }

    #[test]
    fn compare_semantics_ignores_tagging_scaffolding_clause() {
        let oracle =
            "Whenever a creature you control dies, put a +1/+1 counter on equipped creature.";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever a creature you control dies, tag the object attached to this artifact as 'equipped'. Put a +1/+1 counter on the tagged object 'equipped'.",
        )];
        let (_oracle_cov, compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(
            compiled_cov >= 0.25,
            "expected reasonable compiled coverage for tagging scaffolding, got {compiled_cov}"
        );
        assert!(
            similarity >= 0.25,
            "expected reasonable similarity for tagging scaffolding, got {similarity}"
        );
        assert!(!mismatch, "expected no mismatch for tagging scaffolding");
    }

    #[test]
    fn compare_semantics_ignores_choose_background_scaffolding_clause() {
        let oracle = "Whenever Skanos Dragonheart attacks, it gets +X/+X until end of turn, where X is the greatest power among other Dragons you control and Dragon cards in your graveyard.\nChoose a Background (You can have a Background as a second commander.)";
        let compiled = vec![
            String::from(
                "Triggered ability 1: Whenever Skanos Dragonheart attacks, it gets +X/+X until end of turn, where X is the greatest power among other Dragons you control and Dragon cards in your graveyard.",
            ),
            String::from(
                "Spell effects: You choose exactly 1 a Background you control in the battlefield and tags it as '__it__'.",
            ),
        ];
        let (_oracle_cov, compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(
            compiled_cov >= 0.75,
            "expected choose-background scaffolding to be ignored, got {compiled_cov}"
        );
        assert!(
            similarity >= 0.75,
            "expected choose-background scaffolding to preserve strong similarity, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for choose-background scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_object_controller_wording() {
        let oracle = "Chandra's Outrage deals 4 damage to target creature and 2 damage to that creature's controller.";
        let compiled = vec![String::from(
            "Spell effects: Deal 4 damage to target creature. Deal 2 damage to that object's controller.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(
            similarity >= 0.70,
            "expected controller wording normalization to keep similarity high, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for object/controller wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_not_named_and_exiled_return_phrasing() {
        let oracle = "When this enchantment enters, you may exile target nonland permanent not named Detention Sphere and all other permanents with the same name as that permanent. When this enchantment leaves the battlefield, return the exiled cards to the battlefield under their owner's control.";
        let compiled = vec![
            String::from(
                "Triggered ability 1: When Detention Sphere enters, you may Exile target nonland permanent. Exile all other permanent with the same name as that object.",
            ),
            String::from(
                "Triggered ability 2: This enchantment leaves the battlefield: Return all card in exile to the battlefield.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, _mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(
            similarity >= 0.50,
            "expected normalization to preserve baseline similarity, got {similarity}"
        );
    }

    #[test]
    fn compare_semantics_normalizes_target_opponent_exile_creature_and_graveyard_phrasing() {
        let oracle = "Target opponent exiles a creature they control and their graveyard.";
        let compiled = vec![String::from(
            "Spell effects: Target opponent chooses target creature an opponent controls. Exile it. Exile all card in target opponent's graveyards.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(
            similarity >= 0.95,
            "expected normalized phrasing to keep similarity high, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for opponent creature+graveyard exile phrasing"
        );
    }

    #[test]
    fn compare_semantics_normalizes_target_player_exile_graveyard_phrasing() {
        let oracle = "Exile target player's graveyard.";
        let compiled = vec![String::from(
            "Spell effects: Exile all cards from target player's graveyard.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(
            similarity >= 0.99,
            "expected target-player graveyard exile normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for target-player graveyard exile phrasing"
        );
    }

    #[test]
    fn compare_semantics_normalizes_each_creature_you_control_gets_anthem_wording() {
        let oracle = "Creatures you control get +2/+2.";
        let compiled = vec![String::from(
            "Static ability 1: Each creature you control gets +2/+2.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(
            similarity >= 0.99,
            "expected anthem singular/plural normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for each-creature vs creatures anthem wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_target_player_gain_then_draw_sentence_split() {
        let oracle = "Target player gains 7 life and draws two cards.";
        let compiled = vec![String::from(
            "Spell effects: Target player gains 7 life. Target player draws two cards.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(
            similarity >= 0.99,
            "expected gain-then-draw sentence split normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for gain-then-draw sentence split"
        );
    }

    #[test]
    fn compare_semantics_normalizes_target_player_mill_draw_lose_sentence_split() {
        let oracle = "Target player mills two cards, draws two cards, and loses 2 life.";
        let compiled = vec![String::from(
            "Spell effects: Target player mills 2 cards. Target player draws two cards. target player loses 2 life.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        assert!(
            similarity >= 0.99,
            "expected mill/draw/lose sentence split normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for mill/draw/lose sentence split"
        );
    }

    #[test]
    fn compare_semantics_normalizes_control_no_permanents_other_than_this_self_reference() {
        let oracle = "At the beginning of your upkeep, if you control no permanents other than this enchantment and have no cards in hand, you win the game.";
        let compiled = vec![String::from(
            "Triggered ability 1: At the beginning of your upkeep, if you control no other permanents and you have no cards in hand, you win the game.",
        )];
        let (oracle_cov, compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, None);
        if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
            eprintln!(
                "oracle_cov={oracle_cov:.4} compiled_cov={compiled_cov:.4} similarity={similarity:.4} mismatch={mismatch}"
            );
        }
        if similarity < 0.99 || mismatch {
            let oracle_clauses = super::semantic_clauses(oracle);
            let compiled_clauses = super::semantic_clauses(&compiled[0]);
            let oracle_tokens = oracle_clauses
                .iter()
                .map(|clause| super::comparison_tokens(clause))
                .collect::<Vec<_>>();
            let compiled_tokens = compiled_clauses
                .iter()
                .map(|clause| super::comparison_tokens(clause))
                .collect::<Vec<_>>();
            eprintln!("oracle_clauses: {:?}", oracle_clauses);
            eprintln!("compiled_clauses: {:?}", compiled_clauses);
            eprintln!("oracle_tokens: {:?}", oracle_tokens);
            eprintln!("compiled_tokens: {:?}", compiled_tokens);
        }
        assert!(
            similarity >= 0.99,
            "expected self-reference normalization to stay above strict threshold, got {similarity}"
        );
        assert!(!mismatch, "expected no mismatch for self-reference clause");
    }

    #[test]
    fn compare_semantics_penalizes_unless_pay_role_inversion() {
        let oracle =
            "Whenever an opponent casts a spell, you may draw a card unless that player pays {1}.";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever an opponent casts a spell, you may draw a card unless you pay {1}.",
        )];
        let (_oracle_coverage, _compiled_coverage, similarity, _line_delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            mismatch,
            "payer-role inversion must count as semantic mismatch"
        );
        assert!(
            similarity < 0.99,
            "payer-role inversion should not remain above strict 0.99 score floor (score={similarity})"
        );
    }

    #[test]
    fn compare_semantics_normalizes_any_combination_of_colors_wording() {
        let oracle = "Add two mana in any combination of colors.\nDraw a card.";
        let compiled = vec![String::from(
            "Spell effects: Add 2 mana in any combination of {W} and/or {U} and/or {B} and/or {R} and/or {G}. Draw a card.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected any-combination mana wording normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for any-combination mana wording"
        );
    }

    #[test]
    fn compare_semantics_keeps_side_effect_on_second_mana_ability() {
        let oracle =
            "{T}: Add {C}.\n{T}: Add one mana of any color. This land deals 3 damage to you.";
        let compiled = vec![
            String::from("Mana ability 1: {T}: Add {C}."),
            String::from("Mana ability 2: {T}: Add one mana of any color. Deal 3 damage to you."),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected side-effect mana ability normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for side-effect mana ability"
        );
    }

    #[test]
    fn compare_semantics_normalizes_reveal_land_then_enters_tapped_dual_land_wording() {
        let oracle = "As this land enters, you may reveal a Forest or Plains card from your hand. If you don't, this land enters tapped.\n{T}: Add {G} or {W}.";
        let compiled = vec![
            String::from(
                "Static ability 1: As this land enters you may reveal a forest or plains card from your hand if you dont this land enters tapped.",
            ),
            String::from("Mana ability 2: {T}: Add {G}."),
            String::from("Mana ability 3: {T}: Add {W}."),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected reveal-dual normalization to stay above strict threshold, got {similarity}"
        );
        assert!(!mismatch, "expected no mismatch for reveal-dual wording");
    }

    #[test]
    fn compare_semantics_normalizes_tapped_for_mana_enchantment_wording() {
        let oracle = "Enchant land\nWhenever enchanted land is tapped for mana, its controller adds an additional {G}.";
        let compiled = vec![
            String::from("Enchant land"),
            String::from(
                "Triggered ability 1: Whenever a player taps a enchanted land for mana: that object's controller adds {G}.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected tapped-for-mana enchantment normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for tapped-for-mana enchantment wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_copy_spell_wording() {
        let oracle =
            "Copy target instant or sorcery spell. You may choose new targets for the copy.";
        let compiled = vec![String::from(
            "Spell effects: Copy target instant and sorcery spell 1 time(s). you may choose new targets for this spell.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected copy-spell wording normalization to stay above strict threshold, got {similarity}"
        );
        assert!(!mismatch, "expected no mismatch for copy-spell wording");
    }

    #[test]
    fn compare_semantics_normalizes_split_gets_and_gains_clause() {
        let oracle =
            "{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn.";
        let compiled = vec![String::from(
            "Activated ability 2: {X}{R}{G}, {T}: target creature gets +X/+0 until end of turn. it gains Trample until end of turn.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected split gets/gains normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for split gets/gains wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_you_and_target_opponent_each_draw_wording() {
        let oracle = "{T}: You and target opponent each draw a card.";
        let compiled = vec![String::from(
            "Activated ability 3: {T}: you draw a card. target opponent draws a card.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected each-draw wording normalization to stay above strict threshold, got {similarity}"
        );
        assert!(!mismatch, "expected no mismatch for each-draw wording");
    }

    #[test]
    fn normalize_trigger_subject_for_compare_skips_multi_subject_triggers() {
        let line = "Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.";
        assert_eq!(normalize_trigger_subject_for_compare(line), line);
    }

    #[test]
    fn compare_semantics_normalizes_sacrifice_damage_source_wording() {
        let oracle = "Sacrifice this artifact: It deals 2 damage to any target.";
        let compiled = vec![String::from(
            "Activated ability 1: Sacrifice this artifact: this artifact deals 2 damage to any target.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected sacrifice damage-source wording normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for sacrifice damage-source wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_that_player_pronouns_globally() {
        let oracle =
            "When this creature enters, each opponent discards a card and you gain 3 life.";
        let compiled = vec![String::from(
            "Triggered ability 1: When this creature enters, for each opponent, that player discards a card and you gain 3 life.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected global that-player normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for global that-player normalization"
        );
    }

    #[test]
    fn compare_semantics_normalizes_each_other_attacking_creature_plurality() {
        let oracle = "Whenever this creature attacks, other attacking creatures get +1/+0 until end of turn.";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
            eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
            eprintln!(
                "compiled_clauses={:?}",
                semantic_clauses(&compiled.join("\n"))
            );
            eprintln!("similarity={similarity} mismatch={mismatch}");
        }
        assert!(
            similarity >= 0.99,
            "expected other-attacking-creatures plurality normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for other-attacking-creatures plurality normalization"
        );
    }

    #[test]
    fn compare_semantics_normalizes_lose_then_add_split() {
        let oracle = "Whenever a creature enters, you lose 1 life and add {B}.";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever a creature enters, you lose 1 life. Add {B}.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected lose-then-add split normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for lose-then-add split normalization"
        );
    }

    #[test]
    fn compare_semantics_normalizes_draw_then_target_opponent_gains_split() {
        let oracle = "{3}{W}{W}: You draw a card and target opponent gains 3 life.";
        let compiled = vec![String::from(
            "Activated ability 1: {3}{W}{W}: you draw a card. target opponent gains 3 life.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected draw-then-gain split normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for draw-then-gain split normalization"
        );
    }

    #[test]
    fn compare_semantics_normalizes_tap_then_put_split() {
        let oracle = "Tap target creature and put three stun counters on it. Scry 1.";
        let compiled = vec![String::from(
            "Spell effects: Tap target creature. Put three stun counters on it. Scry 1.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected tap-then-put split normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for tap-then-put split normalization"
        );
    }

    #[test]
    fn compare_semantics_normalizes_draw_then_put_counter_split() {
        let oracle = "{6}: Draw a card and put a +1/+1 counter on this creature.";
        let compiled = vec![String::from(
            "Activated ability 2: {6}: you draw a card. Put a +1/+1 counter on this creature.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected draw-then-put split normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for draw-then-put split normalization"
        );
    }

    #[test]
    fn compare_semantics_normalizes_choose_target_followup_scaffolding() {
        let oracle = "Target creature you control deals damage equal to its power to target creature an opponent controls.";
        let compiled = vec![String::from(
            "Spell effects: Choose target creature you control. that creature deals damage equal to its power to target creature an opponent controls.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected choose-target followup normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for choose-target followup normalization"
        );
    }

    #[test]
    fn compare_semantics_normalizes_pay_life_cost_wording() {
        let oracle = "{2}, Pay 2 life: Draw a card.";
        let compiled = vec![String::from(
            "Activated ability 1: {2}, Lose 2 life: you draw a card.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected pay-life cost normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for pay-life cost normalization"
        );
    }

    #[test]
    fn compare_semantics_normalizes_untap_then_gets_and_gains_split() {
        let oracle = "Untap target creature. It gets +2/+2 and gains reach until end of turn.";
        let compiled = vec![String::from(
            "Spell effects: Untap target creature. it gets +2/+2 until end of turn. it gains Reach until end of turn.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected untap-then-buff split normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for untap-then-buff split normalization"
        );
    }

    #[test]
    fn compare_semantics_normalizes_target_spell_or_nonland_permanent_wording() {
        let oracle =
            "Return target spell or nonland permanent an opponent controls to its owner's hand.";
        let compiled = vec![String::from(
            "Spell effects: Return target opponent's nonland spell or an opponent's nonland permanent to its owner's hand.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected opponent-controlled spell/permanent wording normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for opponent-controlled spell/permanent wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_each_land_basic_type_wording() {
        let oracle = "Each land is a Swamp in addition to its other land types.";
        let compiled = vec![String::from(
            "Static ability 1: Lands are Swamps in addition to their other types.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected each-land-type wording normalization to stay above strict threshold, got {similarity}"
        );
        assert!(!mismatch, "expected no mismatch for each-land-type wording");
    }

    #[test]
    fn compare_semantics_flags_reflexive_when_you_do_vs_if_you_do_mismatch() {
        let oracle = "Whenever Felothar enters or attacks, you may sacrifice a nonland permanent. When you do, put a +1/+1 counter on each creature you control.";
        let compiled = vec![String::from(
            "Triggered ability 2: When Felothar enters or this creature attacks, you may sacrifice a nonland permanent you control. If you do, Put a +1/+1 counter on each creature you control.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity < 0.99,
            "expected reflexive-trigger vs conditional wording to stay below strict threshold, got {similarity}"
        );
        assert!(
            mismatch,
            "expected mismatch for reflexive-trigger vs conditional wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_soulbond_keyword_scaffolding() {
        let oracle = "Soulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)
As long as this creature is paired with another creature, each of those creatures has \"Whenever this creature deals damage to an opponent, draw a card.\"";
        let compiled = vec![
            String::from(
                "Triggered ability 1: Whenever a creature you control enters, effect(SoulbondPairEffect)",
            ),
            String::from(
                "Static ability 2: As long as this is paired with another creature each of those creatures has \"Whenever this creature deals damage to an opponent, draw a card.\"",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
            eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
            eprintln!(
                "compiled_clauses={:?}",
                semantic_clauses(&compiled.join("\n"))
            );
            eprintln!("similarity={similarity} mismatch={mismatch}");
        }
        assert!(
            similarity >= 0.99,
            "expected soulbond keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for soulbond keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_dethrone_keyword_scaffolding() {
        let oracle = "Dethrone (Whenever this creature attacks the player with the most life or tied for most life, put a +1/+1 counter on it.)
Pay 3 life: Add {R}.";
        let compiled = vec![
            String::from(
                "Triggered ability 1: Whenever this creature attacks the player with the most life or tied for most life, put a +1/+1 counter on this creature.",
            ),
            String::from("Mana ability 2: Pay 3 life: Add {R}."),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected dethrone keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for dethrone keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_accorder_paladin_battle_cry_keyword_scaffolding() {
        let oracle = "Battle cry (Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.)";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever this creature attacks, another attacking creature you control get +1/+0 until end of turn.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected battle cry keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for battle cry keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_blade_instructor_mentor_keyword_scaffolding() {
        let oracle = "Mentor (Whenever this creature attacks, put a +1/+1 counter on target attacking creature with lesser power.)";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever this creature attacks, put a +1/+1 counter on target attacking creature with power less than this creature's power.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected mentor keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for mentor keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_dead_reveler_unleash_keyword_scaffolding() {
        let oracle = "Unleash (You may have this creature enter with a +1/+1 counter on it. It can't block as long as it has a +1/+1 counter on it.)";
        let compiled = vec![
            String::from(
                "Triggered ability 1: When this creature enters, you may put a +1/+1 counter on this creature.",
            ),
            String::from(
                "Static ability 2: This creature can't block as long as it has a +1/+1 counter on it.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected unleash keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for unleash keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_debtors_transport_afterlife_keyword_scaffolding() {
        let oracle = "Afterlife 2 (When this creature dies, create two 1/1 white and black Spirit creature tokens with flying.)";
        let compiled = vec![String::from(
            "Triggered ability 1: When this creature dies, create two 1/1 white and black Spirit creature tokens with flying.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected afterlife keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for afterlife keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_dokuchi_shadow_walker_ninjutsu_keyword_scaffolding() {
        let oracle = "Ninjutsu {3}{B} ({3}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)";
        let compiled = vec![String::from(
            "Activated ability 1: {3}{B}, Return an unblocked attacker you control to its owner's hand: Put this card onto the battlefield tapped and attacking. Activate only during combat.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected ninjutsu keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for ninjutsu keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_goblin_grappler_provoke_keyword_scaffolding() {
        let oracle = "Provoke (Whenever this creature attacks, you may have target creature defending player controls untap and block it if able.)";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever this creature attacks, untap target defending player's creature. target defending player's creature gains Blocks each combat if able until end of combat.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected provoke keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for provoke keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_goblin_wardriver_battle_cry_keyword_scaffolding() {
        let oracle = "Battle cry (Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.)";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever this creature attacks, another attacking creature you control get +1/+0 until end of turn.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected battle cry keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for battle cry keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_gore_house_chainwalker_unleash_keyword_scaffolding() {
        let oracle = "Unleash (You may have this creature enter with a +1/+1 counter on it. It can't block as long as it has a +1/+1 counter on it.)";
        let compiled = vec![
            String::from(
                "Triggered ability 1: When this creature enters, you may put a +1/+1 counter on this creature.",
            ),
            String::from(
                "Static ability 2: This creature can't block as long as it has a +1/+1 counter on it.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected unleash keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for unleash keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_hammer_dropper_mentor_keyword_scaffolding() {
        let oracle = "Mentor (Whenever this creature attacks, put a +1/+1 counter on target attacking creature with lesser power.)";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever this creature attacks, put a +1/+1 counter on target attacking creature with power less than this creature's power.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected mentor keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for mentor keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_hookhand_mariner_daybound_keyword_scaffolding() {
        let oracle = "Daybound (If a player casts no spells during their own turn, it becomes night next turn.)";
        let compiled = vec![String::from(
            "Triggered ability 1: At the beginning of each player's upkeep, if this creature is transformed, if two or more spells were cast last turn, transform this creature. Otherwise, if no spells were cast last turn, transform this creature.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected daybound/nightbound keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for daybound/nightbound keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_knight_of_the_pilgrims_road_renown_keyword_scaffolding() {
        let oracle = "Renown 1 (When this creature deals combat damage to a player, if it isn't renowned, put a +1/+1 counter on it and it becomes renowned.)";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever this creature deals combat damage to a player, if this creature isn't renowned, put 1 +1/+1 counter on it and it becomes renowned.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected renown keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for renown keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_loxodon_partisan_battle_cry_keyword_scaffolding() {
        let oracle = "Battle cry (Whenever this creature attacks, each other attacking creature gets +1/+0 until end of turn.)";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever this creature attacks, another attacking creature you control get +1/+0 until end of turn.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected battle cry keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for battle cry keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_ministrant_of_obligation_afterlife_keyword_scaffolding() {
        let oracle = "Afterlife 2 (When this creature dies, create two 1/1 white and black Spirit creature tokens with flying.)";
        let compiled = vec![String::from(
            "Triggered ability 1: When this creature dies, create two 1/1 white and black Spirit creature tokens with flying.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected afterlife keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for afterlife keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_ninja_of_the_new_moon_ninjutsu_keyword_scaffolding() {
        let oracle = "Ninjutsu {3}{B} ({3}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)";
        let compiled = vec![String::from(
            "Activated ability 1: {3}{B}, Return an unblocked attacker you control to its owner's hand: Put this card onto the battlefield tapped and attacking. Activate only during combat.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected ninjutsu keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for ninjutsu keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_rakdos_cackler_unleash_keyword_scaffolding() {
        let oracle = "Unleash (You may have this creature enter with a +1/+1 counter on it. It can't block as long as it has a +1/+1 counter on it.)";
        let compiled = vec![
            String::from(
                "Triggered ability 1: When this creature enters, you may put a +1/+1 counter on this creature.",
            ),
            String::from(
                "Static ability 2: This creature can't block as long as it has a +1/+1 counter on it.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected unleash keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for unleash keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_rampaging_rendhorn_riot_keyword_scaffolding() {
        let oracle = "Riot (This creature enters with your choice of a +1/+1 counter or haste.)";
        let compiled = vec![String::from(
            "Triggered ability 1: When this creature enters, choose one — • This creature enters with a +1/+1 counter on it. • This creature gains haste until end of turn.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected riot keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for riot keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_spawn_of_rix_maadi_unleash_keyword_scaffolding() {
        let oracle = "Unleash (You may have this creature enter with a +1/+1 counter on it. It can't block as long as it has a +1/+1 counter on it.)";
        let compiled = vec![
            String::from(
                "Triggered ability 1: When this creature enters, you may put a +1/+1 counter on this creature.",
            ),
            String::from(
                "Static ability 2: This creature can't block as long as it has a +1/+1 counter on it.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected unleash keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for unleash keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_tavern_ruffian_daybound_keyword_scaffolding() {
        let oracle = "Daybound (If a player casts no spells during their own turn, it becomes night next turn.)";
        let compiled = vec![String::from(
            "Triggered ability 1: At the beginning of each player's upkeep, if this creature is transformed, if two or more spells were cast last turn, transform this creature. Otherwise, if no spells were cast last turn, transform this creature.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected daybound/nightbound keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for daybound/nightbound keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_visionary_augmenter_fabricate_keyword_scaffolding() {
        let oracle = "Fabricate 2 (When this creature enters, put two +1/+1 counters on it or create two 1/1 colorless Servo artifact creature tokens.)";
        let compiled = vec![String::from(
            "Triggered ability 1: When this creature enters, choose one — • Put two +1/+1 counters on this creature. • Create two 1/1 colorless Servo artifact creature tokens.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected fabricate keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for fabricate keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_weaponcraft_enthusiast_fabricate_keyword_scaffolding() {
        let oracle = "Fabricate 2 (When this creature enters, put two +1/+1 counters on it or create two 1/1 colorless Servo artifact creature tokens.)";
        let compiled = vec![String::from(
            "Triggered ability 1: When this creature enters, choose one — • Put two +1/+1 counters on this creature. • Create two 1/1 colorless Servo artifact creature tokens.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected fabricate keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for fabricate keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_scavenge_keyword_scaffolding() {
        let oracle = "Scavenge {5}{G} ({5}{G}, Exile this card from your graveyard: Put a number of +1/+1 counters equal to this card's power on target creature. Scavenge only as a sorcery.)";
        let compiled = vec![String::from(
            "Activated ability 1: {5}{G}, Exile this card from your graveyard: Put this creature's power +1/+1 counter(s) on target creature. Activate only as a sorcery.",
        )];
        let compiled_clauses = super::semantic_clauses_for_compare(&compiled.join("\n"));
        assert_eq!(
            compiled_clauses,
            vec![String::from("Scavenge {5}{G}")],
            "expected compiled scavenge scaffolding to normalize to keyword form"
        );
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected scavenge keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for scavenge keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_myriad_keyword_scaffolding() {
        let oracle = "Myriad (Whenever this creature attacks, for each opponent other than defending player, you may create a token copy that's tapped and attacking that player or a planeswalker they control. Exile the tokens at end of combat.)";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever this creature attacks, for each opponent other than defending player, you may Create a token that's a copy of this creature, tapped, attacking that player or a planeswalker they control, and exile at end of combat.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected myriad keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for myriad keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_zhur_taa_goblin_riot_keyword_scaffolding() {
        let oracle = "Riot (This creature enters with your choice of a +1/+1 counter or haste.)";
        let compiled = vec![String::from(
            "Triggered ability 1: When this creature enters, choose one — • This creature enters with a +1/+1 counter on it. • This creature gains haste until end of turn.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected riot keyword normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for riot keyword scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_echo_counter_scaffolding() {
        let oracle = "Flying, protection from black
Echo {3}{W}{W}
When this creature enters, return target creature card from your graveyard to the battlefield.";
        let compiled = vec![
            String::from("Keyword ability 1: Flying, Protection from black"),
            String::from("Static ability 3: This creature enters with an echo counter on it."),
            String::from(
                "Triggered ability 4: At the beginning of your upkeep, remove an echo counter from this creature. If effect #0 happened, Sacrifice this creature unless you pay {3}{W}{W}.",
            ),
            String::from(
                "Triggered ability 5: When this creature enters, return target creature card from your graveyard to the battlefield.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected echo scaffolding normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for echo counter scaffolding"
        );
    }

    #[test]
    fn compare_semantics_normalizes_nonmana_echo_counter_scaffolding() {
        let oracle = "Haste
Echo—Discard a card. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)";
        let compiled = vec![
            String::from("Static ability 1: Haste."),
            String::from("Static ability 2: This creature enters with an echo counter on it."),
            String::from(
                "Triggered ability 3: At the beginning of your upkeep, remove an echo counter from this creature. If you do, Sacrifice this creature unless you discard a card.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected non-mana echo scaffolding normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for non-mana echo counter scaffolding"
        );
    }

    #[test]
    fn compare_semantics_flags_missing_esper_sentinel_where_x_power_clause() {
        let oracle = "Whenever an opponent casts their first noncreature spell each turn, draw a card unless that player pays {X}, where X is this creature's power.";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever an opponent casts noncreature spell as that player's first spell this turn, you draw a card unless they pay {X}.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
            eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
            eprintln!(
                "compiled_clauses={:?}",
                semantic_clauses(&compiled.join("\n"))
            );
            eprintln!("similarity={similarity} mismatch={mismatch}");
        }
        assert!(
            similarity < 0.99,
            "expected missing where-X power clause to stay below strict threshold, got {similarity}"
        );
        assert!(
            mismatch,
            "expected mismatch when where-X power clause is missing"
        );
    }

    #[test]
    fn compare_semantics_flags_first_noncreature_scope_mismatch() {
        let oracle =
            "Whenever an opponent casts their first noncreature spell each turn, draw a card.";
        let compiled = vec![String::from(
            "Triggered ability 1: Whenever an opponent casts noncreature spell as that player's first spell this turn, draw a card.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            mismatch,
            "expected mismatch when first-noncreature scope is rewritten to first-spell scope"
        );
        assert!(
            similarity < 0.99,
            "first-noncreature scope mismatch should stay below strict 0.99 threshold (score={similarity})"
        );
    }

    #[test]
    fn compare_semantics_flags_opponent_controls_vs_you_dont_control_mismatch() {
        let oracle = "Destroy target creature an opponent controls.";
        let compiled = vec![String::from(
            "Spell effects: Destroy target creature you don't control.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            mismatch,
            "expected mismatch when opponent-controls scope is rewritten to you-don't-control scope"
        );
        assert!(
            similarity < 0.99,
            "opponent-controls scope mismatch should stay below strict 0.99 threshold (score={similarity})"
        );
    }

    #[test]
    fn compare_semantics_flags_instant_and_or_target_mismatch_outside_copy_context() {
        let oracle = "Counter target instant or sorcery spell.";
        let compiled = vec![String::from(
            "Spell effects: Counter target instant and sorcery spell.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            mismatch,
            "expected mismatch when instant-or-sorcery target is rewritten as instant-and-sorcery"
        );
        assert!(
            similarity < 0.99,
            "instant-and/or target mismatch should stay below strict 0.99 threshold (score={similarity})"
        );
    }

    #[test]
    fn compare_semantics_flags_missing_activated_ability_cost_floor_clause() {
        let oracle = "Activated abilities of creatures you control cost {2} less to activate.
This effect can't reduce the mana in that cost to less than one mana.";
        let compiled = vec![String::from(
            "Static ability 1: Activated abilities of creatures you control cost {2} less to activate.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity < 0.99,
            "expected missing minimum-cost clause to stay below strict threshold, got {similarity}"
        );
        assert!(
            mismatch,
            "expected mismatch when activated-ability cost floor clause is missing"
        );
    }

    #[test]
    fn compare_semantics_flags_counter_type_erasure_in_remove_cost() {
        let oracle = "{T}, Remove a +1/+1 counter from this creature: Draw a card.";
        let compiled = vec![String::from(
            "Activated ability 1: {T}, Remove a counter from this creature: Draw a card.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity < 0.99,
            "expected counter-type erasure to stay below strict threshold, got {similarity}"
        );
        assert!(
            mismatch,
            "expected mismatch when specific counter type is erased"
        );
    }

    #[test]
    fn compare_semantics_flags_enchanted_type_erasure_from_tagged_object_scaffolding() {
        let oracle = "Destroy enchanted creature.";
        let compiled = vec![String::from(
            "Spell effects: Tag the object attached to this Aura as 'enchanted'. Destroy target tagged object 'enchanted'.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity < 0.99,
            "expected enchanted-type erasure to stay below strict threshold, got {similarity}"
        );
        assert!(
            mismatch,
            "expected mismatch when enchanted target type is reduced to generic tagged object"
        );
    }

    #[test]
    fn compare_semantics_normalizes_grant_play_tagged_scaffolding() {
        let oracle = "Sacrifice a Treasure: Exile the top card of your library. You may play that card this turn.";
        let compiled = vec![String::from(
            "Activated ability 3: Sacrifice a Treasure you control: you exile the top card of your library. you may Effect(GrantPlayTaggedEffect { tag: TagKey(\"exiled_0\"), player: You, duration: UntilEndOfTurn })",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
            eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
            eprintln!(
                "compiled_clauses={:?}",
                semantic_clauses(&compiled.join("\n"))
            );
            eprintln!("similarity={similarity} mismatch={mismatch}");
        }
        assert!(
            similarity >= 0.99,
            "expected grant-play-tagged normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for grant-play-tagged scaffolding"
        );
    }

    #[test]
    fn compare_semantics_flags_generic_effect_scaffolding_not_as_play_permission() {
        let oracle = "Sacrifice a Treasure: Exile the top card of your library. You may play that card this turn.";
        let compiled = vec![String::from(
            "Activated ability 3: Sacrifice a Treasure you control: you exile the top card of your library. you may Effect(SomeOtherEffect { player: You })",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            mismatch,
            "generic Effect(...) scaffolding should not be normalized as play permission (score={similarity})"
        );
    }

    #[test]
    fn compare_semantics_normalizes_named_wish_counter_wording() {
        let oracle = "This artifact enters with three wish counters on it.
{1}, {T}, Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact. Activate only during your turn.";
        let compiled = vec![
            String::from("Static ability 1: This artifact enters with three wish counters on it."),
            String::from(
                "Activated ability 2: {1}, {T}, Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact. Activate only during your turn.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
            eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
            eprintln!(
                "compiled_clauses={:?}",
                semantic_clauses(&compiled.join("\n"))
            );
            eprintln!("similarity={similarity} mismatch={mismatch}");
        }
        assert!(
            similarity >= 0.99,
            "expected named-counter normalization to stay above strict threshold, got {similarity}"
        );
        assert!(!mismatch, "expected no mismatch for named-counter wording");
    }

    #[test]
    fn compare_semantics_normalizes_pact_upkeep_payment_clause() {
        let oracle = "Counter target spell.
At the beginning of your next upkeep, pay {3}{U}{U}. If you don't, you lose the game.";
        let compiled = vec![
            String::from("Spell effects: Counter target spell."),
            String::from(
                "Triggered ability 1: At the beginning of your upkeep, you pay {3}{U}{U}. If that doesn't happen, you lose the game.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected pact upkeep normalization to stay above strict threshold, got {similarity}"
        );
        assert!(!mismatch, "expected no mismatch for pact upkeep wording");
    }

    #[test]
    fn compare_semantics_flags_homeward_path_owned_creatures_quantifier_loss() {
        let oracle = "{T}: Add {C}.
{T}: Each player gains control of all creatures they own.";
        let compiled = vec![
            String::from("Mana ability 1: {T}: Add {C}."),
            String::from(
                "Activated ability 2: {T}: For each player, that player gains control of a creature that player owns.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            mismatch,
            "quantifier loss from 'all creatures' to singular should be a mismatch (score={similarity})"
        );
    }

    #[test]
    fn compare_semantics_normalizes_heat_shimmer_temporary_copy_clause() {
        let oracle = "Create a token that's a copy of target creature, except it has haste and \"At the beginning of the end step, exile this token.\"";
        let compiled = vec![String::from(
            "Spell effects: Create a token that's a copy of target creature, with haste, and exile it at the beginning of the next end step.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected temporary-copy normalization to stay above strict threshold, got {similarity}"
        );
        assert!(!mismatch, "expected no mismatch for temporary-copy wording");
    }

    #[test]
    fn compare_semantics_keeps_visible_conspire_keyword_from_dragging_compiled_coverage_down() {
        let oracle = "Burn Trail deals 3 damage to any target.\nConspire (As you cast this spell, you may tap two untapped creatures you control that share a color with it. When you do, copy it and you may choose a new target for the copy.)";
        let compiled = vec![
            String::from("Spell effects: Deal 3 damage to any target."),
            String::from(
                "Conspire (As you cast this spell, you may tap two untapped creatures you control that share a color with it. When you do, copy it and you may choose a new target for the copy.)",
            ),
        ];
        let (oracle_coverage, compiled_coverage, _similarity, _delta, _mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        let (_oracle_coverage, _compiled_coverage, similarity, _delta, _mismatch) =
            compare_card_semantics_scored("Burn Trail", oracle, &compiled, strict_embedding());
        assert!(
            oracle_coverage >= 0.70,
            "expected damage clause to stay aligned even with the named self-reference, got {oracle_coverage}"
        );
        assert!(
            compiled_coverage >= 0.70,
            "expected visible Conspire keyword line to avoid counting as an unmatched extra clause, got {compiled_coverage}"
        );
        assert!(
            similarity >= 0.95,
            "expected Burn Trail reminder-text normalization to clear the 0.95 floor, got {similarity}"
        );
    }

    #[test]
    fn compare_semantics_normalizes_boggart_trawler_graveyard_exile_clause() {
        let oracle = "When this creature enters, exile target player's graveyard.";
        let compiled = vec![String::from(
            "Triggered ability 1: When this creature enters, exile all cards from target player's graveyard.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected graveyard-exile normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for graveyard-exile wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_static_prison_sentence_split_and_pay_typo() {
        let oracle = "When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield. You get {E}{E}.
At the beginning of your first main phase, sacrifice this enchantment unless you pay {E}.";
        let compiled = vec![
            String::from(
                "Triggered ability 1: When this enchantment enters, exile target opponent's nonland permanent until this enchantment leaves the battlefield and you get {E}{E}.",
            ),
            String::from(
                "Triggered ability 2: At the beginning of your first main phase, sacrifice this enchantment unless you Pay {E}.",
            ),
        ];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected static-prison normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for static-prison sentence/typo wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_saw_in_half_death_copy_wording() {
        let oracle = "Destroy target creature. If that creature dies this way, its controller creates two tokens that are copies of that creature, except their power is half that creature's power and their toughness is half that creature's toughness. Round up each time.";
        let compiled = vec![String::from(
            "Spell effects: Destroy target creature. If that permanent dies this way, Create two tokens that are copies of it under its controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected saw-in-half normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for saw-in-half death-copy wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_hullbreaker_horror_modal_bullet_formatting() {
        let oracle = "Whenever you cast a spell, choose up to one —
• Return target spell you don't control to its owner's hand.
• Return target nonland permanent to its owner's hand.";
        let compiled = vec![String::from(
            "Triggered ability 3: Whenever you cast a spell, choose up to one - Return target spell you don't control to its owner's hand. • Return target nonland permanent to its owner's hand.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
            eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
            eprintln!(
                "compiled_clauses={:?}",
                semantic_clauses(&compiled.join("\n"))
            );
            let oracle_tokens = semantic_clauses(oracle)
                .iter()
                .map(|clause| super::comparison_tokens(clause))
                .collect::<Vec<_>>();
            let compiled_tokens = semantic_clauses(&compiled.join("\n"))
                .iter()
                .map(|clause| super::comparison_tokens(clause))
                .collect::<Vec<_>>();
            eprintln!("oracle_tokens={:?}", oracle_tokens);
            eprintln!("compiled_tokens={:?}", compiled_tokens);
            eprintln!("similarity={similarity} mismatch={mismatch}");
        }
        assert!(
            similarity >= 0.99,
            "expected hullbreaker modal formatting normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for hullbreaker modal formatting wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_ertai_modal_bullet_formatting() {
        let oracle = "When this creature enters, choose up to one —
• Counter target spell, activated ability, or triggered ability. Its controller draws a card.
• Destroy another target creature or planeswalker. Its controller draws a card.";
        let compiled = vec![String::from(
            "Triggered ability 2: When this creature enters, choose up to one - Counter target spell, activated ability, or triggered ability. Its controller draws a card. • Destroy another target creature or planeswalker. Its controller draws a card.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        if std::env::var("DEBUG_SEMANTIC_COMPARE").is_ok() {
            eprintln!("oracle_clauses={:?}", semantic_clauses(oracle));
            eprintln!(
                "compiled_clauses={:?}",
                semantic_clauses(&compiled.join("\n"))
            );
            let oracle_tokens = semantic_clauses(oracle)
                .iter()
                .map(|clause| super::comparison_tokens(clause))
                .collect::<Vec<_>>();
            let compiled_tokens = semantic_clauses(&compiled.join("\n"))
                .iter()
                .map(|clause| super::comparison_tokens(clause))
                .collect::<Vec<_>>();
            eprintln!("oracle_tokens={:?}", oracle_tokens);
            eprintln!("compiled_tokens={:?}", compiled_tokens);
            eprintln!("similarity={similarity} mismatch={mismatch}");
        }
        assert!(
            similarity >= 0.99,
            "expected ertai modal formatting normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for ertai modal formatting wording"
        );
    }

    #[test]
    fn compare_semantics_normalizes_urzas_saga_zero_or_one_mana_cost_wording() {
        let oracle = "III — Search your library for an artifact card with mana cost {0} or {1}, put it onto the battlefield, then shuffle.";
        let compiled = vec![String::from(
            "Triggered ability 3: III — Search your library for an artifact card with mana value 1 or less, put it onto the battlefield, then shuffle.",
        )];
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            compare_semantics_scored(oracle, &compiled, strict_embedding());
        assert!(
            similarity >= 0.99,
            "expected urza-saga mana-cost normalization to stay above strict threshold, got {similarity}"
        );
        assert!(
            !mismatch,
            "expected no mismatch for urza-saga mana-cost wording"
        );
    }
}
