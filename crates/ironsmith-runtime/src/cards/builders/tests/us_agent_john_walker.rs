#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::ability::ActivationTiming;
use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};
use crate::object::{AttachmentTarget, ObjectKind};

#[test]
fn us_agent_creates_functional_sturdy_shield_and_attaches_it_to_the_source() {
    let definition = parse_oracle_card_definition("U.S.Agent, John Walker");
    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("U.S.Agent should have an enters trigger");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let us_agent = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let ally_definition = CardDefinitionBuilder::new(CardId::new(), "Shield Recipient")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let ally = game.create_object_from_definition(&ally_definition, alice, Zone::Battlefield);

    let mut ctx = ExecutionContext::new_default(us_agent, alice);
    for effect in trigger.effects.flattened_default_effects() {
        execute_effect(&mut game, effect, &mut ctx).expect("U.S.Agent's trigger should resolve");
    }

    let shields = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Sturdy Shield")
        })
        .collect::<Vec<_>>();
    let [shield] = shields.as_slice() else {
        panic!("the trigger should create exactly one Sturdy Shield: {shields:#?}");
    };
    let shield = *shield;
    let shield_object = game.object(shield).expect("the Shield token should exist");
    assert_eq!(shield_object.kind, ObjectKind::Token);
    assert!(shield_object.has_card_type(CardType::Artifact));
    assert!(shield_object.has_subtype(Subtype::Equipment));
    assert!(
        game.current_colors(shield)
            .is_some_and(|colors| colors.is_empty())
    );
    assert_eq!(
        shield_object.attached_to,
        Some(AttachmentTarget::Object(us_agent)),
        "the follow-up must attach the newly created token, not an arbitrary Equipment"
    );
    assert_eq!(game.calculated_power(us_agent), Some(4));
    assert_eq!(game.calculated_toughness(us_agent), Some(4));

    let equip = shield_object
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("Sturdy Shield should have an equip ability");
    assert_eq!(equip.timing, ActivationTiming::SorcerySpeed);
    assert!(
        format!("{:?}", equip.mana_cost).contains("Generic(2)"),
        "Sturdy Shield's equip activation must cost {{2}}: {:?}",
        equip.mana_cost
    );

    let mut equip_ctx = ExecutionContext::new_default(shield, alice)
        .with_targets(vec![ResolvedTarget::Object(ally)]);
    for effect in equip.effects.flattened_default_effects() {
        execute_effect(&mut game, effect, &mut equip_ctx)
            .expect("Sturdy Shield's equip ability should resolve");
    }
    assert_eq!(
        game.object(shield).and_then(|object| object.attached_to),
        Some(AttachmentTarget::Object(ally))
    );
    assert_eq!(game.calculated_power(us_agent), Some(3));
    assert_eq!(game.calculated_toughness(us_agent), Some(2));
    assert_eq!(game.calculated_power(ally), Some(3));
    assert_eq!(game.calculated_toughness(ally), Some(4));
}
