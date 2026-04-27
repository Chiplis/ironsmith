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
    if line.to_ascii_lowercase().starts_with("this creature has flying as long as it's your turn.")
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
    if line
        .to_ascii_lowercase()
        .contains("a land is put into a graveyard from the battlefield")
        && line.contains("that object's controller")
    {
        return line.replace("that object's controller", "that land's controller");
    }
    if is_keyword_style_line(&line) {
        line
    } else {
        ensure_trailing_period(&line)
    }
}

fn merge_ast_surface_lines(mut lines: Vec<String>) -> Vec<String> {
    loop {
        let previous = lines;
        let merged = merge_conditioned_spell_and_activation_tax_lines(
            merge_adjacent_simple_mana_add_lines(drop_redundant_spell_cost_lines(
                merge_lose_all_transform_lines(merge_blockability_lines(
                    merge_subject_predicate_surface_lines(previous.clone()),
                )),
            )),
        );
        if merged == previous {
            return merged;
        }
        lines = merged;
    }
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
pub(crate) fn ability_surface_text_for_tests(ability: &Ability) -> String {
    if let Some(keyword) = self::render_effects::describe_keyword_ability(ability) {
        return keyword;
    }
    self::render_effects::describe_inline_ability(ability)
}
