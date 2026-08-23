#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "+2: Create a 1/1 white Kor Soldier creature token. You may attach an Equipment you control to it.\n−2: You may put an Equipment card from your hand or graveyard onto the battlefield.\n−10: Create a colorless Equipment artifact token named Stoneforged Blade. It has indestructible, \"Equipped creature gets +5/+5 and has double strike,\" and equip {0}.\nNahiri, the Lithomancer can be your commander.";

fn find_nested_create(effect: &Effect) -> Option<CreateTokenEffect> {
    if let Some(create) = effect.downcast_ref::<CreateTokenEffect>() {
        return Some(create.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_nested_create(child);
        }
    });
    found
}

fn stoneforged_blade_creation(definition: &CardDefinition) -> CreateTokenEffect {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_loyalty_ability() => {
                Some(activated.effects.flattened_default_effects())
            }
            _ => None,
        })
        .flatten()
        .filter_map(find_nested_create)
        .find(|create| create.token.card.name == "Stoneforged Blade")
        .expect("Nahiri's ultimate should create Stoneforged Blade")
}

fn test_creature() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Blade Host")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn nahiri_keeps_the_complete_stoneforged_blade_rules_and_surface() {
    let definition = parse_oracle_card_definition("Nahiri, the Lithomancer");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let create = stoneforged_blade_creation(&definition);
    assert_eq!(
        create.ability_presentation,
        Some(ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined)
    );
    let abilities = &create.token.abilities;
    assert!(abilities.iter().any(|ability| matches!(
        &ability.kind,
        AbilityKind::Static(ability) if ability.id() == StaticAbilityId::Indestructible
    )));
    assert!(abilities.iter().any(|ability| matches!(
        &ability.kind,
        AbilityKind::Static(ability)
            if ability.id() == StaticAbilityId::Anthem
                && ability.anthem_payload().is_some_and(|anthem| {
                    anthem.power == crate::static_abilities::AnthemValue::Fixed(5)
                        && anthem.toughness == crate::static_abilities::AnthemValue::Fixed(5)
                })
    )));
    assert!(abilities.iter().any(|ability| matches!(
        &ability.kind,
        AbilityKind::Static(ability)
            if ability.id() == StaticAbilityId::GrantObjectAbilityForFilter
                && ability.display().to_ascii_lowercase().contains("double strike")
    )));
    assert!(abilities.iter().any(|ability| {
        matches!(&ability.kind, AbilityKind::Activated(_))
            && crate::ability::ability_surface_text_for_tests(ability)
                .is_some_and(|text| text.starts_with("Equip {0}"))
    }));
}

#[test]
fn stoneforged_blade_is_indestructible_and_grants_both_attached_bonuses() {
    let definition = parse_oracle_card_definition("Nahiri, the Lithomancer");
    let create = stoneforged_blade_creation(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let nahiri = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut context = crate::effects::ExecutionContext::new_default(nahiri, alice);
    let outcome = create
        .execute(&mut game, &mut context)
        .expect("Stoneforged Blade creation should resolve");
    let blade = outcome
        .affected_objects()
        .and_then(|objects| objects.first().copied())
        .expect("the creation effect should report Stoneforged Blade");
    let host = game.create_object_from_definition(&test_creature(), alice, Zone::Battlefield);

    assert_eq!(
        game.object(blade).map(|object| object.name.as_str()),
        Some("Stoneforged Blade")
    );
    assert!(game.object_has_static_ability_id(blade, StaticAbilityId::Indestructible));
    assert!(game.attach_object_to_target(blade, crate::object::AttachmentTarget::Object(host),));
    assert_eq!(game.calculated_power(host), Some(7));
    assert_eq!(game.calculated_toughness(host), Some(7));
    assert!(game.object_has_static_ability_id(host, StaticAbilityId::DoubleStrike));

    assert!(game.detach_object_from_current_target(blade));
    assert_eq!(game.calculated_power(host), Some(2));
    assert_eq!(game.calculated_toughness(host), Some(2));
    assert!(!game.object_has_static_ability_id(host, StaticAbilityId::DoubleStrike));
}
