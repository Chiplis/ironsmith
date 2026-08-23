#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::SelectFirstDecisionMaker;

fn test_card(name: &str, card_types: Vec<CardType>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .build()
}

fn resolve_ego_drain(
    game: &mut crate::GameState,
    definition: &CardDefinition,
    controller: PlayerId,
    opponent: PlayerId,
) {
    let source = game.create_object_from_definition(definition, controller, Zone::Stack);
    let mut decisions = SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, controller, &mut decisions)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(opponent)]);
    crate::game_loop::execute_resolution_program(
        game,
        &mut context,
        controller,
        source,
        definition
            .spell_effect
            .as_ref()
            .expect("Ego Drain should have a resolution program"),
        None,
        &[],
    )
    .expect("Ego Drain should resolve");
}

#[test]
fn ego_drain_keeps_the_faerie_control_gate_independent_from_discard_success() {
    let definition = parse_oracle_card_definition("Ego Drain");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card. If you don't control a Faerie, exile a card from your hand."
    );

    let effects = definition
        .spell_effect
        .as_ref()
        .expect("Ego Drain should have a resolution program")
        .flattened_default_effects();
    let conditional = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
        .expect("the final sentence should remain an independent state conditional");
    let crate::effect::Condition::Not(inner) = &conditional.condition else {
        panic!("expected a negative control condition: {conditional:#?}");
    };
    let crate::effect::Condition::PlayerControls { player, filter } = inner.as_ref() else {
        panic!("expected a typed player-controls condition: {inner:#?}");
    };
    assert_eq!(*player, PlayerFilter::You);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.subtypes.as_slice(), [Subtype::Faerie]);
    assert!(
        effects
            .iter()
            .all(|effect| effect.downcast_ref::<crate::effects::IfEffect>().is_none()),
        "the Faerie check must not become a DidNotHappen gate on the discard: {effects:#?}"
    );
}

#[test]
fn ego_drain_exiles_from_your_hand_only_when_you_control_no_faerie() {
    let definition = parse_oracle_card_definition("Ego Drain");

    for controls_faerie in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let discarded = game.create_object_from_definition(
            &test_card("Chosen Nonland", vec![CardType::Creature]),
            bob,
            Zone::Hand,
        );
        let discarded_stable_id = game
            .object(discarded)
            .expect("discard candidate should exist")
            .stable_id;
        let penalty_card = game.create_object_from_definition(
            &test_card("Penalty Card", vec![CardType::Creature]),
            alice,
            Zone::Hand,
        );
        let penalty_stable_id = game
            .object(penalty_card)
            .expect("penalty card should exist")
            .stable_id;
        if controls_faerie {
            game.create_object_from_definition(
                &CardDefinitionBuilder::new(CardId::new(), "Faerie Witness")
                    .card_types(vec![CardType::Creature])
                    .subtypes(vec![Subtype::Faerie])
                    .power_toughness(PowerToughness::fixed(1, 1))
                    .build(),
                alice,
                Zone::Battlefield,
            );
        }

        resolve_ego_drain(&mut game, &definition, alice, bob);

        assert_eq!(
            game.find_object_by_stable_id(discarded_stable_id)
                .and_then(|id| game.object(id))
                .map(|object| object.zone),
            Some(Zone::Graveyard),
            "the opponent's chosen nonland card must be discarded in either branch"
        );
        assert_eq!(
            game.find_object_by_stable_id(penalty_stable_id)
                .and_then(|id| game.object(id))
                .map(|object| object.zone),
            Some(if controls_faerie {
                Zone::Hand
            } else {
                Zone::Exile
            }),
            "the hand-exile penalty must depend on controlling a Faerie, not on whether the discard happened"
        );
    }
}
