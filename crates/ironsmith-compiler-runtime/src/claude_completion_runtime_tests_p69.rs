//! Claude package 69 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

#[test]
fn spectral_adversary_phases_out_a_serial_comma_target_list() {
    let definition = compile_to_runtime_definition("Spectral Adversary",
        "Mana cost: {1}{U}\nType: Creature — Spirit\nPower/Toughness: 2/1\nFlash\nFlying\nWhen this creature enters, you may pay {1}{U} any number of times. When you pay this cost one or more times, put that many +1/+1 counters on this creature, then up to that many other target artifacts, creatures, and/or enchantments phase out.", false)
        .expect("Spectral Adversary should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("PhaseOut"), "the serial-comma target list is one phase subject: {debug}");
    assert!(debug.contains("Artifact") && debug.contains("Enchantment"), "{debug}");
    assert!(debug.contains("other: true"), "{debug}");
}

#[test]
fn fantastic_four_modal_trigger_keeps_the_characteristic_list() {
    let definition = compile_to_runtime_definition("The Fantastic Four",
        "Mana cost: {R}{G}{W}{U}\nType: Legendary Creature — Human Hero\nPower/Toughness: 4/4\nWhen The Fantastic Four enter and whenever you cast a spell with power, toughness, or mana value 4, choose one that hasn't been chosen this turn.\n• Create a 0/4 colorless Wall creature token with defender.\n• The Fantastic Four deal 3 damage to each opponent.\n• Put two +1/+1 counters on target creature.\n• Draw a card.", false)
        .expect("The Fantastic Four should compile");
    let debug = format!("{:?}", definition.abilities);
    // The serial commas stay inside the trigger's spell filter.
    assert!(debug.contains("power: Some(Equal(4))"), "{debug}");
    assert!(debug.contains("toughness: Some(Equal(4))"), "{debug}");
    assert!(debug.contains("mana_value: Some(Equal(4))"), "{debug}");
    assert!(debug.contains("disallow_previously_chosen_modes_this_turn: true"), "{debug}");
}
