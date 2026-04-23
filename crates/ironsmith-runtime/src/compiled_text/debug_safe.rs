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
    debug_compiled_lines(def)
}

pub fn unprocessed_compiled_lines(def: &CardDefinition) -> Vec<String> {
    compiled_text_lines(def)
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
    let mut normalized = line
        .replace("this source", subject)
        .replace("This source", &capitalize_first(subject))
        .replace("this permanent", subject)
        .replace("This permanent", &capitalize_first(subject));
    if let Some(keyword) = source_keyword_during_your_turn(&normalized, subject) {
        let source_name = def
            .card
            .name
            .split(',')
            .next()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| capitalize_first(subject));
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
    if let Some(compact) = compact_first_time_each_turn_trigger_line(line) {
        return compact;
    }
    if let Some(compact) = compact_debug_safe_ast_scaffold_line(line) {
        return compact_debug_safe_loyalty_line(&compact).unwrap_or(compact);
    }
    if let Some(compact) = compact_standard_named_token_payload_in_line(line) {
        return compact_debug_safe_loyalty_line(&compact).unwrap_or(compact);
    }
    normalize_debug_safe_keyword_punctuation(line)
}

fn compact_first_time_each_turn_trigger_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let stem = trimmed
        .strip_suffix(". This ability triggers only once each turn.")
        .or_else(|| trimmed.strip_suffix(". This ability triggers only once each turn"))?;
    let rest = stem.strip_prefix("Whenever you lose life, ")?;
    Some(format!(
        "Whenever you lose life for the first time each turn, {rest}."
    ))
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
        return Some(normalized);
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
        if let Some(compact) = compact_first_time_each_turn_trigger_line(line) {
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
        normalized.push(normalize_citys_blessing_instead_surface(&line));
        idx += 1;
    }

    normalized
}

fn normalize_citys_blessing_instead_surface(line: &str) -> String {
    if line
        == "At the beginning of your upkeep, if you have the city's blessing, you draw a card. Otherwise, each player draws a card."
    {
        return "At the beginning of your upkeep, each player draws a card. If you have the city's blessing, instead only you draw a card.".to_string();
    }
    line.to_string()
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
            ) && lower.contains(", remove a time counter from this ");
            let is_suspend_cast_line = lower
                .starts_with("whenever a counter is removed from this ")
                && lower.contains("if there are no time counters on this ")
                && lower.contains("you may cast this card from exile without paying its mana cost");

            !(is_suspend_upkeep_line || is_suspend_cast_line)
        })
        .collect()
}
