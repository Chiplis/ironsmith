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

#[test]
fn snapshot_village_rites() {
    let snapshot = render_definition(&village_rites());
    let expected = include_str!("parse_snapshots/village_rites.txt");
    assert_eq!(snapshot, expected);
}

#[test]
fn snapshot_culling_the_weak() {
    let snapshot = render_definition(&culling_the_weak());
    let expected = include_str!("parse_snapshots/culling_the_weak.txt");
    assert_eq!(snapshot, expected);
}

#[test]
fn snapshot_cataclysm() {
    let snapshot = render_definition(&cataclysm());
    let expected = include_str!("parse_snapshots/cataclysm.txt");
    assert_eq!(snapshot, expected);
}

#[test]
fn snapshot_cataclysmic_gearhulk() {
    let snapshot = render_definition(&cataclysmic_gearhulk());
    let expected = include_str!("parse_snapshots/cataclysmic_gearhulk.txt");
    assert_eq!(snapshot, expected);
}

#[test]
fn snapshot_buried_alive() {
    let snapshot = render_definition(&buried_alive());
    let expected = include_str!("parse_snapshots/buried_alive.txt");
    assert_eq!(snapshot, expected);
}
