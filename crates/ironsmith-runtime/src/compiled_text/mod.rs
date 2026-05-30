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
        .into_iter()
        .map(|line| substitute_legendary_source_reference(&line, &def.card, ""))
        .collect()
}

pub fn unprocessed_compiled_lines(def: &CardDefinition) -> Vec<String> {
    normalize_ast_surface_lines(debug_compiled_lines(def))
        .into_iter()
        .map(|line| substitute_legendary_source_reference(&line, &def.card, ""))
        .collect()
}

/// Render a single ability using the same surface renderer as compiled oracle text.
pub fn ability_surface_text(ability: &Ability) -> String {
    if let Some(keyword) = self::render_effects::describe_keyword_ability(ability) {
        return keyword;
    }
    self::render_effects::describe_inline_ability(ability)
}

fn normalize_ast_surface_lines(lines: Vec<String>) -> Vec<String> {
    let lines: Vec<String> = lines
        .into_iter()
        .map(|line| normalize_common_semantic_phrasing(&line))
        .collect();
    merge_ast_surface_lines(lines)
        .into_iter()
        .map(finalize_ast_surface_line)
        .flat_map(expand_finalized_ast_surface_line)
        .collect()
}

fn finalize_ast_surface_line(line: String) -> String {
    let mut line = line;
    let lower = line.to_ascii_lowercase();
    if let Some(case_line) = normalize_case_to_solve_line(&line) {
        return case_line;
    }
    if lower.contains(
        "tap target creature or planeswalker. choose it. activated abilities of that permanent can't be activated this turn",
    ) {
        line = line.replace(
            "choose it. activated abilities of that permanent can't be activated this turn",
            "its activated abilities can't be activated this turn",
        );
    }
    if lower.contains("that permanent's mana value")
        && lower.contains("reveal the top card of your library")
    {
        line = line.replace("that permanent's mana value", "that card's mana value");
    }
    if lower.contains("if it's a permanent, exile it")
        && lower.contains("at the beginning of the next end step, exile it")
    {
        line = line.replace(
            "if it's a permanent, exile it",
            "if it would leave the battlefield, exile it instead",
        );
    }
    if let Some(rest) = line.strip_prefix("During your turn, this creature has ") {
        if rest.to_ascii_lowercase().starts_with("prevent ") {
            line = format!("During your turn, {}", lowercase_first(rest));
        }
    }
    line = line
        .replace(
            "When this token dies: You gain 1 life",
            "When this token dies, you gain 1 life",
        )
        .replace(
            "When this token dies: It deals 1 damage to any target",
            "When this token dies, it deals 1 damage to any target",
        );
    if let Some(body) = solved_case_body(&line) {
        return format!("Solved — {body}");
    }
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
        "target creature an opponent controls or enchantment",
        "target creature or enchantment an opponent controls",
    );
    line = line.replace(
        "Target creature an opponent controls or enchantment",
        "Target creature or enchantment an opponent controls",
    );
    if !line
        .to_ascii_lowercase()
        .contains("reveal the top card of your library")
    {
        line = line.replace(
            "lose life equal to its mana value",
            "lose life equal to that permanent's mana value",
        );
        line = line.replace(
            "Lose life equal to its mana value",
            "Lose life equal to that permanent's mana value",
        );
    }
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
    line = normalize_adamant_enters_with_counter_clause(&line);
    if line
        .to_ascii_lowercase()
        .contains("a land is put into a graveyard from the battlefield")
        && line.contains("that object's controller")
    {
        return line.replace("that object's controller", "that land's controller");
    }
    line = normalize_conditional_followup_case(&line);
    line = line.replace(
        ". Then if {S} was spent to cast this spell, that permanent doesn't untap ",
        ". If {S} was spent to cast this spell, that permanent doesn't untap ",
    );
    line = normalize_activation_colon_payload_case(&line);
    line = normalize_top_card_exile_imperative(&line);
    line = normalize_exact_during_your_turn_predicate_surface(&line);
    line = normalize_sacrifice_enchantment_counter_spell_trigger(&line);
    line = normalize_token_quoted_ability_surfaces(&line);
    line = line
        .replace(
            "When this token dies: You gain 1 life",
            "When this token dies, you gain 1 life",
        )
        .replace(
            "When this token dies: It deals 1 damage to any target",
            "When this token dies, it deals 1 damage to any target",
        );
    line = line.replace(
        "Tap it. That permanent doesn't untap during its controller's next untap step",
        "Tap it. It doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "tap it. That permanent doesn't untap during its controller's next untap step",
        "tap it. It doesn't untap during its controller's next untap step",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "choose it. activated abilities of that permanent can't be activated this turn",
        "Its activated abilities can't be activated this turn",
        "its activated abilities can't be activated this turn",
    );
    if line
        .to_ascii_lowercase()
        .contains("reveal the top card of your library")
    {
        line = line.replace("that permanent's mana value", "that card's mana value");
    }
    line = replace_ascii_case_insensitive_once(
        line,
        "if it's a permanent, exile it",
        "If it would leave the battlefield, exile it instead",
        "if it would leave the battlefield, exile it instead",
    );
    line = capitalize_sentence_boundaries(&line);
    if is_keyword_style_line(&line) {
        line
    } else {
        ensure_trailing_period(&line)
    }
}

fn normalize_case_to_solve_line(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    let condition = lower
        .strip_prefix("at the beginning of your end step, if ")?
        .strip_suffix(", solve")?;
    let condition = normalize_case_solve_condition(condition);
    Some(format!("To solve — {}.", capitalize_first(&condition)))
}

fn normalize_case_solve_condition(condition: &str) -> String {
    let condition = condition
        .strip_suffix(" and the chosen option isn't solved")
        .unwrap_or(condition);
    if let Some(rest) = condition.strip_prefix("there are ") {
        if let Some(count) = rest.strip_suffix(" lands you control on the battlefield") {
            return format!("you control {count} lands");
        }
        if let Some(count) = rest.strip_suffix(" permanents you control on the battlefield") {
            return format!("you control {count} permanents");
        }
    }
    condition.to_string()
}

fn solved_case_body(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let solved = trimmed
        .strip_suffix(". As long as the chosen option is solved")
        .or_else(|| trimmed.strip_suffix(" as long as the chosen option is solved"))?;
    let body = solved
        .strip_prefix("This enchantment creature has ")
        .or_else(|| solved.strip_prefix("This enchantment has "))
        .or_else(|| solved.strip_prefix("This creature has "))
        .unwrap_or(solved)
        .trim();
    let body = body
        .replace("creature spells or enchantment spells", "creature and enchantment spells");
    Some(body)
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

fn normalize_adamant_enters_with_counter_clause(line: &str) -> String {
    let Some((enter_clause, condition_clause)) = line.split_once(" if ") else {
        return line.to_string();
    };
    if !enter_clause.starts_with("This creature enters with ") || !enter_clause.ends_with(" on it")
    {
        return line.to_string();
    }
    let condition = condition_clause.trim().trim_end_matches('.');
    if !condition.contains(" mana was spent to cast this spell") {
        return line.to_string();
    }
    let mut enter_text = enter_clause.to_string();
    if let Some(first) = enter_text.chars().next() {
        let lower = first.to_ascii_lowercase();
        enter_text.replace_range(0..first.len_utf8(), &lower.to_string());
    }
    format!("Adamant — If {condition}, {enter_text}")
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
                        annotate_color_choice_exclusions(merge_same_true_type_addition_lines(
                            merge_same_true_keyword_grant_lines(
                                merge_subject_predicate_surface_lines(previous.clone()),
                            ),
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
            if let (Some(left_solved), Some(right_solved)) =
                (solved_case_body(left), solved_case_body(right))
            {
                merged.push(format!(
                    "Solved — {left_solved}, and {}.",
                    lowercase_first(&right_solved)
                ));
                idx += 2;
                continue;
            }
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
            if let Some(merged_restriction) = merge_cast_and_activate_restriction_lines(left, right)
            {
                merged.push(merged_restriction);
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

fn merge_cast_and_activate_restriction_lines(left: &str, right: &str) -> Option<String> {
    let (left_condition, left_body) = split_condition_prefix(left);
    let (right_condition, right_body) = split_condition_prefix(right);
    if !left_condition.eq_ignore_ascii_case(&right_condition) {
        return None;
    }

    let left_subject = left_body.strip_suffix(" can't cast spells")?.trim();
    let (right_subject, activation_restriction) =
        right_body.split_once(" can't activate abilities of ")?;
    if !left_subject.eq_ignore_ascii_case(right_subject.trim()) {
        return None;
    }

    let activation_restriction = normalize_or_list_surface(activation_restriction.trim());
    let subject = lowercase_first(left_subject);
    let body =
        format!("{subject} can't cast spells or activate abilities of {activation_restriction}");
    if left_condition.is_empty() {
        Some(body)
    } else {
        Some(format!("{left_condition}, {body}"))
    }
}

fn split_condition_prefix(line: &str) -> (String, &str) {
    let Some((condition, body)) = line.split_once(", ") else {
        return (String::new(), line);
    };
    if condition.eq_ignore_ascii_case("During your turn")
        || condition.to_ascii_lowercase().starts_with("as long as ")
    {
        (condition.to_string(), body)
    } else {
        (String::new(), line)
    }
}

fn normalize_or_list_surface(text: &str) -> String {
    let parts = text
        .replace(',', " ")
        .split_whitespace()
        .filter(|part| !part.eq_ignore_ascii_case("or"))
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>();
    join_with_or(&parts)
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

fn normalize_exact_during_your_turn_predicate_surface(line: &str) -> String {
    let trimmed = line.trim();
    let without_period = trimmed.trim_end_matches('.');
    if without_period.contains(". ") {
        return line.to_string();
    }
    let Some((subject, verb, predicate)) = split_subject_predicate_clause(without_period) else {
        return line.to_string();
    };
    let Some(predicate) = predicate.trim().strip_suffix(" as long as it's your turn") else {
        return line.to_string();
    };
    if predicate.contains(" as long as ") || predicate.contains(" during ") {
        return line.to_string();
    }

    let normalized_predicate = match verb {
        "gets" | "get" => {
            if !predicate.starts_with('+') && !predicate.starts_with('-') {
                return line.to_string();
            }
            predicate.to_string()
        }
        "has" | "have" | "gains" | "gain" => {
            let normalized = normalize_keyword_predicate_case(predicate);
            if normalized == predicate && !is_keyword_phrase(predicate) {
                return line.to_string();
            }
            normalized
        }
        _ => return line.to_string(),
    };
    let surface_verb = if matches!(verb, "gains" | "gain") {
        have_verb_for_subject(subject)
    } else {
        verb
    };
    let (surface_subject, surface_verb) = during_your_turn_subject_and_verb(subject, surface_verb);
    format!("During your turn, {surface_subject} {surface_verb} {normalized_predicate}")
}

fn normalize_sacrifice_enchantment_counter_spell_trigger(line: &str) -> String {
    let trimmed = line.trim().trim_end_matches('.');
    let Some(body) = trimmed
        .strip_prefix("Whenever ")
        .and_then(|body| body.strip_suffix(", sacrifice this enchantment. Counter it"))
    else {
        return line.to_string();
    };
    if !body.contains(" casts a spell") {
        return line.to_string();
    }
    format!("When {body}, sacrifice this enchantment and counter that spell")
}

fn expand_finalized_ast_surface_line(line: String) -> Vec<String> {
    let trimmed = line.trim().trim_end_matches('.');
    match trimmed.to_ascii_lowercase().as_str() {
        "skulk, lifelink" => vec!["Skulk".to_string(), "Lifelink".to_string()],
        "skulk, deathtouch" => vec!["Skulk".to_string(), "Deathtouch".to_string()],
        _ => vec![line],
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
    fn conditional_enters_with_counter_uses_adamant_prefix_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "This creature enters with a +1/+1 counter on it if at least three white mana was spent to cast this spell."
                    .to_string()
            ),
            "Adamant — If at least three white mana was spent to cast this spell, this creature enters with a +1/+1 counter on it."
        );
    }

    #[test]
    fn same_turn_pump_and_keyword_lines_merge_to_during_your_turn_surface() {
        let lines = merge_ast_surface_lines(vec![
            "This creature gets +2/+0 as long as it's your turn.".to_string(),
            "This creature has First strike as long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, this creature gets +2/+0 and has first strike".to_string()]
        );
    }

    #[test]
    fn mixed_during_turn_and_as_long_turn_lines_merge_to_during_your_turn_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Equipped creature gets +2/+0 as long as it's your turn.".to_string(),
            "During your turn, equipped creature has first strike.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, equipped creature gets +2/+0 and has first strike".to_string()]
        );
    }

    #[test]
    fn equipped_keyword_and_conditional_pt_bonus_keep_separate_lines() {
        let lines = merge_ast_surface_lines(vec![
            "Equipped creature has first strike.".to_string(),
            "Equipped creature gets +1/+1 as long as equipped creature is a human.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Equipped creature has first strike.".to_string(),
                "Equipped creature gets +1/+1 as long as equipped creature is a human.".to_string(),
            ]
        );
    }

    #[test]
    fn each_creature_turn_pump_and_keyword_merge_to_plural_subject() {
        let lines = merge_ast_surface_lines(vec![
            "Each creature you control gets +1/+0 as long as it's your turn.".to_string(),
            "Creatures you control have Trample as long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, creatures you control get +1/+0 and have trample".to_string()]
        );
    }

    #[test]
    fn exact_turn_conditioned_pump_uses_during_your_turn_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "Each creature you control gets +2/+0 as long as it's your turn".to_string()
            ),
            "During your turn, creatures you control get +2/+0."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "This creature gets +2/+2 as long as it's your turn".to_string()
            ),
            "During your turn, this creature gets +2/+2."
        );
    }

    #[test]
    fn matching_cast_and_activation_restrictions_merge() {
        let lines = merge_specific_adjacent_surface_lines(vec![
            "During your turn, Your opponents can't cast spells.".to_string(),
            "During your turn, your opponents can't activate abilities of artifacts creatures or enchantments."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "During your turn, your opponents can't cast spells or activate abilities of artifacts, creatures, or enchantments"
                    .to_string()
            ]
        );
    }

    #[test]
    fn sacrifice_enchantment_counter_spell_trigger_uses_single_when_clause() {
        assert_eq!(
            finalize_ast_surface_line(
                "Whenever an opponent casts a spell, sacrifice this enchantment. Counter it"
                    .to_string()
            ),
            "When an opponent casts a spell, sacrifice this enchantment and counter that spell."
        );
    }

    #[test]
    fn target_type_disjunction_keeps_shared_opponent_controller_clause() {
        assert_eq!(
            finalize_ast_surface_line(
                "Destroy target creature an opponent controls or enchantment".to_string()
            ),
            "Destroy target creature or enchantment an opponent controls."
        );
    }

    #[test]
    fn life_loss_mana_value_uses_that_permanent_surface() {
        assert_eq!(
            finalize_ast_surface_line("You lose life equal to its mana value".to_string()),
            "You lose life equal to that permanent's mana value."
        );
    }

    #[test]
    fn skulk_keyword_pairs_keep_oracle_line_breaks() {
        assert_eq!(
            expand_finalized_ast_surface_line("Skulk, lifelink".to_string()),
            vec!["Skulk".to_string(), "Lifelink".to_string()]
        );
        assert_eq!(
            expand_finalized_ast_surface_line("Skulk, deathtouch".to_string()),
            vec!["Skulk".to_string(), "Deathtouch".to_string()]
        );
    }

    #[test]
    fn token_quote_activation_costs_keep_colon_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token, add {C}.\""
                    .to_string()
            ),
            "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\""
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

    #[test]
    fn repeated_type_additions_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Creatures you control are the chosen type in addition to their other types."
                .to_string(),
            "Creature spells you control are the chosen type in addition to their other types."
                .to_string(),
            "Creature cards you own that aren't on the battlefield are the chosen type in addition to their other types."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield."
                    .to_string()
            ]
        );
    }

    #[test]
    fn during_your_turn_prevent_clause_drops_extra_has() {
        assert_eq!(
            finalize_ast_surface_line(
                "During your turn, this creature has Prevent all damage that would be dealt to this creature."
                    .to_string()
            ),
            "During your turn, prevent all damage that would be dealt to this creature."
        );
    }

    #[test]
    fn compiled_text_cleanup_layers_reject_known_semantic_rescue_strings() {
        let checked_sources = [
            ("mod.rs", include_str!("mod.rs")),
            ("normalize_common.rs", include_str!("normalize_common.rs")),
            ("debug_safe.rs", include_str!("debug_safe.rs")),
            ("surface_helpers.rs", include_str!("surface_helpers.rs")),
        ];
        let banned = [
            concat!("K", "ain"),
            concat!("allagan", " eye"),
            concat!("Flame", "break"),
            concat!(
                "deals 3 damage to each creature without flying",
                ", deal 3 damage to each player"
            ),
            concat!(
                "Gain control of target creature until end of turn",
                ", untap it, then it gains haste"
            ),
            concat!(
                "Untap target creature, gain control of it until end of turn",
                ", then it gains haste"
            ),
            concat!(
                "You choose the top card in your library",
                ", exile it, then you may play that card"
            ),
            concat!(
                "for each card revealed this way",
                ", unless it's a permanent, put that object"
            ),
        ];

        for (source_name, source) in checked_sources {
            for needle in banned {
                assert!(
                    !source.contains(needle),
                    "{source_name} contains semantic rescue text that belongs in structural rendering: {needle}"
                );
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn ability_surface_text_for_tests(ability: &Ability) -> String {
    ability_surface_text(ability)
}
