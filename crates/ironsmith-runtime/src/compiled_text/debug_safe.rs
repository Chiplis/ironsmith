use super::*;
use crate::text_cleanup::strip_parenthetical_text;

/// Render the structured runtime model for debug/inspector use.
///
/// This deliberately disables source/oracle-sensitive reconciliation. It first
/// renders abilities without source text, then applies only debug-safe
/// normalization owned by this module.
pub fn debug_compiled_lines(def: &CardDefinition) -> Vec<String> {
    let structured_def = debug_safe_surface_definition(def);
    normalize_debug_safe_surface(
        &structured_def,
        def,
        raw_compiled_lines_with_mode(&structured_def, CompiledTextMode::DebugSafe),
    )
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
        .map(|line| normalize_compiled_post_pass_effect(&normalize_common_semantic_phrasing(&line)))
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
    let compact_echo = compact_echo_keyword_marker_lines(safe_intrinsics);
    compact_echo
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
        .collect()
}

fn normalize_debug_safe_card_reference_surface(def: &CardDefinition, line: &str) -> String {
    let subject = subject_for_card(&def.card);
    line.replace("this source", subject)
        .replace("This source", &capitalize_first(subject))
}

fn normalize_debug_safe_oracle_like_surface(line: &str) -> String {
    if let Some(compact) = compact_debug_safe_pile_choice_surface(line) {
        return compact;
    }
    if let Some(compact) = normalize_divvy_chosen_sequence(line) {
        let compact = strip_parenthetical_text(&compact).trim().to_string();
        return compact_debug_safe_pile_choice_surface(&compact).unwrap_or(compact);
    }
    if let Some(compact) = compact_standard_named_token_payload_in_line(line) {
        return compact;
    }
    if let Some(compact) = compact_debug_safe_loyalty_line(line) {
        return compact;
    }
    line.to_string()
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
    if let Some(cost) = compact.strip_suffix(
        ", Discard this card: Search your library for a card with mana value equal to a dynamic value, put it into your hand, then shuffle. Activate only as a sorcery.",
    ) && cost.starts_with('{')
    {
        return Some(format!("Transmute {cost}"));
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

fn compact_debug_safe_pile_choice_surface(line: &str) -> Option<String> {
    let attack_marker = "that player chooses any number of creatures that player controls. Other creatures that player controls can't attack this turn.";
    if let Some((before, _)) = split_once_ascii_ci(line, attack_marker) {
        return Some(format!(
            "{}separate all creatures that player controls into two piles. Only creatures in the pile of their choice can attack this turn.",
            before
        ));
    }

    let block_marker = "that player chooses any number of creatures that player controls. Other creatures that player controls can't block this turn.";
    if let Some((before, _)) = split_once_ascii_ci(line, block_marker) {
        return Some(format!(
            "{}separate all creatures that player controls into two piles and that player chooses one. Only creatures in the chosen piles can block this turn.",
            before
        ));
    }

    None
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

        if end == idx + 1 {
            merged.push(lines[idx].clone());
        } else {
            let mut keywords = Vec::new();
            for keyword in lines[idx..end]
                .iter()
                .map(|line| normalize_intrinsic_keyword_marker_for_bundle(line))
            {
                if !keywords
                    .iter()
                    .any(|existing: &String| existing.eq_ignore_ascii_case(&keyword))
                {
                    keywords.push(keyword);
                }
            }
            merged.push(keywords.join(", "));
        }
        idx = end;
    }

    merged
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
    let normalized =
        normalize_compiled_post_pass_effect(&normalize_common_semantic_phrasing(&stripped));
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
