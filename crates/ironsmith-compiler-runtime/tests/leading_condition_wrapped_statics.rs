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
