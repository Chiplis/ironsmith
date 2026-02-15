use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy)]
pub struct EmbeddingConfig {
    pub dims: usize,
    pub mismatch_threshold: f32,
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
    .replace(" It has \"Sacrifice this token: Add {C}.\"", "")
    .replace(" It has \"Sacrifice this creature: Add {C}.\"", "")
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
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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

fn looks_like_named_subject(subject: &str) -> bool {
    let trimmed = subject.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
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

fn normalize_named_self_references(text: &str) -> String {
    let mut normalized = text.to_string();

    for prefix in ["When ", "Whenever "] {
        if !normalized.starts_with(prefix) {
            continue;
        }
        for marker in [
            " becomes tapped",
            " become tapped",
            " becomes untapped",
            " become untapped",
            " enters",
            " dies",
            " attacks",
            " blocks",
        ] {
            if let Some(idx) = normalized.find(marker) {
                let subject = normalized[prefix.len()..idx].trim();
                if looks_like_named_subject(subject) {
                    normalized = format!("{prefix}this creature{}", &normalized[idx..]);
                }
                break;
            }
        }
    }

    if let Some((subject, rest)) = normalized.split_once("'s power")
        && looks_like_named_subject(subject)
    {
        normalized = format!("this creature's power{rest}");
    }
    if let Some((subject, rest)) = normalized.split_once("'s power and toughness")
        && looks_like_named_subject(subject)
    {
        normalized = format!("this creature's power and toughness{rest}");
    }

    normalized
}

fn split_common_clause_conjunctions(text: &str) -> String {
    let mut normalized = text.to_string();

    normalized = strip_compiled_prefixes(&normalized);
    normalized = strip_not_named_phrase(&normalized);
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

    // Normalize clauses that omit the subject.
    if normalized.starts_with("Can't attack unless defending player controls ") {
        normalized = format!("This creature {normalized}");
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
    if let Some((left, right)) = normalized.split_once(". Deal ") {
        let right = right.trim().trim_end_matches('.').trim();
        if left.to_ascii_lowercase().contains(" deals ") && !right.is_empty() {
            normalized = format!("{} and {}", left.trim_end_matches('.'), right);
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
            normalized = format!(
                "{left} to the battlefield. Put {counter_phrase} counter on it."
            );
        }
    }
    if let Some((left, right)) = normalized.split_once(". Put ")
        && (left.starts_with("Bolster ") || left.starts_with("bolster "))
    {
        normalized = format!("{}, then put {}", left.trim_end_matches('.'), right);
    }
    normalized = normalized
        .replace("this enchantment enters", "this permanent enters")
        .replace("This enchantment enters", "This permanent enters")
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
        .replace(
            "target creature an opponent controls",
            "target creature you don't control",
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
            "Exile 1 card(s) from your hand",
            "Exile a card from your hand",
        )
        .replace(
            ", choose another target attacking creature. another target attacking creature ",
            ", another target attacking creature ",
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

    let normalized = normalized
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
        .replace("that player controls", "they control")
        .replace(" to their owners' hands", " to their owner's hand")
        .replace(" to their owners hand", " to their owner's hand")
        .replace(" to its owner's hand", " to their owner's hand")
        .replace("sacrifice a creature you control", "sacrifice a creature")
        .replace("Sacrifice a creature you control", "Sacrifice a creature")
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
            "Destroy target tagged object 'enchanted'",
            "Destroy enchanted creature",
        )
        .replace(
            "destroy target tagged object 'enchanted'",
            "destroy enchanted creature",
        )
        .replace("Counter spell", "Counter that spell")
        .replace("counter spell", "counter that spell");
    let mut normalized = normalized;

    if let Some((left, right)) = normalized.split_once(". Proliferate") {
        let left = left.trim().trim_end_matches('.');
        let right_tail = right
            .trim_start_matches('.')
            .trim_start_matches(',')
            .trim();
        if right_tail.is_empty() {
            normalized = format!("{left}, then proliferate.");
        } else {
            normalized = format!("{left}, then proliferate. {right_tail}");
        }
    } else if let Some((left, right)) = normalized.split_once(". proliferate") {
        let left = left.trim().trim_end_matches('.');
        let right_tail = right
            .trim_start_matches('.')
            .trim_start_matches(',')
            .trim();
        if right_tail.is_empty() {
            normalized = format!("{left}, then proliferate.");
        } else {
            normalized = format!("{left}, then proliferate. {right_tail}");
        }
    }

    if let Some((left, right)) = normalized.split_once(". Scry ") {
        let left = left.trim().trim_end_matches('.');
        let scry_tail = right.trim().trim_end_matches('.');
        if scry_tail
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit() || ch.eq_ignore_ascii_case(&'x'))
        {
            normalized = format!("{left}, then scry {scry_tail}.");
        }
    } else if let Some((left, right)) = normalized.split_once(". scry ") {
        let left = left.trim().trim_end_matches('.');
        let scry_tail = right.trim().trim_end_matches('.');
        if scry_tail
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit() || ch.eq_ignore_ascii_case(&'x'))
        {
            normalized = format!("{left}, then scry {scry_tail}.");
        }
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

fn normalize_explicit_damage_source_clause(line: &str) -> String {
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

    let mut out = expanded.join(" ");
    if !ability_prefix.is_empty() {
        let first = expanded[0].clone();
        out = format!("{ability_prefix}{first}");
        if expanded.len() > 1 {
            out.push(' ');
            out.push_str(&expanded[1..].join(" "));
        }
    }
    normalize_clause_line(&out)
}

fn semantic_clauses(text: &str) -> Vec<String> {
    let mut clauses = Vec::new();
    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let line = if trimmed.starts_with('(') && trimmed.ends_with(')') {
            let inner = trimmed.trim_start_matches('(').trim_end_matches(')').trim();
            // Keep parenthetical lines only when they carry executable semantics
            // (most notably mana abilities like "({T}: Add {G}.)").
            if inner.contains(':') {
                inner.to_string()
            } else {
                continue;
            }
        } else {
            let no_parenthetical = strip_parenthetical(raw_line);
            let no_inline_reminder = strip_inline_token_reminders(&no_parenthetical);
            let no_quote_reminder = strip_reminder_like_quotes(&no_inline_reminder);
            normalize_clause_line(&no_quote_reminder)
        };
        let line = split_common_clause_conjunctions(&line);
        let line = normalize_named_self_references(&line);
        let line = normalize_explicit_damage_source_clause(&line);
        let line = expand_create_list_clause(&normalize_clause_line(&line));
        let line = expand_return_list_clause(&line);
        if line.is_empty() {
            continue;
        }
        let mut current = String::new();
        for ch in line.chars() {
            if matches!(ch, '.' | ';' | '\n') {
                let trimmed = current.trim();
                if !trimmed.is_empty() && trimmed.chars().any(|ch| ch.is_ascii_alphanumeric()) {
                    clauses.push(trimmed.to_string());
                }
                current.clear();
            } else {
                current.push(ch);
            }
        }
        let trimmed = current.trim();
        if !trimmed.is_empty() && trimmed.chars().any(|ch| ch.is_ascii_alphanumeric()) {
            clauses.push(trimmed.to_string());
        }
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
    if matches!(
        token,
        "object"
            | "objects"
            | "tag"
            | "tagged"
            | "choose"
            | "chooses"
            | "chosen"
            | "matching"
            | "matches"
            | "appropriate"
            | "controller"
            | "controllers"
    ) {
        return None;
    }

    let mut base = token.trim_matches('\'').to_string();
    if base.ends_with("'s") {
        base.truncate(base.len().saturating_sub(2));
    }
    if base.len() > 4 && base.ends_with('s') {
        base.pop();
    }
    if base == "whenever" {
        base = "when".to_string();
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
            | "then"
            | "as"
            | "though"
            | "under"
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

pub fn compare_semantics_scored(
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, f32, isize, bool) {
    let oracle_clauses = semantic_clauses(oracle_text);
    let compiled_clauses = compiled_lines
        .iter()
        .flat_map(|line| semantic_clauses(strip_compiled_prefix(line)))
        .filter(|clause| !is_internal_compiled_scaffolding_clause(clause))
        .collect::<Vec<_>>();

    let oracle_tokens: Vec<Vec<String>> = oracle_clauses
        .iter()
        .map(|clause| comparison_tokens(clause))
        .filter(|tokens| !tokens.is_empty())
        .collect();
    let compiled_tokens: Vec<Vec<String>> = compiled_clauses
        .iter()
        .map(|clause| comparison_tokens(clause))
        .filter(|tokens| !tokens.is_empty())
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

pub fn compare_semantics(
    oracle_text: &str,
    compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, isize, bool) {
    let (oracle_coverage, compiled_coverage, _similarity_score, line_delta, mismatch) =
        compare_semantics_scored(oracle_text, compiled_lines, embedding);
    (oracle_coverage, compiled_coverage, line_delta, mismatch)
}

#[cfg(test)]
mod tests {
    use super::compare_semantics_scored;

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
}
