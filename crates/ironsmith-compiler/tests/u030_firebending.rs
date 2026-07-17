use ironsmith_compiler::ability::{AbilityKind, PresentationKeyword, PresentationLabel};
use ironsmith_compiler::cards::{CardDefinition, CardDefinitionBuilder};
use ironsmith_compiler::effect::Value;
use ironsmith_compiler::effects::mana::AddScaledManaEffect;
use ironsmith_compiler::effects::{
    EmitKeywordActionEffect, ManaRetainedEffect, ManaRetentionDuration,
};
use ironsmith_compiler::events::KeywordActionKind;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::mana::ManaSymbol;
use ironsmith_compiler::target::PlayerFilter;
use ironsmith_compiler::triggers::TriggerKind;
use ironsmith_compiler::types::CardType;

fn compile_creature_definition(name: &str, text: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"))
}

fn compile_creature(name: &str, text: &str) -> String {
    format!("{:#?}", compile_creature_definition(name, text).abilities)
}

#[test]
fn fixed_firebending_is_an_executable_attack_trigger() {
    let debug = compile_creature("Fixed Firebending Probe", "Firebending 2");
    assert!(debug.contains("ThisAttacks"), "{debug}");
    assert!(debug.contains("AddScaledManaEffect"), "{debug}");
    assert!(debug.contains("ManaRetainedEffect"), "{debug}");
    assert!(debug.contains("EndOfCombat"), "{debug}");
    assert!(debug.contains("Firebend"), "{debug}");
    assert!(
        !debug.contains("KeywordMarker(\n                    \"firebending"),
        "Firebending must not lower to a presentation-only marker: {debug}"
    );
}

#[test]
fn every_printed_firebending_instance_is_independent() {
    let definition =
        compile_creature_definition("Double Firebending Probe", "Firebending 1\nFirebending 2");
    assert_eq!(definition.abilities.len(), 2, "{:#?}", definition.abilities);

    for (ability, expected_amount) in definition.abilities.iter().zip([1, 2]) {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            panic!("each Firebending instance must be a triggered ability: {ability:#?}");
        };
        assert_eq!(triggered.trigger.kind, TriggerKind::ThisAttacks);
        assert_eq!(
            triggered.presentation_label,
            Some(PresentationLabel::Keyword(
                PresentationKeyword::Firebending(expected_amount.to_string(),)
            ))
        );

        let effects = triggered.effects.all_effects();
        let retained = effects
            .iter()
            .filter_map(|effect| effect.downcast_ref::<ManaRetainedEffect>())
            .collect::<Vec<_>>();
        assert_eq!(retained.len(), 1, "{triggered:#?}");
        assert_eq!(retained[0].duration, ManaRetentionDuration::EndOfCombat);
        assert_eq!(retained[0].effects.len(), 1, "{triggered:#?}");
        let produced_mana = retained[0].effects[0]
            .downcast_ref::<AddScaledManaEffect>()
            .unwrap_or_else(|| panic!("Firebending must produce scaled red mana: {triggered:#?}"));
        assert_eq!(produced_mana.mana, vec![ManaSymbol::Red]);
        assert_eq!(produced_mana.amount, Value::Fixed(expected_amount));
        assert_eq!(produced_mana.player, PlayerFilter::You);

        let emitted_actions = effects
            .iter()
            .filter_map(|effect| effect.downcast_ref::<EmitKeywordActionEffect>())
            .collect::<Vec<_>>();
        assert_eq!(emitted_actions.len(), 1, "{triggered:#?}");
        assert_eq!(emitted_actions[0].action, KeywordActionKind::Firebend);
        assert_eq!(emitted_actions[0].amount, 1);
    }
}

#[test]
fn dynamic_firebending_values_lower_without_card_name_special_cases() {
    for (name, text, expected_value) in [
        (
            "Named Source Power Probe",
            "Firebending X, where X is Named Source Power Probe's power.",
            "PowerOf",
        ),
        (
            "Self Power Probe",
            "Firebending X, where X is this creature's power.",
            "PowerOf",
        ),
        (
            "Creature Count Probe",
            "Firebending X, where X is the number of creatures you control.",
            "Count",
        ),
        (
            "Experience Counter Probe",
            "Firebending X, where X is the number of experience counters you have.",
            "PlayerCounters",
        ),
    ] {
        let debug = compile_creature(name, text);
        assert!(debug.contains("ThisAttacks"), "{name}: {debug}");
        assert!(debug.contains("ManaRetainedEffect"), "{name}: {debug}");
        assert!(debug.contains(expected_value), "{name}: {debug}");
    }
}

#[test]
fn grants_carry_the_same_executable_firebending_trigger() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Firebending Grant Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature gains firebending 4 until end of turn.")
        .expect("a Firebending grant should compile");
    let debug = format!("{:#?}", definition.spell_effect);
    assert!(
        debug.contains("AddAbilityGeneric") || debug.contains("AddAbility("),
        "{debug}"
    );
    assert!(debug.contains("ThisAttacks"), "{debug}");
    assert!(debug.contains("ManaRetainedEffect"), "{debug}");
    assert!(debug.contains("Firebend"), "{debug}");
}

#[test]
fn firebend_observers_match_the_resolution_action_event() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Firebend Observer Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever you firebend, draw a card.")
        .expect("the CR 702.189b observer should compile");
    assert_eq!(definition.abilities.len(), 1, "{:#?}", definition.abilities);
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!(
            "the Firebend observer must be a triggered ability: {:#?}",
            definition.abilities
        );
    };
    assert_eq!(
        triggered.trigger.kind,
        TriggerKind::KeywordAction {
            action: KeywordActionKind::Firebend,
            player: PlayerFilter::You,
        }
    );
}
