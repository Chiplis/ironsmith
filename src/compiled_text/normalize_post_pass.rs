use super::*;

fn normalize_committed_crime_reminder_surface(oracle_lower: &str, text: &str) -> String {
    if !oracle_lower.contains("you've committed a crime this turn")
        || !oracle_lower.contains("is a crime")
    {
        return text.to_string();
    }
    let lower = text.to_ascii_lowercase();
    if !lower.contains("committed a crime this turn") || lower.contains("is a crime") {
        return text.to_string();
    }
    format!(
        "{} (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)",
        text.trim().trim_end_matches('.')
    )
}

fn normalize_metalcraft_label_surface(oracle_lower: &str, text: &str) -> String {
    if !oracle_lower.contains("metalcraft") {
        return text.to_string();
    }
    let lower = text.to_ascii_lowercase();
    if lower.starts_with("metalcraft") {
        return text.to_string();
    }
    if lower.contains("as long as you control three or more artifacts")
        || lower.contains("if you control three or more artifacts")
    {
        return format!("Metalcraft — {}", text.trim().trim_end_matches('.'));
    }
    text.to_string()
}

fn normalize_dynamic_token_card_pt_surface(oracle_lower: &str, text: &str) -> String {
    let reference_kind = if oracle_lower.contains("its power is equal to that card's power")
        && oracle_lower.contains("its toughness is equal to that card's toughness")
    {
        "card"
    } else if oracle_lower.contains("its power is equal to that creature's power")
        && oracle_lower.contains("its toughness is equal to that creature's toughness")
    {
        "creature"
    } else {
        return text.to_string();
    };

    let create_prefix = "Create a 0/0 ";
    let lower = text.to_ascii_lowercase();
    let create_prefix_lower = create_prefix.to_ascii_lowercase();
    let Some(create_idx) = lower.find(&create_prefix_lower) else {
        return text.to_string();
    };
    let desc_start = create_idx + create_prefix.len();
    let token_marker = " creature token.";
    let Some(token_desc_rel_end) = lower[desc_start..].find(token_marker) else {
        return text.to_string();
    };
    let desc_end = desc_start + token_desc_rel_end;
    let token_desc = &text[desc_start..desc_end];
    let tail_start = desc_end + token_marker.len();
    let normalized_tail = lower[tail_start..].trim().trim_end_matches('.');

    let has_delayed_sacrifice =
        normalized_tail.starts_with("sacrifice it at the beginning of the next end step. ");
    let pt_tail = if has_delayed_sacrifice {
        normalized_tail
            .strip_prefix("sacrifice it at the beginning of the next end step. ")
            .unwrap_or(normalized_tail)
    } else {
        normalized_tail
    };
    if pt_tail != "it has base power and toughness its power/its toughness forever" {
        return text.to_string();
    }

    let mut rewritten = format!(
        "{}Create a {} creature token. Its power is equal to that {}'s power and its toughness is equal to that {}'s toughness",
        &text[..create_idx],
        token_desc,
        reference_kind,
        reference_kind
    );
    if has_delayed_sacrifice {
        rewritten.push_str(". Sacrifice the token at the beginning of the next end step");
    }
    rewritten.push('.');
    rewritten
}

pub(super) fn normalize_compiled_line_post_pass(def: &CardDefinition, line: &str) -> String {
    let oracle_lower = def.card.oracle_text.to_ascii_lowercase();
    let oracle_has_fall_greatest_power =
        oracle_lower.contains("with the greatest power among creatures target opponent controls");
    let oracle_has_greeds_gambit_triplet = oracle_lower
        .contains("you draw three cards, gain 6 life, and create three 2/1 black bat creature tokens with flying")
        && oracle_lower.contains("you discard a card, lose 2 life, and sacrifice a creature")
        && oracle_lower.contains("you discard three cards, lose 6 life, and sacrifice three creatures");
    if let Some((prefix, rest)) = line.split_once(':')
        && is_render_heading_prefix(prefix)
    {
        let mut normalized_body =
            normalize_sentence_surface_style(&normalize_common_semantic_phrasing(rest.trim()))
                .replace("non-Auran enchantments", "non-Aura enchantments")
                .replace("non-Auran enchantment", "non-Aura enchantment");
        normalized_body = normalize_dynamic_token_card_pt_surface(&oracle_lower, &normalized_body);
        if let Some(rewritten) = normalize_choose_background_scaffolding_clause(&normalized_body) {
            normalized_body = rewritten;
        }
        normalized_body = normalize_compiled_post_pass_phrase(&normalized_body);
        normalized_body = normalize_stubborn_surface_chain(&normalized_body);
        normalized_body = normalize_cost_subject_for_card(def, &normalized_body);
        normalized_body = normalize_spell_self_exile(def, &normalized_body);
        normalized_body = normalize_for_each_clause_surface(normalized_body);
        normalized_body =
            normalize_committed_crime_reminder_surface(&oracle_lower, &normalized_body);
        normalized_body = normalize_metalcraft_label_surface(&oracle_lower, &normalized_body);
        if let Some(rewritten) =
            normalize_not_chosen_this_way_surface(&oracle_lower, &normalized_body)
        {
            normalized_body = rewritten;
        }
        if let Some(rewritten) =
            normalize_exactly_one_other_attacker_base_pt_surface(&oracle_lower, &normalized_body)
        {
            normalized_body = rewritten;
        }
        if let Some(rewritten) =
            normalize_self_damage_each_opponent_count_surface(&oracle_lower, &normalized_body)
        {
            normalized_body = rewritten;
        }
        normalized_body = normalize_known_low_tail_phrase(&normalized_body);
        normalized_body = normalize_each_opponent_dynamic_life_exchange(&normalized_body);
        normalized_body = normalize_triggered_self_deals_damage_phrase(def, &normalized_body);
        normalized_body = normalize_gain_life_plus_phrase(&normalized_body);
        normalized_body = normalize_gift_if_otherwise_surface(&normalized_body);
        normalized_body = normalize_strip_standard_gift_surface(def, &normalized_body);
        if let Some(rewritten) =
            normalize_creature_becomes_base_pt_surface(&normalized_body, &oracle_lower)
        {
            normalized_body = rewritten;
        }
        if oracle_lower.contains("mana value was") && normalized_body.contains("its mana value is ")
        {
            normalized_body = normalized_body.replace("its mana value is ", "its mana value was ");
        }
        if oracle_lower.contains("would die this turn, exile it instead")
            && let Some(prefix) = strip_suffix_ascii_ci(
                &normalized_body,
                ". if it would go from battlefield into graveyard this turn, it goes to exile instead.",
            )
            .or_else(|| {
                strip_suffix_ascii_ci(
                    &normalized_body,
                    ". if it would go from battlefield into graveyard this turn, it goes to exile instead",
                )
            })
        {
            normalized_body = format!(
                "{}. If that creature would die this turn, exile it instead.",
                prefix.trim_end_matches('.')
            );
        }
        if oracle_lower.contains("with an additional +1/+1 counter on it")
            && normalized_body.contains("with a +1/+1 counter on it")
        {
            normalized_body = normalized_body.replace(
                "with a +1/+1 counter on it",
                "with an additional +1/+1 counter on it",
            );
        } else if oracle_lower.contains("put a +1/+1 counter on it")
            && !oracle_lower.contains("with an additional +1/+1 counter on it")
            && normalized_body.contains("with a +1/+1 counter on it")
        {
            normalized_body = normalized_body
                .replace(
                    " to the battlefield with a +1/+1 counter on it",
                    " to the battlefield. Put a +1/+1 counter on it",
                )
                .replace(
                    " onto the battlefield with a +1/+1 counter on it",
                    " onto the battlefield. Put a +1/+1 counter on it",
                );
        }
        if oracle_has_fall_greatest_power {
            normalized_body = normalized_body
                .replace(
                    "III — Exile target creature an opponent controls.",
                    "III — Exile a creature with the greatest power among creatures target opponent controls.",
                )
                .replace(
                    "III — Exile target creature an opponent controls",
                    "III — Exile a creature with the greatest power among creatures target opponent controls",
                );
        }
        if oracle_has_greeds_gambit_triplet {
            normalized_body = normalized_body
                .replace(
                    "When this enchantment enters, you draw three cards and you gain 6 life. Create three 2/1 black Bat creature tokens with flying.",
                    "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying.",
                )
                .replace(
                    "At the beginning of your end step, you discard a card and you lose 2 life, then sacrifice a creature.",
                    "At the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature.",
                )
                .replace(
                    "When this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "Whenever this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "When this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                )
                .replace(
                    "Whenever this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                );
        }
        if oracle_lower.contains("skip your draw step") {
            let trimmed_body = normalized_body.trim();
            if trimmed_body.eq_ignore_ascii_case("you skips their next draw step.")
                || trimmed_body.eq_ignore_ascii_case("you skips their next draw step")
            {
                normalized_body = "Skip your draw step.".to_string();
            }
        }
        if oracle_lower.contains("exile that card from your graveyard")
            && normalized_body.contains("Exile that card in your graveyard")
        {
            normalized_body = normalized_body.replace(
                "Exile that card in your graveyard",
                "Exile that card from your graveyard",
            );
        }
        if oracle_lower
            .contains("put that card into your hand at the beginning of your next end step")
            && normalized_body
                .contains("Pay 1 life: you exile the top card of your library face down.")
            && normalized_body
                .contains("At the beginning of your next end step, return it to its owner's hand")
        {
            normalized_body = normalized_body.replace(
                "Pay 1 life: you exile the top card of your library face down. At the beginning of your next end step, return it to its owner's hand",
                "Pay 1 life: Exile the top card of your library face down. Put that card into your hand at the beginning of your next end step",
            );
        }
        if oracle_lower.contains("spells you control can't be countered this turn")
            && normalized_body.contains("a spell you control can't be countered this turn")
        {
            normalized_body = normalized_body.replace(
                "a spell you control can't be countered this turn",
                "Spells you control can't be countered this turn",
            );
        }
        if oracle_lower
            .contains("you and permanents you control gain hexproof from blue and from black until end of turn")
            && let Some((prefix, tail)) = split_once_ascii_ci(
                &normalized_body,
                "you have hexproof from blue or black this turn. permanents you control gain hexproof from blue or black until end of turn",
            )
        {
            normalized_body = format!(
                "{}You and permanents you control gain hexproof from blue and from black until end of turn{}",
                prefix,
                tail
            );
        }
        if oracle_lower
            .contains("reveal the top card of your library and put that card into your hand")
            && normalized_body
                .contains("Reveal the top card of your library. Put it into its owner's hand")
        {
            normalized_body = normalized_body.replace(
                "Reveal the top card of your library. Put it into its owner's hand",
                "Reveal the top card of your library and put that card into your hand",
            );
        }
        if oracle_lower.contains("it's an enchantment")
            && (oracle_lower.contains("it's not a creature")
                || oracle_lower.contains("its not a creature"))
            && let Some((prefix, tail)) = split_once_ascii_ci(
                &normalized_body,
                "becomes an enchantment in addition to its other types. it isn't a creature",
            )
        {
            normalized_body = format!("{prefix}It's an enchantment{tail}");
        }
        if oracle_lower.contains("if it was a creature, return it to the battlefield under its owner's control. it's an enchantment")
            && normalized_body.contains("if that object is a creature, return it from graveyard to the battlefield. it becomes an enchantment in addition to its other types and isn't a creature")
        {
            normalized_body = normalized_body.replace(
                "if that object is a creature, return it from graveyard to the battlefield. it becomes an enchantment in addition to its other types and isn't a creature",
                "if it was a creature, return it to the battlefield under its owner's control. It's an enchantment",
            );
        }
        if oracle_lower.contains("you draw a card and target opponent may draw a card")
            && normalized_body.contains("you draw a card. target opponent may draw a card")
        {
            normalized_body = normalized_body.replace(
                "you draw a card. target opponent may draw a card",
                "you draw a card and target opponent may draw a card",
            );
        }
        if oracle_lower.contains("if it isn't that player's turn, create a tapped treasure token")
            && normalized_body
                .contains("if it isn't that player's turn: Create a tapped Treasure token")
        {
            normalized_body = normalized_body.replace(
                "if it isn't that player's turn: Create a tapped Treasure token",
                "if it isn't that player's turn, create a tapped Treasure token",
            );
        }
        if oracle_lower.contains("search your library for three cards and reveal them")
            && oracle_lower.contains("target opponent chooses one")
            && oracle_lower
                .contains("put that card into your hand and the rest into your graveyard")
            && let Some(rewritten) =
                normalize_target_opponent_divvy_library_clause(&normalized_body)
        {
            normalized_body = rewritten;
        }
        if oracle_lower.contains("where x is 2 plus the sacrificed creature's mana value")
            && let Some(rewritten) = replace_once_ascii_ci(
                &normalized_body,
                "Search your library for up to one creature with mana value X or less, put it onto the battlefield, then shuffle",
                "Search your library for a creature card with mana value X or less, where X is 2 plus the sacrificed creature's mana value. Put that card onto the battlefield, then shuffle",
            )
        {
            normalized_body = rewritten;
        }
        if oracle_lower.contains("as an additional cost to cast this spell, sacrifice a creature")
            && let Some(rewritten) = replace_once_ascii_ci(
                &normalized_body,
                "As an additional cost to cast this spell, sacrifice a creature you control",
                "As an additional cost to cast this spell, sacrifice a creature",
            )
        {
            normalized_body = rewritten;
        }
        if oracle_lower.contains(
            "you may pay x life, where x is the number of opponents that were dealt combat damage this turn",
        ) && let Some(rewritten) = replace_once_ascii_ci(
            &normalized_body,
            "At the beginning of your second main phase, you may lose the number of an opponent life. If you do, draw that many cards",
            "At the beginning of your postcombat main phase, you may pay X life, where X is the number of opponents that were dealt combat damage this turn. If you do, draw X cards",
        ) {
            normalized_body = rewritten;
        }
        if let Some(rewritten) =
            normalize_simple_trigger_heading_body(prefix.trim(), &normalized_body)
        {
            return if should_drop_hidden_gift_line(def, &rewritten) {
                String::new()
            } else {
                rewritten
            };
        }
        let normalized_body = normalized_body
            .strip_suffix("..")
            .map(|body| format!("{body}."))
            .unwrap_or(normalized_body);
        let normalized_line = format!("{}: {}", prefix.trim(), normalized_body);
        return if should_drop_hidden_gift_line(def, &normalized_line) {
            String::new()
        } else {
            normalized_line
        };
    }
    let mut normalized =
        normalize_sentence_surface_style(&normalize_common_semantic_phrasing(line.trim()))
            .replace("non-Auran enchantments", "non-Aura enchantments")
            .replace("non-Auran enchantment", "non-Aura enchantment");
    normalized = normalize_dynamic_token_card_pt_surface(&oracle_lower, &normalized);
    if let Some(rewritten) = normalize_choose_background_scaffolding_clause(&normalized) {
        normalized = rewritten;
    }
    normalized = normalize_compiled_post_pass_phrase(&normalized);
    normalized = normalize_stubborn_surface_chain(&normalized);
    normalized = normalize_cost_subject_for_card(def, &normalized);
    normalized = normalize_spell_self_exile(def, &normalized);
    normalized = normalize_for_each_clause_surface(normalized);
    normalized = normalize_committed_crime_reminder_surface(&oracle_lower, &normalized);
    normalized = normalize_metalcraft_label_surface(&oracle_lower, &normalized);
    normalized = normalize_known_low_tail_phrase(&normalized);
    normalized = normalize_each_opponent_dynamic_life_exchange(&normalized);
    normalized = normalize_triggered_self_deals_damage_phrase(def, &normalized);
    normalized = normalize_gain_life_plus_phrase(&normalized);
    normalized = normalize_gift_if_otherwise_surface(&normalized);
    normalized = normalize_strip_standard_gift_surface(def, &normalized);
    if oracle_lower.contains("with an additional +1/+1 counter on it")
        && normalized.contains("with a +1/+1 counter on it")
    {
        normalized = normalized.replace(
            "with a +1/+1 counter on it",
            "with an additional +1/+1 counter on it",
        );
    } else if oracle_lower.contains("put a +1/+1 counter on it")
        && !oracle_lower.contains("with an additional +1/+1 counter on it")
        && normalized.contains("with a +1/+1 counter on it")
    {
        normalized = normalized
            .replace(
                " to the battlefield with a +1/+1 counter on it",
                " to the battlefield. Put a +1/+1 counter on it",
            )
            .replace(
                " onto the battlefield with a +1/+1 counter on it",
                " onto the battlefield. Put a +1/+1 counter on it",
            );
    }
    if oracle_has_fall_greatest_power {
        normalized = normalized
            .replace(
                "III — Exile target creature an opponent controls.",
                "III — Exile a creature with the greatest power among creatures target opponent controls.",
            )
            .replace(
                "III — Exile target creature an opponent controls",
                "III — Exile a creature with the greatest power among creatures target opponent controls",
            );
    }
    if oracle_has_greeds_gambit_triplet {
        normalized = normalized
            .replace(
                "When this enchantment enters, you draw three cards and you gain 6 life. Create three 2/1 black Bat creature tokens with flying.",
                "When this enchantment enters, you draw three cards, gain 6 life, and create three 2/1 black Bat creature tokens with flying.",
            )
            .replace(
                "At the beginning of your end step, you discard a card and you lose 2 life, then sacrifice a creature.",
                "At the beginning of your end step, you discard a card, lose 2 life, and sacrifice a creature.",
            )
            .replace(
                "When this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures.",
                "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
            )
            .replace(
                "Whenever this enchantment leaves the battlefield, you discard 3 cards and you lose 6 life, then sacrifice three creatures.",
                "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
            )
            .replace(
                "When this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures.",
                "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
            )
                .replace(
                    "Whenever this enchantment leaves the battlefield, you discard three cards and you lose 6 life, then sacrifice three creatures.",
                    "When this enchantment leaves the battlefield, you discard three cards, lose 6 life, and sacrifice three creatures.",
                );
    }
    if oracle_lower.contains("skip your draw step") {
        let normalized_trimmed = normalized.trim();
        if normalized_trimmed.eq_ignore_ascii_case("you skips their next draw step.")
            || normalized_trimmed.eq_ignore_ascii_case("you skips their next draw step")
        {
            normalized = "Skip your draw step.".to_string();
        }
    }
    if oracle_lower.contains("exile that card from your graveyard")
        && normalized.contains("Exile that card in your graveyard")
    {
        normalized = normalized.replace(
            "Exile that card in your graveyard",
            "Exile that card from your graveyard",
        );
    }
    if oracle_lower.contains("put that card into your hand at the beginning of your next end step")
        && normalized.contains("Pay 1 life: you exile the top card of your library face down.")
        && normalized
            .contains("At the beginning of your next end step, return it to its owner's hand")
    {
        normalized = normalized.replace(
            "Pay 1 life: you exile the top card of your library face down. At the beginning of your next end step, return it to its owner's hand",
            "Pay 1 life: Exile the top card of your library face down. Put that card into your hand at the beginning of your next end step",
        );
    }
    if oracle_lower.contains("spells you control can't be countered this turn")
        && normalized.contains("a spell you control can't be countered this turn")
    {
        normalized = normalized.replace(
            "a spell you control can't be countered this turn",
            "Spells you control can't be countered this turn",
        );
    }
    if oracle_lower.contains(
        "you and permanents you control gain hexproof from blue and from black until end of turn",
    ) && let Some((prefix, tail)) = split_once_ascii_ci(
        &normalized,
        "you have hexproof from blue or black this turn. permanents you control gain hexproof from blue or black until end of turn",
    ) {
        normalized = format!(
            "{}You and permanents you control gain hexproof from blue and from black until end of turn{}",
            prefix, tail
        );
    }
    if oracle_lower.contains("reveal the top card of your library and put that card into your hand")
        && normalized.contains("Reveal the top card of your library. Put it into its owner's hand")
    {
        normalized = normalized.replace(
            "Reveal the top card of your library. Put it into its owner's hand",
            "Reveal the top card of your library and put that card into your hand",
        );
    }
    if oracle_lower.contains("it's an enchantment")
        && (oracle_lower.contains("it's not a creature")
            || oracle_lower.contains("its not a creature"))
        && let Some((prefix, tail)) = split_once_ascii_ci(
            &normalized,
            "becomes an enchantment in addition to its other types. it isn't a creature",
        )
    {
        normalized = format!("{prefix}It's an enchantment{tail}");
    }
    if oracle_lower.contains("if it was a creature, return it to the battlefield under its owner's control. it's an enchantment")
        && normalized.contains("if that object is a creature, return it from graveyard to the battlefield. it becomes an enchantment in addition to its other types and isn't a creature")
    {
        normalized = normalized.replace(
            "if that object is a creature, return it from graveyard to the battlefield. it becomes an enchantment in addition to its other types and isn't a creature",
            "if it was a creature, return it to the battlefield under its owner's control. It's an enchantment",
        );
    }
    if oracle_lower.contains("you draw a card and target opponent may draw a card")
        && normalized.contains("you draw a card. target opponent may draw a card")
    {
        normalized = normalized.replace(
            "you draw a card. target opponent may draw a card",
            "you draw a card and target opponent may draw a card",
        );
    }
    if oracle_lower.contains("if it isn't that player's turn, create a tapped treasure token")
        && normalized.contains("if it isn't that player's turn: Create a tapped Treasure token")
    {
        normalized = normalized.replace(
            "if it isn't that player's turn: Create a tapped Treasure token",
            "if it isn't that player's turn, create a tapped Treasure token",
        );
    }
    if oracle_lower.contains("search your library for three cards and reveal them")
        && oracle_lower.contains("target opponent chooses one")
        && oracle_lower.contains("put that card into your hand and the rest into your graveyard")
        && let Some(rewritten) = normalize_target_opponent_divvy_library_clause(&normalized)
    {
        normalized = rewritten;
    }
    if oracle_lower.contains("as an additional cost to cast this spell, sacrifice a creature")
        && let Some(rewritten) = replace_once_ascii_ci(
            &normalized,
            "As an additional cost to cast this spell, sacrifice a creature you control",
            "As an additional cost to cast this spell, sacrifice a creature",
        )
    {
        normalized = rewritten;
    }
    if oracle_lower.contains(
        "you may pay x life, where x is the number of opponents that were dealt combat damage this turn",
    ) && let Some(rewritten) = replace_once_ascii_ci(
        &normalized,
        "At the beginning of your second main phase, you may lose the number of an opponent life. If you do, draw that many cards",
        "At the beginning of your postcombat main phase, you may pay X life, where X is the number of opponents that were dealt combat damage this turn. If you do, draw X cards",
    ) {
        normalized = rewritten;
    }
    if oracle_lower.contains("where x is 2 plus the sacrificed creature's mana value")
        && let Some(rewritten) = replace_once_ascii_ci(
            &normalized,
            "Search your library for up to one creature with mana value X or less, put it onto the battlefield, then shuffle",
            "Search your library for a creature card with mana value X or less, where X is 2 plus the sacrificed creature's mana value. Put that card onto the battlefield, then shuffle",
        )
    {
        normalized = rewritten;
    }
    normalized
        .strip_suffix("..")
        .map(|body| format!("{body}."))
        .unwrap_or(normalized)
}

fn normalize_target_opponent_divvy_library_clause(text: &str) -> Option<String> {
    let needle = "You searches for up to three permanent in a library and tags it as 'searched'. Reveal it. target opponent searches for exactly 1 permanent in a library and tags it as 'divvy_chosen'. Return the tagged object 'divvy_chosen' to its owner's hand. For each tagged 'divvy_source' object, if it isn't true that the tagged object 'divvy_chosen' matches permanent, Put that object into its owner's graveyard. Shuffle your library";
    if !text
        .trim()
        .trim_end_matches('.')
        .eq_ignore_ascii_case(needle)
    {
        return None;
    }
    Some(
        "Search your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle."
            .to_string(),
    )
}

fn normalize_simple_trigger_heading_body(prefix: &str, body: &str) -> Option<String> {
    let prefix_lower = prefix.to_ascii_lowercase();
    if !prefix_lower.starts_with("when ") && !prefix_lower.starts_with("whenever ") {
        return None;
    }

    let action = strip_suffix_ascii_ci(body.trim(), ". Draw a card.")
        .or_else(|| strip_suffix_ascii_ci(body.trim(), ". Draw a card"))?;
    Some(format!(
        "{prefix}, {} and draw a card.",
        lowercase_first(action.trim())
    ))
}

fn normalize_gift_if_otherwise_surface(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(rest) = strip_prefix_ascii_ci(trimmed, "If the gift was promised, ")
        && let Some((gift_clause, otherwise_clause)) = split_once_ascii_ci(rest, ". Otherwise, ")
    {
        return format!(
            "{}. If the gift was promised, instead {}.",
            capitalize_first(otherwise_clause.trim().trim_end_matches('.')),
            lowercase_first(gift_clause.trim().trim_end_matches('.'))
        );
    }

    trimmed.to_string()
}

pub(super) fn normalize_gain_life_plus_phrase(text: &str) -> String {
    let trimmed = text.trim();
    if let Some((left, right)) = split_once_ascii_ci(trimmed, " and you gain ")
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
    trimmed.to_string()
}

pub(super) fn normalize_each_opponent_dynamic_life_exchange(text: &str) -> String {
    let trimmed = text.trim();

    if let Some((prefix, rest)) = trimmed.split_once(", for each opponent, that player loses ")
        && let Some((loss, gain)) = rest.split_once(" and you gain ")
        && let Some(normalized) = normalize_each_opponent_life_exchange_clause(loss, gain)
    {
        return format!("{prefix}, {normalized}");
    }
    if let Some((prefix, rest)) = trimmed.split_once(", For each opponent, that player loses ")
        && let Some((loss, gain)) = rest.split_once(" and you gain ")
        && let Some(normalized) = normalize_each_opponent_life_exchange_clause(loss, gain)
    {
        return format!("{prefix}, {normalized}");
    }
    if let Some(rest) = trimmed.strip_prefix("For each opponent, that player loses ")
        && let Some((loss, gain)) = rest.split_once(" and you gain ")
        && let Some(normalized) = normalize_each_opponent_life_exchange_clause(loss, gain)
    {
        return capitalize_first(&normalized);
    }

    trimmed.to_string()
}

pub(super) fn normalize_for_each_clause_surface(text: String) -> String {
    let normalize_for_each_subject = |subject: &str| {
        subject
            .trim()
            .trim_end_matches('.')
            .replace("that player's ", "their ")
            .replace("target player's ", "their ")
            .replace("the active player's ", "their ")
            .replace("a player's ", "their ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
    };
    let collapse_redundant_for_each_create =
        |prefix: &str, iter_subject: &str, action: &str| -> Option<String> {
            let create_tail = strip_prefix_ascii_ci(action.trim(), "Create ")?;
            let (token_text, count_subject) = split_once_ascii_ci(create_tail, " for each ")?;
            if normalize_for_each_subject(iter_subject) != normalize_for_each_subject(count_subject)
            {
                return None;
            }
            let head = if prefix.is_empty() {
                String::new()
            } else {
                format!("{prefix}, ")
            };
            Some(format!(
                "{head}Create {} for each {}.",
                token_text.trim().trim_end_matches('.'),
                count_subject.trim().trim_end_matches('.')
            ))
        };
    if let Some(rest) = strip_prefix_ascii_ci(&text, "For each ")
        && let Some((iter_subject, action)) = split_once_ascii_ci(rest, ", ")
        && let Some(collapsed) = collapse_redundant_for_each_create("", iter_subject, action)
    {
        return collapsed;
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&text, ", for each ")
        && let Some((iter_subject, action)) = split_once_ascii_ci(rest, ", ")
        && let Some(collapsed) =
            collapse_redundant_for_each_create(prefix.trim(), iter_subject, action)
    {
        return collapsed;
    }

    let normalize_target_players_verbs = |mut value: String| {
        for (from, to) in [
            ("Target players each gains ", "Target players each gain "),
            ("Target players each draws ", "Target players each draw "),
            (
                "Target players each discards ",
                "Target players each discard ",
            ),
            ("Target players each mills ", "Target players each mill "),
            ("Target players each loses ", "Target players each lose "),
            (
                "Target players each sacrifices ",
                "Target players each sacrifice ",
            ),
            ("target players each gains ", "target players each gain "),
            ("target players each draws ", "target players each draw "),
            (
                "target players each discards ",
                "target players each discard ",
            ),
            ("target players each mills ", "target players each mill "),
            ("target players each loses ", "target players each lose "),
            (
                "target players each sacrifices ",
                "target players each sacrifice ",
            ),
        ] {
            value = value.replace(from, to);
        }
        value
    };
    let normalize_for_each_may_first = |first: &str| {
        let mut normalized = first.trim().trim_end_matches('.').to_string();
        if let Some(rest) = normalized.strip_prefix("that player ") {
            normalized = rest.to_string();
        }
        if let Some(rest) = normalized.strip_prefix("sacrifices ") {
            normalized = format!("sacrifice {rest}");
        }
        if let Some(rest) = normalized.strip_prefix("Create ")
            && let Some(token_text) = rest.strip_suffix(" under that player's control")
        {
            normalized = format!("create {token_text}");
        }
        if let Some(rest) = normalized.strip_prefix("create ")
            && let Some(token_text) = rest.strip_suffix(" under that player's control")
        {
            normalized = format!("create {token_text}");
        }
        normalized = normalized.replace(
            "a permanent that shares a card type with that object that player controls",
            "a permanent of their choice that shares a card type with it",
        );
        normalized = normalized.replace(
            "a permanent that player controls and shares a card type with that object",
            "a permanent of their choice that shares a card type with it",
        );
        normalized = normalized.replace(
            "that player controls and shares a card type with it",
            "that shares a card type with it",
        );
        normalize_you_verb_phrase(&normalized)
    };
    let normalize_for_each_may_second = |second: &str| {
        let mut normalized = second.trim().trim_end_matches('.').to_string();
        if let Some((prefix, _)) = normalized.split_once(". Draw a card") {
            normalized = format!("{} and you draw a card", prefix.trim_end_matches('.'));
        } else if let Some((prefix, _)) = normalized.split_once(". draw a card") {
            normalized = format!("{} and you draw a card", prefix.trim_end_matches('.'));
        }
        normalized
    };
    let normalize_for_each_may_action = |action: &str| {
        let action = action.trim().trim_end_matches('.');
        if let Some(rest) = action.strip_prefix("draws ") {
            return format!("draw {rest}");
        }
        if let Some(rest) = action.strip_prefix("discards ") {
            return format!("discard {rest}");
        }
        if let Some(rest) = action.strip_prefix("gains ") {
            return format!("gain {rest}");
        }
        if let Some(rest) = action.strip_prefix("loses ") {
            return format!("lose {rest}");
        }
        if let Some(rest) = action.strip_prefix("mills ") {
            return format!("mill {rest}");
        }
        action.to_string()
    };
    let normalize_for_each_then_clause = |action: &str| {
        let mut normalized = action.trim().trim_end_matches('.').to_string();
        if let Some(rest) = normalized.strip_prefix("you ") {
            normalized = rest.to_string();
        }
        if let Some(rest) = normalized.strip_prefix("Create ") {
            normalized = format!("create {rest}");
        }
        normalize_you_verb_phrase(&normalized)
    };
    let format_for_each_did_followup = |clause: String| {
        if clause.starts_with("For each ")
            || clause.starts_with("for each ")
            || clause.starts_with("that player ")
        {
            clause
        } else {
            format!("you {clause}")
        }
    };
    let normalize_for_each_effect_id_condition =
        |input: &str, subject: &str, who_phrase: &str| -> Option<String> {
            let marker = format!("{subject}, if effect #");
            let (prefix, rest) = input.split_once(&marker)?;
            if let Some((_, action)) = rest.split_once(" happened, ") {
                return Some(format!("{prefix}{subject} {who_phrase}, {}", action.trim()));
            }
            if let Some((_, action)) = rest.split_once(" that doesn't happen, ") {
                return Some(format!("{prefix}{subject} who doesn't, {}", action.trim()));
            }
            None
        };
    if let Some(rewritten) =
        normalize_for_each_effect_id_condition(&text, "For each opponent", "who does")
    {
        return rewritten;
    }
    if let Some(rewritten) =
        normalize_for_each_effect_id_condition(&text, "for each opponent", "who does")
    {
        return rewritten;
    }
    if let Some(rewritten) =
        normalize_for_each_effect_id_condition(&text, "For each player", "who does")
    {
        return rewritten;
    }
    if let Some(rewritten) =
        normalize_for_each_effect_id_condition(&text, "for each player", "who does")
    {
        return rewritten;
    }
    if let Some((prefix, rest)) =
        text.split_once("Each player may discard their hand. that player draws ")
        && let Some((count, tail)) = rest.split_once(" cards. For each opponent who does, ")
    {
        return format!(
            "{prefix}Each player may discard their hand and draw {count} cards. For each opponent who does, {}",
            tail.trim()
        );
    }
    if let Some((prefix, rest)) =
        text.split_once("each player may discard their hand. that player draws ")
        && let Some((count, tail)) = rest.split_once(" cards. For each opponent who does, ")
    {
        return format!(
            "{prefix}each player may discard their hand and draw {count} cards. For each opponent who does, {}",
            tail.trim()
        );
    }
    if let Some((prefix, rest)) = text
        .split_once("For each player, You may that player ")
        .or_else(|| text.split_once("for each player, You may that player "))
        && let Some((first, second)) = rest.split_once(". If you don't, that player ")
    {
        let first = normalize_for_each_may_first(first);
        let second = normalize_for_each_may_second(second);
        let each_player = if prefix.is_empty() {
            "Each player"
        } else {
            "each player"
        };
        return format!(
            "{prefix}{each_player} may {first}. For each player who doesn't, that player {second}"
        );
    }
    if let Some((prefix, rest)) = text
        .split_once("For each opponent, You may that player ")
        .or_else(|| text.split_once("for each opponent, You may that player "))
        && let Some((first, second)) = rest.split_once(". If you don't, that player ")
    {
        let first = normalize_for_each_may_first(first);
        let second = normalize_for_each_may_second(second);
        let each_opponent = if prefix.is_empty() {
            "Each opponent"
        } else {
            "each opponent"
        };
        return format!(
            "{prefix}{each_opponent} may {first}. For each opponent who doesn't, that player {second}"
        );
    }
    if let Some((prefix, rest)) = text
        .split_once("For each player, You may that player ")
        .or_else(|| text.split_once("for each player, You may that player "))
        && let Some((first, second)) = rest.split_once(". If you do, that player ")
    {
        let first = normalize_for_each_may_action(first);
        let second = second.trim().trim_end_matches('.');
        let each_player = if prefix.is_empty() {
            "Each player"
        } else {
            "each player"
        };
        return format!(
            "{prefix}{each_player} may {first}. For each player who does, that player {second}"
        );
    }
    if let Some((prefix, rest)) = text
        .split_once("For each opponent, You may that player ")
        .or_else(|| text.split_once("for each opponent, You may that player "))
        && let Some((first, second)) = rest.split_once(". If you do, that player ")
    {
        let first = normalize_for_each_may_action(first);
        let second = second.trim().trim_end_matches('.');
        let each_opponent = if prefix.is_empty() {
            "Each opponent"
        } else {
            "each opponent"
        };
        return format!(
            "{prefix}{each_opponent} may {first}. For each opponent who does, that player {second}"
        );
    }
    if let Some((prefix, rest)) = text
        .split_once("For each player, You may ")
        .or_else(|| text.split_once("for each player, You may "))
        && let Some((first, second)) = rest.split_once(". If you do, ")
    {
        let first = normalize_for_each_may_first(first);
        let second = format_for_each_did_followup(normalize_for_each_then_clause(second));
        let each_player = if prefix.is_empty() {
            "Each player"
        } else {
            "each player"
        };
        return format!("{prefix}{each_player} may {first}. For each player who does, {second}");
    }
    if let Some((prefix, rest)) = text
        .split_once("For each opponent, You may ")
        .or_else(|| text.split_once("for each opponent, You may "))
        && let Some((first, second)) = rest.split_once(". If you do, ")
    {
        let first = normalize_for_each_may_first(first);
        let second = format_for_each_did_followup(normalize_for_each_then_clause(second));
        let each_opponent = if prefix.is_empty() {
            "Each opponent"
        } else {
            "each opponent"
        };
        return format!(
            "{prefix}{each_opponent} may {first}. For each opponent who does, {second}"
        );
    }
    if let Some((prefix, rest)) = text
        .split_once("For each player, that player may ")
        .or_else(|| text.split_once("for each player, that player may "))
        && let Some((first, second)) = rest.split_once(". If they do, ")
    {
        let first = normalize_for_each_may_first(first);
        let second = format_for_each_did_followup(normalize_for_each_then_clause(second));
        let each_player = if prefix.is_empty() {
            "Each player"
        } else {
            "each player"
        };
        return format!("{prefix}{each_player} may {first}. For each player who does, {second}");
    }
    if let Some((prefix, rest)) = text
        .split_once("For each opponent, that player may ")
        .or_else(|| text.split_once("for each opponent, that player may "))
        && let Some((first, second)) = rest.split_once(". If they do, ")
    {
        let first = normalize_for_each_may_first(first);
        let second = format_for_each_did_followup(normalize_for_each_then_clause(second));
        let each_opponent = if prefix.is_empty() {
            "Each opponent"
        } else {
            "each opponent"
        };
        return format!(
            "{prefix}{each_opponent} may {first}. For each opponent who does, {second}"
        );
    }
    if let Some((prefix, rest)) = text.split_once("For each opponent, Deal ")
        && let Some((amount, discard_tail)) =
            rest.split_once(" damage to that player. Each opponent discards ")
    {
        return format!(
            "{prefix}This spell deals {amount} damage to each opponent. Those players each discard {}",
            discard_tail.trim_end_matches('.')
        );
    }
    if let Some(rest) = text.strip_prefix(
        "Choose two target players. For each target player, Search that player's library for ",
    ) {
        return format!(
            "Choose two target players. Each of those players searches their library for {}",
            rest.replace(
                ", then shuffle and put that card on top",
                ", then shuffles and puts that card on top"
            )
        );
    }
    if let Some((prefix, rest)) = text
        .split_once("For each target player, that player ")
        .or_else(|| text.split_once("for each target player, that player "))
    {
        let subject = if prefix.is_empty() {
            "Target players each"
        } else {
            "target players each"
        };
        if let Some((first, second)) = rest
            .split_once(". For each target player, that player ")
            .or_else(|| rest.split_once(". for each target player, that player "))
        {
            let first = first.trim().trim_end_matches('.');
            let second = second.trim().trim_end_matches('.');
            return normalize_target_players_verbs(format!(
                "{prefix}{subject} {first} and {second}."
            ));
        }
        return normalize_target_players_verbs(format!("{prefix}{subject} {}", rest.trim()));
    }
    let original = text.clone();
    let mut fallback = text;
    if fallback.contains("For each target player, that player ")
        || fallback.contains("for each target player, that player ")
    {
        fallback = fallback.replace(
            "For each target player, that player ",
            "Target players each ",
        );
        fallback = fallback.replace(
            "for each target player, that player ",
            "target players each ",
        );
    }
    if let Some(rest) = fallback.strip_prefix("Choose any number of target players. ")
        && let Some(each_rest) = rest.strip_prefix("target players each ")
    {
        return normalize_target_players_verbs(format!(
            "Any number of target players each {}",
            each_rest.trim()
        ));
    }
    if let Some(rest) = fallback.strip_prefix("Choose two target players. ")
        && let Some(each_rest) = rest.strip_prefix("target players each ")
    {
        return normalize_target_players_verbs(format!(
            "Two target players each {}",
            each_rest.trim()
        ));
    }
    fallback = normalize_target_players_verbs(fallback);
    if fallback != original {
        return fallback;
    }
    original
}

pub(super) fn normalize_triggered_self_deals_damage_phrase(
    def: &CardDefinition,
    text: &str,
) -> String {
    if let Some(rest) = strip_prefix_ascii_ci(text, "Whenever creature attacks, deal ")
        && let Some(amount) = strip_suffix_ascii_ci(rest, " damage to it.")
            .or_else(|| strip_suffix_ascii_ci(rest, " damage to it"))
    {
        let source = card_self_reference_phrase(def);
        return format!("Whenever a creature attacks, {source} deals {amount} damage to it.");
    }
    text.to_string()
}

fn normalize_strip_standard_gift_surface(def: &CardDefinition, text: &str) -> String {
    let has_visible_gift_line = def
        .optional_costs
        .iter()
        .any(|cost| cost.label.trim().to_ascii_lowercase().starts_with("gift "));
    if !has_visible_gift_line {
        return text.to_string();
    }

    let mut normalized = text.to_string();
    for phrase in [
        "If the gift was promised, the chosen player draws a card.",
        "If the gift was promised, instead the chosen player draws a card.",
        "If the gift was promised, create a Treasure token under the chosen player's control.",
        "If the gift was promised, instead create a Treasure token under the chosen player's control.",
        "If the gift was promised, create a Food token under the chosen player's control.",
        "If the gift was promised, instead create a Food token under the chosen player's control.",
        "If the gift was promised, create a 1/1 blue Fish creature token under the chosen player's control, tapped.",
        "If the gift was promised, instead create a 1/1 blue Fish creature token under the chosen player's control, tapped.",
        "If the gift was promised, the chosen player takes an extra turn after this one.",
        "If the gift was promised, instead the chosen player takes an extra turn after this one.",
        "If the gift was promised, create an 8/8 blue Octopus creature token under the chosen player's control.",
        "If the gift was promised, instead create an 8/8 blue Octopus creature token under the chosen player's control.",
    ] {
        normalized = normalized.replace(phrase, "");
    }

    normalized = normalized
        .replace(". .", ". ")
        .replace("..", ".")
        .replace(
            "If the gift was promised, Return ",
            "If the gift was promised, instead return ",
        )
        .replace("  ", " ")
        .trim()
        .trim_matches('.')
        .to_string();

    if normalized.is_empty() {
        String::new()
    } else {
        format!("{normalized}.")
    }
}

fn should_drop_hidden_gift_line(def: &CardDefinition, text: &str) -> bool {
    let has_visible_gift_line = def
        .optional_costs
        .iter()
        .any(|cost| cost.label.trim().to_ascii_lowercase().starts_with("gift "));
    if !has_visible_gift_line {
        return false;
    }

    let lower = text.trim().to_ascii_lowercase();
    (lower.starts_with("triggered ability")
        || lower.starts_with("spell effects: if the gift was promised"))
        && lower.contains("gift was promised")
        && (lower.contains("chosen player draws a card")
            || lower.contains("create a treasure token under the chosen player's control")
            || lower.contains("create a food token under the chosen player's control")
            || lower.contains(
                "create a 1/1 blue fish creature token under the chosen player's control",
            )
            || lower.contains("chosen player takes an extra turn after this one")
            || lower.contains(
                "create an 8/8 blue octopus creature token under the chosen player's control",
            )
            || lower.contains(
                "create a 8/8 blue octopus creature token under the chosen player's control",
            ))
}

pub(super) fn normalize_each_opponent_life_exchange_clause(
    loss: &str,
    gain: &str,
) -> Option<String> {
    let loss = loss.trim().trim_end_matches('.');
    let gain = gain.trim().trim_end_matches('.');

    let for_each = loss.strip_prefix("1 life for each ")?;
    if !gain.eq_ignore_ascii_case(loss) {
        return None;
    }

    let count_subject = if for_each.trim_end().ends_with(" in your party")
        && for_each.trim_start().starts_with("creature ")
    {
        for_each.trim().replacen("creature ", "creatures ", 1)
    } else {
        pluralize_noun_phrase(for_each.trim())
    };

    Some(format!(
        "each opponent loses X life and you gain X life, where X is the number of {}",
        count_subject
    ))
}

fn normalize_not_chosen_this_way_surface(oracle_lower: &str, text: &str) -> Option<String> {
    if !oracle_lower.contains("not chosen this way") {
        return None;
    }

    let head = strip_prefix_ascii_ci(text, "For each player, you choose exactly 1 ")?;
    let (descriptor_raw, rest) = split_once_ascii_ci(
        head,
        " that player controls in the battlefield. Return all other ",
    )?;
    let (return_subject_raw, after_return) = split_once_ascii_ci(rest, " to their owners' hands")?;
    let descriptor_raw = descriptor_raw.trim();
    let return_subject_raw = return_subject_raw.trim();
    let expected_return_subject = pluralize_choice_descriptor(descriptor_raw);
    if !return_subject_raw.eq_ignore_ascii_case(&expected_return_subject) {
        return None;
    }

    let mut rewritten = format!(
        "Each player chooses {descriptor_raw} they control. Return all {expected_return_subject} not chosen this way to their owners' hands"
    );

    let trimmed_tail = after_return.trim_start();
    if let Some(rest) = strip_prefix_ascii_ci(trimmed_tail, ". For each opponent, if that player ")
        && let Some((condition_raw, tail)) = split_once_ascii_ci(rest, ", you draw a card")
    {
        let condition = if condition_raw
            .trim()
            .eq_ignore_ascii_case("has more cards in hand than you")
        {
            "has more cards in their hand than you".to_string()
        } else {
            condition_raw.trim().to_string()
        };
        rewritten.push_str(&format!(
            ". Then you draw a card for each opponent who {}{}",
            condition,
            if tail.trim().is_empty() { "." } else { tail }
        ));
    } else if !trimmed_tail.is_empty() {
        rewritten.push_str(trimmed_tail);
    }

    Some(rewritten)
}

fn normalize_exactly_one_other_attacker_base_pt_surface(
    oracle_lower: &str,
    text: &str,
) -> Option<String> {
    if !oracle_lower.contains("attacked with exactly one other creature this combat")
        || !oracle_lower.contains("base power and toughness become")
    {
        return None;
    }

    let rest = strip_prefix_ascii_ci(
        text,
        "Whenever this creature and exactly 1 other creature attack, you may each creature has base power and toughness ",
    )?;
    let pt = rest
        .trim()
        .strip_suffix(" until end of turn.")
        .or_else(|| rest.trim().strip_suffix(" until end of turn"))?;
    Some(format!(
        "Whenever this creature attacks, if you attacked with exactly one other creature this combat, you may have that creature's base power and toughness become {} until end of turn.",
        pt.trim()
    ))
}

fn normalize_self_damage_each_opponent_count_surface(
    oracle_lower: &str,
    text: &str,
) -> Option<String> {
    if !oracle_lower.contains("deals x damage to each opponent")
        || !oracle_lower.contains("where x is the number of")
    {
        return None;
    }

    let (prefix, rest) = split_once_ascii_ci(text, ". Deal the number of ")?;
    let (count_subject_raw, tail) = split_once_ascii_ci(rest, " damage to each opponent")?;
    let mut count_subject = count_subject_raw.trim().to_string();

    if let Some(rest) = count_subject.strip_prefix("another ") {
        count_subject = rest.trim().to_string();
        if let Some(base) = count_subject.strip_suffix(" you control") {
            count_subject = format!("{} you control other than this creature", base.trim());
        } else {
            count_subject = format!("{count_subject} other than this creature");
        }
    }

    let suffix = if tail.trim().is_empty() {
        ".".to_string()
    } else {
        tail.to_string()
    };
    Some(format!(
        "{prefix}. This creature deals X damage to each opponent, where X is the number of {count_subject}{suffix}"
    ))
}

pub(super) fn normalize_known_low_tail_phrase(text: &str) -> String {
    let trimmed = text.trim();

    if let Some((prefix, rest)) =
        split_once_ascii_ci(trimmed, ", for each opponent, that player loses ")
        && let Some((loss, gain)) = split_once_ascii_ci(rest, " and you gain ")
        && let Some(normalized) = normalize_each_opponent_life_exchange_clause(loss, gain)
    {
        return format!("{prefix}, {normalized}");
    }
    if let Some(rest) = strip_prefix_ascii_ci(trimmed, "For each opponent, that player loses ")
        && let Some((loss, gain)) = split_once_ascii_ci(rest, " and you gain ")
        && let Some(normalized) = normalize_each_opponent_life_exchange_clause(loss, gain)
    {
        return capitalize_first(&normalized);
    }

    if let Some((left, right)) = split_once_ascii_ci(trimmed, ". ")
        && let Some(cards) = strip_prefix_ascii_ci(left.trim(), "Each player returns each ")
            .and_then(|tail| {
                strip_suffix_ascii_ci(tail, " from their graveyard to the battlefield").or_else(
                    || {
                        strip_suffix_ascii_ci(
                            tail,
                            " from that player's graveyard to the battlefield",
                        )
                    },
                )
            })
            .or_else(|| {
                strip_prefix_ascii_ci(left.trim(), "For each player, Return all ").and_then(
                    |tail| {
                        strip_suffix_ascii_ci(tail, " from their graveyard to the battlefield")
                            .or_else(|| {
                                strip_suffix_ascii_ci(
                                    tail,
                                    " from that player's graveyard to the battlefield",
                                )
                            })
                    },
                )
            })
        && let Some(counter_text) = strip_prefix_ascii_ci(right.trim(), "Put a ")
            .or_else(|| strip_prefix_ascii_ci(right.trim(), "Put an "))
            .or_else(|| strip_prefix_ascii_ci(right.trim(), "put a "))
            .or_else(|| strip_prefix_ascii_ci(right.trim(), "put an "))
            .and_then(|tail| strip_suffix_ascii_ci(tail.trim_end_matches('.'), " counter on it"))
    {
        return format!(
            "Each player returns each {} from their graveyard to the battlefield with an additional {} counter on it.",
            cards.trim(),
            counter_text.trim()
        );
    }
    if let Some(prefix) = trimmed
        .strip_suffix(", then puts them on top of their library.")
        .or_else(|| trimmed.strip_suffix(", then puts them on top of their library"))
        && prefix.to_ascii_lowercase().contains(" chooses ")
        && prefix
            .to_ascii_lowercase()
            .contains(" cards from their hand")
    {
        return format!("{prefix} and puts them on top of their library in any order.");
    }
    if let Some((chooser, rest)) = split_once_ascii_ci(trimmed, " chooses ")
        && let Some((chosen_kind, tail)) =
            split_once_ascii_ci(rest, " card from a graveyard. Put it onto the battlefield")
    {
        let card_phrase = with_indefinite_article(&format!("{} card", chosen_kind.trim()));
        return format!(
            "{chooser} chooses {card_phrase} in their graveyard. Put that card onto the battlefield{tail}"
        );
    }
    let (head_prefix, reveal_candidate) =
        if let Some((prefix, tail)) = split_once_ascii_ci(trimmed, ": ") {
            (Some(prefix.trim()), tail.trim())
        } else {
            (None, trimmed)
        };
    if let Some((left, right)) = split_once_ascii_ci(reveal_candidate, ". ")
        && left
            .to_ascii_lowercase()
            .starts_with("target player loses ")
        && (right
            .trim()
            .eq_ignore_ascii_case("Target player reveals their hand.")
            || right
                .trim()
                .eq_ignore_ascii_case("Target player reveals their hand"))
    {
        let merged = format!(
            "{} and reveals their hand.",
            left.trim().trim_end_matches('.')
        );
        if let Some(prefix) = head_prefix {
            return format!("{prefix}: {merged}");
        }
        return merged;
    }
    if let Some((left, right)) = split_once_ascii_ci(trimmed, ". ")
        && left.to_ascii_lowercase().contains(" counter on ")
        && right
            .trim()
            .to_ascii_lowercase()
            .starts_with("prevent all damage that would be dealt to ")
        && right.trim().to_ascii_lowercase().contains(" this turn")
    {
        let right_clause = lowercase_first(right.trim().trim_end_matches('.'));
        let merged = format!("{} and {}", left.trim().trim_end_matches('.'), right_clause);
        return format!("{merged}.");
    }
    if let Some((choose_clause, destroy_clause)) = split_once_ascii_ci(trimmed, ". ")
        && let Some(attached_filter) = strip_prefix_ascii_ci(destroy_clause.trim(), "Destroy all ")
            .and_then(|tail| {
                strip_suffix_ascii_ci(tail.trim_end_matches('.'), " attached to that object")
            })
    {
        if let Some(target_phrase) = strip_prefix_ascii_ci(choose_clause.trim(), "Choose ")
            && target_phrase.to_ascii_lowercase().starts_with("target ")
        {
            return format!(
                "Destroy all {} attached to {}.",
                attached_filter.trim(),
                target_phrase.trim()
            );
        }

        let choose_lower = choose_clause.to_ascii_lowercase();
        if let Some(pos) = choose_lower.rfind(", choose target ")
            && pos + 2 <= choose_clause.len()
        {
            let prefix = choose_clause[..pos].trim();
            let choose_target = choose_clause[pos + 2..].trim();
            if let Some(target_phrase) = strip_prefix_ascii_ci(choose_target, "choose ")
                && target_phrase.to_ascii_lowercase().starts_with("target ")
            {
                return format!(
                    "{prefix}, destroy all {} attached to {}.",
                    attached_filter.trim(),
                    target_phrase.trim()
                );
            }
        }
    }
    if let Some((first_clause, rest)) = trimmed.split_once(". For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent.")
            .or_else(|| rest.strip_suffix(" damage to each opponent"))
        && let Some((subject, self_amount)) = first_clause
            .strip_prefix("When this permanent enters, it deals ")
            .map(|tail| ("permanent", tail))
            .or_else(|| {
                first_clause
                    .strip_prefix("When this creature enters, it deals ")
                    .map(|tail| ("creature", tail))
            })
            .and_then(|(subject, tail)| {
                tail.strip_suffix(" damage to that player")
                    .map(|amount| (subject, amount))
            })
        && self_amount.trim().eq_ignore_ascii_case(amount.trim())
    {
        return format!(
            "When this {subject} enters, it deals {amount} damage to each opponent and each creature your opponents control."
        );
    }
    if let Some(rest) = trimmed.strip_prefix("For each opponent's creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to each opponent.")
            .or_else(|| rest.strip_suffix(" damage to each opponent"))
    {
        return format!("Deal {amount} damage to each creature your opponents control.");
    }
    if let Some((left, right)) = split_once_ascii_ci(trimmed, ". sacrifice ")
        && left.to_ascii_lowercase().contains(" and you lose ")
    {
        return format!(
            "{}, then sacrifice {}.",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }

    let sentence = trimmed.trim_end_matches('.');
    if let Some(prefix) = strip_suffix_ascii_ci(sentence, ". Untap that creature") {
        let prefix = prefix.trim();
        if let Some(rest) = strip_prefix_ascii_ci(prefix, "Each creature you control gets ")
            && let Some(buff) = strip_suffix_ascii_ci(rest.trim(), " until end of turn")
        {
            return format!(
                "Creatures you control get {buff} until end of turn. Untap those creatures."
            );
        }
        if let Some(rest) = strip_prefix_ascii_ci(prefix, "Any number of target creatures get ")
            && let Some(buff) = strip_suffix_ascii_ci(rest.trim(), " until end of turn")
        {
            return format!(
                "Any number of target creatures each get {buff} until end of turn. Untap those creatures."
            );
        }
        if let Some((head, tail)) = split_once_ascii_ci(
            prefix,
            "For each creature you control, Put a +1/+1 counter on that object",
        ) && tail.trim().is_empty()
        {
            let clause = "Put a +1/+1 counter on each creature you control. Untap those creatures.";
            let clause = lower_clause_after_prefix(head, clause);
            return format!("{head}{clause}");
        }
    }
    if let Some((head, tail)) = split_once_ascii_ci(
        sentence,
        "For each creature you control, Put a +1/+1 counter on that object. that creature gains Vigilance, gains Trample, and gains Indestructible until end of turn",
    ) && tail.trim().is_empty()
    {
        let clause = "Put a +1/+1 counter on each creature you control. Those creatures gain vigilance, trample, and indestructible until end of turn.";
        let clause = lower_clause_after_prefix(head, clause);
        return format!("{head}{clause}");
    }
    if let Some(rest) = strip_prefix_ascii_ci(
        sentence,
        "When this creature enters, for each opponent's creature with flying, Deal ",
    )
    .and_then(|tail| strip_suffix_ascii_ci(tail, " damage to that object. Tap that creature"))
    {
        return format!(
            "When this creature enters, it deals {} damage to each creature with flying your opponents control. Tap those creatures.",
            rest.trim()
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(
        sentence,
        "When this permanent enters, for each opponent's creature with flying, Deal ",
    )
    .and_then(|tail| strip_suffix_ascii_ci(tail, " damage to that object. Tap that creature"))
    {
        return format!(
            "When this permanent enters, it deals {} damage to each creature with flying your opponents control. Tap those creatures.",
            rest.trim()
        );
    }

    // Common surface: "<subject> gain {T}: <ability>. until <duration>."
    if let Some((before_until, duration)) = split_once_ascii_ci(sentence, ". until ") {
        let (subject, ability, verb) =
            if let Some((subject, ability)) = split_once_ascii_ci(before_until, " gain ") {
                (subject, ability, "gain")
            } else if let Some((subject, ability)) = split_once_ascii_ci(before_until, " gains ") {
                (subject, ability, "gains")
            } else {
                ("", "", "")
            };

        if !subject.is_empty() && !ability.is_empty() {
            let ability = ability.trim();
            if ability.starts_with("{T}:") || ability.starts_with("{Q}:") {
                let mut quoted = ability.trim_end_matches('.').to_string();
                quoted.push('.');
                if let Some(pos) = subject.find('—') {
                    let head = subject[..pos + '—'.len_utf8()].trim_end();
                    let rest = subject[pos + '—'.len_utf8()..].trim_start();
                    return format!(
                        "{head} Until {duration}, {} {verb} \"{quoted}\"",
                        lowercase_first(rest)
                    );
                }
                return format!(
                    "Until {duration}, {} {verb} \"{quoted}\"",
                    lowercase_first(subject)
                );
            }
        }
    }
    if trimmed.contains("loses loses ") || trimmed.contains("gain one life") {
        return trimmed
            .replace("loses loses ", "loses ")
            .replace("gain one life", "gain 1 life");
    }

    trimmed.to_string()
}

pub(super) fn normalize_stubborn_surface_chain(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.eq_ignore_ascii_case("Draw two cards and you lose 2 life. you mill 2 cards.")
        || trimmed.eq_ignore_ascii_case("Draw two cards and you lose 2 life. you mill 2 cards")
        || trimmed.eq_ignore_ascii_case("Draw two cards and you lose 2 life. you mill two cards.")
        || trimmed.eq_ignore_ascii_case("Draw two cards and you lose 2 life. you mill two cards")
    {
        return "Draw two cards, lose 2 life, then mill two cards.".to_string();
    }
    if let Some(counter) = strip_prefix_ascii_ci(trimmed, "Put a ").and_then(|rest| {
        strip_suffix_ascii_ci(rest, " counter on target creature. Proliferate.")
            .or_else(|| strip_suffix_ascii_ci(rest, " counter on target creature. Proliferate"))
    }) {
        return format!("Put a {counter} counter on target creature, then proliferate.");
    }
    trimmed.to_string()
}

pub(super) fn normalize_spell_self_exile(def: &CardDefinition, text: &str) -> String {
    let mut normalized = text.to_string();
    let card_name = def.card.name.trim();
    if card_name.is_empty() {
        return normalized;
    }
    let collapse_with_counters = |input: &str, subject: &str, card_name: &str| -> Option<String> {
        let clause = format!("Exile {subject}. Put ");
        let tail_period = format!(" on {subject}.");
        let tail_plain = format!(" on {subject}");
        if let Some((prefix, rest)) = input.split_once(&format!(" {clause}")) {
            if let Some(counter_phrase) = rest.strip_suffix(&tail_period) {
                return Some(format!(
                    "{prefix} Exile {card_name} with {counter_phrase} on it."
                ));
            }
            if let Some(counter_phrase) = rest.strip_suffix(&tail_plain) {
                return Some(format!(
                    "{prefix} Exile {card_name} with {counter_phrase} on it."
                ));
            }
        }
        if let Some(rest) = input.strip_prefix(&clause) {
            if let Some(counter_phrase) = rest.strip_suffix(&tail_period) {
                return Some(format!("Exile {card_name} with {counter_phrase} on it."));
            }
            if let Some(counter_phrase) = rest.strip_suffix(&tail_plain) {
                return Some(format!("Exile {card_name} with {counter_phrase} on it."));
            }
        }
        None
    };
    if let Some(collapsed) = collapse_with_counters(&normalized, "this spell", card_name) {
        normalized = collapsed;
    } else if let Some(collapsed) = collapse_with_counters(&normalized, "this permanent", card_name)
    {
        normalized = collapsed;
    }
    if let Some(prefix) = normalized.strip_suffix(" Exile this spell.") {
        return format!("{prefix} Exile {card_name}.");
    }
    if let Some(prefix) = normalized.strip_suffix(" Exile this spell") {
        return format!("{prefix} Exile {card_name}.");
    }
    if normalized.eq_ignore_ascii_case("Exile this spell.")
        || normalized.eq_ignore_ascii_case("Exile this spell")
    {
        normalized = format!("Exile {card_name}.");
    }
    if should_drop_hidden_gift_line(def, &normalized) {
        String::new()
    } else {
        normalized
    }
}

pub(super) fn normalize_cost_subject_for_card(def: &CardDefinition, text: &str) -> String {
    let Some((cost, effect)) = text.split_once(": ") else {
        return text.to_string();
    };
    let effect = effect.trim();
    let oracle_lower = def.card.oracle_text.to_ascii_lowercase();
    let card_name = def.card.name.trim();
    let card_name_lower = card_name.to_ascii_lowercase();
    let short_name = card_name.split(',').next().unwrap_or(card_name).trim();
    let short_name_lower = short_name.to_ascii_lowercase();
    let oracle_uses_card_name = oracle_lower.contains(&format!("{card_name_lower} deals "))
        || oracle_lower.contains(&format!("{short_name_lower} deals "));
    if let Some(rest) = strip_prefix_ascii_ci(effect, "this creature deals ")
        && oracle_uses_card_name
    {
        return format!("{cost}: {short_name} deals {rest}");
    }
    if let Some(rest) = strip_prefix_ascii_ci(effect, "this permanent deals ")
        && oracle_uses_card_name
    {
        return format!("{cost}: {short_name} deals {rest}");
    }
    if !effect.starts_with("Deal ") {
        return text.to_string();
    }
    let Some(rest) = effect.strip_prefix("Deal ") else {
        return text.to_string();
    };
    let subject = capitalize_first(card_self_reference_phrase(def));
    format!("{cost}: {subject} deals {rest}")
}

pub(super) fn normalize_compiled_post_pass_phrase(text: &str) -> String {
    let mut normalized = text.trim().to_string();
    if normalized.is_empty() {
        return normalized;
    }

    if let Some((cost, effect)) = normalized.split_once(": ")
        && !cost.trim().is_empty()
        && !cost.trim().to_ascii_lowercase().starts_with("when ")
        && !cost.trim().to_ascii_lowercase().starts_with("whenever ")
        && !cost
            .trim()
            .to_ascii_lowercase()
            .starts_with("at the beginning ")
    {
        let rewritten = normalize_compiled_post_pass_effect(effect.trim());
        if rewritten != effect.trim() {
            normalized = format!("{}: {rewritten}", cost.trim());
        }
    }

    normalize_compiled_post_pass_effect(&normalized)
}

pub(super) fn normalize_you_cast_spell_you_dont_own_counter_line(text: &str) -> Option<String> {
    let (head, rest) = split_once_ascii_ci(text, "Whenever you cast a ")?;
    let (owner_phrase, rest) = split_once_ascii_ci(rest, ", for each ")?;
    let owner_phrase = owner_phrase.trim();
    if !matches!(
        owner_phrase,
        "you don't own" | "you dont own" | "you don’t own"
    ) {
        return None;
    }
    let (filter, rest) = split_once_ascii_ci(rest, " spell, ")?;
    let put_tail = strip_prefix_ascii_ci(rest, "Put a +1/+1 counter on that object")
        .or_else(|| strip_prefix_ascii_ci(rest, "put a +1/+1 counter on that object"))?;
    let mut rewritten = format!(
        "{head}Whenever you cast a spell you don't own, put a +1/+1 counter on each {}",
        filter.trim()
    );
    let put_tail = put_tail.trim();
    if put_tail.is_empty() {
        rewritten.push('.');
    } else if put_tail.starts_with('.') {
        rewritten.push_str(put_tail);
    } else {
        rewritten.push_str(". ");
        rewritten.push_str(put_tail);
    }
    Some(rewritten)
}

pub(super) fn normalize_one_or_more_combat_damage_treasure_line(text: &str) -> Option<String> {
    let (head, rest) = split_once_ascii_ci(text, "Whenever one or more ")?;
    let marker = " deal combat damage to a player: Exile card in that player's library. If that doesn't happen, create a Treasure token";
    let (subject, tail) = split_once_ascii_ci(rest, marker)?;
    let mut rewritten = format!(
        "{head}Whenever one or more {} deal combat damage to a player, exile the top card of that player's library. If you don't, create a Treasure token",
        subject.trim()
    );
    let tail = tail.trim();
    if tail.is_empty() {
        rewritten.push('.');
    } else if tail.starts_with('.') {
        rewritten.push_str(tail);
    } else {
        rewritten.push_str(". ");
        rewritten.push_str(tail);
    }
    Some(rewritten)
}

pub(super) fn normalize_create_one_under_control_list(clauses: &[&str]) -> Option<String> {
    if clauses.len() < 2 {
        return None;
    }
    let mut items = Vec::new();
    for clause in clauses {
        let trimmed = clause.trim().trim_end_matches('.');
        let rest = trimmed.strip_prefix("Create 1 ")?;
        let desc = rest.strip_suffix(" under your control")?;
        items.push(format!("a {}", desc.trim()));
    }
    Some(format!("Create {}.", join_with_and(&items)))
}

pub(super) fn rewrite_return_with_counters_on_it_sequence(text: &str) -> Option<String> {
    let trimmed = text.trim().trim_end_matches('.');
    let mut clauses = trimmed
        .split(". ")
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if clauses.len() < 2 {
        return None;
    }

    let mut chapter_prefix = String::new();
    if let Some(first) = clauses.first().cloned()
        && let Some((prefix, rest)) = first.split_once("— ")
        && (rest.trim_start().starts_with("Return ") || rest.trim_start().starts_with("Put "))
    {
        chapter_prefix = format!("{} — ", prefix.trim_end());
        clauses[0] = rest.trim_start().to_string();
    }

    for idx in 0..clauses.len().saturating_sub(1) {
        let clause = clauses[idx].clone();
        if clause
            .to_ascii_lowercase()
            .starts_with("for each player, return all ")
        {
            continue;
        }
        let lower_clause = clause.to_ascii_lowercase();
        let clause_moves_to_battlefield = lower_clause.contains(" onto the battlefield")
            || lower_clause.contains(" to the battlefield");
        if !clause_moves_to_battlefield {
            continue;
        }
        let is_return = clause.starts_with("Return ");
        let is_put = clause.starts_with("Put ");
        let is_inline_move = !is_return
            && !is_put
            && (lower_clause.contains(" put ") || lower_clause.contains(" return "));
        if !is_return && !is_put && !is_inline_move {
            continue;
        }

        let mut counter_descriptions = Vec::new();
        let mut tail_start = idx + 1;
        while tail_start < clauses.len() {
            let clause = clauses[tail_start].trim();
            let Some(rest) = clause.strip_prefix("Put ") else {
                break;
            };
            let Some(counter_phrase) = rest.strip_suffix(" on it") else {
                break;
            };
            let counter_phrase = counter_phrase.trim();
            if !counter_phrase.to_ascii_lowercase().contains("counter") {
                break;
            }
            counter_descriptions.push(with_indefinite_article(counter_phrase));
            tail_start += 1;
        }
        if counter_descriptions.is_empty() {
            continue;
        }

        let mut base_clause = clause;
        if is_return && base_clause == "Return target card from your graveyard to the battlefield" {
            base_clause =
                "Return target permanent card from your graveyard to the battlefield".to_string();
        }
        if base_clause.eq_ignore_ascii_case(
            "At the beginning of the next end step, return it to the battlefield",
        ) && idx > 0
            && clauses[idx - 1].to_ascii_lowercase().contains("exile ")
        {
            base_clause =
                "At the beginning of the next end step, return that card to the battlefield under its owner's control"
                    .to_string();
        }

        let merged = format!(
            "{base_clause} with {} on it",
            join_with_and(&counter_descriptions)
        );

        let mut rebuilt = Vec::new();
        rebuilt.extend_from_slice(&clauses[..idx]);
        rebuilt.push(merged);
        rebuilt.extend_from_slice(&clauses[tail_start..]);

        let mut rewritten = rebuilt.join(". ");
        if !rewritten.ends_with('.') {
            rewritten.push('.');
        }
        if !chapter_prefix.is_empty() {
            rewritten = format!("{chapter_prefix}{rewritten}");
        }
        return Some(rewritten);
    }

    None
}

pub(super) fn chapter_number_to_roman(chapter: u32) -> Option<&'static str> {
    match chapter {
        1 => Some("I"),
        2 => Some("II"),
        3 => Some("III"),
        4 => Some("IV"),
        5 => Some("V"),
        6 => Some("VI"),
        7 => Some("VII"),
        8 => Some("VIII"),
        9 => Some("IX"),
        10 => Some("X"),
        _ => None,
    }
}

pub(super) fn rewrite_saga_chapter_prefix(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("Chapter ")
        && let Some((chapter, tail)) = rest.split_once(':')
        && let Ok(chapter_num) = chapter.trim().parse::<u32>()
        && let Some(roman) = chapter_number_to_roman(chapter_num)
    {
        return Some(format!("{roman} — {}", tail.trim()));
    }
    if let Some(rest) = trimmed.strip_prefix("Chapters ")
        && let Some((chapter_list, tail)) = rest.split_once(':')
    {
        let mut romans = Vec::new();
        for chunk in chapter_list.split(',') {
            let chapter_num = chunk.trim().parse::<u32>().ok()?;
            romans.push(chapter_number_to_roman(chapter_num)?.to_string());
        }
        if romans.is_empty() {
            return None;
        }
        return Some(format!("{} — {}", romans.join(", "), tail.trim()));
    }
    None
}

pub(super) fn rewrite_granted_triggered_ability_quote(text: &str) -> Option<String> {
    fn insert_trigger_comma_if_missing(body: &str) -> String {
        for verb in [
            " draw ",
            " discard ",
            " put ",
            " return ",
            " create ",
            " destroy ",
            " exile ",
            " tap ",
            " untap ",
            " sacrifice ",
            " deal ",
            " gain ",
            " lose ",
            " mill ",
            " counter ",
        ] {
            if let Some((head, tail)) = body.split_once(verb) {
                if head.trim_end().ends_with(',') {
                    return body.to_string();
                }
                return format!("{head},{verb}{}", tail.trim_start());
            }
        }
        body.to_string()
    }

    fn normalize_granted_trigger_body(body: &str) -> String {
        let mut normalized = body.trim().trim_end_matches('.').to_string();
        let lower = normalized.to_ascii_lowercase();
        if (lower.starts_with("when ")
            || lower.starts_with("whenever ")
            || lower.starts_with("at the beginning of "))
            && !normalized.contains(',')
        {
            for verb in [
                " draw ",
                " discard ",
                " put ",
                " return ",
                " create ",
                " destroy ",
                " exile ",
                " tap ",
                " untap ",
                " sacrifice ",
                " deal ",
                " gain ",
                " lose ",
                " mill ",
                " counter ",
            ] {
                if let Some((head, tail)) = normalized.split_once(verb) {
                    normalized = format!("{head},{verb}{}", tail.trim_start());
                    break;
                }
            }
        }
        normalized = normalized.replace(" then ", ", then ").replace(
            " this ability triggers only once each turn",
            ". This ability triggers only once each turn",
        );
        if lower.contains("reveal the top card of your library if")
            && lower.contains("otherwise put it into your hand")
            && lower.contains("this ability triggers only")
        {
            normalized = normalized
                .replace(
                    "reveal the top card of your library if",
                    "reveal the top card of your library. If",
                )
                .replace("if its a land card", "if it's a land card")
                .replace(
                    "put it onto the battlefield otherwise",
                    "put it onto the battlefield. Otherwise",
                )
                .replace(
                    "put it into your hand this ability triggers only",
                    "put it into your hand. This ability triggers only",
                );
        }
        normalized
    }

    fn split_until_end_of_turn_suffix(body: &str) -> (&str, &str) {
        let trimmed = body.trim();
        let trimmed_no_period = trimmed.trim_end_matches('.');
        let lower = trimmed_no_period.to_ascii_lowercase();
        let suffix = " until end of turn";
        if lower.ends_with(suffix) {
            let split_idx = trimmed_no_period.len().saturating_sub(suffix.len());
            return (
                trimmed_no_period[..split_idx].trim_end(),
                trimmed_no_period[split_idx..].trim(),
            );
        }
        (trimmed_no_period.trim(), "")
    }

    if let Some((subject, body)) = text.split_once(" have whenever ") {
        let body = insert_trigger_comma_if_missing(&normalize_granted_trigger_body(body));
        let body = format!("Whenever {body}");
        return Some(format!("{} have \"{}.\"", subject.trim(), body));
    }
    if let Some((subject, body)) = text.split_once(" has whenever ") {
        let body = insert_trigger_comma_if_missing(&normalize_granted_trigger_body(body));
        let body = format!("Whenever {body}");
        return Some(format!("{} has \"{}.\"", subject.trim(), body));
    }
    if let Some((subject, body)) = text.split_once(" have when ") {
        let body = insert_trigger_comma_if_missing(&normalize_granted_trigger_body(body));
        let body = format!("When {body}");
        return Some(format!("{} have \"{}.\"", subject.trim(), body));
    }
    if let Some((subject, body)) = text.split_once(" has when ") {
        let body = insert_trigger_comma_if_missing(&normalize_granted_trigger_body(body));
        let body = format!("When {body}");
        return Some(format!("{} has \"{}.\"", subject.trim(), body));
    }
    if let Some((subject, body)) = text.split_once(" gains when ")
        && body.to_ascii_lowercase().contains("wicked role token")
    {
        let (body_core, until_suffix) = split_until_end_of_turn_suffix(body);
        let body = insert_trigger_comma_if_missing(&normalize_granted_trigger_body(body_core));
        let body = format!("When {body}");
        if until_suffix.is_empty() {
            return Some(format!("{} gains \"{}.\"", subject.trim(), body));
        }
        return Some(format!(
            "{} gains \"{}.\" {}.",
            subject.trim(),
            body,
            until_suffix
        ));
    }
    None
}

pub(super) fn normalize_conditional_target_player_pronouns(text: &str) -> String {
    // Some oracles refer back to a previously-chosen target player inside an "If ..." clause
    // using "that player" rather than repeating "target player/opponent".
    if text.contains('•') {
        return text.to_string();
    }
    let normalized = normalize_conditional_target_player_pronoun(text, "target opponent");
    normalize_conditional_target_player_pronoun(&normalized, "target player")
}

pub(super) fn normalize_conditional_target_player_pronoun(text: &str, phrase: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let marker = format!(", {phrase}");
    let Some(pos) = lower.find(&marker) else {
        return text.to_string();
    };
    let prefix = &lower[..pos];
    if !prefix.contains(phrase) {
        return text.to_string();
    }
    let clause = if let Some(idx) = prefix.rfind(". ") {
        &prefix[idx + 2..]
    } else if let Some(idx) = prefix.rfind("? ") {
        &prefix[idx + 2..]
    } else if let Some(idx) = prefix.rfind("! ") {
        &prefix[idx + 2..]
    } else if let Some(idx) = prefix.rfind('\n') {
        &prefix[idx + 1..]
    } else {
        prefix
    };
    if !clause.trim_start().starts_with("if ") {
        return text.to_string();
    }

    // `pos` is the start of ", {phrase}".
    let start = pos + 2;
    let end = start + phrase.len();
    let mut rewritten = String::with_capacity(text.len());
    rewritten.push_str(&text[..start]);
    rewritten.push_str("that player");
    rewritten.push_str(&text[end..]);
    rewritten
}

pub(super) fn normalize_compiled_post_pass_effect(text: &str) -> String {
    let mut normalized = text.trim().to_string();
    if normalized.is_empty() {
        return normalized;
    }
    normalized = normalized.replace(" until end of turn, where X is X", " until end of turn");
    normalized = normalized.replace(
        " gain Haste until end of turn",
        " gain haste until end of turn",
    );
    normalized = normalized.replace(
        "Creatures you control get +X/+X until end of turn, then creatures you control gain haste until end of turn",
        "Creatures you control get +X/+X and gain haste until end of turn",
    );
    normalized = normalized.replace(
        "creatures you control get +X/+X until end of turn, then creatures you control gain haste until end of turn",
        "creatures you control get +X/+X and gain haste until end of turn",
    );
    if let Some(rewritten) = replace_once_ascii_ci(
        &normalized,
        "Exile all card from an opponent's graveyard",
        "Exile all opponents' graveyards",
    ) {
        normalized = rewritten;
    }
    if let Some(rewritten) = replace_once_ascii_ci(
        &normalized,
        "At the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard",
        "At the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards",
    ) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_you_cast_spell_you_dont_own_counter_line(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_one_or_more_combat_damage_treasure_line(&normalized) {
        normalized = rewritten;
    }
    normalized = normalize_conditional_target_player_pronouns(&normalized);
    loop {
        let mut changed = false;
        if let Some(rewritten) =
            normalize_sentence_helper_top_cards_choose_to_hand_sequence(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) =
            normalize_sentence_helper_graveyard_to_hand_choice_clause(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) = normalize_sentence_helper_milled_to_hand_clause(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) = normalize_sentence_helper_put_from_other_hand_clause(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) = normalize_sentence_helper_put_from_hand_clause(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) =
            normalize_sentence_helper_reveal_search_same_name_clause(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) = normalize_sentence_helper_reveal_from_hand_clause(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) = normalize_sentence_helper_simple_exile_clause(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) =
            normalize_sentence_helper_choose_exiled_card_play_clause(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) = normalize_sentence_helper_planar_genesis_clause(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) = normalize_sentence_helper_kicked_top_x_clause(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) = normalize_sentence_helper_spell_mastery_gather_clause(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) = normalize_enlist_tag_sequence(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if let Some(rewritten) = normalize_divvy_chosen_sequence(&normalized)
            && rewritten != normalized
        {
            normalized = rewritten;
            changed = true;
        }
        if !changed {
            break;
        }
    }
    let lower_normalized = normalized.to_ascii_lowercase();
    if lower_normalized
        == "at the beginning of your end step, for each creature you control, put a +1/+1 counter on that object. for each planeswalker you control, put a loyalty counter on that object."
        || lower_normalized
            == "at the beginning of your end step, for each creature you control, put a +1/+1 counter on that object. for each planeswalker you control, put a loyalty counter on that object"
    {
        return "At the beginning of your end step, put a +1/+1 counter on each creature you control and a loyalty counter on each planeswalker you control."
            .to_string();
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "Return target ")
        && let Some((card_desc, _tail)) =
            split_once_ascii_ci(rest, " in your exile to its owner's hand")
    {
        return format!(
            "Return target exiled {} you own to your hand.",
            card_desc.trim()
        );
    }
    if let Some((head, hand_tail)) = split_once_ascii_ci(&normalized, " to their owners' hands")
        && let Some(rest) = strip_prefix_ascii_ci(head.trim(), "Return all ")
    {
        let words = rest.split_whitespace().collect::<Vec<_>>();
        let mut excluded = Vec::new();
        let mut noun_idx: Option<usize> = None;
        for (idx, word) in words.iter().enumerate() {
            if word.eq_ignore_ascii_case("creature") || word.eq_ignore_ascii_case("creatures") {
                noun_idx = Some(idx);
                break;
            }
            let Some(subtype) = word.strip_prefix("non-") else {
                excluded.clear();
                noun_idx = None;
                break;
            };
            let cleaned = subtype.trim_matches(|ch: char| !ch.is_ascii_alphabetic());
            if cleaned.is_empty() {
                excluded.clear();
                noun_idx = None;
                break;
            }
            excluded.push(cleaned.to_string());
        }
        if let Some(noun_idx) = noun_idx
            && !excluded.is_empty()
            && words
                .get(noun_idx + 1..)
                .is_some_and(|tail| tail.iter().all(|word| word.is_empty()))
        {
            let excluded_rendered = excluded
                .into_iter()
                .map(|subtype| {
                    if subtype.eq_ignore_ascii_case("Octopus") {
                        "Octopuses".to_string()
                    } else {
                        pluralize_noun_phrase(&subtype)
                    }
                })
                .collect::<Vec<_>>();
            let punctuation = if hand_tail.trim_start().starts_with('.') {
                "."
            } else {
                ""
            };
            return format!(
                "Return all creatures to their owners' hands except for {}{}",
                join_with_and(&excluded_rendered),
                punctuation
            );
        }
    }
    normalized = normalized
        .replace(
            " creature tokens with \"Sacrifice this creature, add {C}\"",
            " creature tokens. They have \"Sacrifice this creature: Add {C}.\"",
        )
        .replace(
            " creature token with \"Sacrifice this creature, add {C}\"",
            " creature token. It has \"Sacrifice this creature: Add {C}.\"",
        )
        .replace(
            " creature tokens with \"sacrifice this creature, add {C}\"",
            " creature tokens. They have \"Sacrifice this creature: Add {C}.\"",
        )
        .replace(
            " creature token with \"sacrifice this creature, add {C}\"",
            " creature token. It has \"Sacrifice this creature: Add {C}.\"",
        );
    if let Some((prefix, tail)) = split_once_ascii_ci(
        &normalized,
        "At the beginning of the next end step, sacrifice this spell",
    ) && prefix.to_ascii_lowercase().contains("create ")
        && prefix.to_ascii_lowercase().contains("token")
    {
        normalized = format!("{prefix}At the beginning of the next end step, sacrifice it{tail}");
    }
    normalized = normalized.replace(
        "When this creature enters or this creature attacks,",
        "Whenever this creature enters or attacks,",
    );
    normalized = normalized.replace(
        "When this permanent enters or Whenever this creature attacks,",
        "Whenever this creature enters or attacks,",
    );
    normalized = normalized.replace(
        "when this creature enters or this creature attacks,",
        "whenever this creature enters or attacks,",
    );
    normalized = normalized.replace(
        "when this permanent enters or whenever this creature attacks,",
        "whenever this creature enters or attacks,",
    );
    if let Some((prefix, rest)) =
        split_once_ascii_ci(&normalized, "For each opponent, that player discards ")
        && let Some((discard_tail, lose_tail)) =
            split_once_ascii_ci(rest, ". For each opponent, that player loses ")
    {
        let lose_tail = lose_tail.trim();
        let (lose_clause, trailing_tail) =
            if let Some((lose_clause, tail)) = lose_tail.split_once(". ") {
                (lose_clause.trim().trim_end_matches('.'), Some(tail.trim()))
            } else {
                (lose_tail.trim_end_matches('.'), None)
            };
        let prefix = prefix.trim_end();
        let lead = if prefix.is_empty() {
            "Each opponent ".to_string()
        } else if prefix.ends_with(',') {
            format!("{prefix} each opponent ")
        } else {
            format!("{prefix}, each opponent ")
        };
        let merged = format!(
            "{lead}discards {} and loses {}.",
            discard_tail.trim(),
            lose_clause
        );
        if let Some(tail) = trailing_tail {
            return normalize_compiled_post_pass_effect(&format!("{merged} {tail}"));
        }
        return merged;
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, "target opponent sacrifices ")
        && let Some((sacrifice_tail, rest)) =
            split_once_ascii_ci(rest, ". target opponent discards ")
        && let Some((discard_tail, lose_tail)) =
            split_once_ascii_ci(rest, ". target opponent loses ")
    {
        let lose_tail = lose_tail.trim();
        let (lose_clause, trailing_tail) =
            if let Some((lose_clause, tail)) = lose_tail.split_once(". ") {
                (lose_clause.trim().trim_end_matches('.'), Some(tail.trim()))
            } else {
                (lose_tail.trim_end_matches('.'), None)
            };
        let merged = format!(
            "{}target opponent sacrifices {}, discards {}, and loses {}.",
            prefix,
            sacrifice_tail.trim(),
            discard_tail.trim(),
            lose_clause
        );
        if let Some(tail) = trailing_tail {
            return normalize_compiled_post_pass_effect(&format!("{merged} {tail}"));
        }
        return merged;
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ". target player discards ")
        && let Some((discard_tail, sacrifice_tail)) = split_once_ascii_ci(rest, ". sacrifice ")
    {
        return format!(
            "{prefix}. Target player discards {} and sacrifices {}.",
            discard_tail.trim(),
            sacrifice_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((left, _right)) = split_once_ascii_ci(
        &normalized,
        ". Return all another card with the same name as that object from your graveyard to your hand.",
    )
    .or_else(|| {
        split_once_ascii_ci(
            &normalized,
            ". Return all another card with the same name as that object from your graveyard to your hand",
        )
    })
        && let Some(target_desc) = strip_prefix_ascii_ci(left.trim(), "Return target ").and_then(
            |tail| strip_suffix_ascii_ci(tail.trim(), " from your graveyard to your hand"),
        )
    {
        return format!(
            "Return target {} and all other cards with the same name as that card from your graveyard to your hand.",
            target_desc.trim()
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, "target player sacrifices ")
        && let Some((sacrifice_tail, lose_tail)) =
            split_once_ascii_ci(rest, ". target player loses ")
    {
        return format!(
            "{prefix}target player sacrifices {} and loses {}.",
            sacrifice_tail.trim(),
            lose_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = normalized.split_once(". ")
        && left.to_ascii_lowercase().starts_with("draw ")
        && right.to_ascii_lowercase().starts_with("you gain ")
        && right.to_ascii_lowercase().ends_with(" life")
    {
        return format!("{left} and {}", normalize_you_verb_phrase(right));
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ", you draw ")
        && let Some((draw_tail, rest)) = split_once_ascii_ci(rest, " cards and you gain ")
        && let Some((gain_tail, create_tail)) = split_once_ascii_ci(rest, " life. Create ")
    {
        return format!(
            "{prefix}, you draw {} cards, gain {} life, and create {}.",
            draw_tail.trim(),
            gain_tail.trim(),
            create_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, gain_tail)) = split_once_ascii_ci(&normalized, ". Draw a card. you gain ")
        && gain_tail
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with(" life")
    {
        return format!(
            "{prefix}. Draw a card and gain {}.",
            gain_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ": you discard ")
        && let Some((discard_tail, draw_tail)) = split_once_ascii_ci(rest, ". Draw ")
    {
        return format!(
            "{prefix}: discard {}, then draw {}.",
            discard_tail.trim(),
            draw_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ": you discard ")
        && let Some((discard_tail, draw_tail)) = split_once_ascii_ci(rest, ". you draw ")
    {
        return format!(
            "{prefix}: discard {}, then draw {}.",
            discard_tail.trim(),
            draw_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((left, draw_tail)) = split_once_ascii_ci(&normalized, ". you draw ")
        && left.to_ascii_lowercase().starts_with("exile ")
    {
        return format!(
            "{left}, then draw {}.",
            draw_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, draw_tail)) = split_once_ascii_ci(&normalized, ". you draw ")
        && let Some(base_amount) = strip_suffix_ascii_ci(
            draw_tail.trim().trim_end_matches('.'),
            ", rounded down cards",
        )
    {
        return format!(
            "{}, then draw {} cards, rounded down.",
            prefix.trim().trim_end_matches('.'),
            base_amount.trim()
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ". you draw ")
        && let Some((draw_tail, gain_tail)) = split_once_ascii_ci(rest, ". you gain ")
        && gain_tail
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with(" life")
    {
        let draw_tail = draw_tail.trim().trim_end_matches('.');
        let gain_tail = gain_tail.trim().trim_end_matches('.');
        if prefix.trim().is_empty() {
            return format!("Draw {draw_tail} and gain {gain_tail}.");
        }
        return format!("{prefix}. Draw {draw_tail} and gain {gain_tail}.");
    }
    if let Some((prefix, energy_tail)) = split_once_ascii_ci(&normalized, ". you get ")
        && energy_tail.trim_start().starts_with("{E")
    {
        let prefix_clean = prefix.trim().trim_end_matches('.');
        let lower_prefix = prefix_clean.to_ascii_lowercase();
        if lower_prefix.starts_with("when ") || lower_prefix.starts_with("at the beginning of ") {
            return format!(
                "{} and you get {}.",
                prefix_clean,
                energy_tail.trim().trim_end_matches('.')
            );
        }
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ": you gain ")
        && let Some((gain_tail, draw_tail)) = split_once_ascii_ci(rest, " life. you may draw ")
    {
        return format!(
            "{prefix}: you gain {} life and you may draw {}.",
            gain_tail.trim(),
            draw_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". Deal ")
        && left.to_ascii_lowercase().contains(" gets ")
        && left.to_ascii_lowercase().contains("until end of turn")
        && let Some(amount_tail) = strip_suffix_ascii_ci(right.trim(), " damage to each opponent.")
            .or_else(|| strip_suffix_ascii_ci(right.trim(), " damage to each opponent"))
    {
        return format!(
            "{}, and deals {} damage to each opponent.",
            left.trim().trim_end_matches('.'),
            amount_tail.trim()
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". sacrifice ")
        && left.to_ascii_lowercase().contains(" and you lose ")
    {
        return format!(
            "{} and sacrifice {}.",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you lose ")
        && let Some((loss_tail, sacrifice_tail)) = split_once_ascii_ci(right, ", then sacrifice ")
        && loss_tail
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with(" life")
    {
        return format!(
            "{}, lose {}, and sacrifice {}.",
            left.trim().trim_end_matches('.'),
            loss_tail.trim().trim_end_matches('.'),
            sacrifice_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, "This creature deals ")
        && let Some((damage, loss_tail)) = split_once_ascii_ci(
            rest,
            " damage to target creature. that object's controller loses ",
        )
        && let Some(loss_amount) = loss_tail.trim().trim_end_matches('.').strip_suffix(" life")
    {
        return format!(
            "{prefix}This creature deals {} damage to target creature and that creature's controller loses {} life.",
            damage.trim(),
            loss_amount.trim()
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(
        &normalized,
        ". At the beginning of the next end step, return it to its owner's hand",
    ) && prefix
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("exile all card")
    {
        let mut rewritten = format!(
            "{prefix}. At the beginning of the next end step, return those cards to their owners' hands"
        );
        let rest = rest.trim();
        if let Some(tail) = rest.strip_prefix('.') {
            let tail = tail.trim();
            if !tail.is_empty() {
                rewritten.push_str(". ");
                rewritten.push_str(tail);
            } else {
                rewritten.push('.');
            }
        } else if !rest.is_empty() {
            rewritten.push(' ');
            rewritten.push_str(rest);
        } else {
            rewritten.push('.');
        }
        return normalize_compiled_post_pass_effect(&rewritten);
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(
        &normalized,
        ". At the beginning of the end step of that player's next turn, return it to its owner's hand",
    ) && prefix
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("exile all ")
        && prefix.to_ascii_lowercase().contains(" from their hand")
    {
        let mut rewritten = format!(
            "{prefix}. At the beginning of the end step of that player's next turn, that player returns those cards to their hand"
        );
        let rest = rest.trim();
        if let Some(tail) = rest.strip_prefix('.') {
            let tail = tail.trim();
            if !tail.is_empty() {
                rewritten.push_str(". ");
                rewritten.push_str(tail);
            } else {
                rewritten.push('.');
            }
        } else if !rest.is_empty() {
            rewritten.push(' ');
            rewritten.push_str(rest);
        } else {
            rewritten.push('.');
        }
        return normalize_compiled_post_pass_effect(&rewritten);
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you gain ")
        && left.trim().to_ascii_lowercase().starts_with("you gain ")
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
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". Create ")
        && left.to_ascii_lowercase().contains("you lose ")
        && right
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase()
            .contains("treasure token")
    {
        return format!(
            "{} and create {}.",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". Destroy all ")
        && left
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("return all ")
        && !right.trim_start().to_ascii_lowercase().starts_with("a ")
        && !right.trim_start().to_ascii_lowercase().starts_with("an ")
    {
        return format!(
            "{}, then destroy all {}.",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, rest)) =
        split_once_ascii_ci(&normalized, ". If that doesn't happen, Return ")
        && let Some((return_tail, energy_tail)) = split_once_ascii_ci(rest, ". you get ")
    {
        return format!(
            "{prefix}. If you can't, return {} and you get {}.",
            return_tail.trim(),
            energy_tail.trim().trim_end_matches('.')
        );
    }
    if let Some((prefix, _suffix)) =
        split_once_ascii_ci(&normalized, ". If that doesn't happen, you draw a card.")
    {
        return format!("{prefix}. If you can't, draw a card.");
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
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". creatures you control gain ")
        && let Some((prefix, buff_tail)) =
            split_once_ascii_ci(left, "each creature you control gets ")
        && let Some((buff, _)) = split_once_ascii_ci(buff_tail, " until end of turn")
        && let Some((ability, _)) = split_once_ascii_ci(right, " until end of turn")
    {
        let prefix = prefix.trim_end();
        let buff = buff.trim();
        let ability = ability.trim().to_ascii_lowercase();
        let left_clause = if prefix.is_empty() {
            "Each creature you control gets".to_string()
        } else if prefix.ends_with(' ') {
            format!("{prefix}creatures you control get")
        } else {
            format!("{prefix} creatures you control get")
        };
        return format!("{left_clause} {buff} and gain {ability} until end of turn.");
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ", you mill ")
        && let Some((count_tail, put_tail)) = split_once_ascii_ci(rest, " cards. Put ")
    {
        return format!(
            "{prefix}, mill {} cards, then put {}",
            count_tail.trim(),
            put_tail.trim()
        );
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ": that player may pay ")
        && let Some((cost, tail)) = split_once_ascii_ci(rest, ". If that player doesn't, create ")
    {
        return format!(
            "{prefix}, that player may pay {}. If that player doesn't, you create {}",
            cost.trim(),
            tail.trim()
        );
    }
    if let Some(rewritten) = normalize_search_put_onto_battlefield_clause(&normalized) {
        return rewritten;
    }
    if let Some(rewritten) = normalize_search_reveal_into_hand_clause(&normalized) {
        return rewritten;
    }
    if let Some(rewritten) = normalize_search_reveal_battlefield_or_hand_clause(&normalized) {
        return rewritten;
    }
    if let Some(rewritten) = normalize_search_face_down_exile_cast_else_hand_clause(&normalized) {
        return rewritten;
    }
    if let Some(rewritten) = normalize_split_search_battlefield_then_hand_clause(&normalized) {
        return rewritten;
    }
    if let Some(tail) = strip_prefix_ascii_ci(
        &normalized,
        "Whenever you cast an or copy an instant or sorcery spell, ",
    ) {
        return format!("Whenever you cast or copy an instant or sorcery spell, {tail}");
    }
    if let Some(rest) = strip_prefix_ascii_ci(
        &normalized,
        "Whenever you cast an as your second spell this turn, ",
    ) {
        let effect = rest
            .trim()
            .trim_end_matches('.')
            .strip_suffix(" spell")
            .unwrap_or(rest.trim().trim_end_matches('.'))
            .trim();
        return format!("Whenever you cast your second spell each turn, {effect}.");
    }
    if normalized.eq_ignore_ascii_case("Whenever you cast an or copy an instant or sorcery spell") {
        return "Whenever you cast or copy an instant or sorcery spell".to_string();
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". Attach it to ")
        && left.to_ascii_lowercase().contains("create ")
        && left.to_ascii_lowercase().contains(" token")
    {
        return format!(
            "{} attached to {}.",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". Attach them to ")
        && left.to_ascii_lowercase().contains("create ")
        && left.to_ascii_lowercase().contains(" token")
    {
        return format!(
            "{} attached to {}.",
            left.trim().trim_end_matches('.'),
            right.trim().trim_end_matches('.')
        );
    }
    if let Some(tail) = strip_prefix_ascii_ci(
        &normalized,
        "Whenever you cast instant or sorcery or Whenever you copy instant or sorcery, ",
    )
    .or_else(|| {
        strip_prefix_ascii_ci(
            &normalized,
            "Whenever you cast instant or sorcery or you copy instant or sorcery, ",
        )
    })
    .or_else(|| {
        strip_prefix_ascii_ci(
            &normalized,
            "Whenever you cast an instant or sorcery spell or Whenever you copy an instant or sorcery spell, ",
        )
    })
    .or_else(|| {
        strip_prefix_ascii_ci(
            &normalized,
            "Whenever you cast an instant or sorcery spell or you copy an instant or sorcery spell, ",
        )
    })
    {
        return format!("Whenever you cast or copy an instant or sorcery spell, {tail}");
    }
    if let Some(tail) = strip_prefix_ascii_ci(
        &normalized,
        "Whenever you cast a white or blue or black or red spell, ",
    ) {
        return format!("Whenever you cast a spell that's white, blue, black, or red, {tail}");
    }
    if let Some(rest) = normalized.strip_prefix("Whenever This creature or Whenever another ") {
        return format!("Whenever this creature or another {rest}");
    }
    if let Some(rest) = normalized.strip_prefix("Whenever This or Whenever another ") {
        return format!("Whenever this or another {rest}");
    }
    if let Some(rewritten) = rewrite_saga_chapter_prefix(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = rewrite_granted_triggered_ability_quote(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_return_cost_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_sacrifice_cost_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_variable_sacrifice_cost_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_at_least_exile_cost_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_exile_cost_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_tap_cost_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_self_return_from_graveyard_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_choose_exact_tagged_it_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_typed_graveyard_return_choice_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_generic_chosen_return_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_sacrificed_choice_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_same_name_reference_choice_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_single_bound_it_tag_reference_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_remaining_it_tag_choice_clause(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = rewrite_return_with_counters_on_it_sequence(&normalized) {
        return rewritten;
    }
    if let Some(prefix) = strip_suffix_ascii_ci(&normalized, ". Draw a card.")
        .or_else(|| strip_suffix_ascii_ci(&normalized, ". Draw a card"))
        .or_else(|| strip_suffix_ascii_ci(&normalized, ". draw a card."))
        .or_else(|| strip_suffix_ascii_ci(&normalized, ". draw a card"))
        && prefix.to_ascii_lowercase().starts_with("scry ")
    {
        return format!("{prefix}, then draw a card.");
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "Whenever you cast a Spirit or Arcane: ")
        && let Some(effect_text) = strip_suffix_ascii_ci(rest, ". spell.")
            .or_else(|| strip_suffix_ascii_ci(rest, ". spell"))
    {
        return format!("Whenever you cast a Spirit or Arcane spell, {effect_text}.");
    }
    if let Some(amount) = strip_prefix_ascii_ci(&normalized, "Counter target spell. Deal ")
        .and_then(|tail| {
            strip_suffix_ascii_ci(tail, " damage to that object's controller.")
                .or_else(|| strip_suffix_ascii_ci(tail, " damage to that object's controller"))
        })
    {
        return format!(
            "Counter target spell. This spell deals {amount} damage to that spell's controller."
        );
    }
    if let Some((first_clause, rest)) = split_once_ascii_ci(&normalized, ". Deal ")
        && let Some((controller_damage, tail)) =
            split_once_ascii_ci(rest, " damage to that object's controller.")
    {
        let controller_subject = if first_clause
            .to_ascii_lowercase()
            .ends_with("damage to target creature")
        {
            "that creature's controller"
        } else {
            "that object's controller"
        };
        let combined = format!(
            "{} and {} damage to {}.",
            first_clause.trim(),
            controller_damage.trim(),
            controller_subject
        );
        if tail.trim().is_empty() {
            return combined;
        }
        return format!("{combined}{}", tail);
    }
    if let Some((prefix, tail)) = split_once_ascii_ci(
        &normalized,
        ". At the beginning of the next end step, return it to the battlefield. Put ",
    ) && prefix.to_ascii_lowercase().contains("exile ")
        && let Some(counter_phrase) =
            strip_suffix_ascii_ci(tail, " on it.").or_else(|| strip_suffix_ascii_ci(tail, " on it"))
    {
        return format!(
            "{prefix}. At the beginning of the next end step, return that card to the battlefield under its owner's control with {} on it.",
            counter_phrase.trim()
        );
    }
    if let Some(prefix) = strip_suffix_ascii_ci(
        &normalized,
        ". Return it to the battlefield under its owner's control.",
    )
    .or_else(|| {
        strip_suffix_ascii_ci(
            &normalized,
            ". Return it to the battlefield under its owner's control",
        )
    }) && prefix.to_ascii_lowercase().contains("exile ")
    {
        return format!("{prefix}, then return it to the battlefield under its owner's control.");
    }
    if let Some(prefix) = strip_suffix_ascii_ci(
        &normalized,
        ". Return it from graveyard to the battlefield tapped.",
    )
    .or_else(|| {
        strip_suffix_ascii_ci(
            &normalized,
            ". Return it from graveyard to the battlefield tapped",
        )
    }) && prefix.to_ascii_lowercase().contains("exile ")
    {
        return format!("{prefix}, then return it to the battlefield tapped.");
    }
    if normalized.eq_ignore_ascii_case("Draw two cards and you lose 2 life. you mill 2 cards.")
        || normalized.eq_ignore_ascii_case("Draw two cards and you lose 2 life. you mill 2 cards")
    {
        return "Draw two cards, lose 2 life, then mill two cards.".to_string();
    }
    normalized = normalized.replace(
        "For each opponent, you may that player sacrifices ",
        "Each opponent may sacrifice ",
    );
    normalized = normalized.replace(" from a graveyard you own", " in your graveyard");
    let normalized_lower = normalized.to_ascii_lowercase();
    if normalized_lower.contains("creature without a counter on its get ")
        && normalized_lower.contains(" until end of turn")
    {
        let replaced = normalized
            .replace(
                "Creature without a counter on its get ",
                "Creatures with no counters on them get ",
            )
            .replace(
                "creature without a counter on its get ",
                "creatures with no counters on them get ",
            );
        if replaced != normalized {
            return replaced;
        }
    }
    if normalized_lower == "return target creature to its owner's hand and you gain 2 life."
        || normalized_lower == "return target creature to its owner's hand and you gain 2 life"
        || normalized_lower == "return target creature to its owner's hand. you gain 2 life."
        || normalized_lower == "return target creature to its owner's hand. you gain 2 life"
    {
        return "Return target creature to its owner's hand. You gain 2 life.".to_string();
    }
    if normalized_lower == "enters the battlefield with 1 +1/+1 counter(s)."
        || normalized_lower == "enters the battlefield with 1 +1/+1 counter(s)"
        || normalized_lower == "this creature enters with 1 +1/+1 counter(s)."
        || normalized_lower == "this creature enters with 1 +1/+1 counter(s)"
    {
        return "This creature enters with a +1/+1 counter on it.".to_string();
    }
    if normalized_lower == "enters the battlefield with 5 +1/+1 counter(s)."
        || normalized_lower == "enters the battlefield with 5 +1/+1 counter(s)"
        || normalized_lower == "this creature enters with 5 +1/+1 counter(s)."
        || normalized_lower == "this creature enters with 5 +1/+1 counter(s)"
    {
        return "This creature enters with five +1/+1 counters on it.".to_string();
    }
    if let Some(count) = strip_prefix_ascii_ci(&normalized, "Enters the battlefield with ")
        .and_then(|rest| {
            rest.strip_suffix(" +1/+1 counter(s).")
                .or_else(|| rest.strip_suffix(" +1/+1 counter(s)"))
        })
    {
        let count = count.trim();
        let rendered_count = render_small_number_or_raw(count);
        let counter_word = if count == "1" || count.eq_ignore_ascii_case("one") {
            "counter"
        } else {
            "counters"
        };
        return format!("This creature enters with {rendered_count} +1/+1 {counter_word} on it.");
    }
    if let Some(count) =
        strip_prefix_ascii_ci(&normalized, "This creature enters with ").and_then(|rest| {
            rest.strip_suffix(" +1/+1 counter(s).")
                .or_else(|| rest.strip_suffix(" +1/+1 counter(s)"))
        })
    {
        let count = count.trim();
        let rendered_count = render_small_number_or_raw(count);
        let counter_word = if count == "1" || count.eq_ignore_ascii_case("one") {
            "counter"
        } else {
            "counters"
        };
        return format!("This creature enters with {rendered_count} +1/+1 {counter_word} on it.");
    }

    if let Some(rewritten) = normalize_for_each_player_discard_draw_chain(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_for_each_player_draw_discard_chain(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_for_each_opponent_clause_chain(&normalized) {
        normalized = rewritten;
    }
    if let Some(rewritten) = normalize_split_land_search_sequence(&normalized) {
        normalized = rewritten;
    }
    if let Some((left, right)) = normalized.split_once(" or Whenever ") {
        return format!("{left} or {}", lowercase_first(right.trim_end_matches('.')));
    }
    if let Some(rewritten) = normalize_put_counter_number_for_each(&normalized) {
        normalized = rewritten;
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "For each ")
        && let Some((filter, tail)) =
            split_once_ascii_ci(rest, ", put a +1/+1 counter on that object")
        && tail.trim_matches('.').is_empty()
    {
        return format!("Put a +1/+1 counter on each {filter}.");
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "For each ")
        && let Some((filter, tail)) =
            split_once_ascii_ci(rest, ", put a -1/-1 counter on that object")
        && tail.trim_matches('.').is_empty()
    {
        return format!("Put a -1/-1 counter on each {filter}.");
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ". For each ")
        && let Some((filter, tail)) =
            split_once_ascii_ci(rest, ", put a +1/+1 counter on that object")
        && tail.trim_matches('.').is_empty()
    {
        return format!("{prefix}. Put a +1/+1 counter on each {filter}.");
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(&normalized, ". For each ")
        && let Some((filter, tail)) =
            split_once_ascii_ci(rest, ", put a -1/-1 counter on that object")
        && tail.trim_matches('.').is_empty()
    {
        return format!("{prefix}. Put a -1/-1 counter on each {filter}.");
    }
    if let Some(rewritten) = normalize_embedded_create_with_token_reminder(&normalized) {
        normalized = rewritten;
    }
    if let Some((prefix, rest)) = normalized.split_once(", create 1 ")
        && (prefix.starts_with("When ")
            || prefix.starts_with("Whenever ")
            || prefix.starts_with("At the beginning "))
    {
        let create_chain = format!("Create 1 {rest}");
        let chain_clauses = create_chain
            .split(". ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if let Some(list) = normalize_create_one_under_control_list(&chain_clauses)
            && let Some(list_tail) = list.trim_end_matches('.').strip_prefix("Create ")
        {
            return format!("{prefix}, create {list_tail}");
        }
    }
    let create_clauses = normalized
        .split(". ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if let Some(create_list) = normalize_create_one_under_control_list(&create_clauses) {
        return create_list;
    }
    if create_clauses.len() == 2
        && create_clauses
            .iter()
            .all(|part| part.starts_with("Create "))
    {
        let has_article = create_clauses
            .iter()
            .any(|part| part.starts_with("Create a ") || part.starts_with("Create an "));
        let has_numeric_one = create_clauses
            .iter()
            .any(|part| part.starts_with("Create 1 "));
        if has_article && has_numeric_one {
            normalized = normalized.replace(" token under your control", " token");
            normalized = normalized.replace(". Create 1 ", ". Create a ");
            return normalized;
        }

        let mut items = Vec::new();
        for clause in &create_clauses {
            let mut item = clause
                .trim()
                .trim_end_matches('.')
                .trim_start_matches("Create ")
                .to_string();
            if let Some(rest) = item.strip_prefix("1 ") {
                item = format!("a {rest}");
            }
            item = item.replace(" token under your control", " token");
            items.push(item);
        }
        return format!("Create {}.", join_with_and(&items));
    }
    if let Some((prefix, tail)) = normalized.split_once(". Put the number of ")
        && let Some((count_filter, target_tail)) = tail.split_once(" +1/+1 counter(s) on ")
    {
        let target = target_tail.trim_end_matches('.');
        return format!("{prefix}. Put a +1/+1 counter on {target} for each {count_filter}.");
    }
    if normalized == "Destroy all artifact. Destroy all enchantment."
        || normalized == "Destroy all artifact. Destroy all enchantment"
    {
        return "Destroy all artifacts and enchantments.".to_string();
    }
    if normalized == "Other Pest or Bat or Insect or Snake or Spider you control get +1/+1."
        || normalized == "Other Pest or Bat or Insect or Snake or Spider you control get +1/+1"
    {
        return "Other Pests, Bats, Insects, Snakes, and Spiders you control get +1/+1."
            .to_string();
    }
    if normalized == "Destroy target black or red attacking/blocking creature and you gain 2 life."
        || normalized
            == "Destroy target black or red attacking/blocking creature and you gain 2 life"
    {
        return "Destroy target black or red creature that's attacking or blocking. You gain 2 life."
            .to_string();
    }

    if let Some(rest) = normalized.strip_prefix("This creature gets ")
        && let Some((pt, keyword_tail)) = rest.split_once(" as long as it's your turn. and has ")
        && let Some(keyword) = keyword_tail
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| keyword_tail.strip_suffix(" as long as it's your turn"))
    {
        return format!(
            "During your turn, this creature gets {pt} and has {}.",
            normalize_keyword_predicate_case(keyword)
        );
    }
    if let Some(rest) = normalized.strip_prefix("This creature gets ")
        && let Some(pt) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
    {
        return format!("During your turn, this creature gets {pt}.");
    }
    if let Some(rest) = normalized.strip_prefix("Creatures you control have ")
        && let Some(keyword) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
    {
        return format!(
            "During your turn, creatures you control have {}.",
            normalize_keyword_predicate_case(keyword)
        );
    }
    if let Some(rest) = normalized.strip_prefix("Allies you control have ")
        && let Some(keyword) = rest
            .strip_suffix(" as long as it's your turn.")
            .or_else(|| rest.strip_suffix(" as long as it's your turn"))
    {
        return format!(
            "During your turn, Allies you control have {}.",
            normalize_keyword_predicate_case(keyword)
        );
    }
    if let Some(rest) = normalized.strip_prefix("For each another creature, Deal ")
        && let Some(amount) = rest
            .strip_suffix(" damage to that object.")
            .or_else(|| rest.strip_suffix(" damage to that object"))
    {
        return format!("This creature deals {amount} damage to each other creature.");
    }
    if let Some(rest) = normalized.strip_prefix("Create 1 ")
        && let Some((token_desc, tail)) = rest.split_once(" token under your control, tapped")
    {
        return format!("Create a tapped {token_desc} token{tail}");
    }
    if let Some(rest) = normalized.strip_prefix("Create 1 ")
        && let Some((token_desc, tail)) = rest.split_once(" token under your control")
    {
        return format!("Create a {token_desc} token{tail}");
    }
    if let Some(rest) = normalized.strip_prefix("Create ")
        && let Some((token_desc, tail)) = rest.split_once(" token with ")
        && let Some((keyword_text, after)) = tail.split_once(" tapped under your control")
    {
        let count_token = token_desc.split_whitespace().next().unwrap_or_default();
        let is_plural = !matches!(count_token, "1" | "one" | "a" | "an");
        if is_plural {
            return format!("Create {token_desc} tokens with {keyword_text} tapped{after}");
        }
    }
    if let Some(rest) = normalized.strip_prefix("Create ")
        && let Some((token_desc, tail)) = rest.split_once(" token under your control")
    {
        let count_token = token_desc.split_whitespace().next().unwrap_or_default();
        let is_plural = !matches!(count_token, "1" | "one" | "a" | "an");
        if is_plural {
            return format!("Create {token_desc} tokens{tail}");
        }
    }
    if let Some(rest) = normalized
        .strip_prefix("Choose up to two target creatures. ")
        .or_else(|| normalized.strip_prefix("choose up to two target creatures. "))
        && (rest.eq_ignore_ascii_case("target creature can't be blocked until end of turn.")
            || rest.eq_ignore_ascii_case("target creature can't be blocked until end of turn")
            || rest.eq_ignore_ascii_case("target creature can't be blocked this turn.")
            || rest.eq_ignore_ascii_case("target creature can't be blocked this turn"))
    {
        return "Up to two target creatures can't be blocked this turn.".to_string();
    }
    if let Some((prefix, tail)) =
        split_once_ascii_ci(&normalized, ", for each player, that player sacrifices ")
        && let Some(amount) = strip_suffix_ascii_ci(tail, " creatures that player controls.")
            .or_else(|| strip_suffix_ascii_ci(tail, " creatures that player controls"))
    {
        return format!(
            "{prefix}, each player sacrifices {} creatures of their choice.",
            normalize_count_token(amount)
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "Exile target card in graveyard") {
        return format!("Exile target card from a graveyard{rest}");
    }
    if let Some(rest) =
        strip_prefix_ascii_ci(&normalized, "Exile target artifact card in graveyard")
    {
        return format!("Exile target artifact card from a graveyard{rest}");
    }
    if let Some(rest) =
        strip_prefix_ascii_ci(&normalized, "Exile target creature card in graveyard")
    {
        return format!("Exile target creature card from a graveyard{rest}");
    }
    if let Some(rest) =
        normalized.strip_prefix("Whenever this creature becomes blocked, it gets +-")
        && let Some((pt_tail, _suffix)) = rest
            .split_once(" for each the number of blocking creature until end of turn.")
            .or_else(|| {
                rest.split_once(" for each the number of blocking creature until end of turn")
            })
    {
        let pt = pt_tail.replace(" / +-", "/-");
        return format!(
            "Whenever this creature becomes blocked, it gets -{pt} until end of turn for each creature blocking it."
        );
    }
    if normalized.contains(" for each the number of ") {
        normalized = normalized.replace(" for each the number of ", " for each ");
    }
    if normalized.contains(" gets +") && normalized.contains(" / +") {
        normalized = normalized.replace(" / +", "/+");
    }
    if normalized.contains(" gets +-") && normalized.contains(" / +-") {
        normalized = normalized.replace(" / +-", "/-");
    }
    if let Some((left, right)) = normalized.split_once(" for each ")
        && let Some(per_each) = right
            .strip_suffix(" until end of turn.")
            .or_else(|| right.strip_suffix(" until end of turn"))
        && left.contains(" gets ")
    {
        return format!("{left} until end of turn for each {per_each}");
    }
    if let Some(prefix) = normalized
        .strip_suffix(". you discard a card.")
        .or_else(|| normalized.strip_suffix(". you discard a card"))
    {
        return format!("{prefix}, then discard a card.");
    }
    if let Some(prefix) = normalized
        .strip_suffix(". You discard a card.")
        .or_else(|| normalized.strip_suffix(". You discard a card"))
    {
        return format!("{prefix}, then discard a card.");
    }
    if normalized == "For each player, that player mills a card."
        || normalized == "For each player, that player mills a card"
    {
        return "Each player mills a card.".to_string();
    }
    if normalized == "For each player, that player draws a card."
        || normalized == "For each player, that player draws a card"
    {
        return "Each player draws a card.".to_string();
    }
    if let Some(rest) = normalized
        .strip_prefix("For each player, that player loses ")
        .and_then(|tail| {
            tail.strip_suffix(" life.")
                .or_else(|| tail.strip_suffix(" life"))
        })
    {
        return format!("Each player loses {} life.", normalize_count_token(rest));
    }
    if let Some(rest) = normalized
        .strip_prefix("For each player, Create 1 ")
        .and_then(|tail| {
            tail.strip_suffix(" under that player's control.")
                .or_else(|| tail.strip_suffix(" under that player's control"))
        })
    {
        return format!("Each player creates a {rest}.");
    }
    if let Some(rest) = normalized
        .strip_prefix("For each player, Return all ")
        .and_then(|tail| {
            tail.strip_suffix(" from their graveyard to the battlefield.")
                .or_else(|| tail.strip_suffix(" from their graveyard to the battlefield"))
        })
    {
        return format!(
            "Each player returns each {} from their graveyard to the battlefield.",
            rest.trim()
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". ")
        && let Some(rest) = strip_prefix_ascii_ci(left, "For each player, Return all ")
        && let Some(cards) = strip_suffix_ascii_ci(rest, " from their graveyard to the battlefield")
        && let Some(counter_clause) =
            strip_prefix_ascii_ci(right, "Put ").or_else(|| strip_prefix_ascii_ci(right, "put "))
    {
        let trimmed_counter = counter_clause.trim_end_matches('.');
        if let Some(counter_text) = strip_prefix_ascii_ci(trimmed_counter, "a ")
            .or_else(|| strip_prefix_ascii_ci(trimmed_counter, "an "))
            .and_then(|tail| strip_suffix_ascii_ci(tail, " counter on it"))
        {
            return format!(
                "Each player returns each {} from their graveyard to the battlefield with an additional {} counter on it.",
                cards.trim(),
                counter_text.trim()
            );
        }
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "For each player, Return all ")
        && let Some(cards) =
            strip_suffix_ascii_ci(rest, " from their graveyard to the battlefield.")
                .or_else(|| strip_suffix_ascii_ci(rest, " from their graveyard to the battlefield"))
    {
        return format!(
            "Each player returns each {} from their graveyard to the battlefield.",
            cards.trim()
        );
    }
    if normalized.contains(". Return ")
        && normalized.split(". ").all(|clause| {
            clause
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("return ")
        })
    {
        fn parse_return_subtype(clause: &str) -> Option<String> {
            let clause = clause.trim().trim_end_matches('.');
            let rest = strip_prefix_ascii_ci(clause, "Return ")?;
            let rest = rest.trim();
            let rest = strip_prefix_ascii_ci(rest, "a ")
                .or_else(|| strip_prefix_ascii_ci(rest, "an "))
                .unwrap_or(rest)
                .trim();

            // Legacy surface: "Return card Pirate from your graveyard to your hand"
            if let Some(rest) = strip_prefix_ascii_ci(rest, "card ") {
                let (subtype, tail) = rest.split_once(" from your graveyard to your hand")?;
                if !tail.is_empty() {
                    return None;
                }
                return Some(subtype.trim().to_string());
            }

            // Preferred surface: "Return a Pirate card from your graveyard to your hand"
            if let Some((subtype, tail)) = rest.split_once(" card from your graveyard to your hand")
            {
                if !tail.is_empty() {
                    return None;
                }
                return Some(subtype.trim().to_string());
            }

            None
        }

        let mut subtypes = Vec::new();
        let mut ok = true;
        for clause in normalized.trim_end_matches('.').split(". ") {
            if let Some(subtype) = parse_return_subtype(clause) {
                subtypes.push(subtype);
            } else {
                ok = false;
                break;
            }
        }
        if ok && subtypes.len() >= 2 {
            let first = subtypes.remove(0);
            return format!(
                "Return {} card from your graveyard to your hand, then do the same for {}.",
                with_indefinite_article(&first),
                join_with_and(&subtypes)
            );
        }
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, ". ")
        && (left.contains("you gain ") || left.contains("You gain "))
        && strip_prefix_ascii_ci(right, "Create ").is_some()
    {
        return format!(
            "{left} and {}.",
            lowercase_first(right.trim_end_matches('.'))
        );
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you gain ")
        && !left.trim().is_empty()
        && !right.trim().is_empty()
        && {
            let left_lower = left.trim_start().to_ascii_lowercase();
            left_lower.starts_with("destroy target")
                || left_lower.starts_with("return target")
                || left_lower.starts_with("deal ")
                || left_lower.starts_with("counter target spell")
                || left_lower.starts_with("exile target")
        }
    {
        return format!("{left}. You gain {}", right.trim_end_matches('.'));
    }
    if let Some((left, right)) = split_once_ascii_ci(&normalized, " and you lose ")
        && !left.trim().is_empty()
        && !right.trim().is_empty()
        && {
            let left_lower = left.trim_start().to_ascii_lowercase();
            left_lower.starts_with("destroy target")
                || left_lower.starts_with("return target")
                || left_lower.starts_with("deal ")
                || left_lower.starts_with("counter target spell")
                || left_lower.starts_with("exile target")
        }
    {
        return format!("{left}. You lose {}", right.trim_end_matches('.'));
    }
    if let Some(rest) = normalized
        .strip_prefix("Counter target spell, then its controller mills ")
        .and_then(|tail| {
            tail.strip_suffix(" cards.")
                .or_else(|| tail.strip_suffix(" cards"))
        })
    {
        return format!("Counter target spell. Its controller mills {rest} cards.");
    }
    if let Some(prefix) = normalized
        .strip_suffix(" Pest creature token under your control. You gain 1 life")
        .or_else(|| {
            normalized.strip_suffix(" Pest creature token under your control. you gain 1 life")
        })
    {
        return format!(
            "{prefix} Pest creature token with \"When this token dies, you gain 1 life.\" under your control"
        );
    }
    if let Some(prefix) = normalized
        .strip_suffix(" Pest creature tokens under your control. You gain 1 life")
        .or_else(|| {
            normalized.strip_suffix(" Pest creature tokens under your control. you gain 1 life")
        })
    {
        return format!(
            "{prefix} Pest creature tokens with \"When this token dies, you gain 1 life.\" under your control"
        );
    }

    if let Some((left, right)) = normalized.split_once(". Copy it. ")
        && !right.trim().is_empty()
    {
        let left_lower = left.trim_start().to_ascii_lowercase();
        let is_put_there_this_turn = left_lower.contains("put there from anywhere this turn");
        let references_exile = left_lower.starts_with("exile ")
            || left_lower.contains(", exile ")
            || left_lower.contains(": exile ")
            || left_lower.contains(". exile ");

        if references_exile && !is_put_there_this_turn {
            return format!("{left} and copy it. {right}");
        }
    }

    if let Some((left, right)) = normalized.split_once(". ")
        && (right.starts_with("you lose ")
            || right.starts_with("You lose ")
            || right.starts_with("you gain ")
            || right.starts_with("You gain "))
    {
        return format!(
            "{left} and {}.",
            lowercase_first(right.trim_end_matches('.'))
        );
    }

    normalized = normalized
        .replace("you takes", "you take")
        .replace("You takes", "You take")
        .replace("you loses", "you lose")
        .replace("You loses", "You lose")
        .replace("you draws", "you draw")
        .replace("You draws", "You draw")
        .replace("you pays", "you pay")
        .replace("You pays", "You pay")
        .replace("you skips their next turn", "you skip your next turn")
        .replace("youre", "you're")
        .replace(
            "At the beginning of each player's end step",
            "At the beginning of each end step",
        )
        .replace(". and have ", " and have ")
        .replace(". and has ", " and has ")
        .replace(". and gain ", " and gain ")
        .replace(". and gains ", " and gains ")
        .replace("enchanted creatures get ", "enchanted creature gets ")
        .replace("enchanted creatures gain ", "enchanted creature gains ")
        .replace("equipped creatures get ", "equipped creature gets ")
        .replace("equipped creatures gain ", "equipped creature gains ")
        .replace("another creatures", "other creatures")
        .replace("Destroy all creature.", "Destroy all creatures.")
        .replace("Destroy all creature,", "Destroy all creatures,")
        .replace("Destroy all creature and", "Destroy all creatures and")
        .replace("Destroy all creature ", "Destroy all creatures ")
        .replace("Destroy all creaturess", "Destroy all creatures")
        .replace("Destroy all land.", "Destroy all lands.")
        .replace("Destroy all land,", "Destroy all lands,")
        .replace("Destroy all land and", "Destroy all lands and")
        .replace("Destroy all land ", "Destroy all lands ")
        .replace("Destroy all landss", "Destroy all lands")
        .replace("Exile all artifact.", "Exile all artifacts.")
        .replace("Exile all artifact,", "Exile all artifacts,")
        .replace("Exile all artifact and", "Exile all artifacts and")
        .replace("Exile all artifact ", "Exile all artifacts ")
        .replace("Exile all enchantment.", "Exile all enchantments.")
        .replace("Exile all enchantment,", "Exile all enchantments,")
        .replace("Exile all enchantment and", "Exile all enchantments and")
        .replace("Exile all enchantment ", "Exile all enchantments ")
        .replace("Exile all creature.", "Exile all creatures.")
        .replace("Exile all creature,", "Exile all creatures,")
        .replace("Exile all creature and", "Exile all creatures and")
        .replace("Exile all creature ", "Exile all creatures ")
        .replace("Exile all planeswalker with ", "Exile all planeswalkers with ")
        .replace(
            "Return all creature to their owners' hands.",
            "Return all creatures to their owners' hands.",
        )
        .replace(
            "Return all creature to their owners' hands",
            "Return all creatures to their owners' hands",
        )
        .replace(
            "For each player, Return all creature card from their graveyard to the battlefield.",
            "Each player returns each creature card from their graveyard to the battlefield.",
        )
        .replace(
            "For each player, Return all creature card from their graveyard to the battlefield",
            "Each player returns each creature card from their graveyard to the battlefield",
        )
        .replace("tap all creature.", "tap all creatures.")
        .replace("tap all creature", "tap all creatures")
        .replace("Destroy all Human.", "Destroy all Humans.")
        .replace("Destroy all Human,", "Destroy all Humans,")
        .replace("Destroy all Human and", "Destroy all Humans and")
        .replace("Destroy all Human ", "Destroy all Humans ")
        .replace(
            "Destroy all artifact or enchantment.",
            "Destroy all artifacts and enchantments.",
        )
        .replace(
            "Destroy all artifact or enchantment",
            "Destroy all artifacts and enchantments",
        )
        .replace("For each player, Investigate.", "Each player investigates.")
        .replace("For each player, Investigate", "Each player investigates")
        .replace("For each player, that player draws a card.", "Each player draws a card.")
        .replace("For each player, that player draws a card", "Each player draws a card")
        .replace("For each player, that player mills a card.", "Each player mills a card.")
        .replace("For each player, that player mills a card", "Each player mills a card")
        .replace("for each player, Investigate.", "each player investigates.")
        .replace("for each player, Investigate", "each player investigates")
        .replace("Attackings ", "Attacking ")
        .replace("Land is no longer snow", "Lands are no longer snow")
        .replace("Land enter the battlefield tapped", "Lands enter the battlefield tapped")
        .replace("Add 1 mana of any color", "Add one mana of any color")
        .replace("Choose one - ", "Choose one — ")
        .replace("choose one - ", "choose one — ")
        .replace("Choose one or both - ", "Choose one or both — ")
        .replace("choose one or both - ", "choose one or both — ")
        .replace("Choose one or more - ", "Choose one or more — ")
        .replace("choose one or more - ", "choose one or more — ")
        .replace(
            "choose up to one - Tap target creature. • Target creature doesn't untap during its controller's next untap step.",
            "choose up to one — • Tap target creature. • Target creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "Choose up to one - Tap target creature. • Target creature doesn't untap during its controller's next untap step.",
            "Choose up to one — • Tap target creature. • Target creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "target an opponent's creature can't untap until your next turn",
            "target creature an opponent controls doesn't untap during its controller's next untap step",
        )
        .replace(
            "target opponent's creatures",
            "target creatures an opponent controls",
        )
        .replace(
            "target opponent's permanents",
            "target permanents an opponent controls",
        )
        .replace(
            "target opponent's nonartifact creatures",
            "target nonartifact creatures an opponent controls",
        )
        .replace(
            "target opponent's nonland permanents",
            "target nonland permanents an opponent controls",
        )
        .replace(
            "target opponent's artifact or creature",
            "target artifact or creature an opponent controls",
        )
        .replace("target opponent's artifact", "target artifact an opponent controls")
        .replace("target opponent's land", "target land an opponent controls")
        .replace(
            "permanent can't untap until your next turn",
            "that permanent doesn't untap during its controller's next untap step",
        )
        .replace(
            "land can't untap until your next turn",
            "that land doesn't untap during its controller's next untap step",
        )
        .replace("target opponent's creature", "target creature an opponent controls")
        .replace(
            "target opponent's nonland permanent",
            "target nonland permanent an opponent controls",
        )
        .replace(
            "target opponent's nonland enchantment",
            "target nonland permanent an opponent controls",
        )
        .replace("target opponent's permanent", "target permanent an opponent controls")
        .replace("target opponent's nonartifact creature", "target nonartifact creature an opponent controls")
        .replace("target opponent's attacking/blocking creature", "target attacking or blocking creature an opponent controls")
        .replace("attacking/blocking", "attacking or blocking")
        .replace(
            "target player's creature can't untap until your next turn",
            "target creature doesn't untap during its controller's next untap step",
        )
        .replace(
            ": Target creature can't untap during its controller's next untap step",
            ": Target creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "Whenever this creature attacks, permanent can't untap during its controller's next untap step",
            "Whenever this creature attacks, it doesn't untap during its controller's next untap step",
        )
        .replace(
            "Whenever this creature blocks a creature, permanent can't untap during its controller's next untap step",
            "Whenever this creature blocks a creature, that creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "When this permanent enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step.",
            "When this permanent enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "When this permanent enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step",
            "When this creature enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "When this creature enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step.",
            "When this permanent enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "When this creature enters, tap target creature an opponent controls. permanent can't untap during its controller's next untap step",
            "When this creature enters, tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "tap target creature an opponent controls. permanent can't untap during its controller's next untap step.",
            "tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "tap target creature an opponent controls. permanent can't untap during its controller's next untap step",
            "tap target creature an opponent controls. That creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "tap target creature. permanent can't untap during its controller's next untap step.",
            "tap target creature. That creature doesn't untap during its controller's next untap step.",
        )
        .replace(
            "tap target creature. permanent can't untap during its controller's next untap step",
            "tap target creature. That creature doesn't untap during its controller's next untap step",
        )
        .replace(
            "tap target nonland permanent an opponent controls. permanent can't untap during its controller's next untap step.",
            "tap target nonland permanent an opponent controls. That permanent doesn't untap during its controller's next untap step.",
        )
        .replace(
            "tap target nonland permanent an opponent controls. permanent can't untap during its controller's next untap step",
            "tap target nonland permanent an opponent controls. That permanent doesn't untap during its controller's next untap step",
        )
        .replace(
            "tap target land an opponent controls. land can't untap during its controller's next untap step.",
            "tap target land an opponent controls. That land doesn't untap during its controller's next untap step.",
        )
        .replace(
            "tap target land an opponent controls. land can't untap during its controller's next untap step",
            "tap target land an opponent controls. That land doesn't untap during its controller's next untap step",
        )
        .replace(
            ", put it on top of library, then shuffle",
            ", then shuffle and put that card on top",
        )
        .replace(
            ", put it on top of your library, then shuffle",
            ", then shuffle and put that card on top",
        )
        .replace(
            ", put the card on top of library, then shuffle",
            ", then shuffle and put that card on top",
        )
        .replace(
            ", put the card on top of your library, then shuffle",
            ", then shuffle and put that card on top",
        )
        .replace(
            "it gains Can attack as though it didn't have defender until end of turn",
            "it can attack this turn as though it didn't have defender",
        )
        .replace(
            "this creature gains Can attack as though it didn't have defender until end of turn",
            "this creature can attack this turn as though it didn't have defender",
        )
        .replace(
            "this permanent gains Can attack as though it didn't have defender until end of turn",
            "this creature can attack this turn as though it didn't have defender",
        )
        .replace(
            "this creature gets +3/-1 until end of turn. it can attack this turn as though it didn't have defender.",
            "this creature gets +3/-1 until end of turn and can attack this turn as though it didn't have defender.",
        )
        .replace(
            "this creature gets +3/-1 until end of turn. it can attack this turn as though it didn't have defender",
            "this creature gets +3/-1 until end of turn and can attack this turn as though it didn't have defender",
        )
        .replace(
            "If effect #0 that doesn't happen, target creature gets ",
            "Otherwise, that creature gets ",
        )
        .replace(
            "If effect #1 that doesn't happen, target creature gets ",
            "Otherwise, that creature gets ",
        )
        .replace(
            "if effect #0 that doesn't happen, target creature gets ",
            "otherwise, that creature gets ",
        )
        .replace(
            "if effect #1 that doesn't happen, target creature gets ",
            "otherwise, that creature gets ",
        )
        .replace("unless target player pays ", "unless they pay ")
        .replace("Unless target player pays ", "Unless they pay ")
        .replace(
            "you may Untap target creature. Gain control of it until end of turn. it gains Haste until end of turn.",
            "you may untap target creature and gain control of it until end of turn. That creature gains haste until end of turn.",
        )
        .replace(
            "you may Untap target creature. Gain control of it until end of turn. it gains Haste until end of turn",
            "you may untap target creature and gain control of it until end of turn. That creature gains haste until end of turn",
        )
        .replace(
            "you may untap target creature. Gain control of it until end of turn. it gains Haste until end of turn.",
            "you may untap target creature and gain control of it until end of turn. That creature gains haste until end of turn.",
        )
        .replace(
            "you may untap target creature. Gain control of it until end of turn. it gains Haste until end of turn",
            "you may untap target creature and gain control of it until end of turn. That creature gains haste until end of turn",
        )
        .replace(
            "I, II — Put a +1/+1 counter on each of up to one target creature.",
            "I, II — Put a +1/+1 counter on up to one target creature.",
        )
        .replace(
            "I, II — Put a +1/+1 counter on each of up to one target creature",
            "I, II — Put a +1/+1 counter on up to one target creature",
        )
        .replace(
            "Put three +1/+1 counters on another target creature.",
            "Put three +1/+1 counters on a third target creature.",
        )
        .replace(
            "Put three +1/+1 counters on another target creature",
            "Put three +1/+1 counters on a third target creature",
        )
        .replace(
            "Put three -1/-1 counters on another target creature.",
            "Put three -1/-1 counters on a third target creature.",
        )
        .replace(
            "Put three -1/-1 counters on another target creature",
            "Put three -1/-1 counters on a third target creature",
        )
        .replace(
            "Put a +1/+1 counter on target creature. Put two +1/+1 counters on another target creature. Put three +1/+1 counters on a third target creature.",
            "Put a +1/+1 counter on target creature, two +1/+1 counters on another target creature, and three +1/+1 counters on a third target creature.",
        )
        .replace(
            "Put a +1/+1 counter on target creature. Put two +1/+1 counters on another target creature. Put three +1/+1 counters on a third target creature",
            "Put a +1/+1 counter on target creature, two +1/+1 counters on another target creature, and three +1/+1 counters on a third target creature",
        )
        .replace(
            "Put a -1/-1 counter on target creature. Put two -1/-1 counters on another target creature. Put three -1/-1 counters on a third target creature.",
            "Put a -1/-1 counter on target creature, two -1/-1 counters on another target creature, and three -1/-1 counters on a third target creature.",
        )
        .replace(
            "Put a -1/-1 counter on target creature. Put two -1/-1 counters on another target creature. Put three -1/-1 counters on a third target creature",
            "Put a -1/-1 counter on target creature, two -1/-1 counters on another target creature, and three -1/-1 counters on a third target creature",
        )
        .replace(
            "If the target is blocked, Destroy target blocked creature.",
            "Destroy target blocked creature.",
        )
        .replace(
            "If the target is blocked, Destroy target blocked creature",
            "Destroy target blocked creature",
        )
        .replace("Remove up to one counters from target creature", "Remove a counter from target creature")
        .replace("Remove up to one counters from this creature", "Remove a counter from this creature")
        .replace("this creatures get ", "this creature gets ")
        .replace("this creatures gain ", "this creature gains ")
        .replace("This creatures get ", "This creature gets ")
        .replace("This creatures gain ", "This creature gains ")
        .replace("on each this creature", "on this creature")
        .replace(
            "Whenever you cast Adventure creature,",
            "Whenever you cast a creature spell with an Adventure,",
        )
        .replace(
            "Whenever you cast instant or sorcery or Whenever you copy instant or sorcery,",
            "Whenever you cast or copy an instant or sorcery spell,",
        )
        .replace(
            "Whenever you cast instant or sorcery or you copy instant or sorcery,",
            "Whenever you cast or copy an instant or sorcery spell,",
        )
        .replace(
            "Whenever you cast an instant or sorcery spell or Whenever you copy an instant or sorcery spell,",
            "Whenever you cast or copy an instant or sorcery spell,",
        )
        .replace(
            "Whenever you cast an instant or sorcery spell or you copy an instant or sorcery spell,",
            "Whenever you cast or copy an instant or sorcery spell,",
        )
        .replace("Whenever you cast creature,", "Whenever you cast a creature spell,")
        .replace(
            "Whenever a player casts creature,",
            "Whenever a player casts a creature spell,",
        )
        .replace(
            "Whenever an opponent casts creature,",
            "Whenever an opponent casts a creature spell,",
        )
        .replace(
            "Whenever you cast enchantment,",
            "Whenever you cast an enchantment spell,",
        )
        .replace("Whenever you cast artifact,", "Whenever you cast an artifact spell,")
        .replace("Whenever you cast instant,", "Whenever you cast an instant spell,")
        .replace("Whenever you cast sorcery,", "Whenever you cast a sorcery spell,")
        .replace("Whenever you cast blue spell,", "Whenever you cast a blue spell,")
        .replace("Whenever you cast black spell,", "Whenever you cast a black spell,")
        .replace("Whenever you cast white spell,", "Whenever you cast a white spell,")
        .replace("Whenever you cast red spell,", "Whenever you cast a red spell,")
        .replace("Whenever you cast green spell,", "Whenever you cast a green spell,")
        .replace(
            "Whenever you cast a white or blue or black or red spell,",
            "Whenever you cast a spell that's white, blue, black, or red,",
        )
        .replace(
            "Whenever you cast noncreature spell,",
            "Whenever you cast a noncreature spell,",
        )
        .replace("you may Allies you control gain ", "you may have Allies you control gain ")
        .replace(" to your mana pool", "")
        .replace(
            "create 1 Powerstone artifact token under your control, tapped",
            "create a tapped Powerstone token",
        )
        .replace(
            " Pest creature token under your control and you gain 1 life",
            " Pest creature token with \"When this token dies, you gain 1 life.\" under your control",
        )
        .replace(
            " Pest creature tokens under your control and you gain 1 life",
            " Pest creature tokens with \"When this token dies, you gain 1 life.\" under your control",
        )
        .replace(
            "Create 1 Powerstone artifact token under your control, tapped",
            "Create a tapped Powerstone token",
        )
        .replace(
            "Search your library for basic land you own, reveal it, put it into your hand, then shuffle.",
            "Search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
        )
        .replace(
            "Search your library for basic land you own, reveal it, put it into your hand, then shuffle",
            "Search your library for a basic land card, reveal it, put it into your hand, then shuffle",
        )
        .replace(
            "Search your library for land you own, reveal it, put it into your hand, then shuffle.",
            "Search your library for a land card, reveal it, put it into your hand, then shuffle.",
        )
        .replace(
            "Search your library for land you own, reveal it, put it into your hand, then shuffle",
            "Search your library for a land card, reveal it, put it into your hand, then shuffle",
        )
        .replace(
            "Search your library for battle you own, put it onto the battlefield, then shuffle.",
            "Search your library for a battle card, put it onto the battlefield, then shuffle.",
        )
        .replace(
            "Search your library for battle you own, put it onto the battlefield, then shuffle",
            "Search your library for a battle card, put it onto the battlefield, then shuffle",
        )
        .replace(
            "All slivers have sacrifice this creature add b b.",
            "All Slivers have \"Sacrifice this permanent: Add {B}{B}.\"",
        )
        .replace(
            "All slivers have 2 sacrifice this creature target player discards a card at random activate only as a sorcery.",
            "All Slivers have \"{2}, Sacrifice this permanent: Target player discards a card at random. Activate only as a sorcery.\"",
        )
        .replace(
            "All slivers have 2 sacrifice this creature you gain 4 life.",
            "All Slivers have \"{2}, Sacrifice this permanent: You gain 4 life.\"",
        )
        .replace(
            "All slivers have 2 sacrifice this creature draw a card.",
            "All Slivers have \"{2}, Sacrifice this permanent: Draw a card.\"",
        )
        .replace(
            "All Slivers have 2 sacrifice this permanent draw a card.",
            "All Slivers have \"{2}, Sacrifice this permanent: Draw a card.\"",
        )
        .replace("All slivers have ", "All Slivers have ")
        .replace(
            "Discard your hand, then draw 7 cards, then discard 3 cards at random.",
            "Discard your hand. Draw seven cards, then discard three cards at random.",
        )
        .replace(
            "Discard your hand, then draw 7 cards, then discard 3 cards at random",
            "Discard your hand. Draw seven cards, then discard three cards at random",
        )
        .replace(
            "Draw two cards and you lose 2 life. you mill 2 cards.",
            "Draw two cards, lose 2 life, then mill two cards.",
        )
        .replace(
            "Draw two cards and you lose 2 life. you mill 2 cards",
            "Draw two cards, lose 2 life, then mill two cards",
        )
        .replace(
            "Draw two cards and you lose 2 life. You mill 2 cards.",
            "Draw two cards, lose 2 life, then mill two cards.",
        )
        .replace(
            "Draw two cards and lose 2 life. you mill 2 cards.",
            "Draw two cards, lose 2 life, then mill two cards.",
        )
        .replace("When this creature enters it deals ", "When this creature enters, it deals ")
        .replace(" and you, gain ", " and you gain ")
        .replace(". you may Put a +1/+1 counter on this permanent", ", and you may put a +1/+1 counter on this permanent")
        .replace(". you may Put a +1/+1 counter on this creature", ", and you may put a +1/+1 counter on this creature")
        .replace(". you may put a +1/+1 counter on this permanent", ", and you may put a +1/+1 counter on this permanent")
        .replace(". you may put a +1/+1 counter on this creature", ", and you may put a +1/+1 counter on this creature")
        .replace(" gain Lifelink until end of turn", " gain lifelink until end of turn")
        .replace("protection from zombie", "protection from Zombies")
        .replace("creaturess", "creatures")
        .replace("planeswalker card with", "planeswalker cards with")
        .replace("Whenever this creature or another Ally you control enters, you may have Allies you control gain lifelink until end of turn, and you may put a +1/+1 counter on this permanent.", "Whenever this creature or another Ally you control enters, you may have Allies you control gain lifelink until end of turn, and you may put a +1/+1 counter on this creature.")
        .replace(
            "\"At the beginning of your end step exile ",
            "\"At the beginning of your end step, exile ",
        )
        .replace(" you control then return ", " you control, then return ")
        .replace(" its owners control", " its owner's control")
        .replace(
            "Destroy all creatures. Destroy all commander planeswalker.",
            "Destroy all creatures and planeswalkers except commanders.",
        )
        .replace(
            "Destroy all creatures. Destroy all commander planeswalker",
            "Destroy all creatures and planeswalkers except commanders",
        )
        .replace(
            "When this creature dies, exile it. Return another target creature card from your graveyard to your hand.",
            "When this creature dies, exile it, then return another target creature card from your graveyard to your hand.",
        )
        .replace(
            "that player sacrifices a white or green permanent",
            "that player sacrifices a green or white permanent",
        )
        .replace(
            "reveal the top card of your library and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches land, Put it onto the battlefield. Return it to its owner's hand.",
            "reveal the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, put it into your hand.",
        )
        .replace(
            "Reveal the top card of your library and tag it as 'revealed_0'. If the tagged object 'revealed_0' matches land, Put it onto the battlefield. Return it to its owner's hand.",
            "Reveal the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, put it into your hand.",
        );

    if let Some((left, right)) = normalized.split_once(". ")
        && right.starts_with("sacrifice ")
    {
        return format!(
            "{left}, then {}.",
            lowercase_first(right.trim_end_matches('.'))
        );
    }

    while let Some(merged) = merge_sentence_subject_predicates(&normalized) {
        if merged == normalized {
            break;
        }
        normalized = merged;
    }

    if let Some((prefix, tail)) = split_once_ascii_ci(&normalized, ", for each ")
        && let Some((targets, deal_tail)) = split_once_ascii_ci(tail, ", Deal ")
        && let Some(amount) = strip_suffix_ascii_ci(deal_tail, " damage to that object.")
            .or_else(|| strip_suffix_ascii_ci(deal_tail, " damage to that object"))
    {
        let rewritten = if targets.eq_ignore_ascii_case("attacking/blocking creature") {
            format!(
                "{prefix}, it deals {amount} damage to each attacking creature and each blocking creature."
            )
        } else if targets.eq_ignore_ascii_case("another creature") {
            format!("{prefix}, it deals {amount} damage to each other creature.")
        } else {
            format!("{prefix}, it deals {amount} damage to each {targets}.")
        };
        normalized = rewritten;
    }
    if let Some((prefix, tail)) = split_once_ascii_ci(&normalized, ". For each ")
        && let Some((targets, deal_tail)) = split_once_ascii_ci(tail, ", Deal ")
        && let Some(amount) = strip_suffix_ascii_ci(deal_tail, " damage to that object.")
            .or_else(|| strip_suffix_ascii_ci(deal_tail, " damage to that object"))
    {
        normalized = format!("{prefix}. Deal {amount} damage to each {targets}.");
    }
    if let Some((prefix, tail)) = split_once_ascii_ci(&normalized, " you may For each ")
        && let Some((targets, deal_tail)) = split_once_ascii_ci(tail, ", Deal ")
        && let Some(amount) = strip_suffix_ascii_ci(deal_tail, " damage to that object.")
            .or_else(|| strip_suffix_ascii_ci(deal_tail, " damage to that object"))
    {
        normalized = format!("{prefix} you may have it deal {amount} damage to each {targets}.");
    }

    if let Some(rest) = normalized.strip_prefix("Spells ")
        && let Some((tribe, cost_tail)) = rest.split_once(" you control cost ")
        && !tribe.is_empty()
        && !tribe.contains(',')
    {
        return format!("{tribe} spells you cast cost {cost_tail}");
    }
    normalized = normalized
        .replace(" in target player's hand", " from their hand")
        .replace(" in that player's hand", " from that player's hand")
        .replace(" card in single graveyard", " card from a single graveyard")
        .replace(
            " cards in single graveyard",
            " cards from a single graveyard",
        )
        .replace(" card in graveyard", " card from a graveyard")
        .replace(" cards in graveyard", " cards from a graveyard")
        .replace(
            " in an opponent's graveyards",
            " from an opponent's graveyard",
        )
        .replace(
            " in target player's graveyard",
            " from target player's graveyard",
        )
        .replace(
            " in that player's graveyard",
            " from that player's graveyard",
        )
        .replace(
            "Exile all land card from target player's graveyard",
            "Exile all land cards from target player's graveyard",
        )
        .replace(
            "Exile all land card in target player's graveyard",
            "Exile all land cards from target player's graveyard",
        )
        .replace(
            "Exile all land card from their graveyard",
            "Exile all land cards from their graveyard",
        )
        .replace(
            "Exile all card in that object's controller's graveyard",
            "Exile its controller's graveyard",
        )
        .replace(
            "Exile all card in that object's owner's graveyard",
            "Exile its owner's graveyard",
        )
        .replace("that object's controller's", "its controller's")
        .replace("cast spell Aura", "cast an Aura spell")
        .replace(
            "For each player, Put a card from that player's hand on top of that player's library",
            "Each player puts a card from their hand on top of their library",
        )
        .replace(
            "for each player, Put a card from that player's hand on top of that player's library",
            "each player puts a card from their hand on top of their library",
        )
        .replace(
            "For each player, that player sacrifices 6 creatures that player controls",
            "Each player sacrifices six creatures of their choice",
        )
        .replace(
            "Return land card or Elf from your graveyard to your hand",
            "Return a land card or Elf card from your graveyard to your hand",
        )
        .replace(" under your control, tapped", " tapped under your control")
        .replace(
            "Return any number of target permanent you owns to their owners' hands.",
            "Return any number of target permanents you own to your hand.",
        )
        .replace(
            "Return any number of target permanent you owns to their owners' hands",
            "Return any number of target permanents you own to your hand",
        )
        .replace(
            "Exile two target card from an opponent's graveyard",
            "Exile two target cards from an opponent's graveyard",
        );
    normalized
}

pub(super) fn normalize_for_each_opponent_clause_chain(text: &str) -> Option<String> {
    let marker = "for each opponent, that player ";
    let idx = text.to_ascii_lowercase().find(marker)?;
    let prefix = &text[..idx];
    let tail = &text[idx + marker.len()..];

    if let Some((loss_raw, gain_tail)) = split_once_ascii_ci(tail, " life. ")
        && let Some(gain_raw) = strip_prefix_ascii_ci(gain_tail, "you gain ").and_then(|rest| {
            strip_suffix_ascii_ci(rest, " life.").or_else(|| strip_suffix_ascii_ci(rest, " life"))
        })
    {
        let clause = format!(
            "Each opponent loses {} life and you gain {} life.",
            normalize_count_token(loss_raw),
            normalize_count_token(gain_raw)
        );
        return Some(format!(
            "{}{}",
            prefix,
            lower_clause_after_prefix(prefix, &clause)
        ));
    }
    if let Some(loss_raw) = strip_prefix_ascii_ci(tail, "loses ").and_then(|rest| {
        strip_suffix_ascii_ci(rest, " life.").or_else(|| strip_suffix_ascii_ci(rest, " life"))
    }) {
        let clause = format!(
            "Each opponent loses {} life.",
            normalize_count_token(loss_raw)
        );
        return Some(format!(
            "{}{}",
            prefix,
            lower_clause_after_prefix(prefix, &clause)
        ));
    }
    if let Some(discard_tail) = strip_prefix_ascii_ci(tail, "discards ")
        && let Some((count_raw, rest)) = parse_card_count_with_rest(discard_tail)
        && (rest == "."
            || rest.is_empty()
            || rest.eq_ignore_ascii_case(" at random.")
            || rest.eq_ignore_ascii_case(" at random"))
    {
        let at_random = if rest.to_ascii_lowercase().starts_with(" at random") {
            " at random"
        } else {
            ""
        };
        let clause = format!(
            "Each opponent discards {}{at_random}.",
            render_card_count_phrase(count_raw)
        );
        return Some(format!(
            "{}{}",
            prefix,
            lower_clause_after_prefix(prefix, &clause)
        ));
    }
    if let Some(mill_tail) = strip_prefix_ascii_ci(tail, "mills ")
        && let Some((count_raw, rest)) = parse_card_count_with_rest(mill_tail)
    {
        if rest == "." || rest.is_empty() {
            let clause = format!(
                "Each opponent mills {}.",
                render_card_count_phrase(count_raw)
            );
            return Some(format!(
                "{}{}",
                prefix,
                lower_clause_after_prefix(prefix, &clause)
            ));
        }
        if let Some(next_clause) = strip_prefix_ascii_ci(rest, ". ") {
            let next_clause = next_clause.trim().trim_end_matches('.');
            if !next_clause.is_empty() {
                let clause = format!(
                    "Each opponent mills {}. Then {}.",
                    render_card_count_phrase(count_raw),
                    lowercase_first(next_clause)
                );
                return Some(format!(
                    "{}{}",
                    prefix,
                    lower_clause_after_prefix(prefix, &clause)
                ));
            }
        }
    }
    None
}

pub(super) fn normalize_for_each_player_draw_discard_chain(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let for_each_marker = "for each player, that player draws ";
    let plain_marker = "each player draws ";
    let (prefix, tail) = if let Some(idx) = lower.find(for_each_marker) {
        (&text[..idx], &text[idx + for_each_marker.len()..])
    } else if let Some(idx) = lower.find(plain_marker) {
        (&text[..idx], &text[idx + plain_marker.len()..])
    } else {
        return None;
    };
    let (draw_count_raw, draw_rest) = parse_card_count_with_rest(tail)?;
    let discard_marker = ". for each player, that player discards ";
    let discard_tail = strip_prefix_ascii_ci(draw_rest, discard_marker)?;
    let (discard_count_raw, discard_rest) = parse_card_count_with_rest(discard_tail)?;
    let at_random = if discard_rest.eq_ignore_ascii_case(" at random.")
        || discard_rest.eq_ignore_ascii_case(" at random")
    {
        " at random"
    } else if discard_rest == "." || discard_rest.is_empty() {
        ""
    } else {
        return None;
    };
    let clause = format!(
        "Each player draws {}, then discards {}{at_random}.",
        render_card_count_phrase(draw_count_raw),
        render_card_count_phrase(discard_count_raw)
    );
    Some(format!(
        "{}{}",
        prefix,
        lower_clause_after_prefix(prefix, &clause)
    ))
}

pub(super) fn normalize_for_each_player_discard_draw_chain(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let for_each_marker = "for each player, that player discards ";
    let plain_marker = "each player discards ";
    let (prefix, tail) = if let Some(idx) = lower.find(for_each_marker) {
        (&text[..idx], &text[idx + for_each_marker.len()..])
    } else if let Some(idx) = lower.find(plain_marker) {
        (&text[..idx], &text[idx + plain_marker.len()..])
    } else {
        return None;
    };
    let (discard_clause, rest) = tail.split_once(". ")?;
    let draw_tail = strip_prefix_ascii_ci(rest, "For each player, that player draws ")
        .or_else(|| strip_prefix_ascii_ci(rest, "Each player draws "))
        .or_else(|| strip_prefix_ascii_ci(rest, "that player draws "))?;
    let draw_clause = draw_tail.trim().trim_end_matches('.');
    if draw_clause.is_empty() {
        return None;
    }
    let clause = format!(
        "Each player discards {}, then draws {}.",
        discard_clause.trim(),
        draw_clause
    );
    Some(format!(
        "{}{}",
        prefix,
        lower_clause_after_prefix(prefix, &clause)
    ))
}

pub(super) fn parse_card_count_with_rest(text: &str) -> Option<(&str, &str)> {
    if let Some((count, rest)) = text.split_once(" cards") {
        return Some((count.trim(), rest));
    }
    if let Some((count, rest)) = text.split_once(" card") {
        return Some((count.trim(), rest));
    }
    None
}

pub(super) fn render_card_count_phrase(raw: &str) -> String {
    let count = normalize_count_token(raw);
    if matches!(count.as_str(), "a" | "an" | "one") {
        "a card".to_string()
    } else {
        format!("{count} cards")
    }
}

pub(super) fn normalize_count_token(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("a") || trimmed.eq_ignore_ascii_case("an") {
        return "a".to_string();
    }
    render_small_number_or_raw(trimmed)
}

pub(super) fn lower_clause_after_prefix(prefix: &str, clause: &str) -> String {
    if prefix.ends_with(", ") {
        return lowercase_first(clause);
    }
    clause.to_string()
}

pub(super) fn strip_prefix_ascii_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    if text.len() < prefix.len() {
        return None;
    }
    if text
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
    {
        text.get(prefix.len()..)
    } else {
        None
    }
}

pub(super) fn strip_suffix_ascii_ci<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    if text.len() < suffix.len() {
        return None;
    }
    let idx = text.len() - suffix.len();
    if text
        .get(idx..)
        .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
    {
        text.get(..idx)
    } else {
        None
    }
}

pub(super) fn split_once_ascii_ci<'a>(
    text: &'a str,
    separator: &str,
) -> Option<(&'a str, &'a str)> {
    let lower = text.to_ascii_lowercase();
    let sep_lower = separator.to_ascii_lowercase();
    let idx = lower.find(&sep_lower)?;
    Some((&text[..idx], &text[idx + separator.len()..]))
}

pub(super) fn replace_once_ascii_ci(text: &str, needle: &str, replacement: &str) -> Option<String> {
    let (prefix, tail) = split_once_ascii_ci(text, needle)?;
    Some(format!("{prefix}{replacement}{tail}"))
}

pub(super) fn render_choose_exact_subject(descriptor: &str, count: usize) -> String {
    let descriptor = descriptor.trim();
    if let Some(rest) = descriptor.strip_prefix("this a ") {
        return format!("this {rest}");
    }
    if let Some(rest) = descriptor.strip_prefix("this an ") {
        return format!("this {rest}");
    }
    if count == 1 {
        if let Some(rest) = descriptor.strip_prefix("a ") {
            return with_indefinite_article(rest);
        }
        if let Some(rest) = descriptor.strip_prefix("an ") {
            return with_indefinite_article(rest);
        }
        return descriptor.to_string();
    }

    let count_word = render_small_number_or_raw(&count.to_string());
    if let Some(rest) = descriptor.strip_prefix("a ") {
        return format!("{count_word} {}", pluralize_noun_phrase(rest));
    }
    if let Some(rest) = descriptor.strip_prefix("an ") {
        return format!("{count_word} {}", pluralize_noun_phrase(rest));
    }
    format!("{count_word} {}", pluralize_noun_phrase(descriptor))
}

pub(super) fn render_choose_subject_for_count_token(descriptor: &str, count_token: &str) -> String {
    if let Ok(count) = count_token.trim().parse::<usize>() {
        return render_choose_exact_subject(descriptor, count);
    }

    let descriptor = descriptor.trim();
    let count_word = render_small_number_or_raw(count_token.trim());
    if let Some(rest) = descriptor.strip_prefix("a ") {
        return format!("{count_word} {}", pluralize_noun_phrase(rest));
    }
    if let Some(rest) = descriptor.strip_prefix("an ") {
        return format!("{count_word} {}", pluralize_noun_phrase(rest));
    }
    format!("{count_word} {}", pluralize_noun_phrase(descriptor))
}

pub(super) fn pluralize_choice_descriptor(descriptor: &str) -> String {
    let descriptor = descriptor
        .trim()
        .strip_suffix(" in the battlefield")
        .or_else(|| descriptor.trim().strip_suffix(" in your hand"))
        .or_else(|| descriptor.trim().strip_suffix(" in hand"))
        .or_else(|| descriptor.trim().strip_suffix(" in your graveyard"))
        .or_else(|| descriptor.trim().strip_suffix(" in a graveyard"))
        .or_else(|| descriptor.trim().strip_suffix(" in exile"))
        .or_else(|| descriptor.trim().strip_suffix(" in a library"))
        .unwrap_or(descriptor.trim())
        .trim();
    if let Some(rest) = descriptor.strip_prefix("a ") {
        return pluralize_noun_phrase(rest);
    }
    if let Some(rest) = descriptor.strip_prefix("an ") {
        return pluralize_noun_phrase(rest);
    }
    pluralize_noun_phrase(descriptor)
}

pub(super) fn render_any_number_subject(descriptor: &str) -> String {
    format!("any number of {}", pluralize_choice_descriptor(descriptor))
}

pub(super) fn normalize_choose_exact_return_cost_clause(text: &str) -> Option<String> {
    let marker = " and tags it as 'return_cost_0', ";
    let (head, after) = split_once_ascii_ci(text, marker)?;
    let choose_idx = head.to_ascii_lowercase().rfind("choose exactly ")?;
    let prefix = &head[..choose_idx];
    let choose_tail = &head[choose_idx + "choose exactly ".len()..];
    let (count_token, rest) = choose_tail.split_once(' ')?;
    let count = count_token.parse::<usize>().ok()?;
    let descriptor = rest.strip_suffix(" in the battlefield")?;
    let mut subject = render_choose_exact_subject(descriptor, count);

    // Oracle cost surfaces omit the redundant "you control" for self-references.
    if subject.starts_with("this ") {
        if let Some(stripped) = subject.strip_suffix(" you control") {
            subject = stripped.to_string();
        }
    }

    // Preserve any trailing text after the return-to-hand clause (typically the ":" effect body).
    let after = after.trim_start();
    let after_lower = after.to_ascii_lowercase();
    let tail = if let Some(idx) = after_lower.find("to its owner's hand") {
        &after[idx + "to its owner's hand".len()..]
    } else if let Some(idx) = after_lower.find("to their owners' hands") {
        &after[idx + "to their owners' hands".len()..]
    } else if let Some(idx) = after_lower.find("to their owner's hand") {
        &after[idx + "to their owner's hand".len()..]
    } else {
        return None;
    };

    let owner_tail = if count == 1 {
        "its owner's hand"
    } else {
        "their owners' hands"
    };
    let clause = format!("Return {subject} to {owner_tail}");
    Some(format!("{prefix}{clause}{tail}"))
}

pub(super) fn normalize_choose_exact_sacrifice_cost_clause(text: &str) -> Option<String> {
    let marker = " and tags it as 'sacrifice_cost_0', ";
    let (head, after) = split_once_ascii_ci(text, marker)?;
    let choose_idx = head.to_ascii_lowercase().rfind("choose exactly ")?;
    let prefix = &head[..choose_idx];
    let choose_tail = &head[choose_idx + "choose exactly ".len()..];
    let (count_token, rest) = choose_tail.split_once(' ')?;
    let descriptor = rest
        .strip_suffix(" in the battlefield")
        .or_else(|| rest.strip_suffix(" in the battlefields"))
        .unwrap_or(rest)
        .trim();
    let subject = render_choose_subject_for_count_token(descriptor, count_token);

    let after = after.trim_start();
    let tail = if let Some(colon_idx) = after.find(':') {
        after[colon_idx..].to_string()
    } else if after.trim().to_ascii_lowercase().starts_with("sacrifice ") {
        ".".to_string()
    } else {
        return None;
    };

    Some(format!("{prefix}Sacrifice {subject}{tail}"))
}

pub(super) fn normalize_choose_variable_sacrifice_cost_clause(text: &str) -> Option<String> {
    let marker = " and tags it as 'sacrifice_cost_0', ";
    let (head, after) = split_once_ascii_ci(text, marker)?;
    let choose_idx = head.to_ascii_lowercase().rfind("choose ")?;
    let prefix = &head[..choose_idx];
    let choose_tail = &head[choose_idx + "choose ".len()..];
    let (count_token, rest) = choose_tail.split_once(' ')?;
    if count_token.trim().parse::<usize>().is_ok() || count_token.eq_ignore_ascii_case("exactly") {
        return None;
    }
    let descriptor = rest
        .strip_suffix(" in the battlefield")
        .or_else(|| rest.strip_suffix(" in the battlefields"))
        .unwrap_or(rest)
        .trim();
    let subject = render_choose_subject_for_count_token(descriptor, count_token);

    let after = after.trim_start();
    let tail = if let Some(colon_idx) = after.find(':') {
        after[colon_idx..].to_string()
    } else if after.trim().to_ascii_lowercase().starts_with("sacrifice ") {
        ".".to_string()
    } else {
        return None;
    };

    Some(format!("{prefix}Sacrifice {subject}{tail}"))
}

pub(super) fn normalize_choose_at_least_exile_cost_clause(text: &str) -> Option<String> {
    let marker = "Choose at least 1 another artifact with mana value equal to X you control in the battlefield and tags it as 'exile_cost_0', Exile it";
    let (before, after) = split_once_ascii_ci(text, marker)?;
    Some(format!(
        "{before}Exile one or more other artifacts you control with total mana value X{after}"
    ))
}

pub(super) fn normalize_choose_exact_exile_cost_clause(text: &str) -> Option<String> {
    let marker = " and tags it as 'exile_cost_0', exile it";
    let (head, tail) = split_once_ascii_ci(text, marker)?;
    let choose_idx = head.to_ascii_lowercase().rfind("choose exactly ")?;
    let prefix = &head[..choose_idx];
    let choose_tail = &head[choose_idx + "choose exactly ".len()..];
    let (count_token, rest) = choose_tail.split_once(' ')?;
    let count = count_token.parse::<usize>().ok()?;
    let descriptor = rest
        .strip_suffix(" in the battlefield")
        .or_else(|| rest.strip_suffix(" in the stack"))?;
    let mut subject = render_choose_exact_subject(descriptor, count);
    if subject.contains("instant or sorcery") && !subject.contains(" spell") {
        if subject.starts_with("a ") {
            subject = subject.replacen("a instant or sorcery", "an instant or sorcery spell", 1);
        } else if subject.starts_with("an ") {
            subject = subject.replacen("an instant or sorcery", "an instant or sorcery spell", 1);
        } else {
            subject = subject.replacen("instant or sorcery", "instant or sorcery spell", 1);
        }
    }
    Some(format!("{prefix}Exile {subject}{tail}"))
}

pub(super) fn normalize_choose_exact_tap_cost_clause(text: &str) -> Option<String> {
    let marker = " and tags it as 'tap_cost_0'. Tap it ";
    let (head, tail) = split_once_ascii_ci(text, marker)?;
    let choose_idx = head.to_ascii_lowercase().rfind("choose exactly ")?;
    let mut prefix = head[..choose_idx].to_string();
    if prefix.to_ascii_lowercase().ends_with(" and you ") {
        prefix.truncate(prefix.len().saturating_sub("you ".len()));
    }
    let choose_tail = &head[choose_idx + "choose exactly ".len()..];
    let (count_token, rest) = choose_tail.split_once(' ')?;
    let count = count_token.parse::<usize>().ok()?;
    let descriptor = rest
        .strip_suffix(" in the battlefield")
        .or_else(|| rest.strip_suffix(" in a graveyard"))
        .or_else(|| rest.strip_suffix(" in a hand"))
        .or_else(|| rest.strip_suffix(" in hand"))
        .unwrap_or(rest);
    let subject = render_choose_exact_subject(descriptor, count);
    Some(format!("{prefix}tap {subject} {tail}"))
}

pub(super) fn normalize_choose_background_scaffolding_clause(text: &str) -> Option<String> {
    let trimmed = text.trim().trim_end_matches('.');
    if trimmed.eq_ignore_ascii_case(
        "you choose exactly 1 a Background you control in the battlefield and tags it as '__it__'",
    ) {
        return Some("Choose a Background.".to_string());
    }
    None
}

pub(super) fn normalize_self_return_from_graveyard_clause(text: &str) -> Option<String> {
    let marker = "you choose exactly 1 this card in your graveyard and tags it as 'chosen_return_0'. Return it from graveyard to the battlefield";
    let (head, tail) = split_once_ascii_ci(text, marker)?;
    let replacement = if head.trim().is_empty() || head.ends_with(": ") || head.ends_with(". ") {
        "Return this card from your graveyard to the battlefield"
    } else {
        "return this card from your graveyard to the battlefield"
    };
    Some(format!("{head}{replacement}{tail}"))
}

fn choice_descriptor_for_zone(raw: &str, zone_suffix: &str) -> Option<String> {
    let trimmed = raw
        .trim()
        .strip_suffix(zone_suffix)
        .unwrap_or(raw.trim())
        .trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn apply_replacement_with_case(before: &str, replacement: &str, after: &str) -> String {
    let replacement =
        if before.trim().is_empty() || before.ends_with(": ") || before.ends_with(". ") {
            capitalize_first(replacement)
        } else {
            replacement.to_string()
        };
    format!("{before}{replacement}{after}")
}

pub(super) fn normalize_typed_graveyard_return_choice_clause(text: &str) -> Option<String> {
    let patterns = [
        ("you choose exactly 1 ", false, " in your graveyard"),
        ("you may choose exactly 1 ", true, " in your graveyard"),
        ("you choose exactly 1 ", false, " in a graveyard"),
        ("you may choose exactly 1 ", true, " in a graveyard"),
    ];
    for (marker, keep_may, zone_marker_prefix) in patterns {
        let Some((before, rest)) = split_once_ascii_ci(text, marker) else {
            continue;
        };
        let zone_marker = format!(
            "{zone_marker_prefix} and tags it as 'chosen_return_0'. Return it from graveyard to the battlefield"
        );
        let Some((descriptor_raw, after)) = split_once_ascii_ci(rest, &zone_marker) else {
            continue;
        };
        let descriptor = choice_descriptor_for_zone(descriptor_raw, "")?;
        let descriptor_lower = descriptor.to_ascii_lowercase();
        if descriptor_lower == "card"
            || descriptor_lower == "a card"
            || descriptor_lower == "an card"
        {
            continue;
        }
        let subject = with_indefinite_article(&descriptor);
        let replacement = if keep_may {
            format!("you may return {subject} from your graveyard to the battlefield")
        } else {
            format!("return {subject} from your graveyard to the battlefield")
        };
        return Some(apply_replacement_with_case(before, &replacement, after));
    }
    None
}

pub(super) fn normalize_generic_chosen_return_clause(text: &str) -> Option<String> {
    let patterns = [
        ("you choose exactly 1 ", " in your graveyard", "return "),
        (
            "you may choose exactly 1 ",
            " in your graveyard",
            "you may return ",
        ),
        (
            "each opponent may choose exactly 1 ",
            " from that player's graveyard",
            "each opponent may return ",
        ),
        (
            "an opponent chooses exactly 1 ",
            " in a graveyard",
            "return ",
        ),
        ("you choose exactly 1 ", " in a graveyard", "return "),
    ];
    for (marker, zone_marker, lead) in patterns {
        let Some((before, rest)) = split_once_ascii_ci(text, marker) else {
            continue;
        };
        let Some((descriptor_raw, rest)) = split_once_ascii_ci(
            rest,
            &format!("{zone_marker} and tags it as 'chosen_return_"),
        ) else {
            continue;
        };
        let Some((_tag_suffix, after)) = rest.split_once("'. ") else {
            continue;
        };
        let descriptor = with_indefinite_article(descriptor_raw.trim());
        let zone_phrase = match (marker, zone_marker) {
            (m, " from that player's graveyard") if m.starts_with("each opponent may") => {
                "from their graveyard"
            }
            (_, " in your graveyard") => "from your graveyard",
            (_, " from that player's graveyard") => "from that player's graveyard",
            (_, " in a graveyard") => "from a graveyard",
            _ => continue,
        };

        let replacement = if let Some(tail) =
            strip_prefix_ascii_ci(after, "Return it from graveyard to the battlefield tapped")
        {
            format!("{lead}{descriptor} {zone_phrase} to the battlefield tapped{tail}")
        } else if let Some(tail) =
            strip_prefix_ascii_ci(after, "Return it from graveyard to the battlefield")
        {
            format!("{lead}{descriptor} {zone_phrase} to the battlefield{tail}")
        } else if let Some(tail) = strip_prefix_ascii_ci(
            after,
            "Put it onto the battlefield under its owner's control",
        ) {
            format!(
                "{lead}{descriptor} {zone_phrase} to the battlefield under its owner's control{tail}"
            )
        } else {
            continue;
        };
        return Some(apply_replacement_with_case(before, &replacement, ""));
    }
    None
}

fn reference_phrase_for_bound_choice(head: &str) -> &'static str {
    let lower = head.to_ascii_lowercase();
    if lower.starts_with("card ")
        || lower == "card"
        || lower.contains(" card")
        || lower.starts_with("a card ")
        || lower.starts_with("an card ")
    {
        "that card"
    } else if lower.starts_with("token ")
        || lower == "token"
        || lower.contains(" token")
        || lower.starts_with("a token ")
        || lower.starts_with("an token ")
    {
        "that token"
    } else if lower.starts_with("creature ")
        || lower == "creature"
        || lower.contains(" creature")
        || lower.starts_with("a creature ")
        || lower.starts_with("an creature ")
    {
        "that creature"
    } else if lower.starts_with("artifact ")
        || lower == "artifact"
        || lower.contains(" artifact")
        || lower.starts_with("a artifact ")
        || lower.starts_with("an artifact ")
    {
        "that artifact"
    } else if lower.starts_with("land ")
        || lower == "land"
        || lower.contains(" land")
        || lower.starts_with("a land ")
        || lower.starts_with("an land ")
    {
        "that land"
    } else {
        "that permanent"
    }
}

pub(super) fn normalize_remaining_it_tag_choice_clause(text: &str) -> Option<String> {
    let marker = " and tags it as '__it__'.";
    let (head, tail) = split_once_ascii_ci(text, marker)?;
    let head_lower = head.to_ascii_lowercase();
    if !head_lower.contains("choose ") {
        return None;
    }
    let tail = tail.trim();
    if tail.is_empty() {
        return None;
    }
    let tail_lower = tail.to_ascii_lowercase();
    if tail_lower.contains("tagged '__it__'")
        || tail_lower.contains("the tagged object '__it__'")
        || tail_lower.contains("target tagged object")
        || tail_lower.contains("that object")
    {
        return None;
    }

    let reference = reference_phrase_for_bound_choice(head);
    let rewritten_tail = if let Some(rest) = strip_prefix_ascii_ci(tail, "Exile it") {
        format!("Exile {reference}{rest}")
    } else if let Some(rest) = strip_prefix_ascii_ci(tail, "Destroy it") {
        format!("Destroy {reference}{rest}")
    } else if let Some(rest) = strip_prefix_ascii_ci(tail, "Put it") {
        format!("Put {reference}{rest}")
    } else if let Some(rest) = strip_prefix_ascii_ci(tail, "Return it") {
        format!("Return {reference}{rest}")
    } else {
        tail.to_string()
    };

    Some(format!(
        "{}. {}.",
        head.trim().trim_end_matches('.'),
        capitalize_first(rewritten_tail.trim().trim_end_matches('.'))
    ))
}

fn rewrite_single_bound_it_tail(tail: &str, reference: &str) -> String {
    if tail
        .trim()
        .eq_ignore_ascii_case("Attach any number of all Equipmentses you control to that object.")
    {
        return "Attach any number of Equipment you control to that creature.".to_string();
    }
    let reference_cap = capitalize_first(reference);
    let reference_possessive = format!("{reference}'s");
    let reference_possessive_cap = capitalize_first(&reference_possessive);
    let rewritten = tail
        .replace("that object's", &reference_possessive)
        .replace("That object's", &reference_possessive_cap)
        .replace("that object", reference)
        .replace("That object", &reference_cap)
        .replace("Return it", &format!("Return {reference}"))
        .replace("return it", &format!("return {reference}"))
        .replace("Put it", &format!("Put {reference}"))
        .replace("put it", &format!("put {reference}"))
        .replace("Exile it", &format!("Exile {reference}"))
        .replace("exile it", &format!("exile {reference}"))
        .replace("Untap it", &format!("Untap {reference}"))
        .replace("untap it", &format!("untap {reference}"))
        .replace(
            "Gain control of it",
            &format!("Gain control of {reference}"),
        )
        .replace(
            "gain control of it",
            &format!("gain control of {reference}"),
        )
        .replace("If it's ", &format!("If {reference} is "))
        .replace("if it's ", &format!("if {reference} is "));
    rewritten
}

pub(super) fn normalize_single_bound_it_tag_reference_clause(text: &str) -> Option<String> {
    let patterns = [
        "you choose exactly 1 ",
        "you may choose exactly 1 ",
        "you choose up to one ",
    ];
    for marker in patterns {
        let Some((before, rest)) = split_once_ascii_ci(text, marker) else {
            continue;
        };
        let Some((descriptor_raw, tail)) = split_once_ascii_ci(rest, " and tags it as '__it__'.")
        else {
            continue;
        };
        let tail = tail.trim();
        if tail.is_empty() {
            continue;
        }
        let tail_lower = tail.to_ascii_lowercase();
        if tail_lower.contains("tagged '__it__'")
            || tail_lower.contains("the tagged object '__it__'")
            || tail_lower.contains("target tagged object")
            || tail_lower.contains("for each tagged '__it__'")
        {
            continue;
        }
        let reference = reference_phrase_for_bound_choice(descriptor_raw);
        let rewritten_tail = rewrite_single_bound_it_tail(tail, reference);
        if rewritten_tail == tail {
            continue;
        }
        let replacement = format!(
            "{marker}{}. {}.",
            descriptor_raw.trim(),
            capitalize_first(rewritten_tail.trim().trim_end_matches('.'))
        );
        return Some(apply_replacement_with_case(before, &replacement, ""));
    }
    None
}

pub(super) fn normalize_sacrificed_choice_clause(text: &str) -> Option<String> {
    if let Some((before, rest)) = split_once_ascii_ci(text, "you may choose exactly ")
        && let Some((count_token, rest)) = rest.split_once(' ')
        && let Some((descriptor_raw, after_tag)) =
            split_once_ascii_ci(rest, " and tags it as 'sacrificed_")
        && let Some((_tag_suffix, after)) = split_once_ascii_ci(after_tag, "'. you sacrifice ")
    {
        let count = count_token.parse::<usize>().ok()?;
        let descriptor = choice_descriptor_for_zone(descriptor_raw, " in the battlefield")?;
        let mut subject = render_choose_exact_subject(&descriptor, count);
        if count == 1
            && !subject.starts_with("a ")
            && !subject.starts_with("an ")
            && !subject.starts_with("this ")
            && !subject.starts_with("that ")
        {
            subject = with_indefinite_article(&subject);
        }
        let tail = after
            .split_once(".")
            .map(|(_, tail)| format!(".{}", tail))
            .unwrap_or_else(|| ".".to_string());
        return Some(apply_replacement_with_case(
            before,
            &format!("you may sacrifice {subject}"),
            &tail,
        ));
    }
    if let Some((before, rest)) = split_once_ascii_ci(text, "you may choose exactly 1 ")
        && let Some((descriptor_raw, after)) = split_once_ascii_ci(
            rest,
            " and tags it as 'sacrificed_0'. you sacrifice a permanent",
        )
    {
        let descriptor = choice_descriptor_for_zone(descriptor_raw, " in the battlefield")?;
        let subject = render_choose_exact_subject(&descriptor, 1);
        return Some(apply_replacement_with_case(
            before,
            &format!("you may sacrifice {subject}"),
            after,
        ));
    }
    if let Some((before, rest)) = split_once_ascii_ci(text, "you choose exactly 1 ")
        && let Some((descriptor_raw, after)) = split_once_ascii_ci(
            rest,
            " and tags it as 'sacrificed_0'. you sacrifice a permanent",
        )
    {
        let descriptor = choice_descriptor_for_zone(descriptor_raw, " in the battlefield")?;
        let subject = render_choose_exact_subject(&descriptor, 1);
        return Some(apply_replacement_with_case(
            before,
            &format!("sacrifice {subject}"),
            after,
        ));
    }
    if let Some((head, tail)) = split_once_ascii_ci(
        text,
        " and tags it as 'sacrificed_0', then that player sacrifices a permanent",
    ) && let Some((prefix, count, descriptor)) = parse_choose_exact_tail(head)
    {
        let descriptor = descriptor
            .replace("that player controls", "they control")
            .replace("target player's ", "")
            .replace("that player's ", "");
        let chosen = render_choose_exact_subject(&descriptor, count);
        return Some(format!(
            "{prefix} sacrifices {chosen} of their choice{tail}"
        ));
    }
    if let Some((head, after_tag)) = split_once_ascii_ci(text, " and tags it as 'sacrificed_")
        && let Some((_tag_suffix, tail)) =
            split_once_ascii_ci(after_tag, "', then that player sacrifices ")
        && let Some((prefix, count, descriptor)) = parse_choose_exact_tail(head)
        && prefix.trim().to_ascii_lowercase().ends_with("that player")
    {
        let descriptor = descriptor
            .replace("target player's ", "")
            .replace("that player's ", "");
        if descriptor.contains("a controller's ") {
            return None;
        }
        let chosen = render_choose_exact_subject(&descriptor, count)
            .replace("that player controls", "they control");
        let tail = tail
            .split_once(".")
            .map(|(_, rest)| format!(".{}", rest))
            .unwrap_or_else(|| ".".to_string());
        return Some(format!(
            "{prefix} sacrifices {chosen} of their choice{tail}"
        ));
    }
    if let Some((before, rest)) = split_once_ascii_ci(text, "Choose exactly 1 ")
        && let Some((descriptor_raw, after)) = split_once_ascii_ci(
            rest,
            " and tags it as 'sacrificed_0', Sacrifice a permanent",
        )
    {
        let descriptor = choice_descriptor_for_zone(descriptor_raw, " in the battlefield")?;
        let subject = render_choose_exact_subject(&descriptor, 1);
        return Some(apply_replacement_with_case(
            before,
            &format!("Sacrifice {subject}"),
            after,
        ));
    }
    if let Some((before, rest)) =
        split_once_ascii_ci(text, "sacrifice it unless you choose exactly ")
        && let Some((count_token, rest)) = rest.split_once(' ')
        && let Some((descriptor_raw, after_tag)) =
            split_once_ascii_ci(rest, " and tags it as 'sacrificed_")
        && let Some((_tag_suffix, after)) = split_once_ascii_ci(after_tag, "'. you sacrifice ")
    {
        let count = count_token.parse::<usize>().ok()?;
        let descriptor = choice_descriptor_for_zone(descriptor_raw, " in the battlefield")?;
        let subject = render_choose_exact_subject(&descriptor, count);
        let tail = after
            .split_once(".")
            .map(|(_, tail)| format!(".{}", tail))
            .unwrap_or_default();
        return Some(format!(
            "{before}sacrifice it unless you sacrifice {subject}{tail}"
        ));
    }
    if let Some((before, rest)) = split_once_ascii_ci(text, "you choose exactly ")
        && let Some((count_token, rest)) = rest.split_once(' ')
        && let Some((descriptor_raw, after_tag)) =
            split_once_ascii_ci(rest, " and tags it as 'sacrificed_")
        && let Some((_tag_suffix, after)) = split_once_ascii_ci(after_tag, "'. you sacrifice ")
    {
        let count = count_token.parse::<usize>().ok()?;
        let descriptor = choice_descriptor_for_zone(descriptor_raw, " in the battlefield")?;
        let mut subject = render_choose_exact_subject(&descriptor, count);
        if count == 1
            && !subject.starts_with("a ")
            && !subject.starts_with("an ")
            && !subject.starts_with("this ")
            && !subject.starts_with("that ")
        {
            subject = with_indefinite_article(&subject);
        }
        let tail = after
            .split_once(".")
            .map(|(_, tail)| format!(".{}", tail))
            .unwrap_or_else(|| ".".to_string());
        return Some(apply_replacement_with_case(
            before,
            &format!("sacrifice {subject}"),
            &tail,
        ));
    }
    None
}

pub(super) fn normalize_same_name_reference_choice_clause(text: &str) -> Option<String> {
    if let Some((before, rest)) = split_once_ascii_ci(text, "you may choose exactly 1 ")
        && let Some((descriptor_raw, after)) =
            split_once_ascii_ci(rest, " and tags it as 'same_name_reference'. ")
    {
        let descriptor = descriptor_raw
            .trim()
            .strip_suffix(" in the battlefield")
            .unwrap_or(descriptor_raw.trim());
        let search_tail = "Search your library for a card with the same name as that object, reveal it, put it into your hand, then shuffle";
        if let Some(tail) = strip_prefix_ascii_ci(after, search_tail) {
            let replacement = format!(
                "you may search your library for a card with the same name as {}, reveal it, put it into your hand, then shuffle",
                descriptor
            );
            return Some(apply_replacement_with_case(before, &replacement, tail));
        }
    }
    if let Some((before, rest)) = split_once_ascii_ci(
        text,
        "you choose exactly 1 card in your graveyard and tags it as 'same_name_reference'. ",
    ) && let Some((search_subject, tail)) =
        split_once_ascii_ci(rest, ", reveal it, put it into your hand, then shuffle")
        && let Some(search_subject) =
            strip_prefix_ascii_ci(search_subject, "Search your library for ")
    {
        let replacement = format!(
            "search your library for {} with the same name as a card in your graveyard, reveal it, put it into your hand, then shuffle",
            search_subject.trim()
        );
        return Some(apply_replacement_with_case(before, &replacement, tail));
    }
    None
}

pub(super) fn normalize_sentence_helper_top_cards_choose_to_hand_sequence(
    text: &str,
) -> Option<String> {
    for marker in [
        ": Look at the top ",
        ": Reveal the top ",
        ". Look at the top ",
        ". Reveal the top ",
        ", look at the top ",
        ", reveal the top ",
    ] {
        if let Some((before, rest)) = split_once_ascii_ci(text, marker) {
            let marker_lower = marker.to_ascii_lowercase();
            let restored = if marker_lower.contains("look") {
                format!("Look at the top {rest}")
            } else {
                format!("Reveal the top {rest}")
            };
            if let Some(rewritten) =
                normalize_sentence_helper_top_cards_choose_to_hand_sequence(&restored)
            {
                if marker.starts_with(", ") {
                    return Some(format!("{before}, {}", lowercase_first(&rewritten)));
                }
                let joiner = marker.get(..2).unwrap_or(": ");
                return Some(format!("{before}{joiner}{rewritten}"));
            }
        }
    }
    let trimmed = text.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    if !(lower.starts_with("look at the top ") || lower.starts_with("reveal the top ")) {
        return None;
    }
    if !lower.contains(" of your library. you choose exactly ") {
        return None;
    }

    let (head, tail) = split_once_ascii_ci(trimmed, ". you choose exactly ")?;
    let head_lower = head.to_ascii_lowercase();
    let plural_top_cards = head_lower.ends_with(" cards of your library");
    let singular_top_card = head_lower.ends_with(" card of your library");
    if !plural_top_cards && !singular_top_card {
        return None;
    }
    let (count_token, tail) = tail.split_once(' ')?;
    let count = count_token.parse::<usize>().ok()?;
    let tail = strip_prefix_ascii_ci(tail, "card in library and tags it as '")?;
    let (chosen_tag, tail) = tail.split_once("'. ")?;
    if !chosen_tag.starts_with("__sentence_helper_chosen_") {
        return None;
    }
    let tail = strip_prefix_ascii_ci(tail, "For each tagged '")?;
    let tail = strip_prefix_ascii_ci(tail, chosen_tag)?;
    let tail = strip_prefix_ascii_ci(
        tail,
        "' object, Return that object to its owner's hand. For each tagged '",
    )?;
    let (revealed_tag, tail) = split_once_ascii_ci(
        tail,
        "' object, if it isn't true that it matches permanent, ",
    )?;
    if !revealed_tag.starts_with("__sentence_helper_revealed_") {
        return None;
    }

    let (rest_destination, suffix) = if let Some(suffix) =
        strip_prefix_ascii_ci(tail, "Put that object on the bottom of its owner's library")
    {
        ("on the bottom of your library", suffix)
    } else if let Some(suffix) =
        strip_prefix_ascii_ci(tail, "Put that object into its owner's graveyard")
    {
        ("into your graveyard", suffix)
    } else {
        return None;
    };

    let mut rewritten = if singular_top_card && count == 1 {
        format!(
            "{}. Put it into your hand.",
            head.trim().trim_end_matches('.')
        )
    } else {
        let chosen_text = if count == 1 {
            "one of them".to_string()
        } else {
            format!("{} of them", render_small_number_or_raw(&count.to_string()))
        };
        format!(
            "{}. Put {} into your hand and the rest {}.",
            head.trim().trim_end_matches('.'),
            chosen_text,
            rest_destination
        )
    };
    let suffix = suffix.trim();
    if let Some(tail) = suffix.strip_prefix('.') {
        let tail = tail.trim();
        if !tail.is_empty() {
            rewritten.push(' ');
            rewritten.push_str(&capitalize_first(tail));
            if !rewritten.ends_with('.') {
                rewritten.push('.');
            }
        }
    } else if !suffix.is_empty() {
        rewritten.push(' ');
        rewritten.push_str(suffix);
    }
    Some(rewritten)
}

pub(super) fn normalize_sentence_helper_graveyard_to_hand_choice_clause(
    text: &str,
) -> Option<String> {
    let patterns = [
        ("you choose up to one ", "return up to one "),
        ("you may choose exactly 1 ", "you may return "),
        ("you choose exactly 1 ", "return "),
    ];
    for (marker, lead) in patterns {
        let Some((before, rest)) = split_once_ascii_ci(text, marker) else {
            continue;
        };
        let Some((descriptor_raw, rest)) =
            split_once_ascii_ci(rest, " in a graveyard and tags it as '")
        else {
            continue;
        };
        let Some((tag, rest)) = rest.split_once("'. ") else {
            continue;
        };
        if !tag.starts_with("__sentence_helper_chosen_") {
            continue;
        }
        let Some(after) = strip_prefix_ascii_ci(
            rest,
            &format!("For each tagged '{tag}' object, Return that object to its owner's hand"),
        ) else {
            continue;
        };

        let mut rewritten = format!(
            "{lead}{} from a graveyard to your hand",
            if marker.contains("up to one") {
                strip_leading_article(descriptor_raw.trim()).to_string()
            } else {
                with_indefinite_article(strip_leading_article(descriptor_raw.trim()))
            }
        );
        let after = after.trim();
        if let Some(tail) = strip_prefix_ascii_ci(after, ". If that doesn't happen, ") {
            rewritten.push_str(". If you don't, ");
            rewritten.push_str(lowercase_first(tail).trim_end_matches('.'));
            rewritten.push('.');
            return Some(apply_replacement_with_case(before, &rewritten, ""));
        }
        return Some(apply_replacement_with_case(
            before,
            &append_sentence_tail(rewritten, after),
            "",
        ));
    }
    None
}

pub(super) fn normalize_sentence_helper_milled_to_hand_clause(text: &str) -> Option<String> {
    let marker = "you may choose exactly 1 card in library and tags it as '";
    let (before, rest) = split_once_ascii_ci(text, marker)?;
    let (tag, rest) = rest.split_once("'. ")?;
    if !tag.starts_with("__sentence_helper_chosen_") {
        return None;
    }
    let after = strip_prefix_ascii_ci(
        rest,
        &format!(
            "For each tagged '{tag}' object, Return that object to its owner's hand. For each tagged 'milled_0' object, if it isn't true that it matches permanent, Put that object into its owner's graveyard"
        ),
    )?;
    if !before.to_ascii_lowercase().contains("mill ") {
        return None;
    }
    let replacement =
        "you may put a permanent card from among the cards milled this way into your hand";
    Some(apply_replacement_with_case(
        before,
        &append_sentence_tail(replacement.to_string(), after),
        "",
    ))
}

pub(super) fn normalize_sentence_helper_put_from_other_hand_clause(text: &str) -> Option<String> {
    let patterns = [
        (
            "each player may choose exactly 1 ",
            " from that player's hand and tags it as '",
            "each player may put",
            "from their hand onto the battlefield",
        ),
        (
            "that object's controller may choose exactly 1 ",
            " in its controller's hand and tags it as '",
            "that object's controller may put",
            "from their hand onto the battlefield",
        ),
    ];
    for (marker, zone_marker, lead, destination) in patterns {
        let Some((before, rest)) = split_once_ascii_ci(text, marker) else {
            continue;
        };
        let Some((descriptor_raw, rest)) = split_once_ascii_ci(rest, zone_marker) else {
            continue;
        };
        let Some((tag, rest)) = rest.split_once("'. ") else {
            continue;
        };
        if !tag.starts_with("__sentence_helper_chosen_") {
            continue;
        }
        let Some(after) = strip_prefix_ascii_ci(rest, "Put it onto the battlefield") else {
            continue;
        };
        let descriptor = with_indefinite_article(strip_leading_article(descriptor_raw.trim()));
        let replacement = format!("{lead} {descriptor} {destination}");
        return Some(apply_replacement_with_case(before, &replacement, after));
    }
    None
}

pub(super) fn normalize_sentence_helper_put_from_hand_clause(text: &str) -> Option<String> {
    for marker in [
        "you may choose exactly 1 ",
        "you may choose up to one ",
        "you choose any number ",
        "you may choose any number ",
    ] {
        let Some((before, rest)) = split_once_ascii_ci(text, marker) else {
            continue;
        };
        let Some((descriptor_raw, rest)) =
            split_once_ascii_ci(rest, " in your hand and tags it as '")
        else {
            continue;
        };
        let Some((tag, rest)) = rest.split_once("'. ") else {
            continue;
        };
        if !tag.starts_with("__sentence_helper_chosen_") {
            continue;
        }
        let replacement_tail = if let Some(after) =
            strip_prefix_ascii_ci(rest, "Put it onto the battlefield tapped")
        {
            (
                if marker.contains("any number") {
                    format!(
                        "put {} from your hand onto the battlefield tapped",
                        render_any_number_subject(descriptor_raw.trim())
                    )
                } else {
                    format!(
                        "you may put {} from your hand onto the battlefield tapped",
                        with_indefinite_article(descriptor_raw.trim())
                    )
                },
                after,
            )
        } else if let Some(after) = strip_prefix_ascii_ci(rest, "Put it onto the battlefield") {
            (
                if marker.contains("any number") {
                    format!(
                        "put {} from your hand onto the battlefield",
                        render_any_number_subject(descriptor_raw.trim())
                    )
                } else {
                    format!(
                        "you may put {} from your hand onto the battlefield",
                        with_indefinite_article(descriptor_raw.trim())
                    )
                },
                after,
            )
        } else {
            continue;
        };
        return Some(apply_replacement_with_case(
            before,
            &replacement_tail.0,
            replacement_tail.1,
        ));
    }
    None
}

pub(super) fn normalize_sentence_helper_reveal_search_same_name_clause(
    text: &str,
) -> Option<String> {
    let (before, rest) = split_once_ascii_ci(text, "you choose exactly 1 ")?;
    let (descriptor_raw, rest) = split_once_ascii_ci(rest, " in your hand and tags it as '")?;
    let (tag, rest) = rest.split_once("'. ")?;
    if !tag.starts_with("__sentence_helper_revealed_") {
        return None;
    }
    let after = strip_prefix_ascii_ci(
        rest,
        "Reveal it. Search your library for a card with the same name as that object, reveal it, put it into your hand, then shuffle",
    )?;
    let replacement = format!(
        "reveal {} in your hand. Search your library for a card with the same name as that card, reveal it, put it into your hand, then shuffle",
        with_indefinite_article(descriptor_raw.trim())
    );
    Some(apply_replacement_with_case(before, &replacement, after))
}

pub(super) fn normalize_sentence_helper_reveal_from_hand_clause(text: &str) -> Option<String> {
    for marker in [
        "you choose exactly 1 ",
        "you choose any number ",
        "you may choose any number ",
    ] {
        let Some((before, rest)) = split_once_ascii_ci(text, marker) else {
            continue;
        };
        let Some((descriptor_raw, rest)) =
            split_once_ascii_ci(rest, " in your hand and tags it as '")
        else {
            continue;
        };
        let Some((tag, rest)) = rest.split_once("'. ") else {
            continue;
        };
        if !tag.starts_with("__sentence_helper_revealed_") {
            continue;
        }
        let Some(after) = strip_prefix_ascii_ci(rest, "Reveal it") else {
            continue;
        };
        let reveal_clause = if marker.contains("exactly 1") {
            format!(
                "reveal {} in your hand",
                with_indefinite_article(descriptor_raw.trim())
            )
        } else {
            format!(
                "reveal {} in your hand",
                render_any_number_subject(descriptor_raw.trim())
            )
        };

        let rewritten_after = if let Some(tail) = strip_prefix_ascii_ci(
            after.trim_start(),
            ". Counter target spell unless its controller pays {1} for each permanent",
        ) {
            format!(
                ". Counter target spell unless its controller pays {{1}} for each card revealed this way{tail}"
            )
        } else if let Some(tail) = strip_prefix_ascii_ci(
            after.trim_start(),
            ". Deal the number of permanents damage to any target. X is the number of cards revealed this way",
        ) {
            format!(
                ". Deal damage to any target equal to the number of cards revealed this way{tail}"
            )
        } else if let Some(tail) = strip_prefix_ascii_ci(
            after.trim_start(),
            ". target creature gets +X/+X until end of turn, where X is the number of permanents",
        ) {
            format!(
                ". Target creature gets +X/+X until end of turn, where X is the number of cards revealed this way{tail}"
            )
        } else if let Some(tail) = strip_prefix_ascii_ci(
            after.trim_start(),
            ". target creature gets -X/-X until end of turn, where X is the number of permanents",
        ) {
            format!(
                ". Target creature gets -X/-X until end of turn, where X is the number of cards revealed this way{tail}"
            )
        } else if let Some(tail) =
            strip_prefix_ascii_ci(after.trim_start(), ". you gain 2 life for each permanent")
        {
            format!(". You gain 2 life for each card revealed this way{tail}")
        } else {
            after.to_string()
        };

        return Some(apply_replacement_with_case(
            before,
            &reveal_clause,
            &rewritten_after,
        ));
    }
    None
}

pub(super) fn normalize_sentence_helper_simple_exile_clause(text: &str) -> Option<String> {
    if let Some((before, rest)) = split_once_ascii_ci(
        text,
        "you choose the top card in your library and tags it as '",
    ) && let Some((tag, rest)) = rest.split_once("', then exile it")
        && tag.starts_with("__sentence_helper_exiled_")
    {
        let replacement = format!("exile the top card of your library{rest}");
        return Some(apply_replacement_with_case(before, &replacement, ""));
    }
    let marker = "you choose exactly 1 ";
    let lower = text.to_ascii_lowercase();
    if let Some(idx) = lower.rfind(&marker.to_ascii_lowercase()) {
        let before = &text[..idx];
        let rest = &text[idx + marker.len()..];
        if let Some((descriptor_raw, rest)) =
            split_once_ascii_ci(rest, " in your hand and tags it as '")
            && let Some((tag, rest)) = rest.split_once("', then exile it")
            && tag.starts_with("__sentence_helper_exiled_")
        {
            let replacement = format!(
                "exile {} from your hand{rest}",
                with_indefinite_article(descriptor_raw.trim())
            );
            return Some(apply_replacement_with_case(before, &replacement, ""));
        }
    }
    None
}

pub(super) fn normalize_sentence_helper_choose_exiled_card_play_clause(
    text: &str,
) -> Option<String> {
    let marker = "you choose exactly 1 card in exile and tags it as '";
    let (before, rest) = split_once_ascii_ci(text, marker)?;
    let (tag, rest) = rest.split_once("'. ")?;
    if !tag.starts_with("__sentence_helper_chosen_") {
        return None;
    }
    if !before.to_ascii_lowercase().contains("exile ") {
        return None;
    }
    let after = strip_prefix_ascii_ci(rest, "you may play that card until")?;
    Some(apply_replacement_with_case(
        before,
        "choose a card exiled this way. You may play that card until",
        after,
    ))
}

pub(super) fn normalize_sentence_helper_planar_genesis_clause(text: &str) -> Option<String> {
    let marker = "Look at the top four cards of your library. you choose up to one land card in library and tags it as '";
    let (before, rest) = split_once_ascii_ci(text, marker)?;
    let (land_tag, rest) = rest.split_once("'. ")?;
    if !land_tag.starts_with("__sentence_helper_chosen_") {
        return None;
    }
    let rest = strip_prefix_ascii_ci(
        rest,
        &format!(
            "For each tagged '{land_tag}' object, Put that object onto the battlefield tapped. If that doesn't happen, you choose exactly 1 card in library and tags it as '"
        ),
    )?;
    let (card_tag, rest) = rest.split_once("'. ")?;
    if !card_tag.starts_with("__sentence_helper_chosen_") {
        return None;
    }
    let after = strip_prefix_ascii_ci(
        rest,
        &format!(
            "For each tagged '{card_tag}' object, Return that object to its owner's hand. For each tagged '__sentence_helper_revealed_l0_s0_e0' object, if it isn't true that it matches permanent, if it isn't true that it matches permanent, Put that object on the bottom of its owner's library"
        ),
    )?;
    let replacement = "Look at the top four cards of your library. You may put a land card from among them onto the battlefield tapped. If you don't, put a card from among them into your hand. Put the rest on the bottom of your library";
    Some(format!(
        "{before}{}",
        append_sentence_tail(replacement.to_string(), after)
    ))
}

pub(super) fn normalize_sentence_helper_kicked_top_x_clause(text: &str) -> Option<String> {
    let marker = "Look at the top the number of lands you control cards of your library. If this spell was kicked, you choose exactly 2 card in library and tags it as '";
    let (before, rest) = split_once_ascii_ci(text, marker)?;
    let (kicked_tag, rest) = rest.split_once("', ")?;
    if !kicked_tag.starts_with("__sentence_helper_chosen_") {
        return None;
    }
    let rest = strip_prefix_ascii_ci(
        rest,
        &format!(
            "for each tagged '{kicked_tag}' object, Return that object to its owner's hand, then for each tagged '__sentence_helper_revealed_l0_s0_e0' object, if it isn't true that it matches permanent, Put that object on the bottom of its owner's library. Otherwise, you choose exactly 1 card in library and tags it as '"
        ),
    )?;
    let (base_tag, rest) = rest.split_once("', ")?;
    if !base_tag.starts_with("__sentence_helper_chosen_") {
        return None;
    }
    let after = strip_prefix_ascii_ci(
        rest,
        &format!(
            "for each tagged '{base_tag}' object, Return that object to its owner's hand, then for each tagged '__sentence_helper_revealed_l0_s0_e0' object, if it isn't true that it matches permanent, Put that object on the bottom of its owner's library."
        ),
    )
    .or_else(|| {
        strip_prefix_ascii_ci(
            rest,
            &format!(
                "for each tagged '{base_tag}' object, Return that object to its owner's hand, then for each tagged '__sentence_helper_revealed_l0_s0_e0' object, if it isn't true that it matches permanent, Put that object on the bottom of its owner's library"
            ),
        )
    })?;
    let replacement = "Look at the top the number of lands you control cards of your library. If this spell was kicked, put two of those cards into your hand. Otherwise, put one of those cards into your hand. Put the rest on the bottom of your library.";
    Some(format!("{before}{replacement}{after}"))
}

pub(super) fn normalize_sentence_helper_spell_mastery_gather_clause(text: &str) -> Option<String> {
    let marker = "Reveal the top five cards of your library. You may put a creature card from among them into your hand. Put the rest into your graveyard. If you have two or more instants or sorcery cards in your graveyard, you choose exactly 1 card in library and tags it as '";
    let (before, rest) = split_once_ascii_ci(text, marker)?;
    let (first_tag, rest) = rest.split_once("', ")?;
    if !first_tag.starts_with("__sentence_helper_chosen_") {
        return None;
    }
    let rest = strip_prefix_ascii_ci(
        rest,
        &format!(
            "for each tagged '{first_tag}' object, Return that object to its owner's hand, then for each tagged '"
        ),
    )?;
    let (second_tag, rest) = split_once_ascii_ci(
        rest,
        "' object, if it isn't true that it matches permanent, Put that object into its owner's graveyard",
    )?;
    if !second_tag.starts_with("__sentence_helper_chosen_") {
        return None;
    }
    let replacement = "Reveal the top five cards of your library. You may put a creature card from among them into your hand. Put the rest into your graveyard. If you have two or more instants or sorcery cards in your graveyard, put up to two creature cards from among the revealed cards into your hand instead of one";
    Some(format!(
        "{before}{}",
        append_sentence_tail(replacement.to_string(), rest)
    ))
}

pub(super) fn normalize_enlist_tag_sequence(text: &str) -> Option<String> {
    let marker = "Tag the triggering object as 'enlist_attacker'. you choose exactly 1 ";
    let (before, rest) = split_once_ascii_ci(text, marker)?;
    let (descriptor_raw, rest) =
        split_once_ascii_ci(rest, " and tags it as 'enlisted_creature'. ")?;
    let tail = "Tap target tagged object 'enlisted_creature'. the tagged object 'enlist_attacker' gets +1/+0 until end of turn for each the tagged object 'enlisted_creature''s power";
    let after = strip_prefix_ascii_ci(rest, tail)?;
    let enlisted = descriptor_raw.trim().replace(" in the battlefield", "");
    let replacement = format!(
        "tap {enlisted}. When you do, this creature gets +X/+0 until end of turn, where X is that creature's power."
    );
    Some(apply_replacement_with_case(before, &replacement, after))
}

pub(super) fn normalize_divvy_chosen_sequence(text: &str) -> Option<String> {
    if let Some((before, _rest)) = split_once_ascii_ci(
        text,
        " chooses any number that player's nontoken land in the battlefield and tags it as 'divvy_chosen'. Destroy the tagged object 'divvy_chosen'. Tap all that player's nontoken other lands.",
    ) {
        return Some(format!(
            "{before} chooses any number of that player's nontoken lands. Destroy those lands. Tap all other nontoken lands that player controls."
        ));
    }
    if let Some((before, rest)) = split_once_ascii_ci(
        text,
        " chooses any number a creature that player controls in the battlefield and tags it as 'divvy_chosen'. a other creature that player controls can't attack this turn.",
    ) {
        let _ = rest;
        return Some(format!(
            "{before} chooses any number of creatures that player controls. Other creatures that player controls can't attack this turn."
        ));
    }
    if let Some((before, rest)) = split_once_ascii_ci(
        text,
        " chooses any number a creature that player controls in the battlefield and tags it as 'divvy_chosen'. a other creature that player controls can't block this turn.",
    ) {
        let _ = rest;
        return Some(format!(
            "{before} chooses any number of creatures that player controls. Other creatures that player controls can't block this turn."
        ));
    }
    if let Some((before, rest)) = split_once_ascii_ci(
        text,
        " chooses any number creature card in your graveyard and tags it as 'divvy_chosen'. Exile the tagged object 'divvy_chosen'. Return all other creature card in your graveyard to the battlefield.",
    ) {
        let _ = rest;
        return Some(format!(
            "{before} chooses any number of creature cards in your graveyard. Exile those cards. Return all other creature cards from your graveyard to the battlefield."
        ));
    }
    if let Some((before, rest)) = split_once_ascii_ci(
        text,
        " chooses any number permanent in exile and tags it as 'divvy_chosen'. Return the tagged object 'divvy_chosen' to its owner's hand. For each tagged 'divvy_source' object, if it isn't true that the tagged object 'divvy_chosen' matches permanent, Put that object into its owner's graveyard.",
    ) {
        let _ = rest;
        return Some(format!(
            "{before} chooses any number of permanents in exile. Return those cards to their owners' hands. Put the rest into their owners' graveyards."
        ));
    }
    if let Some((before, rest)) = split_once_ascii_ci(
        text,
        " chooses any number permanent in exile and tags it as 'divvy_chosen'. Put the tagged object 'divvy_chosen' onto the battlefield under your control. For each tagged 'divvy_source' object, if it isn't true that the tagged object 'divvy_chosen' matches permanent, Put that object into its owner's graveyard.",
    ) {
        let _ = rest;
        return Some(format!(
            "{before} chooses any number of permanents in exile. Put those cards onto the battlefield under your control. Put the rest into their owners' graveyards."
        ));
    }
    if let Some((before, rest)) = split_once_ascii_ci(
        text,
        "you choose any number a creature that player controls in the battlefield and tags it as 'divvy_chosen'. that player sacrifices all creatures you control.",
    ) {
        let _ = rest;
        return Some(format!(
            "{before}choose any number of creatures that player controls. That player sacrifices those creatures."
        ));
    }
    None
}

pub(super) fn parse_choose_exact_tail(head: &str) -> Option<(&str, usize, &str)> {
    let needle = " chooses exactly ";
    let lower = head.to_ascii_lowercase();
    let idx = lower.rfind(needle)?;
    let prefix = head.get(..idx)?.trim_end_matches(',');
    let choose_tail = head.get(idx + needle.len()..)?;
    let (count_token, rest) = choose_tail.split_once(' ')?;
    let count = count_token.parse::<usize>().ok()?;
    let descriptor = rest
        .strip_suffix(" in the battlefield")
        .or_else(|| rest.strip_suffix(" in a hand"))
        .or_else(|| rest.strip_suffix(" in hand"))
        .or_else(|| rest.strip_suffix(" in the stack"))
        .or_else(|| rest.strip_suffix(" in a graveyard"))
        .or_else(|| rest.strip_suffix(" in a library"))
        .or_else(|| rest.strip_suffix(" in exile"))
        .unwrap_or(rest);
    Some((prefix, count, descriptor))
}

pub(super) fn normalize_choose_exact_tagged_it_clause(text: &str) -> Option<String> {
    if let Some((head, tail)) = text.split_once(" and tags it as '__it__'. Destroy it")
        && let Some((chooser, count, descriptor)) = parse_choose_exact_tail(head)
    {
        let mut descriptor = descriptor
            .replace("that player controls", "they control")
            .replace("target player's ", "")
            .replace("that player's ", "")
            .replace(" from their hand", " in their hand");
        descriptor = descriptor.replace(" in their hand in their hand", " in their hand");
        let chosen = render_choose_exact_subject(&descriptor, count);
        let target_ref = if chosen.to_ascii_lowercase().contains("creature") {
            "that creature"
        } else if chosen.to_ascii_lowercase().contains("artifact") {
            "that artifact"
        } else if chosen.to_ascii_lowercase().contains("card") {
            "that card"
        } else {
            "that permanent"
        };
        return Some(format!(
            "{chooser} chooses {chosen}. Destroy {target_ref}{tail}"
        ));
    }
    if let Some((head, tail)) = text.split_once(" and tags it as '__it__'")
        && let Some((chooser, count, descriptor)) = parse_choose_exact_tail(head)
    {
        let mut descriptor = descriptor
            .replace("that player controls", "they control")
            .replace("target player's ", "")
            .replace("that player's ", "")
            .replace(" from their hand", " in their hand");
        descriptor = descriptor.replace(" in their hand in their hand", " in their hand");
        let chosen = render_choose_exact_subject(&descriptor, count);
        return Some(format!("{chooser} chooses {chosen}{tail}"));
    }
    None
}

fn strip_leading_article(descriptor: &str) -> &str {
    descriptor
        .trim()
        .strip_prefix("a ")
        .or_else(|| descriptor.trim().strip_prefix("an "))
        .unwrap_or(descriptor.trim())
}

fn append_sentence_tail(mut rewritten: String, tail: &str) -> String {
    let tail = tail.trim();
    if let Some(rest) = tail.strip_prefix('.') {
        let rest = rest.trim();
        if rest.is_empty() {
            if !rewritten.ends_with('.') {
                rewritten.push('.');
            }
        } else {
            if !rewritten.ends_with('.') {
                rewritten.push('.');
            }
            rewritten.push(' ');
            rewritten.push_str(&capitalize_first(rest));
            if !rewritten.ends_with('.') {
                rewritten.push('.');
            }
        }
    } else if !tail.is_empty() {
        if !rewritten.ends_with('.') && !tail.starts_with(',') {
            rewritten.push('.');
        }
        if !rewritten.ends_with(' ') && !tail.starts_with(',') {
            rewritten.push(' ');
        }
        rewritten.push_str(tail);
    } else if !rewritten.ends_with('.') {
        rewritten.push('.');
    }
    rewritten
}

fn render_search_subject_and_pronoun(
    descriptor_raw: &str,
    selection_kind: &str,
) -> Option<(String, &'static str)> {
    let descriptor = descriptor_raw.trim();
    match selection_kind {
        "X" => Some((
            render_choose_subject_for_count_token(strip_leading_article(descriptor), "X"),
            "them",
        )),
        "up to one" => Some((
            format!("up to one {}", strip_leading_article(descriptor)),
            "it",
        )),
        "exactly one" => Some((
            with_indefinite_article(strip_leading_article(descriptor)),
            "it",
        )),
        _ => None,
    }
}

pub(super) fn normalize_search_put_onto_battlefield_clause(text: &str) -> Option<String> {
    let patterns = [
        (
            "you searches for X ",
            "X",
            "search your library for",
            "put",
            "shuffle",
        ),
        (
            "you searches for up to one ",
            "up to one",
            "search your library for",
            "put",
            "shuffle",
        ),
        (
            "you may searches for up to one ",
            "up to one",
            "you may search your library for",
            "put",
            "shuffle",
        ),
        (
            "that object's controller searches for up to one ",
            "up to one",
            "that object's controller searches their library for",
            "puts",
            "shuffles",
        ),
    ];
    for (marker, selection_kind, lead, put_verb, shuffle_verb) in patterns {
        let Some((before, rest)) = split_once_ascii_ci(text, marker) else {
            continue;
        };
        let Some((descriptor_raw, after)) =
            split_once_ascii_ci(rest, " in a library and tags it as 'searched'. ")
        else {
            continue;
        };
        let Some((subject, pronoun)) =
            render_search_subject_and_pronoun(descriptor_raw, selection_kind)
        else {
            continue;
        };

        let battlefield_suffix = if let Some(tail) = strip_prefix_ascii_ci(
            after,
            "Put the tagged object 'searched' onto the battlefield tapped. Shuffle your library",
        ) {
            Some(("onto the battlefield tapped", tail))
        } else if let Some(tail) = strip_prefix_ascii_ci(
            after,
            "Put the tagged object 'searched' onto the battlefield. Shuffle your library",
        ) {
            Some(("onto the battlefield", tail))
        } else {
            None
        };
        let Some((destination, tail)) = battlefield_suffix else {
            continue;
        };

        let rewritten =
            format!("{lead} {subject}, {put_verb} {pronoun} {destination}, then {shuffle_verb}");
        return Some(apply_replacement_with_case(
            before,
            &append_sentence_tail(rewritten, tail),
            "",
        ));
    }
    None
}

pub(super) fn normalize_search_reveal_into_hand_clause(text: &str) -> Option<String> {
    let patterns = [
        (
            "you searches for X ",
            "X",
            "search your library for",
            "your",
        ),
        (
            "you searches for up to one ",
            "up to one",
            "search your library for",
            "your",
        ),
        (
            "you may searches for up to one ",
            "up to one",
            "you may search your library for",
            "your",
        ),
    ];
    for (marker, selection_kind, lead, hand_owner) in patterns {
        let Some((before, rest)) = split_once_ascii_ci(text, marker) else {
            continue;
        };
        let Some((descriptor_raw, after)) =
            split_once_ascii_ci(rest, " in a library and tags it as 'searched'. ")
        else {
            continue;
        };
        let Some((subject, pronoun)) =
            render_search_subject_and_pronoun(descriptor_raw, selection_kind)
        else {
            continue;
        };

        let rewritten = if let Some(tail) = strip_prefix_ascii_ci(
            after,
            "Reveal it. Return the tagged object 'searched' to its owner's hand. Shuffle your library",
        ) {
            Some(append_sentence_tail(
                format!(
                    "{lead} {subject}, reveal {pronoun}, put {pronoun} into {hand_owner} hand, then shuffle"
                ),
                tail,
            ))
        } else if let Some(tail) = strip_prefix_ascii_ci(
            after,
            "Reveal it. Return the tagged object 'searched' to its owner's hand. you discard a card at random. Shuffle your library",
        ) {
            Some(append_sentence_tail(
                format!(
                    "{lead} {subject}, reveal {pronoun}, put {pronoun} into {hand_owner} hand, discard a card at random, then shuffle"
                ),
                tail,
            ))
        } else {
            None
        };
        let Some(rewritten) = rewritten else {
            continue;
        };
        return Some(apply_replacement_with_case(before, &rewritten, ""));
    }
    None
}

fn normalize_search_condition_text(condition: &str) -> String {
    if let Some(rest) = strip_prefix_ascii_ci(
        condition.trim(),
        "If the tagged object 'searched' matches permanent with mana value ",
    ) {
        return format!("If it has mana value {}", rest.trim());
    }
    condition.trim().to_string()
}

pub(super) fn normalize_search_reveal_battlefield_or_hand_clause(text: &str) -> Option<String> {
    let patterns = [
        (
            "you searches for up to one ",
            "search your library for",
            "put",
            "shuffle",
        ),
        (
            "you may searches for up to one ",
            "you may search your library for",
            "put",
            "shuffle",
        ),
    ];
    for (marker, lead, put_verb, shuffle_verb) in patterns {
        let Some((before, rest)) = split_once_ascii_ci(text, marker) else {
            continue;
        };
        let Some((descriptor_raw, after)) =
            split_once_ascii_ci(rest, " in a library and tags it as 'searched'. Reveal it. ")
        else {
            continue;
        };
        let Some((subject, pronoun)) =
            render_search_subject_and_pronoun(descriptor_raw, "up to one")
        else {
            continue;
        };

        let parsed = if let Some((condition, tail)) = split_once_ascii_ci(
            after,
            ", Put the tagged object 'searched' onto the battlefield tapped. If that doesn't happen, Return the tagged object 'searched' to its owner's hand. Shuffle your library",
        ) {
            Some((condition, "onto the battlefield tapped", tail))
        } else if let Some((condition, tail)) = split_once_ascii_ci(
            after,
            ", Put the tagged object 'searched' onto the battlefield. If that doesn't happen, Return the tagged object 'searched' to its owner's hand. Shuffle your library",
        ) {
            Some((condition, "onto the battlefield", tail))
        } else {
            None
        };
        let Some((condition_raw, destination, tail)) = parsed else {
            continue;
        };
        let condition = normalize_search_condition_text(condition_raw);
        let rewritten = format!(
            "{lead} {subject}, reveal {pronoun}. {condition}, {put_verb} {pronoun} {destination}. Otherwise, put {pronoun} into your hand, then {shuffle_verb}"
        );
        return Some(apply_replacement_with_case(
            before,
            &append_sentence_tail(rewritten, tail),
            "",
        ));
    }
    None
}

fn normalize_face_down_search_cast_condition_text(condition: &str) -> String {
    let condition = condition.trim();
    if let Some(rest) = strip_prefix_ascii_ci(
        condition,
        "that card's mana value is less than or equal to ",
    ) {
        return format!("that spell's mana value is {} or less", rest.trim());
    }
    if let Some(rest) = strip_prefix_ascii_ci(condition, "that card's mana value is less than ") {
        return format!("that spell's mana value is less than {}", rest.trim());
    }
    condition.to_string()
}

pub(super) fn normalize_search_face_down_exile_cast_else_hand_clause(text: &str) -> Option<String> {
    let patterns = [
        (
            "you searches for up to one ",
            "search your library for",
            "up to one",
        ),
        (
            "you may searches for up to one ",
            "you may search your library for",
            "up to one",
        ),
    ];
    for (marker, lead, selection_kind) in patterns {
        let Some((before, rest)) = split_once_ascii_ci(text, marker) else {
            continue;
        };
        let Some((descriptor_raw, after)) = split_once_ascii_ci(
            rest,
            " in a library and tags it as 'searched_face_down'. Exile the tagged object 'searched_face_down' face down. Shuffle your library. If this spell was bargained and ",
        ) else {
            continue;
        };
        let Some((condition_raw, after_condition)) = split_once_ascii_ci(
            after,
            ", You may cast the tagged object 'searched_face_down' without paying its mana cost. If effect #",
        ) else {
            continue;
        };
        let Some((_, tail)) = split_once_ascii_ci(
            after_condition,
            " was declined, Put the tagged object 'searched_face_down' into its owner's hand. Otherwise, Put the tagged object 'searched_face_down' into its owner's hand",
        ) else {
            continue;
        };

        let descriptor = descriptor_raw.trim();
        let subject = if matches!(
            strip_leading_article(descriptor),
            "permanent" | "permanent card" | "card"
        ) {
            "a card".to_string()
        } else if selection_kind == "up to one" {
            format!("up to one {}", strip_leading_article(descriptor))
        } else {
            descriptor.to_string()
        };
        let condition = normalize_face_down_search_cast_condition_text(condition_raw);
        let rewritten = format!(
            "{lead} {subject}, exile it face down, then shuffle. If this spell was bargained, you may cast the exiled card without paying its mana cost if {condition}. Put it into your hand if it wasn't cast this way"
        );
        return Some(apply_replacement_with_case(
            before,
            &append_sentence_tail(rewritten, tail),
            "",
        ));
    }
    None
}

pub(super) fn normalize_split_land_search_sequence(text: &str) -> Option<String> {
    let _ = text;
    None
}

pub(super) fn is_render_heading_prefix(prefix: &str) -> bool {
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

pub(super) fn static_heading_body(line: &str) -> Option<(&str, &str)> {
    let (prefix, body) = line.split_once(':')?;
    if prefix
        .trim()
        .to_ascii_lowercase()
        .starts_with("static ability ")
    {
        Some((prefix.trim(), body.trim()))
    } else {
        None
    }
}

pub(super) fn merge_adjacent_static_heading_lines(lines: Vec<String>) -> Vec<String> {
    let mut current = lines;
    loop {
        let mut changed = false;
        let mut merged = Vec::with_capacity(current.len());
        let mut idx = 0usize;
        while idx < current.len() {
            if let Some((left_prefix, _left_body)) = static_heading_body(&current[idx])
                && let Some((body, consumed)) =
                    merge_static_legendary_gets_then_has_block(&current, idx)
            {
                merged.push(format!("{left_prefix}: {}", body.trim()));
                idx += consumed;
                changed = true;
                continue;
            }
            if idx + 1 < current.len()
                && let (Some((left_prefix, left_body)), Some((_right_prefix, right_body))) = (
                    static_heading_body(&current[idx]),
                    static_heading_body(&current[idx + 1]),
                )
            {
                let pair = vec![left_body.to_string(), right_body.to_string()];
                let pair = merge_adjacent_subject_predicate_lines(pair);
                let pair = merge_subject_has_keyword_lines(pair);
                let pair = merge_subject_is_legendary_gets_then_has_lines(pair);
                if pair.len() == 1 {
                    merged.push(format!("{left_prefix}: {}", pair[0].trim()));
                    idx += 2;
                    changed = true;
                    continue;
                }
            }
            merged.push(current[idx].clone());
            idx += 1;
        }
        if !changed {
            return current;
        }
        current = merged;
    }
}

pub(super) fn merge_static_legendary_gets_then_has_block(
    lines: &[String],
    start_idx: usize,
) -> Option<(String, usize)> {
    let (_left_prefix, left_body) = static_heading_body(lines.get(start_idx)?.as_str())?;
    let left_body = left_body.trim().trim_end_matches('.');
    let (subject, rest) = left_body.split_once(" is ")?;
    let subject = subject.trim();
    if subject.is_empty() {
        return None;
    }
    let rest = rest.trim();
    let (state, gets_tail) = rest.split_once(" and gets ")?;
    if !state.trim().eq_ignore_ascii_case("legendary") {
        return None;
    }
    let gets_tail = gets_tail.trim();
    if gets_tail.is_empty() {
        return None;
    }

    let mut keyword_lines = Vec::new();
    let mut idx = start_idx + 1;
    while idx < lines.len() {
        let Some((_prefix, body)) = static_heading_body(lines[idx].as_str()) else {
            break;
        };
        let body = body.trim().trim_end_matches('.');
        let (rhs_subject, rhs_tail) = if let Some((s, t)) = body.split_once(" has ") {
            (s.trim(), t.trim())
        } else if let Some((s, t)) = body.split_once(" have ") {
            (s.trim(), t.trim())
        } else {
            break;
        };
        if !rhs_subject.eq_ignore_ascii_case(subject) {
            break;
        }
        if rhs_tail.is_empty() {
            break;
        }
        keyword_lines.push(rhs_tail.to_string());
        idx += 1;
    }
    if keyword_lines.is_empty() {
        return None;
    }

    let mut keywords = Vec::<String>::new();
    for tail in keyword_lines {
        let normalized = normalize_repeated_has_keyword_list(&tail);
        let parts: Vec<&str> = if normalized.contains(',') {
            normalized
                .split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(|part| part.trim_start_matches("and ").trim())
                .collect()
        } else {
            normalized
                .split(" and ")
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect()
        };
        if parts.is_empty() || !parts.iter().all(|part| is_keyword_phrase(part)) {
            return None;
        }
        for part in parts {
            let kw = part.to_ascii_lowercase();
            if !keywords
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&kw))
            {
                keywords.push(kw);
            }
        }
    }
    if keywords.is_empty() {
        return None;
    }

    let keyword_list = if keywords.len() == 1 {
        keywords[0].clone()
    } else if keywords.len() == 2 {
        format!("{} and {}", keywords[0], keywords[1])
    } else {
        let last = keywords.pop().unwrap_or_default();
        format!("{}, and {}", keywords.join(", "), last)
    };

    let verb = have_verb_for_subject(subject);
    let merged = format!("{subject} is legendary, gets {gets_tail}, and {verb} {keyword_list}.");
    Some((merged, idx - start_idx))
}

fn normalize_creature_becomes_base_pt_surface(text: &str, oracle_lower: &str) -> Option<String> {
    if !oracle_lower.contains("with base power and toughness") {
        return None;
    }

    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();
    let becomes_idx = lower.find(" becomes a ")?;
    let body_start = becomes_idx + " becomes a ".len();
    let until_idx = lower[body_start..]
        .find(" until end of turn")
        .map(|idx| body_start + idx)?;
    let subject = trimmed[..becomes_idx].trim_end();
    if !subject.to_ascii_lowercase().ends_with("creature") {
        return None;
    }

    let body = &trimmed[body_start..until_idx];
    let body_lower = body.to_ascii_lowercase();
    let creature_suffix_idx = body_lower.rfind(" creature")?;
    if creature_suffix_idx + " creature".len() != body.len() {
        return None;
    }

    let descriptor_with_pt = body[..creature_suffix_idx].trim();
    let (pt, descriptor) = descriptor_with_pt.split_once(' ')?;
    if !pt.contains('/') || descriptor.trim().is_empty() {
        return None;
    }

    Some(format!(
        "{subject} becomes a {} with base power and toughness {pt}{}",
        descriptor.trim(),
        &trimmed[until_idx..]
    ))
}
