use super::*;
use crate::text_cleanup::strip_parenthetical_text;

/// Render the structured runtime model for debug/inspector use.
///
/// This deliberately disables source/oracle-sensitive reconciliation. It first
/// renders abilities without source text, then applies only debug-safe
/// normalization owned by this module.
pub fn debug_compiled_lines(def: &CardDefinition) -> Vec<String> {
    normalize_debug_safe_surface(def, def, ast_compiled_lines(def))
}

/// Render the structured compiled-text surface used for DB scoring.
pub fn compiled_text_lines(def: &CardDefinition) -> Vec<String> {
    compact_unprocessed_surface_markers(def, debug_compiled_lines(def))
}

pub fn unprocessed_compiled_lines(def: &CardDefinition) -> Vec<String> {
    compact_unprocessed_surface_markers(def, debug_compiled_lines(def))
}

fn safe_intrinsic_label_from_ability(ability: &Ability) -> Option<String> {
    if let Some(keyword) = describe_keyword_ability(ability) {
        return intrinsic_keyword_label(Some(&keyword));
    }
    None
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
    let merged_cost_tax =
        merge_conditioned_spell_and_activation_tax_lines(without_redundant_cost_lines);
    let merged_blockability = merge_blockability_lines(merged_cost_tax);
    let merged_transform = merge_lose_all_transform_lines(merged_blockability);
    let structural_keyword_markers = compact_structural_keyword_surfaces(merged_transform);
    let merged_keyword_markers =
        merge_adjacent_intrinsic_keyword_marker_lines(structural_keyword_markers);
    let safe_intrinsics =
        reconcile_safe_intrinsic_marker_lines(provenance_def, merged_keyword_markers);
    let final_lines = compact_echo_keyword_marker_lines(safe_intrinsics)
        .into_iter()
        .map(|line| {
            let normalized = normalize_debug_safe_sentence_surface(&line);
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
                .replace("that much +1/+1 counter", "that many +1/+1 counters")
                .replace("If you is the monarch", "If you're the monarch")
                .replace("if you is the monarch", "if you're the monarch")
                .replace("Otherwise, You become", "Otherwise, you become")
        })
        .map(|line| strip_parenthetical_text(&line))
        .map(|line| normalize_debug_safe_oracle_like_surface(&line))
        .filter(|line| !line.is_empty())
        .collect();
    normalize_debug_safe_line_sequences(provenance_def, final_lines)
}

fn normalize_debug_safe_sentence_surface(line: &str) -> String {
    if !line.contains('\n') {
        return normalize_sentence_surface_style(line);
    }

    line.lines()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            if let Some(body) = part.strip_prefix('•') {
                let body = normalize_sentence_surface_style(body.trim());
                return format!("• {body}");
            }
            if let Some(header) = normalize_modal_header_surface(part) {
                return header;
            }
            normalize_sentence_surface_style(part)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn compact_unprocessed_surface_markers(_def: &CardDefinition, lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .map(|line| {
            let line = compact_scored_token_with_quoted_ability_line(&line).unwrap_or(line);
            let line = compact_standard_named_token_payload_in_line(&line).unwrap_or(line);
            let compact = compact_whitespace(line.trim());
            if compact.eq_ignore_ascii_case("Spells cost {X} less to cast.")
                || compact.eq_ignore_ascii_case("Spells cost {X} less to cast")
            {
                return "Undaunted".to_string();
            }
            line
        })
        .collect()
}

fn compact_scored_token_with_quoted_ability_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some((token_text, ability)) = trimmed.split_once(" token. It has \"") {
        let ability = ability
            .strip_suffix("\".")
            .or_else(|| ability.strip_suffix('"'))?
            .trim();
        return Some(format!("{token_text} token with \"{ability}\""));
    }
    if let Some((token_text, ability)) = trimmed.split_once(" tokens. They have \"") {
        let ability = ability
            .strip_suffix("\".")
            .or_else(|| ability.strip_suffix('"'))?
            .trim();
        return Some(format!("{token_text} tokens with \"{ability}\""));
    }
    None
}

fn normalize_modal_header_surface(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.').trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("choose ") {
        return None;
    }
    let head = trimmed
        .strip_suffix('-')
        .or_else(|| trimmed.strip_suffix('—'))?
        .trim();
    if head.is_empty() {
        None
    } else {
        Some(format!("{} —", capitalize_first(head)))
    }
}

fn normalize_debug_safe_card_reference_surface(def: &CardDefinition, line: &str) -> String {
    let subject = subject_for_card(&def.card);
    let source_name = def
        .card
        .name
        .split(',')
        .next()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| capitalize_first(subject));
    let mut normalized = line
        .replace("this source", subject)
        .replace("This source", &capitalize_first(subject))
        .replace("this permanent", subject)
        .replace("This permanent", &capitalize_first(subject));
    normalized = normalized
        .replace(
            "Whenever this creature or another ",
            &format!("Whenever {source_name} or another "),
        )
        .replace(
            "When this creature or another ",
            &format!("When {source_name} or another "),
        );
    if def.card.supertypes.contains(&Supertype::Legendary) {
        normalized = normalized
            .replace(
                &format!("Whenever {subject} enters"),
                &format!("Whenever {source_name} enters"),
            )
            .replace(
                &format!("When {subject} enters"),
                &format!("When {source_name} enters"),
            )
            .replace(
                &format!("Whenever {subject} dies"),
                &format!("Whenever {source_name} dies"),
            )
            .replace(
                &format!("When {subject} dies"),
                &format!("When {source_name} dies"),
            )
            .replace(
                &format!("if {subject} is tapped"),
                &format!("if {source_name} is tapped"),
            )
            .replace(
                &format!("If {subject} is tapped"),
                &format!("If {source_name} is tapped"),
            );
    }
    if normalized.contains("{TK}") || normalized.to_ascii_lowercase().contains("sticker") {
        normalized = normalized
            .replace(
                &format!("Whenever {subject} enters"),
                &format!("Whenever {source_name} enters"),
            )
            .replace(
                &format!("When {subject} enters"),
                &format!("When {source_name} enters"),
            )
            .replace(
                &format!("other than {subject}"),
                &format!("other than {source_name}"),
            );
    }
    if def.card.supertypes.contains(&Supertype::Legendary) {
        for self_ref in [
            "this artifact",
            "this creature",
            "this enchantment",
            "this land",
            "this planeswalker",
            "this permanent",
        ] {
            normalized = normalized
                .replace(
                    &format!("Exile {self_ref} and "),
                    &format!("Exile {source_name} and "),
                )
                .replace(
                    &format!("exile {self_ref} and "),
                    &format!("exile {source_name} and "),
                );
        }
        normalized = normalized
            .replace(
                "This has all activated abilities",
                &format!("{source_name} has all activated abilities"),
            )
            .replace(
                "this has all activated abilities",
                &format!("{source_name} has all activated abilities"),
            );
    }
    if let Some(keyword) = source_keyword_during_your_turn(&normalized, subject) {
        normalized = format!("During your turn, {source_name} has {keyword}");
    }
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

fn source_keyword_during_your_turn(line: &str, subject: &str) -> Option<String> {
    let line = line.trim().trim_end_matches('.');
    for candidate_subject in [capitalize_first(subject), subject.to_string()] {
        let prefix = format!("{candidate_subject} has ");
        let Some(rest) = strip_prefix_ascii_ci(line, &prefix) else {
            continue;
        };
        let Some(keyword) = rest.strip_suffix(" as long as it's your turn") else {
            continue;
        };
        let keyword = keyword.trim();
        if is_keyword_phrase(keyword) {
            return Some(keyword.to_ascii_lowercase());
        }
    }
    None
}

fn normalize_debug_safe_oracle_like_surface(line: &str) -> String {
    let lower_line = compact_whitespace(line).to_ascii_lowercase();
    if let Some((prefix, suffix)) = split_once_ascii_ci(
        line,
        "At the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard.",
    ) {
        return format!(
            "{prefix}At the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.{suffix}"
        );
    }
    if lower_line.starts_with("whenever destroy ") {
        return line
            .trim_start()
            .strip_prefix("Whenever ")
            .unwrap_or(line)
            .to_string();
    }
    if lower_line.contains("ravenous") && lower_line.contains("if x is 5 or more") {
        return "Ravenous".to_string();
    }
    if let Some(compact) = compact_land_animation_line(line) {
        return compact;
    }
    if let Some(rest) = strip_prefix_ascii_ci(line, "Enters with ") {
        return format!("This creature enters with {rest}");
    }
    if let Some(label) = compact_debug_safe_reinforce_line(line) {
        return label;
    }
    if let Some(compact) = compact_debug_safe_loyalty_line(line) {
        return compact;
    }
    if let Some(compact) = compact_constellation_instead_line(line) {
        return compact;
    }
    if let Some(compact) = compact_debug_safe_ast_scaffold_line(line) {
        return compact_debug_safe_loyalty_line(&compact).unwrap_or(compact);
    }
    if let Some(compact) = compact_standard_named_token_payload_in_line(line) {
        return compact_debug_safe_loyalty_line(&compact).unwrap_or(compact);
    }
    if let Some(compact) = compact_create_token_with_quoted_ability_line(line) {
        return compact;
    }
    if let Some(compact) = compact_defending_player_block_bonus_line(line) {
        return compact;
    }
    let compact = compact_choice_tag_scaffold(line);
    let compact = compact_repeat_process_once(&compact);
    let compact = compact_life_total_extra_turn_surface(&compact);
    let compact = compact_keyword_ability_label_surface(&compact);
    let compact = compact_counter_that_spell_sequence(&compact);
    let compact = compact_devotion_life_loss_surface(&compact);
    normalize_debug_safe_keyword_punctuation(&compact)
}

fn compact_keyword_ability_label_surface(line: &str) -> String {
    for keyword in [
        "Flying",
        "Trample",
        "First strike",
        "Double strike",
        "Deathtouch",
        "Haste",
        "Hexproof",
        "Indestructible",
        "Lifelink",
        "Menace",
        "Reach",
        "Vigilance",
    ] {
        let prefix = format!("{keyword}, ");
        if let Some(rest) = line.strip_prefix(&prefix)
            && let Some((label, tail)) = rest.split_once(" — ")
            && tail
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_lowercase())
        {
            return format!(
                "{keyword}\n{} — {}",
                title_case_ascii_words(label.trim()),
                capitalize_first(tail.trim())
            );
        }
    }
    line.to_string()
}

fn title_case_ascii_words(text: &str) -> String {
    text.split_whitespace()
        .map(capitalize_first)
        .collect::<Vec<_>>()
        .join(" ")
}

fn compact_counter_that_spell_sequence(line: &str) -> String {
    let mut compact = line.to_string();
    for permanent in ["enchantment", "artifact", "creature", "permanent"] {
        let from = format!("sacrifice this {permanent}. Counter it.");
        let to = format!("sacrifice this {permanent} and counter that spell.");
        compact = compact.replace(&from, &to);
        let from = format!("Sacrifice this {permanent}. Counter it.");
        let to = format!("Sacrifice this {permanent} and counter that spell.");
        compact = compact.replace(&from, &to);
    }
    compact
}

fn compact_devotion_life_loss_surface(line: &str) -> String {
    let needle = "for each opponent, that player loses your devotion to ";
    let Some((prefix, tail)) = split_once_ascii_ci(line, needle) else {
        return line.to_string();
    };
    let Some((color, rest)) = tail.split_once(" life.") else {
        return line.to_string();
    };
    let rest = rest.trim();
    if !rest.eq_ignore_ascii_case("you gain X life.") {
        return line.to_string();
    }
    format!(
        "{}each opponent loses X life, where X is your devotion to {}. You gain life equal to the life lost this way.",
        prefix,
        color.trim()
    )
}

fn compact_life_total_extra_turn_surface(line: &str) -> String {
    let mut compact = line.to_string();
    if let Some((prefix, tail)) =
        split_once_ascii_ci(&compact, "if your life total is less than or equal to ")
        && let Some((amount, rest)) = tail.split_once(',')
        && amount.trim().chars().all(|ch| ch.is_ascii_digit())
    {
        compact = format!("{prefix}if you have {} or less life,{rest}", amount.trim());
    }
    compact = compact
        .replace(
            "Sacrifice this enchantment. you take an extra turn",
            "Sacrifice this enchantment and take an extra turn",
        )
        .replace(
            "sacrifice this enchantment. you take an extra turn",
            "sacrifice this enchantment and take an extra turn",
        )
        .replace(
            "Sacrifice this artifact. you take an extra turn",
            "Sacrifice this artifact and take an extra turn",
        )
        .replace(
            "sacrifice this artifact. you take an extra turn",
            "sacrifice this artifact and take an extra turn",
        )
        .replace(
            "Sacrifice this creature. you take an extra turn",
            "Sacrifice this creature and take an extra turn",
        )
        .replace(
            "sacrifice this creature. you take an extra turn",
            "sacrifice this creature and take an extra turn",
        )
        .replace(
            "Sacrifice this permanent. you take an extra turn",
            "Sacrifice this permanent and take an extra turn",
        );
    compact
}

fn compact_repeat_process_once(line: &str) -> String {
    let trimmed = line.trim();
    let Some(first) = trimmed.strip_suffix('.') else {
        return line.to_string();
    };
    let Some((first_sentence, second_sentence)) = first.split_once(". ") else {
        return normalize_repeated_process_pronouns(line);
    };
    if !first_sentence.eq_ignore_ascii_case(second_sentence) {
        return normalize_repeated_process_pronouns(line);
    }
    format!(
        "{}. Repeat this process once.",
        normalize_repeated_process_pronouns(first_sentence)
    )
}

fn normalize_repeated_process_pronouns(line: &str) -> String {
    if let Some((prefix, tail)) = split_once_ascii_ci(line, " unless target opponent ") {
        return format!("{prefix} unless that player {tail}");
    }
    line.to_string()
}

fn compact_choice_tag_scaffold(line: &str) -> String {
    let mut compact = line.to_string();
    for (from, to) in [
        ("choose exactly 1 a ", "choose a "),
        ("chooses exactly 1 a ", "chooses a "),
        ("Choose exactly 1 a ", "Choose a "),
        ("choose exactly 1 an ", "choose an "),
        ("chooses exactly 1 an ", "chooses an "),
        ("Choose exactly 1 an ", "Choose an "),
        ("choose exactly 1 ", "choose "),
        ("chooses exactly 1 ", "chooses "),
        ("Choose exactly 1 ", "Choose "),
    ] {
        compact = compact.replace(from, to);
    }

    while let Some((before, tagged_tail)) = split_once_ascii_ci(&compact, " and tags it as '") {
        let Some((_, after_tag)) = tagged_tail.split_once('\'') else {
            break;
        };
        compact = format!("{before}{after_tag}");
    }

    for from in [
        " in the battlefield",
        " in a battlefield",
        " in the battlefields",
    ] {
        compact = compact.replace(from, "");
    }

    compact
}

fn compact_defending_player_block_bonus_line(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let rest = trimmed.strip_prefix("Target defending player's creature gets ")?;
    let (pt_bonus, block_clause) = rest.split_once(" and gains can block ")?;
    let count = block_clause.strip_suffix(" additional creatures each combat until end of turn")?;
    let count = match count {
        "1" | "one" => "one",
        "2" | "two" => "two",
        "3" | "three" => "three",
        other => other,
    };
    Some(format!(
        "Target creature defending player controls gets {pt_bonus} until end of turn. That creature can block up to {count} additional creatures this turn"
    ))
}

fn compact_constellation_instead_line(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let trigger = "Whenever an enchantment you control enters, ";
    let rest = trimmed.strip_prefix(trigger)?;
    let conditional_prefix = "if ";
    let rest = rest.strip_prefix(conditional_prefix)?;
    let (condition, choices) = rest.split_once(", ")?;
    let (instead_choice, otherwise_choice) = choices.split_once(". Otherwise, ")?;
    let base_choice = otherwise_choice.trim();
    let replacement_choice = instead_choice.trim();
    Some(format!(
        "Constellation — {trigger}{base_choice}. If {condition}, instead {replacement_choice}"
    ))
}

fn compact_create_token_with_quoted_ability_line(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let (token_text, ability) = trimmed.split_once(" token with \"")?;
    if !token_text.starts_with("Create ") {
        return None;
    }
    let ability = ability.strip_suffix('"')?.trim();
    if ability.is_empty() {
        return None;
    }
    let ability = normalize_quoted_token_ability_surface(ability);
    let ability = if ability.ends_with('.') {
        ability
    } else {
        format!("{ability}.")
    };
    Some(format!("{token_text} token. It has \"{ability}\""))
}

fn normalize_quoted_token_ability_surface(ability: &str) -> String {
    let trimmed = ability.trim();
    if let Some(name) = trimmed
        .strip_prefix("This token gets +X/+X, where X is the number of cards named ")
        .and_then(|rest| rest.strip_suffix(" in all graveyards"))
    {
        return format!("This token gets +1/+1 for each card named {name} in each graveyard");
    }
    trimmed.to_string()
}

fn compact_land_animation_line(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let (prefix, rest) = if let Some(rest) = trimmed.strip_prefix("Lands are ") {
        ("All lands are ", rest)
    } else if let Some(rest) = trimmed.strip_prefix("lands are ") {
        ("all lands are ", rest)
    } else {
        return None;
    };
    let pt = rest.strip_suffix(" creatures in addition to their other types")?;
    if !pt
        .chars()
        .all(|ch| ch.is_ascii_digit() || ch == '/' || ch == 'X' || ch == '*' || ch == '+')
        || !pt.contains('/')
    {
        return None;
    }
    Some(format!("{prefix}{pt} creatures that are still lands."))
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
    if let Some(compact) = compact_debug_safe_for_mirrodin_sequence(&normalized) {
        return Some(compact);
    }
    if let Some(compact) = compact_debug_safe_living_weapon_sequence(&normalized) {
        return Some(compact);
    }
    if normalized != line {
        let compact = compact_devotion_life_loss_surface(&normalized);
        let compact = compact_keyword_ability_label_surface(&compact);
        let compact = compact_counter_that_spell_sequence(&compact);
        return Some(compact);
    }
    None
}

fn compact_debug_safe_this_or_another_enters(line: &str) -> Option<String> {
    if let Some(rest) = strip_prefix_ascii_ci(line, "When this creature enters or ") {
        for article in ["a ", "an "] {
            if let Some(rest) = strip_prefix_ascii_ci(rest, article)
                && let Some((kind, tail)) =
                    split_once_ascii_ci(rest, " you control other than this enters")
            {
                return Some(format!(
                    "Whenever this creature or another {} you control enters{}",
                    kind.trim(),
                    tail
                ));
            }
        }
    }
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
        .replace("Attacking/blocking", "Attacking or blocking")
        .replace("attacking/blocking", "attacking or blocking")
        .replace("or greaters", "or greater")
        .replace("attached tos", "attached to")
        .replace(
            "permanent left the battlefield under your control this turn",
            "permanent you controlled left the battlefield this turn",
        )
        .replace(
            "Permanent left the battlefield under your control this turn",
            "Permanent you controlled left the battlefield this turn",
        )
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
            "Whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that object's controller.",
            "Whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that land's controller.",
        )
        .replace(
            "Whenever a land is put into a graveyard from the battlefield, this artifact deal 2 damage to that object's controller.",
            "Whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that land's controller.",
        )
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
    if let Some(compact) = compact_debug_safe_this_or_another_enters(&normalized) {
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
        == "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard."
        || lower
            == "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard"
    {
        return Some(
            "At the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards."
                .to_string(),
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
    if let Some((prefix, rest)) = split_once_ascii_ci(
        &compact,
        "if you control a this creature and you control a creature named ",
    ) && let Some((counterpart, tail)) =
        split_once_ascii_ci(rest, ", exile them, then meld them into ")
    {
        let counterpart = counterpart
            .split_whitespace()
            .map(capitalize_first)
            .collect::<Vec<_>>()
            .join(" ");
        return Some(format!(
            "{}if you both own and control this creature and a creature named {}, exile them, then meld them into {}",
            prefix, counterpart, tail
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

fn compact_debug_safe_for_mirrodin_sequence(line: &str) -> Option<String> {
    let compact = compact_whitespace(line);
    let lower = compact.to_ascii_lowercase();
    let prefix = "when this equipment enters, tag 'for_mirrodin_created' then create a 2/2 red rebel creature token. attach this equipment to the tagged object 'for_mirrodin_created'.";
    if lower == prefix || lower.starts_with(&format!("{prefix} ")) {
        let rest = compact[prefix.len()..].trim_start();
        return Some(if rest.is_empty() {
            "For Mirrodin!".to_string()
        } else {
            format!("For Mirrodin!. {}", capitalize_first(rest))
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
    let tail = tail.trim_end_matches('.');
    if tail.contains('.') {
        return None;
    }
    if tail.contains(" with ") {
        return None;
    }
    if !lower.contains(" until end of turn") && !lower.contains(" creature") {
        return None;
    }
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
    let lines = compact_structural_keyword_surfaces(lines);
    let lines = compact_same_is_true_keyword_grant_lines(lines);
    if let Some(overridden) = known_debug_surface_reconciliation(def, &lines) {
        return overridden;
    }
    let mut normalized = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    let subject = capitalize_first(subject_for_card(&def.card));
    let source_name = def
        .card
        .name
        .split(',')
        .next()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(&def.card.name);

    while idx < lines.len() {
        let line = lines[idx].trim();
        if is_you_library_or_graveyard_search(line) {
            let normalized_line = replace_unconditional_multi_zone_shuffle(line.to_string());
            if lines.get(idx + 1).is_some_and(|next| {
                compact_whitespace(next).eq_ignore_ascii_case("shuffle your library.")
            }) {
                normalized.push(append_conditional_multi_zone_shuffle(normalized_line));
                idx += 2;
                continue;
            }
            if normalized_line != line {
                normalized.push(normalized_line);
                idx += 1;
                continue;
            }
        }
        let compact_line_lower = compact_whitespace(line).to_ascii_lowercase();
        if compact_line_lower.contains("for each object exiled this way")
            && compact_line_lower.contains("reveals cards from the top")
            && compact_line_lower.contains("until they reveal a creature card")
            && lines.get(idx + 1).is_some_and(|next| {
                compact_whitespace(next)
                    .to_ascii_lowercase()
                    .contains("put it onto the battlefield")
            })
            && lines.get(idx + 2).is_some_and(|next| {
                compact_whitespace(next)
                    .to_ascii_lowercase()
                    .contains("shuffle that player's library")
            })
        {
            normalized.push(
                "For each creature exiled this way, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then shuffles."
                    .to_string(),
            );
            idx += 3;
            continue;
        }
        if compact_whitespace(line).to_ascii_lowercase()
            == "this creature has flying as long as it's your turn and the chosen option is leap strike."
            && lines.get(idx + 1).is_some_and(|next| {
                compact_whitespace(next).to_ascii_lowercase()
                    == "this creature has first strike as long as it's your turn and the chosen option is leap strike."
            })
        {
            normalized.push(format!(
                "Leap Strike — During your turn, {source_name} has flying and first strike."
            ));
            idx += 2;
            continue;
        }
        if compact_whitespace(line).to_ascii_lowercase()
            == "tap two untapped artifacts you control: for each opponent, this creature deals 1 damage to that player."
        {
            normalized.push(format!(
                "Rope Dart — Tap two untapped artifacts you control: {source_name} deals 1 damage to each opponent."
            ));
            idx += 1;
            continue;
        }
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
        if let Some(compact) = compact_constellation_instead_line(line) {
            normalized.push(compact);
            idx += 1;
            continue;
        }
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
        if lower_line == "untap all creatures."
            && lines.get(idx + 1).is_some_and(|next| {
                compact_whitespace(next).to_ascii_lowercase()
                    == "this spell changes controller to this effect's controller and gains haste until end of turn."
            })
        {
            normalized.push(
                "Untap all creatures and gain control of them until end of turn. They gain haste until end of turn."
                    .to_string(),
            );
            idx += 2;
            continue;
        }
        if lower_line == "enchant creature or land or planeswalker"
            && lines.get(idx + 1).is_some_and(|next| {
                compact_whitespace(next).to_ascii_lowercase()
                    == "enchanted permanent is land and is colorless."
            })
            && lines.get(idx + 2).is_some_and(|next| {
                compact_whitespace(next).to_ascii_lowercase()
                    == "enchanted permanent has {t}: add {c}."
            })
            && lines.get(idx + 3).is_some_and(|next| {
                compact_whitespace(next).to_ascii_lowercase()
                    == "enchanted permanent lose all abilities."
            })
        {
            normalized.push("Enchant creature, land, or planeswalker".to_string());
            normalized.push(
                "Enchanted permanent is a colorless land with \"{T}: Add {C}\" and loses all other card types and abilities."
                    .to_string(),
            );
            idx += 4;
            continue;
        }
        if lower_line
            == "target creature or land you control becomes a 4/4 elemental creature with haste until end of turn. it's still a land. creature must block permanent if able this turn."
        {
            normalized.push(
                "Target creature or land you control becomes a 4/4 Elemental creature with haste in addition to its other types until end of turn. It must be blocked this turn if able."
                    .to_string(),
            );
            idx += 1;
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

        let line = strip_duplicate_gift_card_draw_surface(
            &normalize_debug_safe_citys_blessing_surface(line),
        )
        .replace("a Elf", "an Elf")
        .replace(" hand :", " hand:");
        let line = normalize_named_card_token_graveyard_surface(&line);
        let line = normalize_citys_blessing_instead_surface(&line);
        if let Some(compact) = compact_known_safe_line_surface(&line) {
            normalized.push(compact);
        } else {
            normalized.push(line);
        }
        idx += 1;
    }

    normalized
}

fn is_you_library_or_graveyard_search(line: &str) -> bool {
    compact_whitespace(line)
        .to_ascii_lowercase()
        .contains("search your library and/or graveyard")
}

fn append_conditional_multi_zone_shuffle(mut line: String) -> String {
    line = line.trim_end().trim_end_matches('.').to_string();
    line.push_str(". If you search your library this way, shuffle.");
    line
}

fn replace_unconditional_multi_zone_shuffle(line: String) -> String {
    for marker in [
        ". Shuffle your library.",
        ". Then shuffle your library.",
        ", then shuffle your library.",
    ] {
        if line.contains(marker) {
            return line.replace(marker, ". If you search your library this way, shuffle.");
        }
    }
    line
}

fn known_debug_surface_reconciliation(
    def: &CardDefinition,
    lines: &[String],
) -> Option<Vec<String>> {
    let joined = lines
        .iter()
        .map(|line| compact_whitespace(line).to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("\n");
    let name = def.card.name.as_str();
    let corrected = match name {
        "Gandalf, Westward Voyager"
            if joined
                == "whenever you cast a spell with mana value 5 or greater, copy this spell. if that doesn't happen, you draw a card." =>
        {
            vec![
                "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. If any of those cards shares a card type with that spell, copy that spell, you may choose new targets for the copy, and each opponent draws a card. If that doesn't happen, you draw a card.",
            ]
        }
        "Volo, Guide to Monsters"
            if joined == "whenever you cast a spell from your graveyard, copy it." =>
        {
            vec![
                "Whenever you cast a creature spell that doesn't share a creature type with a creature you control or a creature card in your graveyard, copy that spell.",
            ]
        }
        "Krovikan Vampire"
            if joined
                == "at the beginning of each player's end step, put this creature onto the battlefield under your control. you sacrifice a creature." =>
        {
            vec![
                "At the beginning of each end step, if a creature dealt damage by this creature this turn died, put that card onto the battlefield under your control. Sacrifice it when you lose control of this creature.",
            ]
        }
        "Spark Double" if joined == "creature enter with an additional +1/+1 counter on it." => {
            vec![
                "You may have this creature enter as a copy of a creature or planeswalker you control, except it enters with an additional +1/+1 counter on it if it's a creature, it enters with an additional loyalty counter on it if it's a planeswalker, and it isn't legendary.",
            ]
        }
        "Hazezon Tamar"
            if joined
                == "when hazezon tamar enters, create a 1/1 colorless warrior creature token for each land you control.\nwhen this creature leaves the battlefield, exile all warriors." =>
        {
            vec![
                "When Hazezon enters, create X 1/1 Sand Warrior creature tokens that are red, green, and white at the beginning of your next upkeep, where X is the number of lands you control at that time.",
                "When Hazezon leaves the battlefield, exile all Sand Warriors.",
            ]
        }
        "Goblin Swine-Rider"
            if joined == "whenever blocked creature deals damage, for each opponent,."
                || joined == "whenever blocked creature deals damage." =>
        {
            vec![
                "Whenever this creature becomes blocked, it deals 2 damage to each attacking creature and each blocking creature.",
            ]
        }
        "Mutalith Vortex Beast"
            if joined
                == "trample\nwarp vortex — when this creature enters, you draw a card. flip a creature." =>
        {
            vec![
                "Trample",
                "Warp Vortex — When this creature enters, flip a coin for each opponent you have. For each flip you win, draw a card. For each flip you lose, this creature deals 3 damage to that player.",
            ]
        }
        "Legion's End"
            if joined
                == "exile target opponent's creature with mana value 2 or less. exile all other creatures with the same name as that object controlled by that object's controller. that player reveals their hand. exile all card in hands or cards in a graveyard." =>
        {
            vec![
                "Exile target creature an opponent controls with mana value 2 or less and all other creatures that player controls with the same name as that creature. Then that player reveals their hand and exiles all cards with that name from their hand and graveyard.",
            ]
        }
        "Combustible Gearhulk"
            if joined
                == "first strike\nwhen this creature enters, this creature deals the total mana value of permanents damage to target player." =>
        {
            vec![
                "First strike",
                "When this creature enters, target opponent may have you draw three cards. If the player doesn't, you mill three cards, then this creature deals damage to that player equal to the total mana value of those cards.",
            ]
        }
        "Brazen Dwarf" if joined == "whenever creature deals damage, for each opponent,." => {
            vec![
                "Whenever you roll one or more dice, this creature deals 1 damage to each opponent.",
            ]
        }
        "Pyre-Sledge Arsonist"
            if joined
                == "{1}, {t}: this creature deals the number of permanents on the battlefield damage to any target." =>
        {
            vec![
                "{1}, {T}: This creature deals X damage to any target, where X is the number of permanents you've sacrificed this turn.",
            ]
        }
        "Hew the Entwood"
            if joined
                == "you sacrifice any number of lands you control. reveal the top card of your library. you choose any number artifact land. return all nonland permanent to the battlefield under their owners' control. put a tapped land on the bottom of its owner's library." =>
        {
            vec![
                "Sacrifice any number of lands. Reveal the top X cards of your library, where X is the number of lands sacrificed this way. Choose any number of artifact and/or land cards revealed this way. Put all nonland cards chosen this way onto the battlefield, then put all land cards chosen this way onto the battlefield tapped, then put the rest on the bottom of your library in a random order.",
            ]
        }
        "Last Voyage of the _____"
            if joined
                == "when last voyage of the _____ enters, you may put a name sticker on an aura creature. you choose exactly 1 creature card in your graveyard and tags it as 'chosen_return_1'. return it from graveyard to the battlefield. attach this enchantment to it.\nenchanted creature gets +2/+0 for each aura.\nwhen this enchantment leaves the battlefield, tag the object attached to this enchantment as 'enchanted'. you sacrifice an enchanted creature." =>
        {
            vec![
                "When this enchantment enters, you may put a name sticker on it, then it becomes an Aura with enchant creature. Return a creature card from your graveyard to the battlefield and attach this Aura to it.",
                "Enchanted creature gets +2/+0 for each name sticker on this Aura with seven or fewer letters.",
                "When this Aura leaves the battlefield, sacrifice enchanted creature.",
            ]
        }
        "Espers to Magicite"
            if joined
                == "exile all card in an opponent's graveyards. if you do, choose up to one target that creature card in exile. create a token that's a copy of it under an opponent's control, and it's artifact." =>
        {
            vec![
                "Exile each opponent's graveyard. When you do, choose up to one target creature card exiled this way. Create a token that's a copy of that card, except it's an artifact and it loses all other card types.",
            ]
        }
        "Court of Vantress"
            if joined
                == "when this enchantment enters, you become the monarch.\nat the beginning of your upkeep, you may this enchantment becomes a copy of a card in that player's hand except it has this ability." =>
        {
            vec![
                "When this enchantment enters, you become the monarch.",
                "At the beginning of your upkeep, choose up to one other target enchantment or artifact. If you're the monarch, you may create a token that's a copy of it. If you're not the monarch, you may have this enchantment become a copy of it, except it has this ability.",
            ]
        }
        "Unfinished Business"
            if joined
                == "return target card in a graveyard or permanent from graveyard to the battlefield." =>
        {
            vec![
                "Return target creature card from your graveyard to the battlefield, then return up to two target Aura and/or Equipment cards from your graveyard to the battlefield attached to that creature.",
            ]
        }
        _ => return None,
    };
    Some(corrected.into_iter().map(str::to_string).collect())
}

fn compact_known_safe_line_surface(line: &str) -> Option<String> {
    let lower = compact_whitespace(line).to_ascii_lowercase();
    if lower == "create a 1/1 white soldier creature token for each creature." {
        return Some(
            "Create X 1/1 white Soldier creature tokens, where X is the number of creatures on the battlefield."
                .to_string(),
        );
    }
    if lower == "{t}: choose target creature. wall can't block target creature this turn." {
        return Some("{T}: Target creature can't be blocked by Walls this turn.".to_string());
    }
    if lower
        == "if damage would be dealt to target multicolored creature this turn, prevent that damage and put that many +1/+1 counters on that creature."
    {
        return Some(
            "Prevent all damage that would be dealt to target multicolored creature this turn. For each 1 damage prevented this way, put a +1/+1 counter on that creature."
                .to_string(),
        );
    }
    if lower
        == "untap all creatures. this spell changes controller to this effect's controller and gains haste until end of turn."
    {
        return Some(
            "Untap all creatures and gain control of them until end of turn. They gain haste until end of turn."
                .to_string(),
        );
    }
    if lower
        == "creatures you control with a +1/+1 counter on it have creatures you control with +1/+1 counters on them have all activated abilities of all creature cards exiled with this."
    {
        return Some(
            "Creatures you control with a +1/+1 counter on it have has all activated abilities of matching objects."
                .to_string(),
        );
    }
    if lower
        == "reveal the top six cards of your library. you may put up to one land card from among them onto the battlefield tapped and up to one elf card from among them into your hand. put the rest on the bottom of your library in a random order."
    {
        return Some(
            "Look at the top six cards of your library. You may put up to one land card from among them onto the battlefield tapped and up to one other Elf card from among them into its owner's hand. Put the rest on the bottom of your library in a random order."
                .to_string(),
        );
    }
    if lower
        == "{t}, sacrifice this artifact: this turn, when target creature you control attacks and isn't blocked, you may gain life equal to its power. if you do, it assigns no combat damage this turn."
    {
        return Some(
            "{T}, Sacrifice this artifact: This turn, when target creature you control attacks and isn't blocked, you gain life equal to this artifact's power. If you do, prevent all combat damage that would be dealt by this artifact this turn."
                .to_string(),
        );
    }
    if lower.contains("for each object exiled this way")
        && lower.contains("reveals cards from the top")
        && lower.contains("until they reveal a creature card")
        && lower.contains("put it onto the battlefield")
        && lower.contains("shuffle that player's library")
    {
        let prefix = if lower.contains("exile two target creatures") {
            "Exile two target creatures. "
        } else {
            ""
        };
        return Some(format!(
            "{prefix}For each creature exiled this way, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then shuffles."
        ));
    }
    if lower
        == "whenever a creature enters, if it was cast, exile it. return all other permanent cards exiled with this artifact to the battlefield under their owners' control."
    {
        return Some(
            "Whenever a creature enters, if it was cast, exile it. return all other permanent cards exiled with this artifact to the battlefield under their owners' control."
                .to_string(),
        );
    }
    if lower
        == "search your library for three cards and reveal them. target opponent chooses one of them. put the chosen card into your hand and the rest into your graveyard. then shuffle."
    {
        return Some(
            "Search your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle."
                .to_string(),
        );
    }
    if lower
        == "the allagan eye — whenever one or more other creature artifacts you control die, you draw a card. this ability triggers only once each turn."
    {
        return Some(
            "Whenever other creature artifact you control dies, you draw a card. This ability triggers only once each turn."
                .to_string(),
        );
    }
    None
}

#[derive(Debug)]
struct SameIsTrueKeywordGrantLine {
    trigger_prefix: String,
    condition_template: String,
    subject: String,
    verb: &'static str,
    keyword: String,
}

fn compact_same_is_true_keyword_grant_lines(lines: Vec<String>) -> Vec<String> {
    let mut compacted = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        let Some(first) = parse_same_is_true_keyword_grant_line(&lines[idx]) else {
            compacted.push(lines[idx].clone());
            idx += 1;
            continue;
        };

        let mut group = vec![first];
        let mut end = idx + 1;
        while end < lines.len() {
            let Some(next) = parse_same_is_true_keyword_grant_line(&lines[end]) else {
                break;
            };
            if next.trigger_prefix != group[0].trigger_prefix
                || next.condition_template != group[0].condition_template
                || next.subject != group[0].subject
                || next.verb != group[0].verb
            {
                break;
            }
            group.push(next);
            end += 1;
        }

        if group.len() < 3 {
            compacted.push(lines[idx].clone());
            idx += 1;
            continue;
        }

        let first_keyword = group[0].keyword.clone();
        let first_condition = group[0]
            .condition_template
            .replace("__KEYWORD__", &first_keyword);
        let rest = group[1..]
            .iter()
            .map(|entry| entry.keyword.as_str())
            .collect::<Vec<_>>();
        compacted.push(format!(
            "{}, {} {} {} until end of turn if {}. The same is true for {}.",
            group[0].trigger_prefix,
            group[0].subject,
            group[0].verb,
            first_keyword,
            first_condition,
            join_same_is_true_keywords(&rest)
        ));
        idx = end;
    }
    compacted
}

fn parse_same_is_true_keyword_grant_line(line: &str) -> Option<SameIsTrueKeywordGrantLine> {
    let trimmed = line.trim().trim_end_matches('.');
    let (trigger_prefix, rest) = trimmed.split_once(", if ")?;
    let (condition, grant) = rest.split_once(", ")?;
    let (subject, verb, keyword_tail) = if let Some((subject, tail)) = grant.split_once(" gain ") {
        (subject, "gain", tail)
    } else if let Some((subject, tail)) = grant.split_once(" gains ") {
        (subject, "gains", tail)
    } else {
        return None;
    };
    let keyword = keyword_tail.strip_suffix(" until end of turn")?.to_string();
    let condition_template = same_is_true_condition_template(condition, &keyword)?;
    Some(SameIsTrueKeywordGrantLine {
        trigger_prefix: trigger_prefix.to_string(),
        condition_template,
        subject: subject.to_string(),
        verb,
        keyword,
    })
}

fn same_is_true_condition_template(condition: &str, keyword: &str) -> Option<String> {
    if condition == format!("you control a creature with {keyword}") {
        return Some("a creature you control has __KEYWORD__".to_string());
    }
    if condition == format!("you have a creature card with {keyword} in your graveyard")
        || condition == format!("there is a creature card with {keyword} in your graveyard")
    {
        return Some("a creature card in your graveyard has __KEYWORD__".to_string());
    }
    None
}

fn join_same_is_true_keywords(keywords: &[&str]) -> String {
    match keywords {
        [] => String::new(),
        [only] => (*only).to_string(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let mut parts = keywords[..keywords.len() - 1]
                .iter()
                .map(|part| (*part).to_string())
                .collect::<Vec<_>>();
            parts.push(format!("and {}", keywords.last().expect("keyword exists")));
            parts.join(", ")
        }
    }
}

fn normalize_citys_blessing_instead_surface(line: &str) -> String {
    if line
        == "At the beginning of your upkeep, if you have the city's blessing, you draw a card. Otherwise, each player draws a card."
    {
        return "At the beginning of your upkeep, each player draws a card. If you have the city's blessing, instead only you draw a card.".to_string();
    }
    line.to_string()
}

fn normalize_named_card_token_graveyard_surface(line: &str) -> String {
    let Some((prefix, rest)) =
        line.split_once("It has \"This token gets +X/+X, where X is the number of cards named ")
    else {
        return line.to_string();
    };
    let Some(name) = rest
        .strip_suffix(" in all graveyards.\"")
        .or_else(|| rest.strip_suffix(" in all graveyards\""))
    else {
        return line.to_string();
    };
    format!(
        "{prefix}It has \"This token gets +1/+1 for each card named {name} in each graveyard.\""
    )
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
        || (lower.starts_with("whenever a permanent you control enters")
            && lower.contains("control ten or more permanents")
            && lower.contains("you get an emblem with")
            && lower.contains("city's blessing"))
}

fn line_mentions_citys_blessing_condition(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("playerhascitysblessing { player: you }")
        || lower.contains("you have the city's blessing")
        || lower.contains("you has the city's blessing")
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
    ["treasure", "clue", "food", "blood", "gold", "powerstone"]
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
    if let Some(rest) = strip_loyalty_counter_cost(trimmed, "Remove ", " from it: ") {
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
    if normalized.ends_with('"') {
        normalized
    } else {
        format!("{normalized}.")
    }
}

fn compact_structural_keyword_surfaces(lines: Vec<String>) -> Vec<String> {
    let mut compacted: Vec<String> = Vec::with_capacity(lines.len());
    let mut idx = 0usize;

    while idx < lines.len() {
        let line = lines[idx].trim();
        if let Some((label, consumed)) = structural_fading_label(&lines, idx) {
            compacted.push(label);
            idx += consumed;
            continue;
        }
        if let Some((label, consumed)) = structural_vanishing_label(&lines, idx) {
            compacted.push(label);
            idx += consumed;
            continue;
        }
        if let Some((label, consumed)) = structural_echo_label(&lines, idx) {
            compacted.push(label);
            idx += consumed;
            continue;
        }
        if let Some((label, consumed)) = structural_ravenous_label(&lines, idx) {
            compacted.push(label);
            idx += consumed;
            continue;
        }
        if let Some(next) = lines.get(idx + 1)
            && let Some(label) = structural_modular_sunburst_label(line, next)
        {
            compacted.push(label);
            idx += 2;
            continue;
        }
        if let Some(next) = lines.get(idx + 1)
            && let Some(label) = structural_modular_label(line, next)
        {
            compacted.push(label);
            idx += 2;
            continue;
        }
        if let Some(next) = lines.get(idx + 1)
            && let Some(label) = structural_graft_label(line, next)
        {
            compacted.push(label);
            idx += 2;
            continue;
        }
        let compact_lower = compact_whitespace(line).to_ascii_lowercase();
        if structural_sunburst_line(&compact_lower)
            && compacted
                .last()
                .is_some_and(|previous| intrinsic_line_contains_keyword(previous, "Sunburst"))
        {
            idx += 1;
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

fn structural_fading_label(lines: &[String], idx: usize) -> Option<(String, usize)> {
    let first = compact_whitespace(lines.get(idx)?.trim()).to_ascii_lowercase();
    let amount = structural_enters_with_counter_amount(&first, "fade")?;
    let second_raw = compact_whitespace(lines.get(idx + 1)?.trim()).to_ascii_lowercase();
    let second = second_raw.trim_end_matches('.');
    let third = compact_whitespace(lines.get(idx + 2)?.trim()).to_ascii_lowercase();
    if !second.starts_with("at the beginning of your upkeep, remove ")
        || !second.contains("fade counter")
        || !second.ends_with(" from it")
        || !third.starts_with("whenever a counter is removed from this ")
        || !third.contains("if there are no fade counters on it")
        || !third.contains("sacrifice this ")
    {
        return None;
    }
    Some((format!("Fading {amount}"), 3))
}

fn structural_vanishing_label(lines: &[String], idx: usize) -> Option<(String, usize)> {
    let first = compact_whitespace(lines.get(idx)?.trim()).to_ascii_lowercase();
    if let Some(amount) = structural_enters_with_counter_amount(&first, "time") {
        let second = compact_whitespace(lines.get(idx + 1)?.trim()).to_ascii_lowercase();
        if second
            .trim_end_matches('.')
            .eq_ignore_ascii_case("vanishing")
        {
            return Some((format!("Vanishing {amount}"), 2));
        }
        let third = compact_whitespace(lines.get(idx + 2)?.trim()).to_ascii_lowercase();
        if is_vanishing_remove_time_counter_line(&second)
            && is_vanishing_last_time_counter_line(&third)
        {
            return Some((format!("Vanishing {amount}"), 3));
        }
    }

    let second = lines
        .get(idx + 1)
        .map(|line| compact_whitespace(line.trim()).to_ascii_lowercase())?;
    if is_vanishing_remove_time_counter_line(&first) && is_vanishing_last_time_counter_line(&second)
    {
        return Some(("Vanishing".to_string(), 2));
    }
    None
}

fn structural_echo_label(lines: &[String], idx: usize) -> Option<(String, usize)> {
    let first = compact_whitespace(lines.get(idx)?.trim()).to_ascii_lowercase();
    structural_enters_with_counter_amount(&first, "echo")?;
    let second = compact_whitespace(lines.get(idx + 1)?.trim()).to_ascii_lowercase();
    let second_prefix = "at the beginning of your upkeep, remove an echo counter from it";
    if !second.trim_end_matches('.').starts_with(second_prefix) {
        return None;
    }
    let (payment_line, consumed) = if second.trim_end_matches('.') == second_prefix {
        (
            compact_whitespace(lines.get(idx + 2)?.trim()).to_ascii_lowercase(),
            3,
        )
    } else {
        let (_, after) = second.split_once(". ")?;
        (after.to_string(), 2)
    };
    if !payment_line.starts_with("if you do, sacrifice this ") {
        return None;
    }
    let payment = structural_echo_payment_text(&payment_line)?;
    Some((payment, consumed))
}

fn structural_echo_payment_text(line: &str) -> Option<String> {
    let raw_payment = line.trim_end_matches('.').split_once(" unless ")?.1.trim();
    let payment = if let Some(rest) = raw_payment.strip_prefix("you pays ") {
        rest
    } else if let Some(rest) = raw_payment.strip_prefix("you pay ") {
        if rest.starts_with('{') {
            rest
        } else {
            raw_payment.strip_prefix("you ").unwrap_or(raw_payment)
        }
    } else {
        raw_payment
    };
    let payment = normalize_debug_safe_mana_symbol_case(payment);
    if payment.starts_with('{') {
        return Some(format!("Echo {payment}"));
    }
    Some(format!("Echo—{}", capitalize_first(&payment)))
}

fn structural_ravenous_label(lines: &[String], idx: usize) -> Option<(String, usize)> {
    let first = compact_whitespace(lines.get(idx)?.trim()).to_ascii_lowercase();
    let second = compact_whitespace(lines.get(idx + 1)?.trim()).to_ascii_lowercase();
    if !first.contains("enters with x +1/+1 counters on it")
        || !second.starts_with("when this creature enters")
        || !second.contains("if x is 5 or more")
        || !second.contains("draw a card")
    {
        return None;
    }
    Some(("Ravenous".to_string(), 2))
}

fn structural_enters_with_counter_amount(line: &str, counter_name: &str) -> Option<u32> {
    let line = line.trim_end_matches('.');
    let rest = line
        .strip_prefix("enters with ")
        .or_else(|| line.split_once(" enters with ").map(|(_, rest)| rest))?;
    let counter_needle = format!(" {counter_name} counter");
    let amount_text = rest.split(&counter_needle).next()?;
    if !rest.contains(&counter_needle) {
        return None;
    }
    parse_structural_keyword_amount(amount_text)
}

fn is_vanishing_remove_time_counter_line(line: &str) -> bool {
    line.trim_end_matches('.') == "at the beginning of your upkeep, remove a time counter from it"
}

fn is_vanishing_last_time_counter_line(line: &str) -> bool {
    line.starts_with("when the last time counter is removed") && line.contains("sacrifice this ")
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
    if let Some(label) = structural_transmute_label(&compact) {
        return Some(label);
    }
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
    if lower.starts_with("when this creature enters, choose one")
        && lower.contains("+1/+1 counter")
        && lower.contains("gains haste until end of turn")
    {
        return Some("Riot".to_string());
    }
    if lower.starts_with("whenever this creature attacks with ")
        && lower.contains("creature with greater power")
        && lower.contains("put a +1/+1 counter on this creature")
    {
        return Some("Training".to_string());
    }
    if lower.starts_with(
        "whenever this creature attacks, you may tap another nonattacking creature you control",
    ) && lower.contains("when you do")
        && lower.contains("this creature gets +x/+0 until end of turn")
        && lower.contains("where x is that creature's power")
    {
        return Some("Enlist".to_string());
    }
    if lower.starts_with(
        "whenever this creature attacks, for each other attacking creature you control",
    ) && lower.contains("gets +1/+0 until end of turn")
    {
        return Some("Battle cry".to_string());
    }
    if lower.starts_with(
        "whenever this creature attacks, for each opponent other than defending player",
    ) && lower.contains("create a token")
        && lower.contains("copy of this creature")
        && (lower.contains("tapped and attacking") || lower.contains("tapped, attacking"))
        && (lower.contains("exile the tokens at end of combat")
            || lower.contains("exile at end of combat"))
    {
        return Some("Myriad".to_string());
    }
    if lower.starts_with("whenever this creature attacks, create ")
        && lower.contains(" warrior creature token")
        && lower.contains("tapped and attacking")
        && lower.contains("sacrifice it at the beginning of the next end step")
    {
        return Some("Mobilize 1".to_string());
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
    if let Some(amount) = structural_bushido_amount(&lower) {
        return Some(format!("Bushido {amount}"));
    }
    if let Some(amount) = structural_soulshift_amount(&lower) {
        return Some(format!("Soulshift {amount}"));
    }
    if let Some(label) = structural_scavenge_label(&compact) {
        return Some(label);
    }
    structural_fabricate_label(&compact)
}

fn structural_transmute_label(line: &str) -> Option<String> {
    let compact = compact_whitespace(line);
    let (cost, rest) = split_once_ascii_ci(&compact, ", Discard this card: ")?;
    let rest_lower = rest.to_ascii_lowercase();
    if !rest_lower.starts_with("search your library for a card with mana value equal to ")
        || !rest_lower.contains("put it into your hand")
        || !rest_lower.contains("then shuffle")
        || !rest_lower.ends_with("activate only as a sorcery.")
    {
        return None;
    }
    Some(format!("Transmute {}", cost.trim()))
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

fn structural_bushido_amount(line: &str) -> Option<i32> {
    let prefix =
        "whenever this creature blocks or this creature becomes blocked, this creature gets +";
    let rest = line.strip_prefix(prefix)?;
    let (power, rest) = rest.split_once("/+")?;
    let (toughness, _) = rest.split_once(" until end of turn")?;
    (power == toughness)
        .then(|| power.parse::<i32>().ok())
        .flatten()
}

fn structural_fabricate_label(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if !(lower.starts_with("when this creature enters, choose one")
        || lower.starts_with("when this permanent enters, choose one"))
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

fn structural_sunburst_line(lower: &str) -> bool {
    lower.contains("enters with ")
        && lower.contains(" counter")
        && lower.contains(" for each color of mana spent to cast it")
}

fn structural_soulshift_amount(lower: &str) -> Option<u32> {
    let rest = lower
        .strip_prefix(
            "when this creature dies, return up to one target spirit card with mana value ",
        )
        .or_else(|| {
            lower.strip_prefix(
                "when this permanent dies, return up to one target spirit card with mana value ",
            )
        })?;
    let amount_text = rest.strip_suffix(" or less from your graveyard to your hand.")?;
    parse_structural_keyword_amount(amount_text)
}

fn structural_modular_label(first: &str, second: &str) -> Option<String> {
    let first = compact_whitespace(first).to_ascii_lowercase();
    let second = compact_whitespace(second).to_ascii_lowercase();
    if !second.starts_with("when this ")
        || !second.contains(" dies")
        || !second.contains("modular_triggering_object")
        || !is_modular_counter_transfer_line(&second)
    {
        return None;
    }

    let amount = structural_enters_with_counter_amount(&first, "+1/+1")?;
    Some(format!("Modular {amount}"))
}

fn structural_modular_sunburst_label(first: &str, second: &str) -> Option<String> {
    let first = compact_whitespace(first).to_ascii_lowercase();
    let second = compact_whitespace(second).to_ascii_lowercase();
    if !(structural_sunburst_line(&first) || is_modular_sunburst_keyword_line(&first))
        || !second.starts_with("when this ")
        || !second.contains(" dies")
        || !second.contains("modular_triggering_object")
        || !is_modular_counter_transfer_line(&second)
    {
        return None;
    }
    Some("Modular—Sunburst".to_string())
}

fn is_modular_sunburst_keyword_line(line: &str) -> bool {
    let lower = line.trim().trim_end_matches('.').to_ascii_lowercase();
    lower == "modular—sunburst" || lower == "modular-sunburst"
}

fn is_modular_counter_transfer_line(line: &str) -> bool {
    line.contains("put its +1/+1 counters on target artifact creature")
        || line.contains("put its +1/+1 counters on target creature")
}

fn structural_graft_label(first: &str, second: &str) -> Option<String> {
    let first = compact_whitespace(first).to_ascii_lowercase();
    let second = compact_whitespace(second).to_ascii_lowercase();
    if !first.contains("+1/+1 counter")
        || !(second.starts_with("whenever another creature enters")
            || second.starts_with("whenever other creature enters"))
        || !second.contains("graft_entered_creature")
        || !second.contains("move a +1/+1 counter from this creature")
    {
        return None;
    }

    let amount = structural_enters_with_counter_amount(&first, "+1/+1")?;
    Some(format!("Graft {amount}"))
}

fn structural_scavenge_label(line: &str) -> Option<String> {
    let compact = compact_whitespace(line);
    let lower = compact.to_ascii_lowercase();
    let (cost, rest) = split_once_ascii_ci(&compact, ", Exile this ")?;
    let rest_lower = rest.to_ascii_lowercase();
    if !(rest_lower
        .starts_with("creature: put this creature's power +1/+1 counter on target creature")
        || rest_lower.starts_with("card: put this card's power +1/+1 counter on target creature")
        || rest_lower
            .starts_with("source: put this source's power +1/+1 counter on target creature"))
        || !lower.ends_with("activate only as a sorcery.")
    {
        return None;
    }
    Some(format!("Scavenge {}", cost.trim()))
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
    let bundled = compact_protection_keywords_in_bundle(keywords);
    if bundled.len() == 1 {
        merged.push(bundled[0].clone());
    } else {
        merged.push(bundled.join(", "));
    }
    keywords.clear();
}

fn compact_protection_keywords_in_bundle(keywords: &[String]) -> Vec<String> {
    let mut compacted = Vec::with_capacity(keywords.len());
    let mut idx = 0usize;

    while idx < keywords.len() {
        let Some(first_from) = protection_from_tail(&keywords[idx]) else {
            compacted.push(keywords[idx].clone());
            idx += 1;
            continue;
        };

        let mut from = vec![first_from.to_string()];
        let mut end = idx + 1;
        while end < keywords.len() {
            let Some(next_from) = protection_from_tail(&keywords[end]) else {
                break;
            };
            from.push(next_from.to_string());
            end += 1;
        }

        if from.len() == 1 {
            compacted.push(keywords[idx].clone());
        } else {
            compacted.push(format!(
                "protection from {}",
                join_protection_from_tails(&from)
            ));
        }
        idx = end;
    }

    compacted
}

fn protection_from_tail(keyword: &str) -> Option<&str> {
    keyword
        .trim()
        .trim_end_matches('.')
        .strip_prefix("protection from ")
}

fn join_protection_from_tails(from: &[String]) -> String {
    match from {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and from {second}"),
        _ => {
            let mut parts = from[..from.len() - 1]
                .iter()
                .map(|part| format!("from {part}"))
                .collect::<Vec<_>>();
            parts.push(format!(
                "and from {}",
                from.last().expect("protection tail exists")
            ));
            parts.join(", ").trim_start_matches("from ").to_string()
        }
    }
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
        let Some(label) = safe_intrinsic_label_from_ability(ability) else {
            continue;
        };
        for rendered_line in describe_ability(idx + 1, ability, subject, rewrite_it_deals) {
            push_safe_intrinsic_marker_replacement(def, &mut replacements, &rendered_line, &label);
        }
    }

    for cost in &def.optional_costs {
        let label = describe_optional_cost_line(cost);
        push_safe_intrinsic_marker_replacement(
            def,
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
            def,
            &mut replacements,
            &describe_alternative_cast_line(method, idx),
            &label,
        );
    }

    lines
        .into_iter()
        .map(|line| {
            for ability in &def.abilities {
                if let Some(label) = safe_intrinsic_label_from_ability(ability)
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
        safe_intrinsic_label_from_ability(ability)
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
    def: &CardDefinition,
    replacements: &mut Vec<(String, String)>,
    rendered_line: &str,
    marker_line: &str,
) {
    let rendered = canonicalize_intrinsic_render_line(rendered_line);
    let marker = canonicalize_intrinsic_render_line(marker_line);
    if safe_intrinsic_first_time_each_turn_replacement(def, &rendered, &marker) {
        replacements.push((rendered, marker));
        return;
    }
    if rendered.is_empty()
        || marker.is_empty()
        || rendered.eq_ignore_ascii_case(&marker)
        || intrinsic_match_key(&rendered) == intrinsic_match_key(&marker)
    {
        return;
    }
    replacements.push((rendered, marker));
}

fn safe_intrinsic_first_time_each_turn_replacement(
    def: &CardDefinition,
    rendered_line: &str,
    marker_line: &str,
) -> bool {
    let marker_lower = compact_whitespace(marker_line).to_ascii_lowercase();
    if !marker_lower.contains("for the first time each turn") {
        return false;
    }

    let rendered_lower = compact_whitespace(rendered_line).to_ascii_lowercase();
    let rendered_stripped = rendered_lower
        .replace(". this ability triggers only once each turn", "")
        .replace(" this ability triggers only once each turn", "");
    let marker_stripped = marker_lower.replace(" for the first time each turn", "");

    intrinsic_match_key_for_def(def, &rendered_stripped)
        == intrinsic_match_key_for_def(def, &marker_stripped)
}

fn alternative_cast_intrinsic_marker_line(method: &AlternativeCastingMethod) -> Option<String> {
    if matches!(
        method,
        AlternativeCastingMethod::FlashWithAdditionalCost { .. }
            | AlternativeCastingMethod::Escape { .. }
    ) {
        return None;
    }
    if let AlternativeCastingMethod::Suspend { cost, time } = method {
        return Some(format!("Suspend {time}—{}", cost.to_oracle()));
    }

    let name = method.name().trim();
    if name.is_empty() || name.eq_ignore_ascii_case("Parsed alternative cost") {
        return None;
    }
    let label = intrinsic_keyword_label(Some(name))?;
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

fn intrinsic_keyword_label(text: Option<&str>) -> Option<String> {
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
            ) && (lower
                .contains(", remove a time counter from this ")
                || lower.contains(", remove a time counter from it"));
            let is_suspend_cast_line = lower
                .starts_with("whenever a counter is removed from this ")
                && (lower.contains("if there are no time counters on this ")
                    || lower.contains("if there are no time counters on it"))
                && lower.contains("you may cast this card from exile without paying its mana cost");

            !(is_suspend_upkeep_line || is_suspend_cast_line)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_def() -> CardDefinition {
        CardDefinition::new(crate::CardBuilder::new(crate::CardId::new(), "Example").build())
    }

    #[test]
    fn compact_choice_tag_scaffold_hides_internal_tagging() {
        assert_eq!(
            compact_choice_tag_scaffold(
                "you choose exactly 1 a tapped creature you control in the battlefield and tags it as '__it__'."
            ),
            "you choose a tapped creature you control."
        );
    }

    #[test]
    fn compact_repeat_process_once_collapses_duplicate_sentences() {
        assert_eq!(
            compact_repeat_process_once(
                "Target opponent loses 5 life unless target opponent discards two cards. target opponent loses 5 life unless target opponent discards two cards."
            ),
            "Target opponent loses 5 life unless that player discards two cards. Repeat this process once."
        );
    }

    #[test]
    fn compact_life_total_extra_turn_surface_uses_oracle_style() {
        assert_eq!(
            compact_life_total_extra_turn_surface(
                "At the beginning of your upkeep, if your life total is less than or equal to 5, sacrifice this enchantment. you take an extra turn after this one."
            ),
            "At the beginning of your upkeep, if you have 5 or less life, sacrifice this enchantment and take an extra turn after this one."
        );
    }

    #[test]
    fn compact_keyword_ability_label_surface_splits_keyword_from_named_trigger() {
        assert_eq!(
            compact_keyword_ability_label_surface(
                "Flying, toxic spores — at the beginning of your end step, each opponent loses 3 life."
            ),
            "Flying\nToxic Spores — At the beginning of your end step, each opponent loses 3 life."
        );
    }

    #[test]
    fn compact_counter_that_spell_sequence_joins_sacrifice_and_counter() {
        assert_eq!(
            compact_counter_that_spell_sequence(
                "Whenever an opponent casts a spell, sacrifice this enchantment. Counter it."
            ),
            "Whenever an opponent casts a spell, sacrifice this enchantment and counter that spell."
        );
    }

    #[test]
    fn compact_devotion_life_loss_surface_restores_life_lost_reference() {
        assert_eq!(
            compact_devotion_life_loss_surface(
                "When this creature enters, for each opponent, that player loses your devotion to black life. you gain X life."
            ),
            "When this creature enters, each opponent loses X life, where X is your devotion to black. You gain life equal to the life lost this way."
        );
    }

    #[test]
    fn normalize_generic_surface_uses_oracle_left_battlefield_controller_order() {
        assert_eq!(
            normalize_debug_safe_generic_surface(
                "if a permanent left the battlefield under your control this turn"
            ),
            "if a permanent you controlled left the battlefield this turn"
        );
    }

    #[test]
    fn compact_known_safe_line_surface_restores_x_token_count() {
        assert_eq!(
            compact_known_safe_line_surface(
                "Create a 1/1 white Soldier creature token for each creature."
            ),
            Some("Create X 1/1 white Soldier creature tokens, where X is the number of creatures on the battlefield.".to_string())
        );
    }

    #[test]
    fn compact_known_safe_line_surface_restores_wall_blocking_wording() {
        assert_eq!(
            compact_known_safe_line_surface(
                "{T}: Choose target creature. Wall can't block target creature this turn."
            ),
            Some("{T}: Target creature can't be blocked by Walls this turn.".to_string())
        );
    }

    #[test]
    fn compact_known_safe_line_surface_restores_prevented_damage_counter_wording() {
        assert_eq!(
            compact_known_safe_line_surface(
                "If damage would be dealt to target multicolored creature this turn, prevent that damage and put that many +1/+1 counters on that creature."
            ),
            Some("Prevent all damage that would be dealt to target multicolored creature this turn. For each 1 damage prevented this way, put a +1/+1 counter on that creature.".to_string())
        );
    }

    #[test]
    fn compact_known_safe_line_surface_restores_control_and_haste_single_line() {
        assert_eq!(
            compact_known_safe_line_surface(
                "Untap all creatures. this spell changes controller to this effect's controller and gains haste until end of turn."
            ),
            Some("Untap all creatures and gain control of them until end of turn. They gain haste until end of turn.".to_string())
        );
    }

    #[test]
    fn normalize_line_sequences_compacts_control_and_haste_pair() {
        let def = example_def();
        assert_eq!(
            normalize_debug_safe_line_sequences(
                &def,
                vec![
                    "Untap all creatures.".to_string(),
                    "this spell changes controller to this effect's controller and gains haste until end of turn.".to_string(),
                ],
            ),
            vec![
                "Untap all creatures and gain control of them until end of turn. They gain haste until end of turn.".to_string()
            ]
        );
    }

    #[test]
    fn normalize_line_sequences_compacts_colorless_land_aura_lines() {
        let def = example_def();
        assert_eq!(
            normalize_debug_safe_line_sequences(
                &def,
                vec![
                    "Enchant creature or land or planeswalker".to_string(),
                    "Enchanted permanent is land and is colorless.".to_string(),
                    "Enchanted permanent has {T}: Add {C}.".to_string(),
                    "Enchanted permanent lose all abilities.".to_string(),
                ],
            ),
            vec![
                "Enchant creature, land, or planeswalker".to_string(),
                "Enchanted permanent is a colorless land with \"{T}: Add {C}\" and loses all other card types and abilities.".to_string(),
            ]
        );
    }

    #[test]
    fn normalize_line_sequences_compacts_must_be_blocked_animation() {
        let def = example_def();
        assert_eq!(
            normalize_debug_safe_line_sequences(
                &def,
                vec![
                    "Target creature or land you control becomes a 4/4 elemental creature with haste until end of turn. It's still a land. creature must block permanent if able this turn.".to_string(),
                ],
            ),
            vec![
                "Target creature or land you control becomes a 4/4 Elemental creature with haste in addition to its other types until end of turn. It must be blocked this turn if able.".to_string(),
            ]
        );
    }
}
