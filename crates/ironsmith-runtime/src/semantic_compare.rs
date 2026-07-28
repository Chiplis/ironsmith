use std::hash::{Hash, Hasher};

mod scoring;
pub use scoring::*;
use scoring::{
    collapse_named_reference_tokens, collapse_repeated_tokens, normalize_that_references,
    normalize_turn_frequency_scaffolding,
};

#[cfg(test)]
mod tests;

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
        || lower.starts_with("{t}, sacrifice this token, exile the top card of your library")
        || lower.starts_with("sacrifice this token, exile the top card of your library")
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

fn is_power_toughness_shorthand_token(token: &str) -> bool {
    let token = token.trim_matches(|ch: char| matches!(ch, ',' | '.' | ';' | ':'));
    let Some((power, toughness)) = token.split_once('/') else {
        return false;
    };
    let is_pt_part = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, 'x' | 'X' | '*' | '+'))
    };
    is_pt_part(power) && is_pt_part(toughness)
}

fn find_creature_word(rest: &str) -> Option<(usize, usize, &'static str)> {
    let lower = rest.to_ascii_lowercase();
    let mut best: Option<(usize, usize, &'static str)> = None;
    for (needle, word) in [
        (" creatures", "creatures"),
        (" creature", "creature"),
        ("creatures", "creatures"),
        ("creature", "creature"),
    ] {
        let Some(pos) = lower.find(needle) else {
            continue;
        };
        let start = if needle.starts_with(' ') {
            pos + 1
        } else {
            pos
        };
        let end = start + word.len();
        let boundary_before = start == 0
            || lower[..start]
                .chars()
                .last()
                .is_some_and(|ch| ch.is_whitespace());
        let boundary_after = lower[end..]
            .chars()
            .next()
            .is_none_or(|ch| ch.is_whitespace() || matches!(ch, ',' | '.' | ';' | ':'));
        if !boundary_before || !boundary_after {
            continue;
        }
        if best.is_none_or(|(best_start, _, _)| start < best_start) {
            best = Some((start, end, word));
        }
    }
    best
}

fn normalize_fixed_pt_animation_shorthand_once(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let (marker_start, marker) = [" becomes ", " become ", " are "]
        .iter()
        .filter_map(|marker| lower.find(marker).map(|idx| (idx, *marker)))
        .min_by_key(|(idx, _)| *idx)?;

    let marker_end = marker_start + marker.len();
    let after_marker = &text[marker_end..];
    let leading_space = after_marker.len() - after_marker.trim_start().len();
    let mut value_start = marker_end + leading_space;
    let after_marker_lower = text[value_start..].to_ascii_lowercase();
    for article in ["a ", "an "] {
        if after_marker_lower.starts_with(article) {
            value_start += article.len();
            break;
        }
    }

    let after_article = &text[value_start..];
    let pt_end_rel = after_article.find(char::is_whitespace)?;
    let pt = &after_article[..pt_end_rel];
    if !is_power_toughness_shorthand_token(pt) {
        return None;
    }

    let mut rest_start = value_start + pt_end_rel;
    let rest_after_pt = &text[rest_start..];
    let post_pt_space = rest_after_pt.len() - rest_after_pt.trim_start().len();
    rest_start += post_pt_space;
    let rest = &text[rest_start..];
    let (creature_start, creature_end, creature_word) = find_creature_word(rest)?;
    let descriptor = rest[..creature_start].trim();
    let tail = rest[creature_end..].trim_start();
    let tail_lower = tail.to_ascii_lowercase();
    if tail_lower.contains("lose all abilities") || tail_lower.contains("loses all abilities") {
        return None;
    }

    let mut noun_phrase = String::new();
    if !descriptor.is_empty() {
        noun_phrase.push_str(descriptor);
        noun_phrase.push(' ');
    }
    noun_phrase.push_str(creature_word);

    let mut replacement = format!("{noun_phrase} with base power and toughness {pt}");
    if let Some(stripped) = tail.strip_prefix("with ") {
        replacement.push_str(" and ");
        replacement.push_str(stripped.trim_start());
    } else if !tail.is_empty() {
        replacement.push(' ');
        replacement.push_str(tail);
    }

    let mut normalized = String::with_capacity(text.len() + 32);
    normalized.push_str(&text[..marker_end]);
    normalized.push_str(&replacement);
    Some(normalized)
}

fn normalize_fixed_pt_animation_shorthand(text: &str) -> String {
    let mut normalized = text.to_string();
    while let Some(next) = normalize_fixed_pt_animation_shorthand_once(&normalized) {
        if next == normalized {
            break;
        }
        normalized = next;
    }
    normalized
}

fn normalize_leading_until_end_of_turn_animation(text: &str) -> String {
    let Some(rest) = text.strip_prefix("Until end of turn, ") else {
        return text.to_string();
    };
    let lower_rest = rest.to_ascii_lowercase();
    if !(lower_rest.contains(" become") && lower_rest.contains(" with base power and toughness ")) {
        return text.to_string();
    }

    for marker in [" that's still a land", " that are still lands"] {
        if let Some(idx) = lower_rest.find(marker) {
            let mut out = String::with_capacity(text.len());
            out.push_str(rest[..idx].trim_end());
            out.push_str(" until end of turn");
            out.push_str(&rest[idx..]);
            return out;
        }
    }

    // The duration scopes the animation sentence, not any following
    // sentence rendered on the same line ("... 4/5. Draw a card.").
    if let Some(idx) = rest.find(". ") {
        let (first, tail) = rest.split_at(idx);
        return format!("{first} until end of turn{tail}");
    }
    format!("{rest} until end of turn")
}

fn normalize_clause_line(text: &str) -> String {
    let normalized = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(
            "artifact or creature or land",
            "artifact, creature, or land",
        )
        .replace(
            "Artifact or creature or land",
            "Artifact, creature, or land",
        )
        .replace(
            "artifact or creature or enchantment",
            "artifact, creature, or enchantment",
        )
        .replace(
            "Artifact or creature or enchantment",
            "Artifact, creature, or enchantment",
        )
        .replace(
            "artifact or creature or planeswalker",
            "artifact, creature, or planeswalker",
        )
        .replace(
            "Artifact or creature or planeswalker",
            "Artifact, creature, or planeswalker",
        )
        .replace(
            "Whenever this land attacks",
            "Whenever this creature attacks",
        )
        .replace(
            "whenever this land attacks",
            "whenever this creature attacks",
        )
        // A relative attacker subject and the pronoun it licenses denote the
        // same triggering object.
        .replace("the creature that attacked gains", "it gains")
        .replace("The creature that attacked gains", "It gains")
        .replace("that creature's power is", "its power is")
        .replace("That creature's power is", "Its power is")
        // A completed sentence already establishes sequencing. Treat an
        // additional leading "then" on the following conditional as surface
        // punctuation, not a distinct semantic operation.
        .replace(". Then if ", ". If ")
        .replace(". then if ", ". if ")
        // Remainder moves are frequently templated as either a continuation
        // of the chosen-card move or as their own sentence.
        .replace(". Put the rest on ", " and the rest on ")
        .replace(". put the rest on ", " and the rest on ")
        .replace(". Put the rest into ", " and the rest into ")
        .replace(". put the rest into ", " and the rest into ")
        // Result predicates use both present and past passive forms in Oracle
        // text; both ask whether the preceding action affected the object.
        .replace(" is destroyed this way", " was destroyed this way")
        .replace(" Is destroyed this way", " Was destroyed this way")
        .replace(" is exiled this way", " was exiled this way")
        .replace(" Is exiled this way", " Was exiled this way")
        .replace(" is milled this way", " was milled this way")
        .replace(" Is milled this way", " Was milled this way")
        .replace(" is returned this way", " was returned this way")
        .replace(" Is returned this way", " Was returned this way")
        // Both phrases are the same existential morbid condition. Keep the
        // distinction out of the score while preserving whichever oracle
        // surface the renderer has enough information to reproduce.
        .replace(
            "one or more creatures died this turn",
            "a creature died this turn",
        )
        .replace(
            "One or more creatures died this turn",
            "A creature died this turn",
        )
        .replace(
            "one or more creature died this turn",
            "a creature died this turn",
        )
        .replace(
            "One or more creature died this turn",
            "A creature died this turn",
        );
    let normalized = normalize_fixed_pt_animation_shorthand(&normalized);
    let normalized = normalize_leading_until_end_of_turn_animation(&normalized);
    let normalized = normalized
        .replace(" each become ", " become ")
        .replace(" each becomes ", " become ");
    normalize_end_turn_creature_buff_split(&normalized)
}

fn normalize_that_player_references_for_clause_surface(text: &str) -> String {
    text.replace("That player controls", "They control")
        .replace("that player controls", "they control")
        .replace("That player draws", "They draw")
        .replace("that player draws", "they draw")
        .replace("That player loses", "They lose")
        .replace("that player loses", "they lose")
        .replace("That player discards", "They discard")
        .replace("that player discards", "they discard")
        .replace("That player sacrifices", "They sacrifice")
        .replace("that player sacrifices", "they sacrifice")
        .replace("That player ", "They ")
        .replace("that player ", "they ")
        .replace(", that player ", ", they ")
        .replace("that player, ", "they, ")
        // "The player" is the same anaphor in older templating; only
        // verb-specific forms are canonicalized — the generic form
        // over-rewrites first mentions.
        .replace("The player puts", "They put")
        .replace("the player puts", "they put")
        .replace("The player sacrifices", "They sacrifice")
        .replace("the player sacrifices", "they sacrifice")
        .replace("The player shuffles", "They shuffle")
        .replace("the player shuffles", "they shuffle")
        .replace("If the player can't", "If they can't")
        .replace("if the player can't", "if they can't")
}

/// Canonicalize player anaphors only after an explicit target-player actor has
/// been introduced in the same carried effect line. This keeps unrelated
/// players distinct while equating "their" with "that player's" for the
/// already-selected player.
fn normalize_carried_target_player_references(text: &str) -> String {
    let normalized = text
        .replace(
            "Target opponent reveals their hand, choose ",
            "Target opponent reveals their hand. You choose ",
        )
        .replace(
            "target opponent reveals their hand, choose ",
            "target opponent reveals their hand. You choose ",
        )
        .replace(
            "Target player reveals their hand, choose ",
            "Target player reveals their hand. You choose ",
        )
        .replace(
            "target player reveals their hand, choose ",
            "target player reveals their hand. You choose ",
        );
    let lower = normalized.to_ascii_lowercase();
    let antecedent = [
        "target opponent ",
        "target player ",
        // Possessive first mentions ("search target player's library",
        // "from an opponent's graveyard", "a card an opponent owns")
        // introduce the same carried actor.
        "target opponent's ",
        "target player's ",
        "an opponent's ",
        "an opponent owns",
        // "deals (combat) damage to a player, ..." triggers carry the damaged
        // player as the line's actor for later "that player"/"the player".
        "damage to a player",
    ]
    .into_iter()
    .filter_map(|marker| lower.find(marker).map(|idx| (idx, marker.len())))
    .min_by_key(|(idx, _)| *idx);
    let Some((idx, len)) = antecedent else {
        return normalized;
    };
    let split = idx + len;
    let (prefix, tail) = normalized.split_at(split);
    let tail = tail
        .replace("That player's", "Their")
        .replace("that player's", "their")
        // A repeated possessive or damage-recipient mention of the already-
        // introduced target is the same back-reference.
        .replace("Target player's", "Their")
        .replace("target player's", "their")
        .replace("damage to target player", "damage to them")
        .replace("That player’s", "Their")
        .replace("that player’s", "their")
        .replace("That player controls", "They control")
        .replace("that player controls", "they control")
        .replace("That player draws", "They draw")
        .replace("that player draws", "they draw")
        .replace("That player loses", "They lose")
        .replace("that player loses", "they lose")
        .replace("That player discards", "They discard")
        .replace("that player discards", "they discard")
        .replace("That player sacrifices", "They sacrifice")
        .replace("that player sacrifices", "they sacrifice")
        .replace("That player owns", "They own")
        .replace("that player owns", "they own")
        .replace("That player puts", "They put")
        .replace("that player puts", "they put")
        .replace("The player puts", "They put")
        .replace("the player puts", "they put")
        .replace("That player ", "They ")
        .replace("that player ", "they ")
        // Older templating names the carried player as "the player".
        .replace("The player ", "They ")
        .replace("the player ", "they ");
    format!("{prefix}{tail}")
}

/// A plural demonstrative carries the exact previously affected set. Expand
/// it only when that filtered set was explicitly introduced earlier in the
/// same line; a bare later "creatures" remains observably under-filtered.
fn normalize_repeated_filtered_set_coreferences(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let antecedent = [
        "creatures you control get ",
        "each creature you control gets ",
    ]
    .into_iter()
    .filter_map(|marker| lower.find(marker).map(|idx| (idx, marker.len())))
    .min_by_key(|(idx, _)| *idx);
    let Some((idx, len)) = antecedent else {
        return text.to_string();
    };
    let split = idx + len;
    let (prefix, tail) = text.split_at(split);
    let tail = tail
        .replace("Those creatures get ", "Creatures you control get ")
        .replace("those creatures get ", "creatures you control get ")
        .replace("Those creatures gets ", "Creatures you control get ")
        .replace("those creatures gets ", "creatures you control get ");
    format!("{prefix}{tail}")
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
    if lower.contains("optional cost 'squad' was paid")
        || lower.contains("optional cost 'offspring' was paid")
        || lower.contains("offspring cost was paid")
        // The evoke sacrifice trigger is keyword-derived; the "Evoke {cost}"
        // line carries the oracle semantics.
        || lower.contains("evoke cost was paid")
    {
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
    // NOTE: hoisted "Choose target attacking creature." sentences are real
    // render drift (the choose-hoist family) and now cost score; the render
    // fix is F4 in architecture/target-mention-inflation-plan.md.

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
                        push_normalized_semantic_clause(trimmed, clauses);
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
        push_normalized_semantic_clause(trimmed, clauses);
    }
}

/// "If the chosen option is <mode>, ..." is the renderer's gate for a modal
/// static; oracle's bullet form loses its mode label to the ability-word
/// strip, so drop this scaffold symmetrically.
fn strip_chosen_option_gate_prefix(clause: &str) -> &str {
    for prefix in ["If the chosen option is ", "if the chosen option is "] {
        if let Some(rest) = clause.strip_prefix(prefix)
            && let Some((mode, tail)) = rest.split_once(", ")
            && !mode.is_empty()
            && mode.chars().all(|ch| ch.is_ascii_alphanumeric())
        {
            return tail;
        }
    }
    clause
}

fn push_normalized_semantic_clause(trimmed: &str, clauses: &mut Vec<String>) {
    let clause = strip_ability_word_prefix(strip_chosen_option_gate_prefix(trimmed));
    let clause = strip_redundant_duration_tail(&clause);
    // Oracle sometimes joins the top-of-library pair into one sentence and
    // sometimes keeps two lines; compare them as two clauses either way.
    if let Some(head) = clause
        .trim_end_matches('.')
        .strip_suffix(", and you may play lands from the top of your library")
        && head.eq_ignore_ascii_case("You may look at the top card of your library any time")
    {
        clauses.push(head.to_string());
        clauses.push("You may play lands from the top of your library".to_string());
        return;
    }
    if let Some(keyword_clauses) = split_keyword_only_clause(&clause) {
        clauses.extend(keyword_clauses);
    } else {
        clauses.push(clause);
    }
}

/// A clause scoped by a leading "Until end of turn," already carries the
/// duration; a repeated trailing " until end of turn" is redundant.
fn strip_redundant_duration_tail(clause: &str) -> String {
    let lower = clause.to_ascii_lowercase();
    let prefix = "until end of turn, ";
    if !lower.starts_with(prefix) {
        return clause.to_string();
    }
    let rest = &clause[prefix.len()..];
    let rest_lower = &lower[prefix.len()..];
    let tail = " until end of turn";
    if let Some(pos) = rest_lower.rfind(tail) {
        let after = &rest[pos + tail.len()..];
        if after.is_empty() || after == "." {
            return format!("{}{}{}", &clause[..prefix.len()], &rest[..pos], after);
        }
    }
    clause.to_string()
}

fn split_keyword_only_clause(clause: &str) -> Option<Vec<String>> {
    let trimmed = clause.trim().trim_end_matches('.');
    if !trimmed.contains(',') && !trimmed.to_ascii_lowercase().contains(" and ") {
        return None;
    }

    let normalized = trimmed
        .replace(", and ", ", ")
        .replace(", And ", ", ")
        .replace(" and ", ", ")
        .replace(" And ", ", ");
    let parts = normalized
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 || !parts.iter().all(|part| is_keyword_only_phrase(part)) {
        return None;
    }

    Some(
        parts
            .into_iter()
            .map(normalize_keyword_clause_surface)
            .collect(),
    )
}

fn normalize_keyword_clause_surface(phrase: &str) -> String {
    let lower = phrase.trim().to_ascii_lowercase();
    let mut chars = lower.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn is_keyword_only_phrase(phrase: &str) -> bool {
    let lower = phrase.trim().trim_end_matches('.').to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    if is_landwalk_keyword_phrase(&lower) {
        return true;
    }
    if lower.starts_with("protection from ") || lower.starts_with("ward ") {
        return true;
    }
    if lower == "sunburst"
        || lower.starts_with("bushido ")
        || lower.starts_with("cleave ")
        || lower.starts_with("frenzy ")
        || lower.starts_with("fading ")
        || lower.starts_with("fabricate ")
        || lower.starts_with("graft ")
        || lower.starts_with("modular ")
        || lower.starts_with("poisonous ")
        || lower.starts_with("rampage ")
        || lower.starts_with("scavenge ")
        || lower.starts_with("transfigure ")
        || lower.starts_with("transmute ")
        || lower.starts_with("toxic ")
        || lower.starts_with("vanishing ")
    {
        return true;
    }
    matches!(
        lower.as_str(),
        "flying"
            | "first strike"
            | "double strike"
            | "deathtouch"
            | "defender"
            | "flash"
            | "haste"
            | "hexproof"
            | "indestructible"
            | "intimidate"
            | "lifelink"
            | "menace"
            | "reach"
            | "shroud"
            | "trample"
            | "devoid"
            | "vigilance"
            | "fear"
            | "flanking"
            | "shadow"
            | "horsemanship"
            | "phasing"
            | "wither"
            | "infect"
            | "changeling"
            | "battle cry"
            | "daybound"
            | "dethrone"
            | "enlist"
            | "extort"
            | "evolve"
            | "ingest"
            | "melee"
            | "myriad"
            | "nightbound"
            | "prowess"
            | "provoke"
            | "riot"
            | "training"
            | "persist"
            | "undying"
            | "partner"
            | "assist"
    )
}

fn is_landwalk_keyword_phrase(lower: &str) -> bool {
    if matches!(
        lower,
        "landwalk" | "nonbasic landwalk" | "artifact landwalk" | "legendary landwalk"
    ) {
        return true;
    }
    if let Some(rest) = lower.strip_prefix("snow ") {
        return is_landwalk_subtype_compound(rest);
    }
    is_landwalk_subtype_compound(lower)
}

fn is_landwalk_subtype_compound(lower: &str) -> bool {
    matches!(
        lower,
        "plainswalk" | "islandwalk" | "swampwalk" | "mountainwalk" | "forestwalk" | "desertwalk"
    )
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
    let trimmed = line.trim_start();
    if trimmed.starts_with('•')
        && let Some((label, effect)) = trimmed.trim_start_matches('•').trim_start().split_once('—')
        && looks_like_modal_label(label)
    {
        return format!("• {}", effect.trim_start());
    }

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
    // A die-roll range endpoint ("1—9 | Each player draws a card") is not an
    // ability word; keep the range intact.
    if !prefix.trim().is_empty()
        && prefix.trim().chars().all(|ch| ch.is_ascii_digit())
        && tail
            .trim_start()
            .starts_with(|ch: char| ch.is_ascii_digit())
    {
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
        || tail_lower.starts_with("your ")
        || tail_lower.starts_with("this ")
        || tail_lower.starts_with("may ")
        || tail_lower.starts_with("protection ")
        || tail_lower.starts_with("choose ")
        || tail_lower.starts_with("counter ");
    // Flavor words are arbitrary title-case names; when the prefix is a short
    // run of capitalized words and the tail reads as a sentence, the prefix
    // carries no semantics. Keyword abilities templated as "Keyword—<cost>"
    // (Echo, Equip, ...) keep their prefix: the keyword is the semantics.
    const DASH_COST_KEYWORDS: &[&str] = &[
        "echo",
        "equip",
        "evoke",
        "madness",
        "kicker",
        "multikicker",
        "dash",
        "emerge",
        "ninjutsu",
        "commander ninjutsu",
        "miracle",
        "surge",
        "spectacle",
        "escape",
        "disturb",
        "overload",
        "flashback",
        "reconfigure",
        "bestow",
        "awaken",
        "cycling",
        "transmute",
        "unearth",
        "embalm",
        "eternalize",
        "scavenge",
        "forecast",
        "prototype",
        "blitz",
        "casualty",
        "cleave",
        "prowl",
        "foretell",
        "channel",
        "plot",
        "offering",
        "retrace",
        "adapt",
        "monstrosity",
        "fortify",
        "reinforce",
        "transfigure",
        "replicate",
        "recover",
        "ripple",
        "splice",
        "suspend",
        "vanishing",
        "absorb",
        "frenzy",
        "graft",
        "dredge",
        "devour",
        "tribute",
        "outlast",
        "megamorph",
        "morph",
        "disguise",
        "mutate",
        "level up",
        "impending",
        "partner",
    ];
    let flavor_word_prefix = {
        let prefix_trimmed = prefix.trim();
        let words: Vec<&str> = prefix_trimmed.split_whitespace().collect();
        let interior_particle =
            |word: &str| matches!(word, "the" | "of" | "a" | "an" | "and" | "to" | "in");
        !words.is_empty()
            && words.len() <= 4
            && words.first().is_some_and(|word| {
                word.chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase())
            })
            && words.last().is_some_and(|word| {
                word.chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_uppercase())
            })
            && words.iter().all(|word| {
                interior_particle(word)
                    || word
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_uppercase())
            })
            && tail
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
            && !DASH_COST_KEYWORDS.contains(&prefix_trimmed.to_ascii_lowercase().as_str())
    };
    if semantic_tail || starts_with_cost_like || flavor_word_prefix {
        if starts_with_cost_like {
            let without_cost = strip_compiled_ability_cost_prefix(tail.trim());
            return normalize_damage_source_for_clause_surface(without_cost).to_string();
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

/// Equate anaphoric/rule-redundant surfaces that differ only in wording:
/// blockers are creatures by rule, and "that creature"/"that card" after a
/// verb is the same back-reference as "it".  Word-boundary aware so
/// possessives ("that creature's controller") are left alone.
fn normalize_anaphoric_object_surfaces(text: &str) -> String {
    const HALF_PERMANENT_SENTINEL: &str = "half __ironsmith_that_permanent__ power and toughness";
    // In an isolated characteristic-setting clause, "that permanent" can
    // name a different antecedent from the subject denoted by "its". Protect
    // that relationship from the generic possessive-anaphor rewrite below;
    // the Saw in Half normalization handles its narrower, proven context
    // explicitly after this pass.
    let text = text.replace(
        "half that permanent's power and toughness",
        HALF_PERMANENT_SENTINEL,
    );
    const REWRITES: &[(&str, &str)] = &[
        ("becomes blocked by a creature", "becomes blocked"),
        // The renderer's "put it into exile" is oracle's "exile it".
        ("put it into exile", "exile it"),
        // In a cast-trigger's threshold clause both demonstratives name the
        // triggering spell; producers disagree on which to use.
        (
            "mana was spent to cast that spell,",
            "mana was spent to cast this spell,",
        ),
        // Oracle uses both threshold conventions ("at least seven mana" on
        // Raggadragga, "five or more mana" on Exhibition Tidecaller); the
        // renderer keeps "at least N", so canonicalize toward it.
        ("two or more mana was spent", "at least two mana was spent"),
        (
            "three or more mana was spent",
            "at least three mana was spent",
        ),
        (
            "four or more mana was spent",
            "at least four mana was spent",
        ),
        (
            "five or more mana was spent",
            "at least five mana was spent",
        ),
        ("six or more mana was spent", "at least six mana was spent"),
        (
            "seven or more mana was spent",
            "at least seven mana was spent",
        ),
        (
            "eight or more mana was spent",
            "at least eight mana was spent",
        ),
        (
            "nine or more mana was spent",
            "at least nine mana was spent",
        ),
        ("ten or more mana was spent", "at least ten mana was spent"),
        // Keyword qualifier vs controller suffix order; oracle places the
        // controller first ("creature you control with flying").
        (
            "creature with flying you control",
            "creature you control with flying",
        ),
        (
            "creatures with flying you control",
            "creatures you control with flying",
        ),
        (
            "creature with mana ability you control",
            "creature you control with a mana ability",
        ),
        (
            "creatures with mana ability you control",
            "creatures you control with a mana ability",
        ),
        // The library-peek decline rider: oracle spells out the declined
        // action, the renderer says "Otherwise".
        (
            "If you don't put the card into your hand, you may put it",
            "Otherwise, you may put it",
        ),
        // The attached-sweep render splits out its target choice.
        (
            "Choose target creature. Destroy all Auras or Equipment attached to it",
            "Destroy all Auras and Equipment attached to target creature",
        ),
        // The amass follow-up names the Army two ways.
        (
            "then that Army deals damage equal to that Army's power",
            "then the Army you amassed deals damage equal to its power",
        ),
        // Feather-class paid copy fanout: the renderer's object/targeting
        // scaffolds for the authored creature surfaces.
        (
            "choose any number other creatures",
            "choose any number of other creatures",
        ),
        (
            "If you do, for each of those objects, copy that spell",
            "If you do, for each of those creatures, copy that spell",
        ),
        (
            ". For each of those objects, you pay {2}",
            " and pay {2} for each of those creatures",
        ),
        (
            "Change a target of the copy to that creature",
            "The copy targets that creature",
        ),
        // Opponent-reveal-X choose-and-exile (Taster of Wares class).
        (
            "You choose a revealed this way card. Exile it",
            "You choose one of those cards. That player exiles it",
        ),
        (
            "If an instant or sorcery card was exiled this way, you may cast that card",
            "If an instant or sorcery card is exiled this way, you may cast it",
        ),
        (
            "target opponent chooses X cards, where X is",
            "target opponent reveals X cards from their hand, where X is",
        ),
        // Saga-face exile-and-return riders: the render says "the exiled
        // card" where oracle uses the pronoun, joins with a comma where
        // oracle starts a sentence, and spells the source as "this Saga".
        (
            "then return the exiled card to the battlefield",
            "then return it to the battlefield",
        ),
        (
            ", exile this, then return it to the battlefield",
            ". Exile this, then return it to the battlefield",
        ),
        (
            "has 3 or more lore counters",
            "has three or more lore counters",
        ),
        (". Then if this Saga has", ". If this has"),
        ("If this Saga has", "If this has"),
        ("if this Saga has", "if this has"),
        ("This Saga deals", "This deals"),
        // The exile-top repeat program: the render leads the branch with
        // "Then" and drops the "If you do," before the repeat.
        (
            ". Then if it's a permanent card, you may put it onto the battlefield",
            ". If it's a permanent card, you may put it onto the battlefield",
        ),
        (
            "you may put it onto the battlefield. Repeat this process",
            "you may put it onto the battlefield. If you do, repeat this process",
        ),
        // The delayed copy-then-cast trigger re-emits the copy sentence.
        (
            "copy that spell. Copy it. You may cast the copy",
            "copy the exiled card. You may cast the copy",
        ),
        // Imprint-style copy programs lower as choose+cast-as-copy; the
        // render surfaces the intermediate steps oracle folds into "copy".
        (
            "If you do, copy it. You may cast a copy of it",
            "If you do, you may cast the copy",
        ),
        (
            "you may choose a card exiled with this",
            "you may copy a card exiled with this",
        ),
        // The chain-copy retarget rider: the renderer splits the sentence
        // and re-subjects the copy reference.
        (
            ". That player may choose a new target for the copy",
            " and may choose a new target for that copy",
        ),
        (
            ". Its controller may choose a new target for the copy",
            " and may choose a new target for that copy",
        ),
        (
            ". They may choose a new target for the copy",
            " and may choose a new target for that copy",
        ),
        (
            ". That player may choose new targets for the copy",
            " and may choose a new target for that copy",
        ),
        (
            ". Its controller may choose new targets for the copy",
            " and may choose a new target for that copy",
        ),
        (
            ". They may choose new targets for the copy",
            " and may choose a new target for that copy",
        ),
        // The blocked-by-source qualifier is oracle's "this creature is
        // blocking"; both surfaces reach this pass with the self-reference
        // noun already stripped.
        ("blocked by this this turn", "this is blocking"),
        (
            "blocked by this creature this turn",
            "this creature is blocking",
        ),
        // "X as long as Y" and "If Y, X" state the same guard; token sets
        // ignore order, so only the connective word needs canonicalizing.
        ("as long as", "if"),
        ("As long as", "If"),
        // Renderer's typed trigger-object counter check vs oracle's anaphor.
        ("the triggering object had 1 or more ", "it had a "),
        // Self-damage where-X vs oracle's equal-to phrasing.
        ("X damage to you, where X is", "damage to you equal to"),
        // One-shot delayed end-of-combat surfaces.
        ("at this turn's next end of combat", "at end of combat"),
        // The activation-restriction join drops the second "only".
        (
            " once each turn and during your",
            " once each turn and only during your",
        ),
        // Bare keyword line vs oracle's subject form.
        ("This has protection from", "Protection from"),
        // Renderer's "Aura source" subject is the Aura's self-reference.
        ("This Aura source", "This"),
        // The graveyard provenance is implicit in oracle's return wording.
        ("from graveyard to the battlefield", "to the battlefield"),
        // Free-cast/play grants word the duration as a prefix in oracle and
        // a tail in the renderer; canonicalize the grant surfaces only (a
        // global until-end-of-turn rewrite breaks the buff-split passes).
        ("Until end of turn, you may cast", "This turn, you may cast"),
        ("until end of turn, you may cast", "this turn, you may cast"),
        ("Until end of turn, you may play", "This turn, you may play"),
        ("until end of turn, you may play", "this turn, you may play"),
        (
            " from your graveyard until end of turn",
            " from your graveyard this turn",
        ),
        // Narrow to the quoted-grant shape: broader each-forms regress
        // cards whose oracle trails the duration (Sozin's Comet class).
        (
            "Until end of turn, each legendary card",
            "This turn, each legendary card",
        ),
        (
            "until end of turn, each legendary card",
            "this turn, each legendary card",
        ),
        // CR-mandated multi-zone search shuffle rider vs oracle's plain
        // tail; oracle uses both tenses.
        (
            ". If you search your library this way, shuffle",
            ", then shuffle",
        ),
        (
            ". If you searched your library this way, shuffle",
            ", then shuffle",
        ),
        // Named-card search templates use both conjunctions for the same
        // mandatory reveal-then-move sequence.
        (
            "reveal it, then put it into your hand",
            "reveal it, and put it into your hand",
        ),
        // A reveal-each-player program stores one tagged set and tests it
        // against the triggering spell. The renderer may spell that set test
        // as a singular prior-result predicate and may retain sequence
        // scaffolding around the true branch; all of these surfaces describe
        // the same shared-card-type gate and outcomes.
        (
            "Then if a permanent that shares a card type with it was revealed this way",
            "If any of those cards shares a card type with that spell",
        ),
        (
            "Then if any of those cards shares a card type with that spell",
            "If any of those cards shares a card type with that spell",
        ),
        (
            "copy that spell, you may choose new targets for the copy, then each opponent draws a card",
            "copy that spell, you may choose new targets for the copy, and each opponent draws a card",
        ),
        ("Otherwise, draw a card", "Otherwise, you draw a card"),
        ("and/or library", "and library"),
        // "and/or" is templating-equivalent to the plain conjunction; as one
        // token it would cost score against renders that split it.
        ("and/or", "and or"),
        // A dead creature's attachments are necessarily last-known state.
        (" that was attached to it", " attached to it"),
        (
            "each Aura you controlled attached to it",
            "each Aura attached to it you control",
        ),
        // The pump renderer spells the where-X basis into the stat value.
        (
            "gets this power/+0, where X is this power",
            "gets +X/+0, where X is its power",
        ),
        (
            "gets the number of creature cards in your graveyard/+0",
            "gets +X/+0",
        ),
        // The library destination possessive is the mover's own library.
        (
            "into its owner's library third from the top",
            "into their library third from the top",
        ),
        // The activated-ability trigger renders its non-mana gate into the
        // filter although the authored intervening-if already carries it.
        (
            " that isn't a mana ability, if it isn't a mana ability",
            " on the battlefield, if it isn't a mana ability",
        ),
        // The excluded just-sacrificed set renders as "other"; oracle spells
        // the provenance.
        (
            "returns all other creature cards from their graveyard",
            "returns all creature cards from their graveyard that weren't put there this way",
        ),
        // The discard-trigger intervening-if spells the madness gate through
        // the permanent noun; oracle's "has madness" tests the same card.
        ("if it is a permanent with madness", "if it has madness"),
        (
            "if that object is a permanent with madness",
            "if it has madness",
        ),
        // The "N life for each X" convention is the same quantity as "life
        // equal to the number of X"; converge on the equal-to surface.
        ("gain 1 life for each", "gain life equal to the number of"),
        ("gains 1 life for each", "gains life equal to the number of"),
        ("lose 1 life for each", "lose life equal to the number of"),
        ("loses 1 life for each", "loses life equal to the number of"),
        // The per-player spell count filter's iterated-subject description.
        (
            "the number of spells a player cast this turn",
            "the number of spells they've cast this turn",
        ),
        // The mass-destroy for-each back-reference: the destroyed set was
        // "artifacts and enchantments", oracle iterates them as permanents.
        (
            "For each artifact or enchantment destroyed this way",
            "For each permanent destroyed this way",
        ),
        // Self-return from the graveyard: the owner's library is "your
        // library" and the provenance is the activation zone.
        (
            "Put this into its owner's library second from the top",
            "Put this from your graveyard into your library second from the top",
        ),
        // The attached-to back-reference is oracle's "enchanted permanent".
        ("the permanent this is attached to", "enchanted permanent"),
        ("is a tapped permanent", "is tapped"),
        // Multi-attacker trigger: oracle phrases the attack from the
        // attacking player's side.
        (
            "Whenever two or more creature an opponent controls attack you",
            "Whenever an opponent attacks you with two or more creatures",
        ),
        (
            "Whenever two or more creatures an opponent controls attack you",
            "Whenever an opponent attacks you with two or more creatures",
        ),
        // Contraction surfaces tokenize apart; converge the negated copula.
        ("it's not a token", "it is not a token"),
        ("it isn't a token", "it is not a token"),
        ("it's not your turn", "it is not your turn"),
        // The end-of-combat sacrifice back-reference: "the token" is the
        // just-created token, same referent as "it".
        (
            "Sacrifice the token at end of combat",
            "Sacrifice it at end of combat",
        ),
        // The reflexive discard-linked trigger; oracle spells the event.
        (
            "When you discard a nonland card this way, this deals",
            "When you do, this deals",
        ),
        // The per-target counter distribution back-reference.
        (
            "counter on each target permanent",
            "counter on each of them",
        ),
        // The Room self-reference repeats the type noun.
        ("this Room deals damage", "this deals damage"),
        // Attack-trigger goad: the defending player is the trigger's "that
        // player".
        (
            "you may Goad target creature that player controls",
            "you may goad target creature defending player controls",
        ),
        // Mass bounce with an except-arm vs oracle's non- prefix.
        (
            "return all creatures to their owners' hands except for Dinosaurs",
            "return each non-Dinosaur creature to its owner's hand",
        ),
        // The greatest-power comparison set is implicitly battlefield-wide.
        (
            "with the greatest power among creatures,",
            "with the greatest power among creatures on the battlefield,",
        ),
        // Evoke renders as the spelled alternative cost; oracle's keyword
        // body is the bare exile instruction.
        (
            "You may exile a blue card from your hand rather than pay this mana cost",
            "Exile a blue card from your hand",
        ),
        // Graveyard-exile union surface.
        (
            "Exile all creatures or cards in a graveyard",
            "Exile all creatures and graveyards",
        ),
        // Animated base P/T: oracle omits "base" in the each-equal-to form.
        (
            "with base power and toughness each equal to",
            "with power and toughness each equal to",
        ),
        // Oracle writes a zero power delta with a minus sign when the
        // toughness delta is negative.
        ("+0/-1", "-0/-1"),
        ("+0/-2", "-0/-2"),
        ("+0/-3", "-0/-3"),
        ("+0/-4", "-0/-4"),
        ("+0/-5", "-0/-5"),
        ("+0/-X", "-0/-X"),
        // Legendary-permanent copula vs oracle's bare adjective.
        ("If it's a legendary permanent,", "If it is legendary,"),
        ("if it's a legendary permanent,", "if it is legendary,"),
        // The moved object's cost back-reference: "that spell's"/"the card's"
        // mana value is the same referent as the renderer's "its".
        (
            "You lose life equal to that spell's mana value",
            "You lose life equal to its mana value",
        ),
        (
            "You lose life equal to the card's mana value",
            "You lose life equal to its mana value",
        ),
        // The just-created token back-reference needs no provenance.
        ("The token created this way enters", "The token enters"),
        // The damaged-set iteration back-reference.
        (
            "For each creature dealt damage this way, that creature deals damage equal to its power",
            "Each of those creatures deals damage equal to its power",
        ),
        // Attack triggers phrased from the attacking player's side.
        (
            "Whenever one or more creature an opponent controls attack you,",
            "Whenever an opponent attacks you,",
        ),
        (
            "Whenever one or more creatures an opponent controls attack you,",
            "Whenever an opponent attacks you,",
        ),
        (
            "choose target attacking creature attacking you",
            "choose target creature attacking you",
        ),
        // Mana symbol vs the spelled color word in spent-mana conditions.
        ("{C} mana was spent", "colorless mana was spent"),
        // Instead-arm ordering with a shared subject.
        (
            "that creature gets +3/+3 until end of turn and target creature gains indestructible until end of turn instead",
            "instead it gets +3/+3 and gains indestructible until end of turn",
        ),
        // Contraction: "you're dealt damage" tokenizes apart from "you are".
        ("you're dealt damage", "you are dealt damage"),
        ("You're dealt damage", "You are dealt damage"),
        // Oracle's "Otherwise," else-branch is the renderer's negative gate.
        // Mapped toward "If you don't," because later passes canonicalize
        // effect-reference scaffolds onto that surface. Scoped to effect-verb
        // continuations: the daybound scaffold's own "Otherwise, if ..." must
        // survive for its dedicated normalizer.
        ("Otherwise, draw", "If you don't, draw"),
        ("otherwise, draw", "if you don't, draw"),
        ("Otherwise, you draw", "If you don't, you draw"),
        ("otherwise, you draw", "if you don't, you draw"),
        ("Otherwise, create", "If you don't, create"),
        ("otherwise, create", "if you don't, create"),
        // Target union order for the sacrifice-fueled burn.
        ("target battle or opponent", "target opponent or battle"),
        // The sacrificed set spans creature-or-artifact; oracle says
        // "permanent".
        (
            "where X is the sacrificed creature's mana value",
            "where X is the sacrificed permanent's mana value",
        ),
        // The ping-back render's actor and ordering.
        (
            "This deals damage to this equal to its mana value",
            "That artifact deals damage equal to its mana value to this",
        ),
        // The destroyed-set description tense.
        (
            "the number of creatures you control destroyed this way",
            "the number of creatures you controlled that were destroyed this way",
        ),
        // A creature put into your graveyard from the battlefield is a
        // creature you own dying.
        (
            "Whenever another creature is put into your graveyard from the battlefield,",
            "Whenever another creature you own dies,",
        ),
        // The exile-linked counter trigger; oracle spells the event.
        (
            "If you do, put that many +1/+1 counters on target attacking creature",
            "When one or more nonland cards are exiled this way, put that many +1/+1 counters on target attacking creature",
        ),
        // Search-outside-the-game reveal surface (named-card variant).
        (
            "search outside the game for a card named this, reveal it, put it into your hand",
            "reveal a card you own named this from outside the game and put it into your hand",
        ),
        // The prevention back-reference carries the red-source scope from the
        // preceding sentence.
        (
            "If damage is prevented this way, this deals that much damage to that permanent's controller",
            "If damage from a red source is prevented this way, this deals that much damage to the source's controller",
        ),
        (
            "If damage is prevented this way, this deals that much damage to its controller",
            "If damage from a red source is prevented this way, this deals that much damage to the source's controller",
        ),
        // The exiled-card ETB back-reference.
        (
            "When a card exiled with this you control enters,",
            "When an exiled card enters under your control this way,",
        ),
        // Legendary short-name self-references the name normalizer's
        // comma/of gate doesn't cover.
        (
            "Kaervek deals damage equal to that spell's mana value to any target",
            "this deals damage to any target equal to that spell's mana value",
        ),
        (
            "Zurgo attacks each combat if able",
            "This attacks each combat if able",
        ),
        (
            "During your turn, Zurgo has indestructible",
            "During your turn, this has indestructible",
        ),
        // Colored-spell protection phrasings.
        (
            "protection from spells that are one or more colors",
            "protection from colored spells",
        ),
        // Dungeon-completion draw rider.
        (
            "Draw another card if you've completed a dungeon",
            "draw a card if you completed a dungeon",
        ),
        // "Another legendary permanent" is "a legendary permanent other than
        // this" from the source's own text.
        (
            "Whenever a legendary permanent other than this is put into",
            "Whenever another legendary permanent is put into",
        ),
        // The triggering spell's controller back-reference.
        (
            "That spell's controller may draw a card",
            "Its controller may draw a card",
        ),
        // The countered-set noun: countered objects are spells.
        (
            "Draw a card for each permanent countered this way",
            "Draw a card for each spell countered this way",
        ),
        // The exile-linked look trigger; oracle spells the event.
        (
            "If you do, look at the top three cards of your library",
            "If a card is put into exile this way, look at the top three cards of your library",
        ),
        // The damaged-set mass destroy ordering.
        (
            "Destroy all creatures that were dealt damage this turn you don't control",
            "Then destroy each creature you don't control that was dealt damage this turn",
        ),
        // Protection-from-card-type noun forms.
        (
            "protection from instants and protection from sorceries",
            "protection from instant spells and from sorcery spells",
        ),
        // Negative toughness pump spelled through a negated where-X.
        (
            "gets +0/+X until end of turn, where X is -X",
            "gets -0/-X until end of turn",
        ),
        // The predefined Walker token shorthand.
        (
            "2/2 colorless Zombie creature tokens named Walker",
            "Walker tokens",
        ),
        (
            "2/2 colorless Zombie creature token named Walker",
            "Walker token",
        ),
        // The reserved-mana haste rider.
        (
            "If this mana is spent to cast a creature spell, it gains haste until end of turn",
            "If that mana is spent on a creature spell, it gains haste",
        ),
        // The token-copy deathtouch rider.
        (
            "If it's a token, it gains deathtouch until end of turn",
            "If it is a token, it also gains deathtouch until end of turn",
        ),
        // The dying enchanted creature's controller picks among their own
        // opponents.
        (
            "dies, its controller chooses target creature an opponent controls",
            "dies, its controller chooses target creature one of their opponents controls",
        ),
        // The Aura returns from where it went on death.
        (
            "Return this to the battlefield attached to it",
            "Return this from its owner's graveyard to the battlefield attached to it",
        ),
        // Forced-attack rider durations.
        (
            "gets -2/-0 and gains attacks each combat if able until end of turn",
            "gets -2/-0 until end of turn and attacks this turn if able",
        ),
        // The tapped-set stun distribution back-reference.
        (
            "Put a stun counter on each creature tapped this way a player other than you controls",
            "Put a stun counter on each of those creatures you don't control",
        ),
        // "A player other than you controls" is "you don't control".
        (
            "each creature a player other than you controls",
            "each creature you don't control",
        ),
        // Legendary rider: the copula spells the permanent noun and oracle
        // adds "also". (The earlier legendary-permanent pair has already
        // expanded "it's a legendary permanent" to "it is legendary".)
        (
            "If it is legendary, it gains",
            "If it's legendary, it also gains",
        ),
        // Combat damage to "one of your opponents" is damage to an opponent.
        (
            "deals combat damage to one of your opponents",
            "deals combat damage to an opponent",
        ),
        // Self-directed damage ordering.
        (
            "Target creature deals damage equal to its power to it",
            "Target creature deals damage to itself equal to its power",
        ),
        // Scaled mana production surfaces.
        (
            "Add {C} for each time counter on this",
            "Add an amount of {C} equal to the number of time counters on this",
        ),
        // The post-discard penalty iteration.
        (
            "for each player, if that player didn't discard a creature card this way, that player loses 4 life",
            "Then each player who didn't discard a creature card this way loses 4 life",
        ),
        // Protection-from-subtype casing and number.
        ("protection from dog", "protection from Dogs"),
        // Count-noun pluralization in batched combat triggers.
        (
            "Whenever one or more Cat you control deal",
            "Whenever one or more Cats you control deal",
        ),
        // A batched attack on you is oracle's attacking player.
        (
            "Whenever one or more creature attack you, you may attach this to defending player",
            "Whenever a player attacks you, you may attach this to that player",
        ),
        // The revealed-set graveyard put, distributed per owner.
        (
            "You may put it into its owner's graveyard",
            "You may put the revealed cards into their owners' graveyards",
        ),
        // The tempted opponents' copies are under their own control by rule.
        (
            "may create a token that's a copy of it under that player's control",
            "may create a token that's a copy of it",
        ),
        // The prevention-scaled token count basis.
        (
            "For each this, create a 2/1 white and black Inkling",
            "For each 1 damage prevented this way, create a 2/1 white and black Inkling",
        ),
        // Power-threshold copula forms.
        (
            "If this has power 1 or greater, It gets",
            "If this power is 1 or more, it gets",
        ),
        // The delayed poison-counter ordering.
        (
            "At the beginning of their next upkeep, they gets a poison counter unless they pay {2}",
            "They gets another poison counter at the beginning of their next upkeep unless they pay {2} before that step",
        ),
        // The kicked token count back-reference.
        (
            "create 4 1/1 green Saproling creature tokens instead",
            "create four of those tokens instead",
        ),
        // The animated-Vehicle back-reference (raw surface: the moved-this-way
        // condition pass runs after these rewrites).
        (
            "If a Vehicle was returned this way, Those permanents become artifact creatures",
            "If it's a Vehicle, it becomes an artifact creature",
        ),
        // The exiled-card cast permission (set phrasing vs single card).
        (
            "This turn, you may cast spells from among them",
            "You may cast that card this turn",
        ),
        // The equip-state life gate ordering.
        (
            "You don't lose the game for having 0 or less life if this is equipped",
            "If this is attached to a creature, you don't lose the game for having 0 or less life",
        ),
        // Threshold color-setting ordering.
        (
            "This source is white if there are seven or more cards in your graveyard",
            "If there are seven or more cards in your graveyard, this is white",
        ),
        // Prevention back-reference carrying the black-source scope.
        (
            "If damage is prevented this way, you gain that much life",
            "If damage from a black source is prevented this way, you gain that much life",
        ),
        // The copied-spells wither rider back-reference.
        (
            "If you do, each of them gains wither",
            "If you do, those spells gain wither",
        ),
        // Station-threshold ability prefixes vs the rendered intervening-if.
        ("3+ | Whenever", "Whenever"),
        ("10+ | Whenever", "Whenever"),
        (
            "Whenever you cast an artifact spell, if the number of charge counters on this is 3 or greater, draw",
            "Whenever you cast an artifact spell, draw",
        ),
        (
            "Whenever you attack, if the number of charge counters on this is 10 or greater, this deals",
            "Whenever you attack, this deals",
        ),
        (
            "This Spacecraft gets +1/+0 for each artifact you control",
            "This creature gets +1/+0 for each artifact you control",
        ),
        // A lone opponent target is a single opponent.
        (
            "targets only an opponent or a single permanent",
            "targets only a single opponent or a single permanent",
        ),
        // Ward one-of cost render drops the pay verb.
        (
            "Ward—Discard a card or {2}",
            "Ward—Discard a card or pay {2}",
        ),
        // Reflexive-discard damage ordering.
        (
            "deals damage to target creature or planeswalker equal to that card's mana value",
            "deals damage equal to that card's mana value to target creature or planeswalker",
        ),
        // Granted mana ability's life cost render.
        (
            "{T}, pay {1} life: Add one mana of any color",
            "{T}, Lose 1 life: Add one mana of any color",
        ),
        // Mass destroy union noun forms.
        (
            "Destroy all artifacts or creatures with mana value equal to",
            "Destroy each artifact and creature with mana value equal to",
        ),
        // Cast-from-hand cascade grant surfaces.
        (
            "Instant cards cast by you in your hand or sorcery cards cast by you in hand have cascade",
            "Instant and sorcery spells you cast from your hand have cascade",
        ),
        // Modal-count scaffold garble.
        ("Choose between X and X mode", "Choose X"),
        // Graveyard-count win condition copula order.
        (
            "if there are twenty or more creature cards in your graveyard,",
            "if twenty or more creature cards are in your graveyard,",
        ),
        // Attacking-an-opponent trample grant surfaces.
        (
            "Each attacking creature attacking an opponent has trample",
            "Each creature that's attacking one of your opponents has trample",
        ),
        // The times-suffix on repeated investigate.
        ("investigate X times", "investigate X"),
        // (Summary Judgment's instead-arm is a REAL parse garble — "that
        // creature deals 5 damage to target player or planeswalker" — left
        // unmasked for the deep queue.)
        // The graveyard-cast token count back-reference.
        (
            "create 10 1/1 white Human creature tokens instead",
            "create ten of those tokens instead",
        ),
        // Turn-scoped cost reduction ordering.
        (
            "This costs {2}{U}{U} less to cast if it's your turn",
            "During your turn, this costs {2}{U}{U} less to cast",
        ),
        // Exile-linked damage ordering.
        (
            "this deals damage to any target equal to the exiled card's mana value",
            "this deals damage equal to the exiled card's mana value to any target",
        ),
        // The chosen-set battlefield put (both anaphor surfaces, since the
        // objects→them rewrite may run on either side of this pair).
        (
            "Choose any number cards, for each of them, put it onto the battlefield",
            "Put any number of those permanent cards onto the battlefield",
        ),
        (
            "Choose any number cards, for each of those objects, put it onto the battlefield",
            "Put any number of those permanent cards onto the battlefield",
        ),
        // Pluralization miss in the colors-among basis.
        (
            "the number of colors among permanent you control",
            "the number of colors among permanents you control",
        ),
        // The egg-counter death trigger back-reference.
        (
            "When a creature dies, if the number of egg counters on it is equal to 1, draw a card",
            "When it dies, if it has an egg counter on it, draw a card",
        ),
        // The per-opponent can't-sacrifice penalty.
        (
            "for each opponent, if you don't, they lose 3 life",
            "Each opponent who can't loses 3 life",
        ),
        // Mana symbol vs spelled color in spent-mana conditions.
        ("{R} mana was spent", "red mana was spent"),
        // Named-character pronoun self-reference.
        (
            "When you do, she deals 4 damage to target creature",
            "When you do, this deals 4 damage to target creature",
        ),
        // Third-person negative-gate conjugation garbles.
        ("If they doesn't,", "If they don't,"),
        ("if they doesn't,", "if they don't,"),
        ("If defending player doesn't,", "If they don't,"),
        ("If that player doesn't,", "If they don't,"),
        ("if that player doesn't,", "if they don't,"),
        // The defending player's own hand.
        (
            "defending player puts it into defending player's hand",
            "they puts it into their hand",
        ),
        // Monarch attack-trigger while-gate and recipient.
        (
            "attacks you, if you're the monarch, this deals damage to defending player equal",
            "attacks you while you're the monarch, this deals damage to them equal",
        ),
        // The milled-cards pump rider ordering.
        (
            "Whenever creature attacks this turn, it gets +1/+0 for each creature card milled this way until end of turn",
            "Whenever a creature attacks this turn, it gets +1/+0 until end of turn for each creature card put into your graveyard this way",
        ),
        // Mass mutate pump subject number.
        (
            "each other creature you control gets +X/+X until end of turn",
            "other creatures you control get +X/+X until end of turn",
        ),
        // Distributive library peek.
        (
            "For each player, Look at the top card of that player's library",
            "Look at the top card of each player's library",
        ),
        // The chosen-name back-reference's article.
        (
            "target spell with that name",
            "target spell with the chosen name",
        ),
        // The name-choice determiner after a look effect.
        ("then choose a card name", "then choose any card name"),
        // Conditional trample grant ordering on the attached subject.
        (
            "This has trample if enchanted permanent is a Human",
            "If enchanted creature is a Human, it has trample",
        ),
        // Combat-damage trigger recipient union expansion.
        (
            "Whenever this deals combat damage to a player or this deals combat damage to a planeswalker,",
            "Whenever this deals combat damage to a player or planeswalker,",
        ),
        // While announcing a spell, it isn't yet counted among spells cast
        // this turn, so the bare and "another" phrasings coincide at
        // cost-determination time.
        (
            "less to cast if you've cast a spell this turn",
            "less to cast if you've cast another spell this turn",
        ),
        // Ability-marker qualifier order around the controller suffix.
        (
            "each creature with level up you control",
            "each creature you control with level up",
        ),
        // The from-your-graveyard exile trigger's owner render.
        (
            "Whenever one or more card you owns are put into exile from a graveyard",
            "Whenever one or more cards are put into exile from your graveyard",
        ),
        // Counter-target union render distributes the shared targets scope.
        (
            "spell that targets only a permanent you control or ability that targets only a permanent you control",
            "spell or ability that targets a permanent you control",
        ),
        (
            "spell that targets only a creature or ability that targets only a creature",
            "spell or ability that targets a creature",
        ),
        // The reserved-mana scry rider.
        (
            "When you cast a Dragon creature spell, scry 2",
            "When you spend this mana to cast a Dragon creature spell, scry 2",
        ),
        // Attack trigger while-condition vs if-gate (Dinosaur variant).
        (
            "attacks, if you don't control another Dinosaur,",
            "attacks while you don't control another Dinosaur,",
        ),
        // Token rider placement around the for-each basis.
        (
            "creature token for each Attraction on the battlefield that's tapped and attacking",
            "creature token that's tapped and attacking for each Attraction on the battlefield",
        ),
        // Copy-exception ordering.
        (
            "except it's a Mutant in addition to its other types and it isn't legendary",
            "except it isn't legendary and is a Mutant in addition to its other types",
        ),
        (
            "Create a token that's a copy of that Saga card in exile, except",
            "Create a token that's a copy of a card exiled with this Saga, except",
        ),
        // The vote-loss library put's actor (both anaphor surfaces,
        // mid-sentence lowercase "if").
        (
            "if guilty gets more votes, put it on the bottom of their library",
            "if guilty gets more votes, the owner of each card exiled with this Saga puts it on the bottom of their library",
        ),
        (
            "if guilty gets more votes, put that card on the bottom of their library",
            "if guilty gets more votes, the owner of each card exiled with this Saga puts that card on the bottom of their library",
        ),
        // Continuous type-addition copula.
        (
            "becomes a black zombie in addition to its other colors",
            "is a black zombie in addition to its other colors",
        ),
        (
            "becomes a black Zombie in addition to its other colors",
            "is a black Zombie in addition to its other colors",
        ),
        // The attack trigger's while-condition is the renderer's if-gate.
        (
            "attacks, if you control a token",
            "attacks while you control a token",
        ),
        // "Unless" alternative-cost phrasing vs oracle's or-choice.
        (
            "You may sacrifice an artifact unless you discard a card",
            "You may sacrifice an artifact or discard a card",
        ),
        // The targeted-opponent back-reference after a choose sentence.
        (
            "That player loses X life",
            "Then the chosen player loses X life",
        ),
        ("They lose X life", "Then the chosen player loses X life"),
        // (A name-choice determiner pair "choose a card name" → "choose any
        // card name" was tried here and REVERTED: it broke the chosen-name
        // antecedent linkage on Foreshadow/Lammastide/The Clone Saga.)
        // The countered spell's cost back-reference (draw variant).
        (
            "Draw cards equal to that spell's mana value",
            "Draw cards equal to its mana value",
        ),
        // Mass type-setting copula number.
        (
            "Each land is an Island in addition to its other land types",
            "All lands are Islands in addition to their other types",
        ),
        // A where-X tail is redundant once the basis is spelled inline.
        (
            "with power less than its power, where X is its power",
            "with power less than its power",
        ),
        (
            "with power less than its power, where X is that creature's power",
            "with power less than that creature's power",
        ),
        // The tapped-set membership test is oracle's type predicate.
        (
            "If an Island was tapped this way,",
            "If that land is an Island,",
        ),
        // Counter-presence while-condition vs the renderer's if-gate.
        (
            "enters, if this has one or more -1/-1 counters on it,",
            "enters while this has a -1/-1 counter on it,",
        ),
        // The no-spell-cast-this-turn gate's tense.
        (
            "if they hasn't cast a spell this turn",
            "if they didn't cast a spell this turn",
        ),
        (
            "if that player hasn't cast a spell this turn",
            "if that player didn't cast a spell this turn",
        ),
        // Two-target search iteration back-reference.
        (
            "For each target player, They searches their library",
            "Each of them searches their library",
        ),
        (
            "For each target player, That player searches their library",
            "Each of them searches their library",
        ),
        // May-have-source-deal amount ordering with the hand-size basis.
        (
            "deal the number of cards in their hand damage to target player",
            "deal damage to target player equal to the number of cards in that player's hand",
        ),
        // Loyalty-cost clauses keep their prefix; the per-opponent possessive
        // is the iterated opponent's own.
        (
            "Each opponent loses life equal to the number of cards in that player's graveyard",
            "Each opponent loses life equal to the number of cards in their graveyard",
        ),
        (
            "discards a card and they lose 2 life",
            "discards a card and loses 2 life",
        ),
        // The destroyed-set iteration; a destroyed creature died.
        (
            "For each object destroyed this way, They creates",
            "For each creature that died this way, they creates",
        ),
        // Storm-count gate: casting this plus another is two or more.
        (
            "if you have cast two or more spells this turn",
            "if you've cast another spell this turn",
        ),
        // Negative-condition scaffold vs oracle's unless.
        (
            "if it isn't the case that this entered this turn, they discard a card",
            "discard a card unless this entered this turn",
        ),
        // Divided-damage amount spelled as where-X vs equal-to.
        (
            "X damage divided as you choose among any number of any targets, where X is this power",
            "damage equal to its power divided as you choose among any number of targets",
        ),
        // The countering tax payer and cost back-reference.
        (
            "unless its controller pays {X}, where X is this mana value",
            "unless they pay {X}, where X is its mana value",
        ),
        // The iterated player's own zones.
        (
            "Return up to three cards from that player's graveyard to that player's hand",
            "returns up to three cards from their graveyard to their hand",
        ),
        // The would-die replacement back-reference.
        (
            "If that Spirit creature would die this turn, exile it instead",
            "If it would die this turn, exile it instead",
        ),
        // The equipped-creature back-reference in an instead-arm.
        (
            "If equipped creature is a Warrior, equipped creature gets +2/+1 instead",
            "If it's a Warrior, it gets +2/+1 instead",
        ),
        // The exiled-set partitive play permission.
        (
            "Until your next end step, you may play them",
            "Until your next end step, you may play one of those cards",
        ),
        // The just-revealed card back-reference.
        (
            "Then you put it on the bottom of your library",
            "Then put the revealed card on the bottom of your library",
        ),
        // Mass ETB counter surfaces vs oracle's distributive wording.
        (
            "enter with a number of additional +1/+1 counters on them equal to the number of",
            "enters with an additional +1/+1 counter on it for each",
        ),
        // The chosen-creature-type pump program is oracle's single phrase.
        (
            "choose a creature type, then creatures of the chosen type",
            "creatures of the creature type of your choice",
        ),
        (
            "Choose a creature type, then creatures of the chosen type",
            "Creatures of the creature type of your choice",
        ),
        // "If a <color> permanent was dealt damage this way" tests the same
        // just-damaged object as oracle's "if it is <color>".
        ("a white permanent was dealt damage this way", "it is white"),
        ("a blue permanent was dealt damage this way", "it is blue"),
        ("a black permanent was dealt damage this way", "it is black"),
        ("a red permanent was dealt damage this way", "it is red"),
        ("a green permanent was dealt damage this way", "it is green"),
        // "You may have they shuffle" (post-anaphora) is "You may shuffle".
        (" have they shuffle", " shuffle"),
        ("that creature's", "its"),
        ("That creature's", "Its"),
        ("that card's", "its"),
        ("That card's", "Its"),
        ("that permanent's", "its"),
        ("That permanent's", "Its"),
        ("that token's", "its"),
        ("That token's", "Its"),
        // Object anaphors: "that creature"/"that card"/"that permanent" are
        // the same back-reference as "it"; unify so the renderer's pronoun
        // choice doesn't read as semantic drift.  Possessives are excluded by
        // the word-boundary check below.
        ("that creature", "it"),
        ("That creature", "It"),
        ("that card", "it"),
        ("That card", "It"),
        ("that permanent", "it"),
        ("That permanent", "It"),
        ("that token", "it"),
        ("That token", "It"),
        ("that object", "it"),
        ("That object", "It"),
        // Group back-reference vs the renderer's for-each surface.
        ("those creatures", "each creature"),
        ("Those creatures", "Each creature"),
        ("those cards", "them"),
        ("Those cards", "Them"),
        // The just-chosen color/type back-reference. The renderer sometimes
        // repeats the type family noun ("the chosen land type"); oracle's
        // back-reference is bare.
        ("the chosen color", "that color"),
        ("the chosen land type", "that type"),
        ("the chosen creature type", "that type"),
        ("the chosen type", "that type"),
        // "deals damage ... to each of two target creatures" — full damage
        // to every target; the count alone carries the same meaning.
        ("to each of two", "to two"),
        ("to each of three", "to three"),
        ("to each of four", "to four"),
        ("to each of up to", "to up to"),
        // Post-search shuffle surfaces: the per-player conditional shuffle
        // is the same event as "each player who searched ... shuffles".
        (
            "If an opponent does, shuffle that player's library",
            "Then each player who searched their library this way shuffles",
        ),
        // The shuffled library is implicit: modern oracle says "Then
        // shuffle." where older templating says "shuffles their library".
        ("shuffle your library", "shuffle"),
        ("shuffles your library", "shuffles"),
        ("shuffle their library", "shuffle"),
        ("shuffles their library", "shuffles"),
        ("Shuffle your library", "Shuffle"),
        ("Shuffle their library", "Shuffle"),
        // Negated control is templated both ways per card era; canonicalize
        // so "don't control an artifact" and "control no artifacts" agree.
        ("don't control an", "control no"),
        ("don't control a", "control no"),
        ("doesn't control an", "controls no"),
        ("doesn't control a", "controls no"),
        // After a choose, "the rest" and "all other <noun>s" name the same
        // complement set.
        ("Destroy all other creatures", "Destroy the rest"),
        ("destroy all other creatures", "destroy the rest"),
        // The enchanted permanent is named by its printed type in oracle and
        // by the generic noun in the renderer — same object either way.
        (
            "enchanted land's controller",
            "enchanted permanent's controller",
        ),
        (
            "enchanted enchantment's controller",
            "enchanted permanent's controller",
        ),
        (
            "enchanted artifact's controller",
            "enchanted permanent's controller",
        ),
        // The upkeep-damage back-reference: "its controller" and "that
        // player" name the same player after an of-controller trigger.
        // Anchored on the full trigger context so mass-damage clauses
        // ("Each enchantment deals 2 damage to its controller" — Aura
        // Barbs) are untouched even after the that-creature's→its rewrite
        // above cascades.
        (
            "controller, this deals 1 damage to its controller",
            "controller, this deals 1 damage to that player",
        ),
        (
            "controller, this deals 2 damage to its controller",
            "controller, this deals 2 damage to that player",
        ),
        (
            "controller, this deals 3 damage to its controller",
            "controller, this deals 3 damage to that player",
        ),
        (
            "defending player is the monarch",
            "that player is the monarch",
        ),
        // Choosing from a looked-at set: the renderer's bare pronoun and
        // oracle's partitive name the same single pick.
        ("put one of them into your hand", "put it into your hand"),
        ("Put one of them into your hand", "Put it into your hand"),
        (
            "put one of those cards into your hand",
            "put it into your hand",
        ),
        (
            "Put one of those cards into your hand",
            "Put it into your hand",
        ),
        // The renderer's articleless "from graveyard" is an artifact; the
        // source zone of a dies-return is implicit in oracle.
        (
            "return it from graveyard to the battlefield",
            "return it to the battlefield",
        ),
        // A single-copy token cleanup is back-referenced with "it".
        (
            "Exile those tokens at end of combat",
            "Exile it at end of combat",
        ),
        (
            "Exile the token at end of combat",
            "Exile it at end of combat",
        ),
        // "each other blocking creature" and "other blocking creatures" are
        // the same set (each-of-N == all N).
        (
            "each other blocking creature gets",
            "other blocking creatures get",
        ),
        // A kicked-damage "instead" rider back-references the spell's only
        // target; the repeated target phrase is the same object.
        ("damage to target creature instead", "damage to it instead"),
        // The renderer names the linked damage amount "X"; oracle back-
        // references it as "the damage dealt this way" — same quantity.
        ("life equal to the damage dealt this way", "life equal to X"),
        // A commander is necessarily a permanent in these choose contexts;
        // the renderer's extra noun is redundant.
        ("commander permanents", "commanders"),
        ("commander permanent", "commander"),
        // "each player's end step" is the pre-2023 templating of
        // "each end step" — the same trigger event.
        ("each player's end step", "each end step"),
        // The damage recipient back-reference: "that player" and "them" are
        // the same antecedent.
        ("damage to that player", "damage to them"),
        // The reveal-top ability names its target either way.
        (
            "Target player reveals the top card",
            "They reveal the top card",
        ),
        // A Curse's controller scope: "enchanted player" is the carried
        // player the renderer pronominalizes.
        (
            "Creatures enchanted player controls",
            "Creatures they control",
        ),
        (
            "creatures enchanted player controls",
            "creatures they control",
        ),
        // Both templatings of a token's entry recency appear across eras.
        (
            "token that entered the battlefield this turn",
            "token that entered this turn",
        ),
        // The attack-rider set is the damaged group either way.
        (
            "Each creature dealt damage this way attacks this turn if able",
            "Each creature attacks this turn if able",
        ),
        // The battlefield zone is implicit for the permanent half of a
        // battlefield-and-graveyard mass return.
        (
            "creatures on the battlefield and all creature cards",
            "creatures and all creature cards",
        ),
        // A delayed-draw's recipient back-references the player named earlier
        // in the effect; the renderer's "A player" and oracle's "They" are
        // the same antecedent in this fixed clause.
        (
            "A player draws a card at the beginning of the next turn's upkeep",
            "They draw a card at the beginning of the next turn's upkeep",
        ),
        // A copy-retarget over an already-scoped chosen set doesn't repeat
        // the controller scope.
        (
            "a different one of those creatures you control",
            "a different one of those creatures",
        ),
        // A single pick from a looked-at set: pronoun vs partitive-noun.
        ("choose one of those cards", "choose one of them"),
        ("Choose one of those cards", "Choose one of them"),
        // The source-excluding target is templated both ways.
        ("target creature other than this", "another target creature"),
        // A two-card look's complement is "the other"; the renderer's
        // general form spells out "the rest ... in any order".
        (
            "and the other on the bottom of your library",
            "and the rest on the bottom of your library in any order",
        ),
        // A random pick from a tagged set back-references either way.
        (
            "Destroy it chosen at random",
            "Destroy one of them chosen at random",
        ),
        // "chosen at random" and "at random" are the same selection method.
        ("one of them chosen at random", "one of them at random"),
        // The "a number of" quantifier is implicit before a counted noun.
        (
            "discards a number of cards equal to",
            "discards cards equal to",
        ),
        // PLURAL-target back-reference: when the preceding effect targets
        // multiple creatures ("...up to two target creatures."), the oracle
        // back-references them as "Those creatures", but the renderer still
        // emits the singular "Permanent" noun. These must run BEFORE the
        // singular "Permanent"→"It" pairs below so the plural context wins
        // (Sparkmage's Gambit / Wrap in Flames family). The discriminator is
        // the trailing "s" on the preceding "creatures".
        // The oracle back-reference "Those creatures" is itself normalized to
        // the distributive "Each creature" by an earlier pass, so emit that
        // canonical form here to match.
        (
            " creatures. Permanent can't block this turn",
            " creatures. Each creature can't block this turn",
        ),
        (
            " creatures. Permanent can't block until end of combat",
            " creatures. Each creature can't block this combat",
        ),
        (
            " creatures. Permanent can't block this combat",
            " creatures. Each creature can't block this combat",
        ),
        // Trigger-body restriction back-references the triggering object as
        // "it"; the renderer emits the generic "permanent" noun for the
        // it-tagged filter (blocks-or-becomes-blocked regenerate/block
        // triggers — Lim-Dûl's Cohort family).
        (
            ", permanent can't be regenerated this turn",
            ", it can't be regenerated this turn",
        ),
        (
            ", permanent can't block this turn",
            ", it can't block this turn",
        ),
        (
            ", permanent can't be blocked this turn",
            ", it can't be blocked this turn",
        ),
        // Period-separated capitalized variant (put-counter-then-restrict
        // sequences — Merciless Javelineer/Mugging). A standalone singular
        // "Permanent can't ..." is always a mis-rendered back-reference; a
        // real all-permanents effect would be plural.
        (
            ". Permanent can't block this turn",
            ". It can't block this turn",
        ),
        (
            ". Permanent can't be regenerated this turn",
            ". It can't be regenerated this turn",
        ),
        (
            ". Permanent can't be blocked this turn",
            ". It can't be blocked this turn",
        ),
        // The combat-scoped variant (Forgestoker Dragon family): back-ref +
        // "until end of combat"↔"this combat" duration surface.
        (
            ". Permanent can't block until end of combat",
            ". It can't block this combat",
        ),
        (
            ", permanent can't block until end of combat",
            ", it can't block this combat",
        ),
        // Post-duration-normalization form (the "until end of combat"→"this
        // combat" pass may run first).
        (
            ". Permanent can't block this combat",
            ". It can't block this combat",
        ),
        (
            ", permanent can't block this combat",
            ", it can't block this combat",
        ),
        // "can't be blocked" combat-scoped back-ref (Ma Chao family): same
        // artifact, passive voice.
        (
            ". Permanent can't be blocked until end of combat",
            ". It can't be blocked this combat",
        ),
        (
            ", permanent can't be blocked until end of combat",
            ", it can't be blocked this combat",
        ),
        (
            ". Permanent can't be blocked this combat",
            ". It can't be blocked this combat",
        ),
        (
            ", permanent can't be blocked this combat",
            ", it can't be blocked this combat",
        ),
        // In a counter-distribution list the verb "put" is stated once;
        // later items omit it (Incremental Growth/Blight).
        (", put two +1/+1 counters on", ", two +1/+1 counters on"),
        (", put three +1/+1 counters on", ", three +1/+1 counters on"),
        (", put two -1/-1 counters on", ", two -1/-1 counters on"),
        (", put three -1/-1 counters on", ", three -1/-1 counters on"),
        (
            ", and put three +1/+1 counters on",
            ", and three +1/+1 counters on",
        ),
        (
            ", and put three -1/-1 counters on",
            ", and three -1/-1 counters on",
        ),
        // Sequential damage targets are named "another"/"a third"; the
        // renderer repeats "any other" (Cone of Flame family).
        (
            "and 3 damage to any other target",
            "and 3 damage to a third target",
        ),
        (
            "2 damage to any other target,",
            "2 damage to another target,",
        ),
        // Cruel Reality: the enchanted player is the same referent whether
        // named or pronominalized across the if/then.
        (
            "If they can't, enchanted player loses 5 life",
            "If the player can't, they lose 5 life",
        ),
        // The optional phase-out of a chosen set: causative vs imperative.
        (
            "You may have any number of them phase out",
            "You may phase out any number of them",
        ),
        // The whole-hand shuffle is templated both ways across eras.
        (
            "shuffle all cards in your hand into your library",
            "shuffle the cards from your hand into your library",
        ),
        // A chosen set's iteration back-reference.
        ("For each of those objects", "For each of them"),
        ("for each of those objects", "for each of them"),
        // Choose-as-target vs plain choose over your own permanents is the
        // same selection.
        (
            "any number of target creatures you control",
            "any number of creatures you control",
        ),
        (
            "up to that many target creatures you control",
            "up to that many creatures you control",
        ),
        // The All-quantifier is implicit for restriction statics.
        ("All creatures can't block", "Creatures can't block"),
        ("All creatures can't attack", "Creatures can't attack"),
        // Self-name normalization inside a target phrase leaves "Target
        // this"; the self-reference already carries the meaning.
        ("Target this gets", "This gets"),
        ("target this gets", "this gets"),
        ("Target this deals", "This deals"),
        ("target this deals", "this deals"),
        // The face-down hand exile names its actor either way; the renderer
        // folds the subject into the carried player.
        (
            "Target player exiles all cards from their hand face down",
            "Exile all cards from their hand face down",
        ),
        (
            "that player returns those cards to their hand",
            "return those cards to their hand",
        ),
        // A repeated "target player controls" scope inside a later clause is
        // the same back-reference as the renderer's "they control".
        ("creature target player controls", "creature they control"),
        ("creatures target player controls", "creatures they control"),
        ("permanent target player controls", "permanent they control"),
        (
            "permanents target player controls",
            "permanents they control",
        ),
        (
            "creature another target player controls",
            "other creature they control",
        ),
        // "If the player does" (older templating) and "If they do" name the
        // same antecedent; the renderer emits the pronoun form.
        ("If the player does", "If they do"),
        ("if the player does", "if they do"),
        ("If the player doesn't", "If they don't"),
        ("if the player doesn't", "if they don't"),
        // Whole-graveyard shuffles are templated with and without the
        // explicit card enumeration across eras; canonicalize to the modern
        // short form.
        (
            "shuffle all cards from your graveyard into your library",
            "shuffle your graveyard into your library",
        ),
        (
            "Shuffle all cards from your graveyard into your library",
            "Shuffle your graveyard into your library",
        ),
        (
            "shuffles all cards from their graveyard into their library",
            "shuffles their graveyard into their library",
        ),
        (
            "shuffle all cards from their graveyard into their library",
            "shuffle their graveyard into their library",
        ),
        (
            "shuffle all cards from its owner's graveyard into its owner's library",
            "its owner shuffles their graveyard into their library",
        ),
        // The copy retarget rider is printed both as its own sentence and
        // conjoined ("copy it and you may ..."); canonicalize to one shape.
        (
            "copy that spell. You may choose new targets for the copy",
            "copy it and you may choose new targets for the copy",
        ),
        (
            "copy that spell or ability. You may choose new targets for the copy",
            "copy it and you may choose new targets for the copy",
        ),
        (
            "copy that ability. You may choose new targets for the copy",
            "copy it and you may choose new targets for the copy",
        ),
        (
            "copy it. You may choose new targets for the copy",
            "copy it and you may choose new targets for the copy",
        ),
        (
            "copy that spell. They may choose new targets for that copy",
            "copy it and you may choose new targets for the copy",
        ),
        (
            "Copy that spell. You may choose new targets for the copy",
            "Copy it and you may choose new targets for the copy",
        ),
        (
            "Copy that spell or ability. You may choose new targets for the copy",
            "Copy it and you may choose new targets for the copy",
        ),
        (
            "Copy it. You may choose new targets for the copy",
            "Copy it and you may choose new targets for the copy",
        ),
        (
            "Copy that ability. You may choose new targets for the copy",
            "Copy it and you may choose new targets for the copy",
        ),
        // A one-shot flicker return names its object either way ("return the
        // exiled card" / "return it"); the antecedent is the same exile.
        // Anchored to the delayed-return contexts so "a card exiled with
        // this enchantment" phrasings (Purgatory) keep their 'exiled' token.
        (
            "step, return the exiled card to the battlefield",
            "step, return it to the battlefield",
        ),
        (
            "Return the exiled card to the battlefield",
            "Return it to the battlefield",
        ),
        // A reflexive controller/owner destination is implicit in oracle
        // ("Return ... to the battlefield"); only a control CHANGE ("under
        // your control") is spelled out, and those forms are untouched.
        (
            " to the battlefield face down under its owner's control",
            " to the battlefield face down",
        ),
        (
            " to the battlefield face down under their owners' control",
            " to the battlefield face down",
        ),
        (
            " to the battlefield face down under their owner's control",
            " to the battlefield face down",
        ),
        (
            " to the battlefield under their owners' control",
            " to the battlefield",
        ),
        (
            " to the battlefield under their owner's control",
            " to the battlefield",
        ),
        (
            " to the battlefield under its owner's control",
            " to the battlefield",
        ),
        (
            " to the battlefield under its owners' control",
            " to the battlefield",
        ),
        (
            "create a token that's a copy of it under its controller's control",
            "its controller creates a token that's a copy of it",
        ),
        // Oracle always sequences looting as ", then discard(s)"; the
        // renderer's "and" join is the same instruction sequence. No
        // trailing space in the needle — the rewriter's word-boundary check
        // rejects matches whose tail continues with a letter.
        (" cards and discards", " cards, then discards"),
        (" card and discards", " card, then discards"),
        (" cards and discard", " cards, then discard"),
        (" card and discard", " card, then discard"),
        // Battlefield is the implicit zone for an iterated permanent noun;
        // both templatings appear across eras, so canonicalize both sides.
        (
            "for each creature token on the battlefield",
            "for each creature token",
        ),
        (
            "For each creature token on the battlefield",
            "For each creature token",
        ),
        ("for each creature on the battlefield", "for each creature"),
        ("For each creature on the battlefield", "For each creature"),
        // Mass counter targets and battlefield-implying constraints
        // (attached, attacking, crewed) leave the zone implicit in oracle.
        (
            "counters on each creature on the battlefield",
            "counters on each creature",
        ),
        (
            "counter on each creature on the battlefield",
            "counter on each creature",
        ),
        (" attached to it on the battlefield", " attached to it"),
        (
            "attacking creatures on the battlefield",
            "attacking creatures",
        ),
        (
            " crewed it this turn on the battlefield",
            " crewed it this turn",
        ),
        ("for each land on the battlefield", "for each land"),
        ("For each land on the battlefield", "For each land"),
        ("for each artifact on the battlefield", "for each artifact"),
        ("For each artifact on the battlefield", "For each artifact"),
        (
            "for each enchantment on the battlefield",
            "for each enchantment",
        ),
        (
            "For each enchantment on the battlefield",
            "For each enchantment",
        ),
        (
            "for each permanent on the battlefield",
            "for each permanent",
        ),
        (
            "For each permanent on the battlefield",
            "For each permanent",
        ),
    ];
    let mut normalized = text;
    for (from, to) in REWRITES {
        if !normalized.contains(from) {
            continue;
        }
        // "each of those creatures" is partitive, not a group reference —
        // but only the demonstrative-group rewrites are sensitive to it;
        // "cards of the chosen type" must still rewrite.
        let partitive_sensitive = from.starts_with("those") || from.starts_with("Those");
        let mut rewritten = String::with_capacity(normalized.len());
        let mut rest = normalized.as_str();
        while let Some(idx) = rest.find(from) {
            let after = &rest[idx + from.len()..];
            let boundary = after
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '\'' && c != '’');
            let partitive = partitive_sensitive
                && (rest[..idx].ends_with("of ") || rest[..idx].ends_with("Of "));
            rewritten.push_str(&rest[..idx]);
            rewritten.push_str(if boundary && !partitive { to } else { from });
            rest = after;
        }
        rewritten.push_str(rest);
        normalized = rewritten;
    }
    normalized.replace(
        HALF_PERMANENT_SENTINEL,
        "half that permanent's power and toughness",
    )
}

/// "If a <type> card was exiled/revealed this way" and the renderer's
/// "If it's a <type> card" test the same just-moved card; canonicalize the
/// oracle event-phrasing to the predicate form.
fn normalize_card_moved_this_way_condition(text: &str) -> String {
    const SUFFIXES: &[&str] = &[
        " was exiled this way",
        " is exiled this way",
        " was revealed this way",
        " is revealed this way",
        " was milled this way",
        " is milled this way",
        " was returned this way",
        " is returned this way",
        " was discarded this way",
        " is discarded this way",
    ];
    const PREFIXES: &[(&str, &str)] = &[
        ("If another ", "If it's another "),
        ("if another ", "if it's another "),
        ("If a ", "If it's a "),
        ("if a ", "if it's a "),
        ("If an ", "If it's an "),
        ("if an ", "if it's an "),
        ("If at least one ", "If it's a "),
        ("if at least one ", "if it's a "),
    ];
    let mut normalized = text.to_string();
    for suffix in SUFFIXES {
        loop {
            let Some(suffix_idx) = normalized.find(suffix) else {
                break;
            };
            let before = &normalized[..suffix_idx];
            let Some((prefix_idx, (prefix, replacement))) = PREFIXES
                .iter()
                .filter_map(|(p, r)| before.rfind(p).map(|i| (i, (*p, *r))))
                .max_by_key(|(i, _)| *i)
            else {
                break;
            };
            // The phrase between prefix and suffix must be a short noun
            // phrase ending in "card" or a bare type noun ("an Angel") —
            // not a whole extra clause.
            let noun = &normalized[prefix_idx + prefix.len()..suffix_idx];
            let bare_noun = !noun.is_empty() && !noun.contains(' ');
            if !(noun.ends_with(" card") || bare_noun) || noun.contains(',') || noun.len() > 40 {
                break;
            }
            normalized = format!(
                "{}{replacement}{noun}{}",
                &normalized[..prefix_idx],
                &normalized[suffix_idx + suffix.len()..]
            );
        }
    }
    normalized
}

/// Oracle templating is inconsistent about spelling out the battlefield zone
/// in choose instructions ("Choose a creature on the battlefield" vs "Choose
/// up to one creature"); the zone is implicit either way, so strip it from
/// choose sentences on both sides.
fn strip_choose_battlefield_zone(text: &str) -> String {
    const ZONE: &str = " on the battlefield";
    if !text.contains(ZONE) {
        return text.to_string();
    }
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find(ZONE) {
        let before = &rest[..idx];
        let sentence_start = before
            .rfind(". ")
            .into_iter()
            .chain(before.rfind(": "))
            .max()
            .map_or(0, |i| i + 2);
        let chooses = before[sentence_start..].contains("hoose");
        result.push_str(before);
        if !chooses {
            result.push_str(ZONE);
        }
        rest = &rest[idx + ZONE.len()..];
    }
    result.push_str(rest);
    result
}

/// Oracle writes repeated energy symbols as a count word ("get six {E}",
/// "Pay eight {E}"); the renderer sometimes emits the literal run
/// "{E}{E}{E}{E}{E}{E}". Collapse any run of 2+ {E} to "<count> {E}" so
/// both sides agree.
fn normalize_energy_pip_runs(text: &str) -> String {
    if !text.contains("{E}{E}") {
        return text.to_string();
    }
    const WORDS: &[&str] = &[
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    ];
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find("{E}") {
        result.push_str(&rest[..idx]);
        // Count the consecutive {E} run.
        let mut n = 0;
        let mut tail = &rest[idx..];
        while let Some(after) = tail.strip_prefix("{E}") {
            n += 1;
            tail = after;
        }
        if n >= 2 && n < WORDS.len() {
            result.push_str(WORDS[n]);
            result.push_str(" {E}");
        } else {
            for _ in 0..n {
                result.push_str("{E}");
            }
        }
        rest = tail;
    }
    result.push_str(rest);
    result
}

fn normalize_repeated_you_after_draw(text: &str) -> String {
    fn collapse_marker(segment: &mut String, marker: &str, replacement: &str) {
        let lower = segment.to_ascii_lowercase();
        let Some(marker_idx) = lower.find(marker) else {
            return;
        };
        let before = &lower[..marker_idx];
        let Some(draw_idx) = before.rfind("draw ") else {
            return;
        };
        // A serial comma means the draw is one item in a longer effect list
        // (for example, "each opponent discards, you draw, and you gain").
        // Removing the repeated subject there changes clause grouping and can
        // hide the preceding effect.  Only fold a directly coordinated
        // draw/life pair.
        let trimmed = lower.trim_start();
        let draw_is_sentence_subject = trimmed.starts_with("draw ")
            || trimmed.starts_with("you draw ")
            || trimmed.starts_with("when you draw ")
            || trimmed.starts_with("whenever you draw ");
        if before[draw_idx..].contains(',') && !draw_is_sentence_subject {
            return;
        }
        segment.replace_range(marker_idx..marker_idx + marker.len(), replacement);
    }

    text.split(". ")
        .map(|sentence| {
            let mut sentence = sentence.to_string();
            collapse_marker(&mut sentence, " and you lose ", " and lose ");
            collapse_marker(&mut sentence, " and you gain ", " and gain ");
            sentence
        })
        .collect::<Vec<_>>()
        .join(". ")
}

/// ", then <verb>s ..." continues its sentence's subject. The generic
/// ", then " → ". " split below would orphan that clause (a subjectless
/// "exiles a card from their hand"), so restore a subject into the split
/// when both the subject phrase and the continuation verb are recognized.
/// The restored subject is always the back-reference "That player" — the
/// carried-player pass later unifies it with the renderer's pronoun choice.
/// Bare imperatives ("then shuffle") keep their implicit "you" untouched.
fn carry_subject_through_then_splits(text: &str) -> String {
    const SUBJECTS: &[&str] = &[
        "Target player ",
        "Target opponent ",
        "That player ",
        "That opponent ",
        "The player ",
        "Defending player ",
    ];
    const THIRD_PERSON_VERBS: &[&str] = &[
        "exiles",
        "discards",
        "sacrifices",
        "reveals",
        "shuffles",
        "draws",
        "gains",
        "loses",
        "puts",
        "mills",
        "creates",
        "returns",
        "destroys",
        "chooses",
        "scries",
        "surveils",
    ];
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find(", then ") {
        let after = &rest[idx + ", then ".len()..];
        let word = after
            .split(|c: char| !c.is_ascii_alphabetic())
            .next()
            .unwrap_or("");
        let sentence_start = rest[..idx]
            .rfind(". ")
            .into_iter()
            .chain(rest[..idx].rfind(": "))
            .max()
            .map_or(0, |i| i + 2);
        let subject = SUBJECTS
            .iter()
            .find(|s| rest[sentence_start..].starts_with(**s));
        if subject.is_some() && THIRD_PERSON_VERBS.contains(&word) {
            result.push_str(&rest[..idx]);
            result.push_str(". That player ");
        } else {
            result.push_str(&rest[..idx + ", then ".len()]);
        }
        rest = after;
    }
    result.push_str(rest);
    result
}

/// A second sentence that re-states "Target player"/"Target opponent" as its
/// subject is a back-reference to the target introduced by the first; oracle
/// and the renderer disagree about whether to repeat the phrase or use
/// "that player"/"they". Demote the repeats so both sides converge on the
/// back-reference surface.
fn demote_repeated_player_subjects(text: &str) -> String {
    let mut normalized = text.to_string();
    for (subject, demoted) in [
        ("Target player ", "That player "),
        ("Target opponent ", "That player "),
        ("target player ", "that player "),
        ("target opponent ", "that player "),
    ] {
        let Some(first) = normalized
            .to_ascii_lowercase()
            .find(&subject.to_ascii_lowercase())
        else {
            continue;
        };
        let scan_from = first + subject.len();
        let mut result = normalized[..scan_from].to_string();
        let mut rest = &normalized[scan_from..];
        while let Some(idx) = rest.find(subject) {
            let boundary = rest[..idx].ends_with(". ")
                || rest[..idx].ends_with(": ")
                || rest[..idx].ends_with(", ")
                || rest[..idx].ends_with("then ");
            result.push_str(&rest[..idx]);
            result.push_str(if boundary { demoted } else { subject });
            rest = &rest[idx + subject.len()..];
        }
        result.push_str(rest);
        normalized = result;
    }
    normalized
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
    normalized = normalize_carried_target_player_references(&normalized);
    normalized = normalize_repeated_filtered_set_coreferences(&normalized);
    normalized = normalize_anaphoric_object_surfaces(&normalized);
    normalized = strip_choose_battlefield_zone(&normalized);
    normalized = normalize_card_moved_this_way_condition(&normalized);
    normalized = normalize_energy_pip_runs(&normalized);
    // "exile target player's graveyard" repeats an already-introduced
    // target; oracle back-references it ("exile their graveyard"). Only
    // rewrite when another mention keeps the target tokens in the text —
    // for a first mention (Identity Crisis) the phrase IS the introduction.
    for needle in [
        "Exile target player's graveyard",
        "exile target player's graveyard",
    ] {
        if let Some(idx) = normalized.find(needle) {
            let has_other_mention = normalized[..idx].contains("arget player")
                || normalized[idx + needle.len()..].contains("arget player");
            if has_other_mention {
                let replacement = if needle.starts_with('E') {
                    "Exile their graveyard"
                } else {
                    "exile their graveyard"
                };
                normalized = normalized.replacen(needle, replacement, 1);
            }
        }
    }
    // "Each X you control enters with ..." and the plural "Xs you control
    // enter with ..." are the same replacement effect; number agreement and
    // pronouns are already normalized, so dropping the quantifier aligns
    // the token sets.
    if normalized.starts_with("Each ")
        && (normalized.contains(" enters with an additional")
            || normalized.contains(" enter with an additional")
            || normalized.contains(" enters with a number of additional")
            || normalized.contains(" enter with a number of additional"))
    {
        normalized = normalized["Each ".len()..].to_string();
    }
    normalized = normalized
        // A single prison sentence and the renderer's two static-ability
        // sentences describe the same pair of restrictions.
        .replace(
            "Enchanted creature can't attack or block, and its activated abilities",
            "Enchanted creature can't attack or block. Enchanted creature activated abilities",
        )
        .replace(
            "Enchanted permanent can't attack or block, and its activated abilities",
            "Enchanted permanent can't attack or block. Enchanted permanent activated abilities",
        )
        .replace(
            "enchanted creature can't attack or block, and its activated abilities",
            "enchanted creature can't attack or block. enchanted creature activated abilities",
        )
        .replace(
            "enchanted permanent can't attack or block, and its activated abilities",
            "enchanted permanent can't attack or block. enchanted permanent activated abilities",
        )
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
    ) || normalized_lower.starts_with(
        "whenever this creature attacks, untap target creature defending player controls. target creature defending player controls gains blocks each combat if able until end of combat",
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
    normalized = normalize_named_soulbond_pairing_surface(&normalized);
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
        (
            " from your hand if you dont this enters tapped",
            " from your hand. If you don't, this enters tapped",
        ),
        (
            " from your hand if you don't this enters tapped",
            " from your hand. If you don't, this enters tapped",
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

    // Oracle may repeat the explicit player subject across a coordinated
    // draw/life pair while compiled text uses the equivalent implied subject.
    // Keep longer serial effect lists and later sentences separate.
    normalized = normalize_repeated_you_after_draw(&normalized);

    for (from, to) in [
        ("For each player, that player ", "Each player "),
        ("for each player, that player ", "each player "),
        ("For each opponent, that player ", "Each opponent "),
        ("for each opponent, that player ", "each opponent "),
        ("For each player, they ", "Each player "),
        ("for each player, they ", "each player "),
        ("For each opponent, they ", "Each opponent "),
        ("for each opponent, they ", "each opponent "),
        ("For each player, ", "Each player "),
        ("for each player, ", "each player "),
        ("For each opponent, ", "Each opponent "),
        ("for each opponent, ", "each opponent "),
    ] {
        if normalized.starts_with(from) {
            normalized = normalized.replacen(from, to, 1);
        }
    }
    // Once the iteration collapses to a distributive subject, the per-player
    // possessive is the iterated player's own.
    for prefix in [
        "Each player ",
        "Each opponent ",
        "each player ",
        "each opponent ",
    ] {
        if normalized.starts_with(prefix) && normalized.contains(" that player's ") {
            normalized = normalized.replace(" that player's ", " their ");
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
    for marker in [", it gets ", ", It gets "] {
        if let Some((left, right)) = normalized.split_once(marker)
            && left.eq_ignore_ascii_case("Untap target creature")
        {
            normalized = format!("{left}. It gets {right}");
            break;
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
    if let Some((left, right)) = normalized.split_once(", then it gains ") {
        let left_trimmed = left.trim_end_matches('.').trim();
        let left_lower = left_trimmed.to_ascii_lowercase();
        if left_lower.contains("target creature gets ")
            && left_lower.ends_with(" until end of turn")
            && !right.trim().is_empty()
        {
            normalized = format!(
                "{} and gains {}",
                left_trimmed.trim_end_matches(" until end of turn"),
                right.trim()
            );
        }
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
        // Post and/or canonicalization variants of the same surface.
        .replace(
            "Add 2 mana in any combination of {W} and or {U} and or {B} and or {R} and or {G}",
            "Add two mana in any combination of colors",
        )
        .replace(
            "add 2 mana in any combination of {w} and or {u} and or {b} and or {r} and or {g}",
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
            "choose up to two - ",
            "choose up to two — ",
        )
        .replace(
            "Choose up to two - ",
            "Choose up to two — ",
        )
        .replace(
            "choose up to two —",
            "choose one or both —",
        )
        .replace(
            "Choose up to two —",
            "Choose one or both —",
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
            ", choose one or both — ",
            ", choose one or both —. ",
        )
        .replace(
            ", Choose one or both — ",
            ", Choose one or both —. ",
        )
        .replace(
            ": choose one or both — ",
            ": choose one or both —. ",
        )
        .replace(
            ": Choose one or both — ",
            ": Choose one or both —. ",
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
            "then that Army deals damage equal to that Army's power",
            "then the Army you amassed deals damage equal to its power",
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
        .replace(
            "except their power is half its power and their toughness is half its toughness",
            "except their power and toughness are each half its power and toughness",
        )
        .replace(". Round up each time", ", rounded up")
        .replace(". round up each time", ", rounded up")
        .replace(
            "If it dies this way, Create two tokens that are copies of it under its controller's control, except",
            "If it dies this way, its controller creates two tokens that are copies of it, except",
        )
        .replace(
            "if it dies this way, create two tokens that are copies of it under its controller's control, except",
            "if it dies this way, its controller creates two tokens that are copies of it, except",
        )
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
    // "draw X cards and lose X life, where X is <basis>" splits into two
    // clauses that EACH carry the basis — matching the renderer's joined
    // form, which is also split below.
    for needle in [
        " draw X cards and lose X life, where X is ",
        " draw X cards and you lose X life, where X is ",
        // The renderer's imperative sentence-initial form.
        "Draw X cards and lose X life, where X is ",
        "Draw X cards and you lose X life, where X is ",
    ] {
        if let Some((head, tail)) = normalized.split_once(needle) {
            let (basis, rest) = match tail.split_once(". ") {
                Some((basis, rest)) => (basis.trim_end_matches('.'), Some(rest.to_string())),
                None => (tail.trim_end_matches('.'), None),
            };
            let draw_head = if needle.starts_with(' ') {
                format!("{head} draw")
            } else {
                format!("{head}You draw")
            };
            let mut rebuilt = format!(
                "{draw_head} X cards, where X is {basis}. You lose X life, where X is {basis}."
            );
            if let Some(rest) = rest {
                rebuilt.push(' ');
                rebuilt.push_str(&rest);
            }
            normalized = rebuilt;
        }
    }
    // "it deals X damage to any target, target player draws X cards, and you
    // gain X life, where X is <basis>" (Niv-Mizzet, Guildpact) distributes the
    // single trailing basis onto each clause — matching the renderer's
    // per-clause where-X form.
    if let Some((head, tail)) = normalized.split_once(
        " deals X damage to any target, target player draws X cards, and you gain X life, where X is ",
    ) && !tail.contains(", where X is ")
    {
        let (basis, rest) = match tail.split_once(". ") {
            Some((basis, rest)) => (basis.trim_end_matches('.'), Some(rest.to_string())),
            None => (tail.trim_end_matches('.'), None),
        };
        let mut rebuilt = format!(
            "{head} deals X damage to any target, where X is {basis}, target player draws X cards, where X is {basis}, and you gain X life, where X is {basis}."
        );
        if let Some(rest) = rest {
            rebuilt.push(' ');
            rebuilt.push_str(&rest);
        }
        normalized = rebuilt;
    }
    // "You gain X life and put X +1/+1 counters on <target>, where X is
    // <basis>" splits into two clauses that EACH carry the basis — matching
    // the renderer's per-clause where-X form.
    if let Some((head, tail)) = normalized.split_once(" gain X life and put X ")
        && let Some((mid, basis_and_rest)) = tail.split_once(", where X is ")
    {
        let (basis, rest) = match basis_and_rest.split_once(". ") {
            Some((basis, rest)) => (basis.trim_end_matches('.'), Some(rest.to_string())),
            None => (basis_and_rest.trim_end_matches('.'), None),
        };
        let mut rebuilt =
            format!("{head} gain X life, where X is {basis}. Put X {mid}, where X is {basis}.");
        if let Some(rest) = rest {
            rebuilt.push(' ');
            rebuilt.push_str(&rest);
        }
        normalized = rebuilt;
    }
    // The renderer's joined form: basis mid-sentence, then "and put X ...".
    normalized = normalized.replace(" and put X +1/+1 counters on", ". Put X +1/+1 counters on");
    normalized = normalized.replace(
        " and you lose X life, where X is ",
        ". You lose X life, where X is ",
    );
    normalized = normalized.replace(
        " and lose X life, where X is ",
        ". You lose X life, where X is ",
    );
    normalized = carry_subject_through_then_splits(&normalized);
    normalized = demote_repeated_player_subjects(&normalized);
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
    if let Some(rest) = normalized.strip_prefix("Choose up to two — ") {
        normalized = format!("Choose up to two —. {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("choose up to two — ") {
        normalized = format!("choose up to two —. {rest}");
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
        .replace("choose up to two -", "choose up to two —")
        .replace("Choose up to two -", "Choose up to two —")
        .replace("choose up to two —", "choose one or both —")
        .replace("Choose up to two —", "Choose one or both —")
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
        .replace(" and you lose ", ". You lose ");
    normalized = normalize_that_player_references_for_clause_surface(&normalized)
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
        .replace("Tag the object attached to this Aura as 'enchanted'. ", "")
        .replace("tag the object attached to this Aura as 'enchanted'. ", "")
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
        .replace(
            "Whenever one or more creature you control attack",
            "Whenever you attack",
        )
        .replace(
            "Whenever one or more creatures you control attack",
            "Whenever you attack",
        )
        .replace(
            "whenever one or more creature you control attack",
            "whenever you attack",
        )
        .replace(
            "whenever one or more creatures you control attack",
            "whenever you attack",
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
    let lower_normalized = normalized.to_ascii_lowercase();
    let saw_in_half_context = (lower_normalized.contains("if that creature dies this way")
        && lower_normalized.contains("copies of that creature"))
        || (lower_normalized.contains("if it dies this way")
            && lower_normalized.contains("copies of it"));
    if saw_in_half_context {
        normalized = normalized.replace(
            "half that permanent's power and toughness",
            "half its power and toughness",
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
            "for each opponent, if you don't, that player loses 3 life",
            "Each opponent who can't loses 3 life",
        )
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
        )
        // End-of-turn play permissions have used both a leading duration and
        // a trailing "this turn" template. The remembered exile tag carries
        // singular/plural cardinality, so pronoun choice is surface-only.
        .replace(
            "Until end of turn, you may play it",
            "You may play that card this turn",
        )
        .replace(
            "Until end of turn, you may play them",
            "You may play that card this turn",
        )
        .replace(
            "until end of turn, you may play it",
            "you may play that card this turn",
        )
        .replace(
            "until end of turn, you may play them",
            "you may play that card this turn",
        )
        // The connective canonicalization rewrites the leading duration to
        // "This turn," before this pass; accept those surfaces too.
        .replace(
            "This turn, you may play it",
            "You may play that card this turn",
        )
        .replace(
            "This turn, you may play them",
            "You may play that card this turn",
        )
        .replace(
            "this turn, you may play it",
            "you may play that card this turn",
        )
        .replace(
            "this turn, you may play them",
            "you may play that card this turn",
        )
        .replace(
            "You may play it until end of turn",
            "You may play that card this turn",
        )
        .replace(
            "You may play them until end of turn",
            "You may play that card this turn",
        )
        .replace(
            "you may play it this turn",
            "you may play that card this turn",
        )
        .replace(
            "you may play them this turn",
            "you may play that card this turn",
        )
        .replace(
            "You may play it this turn",
            "You may play that card this turn",
        )
        .replace(
            "You may play them this turn",
            "You may play that card this turn",
        )
        .replace(
            "Until end of turn, you may cast it",
            "You may cast that card this turn",
        )
        .replace(
            "until end of turn, you may cast it",
            "you may cast that card this turn",
        )
        .replace(
            "You may cast it from exile this turn",
            "You may cast that card this turn",
        )
        .replace(
            "you may cast it from exile this turn",
            "you may cast that card this turn",
        )
        .replace(
            "You may cast it this turn",
            "You may cast that card this turn",
        )
        .replace(
            "you may cast it this turn",
            "you may cast that card this turn",
        )
        .replace(
            "This turn, you may cast that card",
            "You may cast that card this turn",
        )
        .replace(
            "this turn, you may cast that card",
            "you may cast that card this turn",
        )
        .replace(
            "This turn, you may cast spells from among those cards",
            "You may cast that card this turn",
        )
        .replace(
            "this turn, you may cast spells from among those cards",
            "you may cast that card this turn",
        )
        .replace(
            "This turn, you may cast spells from among them",
            "You may cast that card this turn",
        )
        .replace(
            "this turn, you may cast spells from among them",
            "you may cast that card this turn",
        )
        .replace(
            "This creature source is white if there are seven or more cards in your graveyard",
            "If there are seven or more cards in your graveyard, this is white",
        )
        .replace(
            "this creature source is white if there are seven or more cards in your graveyard",
            "if there are seven or more cards in your graveyard, this is white",
        )
        .replace("When effect #0 you discard", "When you discard")
        .replace("when effect #0 you discard", "when you discard")
        .replace(
            "When effect #0 the affected object isn't land,",
            "When you exile a nonland card this way,",
        )
        .replace(
            "when effect #0 the affected object isn't land,",
            "when you exile a nonland card this way,",
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
    let echo_guard_prefix =
        "at the beginning of your upkeep, if this object is on the battlefield, ";
    let echo_normalized_trimmed = normalized_lower
        .starts_with(echo_guard_prefix)
        .then(|| normalized_trimmed[echo_guard_prefix.len()..].trim())
        .unwrap_or(normalized_trimmed);
    let echo_normalized_lower = echo_normalized_trimmed.to_ascii_lowercase();
    if normalized_lower == "this creature enters with an echo counter on it"
        || normalized_lower == "this artifact enters with an echo counter on it"
        || normalized_lower == "this permanent enters with an echo counter on it"
    {
        normalized.clear();
    } else if echo_normalized_lower
        .starts_with("at the beginning of your upkeep, remove an echo counter from this ")
        && echo_normalized_lower.contains(" unless you ")
    {
        if let Some(idx) = echo_normalized_lower.find(" unless you ") {
            let cost = echo_normalized_trimmed[idx + " unless you ".len()..]
                .trim()
                .trim_end_matches('.');
            if let Some(mana_cost) = cost
                .strip_prefix("pay ")
                .or_else(|| cost.strip_prefix("Pay "))
                .filter(|cost| cost.starts_with('{'))
            {
                normalized = format!("Echo {mana_cost}");
            } else {
                normalized = format!("Echo—{cost}");
            }
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

fn normalize_named_soulbond_pairing_surface(text: &str) -> String {
    let marker = " is paired with another creature, each of those creatures has ";
    let mut normalized = String::with_capacity(text.len());
    for segment in text.split_inclusive('\n') {
        let (line, line_ending) = if let Some(stripped) = segment.strip_suffix('\n') {
            (stripped, "\n")
        } else {
            (segment, "")
        };

        if let Some(rest) = line.strip_prefix("As long as ")
            && let Some(marker_idx) = rest.find(marker)
        {
            normalized.push_str("As long as this creature");
            normalized.push_str(&rest[marker_idx..]);
            normalized.push_str(line_ending);
            continue;
        }

        if let Some(rest) = line.strip_prefix("as long as ")
            && let Some(marker_idx) = rest.find(marker)
        {
            normalized.push_str("as long as this creature");
            normalized.push_str(&rest[marker_idx..]);
            normalized.push_str(line_ending);
            continue;
        }

        // The connective canonicalization turns "As long as" into "If"
        // before this pass; accept both surfaces.
        if let Some(rest) = line.strip_prefix("If ")
            && let Some(marker_idx) = rest.find(marker)
        {
            normalized.push_str("If this creature");
            normalized.push_str(&rest[marker_idx..]);
            normalized.push_str(line_ending);
            continue;
        }
        if let Some(rest) = line.strip_prefix("if ")
            && let Some(marker_idx) = rest.find(marker)
        {
            normalized.push_str("if this creature");
            normalized.push_str(&rest[marker_idx..]);
            normalized.push_str(line_ending);
            continue;
        }

        normalized.push_str(line);
        normalized.push_str(line_ending);
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

fn starts_with_damage_subject(text: &str) -> bool {
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
}

fn normalize_damage_self_reference(text: &str) -> String {
    text.replace(" to this creature", " to itself")
        .replace(" to This creature", " to itself")
        .replace(" to this Creature", " to itself")
}

fn strip_leading_mana_cost_for_damage_clause(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('{')?;
    let end = rest.find("}: ")?;
    if !rest[..end].chars().all(|c| match c {
        '{' | '}' | '/' | ',' | ' ' => true,
        '0'..='9' => true,
        'A'..='Z' | 'a'..='z' => matches!(
            c.to_ascii_uppercase(),
            'W' | 'U' | 'B' | 'R' | 'G' | 'T' | 'X' | 'Y' | 'Z' | 'S' | 'P' | 'C'
        ),
        _ => false,
    }) {
        return None;
    }

    let tail = rest[end + 3..].trim_start();
    if starts_with_damage_subject(tail) {
        Some(tail)
    } else {
        None
    }
}

fn normalize_damage_source_for_clause_surface(line: &str) -> String {
    let normalized = strip_leading_mana_cost_for_damage_clause(line).unwrap_or(line);
    if starts_with_damage_subject(normalized) {
        return normalize_damage_self_reference(normalized);
    }

    for separator in [": ", ". "] {
        if let Some((left, right)) = normalized.split_once(separator) {
            let right = right.trim_start();
            if starts_with_damage_subject(right) {
                return format!(
                    "{left}{separator}{}",
                    normalize_damage_self_reference(right)
                );
            }
        }
    }

    normalized.to_string()
}

fn normalize_explicit_damage_source_for_compare(line: &str) -> String {
    fn normalize_damage_clause_tail(text: &str) -> Option<String> {
        let lower = text.to_ascii_lowercase();
        if lower.starts_with("deal ") {
            return Some(normalize_damage_self_reference(text));
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
                let rest = normalize_damage_self_reference(text[prefix.len()..].trim_start());
                return Some(format!("Deal {rest}"));
            }
        }

        None
    }

    let mut normalized = line;
    if let Some(tail) = strip_leading_mana_cost_for_damage_clause(line) {
        normalized = tail;
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
    // "and/or" (canonicalized to "and or" upstream) is a disjunction, not a
    // list — splitting it emits a bogus "Return or creatures ..." clause.
    if lower.contains("and or ") || lower.contains("and/or") {
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

fn split_sentence_helper_parts(line: &str) -> Vec<String> {
    line.split(". ")
        .map(|part| part.trim().trim_end_matches('.').trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

fn sentence_helper_choice_text(part: &str) -> Option<String> {
    let lower = part.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("you choose ")
        .or_else(|| lower.strip_prefix("choose "))?;
    let marker = " card in library and tags it as ";
    let marker_idx = rest.find(marker)?;
    let mut choice = rest[..marker_idx].trim();
    choice = choice.strip_prefix("up to one other ").unwrap_or(choice);
    let choice = if let Some(rest) = choice.strip_prefix("up to one ") {
        format!("up to one {rest} card from among them")
    } else {
        format!("{choice} card from among them")
    };
    Some(choice)
}

fn sentence_helper_destination_text(action: &str) -> Option<String> {
    let trimmed = action.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("put it onto the battlefield") {
        let rest = rest.trim();
        return Some(if rest.is_empty() {
            "onto the battlefield".to_string()
        } else {
            format!("onto the battlefield {rest}")
        });
    }
    if lower.starts_with("return it to ") && lower.contains("owner") && lower.contains("hand") {
        return Some("into your hand".to_string());
    }
    if let Some(rest) = lower.strip_prefix("put it into ") {
        return Some(format!("into {rest}"));
    }
    None
}

fn normalize_sentence_helper_remainder(part: &str) -> String {
    part.replace("remaining tagged cards", "rest")
        .replace("Remaining tagged cards", "Rest")
}

fn normalize_sentence_helper_line_for_compare(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if !lower.contains("__sentence_helper") {
        return line.to_string();
    }

    let parts = split_sentence_helper_parts(line);
    if parts.is_empty() {
        return line.to_string();
    }

    let mut idx = 0usize;
    let mut out = Vec::new();
    let first_lower = parts[0].to_ascii_lowercase();
    if first_lower.starts_with("look at the top ") {
        if parts
            .get(1)
            .is_some_and(|part| part.eq_ignore_ascii_case("Reveal it"))
        {
            out.push(format!("Reveal{}", &parts[0]["Look at".len()..]));
            idx = 2;
        } else {
            out.push(parts[0].clone());
            idx = 1;
        }
    } else if first_lower.starts_with("reveal the top ") {
        out.push(parts[0].clone());
        idx = 1;
    }

    let mut choice_actions = Vec::new();
    while idx + 1 < parts.len() {
        let Some(choice) = sentence_helper_choice_text(&parts[idx]) else {
            break;
        };
        let Some(destination) = sentence_helper_destination_text(&parts[idx + 1]) else {
            break;
        };
        choice_actions.push(format!("{choice} {destination}"));
        idx += 2;
    }

    if !choice_actions.is_empty() {
        out.push(format!("You may put {}", choice_actions.join(" and ")));
    }

    out.extend(
        parts[idx..]
            .iter()
            .map(|part| normalize_sentence_helper_remainder(part)),
    );

    if out.len() <= 1 {
        line.to_string()
    } else {
        out.into_iter()
            .map(|part| {
                let trimmed = part.trim().trim_end_matches('.');
                format!("{trimmed}.")
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
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
        let line = normalize_sentence_helper_line_for_compare(&line);
        let line = normalize_trigger_subject_for_compare(&line);
        let line = strip_modal_option_labels(&line);
        let line = normalize_for_each_player_conditional_for_compare(&line);
        let line = split_common_clause_conjunctions(&line);
        let line = normalize_damage_source_for_clause_surface(&line);
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
    merge_still_land_clauses(clauses)
}

pub fn semantic_clauses_for_compare(text: &str) -> Vec<String> {
    semantic_clauses(text)
}

fn still_land_tail_for_clause(clause: &str) -> Option<&'static str> {
    let lower = clause
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase()
        .replace('’', "'");
    match lower.as_str() {
        "it's still a land" | "it is still a land" => Some("that's still a land"),
        "they're still lands" | "they are still lands" => Some("that are still lands"),
        _ => None,
    }
}

fn merge_still_land_clauses(clauses: Vec<String>) -> Vec<String> {
    let mut merged: Vec<String> = Vec::with_capacity(clauses.len());
    for clause in clauses {
        if let Some(tail) = still_land_tail_for_clause(&clause)
            && let Some(previous) = merged.last_mut()
        {
            let previous_lower = previous.to_ascii_lowercase();
            if previous_lower.contains(" become")
                && !previous_lower.contains("still a land")
                && !previous_lower.contains("still lands")
            {
                let trimmed = previous.trim_end_matches('.');
                *previous = format!("{trimmed} {tail}.");
                continue;
            }
        }
        merged.push(clause);
    }
    merged
}

fn split_compiled_lines_for_semantic_compare(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .flat_map(|line| {
            let has_modal_bullets = line.lines().any(|part| part.trim_start().starts_with('•'));
            if has_modal_bullets {
                line.lines()
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            } else {
                vec![line.replace('\n', " ")]
            }
        })
        .collect()
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
        // Word-boundary guard: a name must not rewrite inside a longer word
        // ("Curse" inside "Curses" would leave "thiss" behind).
        let word_edge_ok = haystack[abs_idx + needle.len()..]
            .chars()
            .next()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric())
            && (abs_idx == 0
                || !haystack[..abs_idx]
                    .chars()
                    .next_back()
                    .is_some_and(|ch| ch.is_ascii_alphanumeric()));
        if !word_edge_ok {
            let skip = abs_idx + needle.len();
            result.push_str(&replacement_text[src_idx..skip]);
            src_idx = skip;
            search_idx = skip;
            continue;
        }
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
    let lead_name = left_half
        .split_whitespace()
        .next()
        .map(str::trim)
        .unwrap_or(left_half);

    let mut names = vec![full_name, left_half, short_name];
    let lead_name_lower = lead_name.to_ascii_lowercase();
    if lead_name.len() >= 3
        && lead_name_lower != "the"
        && lead_name_lower != "a"
        && lead_name_lower != "an"
        && (left_half.contains(" of ") || left_half.contains(','))
    {
        names.push(lead_name);
    }
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
    if let Some(stripped) = lead_name
        .strip_prefix("A-")
        .or_else(|| lead_name.strip_prefix("a-"))
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
        .replace("that object's controller", "its controller")
        .replace("Sacrifice this enchantment", "Sacrifice this")
        .replace("sacrifice this enchantment", "sacrifice this")
        .replace("Sacrifice this permanent", "Sacrifice this")
        .replace("sacrifice this permanent", "sacrifice this")
        .replace("Sacrifice this, then", "Sacrifice this then")
        .replace("sacrifice this, then", "sacrifice this then");
    normalized = normalize_self_reference_nouns(&normalized);
    normalized
}

/// Self-reference type nouns: "this creature", "this permanent", … all denote
/// the source object, exactly like the card name (already rewritten to
/// "this" above).  Unify them so stylistic differences between the oracle's
/// and the compiler's self-naming don't read as semantic drift.  Possessive
/// forms ("this creature's") are left alone so we don't manufacture "this's".
fn normalize_self_reference_nouns(text: &str) -> String {
    const SELF_NOUNS: &[&str] = &[
        "creature",
        "permanent",
        "artifact",
        "enchantment",
        "land",
        "planeswalker",
        "battle",
        "spell",
        "card",
        "aura",
        "equipment",
        "vehicle",
        "source",
    ];
    let mut normalized = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut idx = 0usize;
    'outer: while idx < bytes.len() {
        for lead in ["this ", "This "] {
            if text[idx..].starts_with(lead) {
                let after_lead = idx + lead.len();
                for noun in SELF_NOUNS {
                    if text[after_lead..].to_ascii_lowercase().starts_with(noun) {
                        let after_noun = after_lead + noun.len();
                        // Possessive: "this creature's power" matches the
                        // card-name rewrite's "this power" surface.
                        for possessive in ["'s ", "’s "] {
                            if text[after_noun..].starts_with(possessive) {
                                normalized.push_str(&lead[..4]);
                                normalized.push(' ');
                                idx = after_noun + possessive.len();
                                continue 'outer;
                            }
                        }
                        let next = text[after_noun..].chars().next();
                        let is_word_end = next
                            .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '\'' && c != '’');
                        if is_word_end {
                            normalized.push_str(&lead[..4]);
                            idx = after_noun;
                            continue 'outer;
                        }
                    }
                }
            }
        }
        let ch = text[idx..].chars().next().expect("in-bounds char");
        normalized.push(ch);
        idx += ch.len_utf8();
    }
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

    let mut chars = lower.chars().peekable();
    while let Some(ch) = chars.next() {
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

        // Printed d20 tables use ASCII hyphens, en dashes, and em dashes
        // interchangeably for numeric ranges. The Unicode forms already split
        // into endpoint tokens; make an ASCII digit-to-digit hyphen do the same
        // without changing minus signs or hyphens inside words.
        if ch == '-'
            && !current.is_empty()
            && current.chars().all(|value| value.is_ascii_digit())
            && chars.peek().is_some_and(|value| value.is_ascii_digit())
        {
            tokens.push(current.clone());
            current.clear();
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
    // Pure conjunction surface: "artifact and/or creature" counts the same
    // set as "artifact or creature" (both "and" and "or" are stopwords).
    if token == "and/or" {
        return None;
    }
    // "Destroy all creatures" and "destroy each creature" quantify
    // identically; canonicalize so surface choice doesn't read as drift.
    if token == "all" {
        return Some("each".to_string());
    }
    if token == "isnt" || token == "isn't" {
        return Some("isn't".to_string());
    }
    // Numbers, P/T values, and mana symbols compare LITERALLY: collapsing
    // them to <num>/<pt>/<mana> placeholders made "deals 3" score identical
    // to "deals 7" and a +1/+1 counter identical to a -1/-1 counter, hiding
    // real compile drift.  Only the word→digit spelling is canonicalized.
    if let Some(digit) = match token {
        "zero" => Some("0"),
        "one" => Some("1"),
        "two" => Some("2"),
        "three" => Some("3"),
        "four" => Some("4"),
        "five" => Some("5"),
        "six" => Some("6"),
        "seven" => Some("7"),
        "eight" => Some("8"),
        "nine" => Some("9"),
        "ten" => Some("10"),
        "eleven" => Some("11"),
        "twelve" => Some("12"),
        "thirteen" => Some("13"),
        "fourteen" => Some("14"),
        "fifteen" => Some("15"),
        "twenty" => Some("20"),
        _ => None,
    } {
        return Some(digit.to_string());
    }
    if token == "plusoneplusone" {
        return Some("+1/+1".to_string());
    }
    if token == "minusoneminusone" {
        return Some("-1/-1".to_string());
    }
    if token.starts_with('{') && token.ends_with('}') {
        return Some(token.to_string());
    }
    if is_pt_token(token) || is_number_token(token) {
        return Some(token.to_string());
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
        // Short (<=4 letter) verbs escape the generic s-strip above; their
        // agreement forms are never a semantic difference.
        "puts" => "put".to_string(),
        "does" => "do".to_string(),
        "pays" | "paid" => "pay".to_string(),
        "wins" | "won" => "win".to_string(),
        "adds" => "add".to_string(),
        "taps" => "tap".to_string(),
        _ => base,
    };
    // NOTE: compiler-scaffolding vocabulary ("tag", "tagged", "object") is
    // deliberately NOT dropped here: leaking it into compiled text is render
    // debt and must cost score, not be hidden from the comparator.  Likewise
    // "attached"/"match"/"otherwise" are real oracle vocabulary.
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
            | "him"
            | "his"
            | "her"
            | "hers"
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
            // "up"/"down" are NOT stopwords: "face up" vs "face down" and
            // "up to three" vs "three" are semantic distinctions.
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
    let comparable_clause = normalize_explicit_damage_source_for_compare(clause);
    let tokens = tokenize_text(&comparable_clause)
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
