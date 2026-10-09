//! "During your turn, prevent all damage that would be dealt to you."
//! (Personal Sanctuary) and "As long as you have a full party, prevent all
//! damage that would be dealt to equipped creature." (Multiclass Baldric):
//! a leading condition over an otherwise complete static ability makes that
//! ability conditional (CR 604.2). Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;

#[path = "p02_line_families/compile.rs"]
mod compile;

const PERSONAL_SANCTUARY: &str = "Mana cost: {2}{W}\nType: Enchantment\nDuring your turn, prevent all damage that would be dealt to you.";
const MULTICLASS_BALDRIC: &str = "Mana cost: {1}\nType: Artifact — Equipment\nEquipped creature has lifelink if you control a Cleric, deathtouch if you control a Rogue, haste if you control a Warrior, and flying if you control a Wizard.\nAs long as you have a full party, prevent all damage that would be dealt to equipped creature.\nEquip {2}";

fn static_debug(definition: &CardDefinition) -> Vec<String> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => Some(format!("{ability:?}")),
            _ => None,
        })
        .collect()
}

#[test]
fn personal_sanctuary_prevention_is_gated_on_your_turn() {
    for definition in compile::compile_both("Personal Sanctuary", PERSONAL_SANCTUARY) {
        let statics = static_debug(&definition);
        assert_eq!(statics.len(), 1, "{statics:?}");
        assert!(statics[0].contains("PreventAllDamage"), "{}", statics[0]);
        assert!(statics[0].contains("YourTurn"), "{}", statics[0]);
    }
}

#[test]
fn multiclass_baldric_prevention_is_gated_on_a_full_party() {
    for definition in compile::compile_both("Multiclass Baldric", MULTICLASS_BALDRIC) {
        let statics = static_debug(&definition);
        let gated = statics
            .iter()
            .find(|debug| debug.contains("PreventAllDamageToSelf"))
            .expect("equipped-creature prevention");
        assert!(gated.contains("FullParty"), "{gated}");
    }
}

const FLARING_FLAME_KIN: &str = "Mana cost: {2}{R}\nType: Creature — Elemental Warrior\nPower/Toughness: 2/2\nAs long as this creature is enchanted, it gets +2/+2, has trample, and has \"{R}: This creature gets +1/+0 until end of turn.\"";

#[test]
fn flaring_flame_kin_buffs_only_while_enchanted() {
    for definition in compile::compile_both("Flaring Flame-Kin", FLARING_FLAME_KIN) {
        let statics = static_debug(&definition);
        assert!(!statics.is_empty(), "{statics:?}");
        for debug in &statics {
            assert!(
                debug.contains("Enchanted") || debug.contains("enchanted"),
                "every piece is gated on being enchanted: {debug}"
            );
        }
        let all = statics.join("\n");
        assert!(all.contains("Trample"), "{all}");
        assert!(all.contains("ModifyPowerToughness") || all.contains("Anthem"), "{all}");
    }
}

#[test]
fn pronoun_remainder_is_not_rebound_when_the_condition_names_another_object() {
    // Security Bypass: "it" is the enchanted creature, not the Aura.
    let text = "Mana cost: {1}{U}\nType: Enchantment — Aura\nEnchant creature\nAs long as enchanted creature is attacking alone, it can't be blocked.";
    let compiled = ironsmith_compiler_runtime::compile_to_runtime_definition("Security Bypass", text, false);
    if let Ok(definition) = compiled {
        let all = static_debug(&definition).join("\n");
        assert!(!all.contains("source: true") || all.contains("attached"), "{all}");
    }
}
