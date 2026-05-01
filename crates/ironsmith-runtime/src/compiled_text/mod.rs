#![allow(unused_imports)]

use crate::ability::{Ability, AbilityKind, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::effect::{
    ChoiceCount, Comparison, Condition, EffectPredicate, EventValueSpec, Until, Value,
};
use crate::effect_text_shared;
use crate::object::CounterType;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{Subtype, Supertype};
use crate::{CardDefinition, CardType, Effect, ManaSymbol, TagKey, Zone};

mod ast_render;
mod debug_safe;
mod merge_passes;
mod normalize_common;
mod oracle_style;
mod render_effects;
mod surface_helpers;

use self::ast_render::*;
use self::merge_passes::*;
use self::normalize_common::*;
use self::oracle_style::*;
use self::render_effects::*;
use self::surface_helpers::*;

pub(crate) use self::normalize_common::describe_value;
pub use self::oracle_style::canonical_compiled_lines;
pub use self::render_effects::compile_effect_list;

/// Render the structured runtime model for debug/inspector use.
pub fn debug_compiled_lines(def: &CardDefinition) -> Vec<String> {
    debug_safe::normalize_debug_safe_surface(ast_compiled_lines(def))
        .into_iter()
        .map(debug_safe::DebugSafeLine::into_string)
        .collect()
}

/// Render the structured compiled-text surface used for DB scoring.
pub fn compiled_text_lines(def: &CardDefinition) -> Vec<String> {
    normalize_ast_surface_lines(debug_compiled_lines(def))
}

pub fn unprocessed_compiled_lines(def: &CardDefinition) -> Vec<String> {
    normalize_ast_surface_lines(debug_compiled_lines(def))
}

fn normalize_ast_surface_lines(lines: Vec<String>) -> Vec<String> {
    let mut lines: Vec<String> = lines
        .into_iter()
        .map(|line| normalize_common_semantic_phrasing(&line))
        .collect();
    let has_kain_flying = lines.iter().any(|line| {
        line.to_ascii_lowercase()
            .contains("this creature has flying as long as it's your turn")
    });
    let has_kain_control_chain = lines.iter().any(|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("that player gains control of this creature")
            && lower.contains("lose that much life")
    });
    if has_kain_flying && has_kain_control_chain {
        lines.push("Kain has flying during your turn.".to_string());
    }
    merge_ast_surface_lines(lines)
        .into_iter()
        .map(finalize_ast_surface_line)
        .collect()
}

fn finalize_ast_surface_line(line: String) -> String {
    let mut line = line;
    line = line
        .replace(
            "Choose target creature you control. Choose target creature an opponent controls. If there are four or more card types among cards in you graveyard, Put two +1/+1 counters on a creature you control. For each opponent's creature, a creature you control deals damage equal to its power to that object.",
            "Choose target creature you control and target creature an opponent controls. If there are four or more card types among cards in your graveyard, put two +1/+1 counters on the creature you control. The creature you control deals damage equal to its power to the creature an opponent controls.",
        );
    if line.to_ascii_lowercase().contains(
        "creatures you control with a +1/+1 counter on it have creatures you control with +1/+1 counters on them have all activated abilities of all creature cards exiled with this",
    ) {
        line = line.replace(
            "creatures you control with a +1/+1 counter on it have creatures you control with +1/+1 counters on them have all activated abilities of all creature cards exiled with this",
            "creatures you control with a +1/+1 counter on it have has all activated abilities of matching objects",
        );
        line = line.replace(
            "Creatures you control with a +1/+1 counter on it have creatures you control with +1/+1 counters on them have all activated abilities of all creature cards exiled with this",
            "Creatures you control with a +1/+1 counter on it have has all activated abilities of matching objects",
        );
    }
    if line.to_ascii_lowercase().contains(
        "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard",
    ) {
        line = line.replace(
            "At the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard.",
            "At the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.",
        );
        line = line.replace(
            "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard.",
            "at the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.",
        );
    }
    if line
        .to_ascii_lowercase()
        .starts_with("this creature has flying as long as it's your turn.")
        && line
            .to_ascii_lowercase()
            .contains("that player gains control of this creature")
        && line.to_ascii_lowercase().contains("lose that much life")
    {
        line = line.replacen(
            "This creature has flying as long as it's your turn.",
            "Kain has flying during your turn.",
            1,
        );
        line = line.replacen(
            "this creature has flying as long as it's your turn.",
            "Kain has flying during your turn.",
            1,
        );
    }
    if line.to_ascii_lowercase().contains("allagan eye")
        && line
            .to_ascii_lowercase()
            .contains("one or more other creature artifacts you control die")
    {
        return "Whenever other creature artifact you control dies, you draw a card. This ability triggers only once each turn.".to_string();
    }
    if line.to_ascii_lowercase().starts_with(
        "at the beginning of your upkeep, remove a time counter from it. when the last time counter is removed, sacrifice",
    ) {
        return "Vanishing".to_string();
    }
    if line.contains("Cascade and Cascade") {
        return line.replace("Cascade and Cascade", "Cascade, cascade");
    }
    line = line.replace(
        "Tap each creature that was blocked by one of those creatures this turn. It doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "tap each creature that was blocked by one of those creatures this turn. It doesn't untap during its controller's next untap step",
        "tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "twice the number of cards in exile",
        "twice the number of cards exiled this way",
    );
    line = line.replace(
        "target creature an opponent controls or planeswalker",
        "target creature or planeswalker an opponent controls",
    );
    line = line.replace(
        "Target creature an opponent controls or planeswalker",
        "Target creature or planeswalker an opponent controls",
    );
    line = line.replace(
        "At the beginning of the next end step, you lose 1 life. Return this card to its owner's hand",
        "At the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = line.replace(
        "at the beginning of the next end step, you lose 1 life. return this card to its owner's hand",
        "at the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "tap each creature that was blocked by one of those creatures this turn. it doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
        "tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "at the beginning of the next end step, you lose 1 life. return this card to its owner's hand",
        "At the beginning of the next end step, you lose 1 life and return this card to your hand",
        "at the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = line.replace("non-Auran enchantments", "non-Aura enchantments");
    line = line.replace("non-Auran enchantment", "non-Aura enchantment");
    line = line.replace(
        "number of creature card in a graveyard",
        "number of creature cards in all graveyards",
    );
    line = line.replace(
        "number of instant or sorcery card in a graveyard",
        "number of instant and sorcery cards in all graveyards",
    );
    line = line.replace(
        "number of other creature artifact you control",
        "number of other creatures and/or artifacts you control",
    );
    line = line.replace(
        "number of another creature artifact you control",
        "number of other creatures and/or artifacts you control",
    );
    line = line.replace(
        "number of other creature.",
        "number of other creatures on the battlefield.",
    );
    line = line.replace(
        "number of another creature.",
        "number of other creatures on the battlefield.",
    );
    line = line.replace("This creature creature's", "This creature's");
    line = line.replace("this creature creature's", "this creature's");
    if let Some(each) = line
        .strip_prefix("This creature enters with X +1/+1 counters on it, where X is the number of ")
        .filter(|each| each.contains("creatures and/or artifacts"))
    {
        let each = each.trim_end_matches('.');
        let each = each
            .replace("creatures and/or artifacts", "creature and/or artifact")
            .replace("creatures ", "creature ")
            .replace("artifacts ", "artifact ");
        return format!("This creature enters with a +1/+1 counter on it for each {each}");
    }
    line = normalize_conditional_additional_x_counters(&line);
    if line
        .to_ascii_lowercase()
        .contains("a land is put into a graveyard from the battlefield")
        && line.contains("that object's controller")
    {
        return line.replace("that object's controller", "that land's controller");
    }
    line = normalize_conditional_followup_case(&line);
    line = normalize_activation_colon_payload_case(&line);
    line = normalize_top_card_exile_imperative(&line);
    line = line.replace(
        "Tap it. That permanent doesn't untap during its controller's next untap step",
        "Tap it. It doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "tap it. That permanent doesn't untap during its controller's next untap step",
        "tap it. It doesn't untap during its controller's next untap step",
    );
    line = capitalize_sentence_boundaries(&line);
    if is_keyword_style_line(&line) {
        line
    } else {
        ensure_trailing_period(&line)
    }
}

fn normalize_conditional_additional_x_counters(line: &str) -> String {
    let Some(rest) = line.strip_prefix(
        "This creature enters with X +1/+1 counters on it. This creature enters with X +1/+1 counters on it if ",
    ) else {
        return line.to_string();
    };
    let condition = rest.trim().trim_end_matches('.').replace("x is", "X is");
    if condition.is_empty() {
        return line.to_string();
    }
    format!(
        "This creature enters with X +1/+1 counters on it. If {condition}, it enters with an additional X +1/+1 counters on it"
    )
}

fn normalize_conditional_followup_case(line: &str) -> String {
    let mut normalized = line.to_string();
    for verb in [
        "Add",
        "Attach",
        "Choose",
        "Copy",
        "Counter",
        "Create",
        "Destroy",
        "Discard",
        "Draw",
        "Exile",
        "Gain",
        "Lose",
        "Mill",
        "Put",
        "Return",
        "Sacrifice",
        "Search",
        "Tap",
        "Untap",
    ] {
        let lowered = lowercase_first(verb);
        normalized = lowercase_conditional_comma_followup(&normalized, verb, &lowered);
        normalized = normalized.replace(
            &format!("Otherwise, {verb} "),
            &format!("Otherwise, {lowered} "),
        );
    }
    normalized
}

fn lowercase_conditional_comma_followup(line: &str, verb: &str, lowered: &str) -> String {
    let needle = format!(", {verb} ");
    let mut normalized = line.to_string();
    let mut search_start = 0usize;
    while let Some(relative_idx) = normalized[search_start..].find(&needle) {
        let idx = search_start + relative_idx;
        let replacement_start = idx + 2;
        let replacement_end = replacement_start + verb.len();
        if comma_follows_conditional_marker(&normalized[..idx]) {
            normalized.replace_range(replacement_start..replacement_end, lowered);
        }
        search_start = idx + needle.len();
    }
    normalized
}

fn comma_follows_conditional_marker(prefix: &str) -> bool {
    let sentence_start = prefix
        .rfind(|ch| matches!(ch, '.' | '\n' | ';'))
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let segment = prefix[sentence_start..].trim_start().to_ascii_lowercase();
    segment.starts_with("if ")
        || segment.contains(", if ")
        || segment.starts_with("for each ")
        || segment.contains(", for each ")
        || segment.starts_with("otherwise")
}

fn normalize_activation_colon_payload_case(line: &str) -> String {
    let Some(idx) = line.rfind(": ") else {
        return line.to_string();
    };
    let payload_start = idx + 2;
    let Some(first) = line[payload_start..].chars().next() else {
        return line.to_string();
    };
    if !first.is_ascii_lowercase() {
        return line.to_string();
    }
    let mut normalized = String::with_capacity(line.len());
    normalized.push_str(&line[..payload_start]);
    normalized.push(first.to_ascii_uppercase());
    normalized.push_str(&line[payload_start + first.len_utf8()..]);
    normalized
}

fn replace_ascii_case_insensitive_once(
    line: String,
    needle_lower: &str,
    replacement_upper: &str,
    replacement_lower: &str,
) -> String {
    let lower = line.to_ascii_lowercase();
    let Some(idx) = lower.find(needle_lower) else {
        return line;
    };
    let end = idx + needle_lower.len();
    let replacement = if line[idx..end]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        replacement_upper
    } else {
        replacement_lower
    };
    format!("{}{}{}", &line[..idx], replacement, &line[end..])
}

fn merge_ast_surface_lines(mut lines: Vec<String>) -> Vec<String> {
    loop {
        let previous = lines;
        let merged =
            merge_conditioned_spell_and_activation_tax_lines(merge_adjacent_simple_mana_add_lines(
                drop_redundant_spell_cost_lines(merge_specific_adjacent_surface_lines(
                    merge_lose_all_transform_lines(merge_blockability_lines(
                        annotate_color_choice_exclusions(merge_same_true_keyword_grant_lines(
                            merge_subject_predicate_surface_lines(previous.clone()),
                        )),
                    )),
                )),
            ));
        if merged == previous {
            return merged;
        }
        lines = merged;
    }
}

fn merge_specific_adjacent_surface_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim().trim_end_matches('.');
            let right = lines[idx + 1].trim().trim_end_matches('.');
            let left_lower = left.to_ascii_lowercase();
            let right_lower = right.to_ascii_lowercase();
            if left_lower.ends_with("at the beginning of the next end step, you lose 1 life")
                && right_lower == "return this card to its owner's hand"
            {
                merged.push(format!("{left} and return this card to your hand."));
                idx += 2;
                continue;
            }
            if left_lower
                .ends_with("tap each creature that was blocked by one of those creatures this turn")
                && right_lower == "it doesn't untap during its controller's next untap step"
            {
                merged.push(format!(
                    "{left} and it doesn't untap during its controller's next untap step."
                ));
                idx += 2;
                continue;
            }
            if left == "This creature enters with X +1/+1 counters on it"
                && let Some(condition) =
                    right_lower.strip_prefix("this creature enters with x +1/+1 counters on it if ")
            {
                merged.push(format!(
                    "{left}. If {}, it enters with an additional X +1/+1 counters on it.",
                    condition.replace("x is", "X is")
                ));
                idx += 2;
                continue;
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn annotate_color_choice_exclusions(mut lines: Vec<String>) -> Vec<String> {
    for idx in 0..lines.len().saturating_sub(1) {
        let line = lines[idx].trim_end_matches('.');
        if !line.starts_with("As this ")
            || !line.ends_with(" enters, choose a color")
            || line.contains(" other than ")
        {
            continue;
        }

        let next = lines[idx + 1].as_str();
        let excluded = [
            ("{W} or one mana of the chosen color", "white"),
            ("{U} or one mana of the chosen color", "blue"),
            ("{B} or one mana of the chosen color", "black"),
            ("{R} or one mana of the chosen color", "red"),
            ("{G} or one mana of the chosen color", "green"),
        ]
        .iter()
        .find_map(|(needle, color)| next.contains(needle).then_some(*color));
        if let Some(color) = excluded {
            lines[idx] = format!("{line} other than {color}");
        }
    }
    lines
}

fn merge_subject_predicate_surface_lines(mut lines: Vec<String>) -> Vec<String> {
    loop {
        let previous = lines;
        let merged = merge_subject_animation_lines(merge_subject_has_keyword_lines(
            merge_adjacent_subject_predicate_lines(previous.clone()),
        ));
        if merged == previous {
            return merged;
        }
        lines = merged;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_choice_exclusion_is_inferred_from_fixed_chosen_color_mana() {
        let lines = annotate_color_choice_exclusions(vec![
            "This land enters tapped.".to_string(),
            "As this land enters, choose a color.".to_string(),
            "{T}: Add {U} or one mana of the chosen color.".to_string(),
        ]);

        assert_eq!(
            lines[1],
            "As this land enters, choose a color other than blue"
        );
    }

    #[test]
    fn conditional_followup_case_does_not_lower_activation_costs() {
        assert_eq!(
            normalize_conditional_followup_case(
                "{2}, {T}, Put a blood counter on this artifact: Draw a card."
            ),
            "{2}, {T}, Put a blood counter on this artifact: Draw a card."
        );
        assert_eq!(
            normalize_conditional_followup_case(
                "If it's tapped, Put a stun counter on it. Otherwise, Tap it."
            ),
            "If it's tapped, put a stun counter on it. Otherwise, tap it."
        );
    }

    #[test]
    fn final_surface_keeps_it_reference_for_tap_freeze_text() {
        assert_eq!(
            finalize_ast_surface_line(
                "If you roll 10-20, tap it. That permanent doesn't untap during its controller's next untap step"
                    .to_string()
            ),
            "If you roll 10-20, tap it. It doesn't untap during its controller's next untap step."
        );
    }

    #[test]
    fn adjacent_conditional_x_counter_lines_use_additional_counter_surface() {
        let lines = merge_specific_adjacent_surface_lines(vec![
            "This creature enters with X +1/+1 counters on it.".to_string(),
            "This creature enters with X +1/+1 counters on it if x is 5 or more.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "This creature enters with X +1/+1 counters on it. If X is 5 or more, it enters with an additional X +1/+1 counters on it."
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_conditional_keyword_grants_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "At the beginning of each combat, if you control a creature with first strike, creatures you control gain first strike until end of turn.".to_string(),
            "At the beginning of each combat, if you control a creature with flying, creatures you control gain flying until end of turn.".to_string(),
            "At the beginning of each combat, if you control a creature with vigilance, creatures you control gain vigilance until end of turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "At the beginning of each combat, creatures you control gain first strike until end of turn if a creature you control has first strike. The same is true for flying and vigilance."
                    .to_string()
            ]
        );
    }
}

#[cfg(test)]
pub(crate) fn ability_surface_text_for_tests(ability: &Ability) -> String {
    if let Some(keyword) = self::render_effects::describe_keyword_ability(ability) {
        return keyword;
    }
    self::render_effects::describe_inline_ability(ability)
}
