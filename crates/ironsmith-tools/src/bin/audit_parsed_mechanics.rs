use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;

use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinitionBuilder;
use ironsmith::ids::CardId;
use ironsmith::static_abilities::StaticAbilityId;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug)]
struct Args {
    cards_path: String,
    limit: Option<usize>,
    allow_unsupported: bool,
    json_out: Option<String>,
    slice_mechanics: Vec<String>,
    slice_fallback_reasons: Vec<String>,
    fail_on_slice_hits: bool,
}

#[derive(Debug, Clone)]
struct CardInput {
    name: String,
    parse_input: String,
}

#[derive(Debug, Default, Clone, Serialize)]
struct MechanicTally {
    mechanic: String,
    ability_instances: usize,
    card_count: usize,
    example_cards: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonReport {
    cards_processed: usize,
    parse_successes: usize,
    parse_failures: usize,
    parse_success_cards_with_unimplemented_markers: usize,
    parse_success_cards_with_fallback_lines: usize,
    parse_success_cards_fully_implemented: usize,
    implemented_mechanics: Vec<MechanicTally>,
    unimplemented_marker_mechanics: Vec<MechanicTally>,
    fallback_reasons: Vec<MechanicTally>,
    slice: Option<SliceReport>,
}

#[derive(Debug, Clone, Serialize)]
struct SliceReport {
    mechanics: Vec<String>,
    fallback_reasons: Vec<String>,
    placeholder_count: usize,
    unsupported_reason_count: usize,
    affected_content_count: usize,
}

fn parse_args() -> Result<Args, String> {
    let mut cards_path = "cards.json".to_string();
    let mut limit = None;
    let mut allow_unsupported = false;
    let mut json_out = None;
    let mut slice_mechanics = Vec::<String>::new();
    let mut slice_fallback_reasons = Vec::<String>::new();
    let mut fail_on_slice_hits = false;

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
            "--allow-unsupported" => {
                allow_unsupported = true;
            }
            "--json-out" => {
                json_out = Some(
                    iter.next()
                        .ok_or_else(|| "--json-out requires a path".to_string())?,
                );
            }
            "--slice-mechanic" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--slice-mechanic requires a value".to_string())?;
                let normalized = canonical_mechanic_name(&raw);
                if !normalized.is_empty() {
                    slice_mechanics.push(normalized);
                }
            }
            "--slice-fallback-reason" => {
                let raw = iter
                    .next()
                    .ok_or_else(|| "--slice-fallback-reason requires a value".to_string())?;
                let normalized = normalize_slice_reason_arg(&raw);
                if !normalized.is_empty() {
                    slice_fallback_reasons.push(normalized);
                }
            }
            "--fail-on-slice-hits" => {
                fail_on_slice_hits = true;
            }
            _ => {
                return Err(format!(
                    "unknown argument '{arg}'. supported: --cards <path> --limit <n> --allow-unsupported --json-out <path> --slice-mechanic <name> --slice-fallback-reason <reason> --fail-on-slice-hits"
                ));
            }
        }
    }

    slice_mechanics.sort();
    slice_mechanics.dedup();
    slice_fallback_reasons.sort();
    slice_fallback_reasons.dedup();

    Ok(Args {
        cards_path,
        limit,
        allow_unsupported,
        json_out,
        slice_mechanics,
        slice_fallback_reasons,
        fail_on_slice_hits,
    })
}

fn pick_field<'a>(card: &'a Value, face: Option<&'a Value>, field: &str) -> Option<&'a str> {
    face.and_then(|value| value.get(field))
        .and_then(Value::as_str)
        .or_else(|| card.get(field).and_then(Value::as_str))
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
        .get("layout")
        .and_then(Value::as_str)
        .is_some_and(|layout| {
            matches!(
                layout,
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
    let mut oracle_text = oracle_text?.trim().to_string();
    if oracle_text.is_empty() {
        return None;
    }
    if card.get("layout").and_then(Value::as_str) == Some("split") {
        let stripped = oracle_text
            .lines()
            .filter(|line| {
                let normalized = line.trim().to_ascii_lowercase();
                !(normalized == "fuse"
                    || normalized
                        .strip_prefix("fuse ")
                        .is_some_and(|rest| rest.starts_with('(')))
            })
            .collect::<Vec<_>>()
            .join("\n");
        if !stripped.trim().is_empty() {
            oracle_text = stripped;
        }
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
    lines.push(oracle_text);

    Some(CardInput {
        name,
        parse_input: lines.join("\n"),
    })
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

fn static_keyword_name(id: StaticAbilityId) -> Option<&'static str> {
    use StaticAbilityId::*;
    Some(match id {
        Flying => "flying",
        FirstStrike => "first strike",
        DoubleStrike => "double strike",
        Deathtouch => "deathtouch",
        Defender => "defender",
        Flash => "flash",
        Haste => "haste",
        Hexproof => "hexproof",
        HexproofFrom => "hexproof from",
        Indestructible => "indestructible",
        Intimidate => "intimidate",
        Lifelink => "lifelink",
        Menace => "menace",
        Protection => "protection",
        Reach => "reach",
        Shroud => "shroud",
        Trample => "trample",
        Vigilance => "vigilance",
        Ward => "ward",
        Fear => "fear",
        Flanking => "flanking",
        Landwalk => "landwalk",
        Bloodthirst => "bloodthirst",
        Morph => "morph",
        Megamorph => "megamorph",
        Shadow => "shadow",
        Horsemanship => "horsemanship",
        Phasing => "phasing",
        Wither => "wither",
        Infect => "infect",
        Changeling => "changeling",
        _ => return None,
    })
}

fn normalize_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn canonical_mechanic_name(raw: &str) -> String {
    let lower = normalize_spaces(raw.trim().trim_matches('.').trim()).to_ascii_lowercase();
    if lower.starts_with("ward ") {
        return "ward".to_string();
    }
    if lower.starts_with("toxic ") {
        return "toxic".to_string();
    }
    if lower.starts_with("bushido ") {
        return "bushido".to_string();
    }
    if lower.starts_with("level up ") {
        return "level up".to_string();
    }
    if lower.starts_with("protection from") {
        return "protection".to_string();
    }
    if let Some(first) = lower.split_whitespace().next()
        && first.ends_with("cycling")
    {
        return first.to_string();
    }
    lower
}

fn implemented_named_mechanic_from_text(text: &str) -> Option<String> {
    let normalized = canonical_mechanic_name(text);
    if normalized.contains(':') || normalized.contains(',') || normalized.is_empty() {
        return None;
    }
    if matches!(
        normalized.as_str(),
        "prowess" | "exalted" | "undying" | "persist" | "storm" | "toxic" | "bushido"
    ) {
        return Some(normalized);
    }
    if normalized == "level up" {
        return Some(normalized);
    }
    if normalized.ends_with("cycling") {
        return Some(normalized);
    }
    None
}

fn parse_fallback_reason(display: &str) -> String {
    let lower = display.to_ascii_lowercase();
    let prefix = "unsupported parser line fallback:";
    let Some(rest) = lower.strip_prefix(prefix) else {
        return "unknown".to_string();
    };
    let Some(start) = rest.rfind('(') else {
        return "unknown".to_string();
    };
    let reason = rest[start + 1..]
        .trim()
        .trim_end_matches(')')
        .trim_matches('"')
        .trim();
    if reason.is_empty() {
        "unknown".to_string()
    } else {
        normalize_fallback_reason(reason)
    }
}

fn normalize_fallback_reason(raw: &str) -> String {
    normalize_spaces(raw.trim().trim_matches('"').trim_matches('\'')).to_ascii_lowercase()
}

fn normalize_slice_reason_arg(raw: &str) -> String {
    let lower = raw.trim().to_ascii_lowercase();
    if lower.starts_with("unsupported parser line fallback:") {
        parse_fallback_reason(raw)
    } else {
        normalize_fallback_reason(raw)
    }
}

fn matches_slice_filter(value: &str, filter: &str) -> bool {
    if value == filter {
        return true;
    }
    let Some(rest) = value.strip_prefix(filter) else {
        return false;
    };
    rest.is_empty()
        || rest.starts_with(' ')
        || rest.starts_with('{')
        || rest.starts_with('(')
        || rest.starts_with(':')
        || rest.starts_with('-')
        || rest.starts_with('—')
}

fn matches_any_slice_filter(value: &str, filters: &HashSet<String>) -> bool {
    filters
        .iter()
        .any(|filter| matches_slice_filter(value, filter))
}

fn add_tally(map: &mut HashMap<String, (usize, HashSet<String>)>, mechanic: String, card: &str) {
    let entry = map
        .entry(mechanic)
        .or_insert_with(|| (0usize, HashSet::<String>::new()));
    entry.0 += 1;
    entry.1.insert(card.to_string());
}

fn finalize_tallies(map: HashMap<String, (usize, HashSet<String>)>) -> Vec<MechanicTally> {
    let mut rows = map
        .into_iter()
        .map(|(mechanic, (ability_instances, cards))| {
            let mut example_cards = cards.iter().cloned().collect::<Vec<_>>();
            example_cards.sort();
            example_cards.truncate(5);
            MechanicTally {
                mechanic,
                ability_instances,
                card_count: cards.len(),
                example_cards,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.card_count
            .cmp(&a.card_count)
            .then_with(|| b.ability_instances.cmp(&a.ability_instances))
            .then_with(|| a.mechanic.cmp(&b.mechanic))
    });
    rows
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args().map_err(std::io::Error::other)?;
    let file_contents = fs::read_to_string(&args.cards_path)?;
    let cards_json: Value = serde_json::from_str(&file_contents)?;
    let cards = cards_json
        .as_array()
        .ok_or_else(|| std::io::Error::other("cards json must be an array"))?;

    let original_allow_unsupported = env::var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED").ok();
    if args.allow_unsupported {
        set_allow_unsupported(true);
    }

    let mut cards_processed = 0usize;
    let mut parse_successes = 0usize;
    let mut parse_failures = 0usize;
    let mut cards_with_unimplemented = HashSet::<String>::new();
    let mut cards_with_fallback = HashSet::<String>::new();
    let mut implemented: HashMap<String, (usize, HashSet<String>)> = HashMap::new();
    let mut unimplemented: HashMap<String, (usize, HashSet<String>)> = HashMap::new();
    let mut fallback_reasons: HashMap<String, (usize, HashSet<String>)> = HashMap::new();
    let slice_mechanics: HashSet<String> = args.slice_mechanics.iter().cloned().collect();
    let slice_fallback_reasons: HashSet<String> =
        args.slice_fallback_reasons.iter().cloned().collect();
    let slice_enabled = !slice_mechanics.is_empty() || !slice_fallback_reasons.is_empty();
    let mut slice_placeholder_count = 0usize;
    let mut slice_unsupported_reason_count = 0usize;
    let mut slice_affected_cards = HashSet::<String>::new();

    for card in cards {
        if let Some(limit) = args.limit
            && cards_processed >= limit
        {
            break;
        }
        let Some(card_input) = build_parse_input(card) else {
            continue;
        };
        cards_processed += 1;

        let parse_result = CardDefinitionBuilder::new(CardId::new(), &card_input.name)
            .parse_text(card_input.parse_input);
        let Ok(def) = parse_result else {
            parse_failures += 1;
            continue;
        };
        parse_successes += 1;

        for ability in &def.abilities {
            if let Some(text) = ability.text.as_deref()
                && let Some(named) = implemented_named_mechanic_from_text(text)
            {
                add_tally(&mut implemented, named, &card_input.name);
            }

            if let AbilityKind::Static(static_ability) = &ability.kind {
                let id = static_ability.id();
                if id == StaticAbilityId::UnsupportedParserLine {
                    let display = static_ability.display();
                    let reason = parse_fallback_reason(&display);
                    cards_with_fallback.insert(card_input.name.clone());
                    add_tally(&mut fallback_reasons, reason.clone(), &card_input.name);
                    if matches_any_slice_filter(&reason, &slice_fallback_reasons) {
                        slice_unsupported_reason_count += 1;
                        slice_affected_cards.insert(card_input.name.clone());
                    }
                } else if matches!(
                    id,
                    StaticAbilityId::KeywordMarker
                        | StaticAbilityId::RuleTextPlaceholder
                        | StaticAbilityId::KeywordFallbackText
                        | StaticAbilityId::RuleFallbackText
                ) {
                    let display = static_ability.display();
                    let mechanic = canonical_mechanic_name(&display);
                    cards_with_unimplemented.insert(card_input.name.clone());
                    add_tally(&mut unimplemented, mechanic.clone(), &card_input.name);
                    if matches_any_slice_filter(&mechanic, &slice_mechanics) {
                        slice_placeholder_count += 1;
                        slice_affected_cards.insert(card_input.name.clone());
                    }
                } else if let Some(name) = static_keyword_name(id) {
                    add_tally(&mut implemented, name.to_string(), &card_input.name);
                }
            }
        }
    }

    match original_allow_unsupported {
        Some(value) => unsafe {
            env::set_var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED", value);
        },
        None => set_allow_unsupported(false),
    }

    let implemented_rows = finalize_tallies(implemented);
    let unimplemented_rows = finalize_tallies(unimplemented);
    let fallback_rows = finalize_tallies(fallback_reasons);

    let mut cards_with_any_non_implemented = cards_with_unimplemented.clone();
    cards_with_any_non_implemented.extend(cards_with_fallback.iter().cloned());
    let fully_implemented_cards =
        parse_successes.saturating_sub(cards_with_any_non_implemented.len());

    println!("Parsed mechanics audit complete");
    println!("- Cards processed: {cards_processed}");
    println!("- Parse successes: {parse_successes}");
    println!("- Parse failures: {parse_failures}");
    println!(
        "- Parse-success cards with unimplemented markers: {}",
        cards_with_unimplemented.len()
    );
    println!(
        "- Parse-success cards with fallback lines: {}",
        cards_with_fallback.len()
    );
    println!(
        "- Parse-success cards fully implemented (no markers/fallback): {}",
        fully_implemented_cards
    );
    println!(
        "- Unique implemented mechanics seen: {}",
        implemented_rows.len()
    );
    println!(
        "- Unique unimplemented marker mechanics seen: {}",
        unimplemented_rows.len()
    );
    if slice_enabled {
        println!("- Slice placeholder count: {slice_placeholder_count}");
        println!("- Slice unsupported-reason count: {slice_unsupported_reason_count}");
        println!(
            "- Slice affected content count: {}",
            slice_affected_cards.len()
        );
    }
    println!();

    if !unimplemented_rows.is_empty() {
        println!("Top unimplemented marker mechanics:");
        for row in unimplemented_rows.iter().take(40) {
            println!(
                "  - {} cards ({} instances): {}",
                row.card_count, row.ability_instances, row.mechanic
            );
        }
        println!();
    }

    if !fallback_rows.is_empty() {
        println!("Top fallback reasons in parse-success cards:");
        for row in fallback_rows.iter().take(20) {
            println!(
                "  - {} cards ({} instances): {}",
                row.card_count, row.ability_instances, row.mechanic
            );
        }
        println!();
    }

    let slice_report = if slice_enabled {
        Some(SliceReport {
            mechanics: args.slice_mechanics.clone(),
            fallback_reasons: args.slice_fallback_reasons.clone(),
            placeholder_count: slice_placeholder_count,
            unsupported_reason_count: slice_unsupported_reason_count,
            affected_content_count: slice_affected_cards.len(),
        })
    } else {
        None
    };

    if let Some(path) = args.json_out {
        let report = JsonReport {
            cards_processed,
            parse_successes,
            parse_failures,
            parse_success_cards_with_unimplemented_markers: cards_with_unimplemented.len(),
            parse_success_cards_with_fallback_lines: cards_with_fallback.len(),
            parse_success_cards_fully_implemented: fully_implemented_cards,
            implemented_mechanics: implemented_rows,
            unimplemented_marker_mechanics: unimplemented_rows,
            fallback_reasons: fallback_rows,
            slice: slice_report,
        };
        let payload = serde_json::to_string_pretty(&report)?;
        fs::write(&path, payload)?;
        println!("Wrote JSON report to {path}");
    }

    if args.fail_on_slice_hits && slice_enabled {
        let total_hits = slice_placeholder_count + slice_unsupported_reason_count;
        if total_hits > 0 {
            return Err(std::io::Error::other(format!(
                "slice gate failed: {total_hits} hits ({slice_placeholder_count} placeholder + {slice_unsupported_reason_count} unsupported reason)"
            ))
            .into());
        }
    }

    Ok(())
}
