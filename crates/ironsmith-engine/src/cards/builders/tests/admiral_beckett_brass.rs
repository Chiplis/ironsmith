#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::game_state::Target;

const ORACLE: &str = "Other Pirates you control get +1/+1.\nAt the beginning of your end step, gain control of target nonland permanent controlled by a player who was dealt combat damage by three or more Pirates this turn.";

fn nested_control_effect(effect: &Effect) -> Option<crate::effects::ApplyContinuousEffect> {
    if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()
        && apply.target_spec.is_some()
    {
        return Some(apply.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = nested_control_effect(child);
        }
    });
    found
}

fn permanent(name: &str, card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build()
}

fn pirate(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Pirate])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

fn record_damage(game: &mut crate::GameState, source: ObjectId, player: PlayerId, is_combat: bool) {
    let cause = if is_combat {
        crate::events::cause::EventCause::combat_damage(source)
    } else {
        crate::events::cause::EventCause::effect()
    };
    game.record_turn_history_event(&crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Player(player),
            1,
            is_combat,
            cause,
        ),
        crate::provenance::ProvNodeId::default(),
    ));
}

#[test]
fn admiral_beckett_brass_keeps_distinct_pirate_damage_controller_targeting() {
    let definition = parse_oracle_card_definition("Admiral Beckett Brass");

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Admiral should have an end-step trigger");
    let control = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(nested_control_effect)
        .expect("the trigger should have a targeted control effect");
    let target_spec = control
        .target_spec
        .as_ref()
        .expect("the control effect should preserve its target spec");
    let ChooseSpec::Object(filter) = target_spec.base() else {
        panic!("expected an object target filter, got {target_spec:#?}");
    };
    assert!(
        filter.subtypes.is_empty(),
        "Pirate leaked onto the target: {filter:#?}"
    );
    assert_eq!(filter.excluded_card_types, [CardType::Land]);
    assert!(matches!(
        filter.controller.as_ref(),
        Some(PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { minimum, .. })
            if *minimum == 3
    ));
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    let mut game = crate::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
        20,
    );
    let admiral = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bob_permanent = game.create_object_from_definition(
        &permanent("Bob's Relic", CardType::Artifact),
        bob,
        Zone::Battlefield,
    );
    let cara_permanent = game.create_object_from_definition(
        &permanent("Cara's Relic", CardType::Artifact),
        cara,
        Zone::Battlefield,
    );
    let bob_land = game.create_object_from_definition(
        &permanent("Bob's Land", CardType::Land),
        bob,
        Zone::Battlefield,
    );

    let pirate_a =
        game.create_object_from_definition(&pirate("Pirate A"), alice, Zone::Battlefield);
    let pirate_b =
        game.create_object_from_definition(&pirate("Pirate B"), alice, Zone::Battlefield);
    let pirate_c =
        game.create_object_from_definition(&pirate("Pirate C"), alice, Zone::Battlefield);
    let non_pirate = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Non-Pirate")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Battlefield,
    );

    for pirate in [pirate_a, pirate_b, pirate_c] {
        record_damage(&mut game, pirate, bob, true);
    }
    record_damage(&mut game, pirate_a, cara, true);
    record_damage(&mut game, pirate_a, cara, true);
    record_damage(&mut game, pirate_b, cara, true);
    record_damage(&mut game, pirate_c, cara, false);
    record_damage(&mut game, non_pirate, cara, true);

    let legal = crate::game_loop::compute_legal_targets(&game, target_spec, alice, Some(admiral));
    assert!(legal.contains(&Target::Object(bob_permanent)), "{legal:#?}");
    assert!(
        !legal.contains(&Target::Object(cara_permanent)),
        "duplicate, noncombat, and non-Pirate sources must not satisfy the threshold: {legal:#?}"
    );
    assert!(
        !legal.contains(&Target::Object(bob_land)),
        "the qualified player's land must remain excluded: {legal:#?}"
    );
}
