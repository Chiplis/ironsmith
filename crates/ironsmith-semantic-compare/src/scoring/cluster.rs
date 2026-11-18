//! Cluster reports group a card's compiled lines before running the ordinary scorer.
use super::{
    compare_semantics_scored, merge_adjacent_compiled_lines, merge_self_block_restrictions,
    merge_simple_mana_add_compiled_lines, merge_transform_compiled_lines, strip_compiled_prefix,
};
use crate::{EmbeddingConfig, normalize_named_self_references};

fn is_destroy_all_clause(line: &str) -> Option<&'static str> {
    let normalized = strip_cost_prefix(line)
        .trim()
        .trim_end_matches('.')
        .trim()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "destroy all artifacts" => Some("artifacts"),
        "destroy all creatures" => Some("creatures"),
        "destroy all enchantments" => Some("enchantments"),
        _ => None,
    }
}

fn strip_cost_prefix(line: &str) -> &str {
    let trimmed = line.trim();
    if let Some((prefix, tail)) = trimmed.split_once(':') {
        let prefix = prefix.trim();
        let looks_like_cost = prefix.starts_with('{')
            || prefix.eq_ignore_ascii_case("t")
            || matches!(
                prefix.to_ascii_lowercase().as_str(),
                "t" | "tap"
                    | "tap this source"
                    | "tap this creature"
                    | "tap this land"
                    | "tap this permanent"
            )
            || prefix.chars().all(|ch| {
                ch.is_ascii_whitespace()
                    || ch == ','
                    || ch == '{'
                    || ch == '}'
                    || ch == '('
                    || ch == ')'
                    || ch.is_ascii_digit()
                    || ch == '/'
                    || ch == '|'
                    || matches!(
                        ch,
                        'W' | 'U' | 'B' | 'R' | 'G' | 'w' | 'u' | 'b' | 'r' | 'g' | 'x' | 'X'
                    )
            });
        if looks_like_cost {
            return strip_cost_prefix(tail.trim());
        }
    }
    trimmed
}

fn is_damage_to_any_target_clause(line: &str) -> bool {
    let normalized = strip_cost_prefix(line).to_ascii_lowercase();
    let lower = normalized.as_str();
    (lower.starts_with("this creature deals") || lower.starts_with("deals"))
        && lower.contains("damage")
        && lower.contains(" to any target")
}

fn normalize_damage_to_self_clause(line: &str) -> Option<String> {
    let trimmed = strip_cost_prefix(line).trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.contains("damage") {
        return None;
    }
    if !(lower.contains("to this creature") || lower.contains("to itself")) {
        return None;
    }
    if lower.starts_with("deal ") {
        let tail = trimmed
            .trim_start_matches("Deal")
            .trim()
            .replace("this creature ", "")
            .replace("to this creature", "to itself");
        Some(format!("deals {tail}"))
    } else if lower.starts_with("this creature deals ") {
        Some(
            trimmed
                .replace("this creature ", "")
                .replace("to this creature", "to itself"),
        )
    } else if lower.starts_with("deals ") {
        Some(trimmed.replace("to this creature", "to itself"))
    } else {
        None
    }
}

fn merge_damage_to_self_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    for line in lines {
        let parts = line
            .split('.')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.len() < 2 {
            merged.push(line.to_string());
            continue;
        }

        let mut idx = 0usize;
        let mut rebuilt = Vec::new();
        while idx < parts.len() {
            if idx + 1 < parts.len()
                && is_damage_to_any_target_clause(parts[idx])
                && let Some(trailing) = normalize_damage_to_self_clause(parts[idx + 1])
            {
                rebuilt.push(format!("{} and {}", parts[idx], trailing));
                idx += 2;
                continue;
            }
            rebuilt.push(parts[idx].to_string());
            idx += 1;
        }
        merged.push(rebuilt.join(". "));
    }
    merged
}

fn merge_destroy_all_compiled_lines(lines: &[String]) -> Vec<String> {
    let mut split_lines = Vec::with_capacity(lines.len() * 2);
    for line in lines {
        let mut has_destroy_all = false;
        for part in line.split('.') {
            if is_destroy_all_clause(part).is_some() {
                has_destroy_all = true;
                break;
            }
        }
        if !has_destroy_all {
            split_lines.push(line.to_string());
            continue;
        }

        for part in line
            .split('.')
            .map(|part| part.trim())
            .filter(|part| !part.is_empty())
        {
            split_lines.push(part.to_string());
        }
    }
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    const DESTROY_ALL_TYPES: [&str; 3] = ["artifacts", "creatures", "enchantments"];

    while idx < split_lines.len() {
        let Some(first) = is_destroy_all_clause(&split_lines[idx]) else {
            merged.push(split_lines[idx].clone());
            idx += 1;
            continue;
        };
        let mut types = vec![first];
        idx += 1;

        while idx < split_lines.len() {
            let Some(next) = is_destroy_all_clause(&split_lines[idx]) else {
                break;
            };
            types.push(next);
            idx += 1;
        }

        if types.len() < 2 {
            merged.push(format!("Destroy all {first}."));
            continue;
        }

        if types.len() == 2 && types[0] == types[1] {
            merged.push(format!("Destroy all {}.", types[0]));
            continue;
        }

        types.dedup();
        let mut ordered = Vec::new();
        for typ in DESTROY_ALL_TYPES {
            if types.contains(&typ) {
                ordered.push(typ);
            }
        }
        let joined = match ordered.as_slice() {
            [single] => format!("Destroy all {single}."),
            [first, second] => format!("Destroy all {first} and {second}."),
            [first, second, third] => format!("Destroy all {first}, {second}, and {third}."),
            _ => format!("Destroy all {}.", ordered.join(", ")),
        };
        merged.push(joined);
    }

    merged
}

/// Scores a card with the line grouping used by the oracle cluster audit.
pub fn compare_cluster_card_semantics_scored(
    card_name: &str,
    oracle_text: &str,
    unprocessed_compiled_lines: &[String],
    embedding: Option<EmbeddingConfig>,
) -> (f32, f32, f32, isize, bool) {
    let normalized_oracle = normalize_named_self_references(oracle_text, card_name, None);
    let stripped_compiled_lines = unprocessed_compiled_lines
        .iter()
        .map(|line| {
            strip_compiled_prefix(&normalize_named_self_references(line, card_name, None))
                .to_owned()
        })
        .collect::<Vec<_>>();
    let merged_mana_lines =
        merge_simple_mana_add_compiled_lines(&stripped_compiled_lines, |_| true);
    let merged_blockability_lines =
        merge_adjacent_compiled_lines(&merged_mana_lines, merge_self_block_restrictions);
    let merged_damage_lines = merge_damage_to_self_compiled_lines(&merged_blockability_lines);
    let merged_destroy_all_lines = merge_destroy_all_compiled_lines(&merged_damage_lines);
    let normalized_compiled_lines = merge_transform_compiled_lines(&merged_destroy_all_lines);
    compare_semantics_scored(&normalized_oracle, &normalized_compiled_lines, embedding)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_simple_mana_add_compiled_lines_merges_three_modes() {
        let merged = merge_simple_mana_add_compiled_lines(
            &[
                "{T}: Add {W}.".to_string(),
                "{T}: Add {B}.".to_string(),
                "{T}: Add {G}.".to_string(),
            ],
            |_| true,
        );
        assert_eq!(merged, vec!["{T}: Add {W} or {B} or {G}".to_string()]);
    }
}
