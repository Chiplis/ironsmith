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
    let lines = debug_compiled_lines(def)
        .into_iter()
        .map(|line| normalize_granted_activated_ability_clause(&line).unwrap_or(line))
        .collect();
    merge_subject_predicate_surface_lines(lines)
}

pub fn unprocessed_compiled_lines(def: &CardDefinition) -> Vec<String> {
    let lines = debug_compiled_lines(def)
        .into_iter()
        .map(|line| normalize_granted_activated_ability_clause(&line).unwrap_or(line))
        .collect();
    merge_subject_predicate_surface_lines(lines)
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
