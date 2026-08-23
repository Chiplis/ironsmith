//! Small gameplay-only presentation fallback.
//!
//! Audit-grade canonical rendering lives in `ironsmith-text`. The engine keeps
//! only these non-recursive labels so browser gameplay does not link the full
//! renderer. Compiled artifacts and browser snapshots should carry authored
//! presentation strings whenever exact wording matters.

use crate::ability::Ability;
use crate::cards::CardDefinition;
use crate::effect::{Condition, Effect, Value};
use crate::filter::ObjectFilter;
use crate::target::PlayerFilter;

pub fn compile_effect_list(effects: &[Effect]) -> String {
    effects
        .iter()
        .map(|effect| format!("{effect:?}"))
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn ability_surface_text(ability: &Ability) -> String {
    format!("{ability:?}")
}

#[cfg(test)]
pub fn ability_surface_text_for_tests(ability: &Ability) -> String {
    ability_surface_text(ability)
}

pub fn compiled_text_lines(definition: &CardDefinition) -> Vec<String> {
    definition
        .abilities
        .iter()
        .map(ability_surface_text)
        .chain(
            definition
                .spell_effect
                .iter()
                .map(|program| format!("{program:?}")),
        )
        .collect()
}

pub fn debug_compiled_lines(definition: &CardDefinition) -> Vec<String> {
    compiled_text_lines(definition)
}

pub fn unprocessed_compiled_lines(definition: &CardDefinition) -> Vec<String> {
    compiled_text_lines(definition)
}

pub fn canonical_compiled_lines(definition: &CardDefinition) -> Vec<String> {
    compiled_text_lines(definition)
}

pub fn describe_effect(effect: &Effect) -> String {
    format!("{effect:?}")
}

pub fn describe_value(value: &Value) -> String {
    format!("{value:?}")
}

pub fn describe_condition(condition: &Condition) -> String {
    format!("{condition:?}")
}

pub fn pluralize_noun_phrase_for_trigger(phrase: &str) -> String {
    if phrase.ends_with('s') {
        phrase.to_string()
    } else {
        format!("{phrase}s")
    }
}

pub fn describe_party_size_for_each_basis(_value: &Value) -> Option<(i32, String)> {
    None
}

pub fn describe_counter_for_each_basis(_value: &Value) -> Option<(i32, String)> {
    None
}

pub fn describe_for_each_multiplier_and_basis(_value: &Value) -> Option<(i32, String)> {
    None
}

pub fn describe_turn_history_for_each_basis(_value: &Value) -> Option<String> {
    None
}

pub fn describe_aggregate_filter_value_subject(filter: &ObjectFilter) -> String {
    filter.description()
}

pub fn describe_death_history_subject(
    subject: &str,
    controller: Option<&PlayerFilter>,
    _controller_surface: ironsmith_core::DeathHistoryControllerSurface,
) -> String {
    controller.map_or_else(
        || format!("{subject} that died this turn"),
        |controller| format!("{subject} ({controller:?}) that died this turn"),
    )
}
