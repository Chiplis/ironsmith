//! Small gameplay-only presentation fallback.
//!
//! Audit-grade canonical rendering lives in `ironsmith-text`. The engine keeps
//! only these non-recursive labels so browser gameplay does not link the full
//! renderer. Compiled artifacts and browser snapshots should carry authored
//! presentation strings whenever exact wording matters.

use crate::ability::{Ability, AbilityKind};
use crate::cards::CardDefinition;
use crate::effect::{Condition, Effect, Value};
use crate::filter::ObjectFilter;
use crate::mana::ManaCost;
use crate::target::PlayerFilter;

pub fn compile_effect_list(effects: &[Effect]) -> String {
    effects
        .iter()
        .map(|effect| format!("{effect:?}"))
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn ability_surface_text(ability: &Ability) -> String {
    if let Some(text) = fixed_mana_ability_surface_text(ability) {
        return text;
    }
    format!("{ability:?}")
}

fn fixed_mana_ability_surface_text(ability: &Ability) -> Option<String> {
    let AbilityKind::Activated(activated) = &ability.kind else {
        return None;
    };
    let mana = activated.mana_output.as_ref()?;
    if mana.is_empty()
        || !activated.effects.is_empty()
        || !activated.choices.is_empty()
        || activated.activation_condition.is_some()
        || !activated.activation_restrictions.is_empty()
        || !activated.additional_restrictions.is_empty()
        || !activated.mana_usage_restrictions.is_empty()
    {
        return None;
    }

    let cost = activated.mana_cost.display();
    let output = ManaCost::from_symbols(mana.clone()).to_oracle();
    Some(format!("{cost}: Add {output}."))
}

/// Merge exact compiled labels with abilities added by current characteristics.
///
/// Printed abilities retain their artifact wording when the same executable
/// ability is still present. Abilities granted by land types or continuous
/// effects use the lightweight runtime renderer instead of invalidating every
/// printed label merely because the list length changed.
pub fn current_ability_surface_texts(
    current_abilities: &[Ability],
    definition: Option<&CardDefinition>,
) -> Vec<String> {
    let Some(definition) = definition else {
        return current_abilities.iter().map(ability_surface_text).collect();
    };

    let definition_labels = if definition.ability_labels.len() == definition.abilities.len() {
        definition.ability_labels.clone()
    } else {
        let canonical = definition
            .canonical_text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if canonical.len() == definition.abilities.len() {
            canonical
        } else {
            Vec::new()
        }
    };
    let mut used_definition_abilities = vec![false; definition.abilities.len()];

    current_abilities
        .iter()
        .map(|current| {
            definition
                .abilities
                .iter()
                .enumerate()
                .find(|(index, printed)| {
                    !used_definition_abilities[*index]
                        && definition_labels.get(*index).is_some()
                        && *printed == current
                })
                .and_then(|(index, _)| {
                    used_definition_abilities[index] = true;
                    definition_labels.get(index).cloned()
                })
                .unwrap_or_else(|| ability_surface_text(current))
        })
        .collect()
}

/// Return the presentation text for one ability in a runtime text box.
///
/// Compiled card text is the authoritative lightweight presentation carried by
/// game objects. When its non-empty lines map one-to-one to the current
/// abilities, preserve that canonical wording instead of falling through to
/// the gameplay-only debug renderer. Dynamic ability changes deliberately use
/// the fallback unless their text box changed in lockstep.
pub fn indexed_ability_surface_text(
    abilities: &[Ability],
    compiled_card_text: &str,
    ability_index: usize,
) -> Option<String> {
    let ability = abilities.get(ability_index)?;
    let canonical = compiled_card_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if canonical.len() == abilities.len() {
        return canonical.get(ability_index).cloned();
    }

    Some(ability_surface_text(ability))
}

#[cfg(test)]
pub fn ability_surface_text_for_tests(ability: &Ability) -> String {
    ability_surface_text(ability)
}

pub fn compiled_text_lines(definition: &CardDefinition) -> Vec<String> {
    let canonical = definition
        .canonical_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !canonical.is_empty() {
        return canonical;
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_ability_text_prefers_canonical_one_to_one_lines() {
        let abilities = vec![crate::ability::flying(), crate::ability::trample()];

        assert_eq!(
            indexed_ability_surface_text(&abilities, "Flying\nTrample", 0).as_deref(),
            Some("Flying")
        );
        assert_eq!(
            indexed_ability_surface_text(&abilities, "Flying\nTrample", 1).as_deref(),
            Some("Trample")
        );
    }

    #[test]
    fn indexed_ability_text_does_not_misalign_changed_ability_lists() {
        let abilities = vec![crate::ability::flying(), crate::ability::trample()];
        let fallback = indexed_ability_surface_text(&abilities, "Flying", 1)
            .expect("the second ability should still have a fallback label");

        assert_ne!(fallback, "Flying");
        assert!(indexed_ability_surface_text(&abilities, "Flying", 2).is_none());
    }
}
