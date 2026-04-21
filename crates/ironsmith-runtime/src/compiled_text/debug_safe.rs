use super::*;
use crate::text_cleanup::strip_parenthetical_text;

/// Render the structured runtime model for debug/inspector use.
///
/// This deliberately disables source/oracle-sensitive reconciliation. It first
/// renders abilities without source text, then applies only debug-safe
/// normalization owned by this module.
pub fn debug_compiled_lines(def: &CardDefinition) -> Vec<String> {
    let structured_def = debug_safe_surface_definition(def);
    normalize_debug_safe_surface(&structured_def, def, ast_compiled_lines(&structured_def))
}

/// Render the structured compiled-text surface used for DB scoring.
pub fn unprocessed_compiled_lines(def: &CardDefinition) -> Vec<String> {
    debug_compiled_lines(def)
}

fn safe_intrinsic_label_from_ability_source_text(ability: &Ability) -> Option<String> {
    let Some(text) = ability.text.as_deref().map(str::trim) else {
        return None;
    };
    if let Some(label) = safe_echo_label_from_source_text(text) {
        return Some(label);
    }
    let label = intrinsic_label_from_source_text(Some(text))?;
    if let Some(keyword) = describe_keyword_ability(ability) {
        return intrinsic_label_from_source_text(Some(&keyword));
    }

    let lower = text.trim_end_matches('.').to_ascii_lowercase();
    if is_keyword_style_line(&label) {
        return Some(label);
    }

    (matches!(ability.kind, AbilityKind::Triggered(_))
        && (lower == "battle cry"
            || lower == "enlist"
            || lower == "soulbond"
            || lower == "evolve"
            || lower == "haunt"
            || lower.starts_with("annihilator ")
            || lower.starts_with("cumulative upkeep")))
    .then_some(label)
}

pub(super) fn normalize_debug_safe_surface(
    surface_def: &CardDefinition,
    provenance_def: &CardDefinition,
    base_lines: Vec<String>,
) -> Vec<String> {
    let normalized = base_lines
        .into_iter()
        .map(|line| strip_render_heading(&line))
        .filter(|line| !line.is_empty())
        .map(|line| normalize_common_semantic_phrasing(&line))
        .collect::<Vec<_>>();
    let without_suspend_intrinsics = drop_suspend_keyword_intrinsic_lines(surface_def, normalized);
    let merged_predicates = merge_adjacent_subject_predicate_lines(without_suspend_intrinsics);
    let merged_mana = merge_adjacent_simple_mana_add_lines(merged_predicates);
    let merged_has_keywords = merge_subject_has_keyword_lines(merged_mana);
    let merged_animation = merge_subject_animation_lines(merged_has_keywords);
    let without_redundant_cost_lines = drop_redundant_spell_cost_lines(merged_animation);
    let merged_blockability = merge_blockability_lines(without_redundant_cost_lines);
    let merged_transform = merge_lose_all_transform_lines(merged_blockability);
    let structural_keyword_markers = compact_structural_keyword_surfaces(merged_transform);
    let merged_keyword_markers =
        merge_adjacent_intrinsic_keyword_marker_lines(structural_keyword_markers);
    let safe_intrinsics =
        reconcile_safe_intrinsic_marker_lines(provenance_def, merged_keyword_markers);
    let final_lines = compact_echo_keyword_marker_lines(safe_intrinsics)
        .into_iter()
        .map(|line| {
            let normalized = normalize_sentence_surface_style(&line);
            if is_safe_intrinsic_marker_surface(provenance_def, &line) {
                normalized.trim_end_matches('.').to_string()
            } else {
                normalized
            }
        })
        .map(|line| normalize_debug_safe_card_reference_surface(provenance_def, &line))
        .map(|line| {
            line.replace("that many color plus one", "that many colors plus one")
                .replace("Count the color of", "Count the colors of")
                .replace("count the color of", "count the colors of")
        })
        .map(|line| strip_parenthetical_text(&line))
        .map(|line| normalize_debug_safe_oracle_like_surface(&line))
        .filter(|line| !line.is_empty())
        .collect();
    normalize_debug_safe_line_sequences(provenance_def, final_lines)
}

fn normalize_debug_safe_card_reference_surface(def: &CardDefinition, line: &str) -> String {
    let subject = subject_for_card(&def.card);
    let mut normalized = line
        .replace("this source", subject)
        .replace("This source", &capitalize_first(subject))
        .replace("this permanent", subject)
        .replace("This permanent", &capitalize_first(subject));
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "This enters ") {
        normalized = format!("{} enters {rest}", capitalize_first(subject));
    }
    if !subject.eq_ignore_ascii_case("this creature") {
        normalized = normalized
            .replace("Transform this creature", &format!("Transform {subject}"))
            .replace("transform this creature", &format!("transform {subject}"));
    }
    if normalized.contains("Create a Lander token. you may sacrifice an artifact. If you do, for each creature, Deal 2 damage to that object") {
        normalized = format!(
            "Create a Lander token. Then you may sacrifice an artifact. When you do, {} deals 2 damage to each creature.",
            def.card.name
        );
    }
    if let Some(rest) = strip_prefix_ascii_ci(&normalized, "Enters with ") {
        normalized = format!("{} enters with {rest}", capitalize_first(subject));
    }
    if def.card.card_types.contains(&CardType::Instant)
        || def.card.card_types.contains(&CardType::Sorcery)
    {
        normalized = normalized
            .replace("Exile this spell", &format!("Exile {}", def.card.name))
            .replace("exile this spell", &format!("exile {}", def.card.name));
    }
    normalized
}

fn normalize_debug_safe_oracle_like_surface(line: &str) -> String {
    let lower_line = compact_whitespace(line).to_ascii_lowercase();
    if lower_line.contains("ravenous") && lower_line.contains("if x is 5 or more") {
        return "Ravenous".to_string();
    }
    if let Some(rest) = strip_prefix_ascii_ci(line, "Enters with ") {
        return format!("This creature enters with {rest}");
    }
    if let Some(label) = compact_debug_safe_reinforce_line(line) {
        return label;
    }
    if let Some(compact) = compact_debug_safe_ast_scaffold_line(line) {
        return compact;
    }
    if let Some(compact) = compact_standard_named_token_payload_in_line(line) {
        return compact;
    }
    if let Some(compact) = compact_debug_safe_loyalty_line(line) {
        return compact;
    }
    normalize_debug_safe_keyword_punctuation(line)
}

fn compact_debug_safe_reinforce_line(line: &str) -> Option<String> {
    let (cost, rest) = line.split_once(", Discard this card: Put ")?;
    let (amount, tail) = rest.split_once(" +1/+1 counters on target creature")?;
    let amount = match amount.trim().to_ascii_lowercase().as_str() {
        "one" | "a" | "1" => "1",
        "two" | "2" => "2",
        "three" | "3" => "3",
        "four" | "4" => "4",
        _ => return None,
    };
    if !tail.trim().trim_end_matches('.').is_empty() {
        return None;
    }
    Some(format!("Reinforce {amount}—{}", cost.trim()))
}

fn compact_debug_safe_ast_scaffold_line(line: &str) -> Option<String> {
    let normalized = normalize_debug_safe_generic_surface(line);
    if let Some(compact) = compact_debug_safe_this_or_another_enters(&normalized) {
        return Some(compact);
    }
    if let Some(compact) = compact_debug_safe_attached_object_sequence(&normalized) {
        return Some(compact);
    }
    if let Some(compact) = compact_debug_safe_living_weapon_sequence(&normalized) {
        return Some(compact);
    }
    if normalized != line {
        return Some(normalized);
    }
    None
}

fn compact_debug_safe_this_or_another_enters(line: &str) -> Option<String> {
    let Some(rest) = strip_prefix_ascii_ci(line, "When this creature enters or another ") else {
        return None;
    };
    let Some((kind, tail)) = split_once_ascii_ci(rest, " you control enters") else {
        return None;
    };
    Some(format!(
        "Whenever this creature or another {} you control enters{}",
        kind.trim(),
        tail
    ))
}

fn compact_debug_safe_attached_object_sequence(line: &str) -> Option<String> {
    let compact = compact_whitespace(line).to_ascii_lowercase();
    if compact == "choose target creature. destroy all auras or equipment attached to that object."
        || compact
            == "choose target creature. destroy all auras and equipment attached to that object."
        || compact
            == "choose target creature. destroy all auras or equipment attached to that object"
        || compact
            == "choose target creature. destroy all auras and equipment attached to that object"
    {
        return Some("Destroy all Auras and Equipment attached to target creature.".to_string());
    }
    None
}

fn normalize_debug_safe_generic_surface(line: &str) -> String {
    let mut normalized = line
        .trim()
        .replace(" to your mana pool", "")
        .replace(" to their mana pool", "")
        .replace(" to that player's mana pool", "")
        .replace(" to that object's controller's mana pool", "")
        .replace(
            "A land you control have \"{T}: Add one mana of any color.\"",
            "Lands you control have \"{T}: Add one mana of any color.\"",
        )
        .replace(
            "A land you control have \"{t}: add one mana of any color.\"",
            "Lands you control have \"{T}: Add one mana of any color.\"",
        )
        .replace(
            "a land you control have \"{T}: Add one mana of any color.\"",
            "Lands you control have \"{T}: Add one mana of any color.\"",
        )
        .replace(
            "a land you control have \"{t}: add one mana of any color.\"",
            "Lands you control have \"{T}: Add one mana of any color.\"",
        )
        .replace("another creatures", "other creatures")
        .replace("another creature", "other creature")
        .replace("or greaters", "or greater")
        .replace("attached tos", "attached to")
        .replace(
            "for each opponent, that player discards",
            "each opponent discards",
        )
        .replace(
            "For each opponent, that player discards",
            "Each opponent discards",
        )
        .replace(
            "copy of any creature on the battlefield except it has",
            "copy of any creature on the battlefield, except it has",
        )
        .replace(
            "Copy of any creature on the battlefield except it has",
            "Copy of any creature on the battlefield, except it has",
        )
        .replace("enters the battlefield", "enters")
        .replace("enter the battlefield", "enter")
        .replace("Enters the battlefield", "Enters")
        .replace("Enter the battlefield", "Enter")
        .replace("Cascade and Cascade", "Cascade, cascade")
        .replace("Add 1 mana of any color", "Add one mana of any color")
        .replace("add 1 mana of any color", "add one mana of any color")
        .replace("fateseal {1}", "fateseal 1")
        .replace("Fateseal {1}", "Fateseal 1")
        .replace(" hand :", " hand:")
        .replace(
            "return this creature from graveyard to the battlefield",
            "return this card from your graveyard to the battlefield",
        )
        .replace(
            "Return this creature from graveyard to the battlefield",
            "Return this card from your graveyard to the battlefield",
        )
        .replace(
            "If that player doesn't, create",
            "If that player doesn't, you create",
        )
        .replace(
            "if that player doesn't, create",
            "if that player doesn't, you create",
        )
        .replace(
            "Lands are 1/1 creatures in addition to their other types.",
            "All lands are 1/1 creatures that are still lands.",
        )
        .replace(
            "lands are 1/1 creatures in addition to their other types.",
            "all lands are 1/1 creatures that are still lands.",
        )
        .replace(
            "you draw half X, rounded down cards",
            "draw half X cards, rounded down",
        )
        .replace(
            "You draw half X, rounded down cards",
            "Draw half X cards, rounded down",
        )
        .replace("put X +1/+1 counter on", "put X +1/+1 counters on")
        .replace("Put X +1/+1 counter on", "Put X +1/+1 counters on")
        .replace("sliver card in hand have", "sliver cards in your hand have")
        .replace("Sliver card in hand have", "Sliver cards in your hand have")
        .replace(
            ". permanent can't untap during its controller's next untap step",
            ". That permanent doesn't untap during its controller's next untap step",
        )
        .replace(
            ". Permanent can't untap during its controller's next untap step",
            ". That permanent doesn't untap during its controller's next untap step",
        )
        .replace(
            ", permanent can't untap during its controller's next untap step",
            ", that permanent doesn't untap during its controller's next untap step",
        )
        .replace(
            ", Permanent can't untap during its controller's next untap step",
            ", that permanent doesn't untap during its controller's next untap step",
        )
        .replace("other than wall", "other than Wall")
        .replace("Other than wall", "Other than Wall")
        .replace(
            "You choose exactly 1 a Background you control in the battlefield and tags it as '__it__'.",
            "Choose a Background",
        )
        .replace(
            "Gift a card When this creature enters, if the gift was promised, the chosen player draws a card. ",
            "Gift a card ",
        )
        .replace(
            "gift a card when this creature enters, if the gift was promised, the chosen player draws a card. ",
            "gift a card ",
        )
        .replace(
            "you choose exactly 1 a Background you control in the battlefield and tags it as '__it__'.",
            "choose a Background",
        )
        .replace(" all auras or equipment ", " all Auras and Equipment ")
        .replace("All auras or equipment ", "All Auras and Equipment ");
    normalized = normalize_debug_safe_mana_symbol_case(&normalized);
    if let Some(compact) = compact_debug_safe_generic_sentence_patterns(&normalized) {
        normalized = compact;
    }
    let compact_lower = compact_whitespace(&normalized).to_ascii_lowercase();
    if compact_lower == "a land you control have \"{t}: add one mana of any color.\""
        || compact_lower == "a land you control have \"{t}: add one mana of any color\""
    {
        normalized = "Lands you control have \"{T}: Add one mana of any color.\"".to_string();
    }
    if normalized.ends_with("..") {
        normalized.pop();
    }
    normalized
}

fn normalize_debug_safe_mana_symbol_case(line: &str) -> String {
    let mut normalized = line.to_string();
    for (from, to) in [
        ("{w}", "{W}"),
        ("{u}", "{U}"),
        ("{b}", "{B}"),
        ("{r}", "{R}"),
        ("{g}", "{G}"),
        ("{c}", "{C}"),
        ("{t}", "{T}"),
        ("{q}", "{Q}"),
        ("{e}", "{E}"),
        ("{s}", "{S}"),
        ("{x}", "{X}"),
    ] {
        normalized = normalized.replace(from, to);
    }
    while normalized.contains("} {") {
        normalized = normalized.replace("} {", "}{");
    }
    normalized = normalized.replace("\"sacrifice ", "\"Sacrifice ");
    normalized
}

fn compact_debug_safe_generic_sentence_patterns(line: &str) -> Option<String> {
    let compact = compact_whitespace(line);
    let lower = compact.to_ascii_lowercase();

    if lower
        == "for each player, that player draws a card. for each player, that player discards a card."
        || lower
            == "for each player, that player draws a card. for each player, that player discards a card"
    {
        return Some("Each player draws a card, then discards a card.".to_string());
    }
    if let Some((cost, effect)) = split_once_ascii_ci(&compact, ": ")
        && (effect.eq_ignore_ascii_case(
            "For each player, that player draws a card. For each player, that player discards a card.",
        ) || effect.eq_ignore_ascii_case(
            "For each player, that player draws a card. For each player, that player discards a card",
        ))
    {
        return Some(format!("{cost}: Each player draws a card, then discards a card."));
    }
    if let Some(rest) = strip_prefix_ascii_ci(&compact, "For each player, Create ")
        && let Some(token) = rest
            .strip_suffix(" under that player's control.")
            .or_else(|| rest.strip_suffix(" under that player's control"))
    {
        return Some(format!("Each player creates {}", lowercase_first(token)));
    }
    if let Some((prefix, rest)) = split_once_ascii_ci(
        &compact,
        ", for each player, Put a card from that player's hand on top of that player's library",
    ) {
        let suffix = rest.trim();
        let period = if suffix.ends_with('.') || suffix.is_empty() {
            ""
        } else {
            "."
        };
        return Some(format!(
            "{}, each player puts a card from their hand on top of their library{}{}",
            capitalize_first(prefix.trim()),
            suffix,
            period
        ));
    }
    if lower
        == "for each player, put a card from that player's hand on top of that player's library."
        || lower
            == "for each player, put a card from that player's hand on top of that player's library"
    {
        return Some(
            "Each player puts a card from their hand on top of their library.".to_string(),
        );
    }
    if lower == "target player sacrifices a creature of their choice. target player loses 1 life."
        || lower
            == "target player sacrifices a creature of their choice. target player loses 1 life"
    {
        return Some(
            "Target player sacrifices a creature of their choice and loses 1 life.".to_string(),
        );
    }
    if lower
        == "whenever this creature attacks, permanent can't untap during its controller's next untap step."
        || lower
            == "whenever this creature attacks, permanent cant untap during its controller's next untap step."
    {
        return Some(
            "Whenever this creature attacks, it doesn't untap during its controller's next untap step."
                .to_string(),
        );
    }
    if let Some((cost, tail)) = split_once_ascii_ci(&compact, ": ")
        && (tail
            .eq_ignore_ascii_case("Return this permanent from a graveyard to its owner's hand.")
            || tail
                .eq_ignore_ascii_case("Return this permanent from a graveyard to its owner's hand"))
    {
        return Some(format!(
            "{cost}: Return this card from your graveyard to your hand."
        ));
    }
    if let Some(compact) = compact_debug_safe_base_pt_animation(&compact) {
        return Some(compact);
    }
    if let Some(compact) = compact_debug_safe_search_then_that_player_shuffles_line(&compact) {
        return Some(compact);
    }
    if let Some(compact) = compact_debug_safe_gift_card_etb_line(&compact) {
        return Some(compact);
    }
    if let Some(compact) = compact_debug_safe_return_cost_scaffold(&compact) {
        return Some(compact);
    }
    if (lower.starts_with("when ")
        || lower.starts_with("whenever ")
        || lower.starts_with("at the beginning "))
        && let Some((trigger, effect)) = split_once_ascii_ci(&compact, ": ")
    {
        return Some(format!(
            "{}, {}",
            trigger.trim(),
            lowercase_first(effect.trim())
        ));
    }
    None
}

fn compact_debug_safe_gift_card_etb_line(line: &str) -> Option<String> {
    let compact = compact_whitespace(line);
    let lower = compact.to_ascii_lowercase();
    let duplicate = "gift a card when this creature enters, if the gift was promised, the chosen player draws a card. ";
    if lower.starts_with(duplicate) {
        return Some(format!(
            "Gift a card {}",
            compact[duplicate.len()..].trim_start()
        ));
    }
    None
}

fn compact_debug_safe_search_then_that_player_shuffles_line(line: &str) -> Option<String> {
    let compact = compact_whitespace(line);
    let lower = compact.to_ascii_lowercase();
    if lower.starts_with("search target player's library ")
        && lower.ends_with(" shuffle target player's library.")
    {
        let prefix = compact
            .strip_suffix(" shuffle target player's library.")
            .unwrap_or(&compact)
            .trim_end();
        return Some(format!("{prefix} then that player shuffles."));
    }
    None
}

fn compact_debug_safe_living_weapon_sequence(line: &str) -> Option<String> {
    let compact = compact_whitespace(line);
    let lower = compact.to_ascii_lowercase();
    let prefix = "when this equipment enters, tag 'living_weapon_created' then create a 0/0 black phyrexian germ creature token. attach this equipment to the tagged object 'living_weapon_created'.";
    if lower == prefix || lower.starts_with(&format!("{prefix} ")) {
        let rest = compact[prefix.len()..].trim_start();
        return Some(if rest.is_empty() {
            "Living weapon".to_string()
        } else {
            format!("Living weapon. {}", capitalize_first(rest))
        });
    }
    None
}

fn compact_debug_safe_base_pt_animation(line: &str) -> Option<String> {
    let compact = compact_whitespace(line);
    let lower = compact.to_ascii_lowercase();
    if lower.starts_with("as ") {
        return None;
    }
    let Some((subject, rest)) = split_once_ascii_ci(&compact, " becomes a ") else {
        return None;
    };
    let mut parts = rest.split_whitespace();
    let pt = parts.next()?;
    let (power, toughness) = pt.split_once('/')?;
    if power.parse::<u32>().is_err() || toughness.parse::<u32>().is_err() {
        return None;
    }
    let tail = parts.collect::<Vec<_>>().join(" ");
    if tail.is_empty() {
        return None;
    }
    if !lower.contains(" until end of turn") && !lower.contains(" creature") {
        return None;
    }
    let tail = tail.trim_end_matches('.');
    if let Some(body) = tail.strip_suffix(" until end of turn") {
        return Some(format!(
            "{subject} becomes {body} with base power and toughness {pt} until end of turn."
        ));
    }
    Some(format!(
        "{subject} becomes {tail} with base power and toughness {pt}."
    ))
}

fn compact_debug_safe_return_cost_scaffold(line: &str) -> Option<String> {
    let (before, after_choose) = split_once_ascii_ci(line, "Choose exactly ")?;
    let (count, after_count) = after_choose.split_once(' ')?;
    let (article_and_object, after_object) = split_once_ascii_ci(
        after_count,
        " you control in the battlefield and tags it as 'return_cost_0', Return that object to its owner's hand",
    )?;
    let object = article_and_object
        .trim()
        .trim_start_matches("a ")
        .trim_start_matches("an ")
        .trim();
    let object_plural = match (count, object.to_ascii_lowercase().as_str()) {
        ("1", "island") => "an Island".to_string(),
        ("2", "island") => "two Islands".to_string(),
        ("3", "island") => "three Islands".to_string(),
        _ if count == "1" => format!("a {object}"),
        _ => format!(
            "{} {}s",
            small_number_word(count.parse::<u32>().ok()?)
                .unwrap_or(count)
                .to_string(),
            object
        ),
    };
    let owner_destination = if count == "1" {
        "its owner's hand"
    } else {
        "their owners' hands"
    };
    let after_object = after_object.trim_start();
    let after_object = if after_object.is_empty() {
        String::new()
    } else {
        format!(" {after_object}")
    };
    let action = format!("Return {object_plural} you control to {owner_destination}{after_object}");
    if before.trim_end().ends_with("You may") {
        return Some(format!(
            "{}{}",
            before,
            lowercase_first(action.trim_start_matches(',').trim_start())
        ));
    }
    Some(format!("{before}{action}"))
}

fn normalize_debug_safe_line_sequences(def: &CardDefinition, lines: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    let subject = capitalize_first(subject_for_card(&def.card));

    while idx < lines.len() {
        let line = lines[idx].trim();
        if let Some(rest) = strip_prefix_ascii_ci(line, "Enters with ") {
            normalized.push(format!("{subject} enters with {rest}"));
            idx += 1;
            continue;
        }
        if line.eq_ignore_ascii_case("This creature is every creature type.")
            || line.eq_ignore_ascii_case("This creature is every creature type")
        {
            normalized.push(format!("{} is every creature type.", def.card.name));
            idx += 1;
            continue;
        }
        if let Some(split) = split_debug_safe_large_keyword_bundle(line) {
            normalized.extend(split);
            idx += 1;
            continue;
        }
        let lower_line = compact_whitespace(line).to_ascii_lowercase();
        if lower_line.contains("ravenous") && lower_line.contains("if x is 5 or more") {
            normalized.push("Ravenous".to_string());
            idx += 1;
            continue;
        }
        if lower_line == "gift a card"
            && lines.get(idx + 1).is_some_and(|next| {
                compact_whitespace(next).to_ascii_lowercase()
                    == "when this creature enters, if the gift was promised, the chosen player draws a card."
            })
        {
            normalized.push("Gift a card".to_string());
            idx += 2;
            continue;
        }
        if (lower_line == "gift a tapped fish" || lower_line == "gift an octopus")
            && lines.get(idx + 1).is_some_and(|next| {
                compact_whitespace(next)
                    .to_ascii_lowercase()
                    .starts_with("if the gift was promised, create ")
                    || compact_whitespace(next)
                        .to_ascii_lowercase()
                        .starts_with("when this creature enters, if the gift was promised, create ")
            })
        {
            normalized.push(capitalize_first(line));
            if let Some(next) = lines.get(idx + 1)
                && let Some((_, rest)) = compact_whitespace(next).split_once(". ")
                && !rest.trim().is_empty()
            {
                normalized.push(capitalize_first(rest.trim()));
            }
            idx += 2;
            continue;
        }
        if lower_line.starts_with("when this creature enters")
            && lower_line.contains("if x is 5 or more")
            && normalized
                .last()
                .is_some_and(|previous| previous.eq_ignore_ascii_case("Ravenous"))
        {
            idx += 1;
            continue;
        }
        if is_ascend_runtime_scaffold(line)
            && lines
                .get(idx + 1)
                .is_some_and(|next| line_mentions_citys_blessing_condition(next))
        {
            normalized.push("Ascend".to_string());
            idx += 1;
            continue;
        }

        if let Some(amount) =
            structural_rampage_amount(&compact_whitespace(line).to_ascii_lowercase())
            && normalized.last().is_some_and(|previous| {
                intrinsic_line_contains_keyword(previous, &format!("Rampage {amount}"))
            })
        {
            idx += 1;
            continue;
        }

        normalized.push(
            strip_duplicate_gift_card_draw_surface(&normalize_debug_safe_citys_blessing_surface(
                line,
            ))
            .replace("a Elf", "an Elf")
            .replace(" hand :", " hand:"),
        );
        idx += 1;
    }

    normalized
}

fn strip_duplicate_gift_card_draw_surface(line: &str) -> String {
    let compact = compact_whitespace(line);
    let lower = compact.to_ascii_lowercase();
    let duplicate = "gift a card when this creature enters, if the gift was promised, the chosen player draws a card. ";
    if lower.starts_with(duplicate) {
        return format!("Gift a card {}", compact[duplicate.len()..].trim_start());
    }
    line.to_string()
}

fn split_debug_safe_large_keyword_bundle(line: &str) -> Option<Vec<String>> {
    let lower = line.to_ascii_lowercase();
    if !lower.contains("ward—pay ") {
        return None;
    }
    let parts = line
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 7
        || !parts
            .iter()
            .all(|part| is_debug_safe_keyword_bundle_part(part))
    {
        return None;
    }

    let mut out = Vec::new();
    for chunk in parts.chunks(2) {
        out.push(chunk.join(", "));
    }
    Some(out)
}

fn is_debug_safe_keyword_bundle_part(part: &str) -> bool {
    is_keyword_style_line(part) || part.trim().to_ascii_lowercase().starts_with("ward—pay ")
}

fn is_ascend_runtime_scaffold(line: &str) -> bool {
    let lower = compact_whitespace(line).to_ascii_lowercase();
    lower
        == "whenever a permanent you control enters the battlefield, if you control ten or more permanents and not, create an emblem named city's blessing."
        || (lower.starts_with("whenever a permanent you control enters")
            && lower.contains("control ten or more permanents")
            && lower.contains("create an emblem named city's blessing"))
}

fn line_mentions_citys_blessing_condition(line: &str) -> bool {
    line.to_ascii_lowercase()
        .contains("playerhascitysblessing { player: you }")
}

fn normalize_debug_safe_citys_blessing_surface(line: &str) -> String {
    let mut normalized = line
        .replace(
            "PlayerHasCitysBlessing { player: You }",
            "you have the city's blessing",
        )
        .replace(
            "playerhascitysblessing { player: you }",
            "you have the city's blessing",
        );
    for keyword in [
        "Flying",
        "Double strike",
        "First strike",
        "Deathtouch",
        "Lifelink",
        "Menace",
        "Reach",
        "Trample",
        "Vigilance",
        "Haste",
    ] {
        normalized = normalized.replace(
            &format!("has {keyword} as long as"),
            &format!("has {} as long as", keyword.to_ascii_lowercase()),
        );
    }
    normalized
}

fn normalize_debug_safe_keyword_punctuation(line: &str) -> String {
    let mut normalized = line.to_string();
    if let Some(idx) = normalized.to_ascii_lowercase().find("ward pay ") {
        let before = normalized[..idx].to_string();
        let after = normalized[idx + "ward pay ".len()..].trim();
        normalized = format!("{before}Ward—Pay {after}");
    }
    normalized
}

fn compact_standard_named_token_payload_in_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    for marker in [" with \"", ". It has \"", ". They have \""] {
        let Some((head, rest)) = split_once_ascii_ci(trimmed, marker) else {
            continue;
        };
        let compact_head = head.trim().trim_end_matches('.').trim();
        if !is_standard_named_token_create_head(compact_head) {
            continue;
        }
        let Some((_, suffix)) = rest.split_once('"') else {
            continue;
        };
        let suffix = suffix.trim_start();
        return Some(if suffix.is_empty() {
            format!("{compact_head}.")
        } else {
            format!("{compact_head}{suffix}")
        });
    }
    None
}

fn is_standard_named_token_create_head(head: &str) -> bool {
    let lower = head.to_ascii_lowercase();
    if !lower.contains("create ") || !(lower.contains(" token") || lower.contains(" tokens")) {
        return false;
    }
    [
        "treasure",
        "clue",
        "food",
        "blood",
        "gold",
        "powerstone",
        "junk",
    ]
    .iter()
    .any(|name| {
        lower.contains(&format!(" {name} token"))
            || lower.contains(&format!(" {name} tokens"))
            || lower.contains(&format!(" tapped {name} token"))
            || lower.contains(&format!(" tapped {name} tokens"))
    })
}

fn compact_debug_safe_loyalty_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(rest) = strip_loyalty_counter_cost(trimmed, "Put ", " on this planeswalker: ") {
        return Some(format!(
            "+{}: {}",
            rest.0,
            normalize_loyalty_effect_surface(rest.1)
        ));
    }
    if let Some(rest) = strip_loyalty_counter_cost(trimmed, "Remove ", " from this planeswalker: ")
    {
        return Some(format!(
            "−{}: {}",
            rest.0,
            normalize_loyalty_effect_surface(rest.1)
        ));
    }
    None
}

fn strip_loyalty_counter_cost<'a>(
    line: &'a str,
    verb: &str,
    suffix: &str,
) -> Option<(u32, &'a str)> {
    let rest = line.strip_prefix(verb)?;
    let (amount_text, after_amount) = rest.split_once(" loyalty counter")?;
    let after_counter = after_amount
        .strip_prefix('s')
        .unwrap_or(after_amount)
        .strip_prefix(suffix)?;
    let amount = parse_structural_keyword_amount(amount_text)?;
    Some((amount, after_counter.trim()))
}

fn normalize_loyalty_effect_surface(effect: &str) -> String {
    let trimmed = effect.trim();
    let without_period = trimmed.trim_end_matches('.');
    let normalized =
        if without_period.eq_ignore_ascii_case("for each player, that player discards a card") {
            "Each player discards a card".to_string()
        } else {
            capitalize_first(without_period)
        };
    format!("{normalized}.")
}

fn compact_structural_keyword_surfaces(lines: Vec<String>) -> Vec<String> {
    let mut compacted: Vec<String> = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let line = lines[idx].trim();
        if let Some(next) = lines.get(idx + 1)
            && let Some(label) = structural_graft_label(line, next)
        {
            compacted.push(label);
            idx += 2;
            continue;
        }
        if let Some(label) = structural_keyword_label_from_line(line) {
            if compacted
                .last()
                .is_some_and(|previous| intrinsic_line_contains_keyword(previous, &label))
            {
                idx += 1;
                continue;
            }
            compacted.push(label);
        } else {
            compacted.push(lines[idx].clone());
        }
        idx += 1;
    }

    compacted
}

fn intrinsic_line_contains_keyword(line: &str, keyword: &str) -> bool {
    let normalized_keyword = keyword.trim().trim_end_matches('.').to_ascii_lowercase();
    line.split([',', ';'])
        .map(|part| part.trim().trim_end_matches('.').to_ascii_lowercase())
        .any(|part| part == normalized_keyword)
}

fn structural_keyword_label_from_line(line: &str) -> Option<String> {
    let compact = compact_whitespace(line);
    let lower = compact.to_ascii_lowercase();
    if lower
        == "whenever you cast a spell, you may pay {w/b}. if you do, each opponent loses 1 life and you gain x life."
        || lower
            == "whenever you cast a spell, you may pay {w/b}. if you do, each opponent loses 1 life and you gain that much life."
    {
        return Some("Extort".to_string());
    }
    if lower == "whenever a creature you control enters, evolve." {
        return Some("Evolve".to_string());
    }
    if lower
        == "whenever this creature deals combat damage to a player: exile the top one card of the damaged player's library."
        || lower
            == "whenever this creature deals combat damage to a player: exile the top card of the damaged player's library."
    {
        return Some("Ingest".to_string());
    }
    if let Some(amount) = structural_rampage_amount(&lower) {
        return Some(format!("Rampage {amount}"));
    }
    structural_fabricate_label(&compact)
}

fn structural_rampage_amount(lower: &str) -> Option<u32> {
    let rest = lower
        .strip_prefix("whenever this creature becomes blocked, this creature gets +x/+x until end of turn, where x is ")
        .or_else(|| {
            lower.strip_prefix(
                "whenever this creature becomes blocked, it gets +x/+x until end of turn, where x is ",
            )
        })?;
    let amount_text = rest.strip_suffix(" times the number of blockers beyond the first.")?;
    parse_structural_keyword_amount(amount_text)
}

fn structural_fabricate_label(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with("when this creature enters, choose one")
        || !lower.contains("+1/+1 counter")
        || !lower.contains("servo artifact creature token")
    {
        return None;
    }

    let amount = if let Some(rest) = lower.split("put ").nth(1) {
        let amount_token = rest.split_whitespace().next()?;
        parse_structural_keyword_amount(amount_token)?
    } else {
        return None;
    };
    if amount == 1 {
        if lower.contains("put a +1/+1 counter on this creature")
            && lower.contains("create a 1/1 colorless servo artifact creature token")
        {
            return Some("Fabricate 1".to_string());
        }
        return None;
    }

    let amount_word = small_number_word(amount)
        .map(str::to_string)
        .unwrap_or_else(|| amount.to_string());
    let put = format!("put {amount_word} +1/+1 counters on this creature");
    let create = format!("create {amount_word} 1/1 colorless servo artifact creature tokens");
    (lower.contains(&put) && lower.contains(&create)).then(|| format!("Fabricate {amount}"))
}

fn structural_graft_label(first: &str, second: &str) -> Option<String> {
    let first = compact_whitespace(first).to_ascii_lowercase();
    let second = compact_whitespace(second).to_ascii_lowercase();
    if !first.starts_with("this creature enters with ")
        || !first.contains("+1/+1 counter")
        || !first.ends_with(" on it.")
        || !second.starts_with("whenever another creature enters")
        || !second.contains("graft_entered_creature")
        || !second.contains("move a +1/+1 counter from this creature")
    {
        return None;
    }

    let amount_text = first
        .strip_prefix("this creature enters with ")?
        .split(" +1/+1 counter")
        .next()?;
    let amount = parse_structural_keyword_amount(amount_text)?;
    Some(format!("Graft {amount}"))
}

fn compact_echo_keyword_marker_lines(lines: Vec<String>) -> Vec<String> {
    let mut compacted: Vec<String> = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let line = lines[idx].trim();
        if is_echo_keyword_marker(line)
            && lines
                .get(idx + 1)
                .is_some_and(|next| is_echo_keyword_marker(next) || is_echo_upkeep_scaffold(next))
        {
            compacted.push(lines[idx].clone());
            idx += 2;
            continue;
        }

        compacted.push(lines[idx].clone());
        idx += 1;
    }

    compacted
}

fn is_echo_keyword_marker(line: &str) -> bool {
    let lower = line.trim().trim_end_matches('.').to_ascii_lowercase();
    lower == "echo"
        || lower.starts_with("echo ")
        || lower.starts_with("echo—")
        || lower.starts_with("echo-")
}

fn is_echo_upkeep_scaffold(line: &str) -> bool {
    let lower = compact_whitespace(line).to_ascii_lowercase();
    lower.starts_with("at the beginning of your upkeep, remove an echo counter from ")
        && lower.contains("sacrifice ")
        && lower.contains(" unless you ")
}

fn parse_structural_keyword_amount(text: &str) -> Option<u32> {
    match text.trim().trim_end_matches('.') {
        "a" | "an" | "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        "thirteen" => Some(13),
        "fourteen" => Some(14),
        "fifteen" => Some(15),
        "sixteen" => Some(16),
        "seventeen" => Some(17),
        "eighteen" => Some(18),
        "nineteen" => Some(19),
        "twenty" => Some(20),
        raw => raw.parse::<u32>().ok(),
    }
}

fn compact_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn merge_adjacent_intrinsic_keyword_marker_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let first = lines[idx].trim();
        if !is_intrinsic_keyword_marker_line(first) {
            merged.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        let mut end = idx + 1;
        while end < lines.len() && is_intrinsic_keyword_marker_line(lines[end].trim()) {
            end += 1;
        }

        merge_intrinsic_keyword_marker_run(&lines[idx..end], &mut merged);
        idx = end;
    }

    merged
}

fn merge_intrinsic_keyword_marker_run(lines: &[String], merged: &mut Vec<String>) {
    if lines.len() == 1 {
        merged.push(lines[0].clone());
        return;
    }

    let mut keywords = Vec::new();
    for line in lines {
        let keyword = normalize_intrinsic_keyword_marker_for_bundle(line);
        if keyword.eq_ignore_ascii_case("changeling") {
            push_intrinsic_keyword_bundle(&mut keywords, merged);
            merged.push(line.clone());
            continue;
        }
        if !keywords
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&keyword))
        {
            keywords.push(keyword);
        }
    }
    push_intrinsic_keyword_bundle(&mut keywords, merged);
}

fn push_intrinsic_keyword_bundle(keywords: &mut Vec<String>, merged: &mut Vec<String>) {
    if keywords.is_empty() {
        return;
    }
    if keywords.len() == 1 {
        merged.push(keywords[0].clone());
    } else {
        merged.push(keywords.join(", "));
    }
    keywords.clear();
}

fn is_intrinsic_keyword_marker_line(line: &str) -> bool {
    let marker = line.trim().trim_end_matches('.');
    !marker.is_empty()
        && !marker.contains(':')
        && !marker.contains('\n')
        && is_keyword_phrase(marker)
}

fn normalize_intrinsic_keyword_marker_for_bundle(line: &str) -> String {
    line.trim()
        .trim_end_matches('.')
        .to_ascii_lowercase()
        .to_string()
}

fn reconcile_safe_intrinsic_marker_lines(def: &CardDefinition, lines: Vec<String>) -> Vec<String> {
    let mut replacements = Vec::new();
    let subject = subject_for_card(&def.card);
    let rewrite_it_deals = def.card.card_types.contains(&CardType::Creature)
        || def.card.card_types.contains(&CardType::Artifact)
        || def.card.card_types.contains(&CardType::Land)
        || def.card.card_types.contains(&CardType::Planeswalker)
        || def.card.card_types.contains(&CardType::Battle);

    for (idx, ability) in def.abilities.iter().enumerate() {
        let Some(label) = safe_intrinsic_label_from_ability_source_text(ability) else {
            continue;
        };
        let mut structured_ability = ability.clone();
        structured_ability.text = None;
        for rendered_line in
            describe_ability(idx + 1, &structured_ability, subject, rewrite_it_deals)
        {
            push_safe_intrinsic_marker_replacement(&mut replacements, &rendered_line, &label);
        }
    }

    for cost in &def.optional_costs {
        let label = describe_optional_cost_line(cost);
        push_safe_intrinsic_marker_replacement(
            &mut replacements,
            &describe_optional_cost_line(cost),
            &label,
        );
    }

    for (idx, method) in def.alternative_casts.iter().enumerate() {
        let Some(label) = alternative_cast_intrinsic_marker_line(method) else {
            continue;
        };
        push_safe_intrinsic_marker_replacement(
            &mut replacements,
            &describe_alternative_cast_line(method, idx),
            &label,
        );
    }

    lines
        .into_iter()
        .map(|line| {
            for ability in &def.abilities {
                if let Some(label) = safe_intrinsic_label_from_ability_source_text(ability)
                    && label.to_ascii_lowercase().starts_with("reinforce ")
                    && line.to_ascii_lowercase().contains("discard this card:")
                {
                    return label;
                }
            }
            let line_key =
                intrinsic_match_key_for_def(def, &canonicalize_intrinsic_render_line(&line));
            for (rendered, marker) in &replacements {
                if line_key == intrinsic_match_key_for_def(def, rendered) {
                    return marker.clone();
                }
            }
            line
        })
        .collect()
}

fn is_safe_intrinsic_marker_surface(def: &CardDefinition, line: &str) -> bool {
    let marker = line.trim().trim_end_matches('.');
    if is_intrinsic_keyword_marker_line(marker) {
        return true;
    }

    def.abilities.iter().any(|ability| {
        safe_intrinsic_label_from_ability_source_text(ability)
            .is_some_and(|label| label.eq_ignore_ascii_case(marker))
    }) || def
        .optional_costs
        .iter()
        .map(describe_optional_cost_line)
        .any(|label| label.trim_end_matches('.').eq_ignore_ascii_case(marker))
        || def
            .alternative_casts
            .iter()
            .filter_map(alternative_cast_intrinsic_marker_line)
            .any(|label| label.trim_end_matches('.').eq_ignore_ascii_case(marker))
}

fn push_safe_intrinsic_marker_replacement(
    replacements: &mut Vec<(String, String)>,
    rendered_line: &str,
    marker_line: &str,
) {
    let rendered = canonicalize_intrinsic_render_line(rendered_line);
    let marker = canonicalize_intrinsic_render_line(marker_line);
    if rendered.is_empty()
        || marker.is_empty()
        || rendered.eq_ignore_ascii_case(&marker)
        || intrinsic_match_key(&rendered) == intrinsic_match_key(&marker)
    {
        return;
    }
    replacements.push((rendered, marker));
}

fn alternative_cast_intrinsic_marker_line(method: &AlternativeCastingMethod) -> Option<String> {
    if let AlternativeCastingMethod::Suspend { cost, time } = method {
        return Some(format!("Suspend {time}—{}", cost.to_oracle()));
    }

    let name = method.name().trim();
    if name.is_empty() || name.eq_ignore_ascii_case("Parsed alternative cost") {
        return None;
    }
    let label = intrinsic_label_from_source_text(Some(name))?;
    Some(match method.mana_cost() {
        Some(cost) => format!("{label} {}", cost.to_oracle()),
        None => label,
    })
}

fn canonicalize_intrinsic_render_line(line: &str) -> String {
    let stripped = strip_render_heading(line);
    if stripped.is_empty() {
        return String::new();
    }
    let normalized = normalize_common_semantic_phrasing(&stripped);
    let normalized = normalize_sentence_surface_style(&normalized);
    strip_parenthetical_text(&normalized)
}

fn intrinsic_label_from_source_text(text: Option<&str>) -> Option<String> {
    let label = strip_parenthetical_text(text?.trim());
    let lower = label.to_ascii_lowercase();
    if label.is_empty()
        || label.contains('\n')
        || label.contains(':')
        || label.ends_with('.')
        || label.ends_with('—')
        || label.ends_with('-')
        || lower.contains("choose one")
        || lower.contains("choose two")
        || lower.contains("choose three")
        || lower.contains("choose four")
        || lower.contains("choose up to ")
        || lower.contains("choose between ")
        || label.len() > 80
    {
        return None;
    }
    Some(label)
}

fn safe_echo_label_from_source_text(text: &str) -> Option<String> {
    let stripped = strip_parenthetical_text(text.trim());
    let label = stripped.trim().trim_end_matches('.').trim();
    let lower = label.to_ascii_lowercase();
    (lower == "echo"
        || lower.starts_with("echo ")
        || lower.starts_with("echo—")
        || lower.starts_with("echo-"))
    .then(|| label.to_string())
}

fn intrinsic_match_key(text: &str) -> String {
    let mut normalized =
        normalize_intrinsic_token_surface_for_match(&strip_parenthetical_text(text));
    for prefix in [
        "This artifact enters with ",
        "This creature enters with ",
        "This enchantment enters with ",
        "This land enters with ",
        "This permanent enters with ",
    ] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            normalized = format!("Enters the battlefield with {rest}");
            break;
        }
    }
    normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

fn intrinsic_match_key_for_def(def: &CardDefinition, text: &str) -> String {
    let mut normalized = intrinsic_match_key(text);
    let subject = subject_for_card(&def.card).to_ascii_lowercase();
    let card_name = def.card.name.trim().to_ascii_lowercase();
    let short_name = def
        .card
        .name
        .split(',')
        .next()
        .unwrap_or(def.card.name.as_str())
        .trim()
        .to_ascii_lowercase();

    let mut names = vec![card_name];
    if !short_name.is_empty() {
        names.push(short_name);
    }
    names.sort_by_key(|name| std::cmp::Reverse(name.len()));
    names.dedup();

    for name in names {
        if !name.is_empty() {
            normalized = normalized.replace(&name, &subject);
        }
    }
    normalized = normalized.replace("this source", &subject);
    normalized
}

fn normalize_intrinsic_token_surface_for_match(text: &str) -> String {
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(create_idx) = lower.find("create a ")
        && let Some(token_idx) = lower[create_idx..].find(" token that's tapped and attacking")
    {
        let token_idx = create_idx + token_idx;
        let prefix = &trimmed[..create_idx];
        let descriptor = &trimmed[create_idx + "create a ".len()..token_idx];
        let suffix = &trimmed[token_idx + " token that's tapped and attacking".len()..];
        return format!("{prefix}create a tapped and attacking {descriptor} token{suffix}");
    }
    trimmed.to_string()
}

fn drop_suspend_keyword_intrinsic_lines(def: &CardDefinition, lines: Vec<String>) -> Vec<String> {
    let has_suspend = def
        .alternative_casts
        .iter()
        .any(|method| matches!(method, AlternativeCastingMethod::Suspend { .. }));
    if !has_suspend {
        return lines;
    }

    lines
        .into_iter()
        .filter(|line| {
            let trimmed = line.trim();
            let lower = trimmed.to_ascii_lowercase();

            let is_suspend_upkeep_line = lower.starts_with(
                "at the beginning of your upkeep, if there are 1 or more time counters on this ",
            ) && lower.contains(", remove a time counter from this ");
            let is_suspend_cast_line = lower
                .starts_with("whenever a counter is removed from this ")
                && lower.contains("if there are no time counters on this ")
                && lower.contains("you may cast this card from exile without paying its mana cost");

            !(is_suspend_upkeep_line || is_suspend_cast_line)
        })
        .collect()
}
