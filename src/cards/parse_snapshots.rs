use std::fmt::Write;

use crate::cards::{
    CardDefinition, buried_alive, cataclysm, cataclysmic_gearhulk, culling_the_weak, village_rites,
};

fn render_definition(def: &CardDefinition) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "name: {}", def.name());

    if def.cost_effects.is_empty() {
        let _ = writeln!(out, "cost_effects: <none>");
    } else {
        let _ = writeln!(out, "cost_effects:");
        for (idx, effect) in def.cost_effects.iter().enumerate() {
            let _ = writeln!(out, "[{}] {:?}", idx, effect);
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
