use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::effect::{Until, Value};
use ironsmith_compiler::effects::ModifyPowerToughnessEffect;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::target::ChooseSpec;
use ironsmith_compiler::triggers::TriggerKind;
use ironsmith_compiler::types::CardType;

#[test]
fn frenzy_instances_lower_to_independent_unblocked_attack_triggers() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Double Frenzy Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Frenzy 1\nFrenzy 3")
        .expect("Frenzy should compile");

    assert_eq!(definition.abilities.len(), 2, "{:#?}", definition.abilities);
    for (ability, amount) in definition.abilities.iter().zip([1, 3]) {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            panic!("Frenzy must be executable: {ability:#?}");
        };
        assert_eq!(
            triggered.trigger.kind,
            TriggerKind::ThisAttacksAndIsntBlocked
        );
        let [effect] = triggered.effects.flattened_default_effects() else {
            panic!("Frenzy should have one pump effect: {triggered:#?}");
        };
        let pump = effect
            .downcast_ref::<ModifyPowerToughnessEffect>()
            .expect("Frenzy should lower to a power/toughness change");
        assert_eq!(pump.power, Value::Fixed(amount));
        assert_eq!(pump.toughness, Value::Fixed(0));
        assert!(matches!(pump.target, ChooseSpec::Source));
        assert!(matches!(pump.duration, Until::EndOfTurn));
    }
}

#[test]
fn frenzy_sliver_grants_the_executable_trigger() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Frenzy Sliver")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "All Sliver creatures have frenzy 1. (Whenever a Sliver attacks and isn't blocked, it gets +1/+0 until end of turn.)",
        )
        .expect("Frenzy Sliver should compile without a card-name override");
    let debug = format!("{:#?}", definition.abilities);

    assert!(debug.contains("AddAbility"), "{debug}");
    assert!(debug.contains("ThisAttacksAndIsntBlocked"), "{debug}");
    assert!(debug.contains("ModifyPowerToughnessEffect"), "{debug}");
}
