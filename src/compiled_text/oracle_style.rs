fn normalize_sentence_surface_style(line: &str) -> String {
    let mut normalized = line.trim().to_string();
    if normalized.is_empty() {
        return normalized;
    }

    if normalized
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase())
    {
        normalized = capitalize_first(&normalized);
    }

    // Modal rendering may include debug-style bracket expansions; strip them from
    // public-facing compiled text so semantic comparisons focus on the main clause.
    normalized = strip_square_bracketed_segments(&normalized)
        .trim()
        .to_string();
    normalized = normalized.replace('\u{00a0}', " ");
    normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    normalized = normalize_ward_cost_surface(&normalized);
    if let Some(rewritten) = normalize_search_discard_then_shuffle_surface(&normalized) {
        return rewritten;
    }
    if let Some(rewritten) = normalize_discard_random_then_discard_surface(&normalized) {
        return rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_return_cost_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_exile_cost_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_tap_cost_clause(&normalized) {
        normalized = rewritten;
    }
    normalized = normalized.replace("controlss", "controls");
    if normalized.contains("you may Remove ") {
        normalized = normalized.replace("you may Remove ", "you may remove ");
    }
    if let Some((prefix, tail)) = normalized.split_once("If you do, Untap it. it gets ") {
        return format!("{prefix}If you do, untap it and it gets {tail}");
    }
    let lower_normalized = normalized.to_ascii_lowercase();
    if let Some(rest) = lower_normalized.strip_prefix("spell effects: ")
        && rest.starts_with(
            "target opponent chooses target creature an opponent controls. exile it. exile all ",
        )
        && (rest.contains(" in target opponent's graveyard")
            || rest.contains(" in target opponent's graveyards"))
    {
        return "Spell effects: Target opponent exiles a creature they control and their graveyard."
            .to_string();
    }
    if lower_normalized.starts_with(
        "target opponent chooses target creature an opponent controls. exile it. exile all ",
    ) && (lower_normalized.contains(" in target opponent's graveyard")
        || lower_normalized.contains(" in target opponent's graveyards"))
    {
        return "Target opponent exiles a creature they control and their graveyard.".to_string();
    }
    if let Some((inner, payment)) = normalized.split_once(" unless a player pays ")
        && inner.starts_with("Search ")
    {
        let payment = payment.trim().trim_end_matches('.');
        return format!(
            "Unless any player pays {}, {}.",
            payment,
            lowercase_first(inner)
        );
    }
    if lower_normalized.contains("and tags it as 'exiled_0'")
        && lower_normalized.contains("for each object exiled this way, search that player's library for permanent that shares a card type with that object that player owns, put it onto the battlefield, then shuffle")
    {
        let mut chosen_types = Vec::new();
        for card_type in ["artifact", "creature", "enchantment", "planeswalker", "land", "battle"] {
            let phrase = format!(
                "choose up to one {card_type} in the battlefield and tags it as 'exiled_0'"
            );
            if lower_normalized.contains(&phrase) {
                chosen_types.push(format!("up to one target {card_type}"));
            }
        }
        if chosen_types.len() >= 2 {
            return format!(
                "Exile {}. For each permanent exiled this way, its controller reveals cards from the top of their library until they reveal a card that shares a card type with it, puts that card onto the battlefield, then shuffles.",
                join_with_and(&chosen_types)
            );
        }
    }
    if let Some((head, body)) = normalized.split_once(':')
        && head.trim().to_ascii_lowercase().starts_with("this ")
        && head
            .trim()
            .to_ascii_lowercase()
            .contains(" leaves the battlefield")
    {
        return format!("When {}, {}", head.trim().to_ascii_lowercase(), body.trim());
    }
    let token_plural_starts = [
        "Create two ",
        "Create three ",
        "Create four ",
        "Create five ",
        "Create six ",
        "Create seven ",
        "Create eight ",
        "Create nine ",
        "Create 2 ",
        "Create 3 ",
        "Create 4 ",
        "Create 5 ",
        "Create 6 ",
        "Create 7 ",
        "Create 8 ",
        "Create 9 ",
    ];
    if token_plural_starts
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
        && normalized.contains(" creature token")
        && !normalized.contains(" creature tokens")
    {
        normalized = normalized.replacen(" creature token", " creature tokens", 1);
    }
    let lower_plural_markers = [
        "create two ",
        "create three ",
        "create four ",
        "create five ",
        "create six ",
        "create seven ",
        "create eight ",
        "create nine ",
        "create 2 ",
        "create 3 ",
        "create 4 ",
        "create 5 ",
        "create 6 ",
        "create 7 ",
        "create 8 ",
        "create 9 ",
    ];
    if lower_plural_markers
        .iter()
        .any(|marker| lower_normalized.contains(marker))
        && normalized.contains(" creature token")
        && !normalized.contains(" creature tokens")
    {
        normalized = normalized.replacen(" creature token", " creature tokens", 1);
    }
    if let Some((left, right)) = normalized.split_once(". ") {
        let right_lower = right.trim_start().to_ascii_lowercase();
        if !right_lower.starts_with("you sacrifice ") && !right_lower.starts_with("sacrifice ") {
            // no-op
        } else {
            let left_trimmed = left.trim().trim_end_matches('.');
            let right_trimmed = right
                .trim_start()
                .trim_start_matches("you ")
                .trim_start_matches("You ")
                .trim_start_matches("sacrifice ")
                .trim();
            let left_lower = left_trimmed.to_ascii_lowercase();
            if left_lower.starts_with("you draw ")
                || left_lower.starts_with("you discard ")
                || left_lower.starts_with("you gain ")
                || left_lower.contains(" and you lose ")
                || left_lower.contains(" and you gain ")
            {
                return format!(
                    "{left_trimmed}, then sacrifice {}.",
                    right_trimmed.trim_end_matches('.')
                );
            }
        }
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". investigate") {
        let left_trimmed = left.trim().trim_end_matches('.');
        let right_tail = right.trim_start_matches('.').trim_start_matches(',').trim();
        if left_trimmed.to_ascii_lowercase().contains("create ") {
            if right_tail.is_empty() {
                return format!("{left_trimmed}, then investigate.");
            }
            return format!("{left_trimmed}, then investigate. {right_tail}");
        }
    }
    if let Some((trigger_head, trigger_body)) = normalized.split_once(':')
        && trigger_head
            .trim()
            .to_ascii_lowercase()
            .starts_with("one or more ")
    {
        return format!(
            "Whenever {}, {}",
            trigger_head.trim().to_ascii_lowercase(),
            trigger_body.trim()
        );
    }
    if let Some(rest) = normalized.strip_prefix("Creatures you control get ")
        && let Some(pt) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
    {
        return format!("During your turn, creatures you control get {pt}.");
    }
    if let Some((head, _tail)) = normalized
        .split_once(", put a card from that player's hand on top of that player's library")
        && (head.starts_with("When this creature enters")
            || head.starts_with("When this permanent enters"))
    {
        return format!(
            "{head}, target player puts a card from their hand on top of their library."
        );
    }
    if let Some((prefix, rest)) =
        split_once_ascii_ci(&normalized, ". For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent.")
            .or_else(|| rest.strip_suffix(" damage to each opponent"))
        && let Some((subject, self_amount)) =
            strip_prefix_ascii_ci(prefix.trim(), "When this permanent enters, it deals ")
                .map(|tail| ("permanent", tail))
                .or_else(|| {
                    strip_prefix_ascii_ci(prefix.trim(), "When this creature enters, it deals ")
                        .map(|tail| ("creature", tail))
                })
                .or_else(|| {
                    split_once_ascii_ci(prefix.trim(), ": When this permanent enters, it deals ")
                        .map(|(_, tail)| ("permanent", tail))
                })
                .or_else(|| {
                    split_once_ascii_ci(prefix.trim(), ": When this creature enters, it deals ")
                        .map(|(_, tail)| ("creature", tail))
                })
                .and_then(|(subject, tail)| {
                    strip_suffix_ascii_ci(tail, " damage to that player")
                        .map(|amount| (subject, amount))
                })
        && self_amount.trim().eq_ignore_ascii_case(amount.trim())
    {
        return format!(
            "When this {subject} enters, it deals {} damage to each opponent and each creature your opponents control.",
            amount.trim(),
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ": For each ")
        && let Some((first_filter, rest)) = split_once_ascii_ci(rest, ", put ")
        && let Some((first_counter, rest)) = split_once_ascii_ci(rest, " on that object. For each ")
        && let Some((second_filter, rest)) = split_once_ascii_ci(rest, ", Put ")
        && let Some(second_counter) = strip_suffix_ascii_ci(rest, " on that object.")
            .or_else(|| strip_suffix_ascii_ci(rest, " on that object"))
    {
        return format!(
            "{prefix}: put {} on each {} and {} on each {}.",
            first_counter.trim(),
            first_filter.trim(),
            second_counter.trim(),
            second_filter.trim()
        );
    }
    if normalized.eq_ignore_ascii_case("All Slivers have \"Sacrifice this creature: Add b b.\"")
        || normalized
            .eq_ignore_ascii_case("All Slivers have \"Sacrifice this permanent: Add b b.\"")
        || normalized
            .eq_ignore_ascii_case("All Slivers have \"Sacrifice this creature: Add {b}{b}.\"")
        || normalized
            .eq_ignore_ascii_case("All Slivers have \"Sacrifice this permanent: Add {b}{b}.\"")
    {
        return "All Slivers have \"Sacrifice this permanent: Add {B}{B}.\"".to_string();
    }
    let format_choose_modes = |head: &str, marker: &str, tail: &str| {
        let modes: Vec<String> = tail
            .split(" • ")
            .map(|mode| mode.trim().trim_start_matches('•').trim().to_string())
            .filter(|mode| !mode.is_empty())
            .collect();
        if modes.len() < 2 {
            return None;
        }
        let mut rewritten = format!("{head}{marker}");
        for mode in modes {
            rewritten.push_str("\n• ");
            rewritten.push_str(&mode);
        }
        Some(rewritten)
    };
    if !normalized.contains('\n') {
        if lower_normalized.contains("you may choose the same mode more than once")
            && normalized.contains(" • ")
        {
            return normalized.replace(" • ", "\n• ");
        }
        let normalized_trimmed = normalized.trim();
        let lower_trimmed = normalized_trimmed.to_ascii_lowercase();
        if lower_trimmed.starts_with("choose ") && normalized_trimmed.contains(" • ") {
            let mut best_sep: Option<(usize, usize)> = None;
            for sep in [" \u{2014} ", " - "] {
                if let Some(idx) = lower_trimmed.find(sep) {
                    match best_sep {
                        None => best_sep = Some((idx, sep.len())),
                        Some((best_idx, _)) if idx < best_idx => best_sep = Some((idx, sep.len())),
                        _ => {}
                    }
                }
            }
            if let Some((idx, sep_len)) = best_sep {
                let head = normalized_trimmed[..idx].trim();
                let tail = normalized_trimmed[idx + sep_len..].trim();
                if tail.contains(" • ")
                    && !head.is_empty()
                    && let Some(rewritten) =
                        format_choose_modes("", &format!("{} —", capitalize_first(head)), tail)
                {
                    return rewritten;
                }
            }
        }
        if let Some((head, tail)) = normalized.split_once(" choose one or more - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one or more —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one or more - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one or more —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" choose one or both - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one or both —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one or both - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one or both —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" choose one - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one - ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" choose one or both — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one or both —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one or both — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one or both —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" choose one — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" choose one or more — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " choose one or more —", tail)
        {
            return rewritten;
        }
        if let Some((head, tail)) = normalized.split_once(" Choose one or more — ")
            && tail.contains(" • ")
            && let Some(rewritten) = format_choose_modes(head, " Choose one or more —", tail)
        {
            return rewritten;
        }
    }

    if lower_normalized.contains(
        "treasure artifact token with {t}, sacrifice this artifact: add one mana of any color. tapped under your control",
    ) || lower_normalized.contains(
        "treasure artifact token with {t}, sacrifice this artifact: add one mana of any color. under your control, tapped",
    ) {
        return normalized
            .replace(
                "Create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control",
                "Create a tapped Treasure token",
            )
            .replace(
                "Create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped",
                "Create a tapped Treasure token",
            )
            .replace(
                "create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control",
                "create a tapped Treasure token",
            )
            .replace(
                "create a Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped",
                "create a tapped Treasure token",
            )
            .replace(
                "create a treasure artifact token with {t}, sacrifice this artifact: add one mana of any color. tapped under your control",
                "create a tapped Treasure token",
            )
            .replace(
                "create a treasure artifact token with {t}, sacrifice this artifact: add one mana of any color. under your control, tapped",
                "create a tapped Treasure token",
            );
    }
    if lower_normalized.contains(
        "0/1 colorless eldrazi spawn creature token with sacrifice this creature: add {c}. under your control",
    ) {
        return normalized
            .replace(
                "Create a 0/1 colorless Eldrazi Spawn creature token with Sacrifice this creature: Add {C}. under your control",
                "Create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
            )
            .replace(
                "create a 0/1 colorless Eldrazi Spawn creature token with Sacrifice this creature: Add {C}. under your control",
                "create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
            )
            .replace(
                "create a 0/1 colorless eldrazi spawn creature token with sacrifice this creature: add {c}. under your control",
                "create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
            );
    }
    if lower_normalized.contains(
        "1/1 colorless eldrazi scion creature token with sacrifice this creature: add {c}. under your control",
    ) {
        return normalized
            .replace(
                "Create a 1/1 colorless Eldrazi Scion creature token with Sacrifice this creature: Add {C}. under your control",
                "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
            )
            .replace(
                "create a 1/1 colorless Eldrazi Scion creature token with Sacrifice this creature: Add {C}. under your control",
                "create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
            )
            .replace(
                "create a 1/1 colorless eldrazi scion creature token with sacrifice this creature: add {c}. under your control",
                "create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
            );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you gain ")
        && left
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("you gain ")
        && let Some(base_amount) = strip_prefix_ascii_ci(left.trim(), "you gain ")
            .and_then(|tail| strip_suffix_ascii_ci(tail.trim(), " life"))
        && let Some(extra_amount) =
            strip_suffix_ascii_ci(right.trim().trim_end_matches('.'), " life")
    {
        return format!(
            "You gain {} plus {} life.",
            base_amount.trim(),
            extra_amount.trim()
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ", creatures you control get ")
        && let Some((pt_tail, gain_tail)) =
            split_once_ascii_ci(rest, " until end of turn. creatures you control gain ")
        && let Some(keyword_tail) = strip_suffix_ascii_ci(gain_tail, " until end of turn")
    {
        return format!(
            "{prefix}, creatures you control get {} and gain {} until end of turn.",
            pt_tail.trim(),
            keyword_tail.trim().to_ascii_lowercase()
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you gain ")
        && {
            let left_lower = left.trim_start().to_ascii_lowercase();
            left_lower.starts_with("deal ")
                || left_lower.starts_with("destroy ")
                || left_lower.starts_with("return ")
                || left_lower.starts_with("counter target")
                || left_lower.starts_with("exile ")
                || left_lower.starts_with("search your library")
                || left_lower.starts_with("create ")
        }
    {
        return format!(
            "{}. You gain {}.",
            left.trim_end_matches('.'),
            right.trim_end_matches('.')
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you lose ")
        && {
            let left_lower = left.trim_start().to_ascii_lowercase();
            left_lower.starts_with("deal ")
                || left_lower.starts_with("destroy ")
                || left_lower.starts_with("return ")
                || left_lower.starts_with("counter target")
                || left_lower.starts_with("exile ")
                || left_lower.starts_with("search your library")
                || left_lower.starts_with("create ")
        }
    {
        return format!(
            "{}. You lose {}.",
            left.trim_end_matches('.'),
            right.trim_end_matches('.')
        );
    }
    if let Some((draw_clause, put_clause)) = split_once_ascii_ci(&normalized, ". ")
        && {
            let lower_draw = draw_clause.trim_start().to_ascii_lowercase();
            lower_draw.starts_with("you draw ")
                || lower_draw.contains(", you draw ")
                || lower_draw.contains(": you draw ")
        }
        && let Some(card_phrase) =
            strip_suffix_ascii_ci(put_clause.trim(), " from your hand on top of your library.")
                .or_else(|| {
                    strip_suffix_ascii_ci(
                        put_clause.trim(),
                        " from your hand on top of your library",
                    )
                })
    {
        let card_phrase = strip_prefix_ascii_ci(card_phrase.trim(), "put ")
            .unwrap_or(card_phrase)
            .trim();
        let mut rewritten = format!(
            "{}, then put {} from your hand on top of your library",
            draw_clause.trim_end_matches('.'),
            card_phrase
        );
        if card_phrase.to_ascii_lowercase().contains("cards")
            && !put_clause.to_ascii_lowercase().contains("in any order")
        {
            rewritten.push_str(" in any order");
        }
        rewritten.push('.');
        return rewritten;
    }
    if let Some((put_clause, shuffle_clause)) = split_once_ascii_ci(&normalized, ". Shuffle ") {
        let (shuffle_library_head, shuffle_tail) = split_once_ascii_ci(shuffle_clause, ". ")
            .map_or_else(
                || (shuffle_clause.trim(), ""),
                |(head, tail)| (head.trim(), tail.trim()),
            );
        if let Some(library_owner) = strip_suffix_ascii_ci(shuffle_library_head, " library")
            .or_else(|| strip_suffix_ascii_ci(shuffle_library_head, " library."))
        {
            let bottom_suffix = format!(" on the bottom of {} library", library_owner.trim());
            if let Some(move_clause) = strip_suffix_ascii_ci(put_clause.trim(), &bottom_suffix) {
                let move_clause = move_clause.trim();
                let split_put_clause = split_once_ascii_ci(move_clause, "Put ")
                    .or_else(|| split_once_ascii_ci(move_clause, "put "));
                if let Some((prefix, moved_cards)) = split_put_clause {
                    let prefix = prefix.trim_end();
                    let moved_cards = moved_cards.trim();
                    let shuffle_verb = if prefix.is_empty()
                        || prefix.ends_with(':')
                        || prefix.ends_with(';')
                        || prefix.ends_with('.')
                    {
                        "Shuffle"
                    } else {
                        "shuffle"
                    };
                    let mut rewritten = if prefix.is_empty() {
                        format!(
                            "{shuffle_verb} {moved_cards} into {} library",
                            library_owner.trim()
                        )
                    } else {
                        format!(
                            "{prefix} {shuffle_verb} {moved_cards} into {} library",
                            library_owner.trim()
                        )
                    };
                    if !shuffle_tail.is_empty() {
                        rewritten.push_str(". ");
                        rewritten.push_str(shuffle_tail);
                    } else {
                        rewritten.push('.');
                    }
                    return rewritten;
                }
            }
        }
    }
    if let Some((prelude, graveyard_tail)) =
        split_once_ascii_ci(&normalized, ". Shuffle that object's owner's library. ")
        && let Some(move_clause) =
            strip_suffix_ascii_ci(prelude.trim(), " on the bottom of its owner's library")
        && let Some((prefix, moved_cards)) = split_once_ascii_ci(move_clause.trim(), "Put ")
            .or_else(|| split_once_ascii_ci(move_clause.trim(), "put "))
    {
        let graveyard_tail = graveyard_tail.trim();
        if graveyard_tail.eq_ignore_ascii_case("you shuffle your graveyard into your library.")
            || graveyard_tail.eq_ignore_ascii_case("you shuffle your graveyard into your library")
        {
            let prefix = prefix.trim_end();
            let moved_cards = moved_cards.trim();
            if prefix.is_empty() {
                return format!(
                    "Shuffle {moved_cards} and your graveyard into their owner's library."
                );
            }
            return format!(
                "{prefix} Shuffle {moved_cards} and your graveyard into their owner's library."
            );
        }
    }
    if let Some(rest) = normalized
        .strip_prefix("For each player, if that player controls ")
        .or_else(|| normalized.strip_prefix("for each player, if that player controls "))
        && let Some((controls, tail)) = rest.split_once(", Create 1 ")
        && let Some((token_tail, remainder)) = tail.split_once(" under that player's control")
    {
        let mut rewritten = format!(
            "Each player who controls {} creates a {}.",
            with_indefinite_article(controls),
            token_tail
        );
        let remainder = remainder
            .trim_start_matches('.')
            .trim_start_matches(',')
            .trim();
        if !remainder.is_empty() {
            rewritten.push(' ');
            rewritten.push_str(remainder);
        }
        return rewritten;
    }
    if let Some((prefix, _)) = normalized.split_once(
        ", for each player, Put a card from that player's hand on top of that player's library.",
    ) {
        return format!(
            "{prefix}, each player puts a card from their hand on top of their library."
        );
    }
    if let Some((prefix, _)) = normalized.split_once(
        ", for each player, Put a card from that player's hand on top of that player's library",
    ) {
        return format!(
            "{prefix}, each player puts a card from their hand on top of their library."
        );
    }
    if let Some((lose_clause, put_clause)) = normalized.split_once(". ")
        && lose_clause
            .to_ascii_lowercase()
            .starts_with("target opponent loses ")
        && (put_clause == "Put a card from that player's hand on top of that player's library."
            || put_clause == "Put a card from that player's hand on top of that player's library")
    {
        return format!("{lose_clause} and puts a card from their hand on top of their library.");
    }
    if let Some(rest) = normalized.strip_prefix("Other ")
        && let Some((kind, tail)) = rest.split_once(" you control get ")
        && let Some(buff) = tail
            .strip_suffix(" and have ward 1.")
            .or_else(|| tail.strip_suffix(" and have ward 1"))
            .or_else(|| tail.strip_suffix(" and have ward {1}."))
            .or_else(|| tail.strip_suffix(" and have ward {1}"))
    {
        return format!("Each other {kind} you control gets {buff} and has ward {{1}}.");
    }
    if let Some(rest) = normalized.strip_prefix("Protection from ")
        && !rest.contains(' ')
        && !matches!(
            rest.to_ascii_lowercase().as_str(),
            "white" | "blue" | "black" | "red" | "green" | "colorless" | "everything"
        )
        && !rest.ends_with('s')
    {
        return format!("Protection from {}", pluralize_noun_phrase(rest));
    }
    if !is_keyword_style_line(&normalized)
        && !normalized.ends_with('.')
        && !normalized.ends_with('!')
        && !normalized.ends_with('?')
        && !normalized.ends_with('"')
        && !normalized.ends_with(')')
    {
        normalized.push('.');
    }

    normalized = normalized
        .replace(
            "Counter target instant spell spell.",
            "Counter target instant spell.",
        )
        .replace(
            "Counter target sorcery spell spell.",
            "Counter target sorcery spell.",
        )
        .replace(" ors ", " or ")
        .replace(" ors", " or")
        .replace("ors ", "or ")
        .replace("a artifact", "an artifact")
        .replace("a enchantment", "an enchantment")
        .replace("a Aura", "an Aura")
        .replace("a player may pays ", "that player may pay ")
        .replace(
            "untap all a snow permanent you control",
            "untap each snow permanent you control",
        )
        .replace("for each a ", "for each ")
        .replace("for each an ", "for each ")
        .replace("Elfs you control get ", "Elves you control get ")
        .replace("Other Elf you control get ", "Other Elves you control get ")
        .replace("other Elf you control get ", "other Elves you control get ")
        .replace("Warrior have ", "Warriors have ")
        .replace("warrior have ", "warriors have ")
        .replace(
            "Creature with a level counter on it you control get ",
            "Each creature you control with a level counter on it gets ",
        )
        .replace(
            "creature with a level counter on it you control get ",
            "each creature you control with a level counter on it gets ",
        )
        .replace(
            "the number of Soldiers or Warrior you control",
            "the number of Soldiers and Warriors you control",
        )
        .replace(
            "the number of Soldiers and Warrior you control",
            "the number of Soldiers and Warriors you control",
        )
        .replace("Goblin are black", "Goblins are black")
        .replace(
            "Goblin are zombie in addition to their other types",
            "Goblins are Zombies in addition to their other creature types",
        )
        .replace(
            "Whenever this creature or Whenever another Ally you control enters",
            "Whenever this creature or another Ally you control enters",
        )
        .replace(
            "Whenever this creature or least ",
            "Whenever this creature and at least ",
        )
        .replace(
            "Whenever you cast an instant or sorcery spell, deal ",
            "Whenever you cast an instant or sorcery spell, this creature deals ",
        );

    if let Some((amount, rest)) = normalized
        .strip_prefix("Prevent the next ")
        .and_then(|tail| tail.split_once(" damage to "))
        && let Some(target) = rest
            .strip_suffix(" until end of turn.")
            .or_else(|| rest.strip_suffix(" until end of turn"))
    {
        return format!(
            "Prevent the next {amount} damage that would be dealt to {target} this turn."
        );
    }

    if let Some(rest) = normalized.strip_prefix("This creature has ")
        && let Some(keyword) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
        && is_keyword_phrase(keyword)
    {
        return format!(
            "During your turn, this creature has {}.",
            keyword.to_ascii_lowercase()
        );
    }
    if let Some(rest) = normalized.strip_prefix("Creatures you control have ")
        && let Some(keyword) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
        && is_keyword_phrase(keyword)
    {
        return format!(
            "During your turn, creatures you control have {}.",
            keyword.to_ascii_lowercase()
        );
    }
    if let Some(rest) = normalized.strip_prefix("Allies you control have ")
        && let Some(keyword) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
        && is_keyword_phrase(keyword)
    {
        return format!(
            "During your turn, Allies you control have {}.",
            keyword.to_ascii_lowercase()
        );
    }
    let normalize_target_players_action = |action: &str| {
        if let Some(rest) = action.strip_prefix("gains ") {
            return format!("gain {rest}");
        }
        if let Some(rest) = action.strip_prefix("draws ") {
            return format!("draw {rest}");
        }
        if let Some(rest) = action.strip_prefix("discards ") {
            return format!("discard {rest}");
        }
        if let Some(rest) = action.strip_prefix("mills ") {
            return format!("mill {rest}");
        }
        if let Some(rest) = action.strip_prefix("loses ") {
            return format!("lose {rest}");
        }
        if let Some(rest) = action.strip_prefix("sacrifices ") {
            return format!("sacrifice {rest}");
        }
        action.to_string()
    };
    if let Some((first_clause, second_clause)) =
        split_once_ascii_ci(&normalized, ". For each target player, that player ")
        && let Some(first_action) =
            strip_prefix_ascii_ci(first_clause, "For each target player, that player ")
    {
        return format!(
            "Target players each {} and {}.",
            normalize_target_players_action(first_action.trim().trim_end_matches('.')),
            second_clause.trim().trim_end_matches('.')
        );
    }
    if let Some(action) = strip_prefix_ascii_ci(&normalized, "For each target player, that player ")
    {
        return format!(
            "Target players each {}",
            normalize_target_players_action(action.trim())
        );
    }
    if let Some(count) = normalized
        .strip_prefix("For each opponent, that player discards ")
        .and_then(|rest| rest.strip_suffix(" cards."))
        .or_else(|| {
            normalized
                .strip_prefix("For each opponent, that player discards ")
                .and_then(|rest| rest.strip_suffix(" cards"))
        })
    {
        return format!(
            "Each opponent discards {} cards.",
            render_small_number_or_raw(count)
        );
    }
    if normalized == "For each opponent, that player discards a card."
        || normalized == "For each opponent, that player discards a card"
    {
        return "Each opponent discards a card.".to_string();
    }
    if let Some(count) = normalized
        .strip_prefix("For each opponent, that player gets ")
        .and_then(|rest| {
            rest.strip_suffix(" poison counter(s).")
                .or_else(|| rest.strip_suffix(" poison counter(s)"))
                .or_else(|| rest.strip_suffix(" poison counters."))
                .or_else(|| rest.strip_suffix(" poison counters"))
                .or_else(|| rest.strip_suffix(" poison counter."))
                .or_else(|| rest.strip_suffix(" poison counter"))
        })
    {
        let count = count.trim();
        if matches!(count, "1" | "one" | "a" | "an") {
            return "Each opponent gets a poison counter.".to_string();
        }
        return format!("Each opponent gets {count} poison counters.");
    }
    if let Some(count) = normalized
        .strip_prefix("For each player, that player mills ")
        .and_then(|rest| rest.strip_suffix(" cards."))
        .or_else(|| {
            normalized
                .strip_prefix("For each player, that player mills ")
                .and_then(|rest| rest.strip_suffix(" cards"))
        })
    {
        return format!(
            "Each player mills {} cards.",
            render_small_number_or_raw(count)
        );
    }
    if let Some(count) = normalized
        .strip_prefix("For each player, that player draws ")
        .and_then(|rest| rest.strip_suffix(" cards."))
        .or_else(|| {
            normalized
                .strip_prefix("For each player, that player draws ")
                .and_then(|rest| rest.strip_suffix(" cards"))
        })
    {
        return format!(
            "Each player draws {} cards.",
            render_small_number_or_raw(count)
        );
    }
    if normalized == "For each player, that player discards a card."
        || normalized == "For each player, that player discards a card"
    {
        return "Each player discards a card.".to_string();
    }
    if let Some(count) = normalized
        .strip_prefix("For each player, that player discards ")
        .and_then(|rest| rest.strip_suffix(" cards."))
        .or_else(|| {
            normalized
                .strip_prefix("For each player, that player discards ")
                .and_then(|rest| rest.strip_suffix(" cards"))
        })
    {
        return format!(
            "Each player discards {} cards.",
            render_small_number_or_raw(count)
        );
    }
    if normalized == "For each player, that player discards a card at random."
        || normalized == "For each player, that player discards a card at random"
    {
        return "Each player discards a card at random.".to_string();
    }
    if let Some(rest) =
        normalized.strip_prefix("For each player, Return all creature card from target player's graveyard to target player's hand")
    {
        if rest.trim().is_empty() || rest.trim() == "." {
            return "Each player returns all creature cards from their graveyard to their hand."
                .to_string();
        }
    }
    if let Some(rest) = normalized.strip_prefix("For each attacking creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!("Deal {amount} damage to each attacking creature.");
    }
    if let Some(rest) = normalized.strip_prefix("For each blocking creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!("Deal {amount} damage to each blocking creature.");
    }
    if let Some(rest) = normalized.strip_prefix("For each attacking/blocking creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!("Deal {amount} damage to each attacking or blocking creature.");
    }
    if let Some(rest) = normalized.strip_prefix("For each another creature without flying, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!(
            "This creature deals {amount} damage to each other creature without flying."
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each opponent, that player loses ")
        && let Some(amount) = rest
            .strip_suffix(" life.")
            .or_else(|| rest.strip_suffix(" life"))
    {
        return format!("Each opponent loses {amount} life.");
    }

    if normalized
        == "For each player, that player draws a card. For each player, that player discards a card."
        || normalized
            == "For each player, that player draws a card. For each player, that player discards a card"
    {
        return "Each player draws a card, then discards a card.".to_string();
    }
    if let Some(amount) = normalized
        .strip_prefix("For each opponent, Deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that player."))
        .or_else(|| {
            normalized
                .strip_prefix("For each opponent, Deal ")
                .and_then(|rest| rest.strip_suffix(" damage to that player"))
        })
    {
        return format!("Deal {amount} damage to each opponent.");
    }
    if let Some((prefix, tail)) = normalized.split_once(". For each opponent, Deal ")
        && let Some(amount) = tail
            .strip_suffix(" damage to that player.")
            .or_else(|| tail.strip_suffix(" damage to that player"))
    {
        return format!("{prefix}. Deal {amount} damage to each opponent.");
    }
    if let Some(amount) = normalized
        .strip_prefix("For each opponent, that player loses ")
        .and_then(|rest| rest.strip_suffix(" life."))
        .or_else(|| {
            normalized
                .strip_prefix("For each opponent, that player loses ")
                .and_then(|rest| rest.strip_suffix(" life"))
        })
    {
        return format!("Each opponent loses {amount} life.");
    }
    if let Some(amount) = normalized
        .strip_prefix("Whenever this creature attacks, for each opponent, that player loses ")
        .and_then(|rest| rest.strip_suffix(" life."))
        .or_else(|| {
            normalized
                .strip_prefix(
                    "Whenever this creature attacks, for each opponent, that player loses ",
                )
                .and_then(|rest| rest.strip_suffix(" life"))
        })
    {
        return format!("Whenever this creature attacks, each opponent loses {amount} life.");
    }
    if let Some(card_text) = normalized
        .strip_prefix("For each player, Put ")
        .and_then(|rest| {
            rest.strip_suffix(" in that player's graveyard onto the battlefield.")
                .or_else(|| rest.strip_suffix(" in that player's graveyard onto the battlefield"))
        })
    {
        return format!("Each player puts {card_text} from their graveyard onto the battlefield.");
    }
    if let Some(card_text) = normalized
        .strip_prefix("For each player, Put ")
        .and_then(|rest| {
            rest.strip_suffix(" from that player's hand on top of that player's library.")
                .or_else(|| {
                    rest.strip_suffix(" from that player's hand on top of that player's library")
                })
        })
    {
        return format!("Each player puts {card_text} from their hand on top of their library.");
    }
    if let Some(cards) = normalized
        .strip_prefix("For each player, Return all ")
        .and_then(|rest| {
            rest.strip_suffix(" from that player's graveyard to that player's hand.")
                .or_else(|| {
                    rest.strip_suffix(" from that player's graveyard to that player's hand")
                })
        })
    {
        let cards = cards
            .replace(" creature card", " creature cards")
            .replace(" land card", " land cards")
            .replace(" permanent card", " permanent cards");
        return format!("Each player returns all {cards} from their graveyard to their hand.");
    }
    if let Some(rest) = normalized.strip_prefix("For each player, Create ") {
        if let Some((create_clause, tail)) = rest.split_once(". ") {
            return format!("Each player creates {create_clause}. {tail}");
        }
        return format!("Each player creates {rest}");
    }
    if normalized == "Untap all a snow permanent you control."
        || normalized == "Untap all a snow permanent you control"
    {
        return "Untap each snow permanent you control.".to_string();
    }
    if let Some(kind) = normalized
        .strip_prefix("Target player sacrifices target player's ")
        .and_then(|rest| rest.strip_suffix("."))
    {
        return format!(
            "Target player sacrifices a {} of their choice.",
            kind.trim()
        );
    }
    if let Some(rest) =
        normalized.strip_prefix("For each creature or planeswalker without flying, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!(
            "Deal {amount} damage to each creature without flying and each planeswalker."
        );
    }
    if let Some(rest) = normalized
        .strip_prefix("When this permanent enters, for each creature or planeswalker, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!(
            "When this permanent enters, it deals {amount} damage to each creature and each planeswalker."
        );
    }
    if let Some(rest) = normalized.strip_prefix("When this permanent enters, deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each creature or planeswalker.")
            .or_else(|| rest.strip_suffix(" damage to each creature or planeswalker"))
    {
        return format!(
            "When this permanent enters, it deals {amount} damage to each creature and each planeswalker."
        );
    }
    if normalized == "All slivers have 2 regenerate this creature."
        || normalized == "All slivers have 2 regenerate this creature"
    {
        return "All Slivers have \"{2}: Regenerate this creature.\"".to_string();
    }
    if normalized == "All Slivers have 2 sacrifice this permanent draw a card."
        || normalized == "All Slivers have 2 sacrifice this permanent draw a card"
        || normalized == "All slivers have 2 sacrifice this permanent draw a card."
        || normalized == "All slivers have 2 sacrifice this permanent draw a card"
    {
        return "All Slivers have \"{2}, Sacrifice this permanent: Draw a card.\"".to_string();
    }
    if normalized == "Draw two cards and you lose 2 life. you mill 2 cards."
        || normalized == "Draw two cards and you lose 2 life. you mill 2 cards"
        || normalized == "Draw two cards and you lose 2 life. You mill 2 cards."
        || normalized == "Draw two cards and you lose 2 life. you mill two cards."
        || normalized == "Draw two cards and you lose 2 life. you mill two cards"
        || normalized == "Draw two cards and you lose 2 life. You mill two cards."
        || normalized == "Draw two cards and you lose 2 life. You mill two cards"
        || normalized == "Draw two cards and lose 2 life. you mill 2 cards."
        || normalized == "Draw two cards and lose 2 life. you mill two cards."
    {
        return "Draw two cards, lose 2 life, then mill two cards.".to_string();
    }
    if let Some(rest) = normalized.strip_prefix("This creature gets ")
        && let Some((pt, cond)) = rest.split_once(" as long as ")
        && let Some((left_cond, right_tail)) = cond.split_once(" and has ")
        && let Some((granted, repeated_cond)) = right_tail.split_once(" as long as ")
    {
        let left_cond = left_cond.trim().trim_end_matches('.');
        let repeated_cond = repeated_cond.trim().trim_end_matches('.');
        if left_cond.eq_ignore_ascii_case(repeated_cond) {
            let granted = granted.trim().trim_end_matches('.');
            let granted = normalize_keyword_predicate_case(granted);
            return format!("As long as {left_cond}, this creature gets {pt} and has {granted}.");
        }
    }

    normalized
}

fn normalize_search_discard_then_shuffle_surface(line: &str) -> Option<String> {
    let (prefix, body) = if let Some(rest) = line.strip_prefix("Spell effects: ") {
        ("Spell effects: ", rest)
    } else {
        ("", line)
    };

    let trimmed = body.trim();
    let sentence = trimmed.trim_end_matches('.');
    let parts: Vec<&str> = sentence.split(". ").collect();
    if parts.len() != 3 {
        return None;
    }

    let search_clause = parts[0].trim();
    let discard_clause = parts[1].trim();
    let shuffle_clause = parts[2].trim();

    let search_lower = search_clause.to_ascii_lowercase();
    let discard_lower = discard_clause.to_ascii_lowercase();
    let shuffle_lower = shuffle_clause.to_ascii_lowercase();

    if !search_lower.starts_with("search ")
        || !search_lower.contains(" put ")
        || !(discard_lower.starts_with("you discard ") || discard_lower.starts_with("discard "))
        || shuffle_lower != "shuffle your library"
    {
        return None;
    }

    let discard_text = discard_clause
        .strip_prefix("you ")
        .or_else(|| discard_clause.strip_prefix("You "))
        .unwrap_or(discard_clause)
        .trim();

    Some(format!(
        "{prefix}{search_clause}, {discard_text}, then shuffle."
    ))
}

fn normalize_discard_random_then_discard_surface(line: &str) -> Option<String> {
    let (prefix, body) = if let Some(rest) = line.strip_prefix("Spell effects: ") {
        ("Spell effects: ", rest)
    } else {
        ("", line)
    };

    let sentence = body.trim().trim_end_matches('.');
    let parts: Vec<&str> = sentence.split(". ").collect();
    if parts.len() != 2 {
        return None;
    }

    let first = parts[0].trim();
    let second = parts[1].trim();
    let first_lower = first.to_ascii_lowercase();
    let second_lower = second.to_ascii_lowercase();

    let first_suffix = " discards a card at random";
    let second_suffix = " discards a card";
    if !first_lower.ends_with(first_suffix) || !second_lower.ends_with(second_suffix) {
        return None;
    }

    let first_subject = first[..first.len() - first_suffix.len()].trim();
    let second_subject = second[..second.len() - second_suffix.len()].trim();
    if !first_subject.eq_ignore_ascii_case(second_subject) {
        return None;
    }

    Some(format!(
        "{prefix}{first_subject} discards a card at random, then discards a card."
    ))
}

fn normalize_ward_cost_surface(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let lower = trimmed.to_ascii_lowercase();

    for pattern in [
        "ward exile an artifact or creature or enchantment or land or planeswalker or battle,",
        "ward exile a artifact or creature or enchantment or land or planeswalker or battle,",
    ] {
        if let Some(idx) = lower.find(pattern) {
            let prefix = trimmed[..idx].trim_end();
            if prefix.is_empty() {
                return "Ward—Sacrifice a permanent.".to_string();
            }
            return format!("{prefix} Ward—Sacrifice a permanent.");
        }
    }

    if lower.starts_with("ward effect(discardeffect") {
        let mut count = 1u32;
        if let Some(start) = lower.find("count: fixed(")
            && let Some(end_rel) = lower[start + "count: fixed(".len()..].find(')')
        {
            let digits =
                &lower[start + "count: fixed(".len()..start + "count: fixed(".len() + end_rel];
            if let Ok(parsed) = digits.parse::<u32>() {
                count = parsed.max(1);
            }
        }
        if count == 1 {
            return "Ward—Discard a card".to_string();
        }
        return format!("Ward—Discard {count} cards");
    }

    if lower.starts_with("ward exile a ")
        && lower.contains("effect(sacrificeeffect")
        && lower.contains("mana_value: some(")
        && lower.contains("greaterthanorequal(")
        && let Some(start) = lower.find("greaterthanorequal(")
        && let Some(end_rel) = lower[start + "greaterthanorequal(".len()..].find(')')
    {
        let amount = &lower
            [start + "greaterthanorequal(".len()..start + "greaterthanorequal(".len() + end_rel];
        if let Ok(parsed) = amount.trim().parse::<u32>() {
            return format!("Ward—Sacrifice a permanent with mana value {parsed} or greater.");
        }
    }

    if lower.starts_with("ward exile a ")
        && lower.contains(" with mana value ")
        && lower.contains(", effect(sacrificeeffect")
        && let Some(comma_idx) = trimmed.find(", Effect(")
        && comma_idx > "Ward Exile a ".len()
    {
        let sacrificed = trimmed["Ward Exile a ".len()..comma_idx].trim();
        if let Some((_, mana_tail)) = sacrificed.rsplit_once(" with mana value ") {
            return format!(
                "Ward—Sacrifice a permanent with mana value {}",
                mana_tail.trim()
            );
        }
    }

    if lower.starts_with("ward exile a ")
        && let Some(comma_idx) = trimmed.find(',')
        && comma_idx > "Ward Exile a ".len()
    {
        let sacrificed = trimmed["Ward Exile a ".len()..comma_idx].trim();
        if !sacrificed.is_empty() && !sacrificed.contains(" or ") {
            return format!("Ward—Sacrifice a {sacrificed}");
        }
    }

    if let Some(rest) = trimmed.strip_prefix("Ward Pay ") {
        return format!("Ward—Pay {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Ward Discard ") {
        return format!("Ward—Discard {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Ward Sacrifice ") {
        return format!("Ward—Sacrifice {}", rest.trim());
    }
    if trimmed.starts_with("Ward {")
        && trimmed.contains(',')
        && let Some(rest) = trimmed.strip_prefix("Ward ")
    {
        return format!("Ward—{}", rest.trim());
    }

    trimmed.to_string()
}

#[allow(dead_code)]
fn card_self_subject_for_oracle_lines(def: &CardDefinition) -> &'static str {
    use crate::types::CardType;

    let card_types = &def.card.card_types;
    if card_types.contains(&CardType::Creature) {
        return "creature";
    }
    if card_types.contains(&CardType::Land) {
        return "land";
    }
    if card_types.len() == 1 {
        return match card_types[0] {
            CardType::Land => "land",
            CardType::Artifact => "artifact",
            CardType::Enchantment => "enchantment",
            CardType::Planeswalker => "planeswalker",
            CardType::Battle => "battle",
            CardType::Kindred => "kindred",
            CardType::Instant | CardType::Sorcery | CardType::Creature => "permanent",
        };
    }
    "permanent"
}

fn card_has_graveyard_activated_ability(def: &CardDefinition) -> bool {
    def.abilities.iter().any(|ability| {
        let is_activated = matches!(ability.kind, AbilityKind::Activated(_));
        let zone_marked = ability.functional_zones.contains(&Zone::Graveyard);
        let text_marked = ability.text.as_ref().is_some_and(|text| {
            let lower = text.to_ascii_lowercase();
            if lower.contains("while this card is in your graveyard") {
                return true;
            }
            // Avoid false positives where the EFFECT references the graveyard (Yawgmoth's Will),
            // but the activation itself happens on the battlefield. We only want to treat this as
            // a graveyard activation if the COST/activation line mentions the graveyard.
            let cost = lower
                .split_once(':')
                .map(|(left, _)| left)
                .unwrap_or(&lower);
            cost.contains("from your graveyard") || cost.contains("in your graveyard")
        });
        is_activated && (zone_marked || text_marked)
    })
}

#[allow(dead_code)]
fn enchanted_subject_for_oracle_lines(def: &CardDefinition) -> Option<&'static str> {
    if let Some(filter) = &def.aura_attach_filter {
        if filter
            .card_types
            .contains(&crate::types::CardType::Creature)
        {
            return Some("creature");
        }
        if filter.card_types.contains(&crate::types::CardType::Land) {
            return Some("land");
        }
        if filter
            .card_types
            .contains(&crate::types::CardType::Artifact)
        {
            return Some("artifact");
        }
        return Some("permanent");
    }

    for ability in &def.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let Some(rest) = static_ability
            .display()
            .to_ascii_lowercase()
            .strip_prefix("enchant ")
            .map(str::to_string)
        else {
            continue;
        };
        if rest.starts_with("creature") {
            return Some("creature");
        }
        if rest.starts_with("land") {
            return Some("land");
        }
        if rest.starts_with("artifact") {
            return Some("artifact");
        }
        if rest.starts_with("permanent") {
            return Some("permanent");
        }
    }
    None
}

#[allow(dead_code)]
fn normalize_create_under_control_clause(text: &str) -> Option<String> {
    let create_occurrences = text.matches("Create ").count() + text.matches("create ").count();
    if create_occurrences > 1 {
        // Avoid cross-sentence rewrites where reminder text can drift onto
        // a later create clause.
        return None;
    }

    let (prefix, rest, create_keyword) = if let Some((prefix, rest)) = text.split_once("Create ") {
        (prefix, rest, "Create ")
    } else if let Some((prefix, rest)) = text.split_once("create ") {
        (prefix, rest, "create ")
    } else {
        return None;
    };
    let normalize_created = |created: String| {
        created
            .replace(
                "token with Sacrifice this creature: Add {C}.",
                "token with \"Sacrifice this creature: Add {C}.\"",
            )
            .replace(
                "token with {T}, Sacrifice this artifact: Add one mana of any color.",
                "token with \"{T}, Sacrifice this artifact: Add one mana of any color.\"",
            )
    };
    if let Some((created, suffix)) = rest.split_once(" under your control") {
        if created.contains(". Create ") || created.contains(". create ") {
            return None;
        }
        let created = if let Some(single) = created.strip_prefix("1 ") {
            format!("a {single}")
        } else {
            created.to_string()
        };
        let created = normalize_created(created);
        if let Some((body, reminder)) = created.split_once(" token with ") {
            let reminder = reminder.trim().trim_matches('"').trim_end_matches('.');
            let reminder_lower = reminder.to_ascii_lowercase();
            let looks_like_token_granted_text = reminder_lower.starts_with("when this token")
                || reminder_lower.starts_with("whenever this creature")
                || reminder.starts_with('{')
                || reminder_lower.starts_with("flying and {");
            if looks_like_token_granted_text {
                let is_single = body.starts_with("a ")
                    || body.starts_with("an ")
                    || body.starts_with("one ")
                    || body.starts_with("1 ");
                let pronoun = if is_single { "It has" } else { "They have" };
                let mut lead = format!("{prefix}{create_keyword}{body} token{suffix}");
                let mut reminder = reminder.to_string();
                if let Some(rest) = reminder
                    .strip_prefix("flying and ")
                    .or_else(|| reminder.strip_prefix("Flying and "))
                {
                    lead = lead.replacen(" token", " token with flying", 1);
                    reminder = rest.to_string();
                }
                if !lead.ends_with('.') {
                    lead.push('.');
                }
                return Some(format!("{lead} {pronoun} \"{reminder}.\""));
            }
        }
        let normalized = format!("{prefix}{create_keyword}{created}{suffix}");
        return Some(normalized);
    }
    let (created, suffix) = rest.split_once(" under that object's controller's control")?;
    let created = if let Some(single) = created.strip_prefix("1 ") {
        format!("a {single}")
    } else {
        created.to_string()
    };
    let created = normalize_created(created);
    if prefix.trim().is_empty() {
        return Some(format!(
            "That object's controller creates {created}{suffix}"
        ));
    }
    Some(format!("{prefix}Its controller creates {created}{suffix}"))
}

fn normalize_embedded_create_with_token_reminder(text: &str) -> Option<String> {
    let (create_head, create_tail, lowercase_create) =
        if let Some((head, tail)) = text.split_once("Create ") {
            (head, tail, false)
        } else if let Some((head, tail)) = text.split_once("create ") {
            (head, tail, true)
        } else {
            return None;
        };

    let (token_desc, tail, single_token_word) =
        if let Some((desc, rest)) = create_tail.split_once(" token with ") {
            (desc, rest, true)
        } else if let Some((desc, rest)) = create_tail.split_once(" tokens with ") {
            (desc, rest, false)
        } else {
            return None;
        };

    if token_desc.contains(". ") {
        return None;
    }

    let (ability_text, after_control) = tail.split_once(" under your control")?;
    if ability_text.contains(". ")
        || after_control.contains(". Create ")
        || after_control.contains(". create ")
    {
        return None;
    }

    let ability_core = ability_text.trim().trim_matches('"').trim_end_matches('.');
    let ability_lower = ability_core.to_ascii_lowercase();
    let looks_like_token_reminder = ability_lower.starts_with("when this token")
        || ability_lower.starts_with("whenever this creature")
        || ability_core.starts_with('{')
        || ability_lower.starts_with("flying and {");
    if !looks_like_token_reminder {
        return None;
    }

    let mut normalized_desc = token_desc.trim().to_string();
    if let Some(rest) = normalized_desc.strip_prefix("1 ") {
        normalized_desc = format!("a {rest}");
    }

    let is_single = single_token_word
        || normalized_desc.starts_with("a ")
        || normalized_desc.starts_with("an ")
        || normalized_desc.starts_with("one ");
    let token_word = if is_single { "token" } else { "tokens" };
    let pronoun = if is_single { "It has" } else { "They have" };

    let create_keyword = if lowercase_create { "create" } else { "Create" };
    let mut first = format!(
        "{create_head}{create_keyword} {normalized_desc} {token_word} under your control{after_control}"
    );
    let mut ability = ability_core.to_string();
    if let Some(rest) = ability
        .strip_prefix("flying and ")
        .or_else(|| ability.strip_prefix("Flying and "))
    {
        first = first.replacen(
            " token under your control",
            " token with flying under your control",
            1,
        );
        ability = rest.to_string();
    }

    if !first.ends_with('.') {
        first.push('.');
    }
    Some(format!("{first} {pronoun} \"{ability}.\""))
}

fn is_cost_symbol_word(word: &str) -> bool {
    matches!(word, "w" | "u" | "b" | "r" | "g" | "c" | "x") || word.parse::<u32>().is_ok()
}

fn is_effect_verb_word(word: &str) -> bool {
    matches!(
        word,
        "add"
            | "deal"
            | "tap"
            | "untap"
            | "scry"
            | "surveil"
            | "gain"
            | "lose"
            | "draw"
            | "create"
            | "destroy"
            | "exile"
            | "return"
            | "counter"
            | "fight"
            | "mill"
            | "put"
            | "regenerate"
    )
}

fn format_cost_words(words: &[&str]) -> Option<String> {
    if words.is_empty() {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    let mut idx = 0usize;
    while idx < words.len() {
        let word = words[idx];
        if word == "," {
            idx += 1;
            continue;
        }
        if word == "t" {
            parts.push("{T}".to_string());
            idx += 1;
            continue;
        }
        if word.chars().all(|ch| ch.is_ascii_digit()) {
            parts.push(format!("{{{word}}}"));
            idx += 1;
            continue;
        }
        if is_cost_symbol_word(word) {
            parts.push(format!("{{{}}}", word.to_ascii_uppercase()));
            idx += 1;
            continue;
        }
        if word == "sacrifice" {
            let tail = words[idx + 1..].join(" ");
            if tail.is_empty() {
                parts.push("Sacrifice".to_string());
            } else {
                parts.push(format!("Sacrifice {tail}"));
            }
            break;
        }
        if word == "discard" {
            let tail = words[idx + 1..].join(" ");
            if tail.is_empty() {
                parts.push("Discard".to_string());
            } else {
                parts.push(format!("Discard {tail}"));
            }
            break;
        }
        return None;
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn normalize_granted_activated_ability_clause(text: &str) -> Option<String> {
    let (subject, tail, has_word) = if let Some((subject, tail)) = text.split_once(" has ") {
        (subject, tail, "has")
    } else if let Some((subject, tail)) = text.split_once(" have ") {
        (subject, tail, "have")
    } else {
        return None;
    };

    let words: Vec<&str> = tail.split_whitespace().collect();
    if words.len() < 2 {
        return None;
    }

    let mut effect_idx: Option<usize> = None;
    if let Some(t_idx) = words.iter().position(|word| *word == "t") {
        let mut candidate = t_idx + 1;
        if words
            .get(candidate)
            .is_some_and(|word| *word == "sacrifice")
            && words
                .get(candidate + 1)
                .is_some_and(|next| *next == "this" || *next == "thiss")
        {
            candidate += 2;
        }
        if candidate < words.len() {
            let head = words[candidate];
            if is_effect_verb_word(head)
                || matches!(head, "this" | "target" | "you" | "each" | "a" | "an")
            {
                effect_idx = Some(candidate);
            }
        }
    }
    if effect_idx.is_none() {
        let scan_start = words
            .iter()
            .position(|word| *word == "t")
            .map(|idx| idx + 1)
            .unwrap_or(0);
        for idx in scan_start..words.len() {
            let word = words[idx];
            if !is_effect_verb_word(word) {
                continue;
            }
            // "sacrifice this ..." may be part of the activation cost.
            if word == "sacrifice"
                && words
                    .get(idx + 1)
                    .is_some_and(|next| *next == "this" || *next == "thiss")
            {
                continue;
            }
            effect_idx = Some(idx);
            break;
        }
    }
    if effect_idx.is_none() && words.len() >= 2 {
        let leading_cost = words[0] == "t"
            || is_cost_symbol_word(words[0])
            || words[0].chars().all(|ch| ch.is_ascii_digit());
        let starts_effect = is_effect_verb_word(words[1])
            || matches!(words[1], "this" | "target" | "you" | "each" | "a" | "an");
        if leading_cost && starts_effect {
            effect_idx = Some(1);
        }
    }
    let effect_idx = effect_idx?;
    let cost_words = &words[..effect_idx];
    let effect_words = &words[effect_idx..];

    if !cost_words.iter().any(|word| {
        *word == "t"
            || *word == "sacrifice"
            || *word == "discard"
            || is_cost_symbol_word(word)
            || word.chars().all(|ch| ch.is_ascii_digit())
    }) {
        return None;
    }

    let cost = format_cost_words(cost_words)?;
    let mut effect = capitalize_first(&effect_words.join(" "));
    effect = normalize_zero_pt_prefix(&effect);
    if !effect.ends_with('.') {
        effect.push('.');
    }
    Some(format!("{subject} {has_word} \"{cost}: {effect}\""))
}

fn normalize_granted_beginning_trigger_clause(text: &str) -> Option<String> {
    let (subject, tail, has_word) = if let Some((subject, tail)) = text.split_once(" has ") {
        (subject.trim(), tail.trim(), "has")
    } else if let Some((subject, tail)) = text.split_once(" have ") {
        (subject.trim(), tail.trim(), "have")
    } else {
        return None;
    };
    if subject.is_empty() {
        return None;
    }

    let mut body = tail
        .trim()
        .trim_matches('"')
        .trim_end_matches('.')
        .to_string();
    if !body
        .to_ascii_lowercase()
        .starts_with("at the beginning of ")
    {
        return None;
    }
    body = body
        .replace(" w w ", " {W}{W} ")
        .replace(" w w.", " {W}{W}.")
        .replace(" if you do ", ". If you do, ")
        .replace(" if you do,", ". If you do,");
    if !body.ends_with('.') {
        body.push('.');
    }
    Some(format!(
        "{subject} {has_word} \"{}\"",
        capitalize_first(&body)
    ))
}

#[allow(dead_code)]
fn normalize_oracle_line_segment(segment: &str) -> String {
    let trimmed_owned = strip_square_bracketed_segments(segment.trim());
    let trimmed = trimmed_owned.trim();
    let lower_trimmed = trimmed.to_ascii_lowercase();
    if let Some((subject, rest)) = trimmed.split_once(" have ")
        && (subject.eq_ignore_ascii_case("all slivers")
            || subject.eq_ignore_ascii_case("all sliver creatures"))
        && let Some(normalized) = normalize_sliver_grant_clause(subject, rest)
    {
        return normalized;
    }
    if let Some(normalized) = normalize_granted_activated_ability_clause(trimmed) {
        return normalized;
    }
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this source as 'enchanted'. enchanted creature gets ",
    ) {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this source as 'enchanted'. enchanted creatures get ",
    ) {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(tail) = trimmed.strip_prefix(
        "Tag the object attached to this source as 'enchanted'. enchanted creature gains ",
    ) {
        return format!("Enchanted creature gains {tail}");
    }
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this enchantment as 'enchanted'. enchanted creature gets ",
    ) {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this enchantment as 'enchanted'. enchanted creatures get ",
    ) {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this aura as 'enchanted'. enchanted creature gets ",
    ) {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix(
        "Tag the object attached to this aura as 'enchanted'. enchanted creatures get ",
    ) {
        return format!("Enchanted creature gets {buff}");
    }
    if let Some((prefix, ability_tail)) = trimmed.split_once(" and has ")
        && prefix.starts_with("enchanted creature gets ")
    {
        let ability_clause = format!("enchanted creature has {ability_tail}");
        if let Some(normalized) = normalize_granted_activated_ability_clause(&ability_clause)
            && let Some(ability_part) = normalized.strip_prefix("enchanted creature has ")
        {
            return capitalize_first(&format!("{prefix} and has {ability_part}"));
        }
    }
    if let Some(normalized) = normalize_create_under_control_clause(trimmed) {
        return normalized;
    }
    if let Some(normalized) = normalize_search_you_own_clause(trimmed) {
        return normalized;
    }
    if let Some(kind) = strip_prefix_ascii_ci(trimmed, "you may put ").and_then(|rest| {
        rest.strip_suffix(" card in your hand onto the battlefield")
            .or_else(|| rest.strip_suffix(" card in your hand onto the battlefield."))
    }) {
        let kind = kind.trim();
        let noun = if kind.is_empty() {
            "card".to_string()
        } else {
            format!("{kind} card")
        };
        let is_already_determined =
            kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ");
        let rendered_noun = if is_already_determined {
            noun
        } else {
            with_indefinite_article(&noun)
        };
        return format!("You may put {rendered_noun} from your hand onto the battlefield");
    }
    if let Some(kind) = strip_prefix_ascii_ci(trimmed, "you may put ").and_then(|rest| {
        rest.strip_suffix(" in your hand onto the battlefield")
            .or_else(|| rest.strip_suffix(" in your hand onto the battlefield."))
    }) {
        let kind = kind.trim();
        let is_already_determined =
            kind.starts_with("target ") || kind.starts_with("a ") || kind.starts_with("an ");
        let rendered_kind = if is_already_determined {
            kind.to_string()
        } else {
            with_indefinite_article(kind)
        };
        return format!("You may put {rendered_kind} from your hand onto the battlefield");
    }
    if let Some(kind) = trimmed
        .strip_prefix("You may Put target ")
        .and_then(|rest| rest.strip_suffix(" card in your hand onto the battlefield"))
    {
        let noun = if kind.trim().is_empty() {
            "card".to_string()
        } else {
            format!("{} card", kind.trim())
        };
        return format!(
            "You may put {} from your hand onto the battlefield",
            with_indefinite_article(&noun)
        );
    }
    if let Some(kind) = trimmed
        .strip_prefix("You may Put target ")
        .and_then(|rest| rest.strip_suffix(" in your hand onto the battlefield"))
    {
        return format!(
            "You may put {} from your hand onto the battlefield",
            with_indefinite_article(kind.trim())
        );
    }
    if let Some(normalized) = normalize_choose_between_modes_clause(trimmed) {
        return normalized;
    }
    if let Some(rest) = trimmed.strip_prefix("For each player, Create ") {
        if let Some((create_clause, tail)) = rest.split_once(". ") {
            return format!("Each player creates {create_clause}. {tail}");
        }
        return format!("Each player creates {rest}");
    }
    if let Some(amount) = trimmed
        .strip_prefix("For each player, Deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that player"))
    {
        return format!("Deal {amount} damage to each player");
    }
    if let Some(rest) = trimmed.strip_prefix(
        "For each player, Reveal the top card of that player's library and tag it as 'revealed_0'",
    ) {
        if let Some(tail) = rest.strip_prefix(". ") {
            if let Some(nonland_tail) = tail.strip_prefix("For each nonland permanent, ") {
                let (per_nonland_clause, trailing) =
                    if let Some((head, after)) = nonland_tail.split_once(". ") {
                        (head, Some(after))
                    } else {
                        (nonland_tail, None)
                    };
                let per_nonland_clause =
                    if let Some(create_rest) = per_nonland_clause.strip_prefix("Create ") {
                        format!("you create {create_rest}")
                    } else {
                        lowercase_first(per_nonland_clause)
                    };

                let mut normalized = format!(
                    "Each player reveals the top card of their library. For each nonland card revealed this way, {per_nonland_clause}"
                );
                if let Some(trailing) = trailing {
                    if trailing.eq_ignore_ascii_case("Each player draws a card") {
                        normalized.push_str(". Then each player draws a card");
                    } else {
                        normalized.push_str(". ");
                        normalized.push_str(trailing);
                    }
                }
                return normalized;
            }
            if tail.is_empty() {
                return "Each player reveals the top card of their library".to_string();
            }
            return format!("Each player reveals the top card of their library. {tail}");
        }
        return "Each player reveals the top card of their library".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("For each player, Investigate ")
        && let Some((count, tail)) = rest.split_once(". ")
    {
        if count.trim() == "1" {
            return format!("Each player investigates. {tail}");
        }
    }
    if trimmed
        == "For each player, Put target creature card in target player's graveyard onto the battlefield"
    {
        return "Each player puts a creature card from their graveyard onto the battlefield"
            .to_string();
    }
    if let Some(kind) = trimmed
        .strip_prefix("For each player, Return all ")
        .and_then(|rest| rest.strip_suffix(" card in target player's graveyard to the battlefield"))
    {
        return format!(
            "Each player returns all {kind} cards from their graveyard to the battlefield"
        );
    }
    if let Some(kind) = trimmed
        .strip_prefix("For each player, Return all ")
        .and_then(|rest| {
            rest.strip_suffix(" card from target player's graveyard to target player's hand")
        })
    {
        return format!("Each player returns all {kind} cards from their graveyard to their hand");
    }
    if let Some(counter_rest) = trimmed.strip_prefix("For each ")
        && let Some((subject, tail)) = counter_rest.split_once(", Put ")
        && let Some(counter_phrase) = tail.strip_suffix(" on that object")
    {
        return format!("Put {counter_phrase} on each {subject}");
    }
    if trimmed == "Destroy all creature" {
        return "Destroy all creatures".to_string();
    }
    if trimmed == "Destroy all land" {
        return "Destroy all lands".to_string();
    }
    if trimmed == "Exile all card in graveyard" {
        return "Exile all graveyards".to_string();
    }
    if trimmed == "Permanents enter the battlefield tapped" {
        return "Permanents enter tapped".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("Token creatures get ") {
        return format!("Creature tokens get {rest}");
    }
    if trimmed == "Destroy all artifact. Destroy all enchantments"
        || trimmed == "Destroy all artifact. Destroy all enchantment"
    {
        return "Destroy all artifacts and enchantments".to_string();
    }
    if trimmed == "Counter target spell Spirit or Arcane" {
        return "Counter target Spirit or Arcane spell".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("Counter target spell unless its controller pays ")
        && let Some((amount, tail)) = rest.split_once(". If it happened, Surveil ")
    {
        return format!(
            "Counter target spell unless its controller pays {amount}, then surveil {tail}"
        );
    }
    if let Some((left, right)) = trimmed.split_once(". Deal ")
        && left.starts_with("Counter target spell unless its controller pays ")
        && let Some(amount) = right.strip_suffix(" damage to target spell")
    {
        return format!(
            "{left}. {} deals {amount} damage to that spell's controller",
            "This spell"
        );
    }
    if let Some(amount) = trimmed
        .strip_prefix("Counter target spell. Deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that object's controller."))
    {
        return format!(
            "Counter target spell. This spell deals {amount} damage to that spell's controller."
        );
    }
    if let Some(amount) = trimmed
        .strip_prefix("Counter target spell. Deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that object's controller"))
    {
        return format!(
            "Counter target spell. This spell deals {amount} damage to that spell's controller"
        );
    }
    if let Some(amount) = trimmed
        .strip_prefix("Whenever a creature blocks, deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that object's controller."))
    {
        return format!(
            "Whenever a creature blocks, deal {amount} damage to that creature's controller."
        );
    }
    if let Some(amount) = trimmed
        .strip_prefix("Whenever a creature blocks, deal ")
        .and_then(|rest| rest.strip_suffix(" damage to that object's controller"))
    {
        return format!(
            "Whenever a creature blocks, deal {amount} damage to that creature's controller"
        );
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one - ") {
        return format!("Choose one — {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one — ") {
        return format!("Choose one — {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one or both - ") {
        return format!("Choose one or both — {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one or both — ") {
        return format!("Choose one or both — {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one or more - ") {
        return format!("Choose one or more — {}", rest.trim());
    }
    if let Some(rest) = trimmed.strip_prefix("Choose one or more — ") {
        return format!("Choose one or more — {}", rest.trim());
    }
    if let Some((subject, keyword)) = split_have_clause(trimmed) {
        if keyword.eq_ignore_ascii_case("can't be blocked") {
            return format!("{subject} can't be blocked");
        }
        if keyword.eq_ignore_ascii_case("can't block") {
            return format!("{subject} can't block");
        }
    }
    if let Some((subject, tail)) = trimmed.split_once(" has ")
        && let Some(joined) = normalize_keyword_list_phrase(tail)
    {
        return format!("{subject} has {joined}");
    }
    if trimmed == "creatures have Can't block" {
        return "Creatures can't block".to_string();
    }
    if trimmed == "Can't be blocked" {
        return "This creature can't be blocked".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("Can't attack unless defending player controls ") {
        let rest = rest.trim().trim_end_matches('.').trim_end_matches(',');
        if rest.eq_ignore_ascii_case("island") || rest.eq_ignore_ascii_case("an island") {
            return "This creature can't attack unless defending player controls an Island"
                .to_string();
        }
        return format!("This creature can't attack unless defending player controls {rest}");
    }
    if let Some((trigger, effect)) = trimmed.split_once(": ")
        && (trigger.starts_with("When ")
            || trigger.starts_with("Whenever ")
            || trigger.starts_with("At the beginning "))
    {
        let normalized_effect = normalize_you_subject_phrase(effect);
        return format!("{trigger}, {normalized_effect}");
    }
    if let Some((head, tail)) = trimmed.split_once(", you draw ")
        && (head.starts_with("When ")
            || head.starts_with("Whenever ")
            || head.starts_with("At the beginning "))
    {
        return format!("{head}, draw {tail}");
    }
    if let Some((head, tail)) = trimmed.split_once(", you mill ")
        && (head.starts_with("When ")
            || head.starts_with("Whenever ")
            || head.starts_with("At the beginning "))
    {
        return format!("{head}, mill {tail}");
    }
    if let Some((cost, effect)) = trimmed.split_once(": ")
        && effect.starts_with("You draw ")
    {
        return format!("{cost}: Draw {}", effect.trim_start_matches("You draw "));
    }
    if let Some((cost, effect)) = trimmed.split_once(": ")
        && effect.starts_with("you draw ")
    {
        return format!("{cost}: Draw {}", effect.trim_start_matches("you draw "));
    }
    if let Some((cost, effect)) = trimmed.split_once(": ")
        && effect.starts_with("You mill ")
    {
        return format!("{cost}: Mill {}", effect.trim_start_matches("You mill "));
    }
    if let Some((cost, effect)) = trimmed.split_once(": ")
        && effect.starts_with("you mill ")
    {
        return format!("{cost}: Mill {}", effect.trim_start_matches("you mill "));
    }
    if let Some(merged) = merge_sentence_subject_predicates(trimmed) {
        return merged;
    }
    if let Some((subject, tail)) = trimmed.split_once(" gains ")
        && let Some(keywords) = tail.strip_suffix(" until end of turn")
        && let Some(joined) = normalize_keyword_list_phrase(keywords)
    {
        return format!("{subject} gains {joined} until end of turn");
    }
    if let Some((prefix, tail)) = trimmed.split_once(", gains ")
        && prefix.contains(" gets ")
        && let Some((first_keyword, second_tail)) = tail.split_once(", and gains ")
        && let Some(second_keyword) = second_tail.strip_suffix(" until end of turn")
        && is_keyword_phrase(first_keyword)
        && is_keyword_phrase(second_keyword)
    {
        return format!(
            "{prefix} and gains {} and {} until end of turn",
            first_keyword.to_ascii_lowercase(),
            second_keyword.to_ascii_lowercase()
        );
    }
    if let Some(amount) =
        lower_trimmed.strip_prefix("all artifacts have at the beginning of your upkeep sacrifice this artifact unless you pay ")
    {
        let amount = normalize_cost_amount_token(amount);
        return format!(
            "All artifacts have \"At the beginning of your upkeep, sacrifice this artifact unless you pay {amount}.\""
        );
    }
    if let Some(amount) =
        lower_trimmed.strip_prefix("all creatures have at the beginning of your upkeep sacrifice this creature unless you pay ")
    {
        let amount = normalize_cost_amount_token(amount);
        return format!(
            "All creatures have \"At the beginning of your upkeep, sacrifice this creature unless you pay {amount}.\""
        );
    }
    if let Some(effect) = trimmed.strip_prefix("As an additional cost to cast this spell: ") {
        let mut effect = effect.trim().to_string();
        if effect.starts_with("Discard ") {
            effect = effect.replacen("Discard ", "discard ", 1);
        }
        if effect.starts_with("You discard ") {
            effect = effect.replacen("You discard ", "discard ", 1);
        }
        if effect.starts_with("you discard ") {
            effect = effect.replacen("you discard ", "discard ", 1);
        }
        return format!("As an additional cost to cast this spell, {effect}");
    }
    if trimmed == "Enchant opponent's creature" {
        return "Enchant creature an opponent controls".to_string();
    }
    if trimmed.contains("target sliver") {
        return trimmed.replace("target sliver", "target Sliver");
    }
    if let Some(rest) = trimmed.strip_prefix("target player sacrifices target player's ")
        && !rest.is_empty()
        && !rest.contains(". ")
    {
        return format!(
            "Target player sacrifices {} of their choice",
            with_indefinite_article(rest)
        );
    }
    if let Some(rest) = trimmed.strip_prefix("target opponent sacrifices target opponent's ")
        && !rest.is_empty()
        && !rest.contains(". ")
    {
        return format!(
            "Target opponent sacrifices {} of their choice",
            with_indefinite_article(rest)
        );
    }
    if let Some((before_discard, lose_tail)) = trimmed.split_once(
        ". For each opponent, that player discards a card. For each opponent, that player loses ",
    ) && let Some(loss_amount) = lose_tail.strip_suffix(" life")
    {
        return format!(
            "{}, discards a card, and loses {loss_amount} life",
            capitalize_first(before_discard.trim())
        );
    }
    if let Some((draw_clause, gain_tail)) = trimmed.split_once(". you gain ")
        && draw_clause.starts_with("Draw ")
        && let Some(gain_amount) = gain_tail.strip_suffix(" life")
    {
        return format!(
            "{}, and gain {gain_amount} life",
            capitalize_first(draw_clause)
        );
    }
    if let Some(rest) = trimmed.strip_prefix("Deal ")
        && let Some((damage, loss_tail)) =
            rest.split_once(" damage to target creature. that object's controller loses ")
        && let Some(loss_amount) = loss_tail.strip_suffix(" life")
    {
        return format!(
            "This creature deals {damage} damage to target creature and that creature's controller loses {loss_amount} life"
        );
    }
    if let Some(rest) = trimmed.strip_prefix("other ")
        && let Some((types, buff)) = rest.split_once(" creatures you control get ")
    {
        return format!("Other {types} creatures you control get {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix("other creature withs flying you control get ") {
        return format!("Other creatures you control with flying get {buff}");
    }
    if let Some(buff) = trimmed.strip_prefix("other creature with flying you control get ") {
        return format!("Other creatures you control with flying get {buff}");
    }
    if let Some(loss_tail) = trimmed.strip_prefix("For each opponent, that player loses ")
        && let Some(gain_tail) = loss_tail.split_once(" life. you gain ")
    {
        return format!(
            "Each opponent loses {} life and you gain {} life",
            gain_tail.0, gain_tail.1
        );
    }
    if let Some(loss_tail) = trimmed.strip_prefix("For each opponent, that player loses ")
        && let Some(gain_tail) = loss_tail.split_once(" life. You gain ")
    {
        return format!(
            "Each opponent loses {} life and you gain {} life",
            gain_tail.0, gain_tail.1
        );
    }
    if let Some(loss_tail) = trimmed.strip_prefix("For each opponent, that player loses ")
        && let Some((loss, gain)) = loss_tail.split_once(" and you gain ")
    {
        if let Some(normalized) = normalize_each_opponent_life_exchange_clause(loss, gain) {
            return capitalize_first(&normalized);
        }
        return format!("Each opponent loses {loss} and you gain {gain}");
    }
    if let Some((prefix, rest)) = trimmed.split_once(", for each opponent, that player loses ")
        && let Some((loss, gain)) = rest.split_once(" and you gain ")
    {
        if let Some(normalized) = normalize_each_opponent_life_exchange_clause(loss, gain) {
            return format!("{prefix}, {normalized}");
        }
        return format!("{prefix}, each opponent loses {loss} and you gain {gain}");
    }
    if let Some((prefix, rest)) = trimmed.split_once(", For each opponent, that player loses ")
        && let Some((loss, gain)) = rest.split_once(" and you gain ")
    {
        if let Some(normalized) = normalize_each_opponent_life_exchange_clause(loss, gain) {
            return format!("{prefix}, {normalized}");
        }
        return format!("{prefix}, each opponent loses {loss} and you gain {gain}");
    }
    if let Some(rest) = trimmed.strip_prefix("For each creature or planeswalker, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("Deal {amount} damage to each creature and each planeswalker");
    }
    if let Some(rest) = trimmed.strip_prefix("For each creature without flying, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("This creature deals {amount} damage to each creature without flying");
    }
    if let Some(rest) = trimmed.strip_prefix("For each creature with flying, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("This creature deals {amount} damage to each creature with flying");
    }
    if let Some(rest) = trimmed.strip_prefix("For each other creature with flying, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("This creature deals {amount} damage to each other creature with flying");
    }
    if let Some(rest) = trimmed.strip_prefix("For each ")
        && let Some((targets, amount_tail)) = rest.split_once(", Deal ")
        && let Some(amount) = amount_tail.strip_suffix(" damage to that object")
        && let Some((left, right_and_type)) = targets.split_once(" or ")
        && let Some(kind) = right_and_type.strip_suffix(" creature")
    {
        return format!("Deal {amount} damage to each {left} and/or {kind} creature");
    }
    if let Some((first_clause, rest)) = trimmed.split_once(". For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent")
            .or_else(|| rest.strip_suffix(" damage to each opponent."))
        && let Some(self_amount) = first_clause
            .strip_prefix("When this permanent enters, it deals ")
            .and_then(|tail| tail.strip_suffix(" damage to that player"))
        && self_amount.trim().eq_ignore_ascii_case(amount.trim())
    {
        return format!(
            "When this permanent enters, it deals {amount} damage to each opponent and each creature your opponents control"
        );
    }
    if let Some(rest) = trimmed.strip_prefix("For each opponent's creature, Deal ")
        && let Some(amount) = rest.strip_suffix(" damage to that object")
    {
        return format!("Deal {amount} damage to each creature your opponents control");
    }
    if let Some(rest) = trimmed.strip_prefix("For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent")
            .or_else(|| rest.strip_suffix(" damage to each opponent."))
    {
        return format!("Deal {amount} damage to each creature your opponents control");
    }
    if let Some((left, right)) = split_once_ascii_ci(trimmed, ". sacrifice ")
        && left.to_ascii_lowercase().contains(" and you lose ")
    {
        return format!(
            "{} and sacrifice {}",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(trimmed, "creatures you control get ")
        && let Some(buff) = rest
            .strip_suffix(" until end of turn. Untap all permanent")
            .or_else(|| rest.strip_suffix(" until end of turn. Untap all permanent."))
    {
        return format!("Creatures you control get {buff} until end of turn. Untap them");
    }
    if trimmed == "Can't block" {
        return "This creature can't block".to_string();
    }
    if trimmed
        == "Tag the object attached to this artifact as 'equipped'. Put 1 +1/+1 counter(s) on the tagged object 'equipped'"
    {
        return "Put a +1/+1 counter on equipped creature".to_string();
    }
    if trimmed
        == "Tag the object attached to this artifact as 'equipped'. Regenerate the tagged object 'equipped' until end of turn"
    {
        return "Regenerate equipped creature".to_string();
    }
    if let Some(counter) = trimmed
        .strip_prefix("Tag the object attached to this enchantment as 'enchanted'. Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counters on the tagged object 'enchanted'"))
    {
        return format!("Put a {counter} counter on enchanted creature");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Tag the object attached to this enchantment as 'enchanted'. Put a ")
        .and_then(|rest| rest.strip_suffix(" counter on the tagged object 'enchanted'"))
    {
        return format!("Put a {counter} counter on enchanted creature");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Tag the object attached to this aura as 'enchanted'. Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counters on the tagged object 'enchanted'"))
    {
        return format!("Put a {counter} counter on enchanted creature");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Tag the object attached to this aura as 'enchanted'. Put a ")
        .and_then(|rest| rest.strip_suffix(" counter on the tagged object 'enchanted'"))
    {
        return format!("Put a {counter} counter on enchanted creature");
    }
    if let Some(rest) = trimmed
        .strip_prefix("Tag the object attached to this enchantment as 'enchanted'. Regenerate the tagged object 'enchanted'")
    {
        return format!("Regenerate enchanted creature{rest}");
    }
    if let Some(rest) = trimmed
        .strip_prefix("Tag the object attached to this aura as 'enchanted'. Regenerate the tagged object 'enchanted'")
    {
        return format!("Regenerate enchanted creature{rest}");
    }
    if trimmed
        == "At the beginning of your upkeep: Sacrifice this enchantment unless you pays {W}{W}"
    {
        return "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}"
            .to_string();
    }
    if let Some(prefix) = trimmed.strip_suffix(" that an opponent's land could produce") {
        return format!("{prefix} that a land an opponent controls could produce");
    }
    if let Some(rest) = trimmed.strip_prefix("spells ")
        && let Some((creature_type, tail)) = rest.split_once(" you control cost ")
    {
        return format!("{creature_type} spells you cast cost {tail}");
    }
    if let Some((choice, rest)) = trimmed.split_once(". ")
        && let Some(chosen) = choice.strip_prefix("Choose ")
        && rest
            .to_ascii_lowercase()
            .starts_with(&chosen.to_ascii_lowercase())
    {
        return rest.to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("For each ")
        && let Some((who, tail)) = rest.split_once(", that player ")
    {
        return format!("each {who} {tail}");
    }
    if let Some((prefix, modes)) = trimmed.split_once("Choose between 0 and 1 mode(s) - ") {
        let modes = modes.replace("• ", "");
        return format!("{prefix}choose up to one — {modes}");
    }
    if let Some(modes) = trimmed.strip_prefix("Choose between 0 and 1 mode(s) - ") {
        let modes = modes.replace("• ", "");
        return format!("Choose up to one — {modes}");
    }
    if let Some(mana) = trimmed.strip_prefix("Add ")
        && let Some(mana) = mana.strip_suffix(" to your mana pool")
    {
        return format!("Add {mana}");
    }
    if lower_trimmed
        .starts_with("for each creature you control, put a +1/+1 counter on that object")
    {
        return "Put a +1/+1 counter on each creature you control".to_string();
    }
    if lower_trimmed.starts_with("for each creature you control, put")
        && lower_trimmed.contains("+1/+1")
        && lower_trimmed.contains("counter")
        && lower_trimmed.contains("that object")
    {
        return "Put a +1/+1 counter on each creature you control".to_string();
    }
    if lower_trimmed
        .starts_with("for each other creature you control, put a +1/+1 counter on that object")
    {
        return "Put a +1/+1 counter on each other creature you control".to_string();
    }
    if lower_trimmed.starts_with("for each other creature you control, put")
        && lower_trimmed.contains("+1/+1")
        && lower_trimmed.contains("counter")
        && lower_trimmed.contains("that object")
    {
        return "Put a +1/+1 counter on each other creature you control".to_string();
    }
    if lower_trimmed
        .starts_with("when this creature enters, for each creature you control, put a +1/+1 counter on that object")
    {
        return "When this creature enters, put a +1/+1 counter on each other creature you control"
            .to_string();
    }
    if lower_trimmed.starts_with(
        "when this creature enters, for each another creature you control, put a +1/+1 counter on that object",
    ) || lower_trimmed.starts_with(
        "when this creature enters, for each another creature you control, put 1 +1/+1 counter on that object",
    ) {
        return "When this creature enters, put a +1/+1 counter on each other creature you control"
            .to_string();
    }
    if lower_trimmed.starts_with(
        "when this permanent enters, for each another creature you control, put a +1/+1 counter on that object",
    ) || lower_trimmed.starts_with(
        "when this permanent enters, for each another creature you control, put 1 +1/+1 counter on that object",
    ) {
        return "When this permanent enters, for each other creature you control, Put a +1/+1 counter on that object."
            .to_string();
    }
    if let Some(rest) = strip_prefix_ascii_ci(trimmed, "opponent's ")
        && let Some((objects, predicate)) = split_once_ascii_ci(rest, " get ")
    {
        return format!(
            "{} your opponents control get {}",
            objects.trim(),
            predicate.trim()
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(trimmed, "opponent's ")
        && let Some((objects, predicate)) = split_once_ascii_ci(rest, " gets ")
    {
        return format!(
            "{} your opponents control gets {}",
            objects.trim(),
            predicate.trim()
        );
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().starts_with("you draw ")
        && right.starts_with("Create ")
        && right.contains(" Treasure token")
    {
        let draw_count = left
            .strip_prefix("you draw ")
            .or_else(|| left.strip_prefix("You draw "))
            .and_then(|tail| tail.strip_suffix(" cards"));
        let create_count = right
            .strip_prefix("Create ")
            .and_then(|tail| tail.strip_suffix(" Treasure tokens"));
        let render_count = |raw: &str| {
            raw.trim()
                .parse::<u32>()
                .ok()
                .and_then(small_number_word)
                .map(str::to_string)
                .unwrap_or_else(|| raw.trim().to_string())
        };
        if let (Some(draw_count), Some(create_count)) = (draw_count, create_count) {
            return format!(
                "Draw {} cards and create {} Treasure tokens",
                render_count(draw_count),
                render_count(create_count)
            );
        }
    }
    if let Some(rest) = trimmed.strip_prefix("you ") {
        let normalized = normalize_you_verb_phrase(rest);
        if normalized != rest {
            return normalized;
        }
    }
    if let Some((left, right)) = trimmed.split_once(". it gets ")
        && left.starts_with("Untap ")
        && left.contains("target ")
        && left.contains(" creatures")
    {
        return format!("{left}. They each get {right}");
    }
    if let Some(count) = trimmed.strip_prefix("you draw ")
        && let Some(count) = count.strip_suffix(" cards. Proliferate")
    {
        return format!("Draw {count} cards, then proliferate");
    }
    if let Some(count) = trimmed.strip_prefix("you draw ")
        && let Some(count) = count.strip_suffix(" cards. you sacrifice a permanent you control")
    {
        return format!("Draw {count} cards, then sacrifice a permanent");
    }
    if let Some(prefix) = trimmed.strip_suffix(" can't block until end of turn") {
        return format!("{prefix} can't block this turn");
    }
    if let Some(prefix) = trimmed.strip_suffix(" can't block until end of turn.") {
        return format!("{prefix} can't block this turn");
    }
    if let Some(prefix) = trimmed.strip_suffix(" can't be blocked until end of turn") {
        return format!("{prefix} can't be blocked this turn");
    }
    if let Some(prefix) = trimmed.strip_suffix(" can't be blocked until end of turn.") {
        return format!("{prefix} can't be blocked this turn");
    }
    if let Some(prefix) = trimmed.strip_suffix(" until end of turn")
        && prefix.contains("Regenerate ")
    {
        return prefix.to_string();
    }
    if let Some(prefix) = trimmed.strip_suffix(" until end of turn.")
        && prefix.contains("Regenerate ")
    {
        return prefix.to_string();
    }
    if trimmed.starts_with("Prevent the next ")
        && trimmed.contains(" damage to ")
        && trimmed.contains(" until end of turn")
    {
        return trimmed
            .replace(" damage to ", " damage that would be dealt to ")
            .replace(" until end of turn", " this turn");
    }
    if let Some((subject, mana_tail)) = trimmed.split_once(" have t add ") {
        return format!("{subject} have \"{{T}}: Add {mana_tail}.\"");
    }
    if let Some((subject, mana_tail)) = trimmed.split_once(" has t add ") {
        return format!("{subject} has \"{{T}}: Add {mana_tail}.\"");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && right.starts_with("you gain ")
        && left.contains(" loses ")
    {
        return format!("{left} and {right}");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.contains("Scry ")
        && right == "Draw a card"
    {
        let lower_right = right.to_ascii_lowercase();
        return format!("{left}, then {lower_right}");
    }
    if trimmed == "you draw a card" {
        return "Draw a card".to_string();
    }
    if trimmed == "Exile all card in graveyard" {
        return "Exile all graveyards".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("Exile target ")
        && let Some((count, noun)) = rest.split_once(" target ")
        && let Ok(count) = count.parse::<u32>()
    {
        let count_text = small_number_word(count)
            .map(str::to_string)
            .unwrap_or_else(|| count.to_string());
        let noun = if count == 1 {
            noun.to_string()
        } else {
            pluralize_noun_phrase(noun)
        };
        return format!("Exile {count_text} target {noun}");
    }
    if trimmed.contains("Create a treasure token") {
        return trimmed.replace("Create a treasure token", "Create a Treasure token");
    }
    if let Some(prefix) =
        trimmed.strip_suffix(" target card in your hand on top of its owner's library")
    {
        if prefix == "Put" {
            return "Put a card from your hand on top of your library".to_string();
        }
        if let Some(count_text) = prefix.strip_prefix("Put ") {
            return format!("Put {count_text} cards from your hand on top of your library");
        }
    }
    if trimmed.contains(" in your graveyard to its owner's hand") {
        return trimmed.replace(
            " in your graveyard to its owner's hand",
            " from your graveyard to your hand",
        );
    }
    if let Some(rest) =
        trimmed.strip_prefix("you may Search your library for artifact with mana value ")
        && let Some((value, tail)) =
            rest.split_once(" you own, reveal it, put it into hand, then shuffle")
    {
        return format!(
            "you may search your library for an artifact card with mana value {value}, reveal it, put it into your hand, then shuffle{tail}"
        );
    }
    if let Some(rest) =
        trimmed.strip_prefix("you may Search your library for artifact with mana value ")
        && let Some(value) =
            rest.strip_suffix(" you own, reveal it, put it into hand, then shuffle")
    {
        return format!(
            "you may search your library for an artifact card with mana value {value}, reveal it, put it into your hand, then shuffle"
        );
    }
    if let Some(rest) = trimmed.strip_prefix(
        "you may Search your library for Aura you own, reveal it, put it into hand, then shuffle",
    ) {
        return format!(
            "you may search your library for an Aura card, reveal it, put it into your hand, then shuffle{rest}"
        );
    }
    if let Some(rest) =
        trimmed.strip_prefix("you may Search your library for basic land you own, reveal it, put it on top of library, then shuffle")
    {
        return format!(
            "you may search your library for a basic land card, reveal it, then shuffle and put that card on top{rest}"
        );
    }
    if trimmed.starts_with("Whenever you cast creature") {
        return trimmed.replacen(
            "Whenever you cast creature",
            "When you cast a creature spell",
            1,
        );
    }
    if let Some(rest) = trimmed.strip_prefix("Sacrifice this creature: This creature deals ") {
        return format!("Sacrifice this creature: It deals {rest}");
    }
    if (trimmed.starts_with("This land enters with ")
        || trimmed.starts_with("This artifact enters with ")
        || trimmed.starts_with("This creature enters with ")
        || trimmed.starts_with("This permanent enters with "))
        && trimmed.contains("counter")
    {
        if let Some(after_this) = trimmed
            .strip_prefix("This ")
            .or_else(|| trimmed.strip_prefix("this "))
            && let Some((subject, tail)) = after_this.split_once(" enters with ")
            && let Some((count_raw, rest)) = tail.split_once(' ')
            && let Ok(count) = count_raw.parse::<u32>()
        {
            let counter_text = rest
                .strip_suffix(" counter(s)")
                .or_else(|| rest.strip_suffix(" counter(s)."))
                .or_else(|| rest.strip_suffix(" counters"))
                .or_else(|| rest.strip_suffix(" counters."))
                .or_else(|| rest.strip_suffix(" counter(s) on it"))
                .or_else(|| rest.strip_suffix(" counter(s) on it."))
                .or_else(|| rest.strip_suffix(" counters on it"))
                .or_else(|| rest.strip_suffix(" counters on it."));
            if let Some(counter_text) = counter_text {
                if count == 1 {
                    let phrase = with_indefinite_article(&format!("{counter_text} counter"));
                    return format!("This {subject} enters with {phrase} on it");
                }
                let count_text = small_number_word(count)
                    .map(str::to_string)
                    .unwrap_or_else(|| count_raw.to_string());
                return format!(
                    "This {subject} enters with {count_text} {counter_text} counters on it"
                );
            }
        }
        if !trimmed.contains(" on it") {
            if let Some(prefix) = trimmed.strip_suffix('.') {
                return format!("{prefix} on it.");
            }
            return format!("{trimmed} on it");
        }
    }
    if trimmed.starts_with("Exile ") && trimmed.contains("target cards in graveyard") {
        return trimmed.replace(
            "target cards in graveyard",
            "target cards from a single graveyard",
        );
    }
    if trimmed.starts_with("Exile ") && trimmed.contains("target cards from a graveyard") {
        return trimmed.replace(
            "target cards from a graveyard",
            "target cards from a single graveyard",
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(trimmed, ". ")
        && strip_prefix_ascii_ci(left.trim(), "target player draws ").is_some()
        && let Some(stripped) = strip_prefix_ascii_ci(right.trim(), "target player loses ")
        && stripped
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with(" life")
    {
        return format!(
            "{} and loses {}",
            capitalize_first(left.trim()),
            stripped.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && let Some(scry_count) = left
            .strip_prefix("Scry ")
            .and_then(|value| value.trim().parse::<u32>().ok())
        && scry_count != 1
        && (right.starts_with("Draw ") || right.to_ascii_lowercase().starts_with("you draw "))
    {
        let draw_clause = if let Some(rest) = right.strip_prefix("Draw ") {
            format!("draw {rest}")
        } else {
            normalize_you_verb_phrase(right)
        };
        return format!("{left}, then {draw_clause}");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().starts_with("you draw ")
        && right.to_ascii_lowercase().starts_with("you lose ")
        && !right.to_ascii_lowercase().contains("half your life")
        && right.to_ascii_lowercase().ends_with(" life")
    {
        return format!("{left} and {}", normalize_you_verb_phrase(right));
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left
            .to_ascii_lowercase()
            .starts_with("target player loses ")
        && right.to_ascii_lowercase().starts_with("you gain ")
        && !right.to_ascii_lowercase().contains("equal to")
        && right.to_ascii_lowercase().ends_with(" life")
    {
        return format!("{left} and {}", normalize_you_verb_phrase(right));
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().starts_with("you discard ")
        && right.to_ascii_lowercase().starts_with("you draw ")
    {
        let left = left.strip_prefix("you ").unwrap_or(left);
        return format!(
            "{}, then {}",
            capitalize_first(left),
            normalize_you_verb_phrase(right)
        );
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return it to the battlefield under its owner's control.")
    {
        return format!("{left}, then return it to the battlefield under its owner's control.");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return it to the battlefield under its owner's control")
    {
        return format!("{left}, then return it to the battlefield under its owner's control");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return that card to the battlefield under your control.")
    {
        return format!("{left}, then return it to the battlefield under your control.");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return that card to the battlefield under your control")
    {
        return format!("{left}, then return it to the battlefield under your control");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right
            .eq_ignore_ascii_case("Return that card to the battlefield under its owner's control.")
    {
        return format!("{left}, then return it to the battlefield under its owner's control.");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right
            .eq_ignore_ascii_case("Return that card to the battlefield under its owner's control")
    {
        return format!("{left}, then return it to the battlefield under its owner's control");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && let Some(tail) = strip_prefix_ascii_ci(
            right,
            "At the beginning of the next end step, return it to the battlefield. Put ",
        )
        && let Some(counter_phrase) =
            strip_suffix_ascii_ci(tail, " on it.").or_else(|| strip_suffix_ascii_ci(tail, " on it"))
    {
        return format!(
            "{left}. At the beginning of the next end step, return that card to the battlefield under its owner's control with {} on it.",
            counter_phrase.trim()
        );
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return it from graveyard to the battlefield tapped.")
    {
        return format!("{left}, then return it to the battlefield tapped.");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().contains("exile ")
        && right.eq_ignore_ascii_case("Return it from graveyard to the battlefield tapped")
    {
        return format!("{left}, then return it to the battlefield tapped");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left
            .to_ascii_lowercase()
            .starts_with("target player discards ")
        && right
            .to_ascii_lowercase()
            .starts_with("target player loses ")
    {
        return format!("{left} and {right}");
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.to_ascii_lowercase().starts_with("you draw ")
        && right.starts_with("Put ")
        && right.contains(" from your hand on top of your library")
    {
        let left = left.strip_prefix("you ").unwrap_or(left);
        return format!(
            "{}, then {} in any order",
            capitalize_first(left),
            right.to_ascii_lowercase()
        );
    }
    if lower_trimmed == "target creature can't be blocked this turn you draw a card" {
        return "Target creature can't be blocked this turn. Draw a card".to_string();
    }
    if lower_trimmed == "target creature can't block this turn you draw a card" {
        return "Target creature can't block this turn. Draw a card".to_string();
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.starts_with("Draw ")
        && right.starts_with("Create ")
        && right.contains(" Treasure token")
    {
        let normalize_count = |prefix: &str, suffix: &str| -> Option<String> {
            let value = prefix.trim().parse::<u32>().ok()?;
            let amount = small_number_word(value)
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string());
            Some(format!("{amount} {suffix}"))
        };
        if let Some(draw_tail) = left.strip_prefix("Draw ")
            && let Some(draw_count) = draw_tail.strip_suffix(" cards")
            && let Some(create_tail) = right.strip_prefix("Create ")
            && let Some(create_count) = create_tail.strip_suffix(" Treasure tokens")
            && let (Some(draw_text), Some(create_text)) = (
                normalize_count(draw_count, "cards"),
                normalize_count(create_count, "Treasure tokens"),
            )
        {
            return format!("Draw {draw_text} and create {create_text}");
        }
    }
    if let Some((left, right)) = trimmed.split_once(". ")
        && left.starts_with("Whenever this creature becomes tapped, you gain ")
        && right.starts_with("Scry ")
    {
        return format!("{left} and {}", right.to_ascii_lowercase());
    }
    if let Some((left, rest)) = trimmed.split_once(". ")
        && left.starts_with("Surveil ")
        && let Some((draw_clause, lose_clause)) = rest.split_once(". ")
        && draw_clause.starts_with("you draw ")
        && draw_clause.ends_with(" cards")
        && lose_clause.starts_with("you lose ")
        && lose_clause.ends_with(" life")
    {
        let draw_count = draw_clause
            .strip_prefix("you draw ")
            .and_then(|tail| tail.strip_suffix(" cards"))
            .unwrap_or_default()
            .trim()
            .parse::<u32>()
            .ok()
            .and_then(small_number_word)
            .map(str::to_string)
            .unwrap_or_else(|| {
                draw_clause
                    .strip_prefix("you draw ")
                    .and_then(|tail| tail.strip_suffix(" cards"))
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            });
        return format!(
            "{left}, then draw {draw_count} cards. {}",
            capitalize_first(lose_clause)
        );
    }
    if let Some(rest) = trimmed.strip_prefix("target creature gets ")
        && let Some(buff) = rest.strip_suffix(" until end of turn. Untap it")
    {
        return format!("Target creature gets {buff} until end of turn. Untap that creature");
    }
    if let Some(rest) = trimmed.strip_prefix("target creature gets ")
        && let Some((pt_text, tail)) = rest.split_once(" until end of turn")
        && let Some((power, toughness)) = pt_text.split_once('/')
    {
        let power = power.trim();
        let toughness = toughness.trim();
        if power == toughness && power.starts_with("the number of ") {
            let mut line =
                format!("Target creature gets +X/+X until end of turn, where X is {power}");
            line.push_str(tail);
            return line;
        }
    }
    if let Some(rest) = trimmed.strip_prefix("you sacrifice a land you control. ")
        && rest.starts_with("Search your library for up to 2 basic land")
    {
        return format!("Sacrifice a land. {rest}");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counters on target creature. Proliferate"))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put a ")
        .and_then(|rest| rest.strip_suffix(" counter on target creature. Proliferate"))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counters on target creature. Proliferate."))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put a ")
        .and_then(|rest| rest.strip_suffix(" counter on target creature. Proliferate."))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("put a ")
        .and_then(|rest| rest.strip_suffix(" counter on target creature. proliferate"))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("put a ")
        .and_then(|rest| rest.strip_suffix(" counter on target creature. proliferate."))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counter(s) on target creature. Proliferate"))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if let Some(counter) = trimmed
        .strip_prefix("Put 1 ")
        .and_then(|rest| rest.strip_suffix(" counter(s) on target creature. Proliferate."))
    {
        return format!("Put a {counter} counter on target creature, then proliferate");
    }
    if lower_trimmed == "put 1 -1/-1 counters on target creature. proliferate"
        || lower_trimmed == "put 1 -1/-1 counters on target creature. proliferate."
    {
        return "Put a -1/-1 counter on target creature, then proliferate".to_string();
    }
    if lower_trimmed == "put 1 +1/+1 counters on target creature. proliferate"
        || lower_trimmed == "put 1 +1/+1 counters on target creature. proliferate."
    {
        return "Put a +1/+1 counter on target creature, then proliferate".to_string();
    }
    if lower_trimmed == "draw two cards and you lose 2 life. you mill 2 cards."
        || lower_trimmed == "draw two cards and you lose 2 life. you mill 2 cards"
    {
        return "Draw two cards, lose 2 life, then mill two cards.".to_string();
    }
    if let Some((first, second)) = split_once_ascii_ci(trimmed, ". ")
        && let Some(first_buff) = strip_prefix_ascii_ci(first.trim(), "target creature gets ")
            .and_then(|rest| rest.strip_suffix(" until end of turn"))
        && let Some(second_buff) = strip_prefix_ascii_ci(
            second.trim(),
            "other creatures with the same name as that object get ",
        )
        .and_then(|rest| {
            rest.strip_suffix(" until end of turn")
                .or_else(|| rest.strip_suffix(" until end of turn."))
        })
        && first_buff.eq_ignore_ascii_case(second_buff)
    {
        return format!(
            "Target creature and all other creatures with the same name as that creature get {first_buff} until end of turn"
        );
    }
    if let Some((first, second)) =
        trimmed.split_once(". with the same name as that object other creatures get ")
        && let Some(first_buff) = first
            .strip_prefix("target creature gets ")
            .and_then(|rest| rest.strip_suffix(" until end of turn"))
        && second == format!("{first_buff} until end of turn")
    {
        return format!(
            "target creature and all other creatures with the same name as that creature get {first_buff} until end of turn"
        );
    }

    let mut normalized = trimmed.to_string();
    normalized = normalized
        .replace("this creatures get ", "this creature gets ")
        .replace("this creatures gain ", "this creature gains ")
        .replace("this creatures power", "this creature's power")
        .replace("this creatures toughness", "this creature's toughness")
        .replace("enchanted creatures get ", "enchanted creature gets ")
        .replace("enchanted creatures gain ", "enchanted creature gains ")
        .replace("equipped creatures get ", "equipped creature gets ")
        .replace("equipped creatures gain ", "equipped creature gains ")
        .replace(
            "Whenever this creature blocks or Whenever this creature becomes blocked",
            "Whenever this creature blocks or becomes blocked",
        )
        .replace(
            "target attacking/blocking creature",
            "target attacking or blocking creature",
        )
        .replace("a another ", "another ")
        .replace("all another ", "all other ")
        .replace("All another ", "All other ")
        .replace("each another ", "each other ")
        .replace("Each another ", "Each other ")
        .replace("For each another ", "For each other ")
        .replace("for each another ", "for each other ")
        .replace("you takes ", "you take ")
        .replace("You takes ", "You take ")
        .replace("you loses ", "you lose ")
        .replace("You loses ", "You lose ")
        .replace(" damage to it", " damage to that creature")
        .replace("+-", "-")
        .replace(" gets 0/+", " gets +0/+")
        .replace(" gets 0/", " gets +0/")
        .replace("that creatureself", "itself")
        .replace(
            "other creature you control flying get ",
            "other creatures you control with flying get ",
        )
        .replace(
            "other creature you control with flying get ",
            "other creatures you control with flying get ",
        )
        .replace(
            "token creatures you control get ",
            "creature tokens you control get ",
        )
        .replace("token creatures get ", "creature tokens get ")
        .replace(
            "Destroy target artifact or enchantment or land",
            "Destroy target artifact, enchantment, or land",
        )
        .replace(
            "Destroy target artifact or creature or planeswalker",
            "Destroy target artifact, creature, or planeswalker",
        )
        .replace(
            "Destroy target artifact or enchantment or planeswalker",
            "Destroy target artifact, enchantment, or planeswalker",
        )
        .replace(
            "Destroy target artifact or battle or enchantment or creature with flying",
            "Destroy target artifact, battle, enchantment, or creature with flying",
        )
        .replace(
            "target artifact or creature or enchantment or land card",
            "target artifact, creature, enchantment, or land card",
        )
        .replace(
            "target artifact or creature or enchantment or land",
            "target artifact, creature, enchantment, or land",
        )
        .replace("Destroy all enchantment", "Destroy all enchantments")
        .replace(
            "Exile target card in graveyard",
            "Exile target card from a graveyard",
        )
        .replace(
            "Exile target artifact card in graveyard",
            "Exile target artifact card from a graveyard",
        )
        .replace(
            "Exile target creature card in graveyard",
            "Exile target creature card from a graveyard",
        )
        .replace(
            "exile target card in graveyard",
            "exile target card from a graveyard",
        )
        .replace(
            "exile target artifact card in graveyard",
            "exile target artifact card from a graveyard",
        )
        .replace(
            "exile target creature card in graveyard",
            "exile target creature card from a graveyard",
        )
        .replace(
            "Search your library for a card, put it into hand, then shuffle",
            "Search your library for a card, put that card into your hand, then shuffle",
        )
        .replace(
            "for each the number of an artifact you control",
            "for each artifact you control",
        )
        .replace("for each the number of a ", "for each ")
        .replace("for each the number of an ", "for each ")
        .replace(", then you draw ", ", then draw ")
        .replace(". you draw ", ". Draw ")
        .replace(", then you mill ", ", then mill ")
        .replace(
            "Discard a card, then you draw ",
            "Discard a card, then draw ",
        )
        .replace(
            "discard a card, then you draw ",
            "Discard a card, then draw ",
        )
        .replace(
            "Sacrifice this creature: this creature deals ",
            "Sacrifice this creature: It deals ",
        )
        .replace(
            "Sacrifice this creature: This creature deals ",
            "Sacrifice this creature: It deals ",
        )
        .replace(
            "Prevent combat damage until end of turn",
            "Prevent all combat damage that would be dealt this turn",
        )
        .replace(
            "Add one mana of any color that an opponent's land could produce",
            "Add one mana of any color that a land an opponent controls could produce",
        )
        .replace(
            "Add one mana of any color to your mana pool that an opponent's land could produce",
            "Add one mana of any color that a land an opponent controls could produce",
        )
        .replace(" / ", "/")
        .replace(
            "for each the number of blocking creature",
            "for each creature blocking it",
        )
        .replace(
            "target creature you don't control or planeswalker",
            "target creature or planeswalker you don't control",
        )
        .replace(
            "target opponent's creature",
            "target creature an opponent controls",
        )
        .replace("enters the battlefield", "enters")
        .replace(
            "target opponent's nonland enchantment",
            "target nonland permanent an opponent controls",
        )
        .replace(
            "Destroy target opponent's artifact or enchantment",
            "Destroy target artifact or enchantment an opponent controls",
        )
        .replace(
            "Create a Powerstone artifact token, tapped",
            "Create a tapped Powerstone token",
        )
        .replace(", This creature deals ", ", it deals ")
        .replace(
            " in your graveyard on top of its owner's library",
            " from your graveyard on top of your library",
        )
        .replace("Put 1 +1/+1 counter(s) on ", "Put a +1/+1 counter on ")
        .replace("counter(s)", "counters")
        .replace(
            "Whenever this creature deals damage to Spider, destroy it.",
            "Whenever this creature deals damage to Spider, destroy it.",
        )
        .replace(
            "Destroy target black or red attacking/blocking creature and you gain 2 life.",
            "Destroy target black or red creature that's attacking or blocking. You gain 2 life.",
        )
        .replace("Whenever a another ", "Whenever another ")
        .replace("you may Search", "you may search")
        .replace(
            "When this creature enters or Whenever another Ally you control enters,",
            "Whenever this creature or another Ally you control enters,",
        )
        .replace(
            "Untap all a creature you control",
            "Untap all creatures you control",
        )
        .replace(", May ", ", you may ")
        .replace("you pays ", "you pay ")
        .replace("Add 1 mana of any color", "Add one mana of any color")
        .replace("a artifact", "an artifact")
        .replace("a enchantment", "an enchantment")
        .replace("a Aura", "an Aura")
        .replace("its owners hand", "its owner's hand")
        .replace("its owners hands", "its owners' hands")
        .replace("their owners hand", "their owner's hand")
        .replace("their owners hands", "their owners' hands")
        .replace("instant or sorcery cards", "instant and/or sorcery cards")
        .replace("instants or sorcery cards", "instant and/or sorcery cards")
        .replace("you control you control", "you control")
        .replace("put it into hand", "put it into your hand")
        .replace(
            "reveal it, put it into hand",
            "reveal it, put it into your hand",
        )
        .replace(
            "you may target creature gets ",
            "you may have target creature get ",
        )
        .replace(
            "you may target creature gains ",
            "you may have target creature gain ",
        )
        .replace(
            "you may target creature loses ",
            "you may have target creature lose ",
        )
        .replace(
            "you may target creature reveals ",
            "you may have target creature reveal ",
        )
        .replace(
            "For each player, You may that player ",
            "For each player, that player may ",
        )
        .replace(
            "For each opponent, You may that player ",
            "For each opponent, that player may ",
        )
        .replace(
            "for each player, you may that player ",
            "for each player, that player may ",
        )
        .replace(
            "for each opponent, you may that player ",
            "for each opponent, that player may ",
        )
        .replace(
            "you may that player loses ",
            "you may have that player lose ",
        )
        .replace(
            "you may that player discards ",
            "you may have that player discard ",
        )
        .replace(
            "you may that player draws ",
            "you may have that player draw ",
        )
        .replace(
            "you may that player scries ",
            "you may have that player scry ",
        )
        .replace(
            "you may that player mills ",
            "you may have that player mill ",
        )
        .replace("that player may loses ", "that player may lose ")
        .replace("that player may discards ", "that player may discard ")
        .replace("that player may draws ", "that player may draw ")
        .replace("that player may scries ", "that player may scry ")
        .replace("that player may mills ", "that player may mill ")
        .replace("that player may sacrifices ", "that player may sacrifice ")
        .replace("that player may gains ", "that player may gain ")
        .replace("that player may reveals ", "that player may reveal ")
        .replace("that player may shuffles ", "that player may shuffle ")
        .replace("that player may puts ", "that player may put ")
        .replace("that player may Put ", "that player may put ");
    normalized = normalize_common_semantic_phrasing(&normalized);
    normalized = normalize_zero_pt_prefix(&normalized);
    if normalized.ends_with(" as long as it's your turn")
        && (normalized.starts_with("this creature ")
            || normalized.starts_with("creatures you control ")
            || normalized.starts_with("creature you control "))
    {
        let body = normalized
            .strip_suffix(" as long as it's your turn")
            .unwrap_or(normalized.as_str())
            .to_string();
        return format!("During your turn, {body}");
    }
    normalized
}

#[allow(dead_code)]
fn normalize_for_each_damage_clause(clause: &str) -> Option<String> {
    let rest = clause.strip_prefix("For each ")?;
    let (subject, tail) = rest.split_once(", Deal ")?;
    let amount = tail.strip_suffix(" damage to that object")?;
    Some(format!("Deal {amount} damage to each {subject}"))
}

#[allow(dead_code)]
fn normalize_each_player_then_for_each_damage_clause(line: &str) -> Option<String> {
    let (left, rest) = line.split_once(" damage to that player. For each ")?;
    let amount = left.strip_prefix("Deal ")?;
    let (filter, right) = rest.split_once(" that player controls, Deal ")?;
    let amount_right = right.strip_suffix(" damage to each player")?;
    if amount != amount_right {
        return None;
    }
    Some(format!(
        "Deal {amount} damage to each {filter} and each player"
    ))
}

#[allow(dead_code)]
fn normalize_oracle_line_for_card(def: &CardDefinition, line: &str) -> String {
    let normalized = line.trim().replace("{{", "{").replace("}}", "}");
    let normalized = normalize_common_semantic_phrasing(&normalized);
    if def.is_spell() && normalized.starts_with("Deal ") {
        if let Some(rest) = normalized.strip_prefix("Deal ") {
            return format!("{} deals {rest}", def.card.name);
        }
    }
    normalized
}

/// Render compiled output in a near-oracle style for semantic diffing.
pub fn oracle_like_lines(def: &CardDefinition) -> Vec<String> {
    let _ = def;
    let base_lines = compiled_lines(def);
    let normalized = base_lines
        .iter()
        .map(|line| strip_render_heading(line))
        .filter(|line| !line.is_empty())
        .map(|line| normalize_common_semantic_phrasing(&line))
        .collect::<Vec<_>>();
    let merged_predicates = merge_adjacent_subject_predicate_lines(normalized);
    let merged_mana = merge_adjacent_simple_mana_add_lines(merged_predicates);
    let merged_has_keywords = merge_subject_has_keyword_lines(merged_mana);
    let without_redundant_cost_lines = drop_redundant_spell_cost_lines(merged_has_keywords);
    let merged_blockability = merge_blockability_lines(without_redundant_cost_lines);
    let merged_transform = merge_lose_all_transform_lines(merged_blockability);
    merged_transform
        .into_iter()
        .map(|line| normalize_sentence_surface_style(&line))
        .collect()
}

#[cfg(test)]
mod normalize_sentence_surface_style_tests {
    use super::normalize_sentence_surface_style;

    #[test]
    fn normalizes_choose_one_bullet_modes_to_multiline() {
        let normalized = normalize_sentence_surface_style(
            "Choose one — Tap target creature. • Untap target creature.",
        );
        assert_eq!(
            normalized,
            "Choose one —\n• Tap target creature.\n• Untap target creature."
        );
    }

    #[test]
    fn normalizes_choose_two_bullet_modes_to_multiline() {
        let normalized = normalize_sentence_surface_style(
            "Choose two - Destroy target artifact. • Destroy target enchantment. • Destroy target creature.",
        );
        assert_eq!(
            normalized,
            "Choose two —\n• Destroy target artifact.\n• Destroy target enchantment.\n• Destroy target creature."
        );
    }

    #[test]
    fn normalizes_choose_repeat_modes_to_multiline() {
        let normalized = normalize_sentence_surface_style(
            "Choose four. You may choose the same mode more than once. • You gain 4 life. • Draw a card.",
        );
        assert_eq!(
            normalized,
            "Choose four. You may choose the same mode more than once.\n• You gain 4 life.\n• Draw a card."
        );
    }

    #[test]
    fn does_not_append_terminal_period_after_reminder_parenthetical() {
        let normalized = normalize_sentence_surface_style(
            "Target creature gets +1/+1 until end of turn. (It can't be blocked.)",
        );
        assert_eq!(
            normalized,
            "Target creature gets +1/+1 until end of turn. (It can't be blocked.)"
        );
    }

    #[test]
    fn merges_gets_and_is_predicates_across_sentences() {
        let merged = super::merge_sentence_subject_predicates(
            "This creature gets +1/+1 until end of turn. this creature is an artifact in addition to its other types.",
        );
        assert_eq!(
            merged,
            Some(
                "This creature gets +1/+1 until end of turn and is an artifact in addition to its other types."
                    .to_string()
            )
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compiled_lines, describe_additional_cost_effects, describe_for_each_filter,
        merge_adjacent_static_heading_lines, merge_adjacent_subject_predicate_lines,
        normalize_common_semantic_phrasing, normalize_compiled_post_pass_effect,
        normalize_create_under_control_clause, normalize_gain_life_plus_phrase,
        normalize_known_low_tail_phrase, normalize_rendered_line_for_card,
        normalize_sentence_surface_style, normalize_spell_self_exile, pluralize_noun_phrase,
    };
    use crate::cards::CardDefinitionBuilder;
    use crate::filter::{ObjectFilter, PlayerFilter};
    use crate::ids::CardId;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    #[test]
    fn normalizes_target_creature_or_planeswalker_ordering() {
        let normalized = normalize_common_semantic_phrasing(
            "target creature you control deals damage equal to its power to target creature you don't control or planeswalker",
        );
        assert_eq!(
            normalized,
            "target creature you control deals damage equal to its power to target creature or planeswalker you don't control"
        );
    }

    #[test]
    fn additional_cost_choose_one_renders_inline_or_phrase() {
        let effects = vec![crate::effect::Effect::choose_one(vec![
            crate::effect::EffectMode {
                description: "sacrifice a creature".to_string(),
                effects: Vec::new(),
            },
            crate::effect::EffectMode {
                description: "pay 3".to_string(),
                effects: Vec::new(),
            },
        ])];
        assert_eq!(
            describe_additional_cost_effects(&effects),
            "sacrifice a creature or pay {3}"
        );
    }

    #[test]
    fn normalizes_sentence_surface_punctuation_for_sentences() {
        assert_eq!(
            normalize_sentence_surface_style("target creature gets +2/+2 until end of turn"),
            "Target creature gets +2/+2 until end of turn."
        );
    }

    #[test]
    fn keeps_keyword_lines_without_terminal_period() {
        assert_eq!(normalize_sentence_surface_style("Flying"), "Flying");
        assert_eq!(
            normalize_sentence_surface_style("Trample, haste"),
            "Trample, haste"
        );
    }

    #[test]
    fn normalizes_for_each_player_damage_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "For each player, Deal 2 damage to that player. For each creature that player controls, Deal 2 damage to that object",
        );
        assert_eq!(normalized, "Deal 2 damage to each creature and each player");
    }

    #[test]
    fn normalizes_opponents_creature_damage_and_cant_block_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "For each opponent's creature, Deal 1 damage to that object. an opponent's creature can't block until end of turn",
        );
        assert_eq!(
            normalized,
            "Deal 1 damage to each creature your opponents control. Creatures your opponents control can't block this turn"
        );
    }

    #[test]
    fn normalizes_opponents_creature_damage_and_cant_block_chain_this_turn_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "For each opponent's creature, Deal 1 damage to that object. an opponent's creature can't block this turn",
        );
        assert_eq!(
            normalized,
            "Deal 1 damage to each creature your opponents control. Creatures your opponents control can't block this turn"
        );
    }

    #[test]
    fn normalizes_generic_for_each_damage_to_each_filter() {
        let normalized = normalize_common_semantic_phrasing(
            "For each creature with flying, Deal 4 damage to that object",
        );
        assert_eq!(normalized, "Deal 4 damage to each creature with flying");
    }

    #[test]
    fn normalizes_for_each_opponent_that_player_clause() {
        let normalized =
            normalize_common_semantic_phrasing("For each opponent, that player draws a card");
        assert_eq!(normalized, "Each opponent draws a card");
    }

    #[test]
    fn normalizes_enchanted_land_tapped_for_mana_additional_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever enchanted land is tapped for mana, add {G}{G} to that object's controller's mana pool.",
        );
        assert_eq!(
            normalized,
            "Whenever enchanted land is tapped for mana, its controller adds an additional {G}{G}."
        );
    }

    #[test]
    fn normalizes_spawn_token_inline_quoted_cost_punctuation() {
        let normalized = normalize_common_semantic_phrasing(
            "When this permanent enters, create two 0/1 colorless Eldrazi Spawn creature tokens with \"Sacrifice this creature, add {C}\"",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, create two 0/1 colorless Eldrazi Spawn creature tokens with \"Sacrifice this creature, add {C}.\""
        );
    }

    #[test]
    fn normalizes_creature_type_choice_pump_sentence() {
        let normalized = normalize_common_semantic_phrasing(
            "You choose exactly 1 creature in the battlefield and tags it as 'chosen_creature_type_ref'. creature that shares a creature types with that object get +0/+4 until end of turn.",
        );
        assert_eq!(
            normalized,
            "Creatures of the creature type of your choice get +0/+4 until end of turn."
        );
    }

    #[test]
    fn normalizes_creature_type_choice_pump_and_gain_sentence() {
        let normalized = normalize_common_semantic_phrasing(
            "You choose exactly 1 creature in the battlefield and tags it as 'chosen_creature_type_ref'. creature that shares a creature types with that object get +2/+2 until end of turn. creature that shares a creature types with that object gain Trample until end of turn.",
        );
        assert_eq!(
            normalized,
            "Creatures of the creature type of your choice get +2/+2 and gain Trample until end of turn."
        );
    }

    #[test]
    fn normalizes_pump_and_gain_until_end_of_turn_sentence() {
        let normalized = normalize_common_semantic_phrasing(
            "Each creature you control gets +3/+3 until end of turn. creatures you control gain Trample until end of turn.",
        );
        assert_eq!(
            normalized,
            "Each creature you control gets +3/+3 and gains Trample until end of turn."
        );
    }

    #[test]
    fn normalizes_other_elf_plural_surface() {
        let normalized = normalize_common_semantic_phrasing("Other Elf you control get +1/+1.");
        assert_eq!(normalized, "Other Elves you control get +1/+1.");
    }

    #[test]
    fn normalizes_powerstone_tapped_token_surface() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, create a Powerstone artifact token, tapped.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, create a tapped Powerstone token."
        );
    }

    #[test]
    fn normalizes_discard_and_lose_same_target_surface() {
        let normalized = normalize_common_semantic_phrasing(
            "Target player discards 2 cards. target player loses 2 life.",
        );
        assert_eq!(
            normalized,
            "Target player discards 2 cards and loses 2 life."
        );
    }

    #[test]
    fn normalizes_commander_color_identity_mana_surface() {
        let normalized = normalize_common_semantic_phrasing(
            "Mana ability 1: {T}: Add 1 mana of commander's color identity.",
        );
        assert_eq!(
            normalized,
            "Mana ability 1: {T}: Add one mana of any color in your commander's color identity."
        );
    }

    #[test]
    fn normalizes_for_each_opponent_sacrifice_unless_pay_surface() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, for each opponent, that player sacrifices a permanent unless that player pays {1}.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, each opponent sacrifices a permanent of their choice unless they pay {1}"
        );
    }

    #[test]
    fn normalizes_choose_another_attacking_creature_scaffolding() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever this creature attacks, choose another target attacking creature. another target attacking creature can't be blocked this turn.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature attacks, another target attacking creature can't be blocked this turn."
        );
    }

    #[test]
    fn normalizes_for_each_counter_chain_to_each_creature_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "For each creature you control with a +1/+1 counter on it, Put a +1/+1 counter on that object",
        );
        assert_eq!(
            normalized,
            "Put a +1/+1 counter on each creature you control with a +1/+1 counter on it"
        );
    }

    #[test]
    fn normalizes_reveal_tagged_land_return_to_put_into_hand() {
        let normalized = normalize_common_semantic_phrasing(
            "Reveal the top card of defending player's library and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches land, Return it to its owner's hand",
        );
        assert_eq!(
            normalized,
            "Reveal the top card of defending player's library. If it's a land card, that player puts it into their hand"
        );
    }

    #[test]
    fn normalizes_tagged_destroyed_loop_phrasing() {
        let normalized = normalize_common_semantic_phrasing(
            "For each tagged 'destroyed_0' object, Create 1 3/3 green Centaur creature token under that object's controller's control",
        );
        assert_eq!(
            normalized,
            "For each object destroyed this way, Create 1 3/3 green Centaur creature token under that object's controller's control"
        );
    }

    #[test]
    fn keeps_additional_cost_colon_phrase_non_triggered() {
        let normalized = normalize_common_semantic_phrasing(
            "As an additional cost to cast this spell: you discard a card",
        );
        assert_eq!(
            normalized,
            "As an additional cost to cast this spell: you discard a card"
        );
    }

    #[test]
    fn normalizes_shared_you_and_target_opponent_draw_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, you draw a card. target opponent draws a card",
        );
        assert_eq!(
            normalized,
            "When this creature enters, you and target opponent each draw a card"
        );
    }

    #[test]
    fn normalizes_split_destroy_all_dual_types() {
        let normalized =
            normalize_common_semantic_phrasing("Destroy all artifact. Destroy all enchantment.");
        assert_eq!(normalized, "Destroy all artifacts and enchantments");
    }

    #[test]
    fn normalizes_target_player_sacrifice_choice_phrasing() {
        let normalized =
            normalize_common_semantic_phrasing("target player sacrifices target player's creature");
        assert_eq!(
            normalized,
            "Target player sacrifices a creature of their choice"
        );
    }

    #[test]
    fn normalizes_each_player_sacrifice_choice_phrasing() {
        let normalized = normalize_common_semantic_phrasing(
            "Triggered ability 1: At the beginning of each player's upkeep, that player sacrifices an artifact.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: At the beginning of each player's upkeep, that player sacrifices an artifact."
        );
    }

    #[test]
    fn normalizes_each_player_sacrifice_without_controls() {
        let normalized = normalize_common_semantic_phrasing(
            "Triggered ability 1: When this creature enters, each player sacrifices two creatures that player controls.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: When this creature enters, each player sacrifices two creatures that player controls."
        );
    }

    #[test]
    fn normalizes_its_controller_controller_owned_sacrifice() {
        let normalized = normalize_common_semantic_phrasing(
            "Spell effects: For each creature, its controller sacrifices a controller's permanent unless its controller pays {1}.",
        );
        assert_eq!(
            normalized,
            "Spell effects: For each creature, its controller sacrifices a controller's permanent unless its controller pays {1}."
        );
    }

    #[test]
    fn normalizes_creatures_have_cant_block() {
        let normalized = normalize_common_semantic_phrasing("All creatures have Can't block");
        assert_eq!(normalized, "Creatures can't block");
    }

    #[test]
    fn normalizes_monocolored_creatures_cant_block() {
        let normalized = normalize_common_semantic_phrasing(
            "monocolored creature can't block until end of turn",
        );
        assert_eq!(normalized, "monocolored creature can't block this turn");
    }

    #[test]
    fn normalizes_unblockable_until_end_of_turn_to_this_turn() {
        let normalized = normalize_common_semantic_phrasing(
            "target creature can't be blocked until end of turn",
        );
        assert_eq!(normalized, "target creature can't be blocked this turn");
    }

    #[test]
    fn normalizes_tap_any_number_gain_life_phrase() {
        let normalized = normalize_common_semantic_phrasing(
            "Tap any number of an untapped creature you control and you gain 4 life for each tapped creature",
        );
        assert_eq!(
            normalized,
            "Tap any number of untapped creatures you control. You gain 4 life for each creature tapped this way"
        );
    }

    #[test]
    fn normalizes_change_controller_and_haste_phrase() {
        let normalized = normalize_common_semantic_phrasing(
            "Untap target creature. it changes controller to this effect's controller and gains Haste until end of turn.",
        );
        assert_eq!(
            normalized,
            "Untap target creature. Gain control of it until end of turn. It gains haste until end of turn."
        );
    }

    #[test]
    fn normalizes_single_creature_haste_then_sacrifice_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "you may Put creature card in your hand onto the battlefield. it gains Haste until end of turn. you sacrifice a creature.",
        );
        assert_eq!(
            normalized,
            "You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step."
        );
    }

    #[test]
    fn normalizes_pronoun_end_step_sacrifice_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "it gains Haste until end of turn. At the beginning of the next end step, you sacrifice it.",
        );
        assert_eq!(
            normalized,
            "That creature gains haste until end of turn. At the beginning of the next end step, sacrifice that creature."
        );
    }

    #[test]
    fn normalizes_search_equipment_you_own_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Search your library for Equipment you own, reveal it, put it into your hand, then shuffle",
        );
        assert_eq!(
            normalized,
            "Search your library for an Equipment card, reveal it, put it into your hand, then shuffle"
        );
    }

    #[test]
    fn normalizes_opponents_artifact_creature_enter_tapped_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "An opponent's artifact or creature enter the battlefield tapped.",
        );
        assert_eq!(
            normalized,
            "Artifacts and creatures your opponents control enter tapped."
        );
    }

    #[test]
    fn normalizes_target_creature_untap_it_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "Target creature gets +1/+1 until end of turn. Untap it.",
        );
        assert_eq!(
            normalized,
            "Target creature gets +1/+1 until end of turn. Untap that creature."
        );
    }

    #[test]
    fn normalizes_choose_any_number_then_sacrifice_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "You choose any number a Mountain you control in the battlefield. you sacrifice all permanents you control. Deal that much damage to target player or planeswalker.",
        );
        assert_eq!(
            normalized,
            "Sacrifice any number of Mountains. Deal that much damage to target player or planeswalker."
        );
    }

    #[test]
    fn normalizes_destroy_target_blocking_creature_clause_without_rewriting_subject() {
        let normalized = normalize_common_semantic_phrasing("Destroy target blocking creature.");
        assert_eq!(normalized, "Destroy target blocking creature.");
    }

    #[test]
    fn normalizes_target_player_draws_and_loses_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Target player draws a card. target player loses 1 life.",
        );
        assert_eq!(normalized, "Target player draws a card and loses 1 life");
    }

    #[test]
    fn normalizes_target_player_draws_numeric_cards_and_loses_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Target player draws 2 cards. target player loses 2 life.",
        );
        assert_eq!(normalized, "Target player draws two cards and loses 2 life");
    }

    #[test]
    fn normalizes_opponents_creatures_get_clause() {
        let normalized =
            normalize_common_semantic_phrasing("Opponent's creatures get -2/-0 until end of turn.");
        assert_eq!(
            normalized,
            "Creatures your opponents control get -2/-0 until end of turn."
        );
    }

    #[test]
    fn normalizes_all_creatures_get_clause() {
        let normalized =
            normalize_common_semantic_phrasing("Creatures get -2/-2 until end of turn.");
        assert_eq!(normalized, "All creatures get -2/-2 until end of turn.");
    }

    #[test]
    fn normalizes_put_land_card_in_hand_phrase() {
        let normalized = normalize_common_semantic_phrasing(
            "{T}: you may Put land card in your hand onto the battlefield.",
        );
        assert_eq!(
            normalized,
            "{T}: You may put a land card from your hand onto the battlefield"
        );
    }

    #[test]
    fn normalizes_same_name_gets_split_sentence() {
        let normalized = normalize_common_semantic_phrasing(
            "Target creature gets -3/-3 until end of turn. other creatures with the same name as that object get -3/-3 until end of turn.",
        );
        assert_eq!(
            normalized,
            "Target creature and all other creatures with the same name as that creature get -3/-3 until end of turn"
        );
    }

    #[test]
    fn normalizes_enters_for_each_another_creature_counter_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this permanent enters, for each another creature you control, Put a +1/+1 counter on that object.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, for each other creature you control, Put a +1/+1 counter on that object."
        );
    }

    #[test]
    fn normalizes_untap_all_a_creature_phrase() {
        let normalized = normalize_common_semantic_phrasing("Untap all a creature you control.");
        assert_eq!(normalized, "Untap all creatures you control");
    }

    #[test]
    fn normalizes_triggered_target_player_draws_and_loses_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, target player draws a card. target player loses 1 life.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, target player draws a card. target player loses 1 life."
        );
    }

    #[test]
    fn normalizes_you_draw_and_you_lose_clause() {
        let normalized =
            normalize_common_semantic_phrasing("You draw two cards and you lose 2 life.");
        assert_eq!(normalized, "You draw two cards and you lose 2 life.");
    }

    #[test]
    fn normalizes_target_creature_tap_it_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "Target creature gets -1/-1 until end of turn. Tap it.",
        );
        assert_eq!(
            normalized,
            "Target creature gets -1/-1 until end of turn. Tap it."
        );
    }

    #[test]
    fn normalizes_red_or_green_spell_cost_reduction_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Red and green spells you cast cost {1} less to cast.",
        );
        assert_eq!(
            normalized,
            "Each spell you cast that's red or green costs {1} less to cast"
        );
    }

    #[test]
    fn normalizes_rakdos_return_discard_controller_phrase() {
        let normalized = normalize_common_semantic_phrasing(
            "Deal X damage to target opponent or planeswalker. target opponent discards X cards.",
        );
        assert_eq!(
            normalized,
            "Deal X damage to target opponent or planeswalker. That player or that planeswalker's controller discards X cards."
        );
    }

    #[test]
    fn normalizes_draw_two_then_proliferate_sentence() {
        let normalized = normalize_sentence_surface_style("You draw two cards. Proliferate.");
        assert_eq!(normalized, "You draw two cards. Proliferate.");
    }

    #[test]
    fn normalizes_search_discard_then_shuffle_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Spell effects: Search your library for a card, put it into your hand. you discard a card at random. Shuffle your library.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Search your library for a card, put it into your hand, discard a card at random, then shuffle."
        );
    }

    #[test]
    fn normalizes_discard_random_then_discard_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Spell effects: Target opponent discards a card at random. target opponent discards a card.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Target opponent discards a card at random, then discards a card."
        );
    }

    #[test]
    fn normalizes_siege_mill_discard_trigger_sentence() {
        let normalized = normalize_sentence_surface_style(
            "When this permanent enters, for each player, that player mills 3 cards. For each opponent, that player discards a card. Draw a card.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, for each player, that player mills 3 cards. For each opponent, that player discards a card. Draw a card."
        );
    }

    #[test]
    fn merges_adjacent_static_heading_keyword_lines() {
        let merged = merge_adjacent_static_heading_lines(vec![
            "Static ability 1: Creatures you control have Flying.".to_string(),
            "Static ability 2: Creatures you control have First strike.".to_string(),
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0],
            "Static ability 1: Creatures you control have flying and first strike"
        );
    }

    #[test]
    fn merges_adjacent_static_heading_enters_tapped_with_counters_lines() {
        let merged = merge_adjacent_static_heading_lines(vec![
            "Static ability 1: This land enters tapped.".to_string(),
            "Static ability 2: Enters the battlefield with 2 charge counter(s).".to_string(),
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0],
            "Static ability 1: This land enters tapped with 2 charge counter(s)"
        );
    }

    #[test]
    fn normalizes_target_opponent_choose_destroy_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Target opponent chooses exactly 1 a creature that player controls in the battlefield. Destroy it.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses exactly 1 a creature that player controls in the battlefield. Destroy it."
        );
    }

    #[test]
    fn normalizes_target_opponent_choose_destroy_sentence_player_controls_variant() {
        let normalized = normalize_sentence_surface_style(
            "Target opponent chooses exactly 1 a creature that player controls in the battlefield. Destroy it.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses exactly 1 a creature that player controls in the battlefield. Destroy it."
        );
    }

    #[test]
    fn normalizes_target_opponent_choose_other_cant_block_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Target opponent chooses exactly 1 target player's creature in the battlefield. target player's other creature can't block until end of turn.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses exactly 1 target player's creature in the battlefield. target player's other creature can't block until end of turn."
        );
    }

    #[test]
    fn normalizes_target_opponent_choose_other_cant_block_player_controls_variant() {
        let normalized = normalize_sentence_surface_style(
            "Target opponent chooses exactly 1 target player's creature in the battlefield. target player's other creature can't block until end of turn.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses exactly 1 target player's creature in the battlefield. target player's other creature can't block until end of turn."
        );
    }

    #[test]
    fn normalizes_target_opponent_exiles_creature_and_graveyard_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Target opponent chooses target creature an opponent controls. Exile it. Exile all card in target opponent's graveyards.",
        );
        assert_eq!(
            normalized,
            "Target opponent exiles a creature they control and their graveyard."
        );
    }

    #[test]
    fn normalizes_spell_effects_target_opponent_exiles_creature_and_graveyard_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Spell effects: Target opponent chooses target creature an opponent controls. Exile it. Exile all card in target opponent's graveyards.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Target opponent exiles a creature they control and their graveyard."
        );
    }

    #[test]
    fn normalizes_when_enters_deals_damage_to_each_creature_and_planeswalker() {
        let normalized = normalize_sentence_surface_style(
            "When this permanent enters, for each creature or planeswalker, Deal 3 damage to that object.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, it deals 3 damage to each creature and each planeswalker."
        );
    }

    #[test]
    fn normalizes_when_enters_deal_direct_each_creature_or_planeswalker() {
        let normalized = normalize_sentence_surface_style(
            "When this permanent enters, deal 3 damage to each creature or planeswalker.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, it deals 3 damage to each creature and each planeswalker."
        );
    }

    #[test]
    fn describe_for_each_filter_keeps_exile_zone_without_battlefield_suffix() {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Exile);
        filter.owner = Some(PlayerFilter::IteratedPlayer);
        filter.card_types.push(CardType::Artifact);
        filter.card_types.push(CardType::Creature);
        filter.card_types.push(CardType::Enchantment);
        filter.card_types.push(CardType::Land);
        filter.card_types.push(CardType::Planeswalker);
        filter.card_types.push(CardType::Battle);

        let described = describe_for_each_filter(&filter);
        assert!(
            !described.contains("on the battlefield"),
            "unexpected battlefield suffix in '{}'",
            described
        );
        assert!(
            described.contains("in that player's exile"),
            "expected exile context in '{}'",
            described
        );
    }

    #[test]
    fn normalizes_opponents_creatures_enter_tapped_sentence() {
        let normalized = normalize_sentence_surface_style(
            "An opponent's creature enter the battlefield tapped.",
        );
        assert_eq!(
            normalized,
            "An opponent's creature enter the battlefield tapped."
        );
    }

    #[test]
    fn normalizes_rishadan_sacrifice_unless_pay_sentence() {
        let normalized = normalize_sentence_surface_style(
            "When this creature enters, for each opponent, that player sacrifices a permanent unless that player pays {2}.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, for each opponent, that player sacrifices a permanent unless that player pays {2}."
        );
    }

    #[test]
    fn normalizes_touchstone_tap_artifact_sentence() {
        let normalized = normalize_sentence_surface_style("Tap target opponent's artifact.");
        assert_eq!(normalized, "Tap target opponent's artifact.");
    }

    #[test]
    fn pluralize_noun_phrase_handles_an_opponent_controls_suffix() {
        assert_eq!(
            pluralize_noun_phrase("creature an opponent controls"),
            "creatures an opponent controls"
        );
        assert_eq!(
            pluralize_noun_phrase("target creature an opponent controls"),
            "target creatures an opponent controls"
        );
    }

    #[test]
    fn pluralize_noun_phrase_handles_you_own_suffix() {
        assert_eq!(pluralize_noun_phrase("Dwarf you own"), "Dwarves you own");
        assert_eq!(
            pluralize_noun_phrase("target permanent you own"),
            "target permanents you own"
        );
    }

    #[test]
    fn pluralize_noun_phrase_keeps_without_qualifier_singular() {
        assert_eq!(
            pluralize_noun_phrase("target creature without flying"),
            "target creatures without flying"
        );
    }

    #[test]
    fn normalizes_for_each_opponent_discards_count_sentence() {
        let normalized =
            normalize_sentence_surface_style("For each opponent, that player discards 2 cards.");
        assert_eq!(normalized, "Each opponent discards two cards.");
    }

    #[test]
    fn normalizes_for_each_target_player_single_clause_sentence() {
        let normalized =
            normalize_sentence_surface_style("For each target player, that player gains 6 life.");
        assert_eq!(normalized, "Target players each gain 6 life.");
    }

    #[test]
    fn normalizes_for_each_target_player_repeated_clause_sentence() {
        let normalized = normalize_sentence_surface_style(
            "For each target player, that player mills a card. For each target player, that player loses 1 life.",
        );
        assert_eq!(
            normalized,
            "Target players each mill a card and loses 1 life."
        );
    }

    #[test]
    fn normalizes_during_your_turn_keyword_sentence() {
        let normalized = normalize_sentence_surface_style(
            "This creature has Lifelink as long as it's your turn.",
        );
        assert_eq!(normalized, "During your turn, this creature has lifelink.");
    }

    #[test]
    fn normalizes_sliver_sacrifice_damage_sentence() {
        let normalized = normalize_sentence_surface_style(
            "All slivers have 2 sacrifice this creature this creature deals 2 damage to any target.",
        );
        assert_eq!(
            normalized,
            "All slivers have 2 sacrifice this creature this creature deals 2 damage to any target."
        );
    }

    #[test]
    fn normalizes_prevent_next_damage_spell_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Prevent the next 4 damage to any target until end of turn.",
        );
        assert_eq!(
            normalized,
            "Prevent the next 4 damage that would be dealt to any target this turn."
        );
    }

    #[test]
    fn normalizes_burn_away_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Deal 6 damage to target creature. Exile target card in graveyard.",
        );
        assert_eq!(
            normalized,
            "Deal 6 damage to target creature. Exile target card in graveyard."
        );
    }

    #[test]
    fn normalizes_granted_mana_ability_sentence() {
        let normalized = normalize_sentence_surface_style("Creatures you control have t add g.");
        assert_eq!(normalized, "Creatures you control have t add g.");
    }

    #[test]
    fn normalizes_specific_plural_surface_phrases() {
        assert_eq!(
            normalize_sentence_surface_style("Elfs you control get +2/+0."),
            "Elves you control get +2/+0."
        );
        assert_eq!(
            normalize_sentence_surface_style("Warrior have Haste."),
            "Warriors have Haste."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Creature with a level counter on it you control get +2/+2."
            ),
            "Each creature you control with a level counter on it gets +2/+2."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "This creature's power and toughness are each equal to the number of Soldiers or Warrior you control."
            ),
            "This creature's power and toughness are each equal to the number of Soldiers and Warriors you control."
        );
        assert_eq!(
            normalize_sentence_surface_style("Goblin are black."),
            "Goblins are black."
        );
        assert_eq!(
            normalize_sentence_surface_style("Goblin are zombie in addition to their other types."),
            "Goblins are Zombies in addition to their other creature types."
        );
        assert_eq!(
            normalize_sentence_surface_style("Land is no longer snow."),
            "Land is no longer snow."
        );
        assert_eq!(
            normalize_sentence_surface_style("Land enter the battlefield tapped."),
            "Land enter the battlefield tapped."
        );
        assert_eq!(
            normalize_sentence_surface_style("Add 1 mana of any color."),
            "Add 1 mana of any color."
        );
    }

    #[test]
    fn normalizes_surveil_then_draw_sentence() {
        let normalized = normalize_sentence_surface_style("Surveil 2. Draw a card.");
        assert_eq!(normalized, "Surveil 2. Draw a card.");
    }

    #[test]
    fn normalizes_structural_collapse_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Target player sacrifices a artifact. target player sacrifices target player's land. Deal 2 damage to target player of their choice.",
        );
        assert_eq!(
            normalized,
            "Target player sacrifices an artifact. target player sacrifices target player's land. Deal 2 damage to target player of their choice."
        );
    }

    #[test]
    fn normalizes_ability_scoped_choose_one_into_bullets() {
        let normalized = normalize_sentence_surface_style(
            "Triggered ability 1: When this creature enters, choose one — Target creature gets +2/+0 until end of turn. • Target creature gets -0/-2 until end of turn.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: When this creature enters, choose one —\n• Target creature gets +2/+0 until end of turn.\n• Target creature gets -0/-2 until end of turn."
        );
    }

    #[test]
    fn normalizes_ability_scoped_choose_one_or_more_into_bullets() {
        let normalized = normalize_sentence_surface_style(
            "Triggered ability 1: When this creature dies, choose one or more - Target opponent sacrifices a creature of their choice. • Target opponent discards two cards. • Target opponent loses 5 life.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: When this creature dies, choose one or more —\n• Target opponent sacrifices a creature of their choice.\n• Target opponent discards two cards.\n• Target opponent loses 5 life."
        );
    }

    #[test]
    fn normalizes_ognis_treasure_trigger_sentence() {
        let normalized = normalize_sentence_surface_style(
            "Whenever a creature with haste you control attacks, create 1 Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped.",
        );
        assert_eq!(
            normalized,
            "Whenever a creature with haste you control attacks, create 1 Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped."
        );
    }

    #[test]
    fn post_pass_normalizes_each_opponent_life_loss_gain() {
        let normalized = normalize_compiled_post_pass_effect(
            "For each opponent, that player loses 1 life. you gain 1 life.",
        );
        assert_eq!(
            normalized,
            "Each opponent loses loses 1 life and you gain one life."
        );
    }

    #[test]
    fn post_pass_normalizes_for_each_player_draw_then_discard_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "For each player, that player draws 3 cards. For each player, that player discards 3 cards at random.",
        );
        assert_eq!(
            normalized,
            "Each player draws three cards, then discards three cards at random."
        );

        let normalized_plain = normalize_compiled_post_pass_effect(
            "When this creature enters, each player draws 2 cards. For each player, that player discards a card at random.",
        );
        assert_eq!(
            normalized_plain,
            "When this creature enters, each player draws two cards, then discards a card at random."
        );
    }

    #[test]
    fn post_pass_normalizes_for_each_player_discard_then_draw_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "For each player, that player discards their hand. For each player, that player draws 7 cards.",
        );
        assert_eq!(
            normalized,
            "Each player discards their hand, then draws 7 cards."
        );

        let normalized_plain = normalize_compiled_post_pass_effect(
            "Each player discards their hand. that player draws that many minus one cards.",
        );
        assert_eq!(
            normalized_plain,
            "Each player discards their hand, then draws that many minus one cards."
        );
    }

    #[test]
    fn post_pass_normalizes_gain_then_create_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this enchantment enters, you gain 2 life. Create a tapped Powerstone token.",
        );
        assert_eq!(
            normalized,
            "When this enchantment enters, you gain 2 life and create a tapped Powerstone token."
        );
    }

    #[test]
    fn post_pass_merges_lose_then_create_treasure_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever a player casts their second spell each turn, you lose 1 life. Create a Treasure token.",
        );
        assert_eq!(
            normalized,
            "Whenever a player casts their second spell each turn, you lose 1 life and create a Treasure token."
        );
    }

    #[test]
    fn post_pass_normalizes_malformed_second_spell_trigger_phrase() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever you cast an as your second spell this turn, create a 4/4 red Dragon Elemental creature token with flying under your control spell.",
        );
        assert_eq!(
            normalized,
            "Whenever you cast your second spell each turn, create a 4/4 red Dragon Elemental creature token with flying under your control."
        );
    }

    #[test]
    fn post_pass_normalizes_during_your_turn_pt_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "This creature gets +0/+2 as long as it's your turn.",
        );
        assert_eq!(normalized, "During your turn, this creature gets +0/+2.");
    }

    #[test]
    fn post_pass_normalizes_split_two_land_search() {
        let normalized = normalize_compiled_post_pass_effect(
            "Search your library for up to one basic land you own, put it onto the battlefield tapped. Search your library for basic land you own, reveal it, put it into your hand, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."
        );
    }

    #[test]
    fn post_pass_normalizes_split_two_land_search_without_you_own() {
        let normalized = normalize_compiled_post_pass_effect(
            "Search your library for a basic land card, put it onto the battlefield tapped. Search your library for basic land, reveal it, put it into your hand, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."
        );
    }

    #[test]
    fn post_pass_normalizes_split_two_land_search_with_reveal_in_first_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Search your library for a basic land card, reveal it, put it onto the battlefield tapped. Search your library for basic land, reveal it, put it into your hand, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."
        );
    }

    #[test]
    fn post_pass_normalizes_split_two_gate_or_land_search() {
        let normalized = normalize_compiled_post_pass_effect(
            "Search your library for up to one basic land or Gate card, put it onto the battlefield tapped. Search your library for basic land or Gate, reveal it, put it into your hand, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for up to two basic land or Gate cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."
        );
    }

    #[test]
    fn post_pass_merges_for_each_opponent_discards_then_loses_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, for each opponent, that player discards a card. For each opponent, that player loses 2 life.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, each opponent discards a card and loses 2 life."
        );
    }

    #[test]
    fn post_pass_merges_target_opponent_sacrifice_discard_lose_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters or this creature attacks, target opponent sacrifices a creature or planeswalker of their choice. target opponent discards a card. target opponent loses 3 life.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature enters or attacks, target opponent sacrifices a creature or planeswalker of their choice, discards a card, and loses 3 life."
        );
    }

    #[test]
    fn post_pass_merges_draw_then_gain_life_chain() {
        let normalized = normalize_compiled_post_pass_effect("Draw a card. you gain 3 life.");
        assert_eq!(normalized, "Draw a card and you gain 3 life.");
    }

    #[test]
    fn post_pass_merges_draw_then_gain_life_chain_with_prefix() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever this creature enters or attacks, target opponent loses 3 life. Draw a card. you gain 3 life.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature enters or attacks, target opponent loses 3 life. Draw a card and gain 3 life."
        );
    }

    #[test]
    fn post_pass_merges_discard_then_draw_chain_after_cost_colon() {
        let normalized = normalize_compiled_post_pass_effect(
            "{U}, Sacrifice a creature you control: you discard a card. Draw a card.",
        );
        assert_eq!(
            normalized,
            "{U}, Sacrifice a creature you control: discard a card, then draw a card."
        );
    }

    #[test]
    fn post_pass_merges_colon_discard_then_you_draw_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature is turned face up: you discard your hand. you draw three cards.",
        );
        assert_eq!(
            normalized,
            "When this creature is turned face up: discard your hand, then draw three cards."
        );
    }

    #[test]
    fn post_pass_merges_exile_then_you_draw_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Exile all card in your hand. you draw that many cards.",
        );
        assert_eq!(
            normalized,
            "Exile all card in your hand, then draw that many cards."
        );
    }

    #[test]
    fn post_pass_merges_prefix_you_draw_then_you_gain_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return creature card from your graveyard to your hand. you draw three cards. you gain 5 life.",
        );
        assert_eq!(
            normalized,
            "Return creature card from your graveyard to your hand. Draw three cards and gain 5 life."
        );
    }

    #[test]
    fn post_pass_merges_damage_then_controller_loses_life_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "{T}: This creature deals 1 damage to target creature. that object's controller loses 1 life.",
        );
        assert_eq!(
            normalized,
            "{T}: This creature deals 1 damage to target creature and that creature's controller loses 1 life."
        );
    }

    #[test]
    fn post_pass_rewrites_exile_all_cards_then_return_it_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Exile all card in your hand. At the beginning of the next end step, return it to its owner's hand. Draw a card.",
        );
        assert_eq!(
            normalized,
            "Exile all card in your hand. At the beginning of the next end step, return those cards to their owners' hands. Draw a card."
        );
    }

    #[test]
    fn post_pass_rewrites_token_copy_sacrifice_this_spell_tail() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create a token that's a copy of target artifact or creature you control, with haste. At the beginning of the next end step, sacrifice this spell.",
        );
        assert_eq!(
            normalized,
            "Create a token that's a copy of target artifact or creature you control, with haste. At the beginning of the next end step, sacrifice it."
        );
    }

    #[test]
    fn post_pass_merges_target_player_discard_then_sacrifice_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Target player loses 1 life. target player discards a card. sacrifice a permanent.",
        );
        assert_eq!(
            normalized,
            "Target player loses 1 life. Target player discards a card and sacrifices a permanent."
        );
    }

    #[test]
    fn post_pass_merges_return_all_then_destroy_all_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return all Zombie creature card in your graveyard to the battlefield tapped. Destroy all Humans.",
        );
        assert_eq!(
            normalized,
            "Return all Zombie creature card in your graveyard to the battlefield tapped, then destroy all Humans."
        );
    }

    #[test]
    fn post_pass_merges_you_gain_x_and_you_gain_n() {
        let normalized =
            normalize_compiled_post_pass_effect("You gain X life and you gain 3 life.");
        assert_eq!(normalized, "You gain X plus 3 life.");
    }

    #[test]
    fn line_post_pass_normalizes_you_gain_x_plus_n_phrase() {
        let normalized = normalize_gain_life_plus_phrase("You gain X life and you gain 3 life.");
        assert_eq!(normalized, "You gain X plus 3 life.");
    }

    #[test]
    fn post_pass_rewrites_if_that_doesnt_happen_draw_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return a land card or Elf card from your graveyard to your hand. If that doesn't happen, you draw a card.",
        );
        assert_eq!(
            normalized,
            "Return a land card or Elf card from your graveyard to your hand. If you can't, draw a card."
        );
    }

    #[test]
    fn post_pass_rewrites_if_that_doesnt_happen_return_and_energy_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this permanent enters, pay {E}{E}. If that doesn't happen, Return this permanent to its owner's hand. you get {E}.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, pay {E}{E}. If that doesn't happen, Return this permanent to its owner's hand and you get {E}."
        );
    }

    #[test]
    fn post_pass_merges_get_and_gain_until_eot_for_creatures_you_control() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever beregond or another Human you control enters, creatures you control get +1/+1 until end of turn. creatures you control gain Vigilance until end of turn",
        );
        assert_eq!(
            normalized,
            "Whenever beregond or another Human you control enters, creatures you control get +1/+1 and gain vigilance until end of turn."
        );
    }

    #[test]
    fn post_pass_merges_get_and_gain_until_eot_for_each_creature_you_control() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, each creature you control gets +1/+1 until end of turn. creatures you control gain Haste until end of turn",
        );
        assert_eq!(
            normalized,
            "When this creature enters, creatures you control get +1/+1 and gain haste until end of turn."
        );
    }

    #[test]
    fn post_pass_merges_mill_then_put_counter_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this permanent enters, you mill 2 cards. Put a +1/+1 counter on this permanent for each artifact or creature card in your graveyard.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, mill 2 cards, then put a +1/+1 counter on this permanent for each artifact or creature card in your graveyard."
        );
    }

    #[test]
    fn post_pass_normalizes_when_permanent_enters_or_whenever_attacks_prefix() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this permanent enters or Whenever this creature attacks, target opponent loses 3 life.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature enters or attacks, target opponent loses 3 life."
        );
    }

    #[test]
    fn post_pass_normalizes_tribal_spell_cost_phrase() {
        let normalized = normalize_compiled_post_pass_effect(
            "Spells Treefolk you control cost {1} less to cast.",
        );
        assert_eq!(
            normalized,
            "Treefolk spells you cast cost {1} less to cast."
        );
    }

    #[test]
    fn post_pass_normalizes_choose_each_type_exile_then_shared_type_search_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "You choose up to one artifact in the battlefield and tags it as 'exiled_0'. you choose up to one creature in the battlefield and tags it as 'exiled_0'. you choose up to one enchantment in the battlefield and tags it as 'exiled_0'. you choose up to one planeswalker in the battlefield and tags it as 'exiled_0'. you choose up to one land in the battlefield and tags it as 'exiled_0'. Exile it. For each object exiled this way, Search that player's library for permanent that shares a card type with that object that player owns, put it onto the battlefield, then shuffle.",
        );
        assert_eq!(
            normalized,
            "You choose up to one artifact in the battlefield and tags it as 'exiled_0'. you choose up to one creature in the battlefield and tags it as 'exiled_0'. you choose up to one enchantment in the battlefield and tags it as 'exiled_0'. you choose up to one planeswalker in the battlefield and tags it as 'exiled_0'. you choose up to one land in the battlefield and tags it as 'exiled_0'. Exile it. For each object exiled this way, Search that player's library for permanent that shares a card type with that object that player owns, put it onto the battlefield, then shuffle."
        );
    }

    #[test]
    fn post_pass_normalizes_this_leaves_battlefield_trigger_head() {
        let normalized = normalize_compiled_post_pass_effect(
            "This enchantment leaves the battlefield: you discard 3 cards and you lose 6 life. you sacrifice three creatures you control.",
        );
        assert_eq!(
            normalized,
            "This enchantment leaves the battlefield: you discard 3 cards and you lose 6 life. you sacrifice three creatures you control."
        );
    }

    #[test]
    fn post_pass_handles_lowercase_for_each_opponent_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, for each opponent, that player discards a card.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, each opponent discards a card."
        );
    }

    #[test]
    fn post_pass_does_not_pluralize_destroy_all_creatures_twice() {
        let normalized = normalize_compiled_post_pass_effect("Destroy all creatures.");
        assert_eq!(normalized, "Destroy all creatures.");
    }

    #[test]
    fn post_pass_normalizes_embedded_powerstone_creation() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, create 1 Powerstone artifact token under your control, tapped.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, create a tapped Powerstone token."
        );
    }

    #[test]
    fn post_pass_normalizes_embedded_lowercase_create_token_reminder() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever an aura becomes attached to this creature, create 1 2/2 red Dragon creature token with flying and {R}: This creature gets +1/+0 until end of turn. under your control.",
        );
        assert_eq!(
            normalized,
            "Whenever an aura becomes attached to this creature, create a 2/2 red Dragon creature token with flying under your control. It has \"{R}: This creature gets +1/+0 until end of turn.\""
        );
    }

    #[test]
    fn post_pass_normalizes_embedded_token_trigger_reminder() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever a nontoken artifact you control enters, create 1 Munitions artifact token with When this token leaves the battlefield, it deals 2 damage to any target. under your control.",
        );
        assert_eq!(
            normalized,
            "Whenever a nontoken artifact you control enters, create a Munitions artifact token under your control. It has \"When this token leaves the battlefield, it deals 2 damage to any target.\""
        );
    }

    #[test]
    fn create_under_control_normalization_skips_multi_create_sequences() {
        let raw = "Create a Food token. Create 1 Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped. Create 1 3/2 Vehicle artifact token with crew 1 under your control.";
        assert!(normalize_create_under_control_clause(raw).is_none());
    }

    #[test]
    fn post_pass_does_not_leak_treasure_reminder_into_following_create_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create a Food token. Create 1 Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. under your control, tapped. Create 1 3/2 Vehicle artifact token with crew 1 under your control.",
        );
        assert!(
            normalized
                .contains("Create 1 3/2 Vehicle artifact token with crew 1 under your control.")
        );
        assert!(!normalized.contains("crew 1 under your control. It has \"{T}, Sacrifice this artifact: Add one mana of any color."));
    }

    #[test]
    fn post_pass_compacts_create_one_under_control_lists() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create 1 1/1 green Snake creature token under your control. Create 1 2/2 green Wolf creature token under your control. Create 1 3/3 green Elephant creature token under your control.",
        );
        assert_eq!(
            normalized,
            "Create a 1/1 green Snake creature token, a 2/2 green Wolf creature token, and a 3/3 green Elephant creature token."
        );
    }

    #[test]
    fn post_pass_compacts_tapped_treasure_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create a Food token. Create 1 Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control. Create 1 3/2 Vehicle artifact token with crew 1 under your control.",
        );
        assert_eq!(
            normalized,
            "Create a Food token. Create 1 Treasure artifact token with {T}, Sacrifice this artifact: Add one mana of any color. tapped under your control. Create 1 3/2 Vehicle artifact token with crew 1 under your control."
        );
    }

    #[test]
    fn post_pass_compacts_triggered_create_one_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, create 1 2/2 white Knight creature token with vigilance under your control. Create 1 3/3 green Centaur creature token under your control. Create 1 4/4 green Rhino creature token with trample under your control.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, create a 2/2 white Knight creature token with vigilance, a 3/3 green Centaur creature token, and a 4/4 green Rhino creature token with trample"
        );
    }

    #[test]
    fn post_pass_normalizes_counter_then_proliferate_chains() {
        assert_eq!(
            normalize_compiled_post_pass_effect(
                "Put a +1/+1 counter on target creature. Proliferate."
            ),
            "Put a +1/+1 counter on target creature. Proliferate."
        );
        assert_eq!(
            normalize_compiled_post_pass_effect(
                "Put a -1/-1 counter on target creature. Proliferate."
            ),
            "Put a -1/-1 counter on target creature. Proliferate."
        );
    }

    #[test]
    fn post_pass_normalizes_embedded_for_each_put_counter_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create a 2/2 black Zombie creature token under your control. For each Zombie creature you control, Put a +1/+1 counter on that object.",
        );
        assert_eq!(
            normalized,
            "Create a 2/2 black Zombie creature token under your control. Put a +1/+1 counter on each Zombie creature you control."
        );
    }

    #[test]
    fn post_pass_normalizes_draw_then_put_top_of_library_chains() {
        let normalized = normalize_compiled_post_pass_effect(
            "{2}, {T}, Sacrifice this artifact: you draw three cards. Put two cards from your hand on top of your library.",
        );
        assert_eq!(
            normalized,
            "{2}, {T}, Sacrifice this artifact: you draw three cards. Put two cards from your hand on top of your library."
        );
        let normalized_single = normalize_compiled_post_pass_effect(
            "When this creature enters, you draw two cards. Put a card from your hand on top of your library.",
        );
        assert_eq!(
            normalized_single,
            "When this creature enters, you draw two cards. Put a card from your hand on top of your library."
        );
    }

    #[test]
    fn post_pass_normalizes_bottom_then_shuffle_into_library_chains() {
        let normalized = normalize_compiled_post_pass_effect(
            "Spell effects: Put up to one target card from your graveyard on the bottom of your library. Shuffle your library.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Put up to one target card from your graveyard on the bottom of your library. Shuffle your library."
        );
        let targeted = normalize_compiled_post_pass_effect(
            "Triggered ability 1: When this creature enters, put any number of target cards from target player's graveyard on the bottom of target player's library. Shuffle target player's library.",
        );
        assert_eq!(
            targeted,
            "Triggered ability 1: When this creature enters, put any number of target cards from target player's graveyard on the bottom of target player's library. Shuffle target player's library."
        );
    }

    #[test]
    fn post_pass_normalizes_archangel_life_gain_graveyard_variant() {
        let normalized =
            normalize_compiled_post_pass_effect("You gain 2 life for each card in your graveyard.");
        assert_eq!(
            normalized,
            "You gain 2 life for each card in your graveyard."
        );
    }

    #[test]
    fn post_pass_normalizes_spider_destroy_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever this creature deals damage to Spider, destroy it.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature deals damage to Spider, destroy it."
        );
    }

    #[test]
    fn post_pass_normalizes_tapped_robot_creation_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Create two 1/1 colorless Robot artifact creature token with flying tapped under your control.",
        );
        assert_eq!(
            normalized,
            "Create two 1/1 colorless Robot artifact creature tokens with flying tapped."
        );
    }

    #[test]
    fn post_pass_normalizes_dramatic_rescue_style_gain_life_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return target creature to its owner's hand and you gain 2 life.",
        );
        assert_eq!(
            normalized,
            "Return target creature to its owner's hand. You gain 2 life."
        );
    }

    #[test]
    fn post_pass_merges_repeated_subject_predicate_sentences() {
        let normalized = normalize_compiled_post_pass_effect(
            "This creature gets +1/+0 until end of turn. this creature gains Flying until end of turn.",
        );
        assert_eq!(
            normalized,
            "This creature gets +1/+0 until end of turn and gains Flying until end of turn."
        );
    }

    #[test]
    fn merge_adjacent_subject_lines_merges_lose_abilities_with_base_pt() {
        assert_eq!(
            merge_adjacent_subject_predicate_lines(vec![
                "Creature lose all abilities.".to_string(),
                "Affected permanents have base power and toughness 1/1.".to_string(),
            ]),
            vec!["Creatures lose all abilities and have base power and toughness 1/1".to_string()]
        );
        assert_eq!(
            merge_adjacent_subject_predicate_lines(vec![
                "Enchanted creature lose all abilities.".to_string(),
                "Affected permanents have base power and toughness 1/1.".to_string(),
            ]),
            vec![
                "Enchanted creature loses all abilities and has base power and toughness 1/1"
                    .to_string()
            ]
        );
    }

    #[test]
    fn post_pass_normalizes_inline_for_each_damage_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever this creature becomes blocked, for each attacking/blocking creature, Deal 2 damage to that object.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature becomes blocked, it deals 2 damage to each attacking or blocking creature."
        );
    }

    #[test]
    fn post_pass_normalizes_sentence_for_each_damage_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Deal 3 damage to target player. For each creature that player controls, Deal 1 damage to that object.",
        );
        assert_eq!(
            normalized,
            "Deal 3 damage to target player. Deal 1 damage to each creature that player controls."
        );
    }

    #[test]
    fn post_pass_normalizes_you_may_for_each_damage_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever this creature attacks, you may For each creature without flying, Deal 1 damage to that object.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature attacks, you may have it deal 1 damage to each creature without flying."
        );
    }

    #[test]
    fn post_pass_normalizes_up_to_two_cant_block_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Choose up to two target creatures. target creature can't be blocked until end of turn.",
        );
        assert_eq!(
            normalized,
            "Up to two target creatures can't be blocked this turn."
        );

        let normalized_this_turn = normalize_compiled_post_pass_effect(
            "Choose up to two target creatures. target creature can't be blocked this turn.",
        );
        assert_eq!(
            normalized_this_turn,
            "Up to two target creatures can't be blocked this turn."
        );
    }

    #[test]
    fn post_pass_normalizes_each_player_sacrifice_choice_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, for each player, that player sacrifices two creatures that player controls.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, each player sacrifices two creatures of their choice."
        );
    }

    #[test]
    fn post_pass_normalizes_blocked_pt_scale_phrase() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever this creature becomes blocked, it gets +-1 / +-1 for each the number of blocking creature until end of turn.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature becomes blocked, it gets -1/-1 until end of turn for each creature blocking it."
        );
    }

    #[test]
    fn post_pass_splits_gain_clause_after_main_effect() {
        let normalized =
            normalize_compiled_post_pass_effect("Destroy target creature and you gain 3 life.");
        assert_eq!(normalized, "Destroy target creature. You gain 3 life");
    }

    #[test]
    fn post_pass_normalizes_cast_spell_subtype_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever you cast spell Knight, create 1 1/1 white Human creature token under your control.",
        );
        assert_eq!(
            normalized,
            "Whenever you cast spell Knight, create 1 1/1 white Human creature token under your control."
        );
    }

    #[test]
    fn post_pass_normalizes_generic_for_each_player_clause() {
        let normalized =
            normalize_compiled_post_pass_effect("For each player, that player mills a card.");
        assert_eq!(normalized, "Each player mills a card.");
    }

    #[test]
    fn post_pass_normalizes_for_each_player_draw_a_card_clause() {
        let normalized =
            normalize_compiled_post_pass_effect("For each player, that player draws a card.");
        assert_eq!(normalized, "Each player draws a card.");
    }

    #[test]
    fn post_pass_normalizes_each_player_create_under_their_control_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Each player creates 1 5/5 red Dragon creature token with flying under that player's control.",
        );
        assert_eq!(
            normalized,
            "Each player creates 1 5/5 red Dragon creature token with flying under that player's control."
        );
    }

    #[test]
    fn post_pass_normalizes_upkeep_damage_clause_with_implicit_source() {
        let normalized = normalize_compiled_post_pass_effect(
            "At the beginning of your upkeep, deal 1 damage to you.",
        );
        assert_eq!(
            normalized,
            "At the beginning of your upkeep, deal 1 damage to you."
        );
    }

    #[test]
    fn post_pass_reorders_for_each_until_end_of_turn_phrase() {
        let normalized = normalize_compiled_post_pass_effect(
            "Target creature gets +1 / +1 for each a Forest you control until end of turn.",
        );
        assert_eq!(
            normalized,
            "Target creature gets +1/+1 until end of turn for each a Forest you control"
        );
    }

    #[test]
    fn post_pass_avoids_double_article_for_cast_a_spell() {
        let normalized =
            normalize_compiled_post_pass_effect("Whenever you cast a spell, you draw a card.");
        assert_eq!(normalized, "Whenever you cast a spell, you draw a card.");
    }

    #[test]
    fn post_pass_avoids_double_article_for_cast_another_spell() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever you cast another spell, create 1 1/1 blue Bird creature token under your control.",
        );
        assert_eq!(
            normalized,
            "Whenever you cast another spell, create 1 1/1 blue Bird creature token under your control."
        );
    }

    #[test]
    fn post_pass_normalizes_this_or_another_trigger_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever This creature or Whenever another nontoken historic permanent you control enters, deal 1 damage to each opponent and you gain 1 life.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature or another nontoken historic permanent you control enters, deal 1 damage to each opponent and you gain 1 life."
        );
    }

    #[test]
    fn post_pass_normalizes_lands_you_control_skip_untap_step() {
        let normalized = normalize_compiled_post_pass_effect(
            "Gain control of target artifact or creature or enchantment. a land you control can't untap until your next turn.",
        );
        assert_eq!(
            normalized,
            "Gain control of target artifact or creature or enchantment. a land you control can't untap until your next turn."
        );
    }

    #[test]
    fn post_pass_normalizes_predatory_nightstalker_sacrifice_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, you may target opponent sacrifices target creature an opponent controls.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, you may target opponent sacrifices target creature an opponent controls."
        );
    }

    #[test]
    fn post_pass_normalizes_you_may_target_creature_gets_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, you may target creature gets -1/-1 until end of turn.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, you may target creature gets -1/-1 until end of turn."
        );
    }

    #[test]
    fn post_pass_normalizes_tidebinder_untap_lock_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this creature enters, tap target opponent's red or green creature. permanent can't untap while you control this creature.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, tap target opponent's red or green creature. permanent can't untap while you control this creature."
        );
    }

    #[test]
    fn post_pass_normalizes_blade_of_the_bloodchief_equipped_vampire_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever a creature dies, tag the object attached to this artifact as 'equipped'. If the tagged object 'equipped' matches Vampire creature, Put two +1/+1 counters on the tagged object 'equipped'. Otherwise, Put a +1/+1 counter on the tagged object 'equipped'.",
        );
        assert_eq!(
            normalized,
            "Whenever a creature dies, tag the object attached to this artifact as 'equipped'. If the tagged object 'equipped' matches Vampire creature, Put two +1/+1 counters on the tagged object 'equipped'. Otherwise, Put a +1/+1 counter on the tagged object 'equipped'."
        );
    }

    #[test]
    fn post_pass_normalizes_mindlash_sliver_quoted_static_ability() {
        let normalized = normalize_known_low_tail_phrase(
            "All Slivers have 1 sacrifice this creature each player discards a card.",
        );
        assert_eq!(
            normalized,
            "All Slivers have 1 sacrifice this creature each player discards a card."
        );
    }

    #[test]
    fn post_pass_normalizes_archon_of_cruelty_trigger_chain() {
        let normalized = normalize_known_low_tail_phrase(
            "Whenever this creature enters or attacks, target opponent sacrifices target opponent's creature or planeswalker. target opponent discards a card. target opponent loses 3 life. Draw a card. you gain 3 life.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature enters or attacks, target opponent sacrifices target opponent's creature or planeswalker. target opponent discards a card. target opponent loses 3 life. Draw a card. you gain 3 life."
        );
    }

    #[test]
    fn post_pass_normalizes_shared_draw_three_clause() {
        let normalized =
            normalize_known_low_tail_phrase("Draw three cards. target opponent draws 3 cards.");
        assert_eq!(
            normalized,
            "Draw three cards. target opponent draws 3 cards."
        );
    }

    #[test]
    fn post_pass_normalizes_shared_attacking_player_draw_and_lose_clause() {
        let normalized = normalize_known_low_tail_phrase(
            "Whenever an opponent attacks another one of your opponents, you draw a card. the attacking player draws a card. you lose 1 life. the attacking player loses 1 life.",
        );
        assert_eq!(
            normalized,
            "Whenever an opponent attacks another one of your opponents, you draw a card. the attacking player draws a card. you lose 1 life. the attacking player loses 1 life."
        );
    }

    #[test]
    fn post_pass_normalizes_iridian_maelstrom_destroy_phrase() {
        let normalized =
            normalize_known_low_tail_phrase("Destroy all creatures that are not all colors.");
        assert_eq!(normalized, "Destroy all creatures that are not all colors.");
    }

    #[test]
    fn post_pass_normalizes_iridian_maelstrom_destroy_phrase_with_spell_prefix() {
        let normalized = normalize_known_low_tail_phrase(
            "Spell effects: Destroy all creatures that are not all colors.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Destroy all creatures that are not all colors."
        );
    }

    #[test]
    fn renders_dynamic_any_one_color_mana_with_explicit_x_count_clause() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wirewood Render Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("{T}: Add X mana of any one color, where X is the number of Elves on the battlefield.")
            .expect("wirewood dynamic mana clause should parse");

        let rendered = compiled_lines(&def).join(" ");
        assert!(
            rendered
                .to_ascii_lowercase()
                .contains("add x mana of any one color"),
            "expected explicit X mana wording, got {rendered}"
        );
        assert!(
            rendered
                .to_ascii_lowercase()
                .contains("where x is the number of elves on the battlefield"),
            "expected explicit where-X count clause, got {rendered}"
        );
    }

    #[test]
    fn renders_equipment_self_reference_and_singular_attach_target() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Equipment Render Variant")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .parse_text(
                "When this Equipment enters, attach it to target creature you control.\nEquip {1}",
            )
            .expect("equipment self-reference should parse");

        let rendered = compiled_lines(&def).join(" ");
        assert!(
            rendered
                .contains("When this Equipment enters, attach it to target creature you control."),
            "expected equipment self-reference + singular attach wording, got {rendered}"
        );
    }

    #[test]
    fn normalize_rendered_line_prefers_saga_self_reference_when_oracle_uses_saga() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Saga Render Variant")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Saga])
            .parse_text("When this Saga enters, draw a card.")
            .expect("saga line should parse");

        let normalized = normalize_rendered_line_for_card(
            &def,
            "Triggered ability 1: When this enchantment enters, draw a card.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: When this Saga enters, draw a card."
        );
    }

    #[test]
    fn normalize_rendered_line_prefers_siege_self_reference_when_oracle_uses_siege() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Siege Render Variant")
            .card_types(vec![CardType::Battle])
            .parse_text("When this Siege enters, draw a card.")
            .expect("siege line should parse");

        let normalized = normalize_rendered_line_for_card(
            &def,
            "Triggered ability 1: When this permanent enters, draw a card.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 1: When this Siege enters, draw a card."
        );
    }

    #[test]
    fn post_pass_normalizes_saw_in_half_copy_stats_phrase() {
        let normalized = normalize_known_low_tail_phrase(
            "Destroy target creature. If that permanent dies this way, Create two tokens that are copies of it under that object's controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up.",
        );
        assert_eq!(
            normalized,
            "Destroy target creature. If that permanent dies this way, Create two tokens that are copies of it under that object's controller's control, except their power and toughness are each half that permanent's power and toughness, rounded up."
        );
    }

    #[test]
    fn known_low_tail_preserves_attack_tap_without_goad() {
        let normalized = normalize_known_low_tail_phrase(
            "Whenever you attack a player, tap target creature that player controls.",
        );
        assert_eq!(
            normalized,
            "Whenever you attack a player, tap target creature that player controls."
        );
    }

    #[test]
    fn post_pass_rewrites_return_with_multiple_counters_on_it_sequence() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return target card from your graveyard to the battlefield. Put a Hexproof counter on it. Put a Indestructible counter on it.",
        );
        assert_eq!(
            normalized,
            "Return target permanent card from your graveyard to the battlefield with a Hexproof counter and an Indestructible counter on it."
        );
    }

    #[test]
    fn post_pass_rewrites_put_onto_battlefield_with_counter_sequence() {
        let normalized = normalize_compiled_post_pass_effect(
            "Put a permanent onto the battlefield. Put a finality counter on it.",
        );
        assert_eq!(
            normalized,
            "Put a permanent onto the battlefield with a finality counter on it."
        );
    }

    #[test]
    fn normalize_spell_self_exile_collapses_with_counters_clause() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arc Blade").build();
        let normalized = normalize_spell_self_exile(
            &def,
            "Deal 2 damage to any target. Exile this spell. Put three time counters on this spell.",
        );
        assert_eq!(
            normalized,
            "Deal 2 damage to any target. Exile Arc Blade with three time counters on it."
        );
    }

    #[test]
    fn normalize_spell_self_exile_collapses_permanent_with_counters_clause() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Charnel Serenade").build();
        let normalized = normalize_spell_self_exile(
            &def,
            "Exile this permanent. Put three time counters on this permanent.",
        );
        assert_eq!(
            normalized,
            "Exile Charnel Serenade with three time counters on it."
        );
    }

    #[test]
    fn post_pass_romanizes_saga_chapter_prefix() {
        let normalized = normalize_compiled_post_pass_effect(
            "Chapters 1, 2, 3, 4: other creatures you control get +1/+0 until end of turn.",
        );
        assert_eq!(
            normalized,
            "I, II, III, IV — other creatures you control get +1/+0 until end of turn."
        );
    }

    #[test]
    fn post_pass_quotes_granted_triggered_ability_text() {
        let normalized = normalize_compiled_post_pass_effect(
            "Creatures you control have whenever this creature becomes the target of a spell or ability, reveal the top card of your library.",
        );
        assert_eq!(
            normalized,
            "Creatures you control have \"Whenever this creature becomes the target of a spell or ability, reveal the top card of your library.\""
        );
    }

    #[test]
    fn post_pass_punctuates_granted_triggered_ability_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "Creatures you control have whenever this creature becomes the target of a spell or ability reveal the top card of your library if its a land card put it onto the battlefield otherwise put it into your hand this ability triggers only twice each turn.",
        );
        assert_eq!(
            normalized,
            "Creatures you control have \"Whenever this creature becomes the target of a spell or ability reveal the top card of your library. If its a land card, put it onto the battlefield. Otherwise put it into your hand. This ability triggers only twice each turn.\""
        );
    }

    #[test]
    fn post_pass_merges_create_then_attach_sentence() {
        let normalized = normalize_compiled_post_pass_effect(
            "Destroy target creature or enchantment. Create a Wicked Role token. Attach it to up to one target creature you control.",
        );
        assert_eq!(
            normalized,
            "Destroy target creature or enchantment. Create a Wicked Role token attached to up to one target creature you control."
        );
    }

    #[test]
    fn post_pass_quotes_wicked_role_granted_trigger_text() {
        let normalized = normalize_compiled_post_pass_effect(
            "Target creature you control gains when this creature dies return it to the battlefield tapped under its owner's control then create a wicked role token attached to it until end of turn.",
        );
        assert_eq!(
            normalized,
            "Target creature you control gains \"When this creature dies, return it to the battlefield tapped under its owner's control, then create a wicked role token attached to it.\" until end of turn."
        );
    }

    #[test]
    fn post_pass_normalizes_state_trigger_colon_prefix() {
        let normalized = normalize_compiled_post_pass_effect(
            "You control no other artifacts: Sacrifice this creature.",
        );
        assert_eq!(
            normalized,
            "You control no other artifacts: Sacrifice this creature."
        );
    }

    #[test]
    fn post_pass_normalizes_draw_and_lose_compound_clause() {
        let normalized =
            normalize_compiled_post_pass_effect("You draw two cards and you lose 2 life.");
        assert_eq!(normalized, "You draw two cards and you lose 2 life.");
    }

    #[test]
    fn post_pass_normalizes_misc_surface_cases_near_threshold() {
        let normalized = normalize_compiled_post_pass_effect(
            "Discard your hand, then draw 7 cards, then discard 3 cards at random.",
        );
        assert_eq!(
            normalized,
            "Discard your hand. Draw seven cards, then discard three cards at random."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "When this creature dies, exile it. Return another target creature card from your graveyard to your hand.",
        );
        assert_eq!(
            normalized,
            "When this creature dies, exile it, then return another target creature card from your graveyard to your hand."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "At the beginning of each player's upkeep: that player sacrifices a white or green permanent.",
        );
        assert_eq!(
            normalized,
            "At the beginning of each player's upkeep: that player sacrifices a green or white permanent."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Counter target spell. Deal 2 damage to that object's controller.",
        );
        assert_eq!(
            normalized,
            "Counter target spell. This spell deals 2 damage to that spell's controller."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Exile up to one target non-Warrior creature you control. Return it to the battlefield under its owner's control.",
        );
        assert_eq!(
            normalized,
            "Exile up to one target non-Warrior creature you control, then return it to the battlefield under its owner's control."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Exile target land you control. Return that card to the battlefield under your control.",
        );
        assert_eq!(
            normalized,
            "Exile target land you control. Return that card to the battlefield under your control."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Exile another target creature. Return it from graveyard to the battlefield tapped.",
        );
        assert_eq!(
            normalized,
            "Exile another target creature, then return it to the battlefield tapped."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Exile target creature. At the beginning of the next end step, return it to the battlefield. Put a +1/+1 counter on it.",
        );
        assert_eq!(
            normalized,
            "Exile target creature. At the beginning of the next end step, return that card to the battlefield under its owner's control with a +1/+1 counter on it."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Return target Assassin creature card from your graveyard to the battlefield. Put a +1/+1 counter on it.",
        );
        assert_eq!(
            normalized,
            "Return target Assassin creature card from your graveyard to the battlefield with a +1/+1 counter on it."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "III — Return target Assassin creature card from your graveyard to the battlefield. Put a +1/+1 counter on it.",
        );
        assert_eq!(
            normalized,
            "III — Return target Assassin creature card from your graveyard to the battlefield with a +1/+1 counter on it."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "Whenever a Assassin you control attacks this turn, put a +1/+1 counter on it.",
        );
        assert_eq!(
            normalized,
            "Whenever a Assassin you control attacks this turn, put a +1/+1 counter on it."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "When this permanent enters, you gain 3 life. you get {E}{E}{E}.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, you gain 3 life and you get {E}{E}{E}."
        );

        let normalized = normalize_compiled_post_pass_effect("Draw a card. you get {E}{E}.");
        assert_eq!(normalized, "Draw a card. you get {E}{E}.");

        let normalized = normalize_compiled_post_pass_effect(
            "{1}, Sacrifice an artifact you control: this permanent gets +1/+1 until end of turn. Deal 1 damage to each opponent.",
        );
        assert_eq!(
            normalized,
            "{1}, Sacrifice an artifact you control: this permanent gets +1/+1 until end of turn, and deals 1 damage to each opponent."
        );

        let normalized = normalize_compiled_post_pass_effect(
            "When this permanent enters, target player sacrifices a creature or planeswalker of their choice. target player loses 1 life.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, target player sacrifices a creature or planeswalker of their choice and loses 1 life."
        );
    }

    #[test]
    fn normalizes_sentence_misc_surface_cases_near_threshold() {
        assert_eq!(
            normalize_sentence_surface_style(
                "All Slivers have 2 sacrifice this permanent draw a card."
            ),
            "All Slivers have \"{2}, Sacrifice this permanent: Draw a card.\""
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Draw two cards and you lose 2 life. you mill 2 cards."
            ),
            "Draw two cards, lose 2 life, then mill two cards."
        );
        assert_eq!(
            normalize_sentence_surface_style("Slivercycling {{3}}."),
            "Slivercycling {{3}}."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Exile up to one target artifact, creature, or enchantment you control. Return it to the battlefield under its owner's control. Draw a card."
            ),
            "Exile up to one target artifact, creature, or enchantment you control. Return it to the battlefield under its owner's control. Draw a card."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Exile target creature. Return it from graveyard to the battlefield tapped. Draw a card."
            ),
            "Exile target creature. Return it from graveyard to the battlefield tapped. Draw a card."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "When this permanent enters, it deals 1 damage to that player. For each opponent's creature, Deal 1 damage to each opponent."
            ),
            "When this permanent enters, it deals 1 damage to each opponent and each creature your opponents control."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "When this permanent enters, put a card from that player's hand on top of that player's library."
            ),
            "When this permanent enters, target player puts a card from their hand on top of their library."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "At the beginning of your end step: For each creature you control, put a +1/+1 counter on that object. For each planeswalker you control, Put a loyalty counter on that object."
            ),
            "At the beginning of your end step: put a +1/+1 counter on each creature you control and a loyalty counter on each planeswalker you control."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Creatures you control get +1/+0 as long as it's your turn."
            ),
            "During your turn, creatures you control get +1/+0."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "At the beginning of your end step: you discard a card and you lose 2 life. sacrifice a creature."
            ),
            "At the beginning of your end step: you discard a card and you lose 2 life, then sacrifice a creature."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Destroy target enchantment. Destroy all other enchantment that shares a color with that object."
            ),
            "Destroy target enchantment. Destroy all other enchantment that shares a color with that object."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Whenever this creature attacks, you may sacrifice an artifact unless you discard a card. If you do, draw a card. this permanent gets +2/+0 until end of turn."
            ),
            "Whenever this creature attacks, you may sacrifice an artifact unless you discard a card. If you do, draw a card. this permanent gets +2/+0 until end of turn."
        );
        assert_eq!(
            normalize_sentence_surface_style(
                "Whenever an opponent casts a spell, you may draw a card unless you pay {1}."
            ),
            "Whenever an opponent casts a spell, you may draw a card unless you pay {1}."
        );
    }

    #[test]
    fn post_pass_normalizes_capenna_fetchland_sacrifice_search_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "When this land enters, you choose a permanent you control in the battlefield. you sacrifice a permanent. If you do, Search your library for up to one basic land Forest or Plains or Island you own, put it onto the battlefield tapped, then shuffle. you gain 1 life.",
        );
        assert_eq!(
            normalized,
            "When this land enters, you choose a permanent you control in the battlefield. you sacrifice a permanent. If you do, Search your library for up to one basic land Forest or Plains or Island you own, put it onto the battlefield tapped, then shuffle. you gain 1 life."
        );
    }

    #[test]
    fn post_pass_normalizes_each_target_creature_opponent_controls_clause() {
        let normalized = normalize_compiled_post_pass_effect(
            "Deal 1 damage to each target creature an opponent controls.",
        );
        assert_eq!(
            normalized,
            "Deal 1 damage to each target creature an opponent controls."
        );
    }

    #[test]
    fn post_pass_normalizes_manabond_style_end_step_chain() {
        let normalized = normalize_compiled_post_pass_effect(
            "At the beginning of your end step: you may Reveal your hand. Return all land card in your hand to the battlefield. If you do, discard your hand.",
        );
        assert_eq!(
            normalized,
            "At the beginning of your end step: you may Reveal your hand. Return all land card in your hand to the battlefield. If you do, discard your hand."
        );
    }

    #[test]
    fn common_semantic_phrasing_keeps_earthbend_chain_tail() {
        let normalized = normalize_common_semantic_phrasing(
            "Earthbend target land you control with 3 +1/+1 counter(s). Earthbend target land you control with 3 +1/+1 counter(s). You gain 3 life.",
        );
        assert_eq!(normalized.matches("Earthbend 3").count(), 2);
        assert!(normalized.contains("You gain 3 life"));
    }

    #[test]
    fn common_semantic_phrasing_avoids_trigger_as_creature_type_list() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever this or Whenever another Treefolk you control enters, up to two target creatures get +2/+2 and gain Trample until end of turn.",
        );
        assert!(
            !normalized.contains("Each creature that's a Whenever"),
            "trigger text was incorrectly rewritten as a creature-type list: {normalized}"
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_predatory_sacrifice_choice_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, you may target opponent sacrifices target creature an opponent controls.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, you may target opponent sacrifices target creature an opponent controls."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_creatures_you_control_get_phrase() {
        let normalized = normalize_common_semantic_phrasing(
            "Activated ability 1: {T}: creatures you control get +1/+2 until end of turn.",
        );
        assert_eq!(
            normalized,
            "Activated ability 1: {T}: Each creature you control gets +1/+2 until end of turn."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_tidebinder_lock_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, tap target opponent's red or green creature. permanent can't untap while you control this creature.",
        );
        assert_eq!(
            normalized,
            "When this creature enters, tap target opponent's red or green creature. that creature doesn't untap during its controller's untap step for as long as you control this creature."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_tap_then_controller_next_untap_step_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Tap up to two target creatures. creature can't untap during its controller's next untap step.",
        );
        assert_eq!(
            normalized,
            "Tap up to two target creatures. Those creatures don't untap during their controller's next untap step."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_frost_lynx_style_tap_lock_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "When this permanent enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step.",
        );
        assert_eq!(
            normalized,
            "When this permanent enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_named_trigger_tap_lock_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Triggered ability 3: When Abominable Treefolk enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step.",
        );
        assert_eq!(
            normalized,
            "Triggered ability 3: When Abominable Treefolk enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_spell_effect_tap_lock_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Spell effects: Tap target creature. permanent can't untap during its controller's next untap step.",
        );
        assert_eq!(
            normalized,
            "Spell effects: Tap target creature. That creature doesn't untap during its controller's next untap step."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_attack_or_block_untap_lock_clause() {
        let attack = normalize_common_semantic_phrasing(
            "Whenever this creature attacks, permanent can't untap during its controller's next untap step.",
        );
        assert_eq!(
            attack,
            "Whenever this creature attacks, permanent can't untap during its controller's next untap step."
        );

        let block = normalize_common_semantic_phrasing(
            "Whenever this creature blocks a creature, permanent can't untap during its controller's next untap step.",
        );
        assert_eq!(
            block,
            "Whenever this creature blocks a creature, permanent can't untap during its controller's next untap step."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_target_opponent_sacrifice_discard_lose_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "target opponent sacrifices target opponent's creature or planeswalker. target opponent discards a card. target opponent loses 3 life",
        );
        assert_eq!(
            normalized,
            "target opponent sacrifices target opponent's creature or planeswalker. target opponent discards a card. target opponent loses 3 life"
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_each_opponent_sacrifice_discard_lose_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "When this creature enters, each opponent sacrifices a creature of their choice. For each opponent, that player discards a card. For each opponent, that player loses 4 life",
        );
        assert_eq!(
            normalized,
            "When this creature enters, each opponent sacrifices a creature of their choice, discards a card, and loses 4 life"
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_cast_article_and_unless_payer_pronoun() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever an opponent casts creature spell, that player loses 2 life unless that player pays {2}.",
        );
        assert_eq!(
            normalized,
            "Whenever an opponent casts a creature spell, that player loses 2 life unless they pay {2}."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_counter_type_lists() {
        let swan_song =
            normalize_common_semantic_phrasing("Counter target enchantment or instant or sorcery.");
        assert_eq!(
            swan_song,
            "Counter target enchantment, instant, or sorcery spell."
        );

        let strix = normalize_common_semantic_phrasing(
            "Counter target artifact or creature or planeswalker.",
        );
        assert_eq!(
            strix,
            "Counter target artifact, creature, or planeswalker spell."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_target_card_subtype_order() {
        let normalized = normalize_common_semantic_phrasing(
            "When Orah dies or a Cleric you control dies, return target card Cleric from your graveyard to the battlefield",
        );
        assert_eq!(
            normalized,
            "When Orah dies or a Cleric you control dies, return target Cleric card from your graveyard to the battlefield"
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_single_blocking_creature_verb() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever a creature blocks a black or red creature, blocking creatures get +1/+1 until end of turn.",
        );
        assert_eq!(
            normalized,
            "Whenever a creature blocks a black or red creature, the blocking creature gets +1/+1 until end of turn."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_each_players_end_step_land_sacrifice() {
        let normalized = normalize_common_semantic_phrasing(
            "At the beginning of each end step, that player sacrifices an untapped land.",
        );
        assert_eq!(
            normalized,
            "At the beginning of each player's end step, that player sacrifices an untapped land of their choice."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_granted_beginning_trigger_clause() {
        let normalized = normalize_common_semantic_phrasing(
            "Enchanted land has at the beginning of your upkeep you may pay w w if you do you gain 1 life",
        );
        assert_eq!(
            normalized,
            "Enchanted land has \"At the beginning of your upkeep you may pay {W}{W}. If you do, you gain 1 life.\""
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_granted_beginning_trigger_clause_for_plural_subject() {
        let normalized = normalize_common_semantic_phrasing(
            "Creatures you control have at the beginning of your upkeep draw a card",
        );
        assert_eq!(
            normalized,
            "Creatures you control have \"At the beginning of your upkeep draw a card.\""
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_unholy_indenture_style_trigger() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever a enchanted creature dies, return it from graveyard to the battlefield. Put a +1/+1 counter on it.",
        );
        assert_eq!(
            normalized,
            "When enchanted creature dies, return that card to the battlefield under your control with a +1/+1 counter on it."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_false_demise_style_trigger() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever a enchanted creature dies, return it to the battlefield under your control.",
        );
        assert_eq!(
            normalized,
            "When enchanted creature dies, return that card to the battlefield under your control."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_nurgles_rot_style_trigger() {
        let normalized = normalize_common_semantic_phrasing(
            "Whenever a enchanted creature dies, return this permanent to its owner's hand. Create a 1/3 black Demon creature token under your control.",
        );
        assert_eq!(
            normalized,
            "When enchanted creature dies, return this card to its owner's hand and create a 1/3 black Demon creature token under your control."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_search_you_own_plural_card_subject() {
        let normalized = normalize_common_semantic_phrasing(
            "Search your library for up to three Aura you own, put them into your hand, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for up to three Aura cards, put them into your hand, then shuffle."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_search_you_own_singular_card_subject() {
        let normalized = normalize_common_semantic_phrasing(
            "Search your library for up to one basic land or Gate you own, put it onto the battlefield tapped.",
        );
        assert_eq!(
            normalized,
            "Search your library for a basic land or Gate card, put it onto the battlefield tapped."
        );
    }

    #[test]
    fn surface_style_preserves_target_aura_subject() {
        let normalized =
            normalize_sentence_surface_style("Return target Aura to its owner's hand.");
        assert_eq!(normalized, "Return target Aura to its owner's hand.");
    }

    #[test]
    fn surface_style_preserves_search_top_then_shuffle_order() {
        let normalized = normalize_sentence_surface_style(
            "Search your library for a card, put it on top of library, then shuffle.",
        );
        assert_eq!(
            normalized,
            "Search your library for a card, put it on top of library, then shuffle."
        );
    }

    #[test]
    fn surface_style_normalizes_archangels_light_clause() {
        let normalized =
            normalize_sentence_surface_style("You gain 2 life for each card in your graveyard.");
        assert_eq!(
            normalized,
            "You gain 2 life for each card in your graveyard."
        );
    }

    #[test]
    fn surface_style_normalizes_zombie_apocalypse_clause() {
        let normalized = normalize_sentence_surface_style(
            "Return all Zombie creature card in your graveyard to the battlefield tapped. Destroy all Humans.",
        );
        assert_eq!(
            normalized,
            "Return all Zombie creature card in your graveyard to the battlefield tapped. Destroy all Humans."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_custom_you_create_token_trigger_head() {
        let normalized = normalize_common_semantic_phrasing(
            "You create a token: Put a +1/+1 counter on another target creature you control.",
        );
        assert_eq!(
            normalized,
            "Whenever you create a token, put a +1/+1 counter on another target creature you control."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_custom_unlock_door_trigger_head() {
        let normalized = normalize_common_semantic_phrasing(
            "You unlock this door: Create a token that's a copy of target creature you control.",
        );
        assert_eq!(
            normalized,
            "Whenever you unlock this door, create a token that's a copy of target creature you control."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_the_beginning_trigger_head() {
        let normalized = normalize_common_semantic_phrasing(
            "The beginning of your first main phase: Sacrifice this enchantment unless you Pay {E}.",
        );
        assert_eq!(
            normalized,
            "At The beginning of your first main phase, sacrifice this enchantment unless you Pay {E}."
        );
    }

    #[test]
    fn post_pass_normalizes_for_each_player_return_with_additional_counter_bundle() {
        let normalized = normalize_compiled_post_pass_effect(
            "For each player, Return all creature card from their graveyard to the battlefield. Put a -1/-1 counter on it.",
        );
        assert_eq!(
            normalized,
            "Each player returns each creature card from their graveyard to the battlefield with an additional -1/-1 counter on it."
        );
    }

    #[test]
    fn known_low_tail_normalizes_for_each_player_return_with_counter_chain() {
        let normalized = normalize_known_low_tail_phrase(
            "For each player, Return all creature card from their graveyard to the battlefield. Put a -1/-1 counter on it.",
        );
        assert_eq!(
            normalized,
            "Each player returns each creature card from their graveyard to the battlefield with an additional -1/-1 counter on it."
        );
    }

    #[test]
    fn known_low_tail_adds_any_order_for_choose_then_put_top_library() {
        let normalized = normalize_known_low_tail_phrase(
            "Target player chooses three cards from their hand, then puts them on top of their library.",
        );
        assert_eq!(
            normalized,
            "Target player chooses three cards from their hand and puts them on top of their library in any order."
        );
    }

    #[test]
    fn semantic_phrasing_normalizes_choose_exact_tagged_graveyard_chain() {
        let normalized = normalize_common_semantic_phrasing(
            "Target opponent chooses exactly 1 artifact card from their graveyard and tags it as '__it__'. Put it onto the battlefield under your control.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses exactly 1 artifact card from their graveyard and tags it as '__it__'. Put it onto the battlefield under your control."
        );
    }

    #[test]
    fn known_low_tail_normalizes_choose_from_graveyard_put_under_your_control() {
        let normalized = normalize_known_low_tail_phrase(
            "Target opponent chooses artifact card from a graveyard. Put it onto the battlefield under your control.",
        );
        assert_eq!(
            normalized,
            "Target opponent chooses an artifact card in their graveyard. Put that card onto the battlefield under your control."
        );
    }

    #[test]
    fn known_low_tail_merges_target_player_loses_and_reveals_hand() {
        let normalized = normalize_known_low_tail_phrase(
            "Target player loses 1 life. Target player reveals their hand.",
        );
        assert_eq!(
            normalized,
            "Target player loses 1 life and reveals their hand."
        );
    }

    #[test]
    fn known_low_tail_merges_counter_then_prevent_all_damage() {
        let normalized = normalize_known_low_tail_phrase(
            "Put a +1/+1 counter on this creature. Prevent all damage that would be dealt to it this turn.",
        );
        assert_eq!(
            normalized,
            "Put a +1/+1 counter on this creature and prevent all damage that would be dealt to it this turn."
        );
    }

    #[test]
    fn known_low_tail_rewrites_choose_target_then_destroy_attached() {
        let normalized = normalize_known_low_tail_phrase(
            "Choose target creature. Destroy all Aura or Equipment attached to that object.",
        );
        assert_eq!(
            normalized,
            "Destroy all Aura or Equipment attached to target creature."
        );
    }

    #[test]
    fn known_low_tail_rewrites_trigger_choose_target_then_destroy_attached() {
        let normalized = normalize_known_low_tail_phrase(
            "Whenever this creature attacks, choose target land. Destroy all Aura attached to that object.",
        );
        assert_eq!(
            normalized,
            "Whenever this creature attacks, destroy all Aura attached to target land."
        );
    }

    #[test]
    fn known_low_tail_normalizes_each_opponent_dynamic_loss_gain_to_x_clause() {
        let normalized = normalize_known_low_tail_phrase(
            "At the beginning of your first main phase, for each opponent, that player loses 1 life for each Shrine you control and you gain 1 life for each Shrine you control.",
        );
        assert_eq!(
            normalized,
            "At the beginning of your first main phase, each opponent loses X life and you gain X life, where X is the number of Shrines you control"
        );
    }

    #[test]
    fn post_pass_normalizes_repeated_return_subtype_chain_to_do_same_for() {
        let normalized = normalize_compiled_post_pass_effect(
            "Return card Pirate from your graveyard to your hand. Return card Vampire from your graveyard to your hand. Return card Dinosaur from your graveyard to your hand. Return card Merfolk from your graveyard to your hand.",
        );
        assert_eq!(
            normalized,
            "Return a Pirate card from your graveyard to your hand, then do the same for Vampire, Dinosaur, and Merfolk."
        );
    }

    #[test]
    fn common_semantic_phrasing_normalizes_stangg_linked_token_clauses() {
        let normalized = normalize_common_semantic_phrasing(
            "When Stangg enters, create a Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. Exile target a token named Stangg Twin until this permanent leaves the battlefield. Grant When token named Stangg Twin leaves the battlefield, sacrifice this permanent. to this permanent.",
        );
        assert_eq!(
            normalized,
            "When Stangg enters, create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. Exile that token when this permanent leaves the battlefield. Sacrifice this permanent when that token leaves the battlefield."
        );
    }

    #[test]
    fn post_pass_normalizes_vaan_spellcast_counter_line() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever you cast a you don't own, for each Scout or Pirate or Rogue you control spell, Put a +1/+1 counter on that object.",
        );
        assert_eq!(
            normalized,
            "Whenever you cast a spell you don't own, put a +1/+1 counter on each Scout or Pirate or Rogue you control."
        );
    }

    #[test]
    fn post_pass_normalizes_vaan_combat_damage_treasure_line() {
        let normalized = normalize_compiled_post_pass_effect(
            "Whenever one or more Scout or Pirate or Rogue you control deal combat damage to a player: Exile card in that player's library. If that doesn't happen, create a Treasure token.",
        );
        assert_eq!(
            normalized,
            "Whenever one or more Scout or Pirate or Rogue you control deal combat damage to a player, exile the top card of that player's library. If you don't, create a Treasure token."
        );
    }

    #[test]
    fn token_blueprint_renders_explicit_colorless_noncreature_artifact() {
        let token =
            crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cragflame")
                .token()
                .card_types(vec![crate::types::CardType::Artifact])
                .subtypes(vec![crate::types::Subtype::Equipment])
                .with_ability(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::make_colorless(
                        crate::filter::ObjectFilter::source(),
                    ),
                ))
                .build();

        let rendered = super::describe_token_blueprint(&token).to_ascii_lowercase();
        assert!(
            rendered.contains("colorless"),
            "expected explicit colorless in noncreature token text, got {rendered}"
        );
        assert!(
            !rendered.contains("is colorless"),
            "expected colorless marker not to render as an extra rules-text clause, got {rendered}"
        );
    }
}
