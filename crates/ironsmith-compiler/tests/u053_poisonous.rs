use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::ability::{AbilityKind, PresentationKeyword, PresentationLabel};
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::effect::Value;
use ironsmith_compiler::effects::PoisonCountersEffect;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::target::PlayerFilter;
use ironsmith_compiler::triggers::TriggerKind;
use ironsmith_compiler::types::CardType;

#[test]
fn poisonous_instances_lower_to_independent_combat_damage_triggers() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Double Poisonous Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Poisonous 1\nPoisonous 3")
        .expect("Poisonous should compile");

    assert_eq!(definition.abilities.len(), 2, "{:#?}", definition.abilities);
    for (ability, amount) in definition.abilities.iter().zip([1, 3]) {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            panic!("Poisonous must be executable: {ability:#?}");
        };
        assert!(matches!(
            triggered.trigger.kind,
            TriggerKind::ThisDealsCombatDamageToPlayer { .. }
        ));
        assert_eq!(
            triggered.presentation_label,
            Some(PresentationLabel::Keyword(PresentationKeyword::Poisonous(
                amount
            )))
        );
        let [effect] = triggered.effects.flattened_default_effects() else {
            panic!("Poisonous should have one effect: {triggered:#?}");
        };
        let poison = effect
            .downcast_ref::<PoisonCountersEffect>()
            .expect("Poisonous should give poison counters");
        assert_eq!(poison.count, Value::Fixed(amount as i32));
        assert_eq!(poison.player, PlayerFilter::DamagedPlayer);
    }
}

#[test]
fn printed_poisonous_grants_carry_the_executable_trigger() {
    for (name, text) in [
        (
            "Virulent Sliver",
            "All Sliver creatures have poisonous 1. (Whenever a Sliver deals combat damage to a player, that player gets a poison counter.)",
        ),
        (
            "Snake Cult Initiation",
            "Enchant creature\nEnchanted creature has poisonous 3. (Whenever it deals combat damage to a player, that player gets three poison counters.)",
        ),
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .parse_text(text)
            .unwrap_or_else(|error| panic!("{name} should compile: {error}"));
        let debug = format!("{:#?}", definition.abilities);
        assert!(
            debug.contains("AddAbility") || debug.contains("AttachedAbilityGrant"),
            "{name}: {debug}"
        );
        assert!(
            debug.contains("ThisDealsCombatDamageToPlayer"),
            "{name}: {debug}"
        );
        assert!(debug.contains("PoisonCountersEffect"), "{name}: {debug}");
    }
}
