use std::fmt::Write;

use crate::cards::CardDefinition;

#[allow(dead_code)]
fn render_definition(def: &CardDefinition) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "name: {}", def.name());

    let additional_costs = def.additional_non_mana_costs();
    if additional_costs.is_empty() {
        let _ = writeln!(out, "additional_costs: <none>");
    } else {
        let _ = writeln!(out, "additional_costs:");
        for (idx, cost) in additional_costs.iter().enumerate() {
            let _ = writeln!(out, "[{}] {:?}", idx, cost);
        }
    }

    let spell_effects = def.spell_effect.as_deref().unwrap_or(&[]);
    if spell_effects.is_empty() {
        let _ = writeln!(out, "spell_effects: <none>");
    } else {
        let _ = writeln!(out, "spell_effects:");
        for (idx, effect) in spell_effects.iter().enumerate() {
            let _ = writeln!(out, "[{}] {:?}", idx, effect);
        }
    }

    if def.abilities.is_empty() {
        let _ = writeln!(out, "abilities: <none>");
    } else {
        let _ = writeln!(out, "abilities:");
        for (idx, ability) in def.abilities.iter().enumerate() {
            let _ = writeln!(out, "[{}] {:?}", idx, ability);
        }
    }

    out
}
