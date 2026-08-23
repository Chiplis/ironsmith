#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::effect::Effect;
use crate::effects::{ExecutionContext, ForPlayersEffect, UnlessPaysEffect, execute_effect};
use crate::object::ObjectKind;
use crate::types::{CardType, Subtype};

const ORACLE: &str = "When Acererak enters, if you haven't completed Tomb of Annihilation, return Acererak to its owner's hand and venture into the dungeon.\nWhenever Acererak attacks, for each opponent, you create a 2/2 black Zombie creature token unless that player sacrifices a creature of their choice.";

fn attack_loop(definition: &crate::cards::CardDefinition) -> &ForPlayersEffect {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .flat_map(|triggered| {
            triggered
                .effects
                .segments
                .iter()
                .flat_map(|segment| &segment.default_effects)
        })
        .find_map(|effect| effect.downcast_ref::<ForPlayersEffect>())
        .expect("Acererak's attack trigger should iterate its opponents")
}

fn quantified_unless(for_players: &ForPlayersEffect) -> &UnlessPaysEffect {
    let [effect] = for_players.effects.as_slice() else {
        panic!("each opponent should receive one independent unless choice: {for_players:#?}");
    };
    effect
        .downcast_ref::<UnlessPaysEffect>()
        .expect("the token creation should be gated by that opponent's sacrifice")
}

#[test]
fn acererak_keeps_controller_token_and_iterated_opponent_sacrifice_typed() {
    let definition = parse_oracle_card_definition("Acererak the Archlich");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let for_players = attack_loop(&definition);
    assert_eq!(for_players.filter, PlayerFilter::Opponent);
    assert!(!for_players.starting_with_controller);
    assert!(!for_players.stop_after_first_happened);
    let unless = quantified_unless(for_players);
    assert_eq!(unless.player, PlayerFilter::IteratedPlayer);
    let Some([cost]) = unless.cost.as_all() else {
        panic!("that opponent should pay exactly one creature-sacrifice cost: {unless:#?}");
    };
    let sacrifice = cost
        .effect_ref()
        .and_then(|effect| effect.downcast_ref::<crate::effects::SacrificeEffect>())
        .expect("that opponent's cost should be a typed creature sacrifice");
    assert_eq!(sacrifice.count, crate::effect::Value::Fixed(1));
    assert_eq!(sacrifice.filter.card_types, [CardType::Creature]);
    assert_eq!(sacrifice.filter.zone, Some(Zone::Battlefield));
    assert_eq!(
        sacrifice.filter.controller,
        Some(PlayerFilter::You),
        "the outer unless effect selects the opponent payer, while You inside the cost is payer-relative"
    );

    let [create] = unless.effects.as_slice() else {
        panic!("declining should create exactly one token: {unless:#?}");
    };
    let create = create
        .downcast_ref::<crate::effects::CreateTokenEffect>()
        .expect("the unpaid consequence should create the Zombie token");
    assert_eq!(create.controller, PlayerFilter::You);
    assert!(create.actor_surface_explicit);
    assert_eq!(create.count, crate::effect::Value::Fixed(1));
    assert_eq!(create.token.card.subtypes, [Subtype::Zombie]);

    let definition_debug = format!("{definition:#?}");
    let definition_debug_lower = definition_debug.to_ascii_lowercase();
    assert!(
        definition_debug.contains("PlayerCompletedDungeon")
            && definition_debug_lower.contains("tomb of annihilation")
            && definition_debug.contains("VentureIntoDungeonEffect"),
        "the unrelated terminal-dungeon branch must remain intact: {definition_debug}"
    );
}

#[test]
fn only_the_iterated_opponent_can_sacrifice_and_unpaid_opponents_create_for_you() {
    let definition = parse_oracle_card_definition("Acererak the Archlich");
    let outer = attack_loop(&definition).clone();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let mut game = crate::game_state::GameState::new(
        vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string()],
        20,
    );
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let sacrifice = CardDefinitionBuilder::new(CardId::new(), "Bob's Sacrifice")
        .card_types(vec![CardType::Creature])
        .build();
    let bob_creature = game.create_object_from_definition(&sacrifice, bob, Zone::Battlefield);
    let bob_creature_stable = game.object(bob_creature).expect("Bob's creature").stable_id;

    let mut decisions = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
    execute_effect(&mut game, &Effect::new(outer), &mut ctx)
        .expect("the per-opponent sacrifice/token choices should resolve");

    assert_eq!(
        game.find_object_by_stable_id(bob_creature_stable)
            .and_then(|id| game.object(id))
            .map(|object| object.zone),
        Some(Zone::Graveyard),
        "Bob should be able to pay with Bob's creature"
    );
    let zombies = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|object| {
            matches!(object.kind, ObjectKind::Token) && object.subtypes.contains(&Subtype::Zombie)
        })
        .collect::<Vec<_>>();
    assert_eq!(zombies.len(), 1, "only Carol should leave the cost unpaid");
    assert_eq!(game.controller_of(zombies[0]), alice);
    assert!(
        zombies
            .iter()
            .all(|token| game.controller_of(token) != bob && game.controller_of(token) != carol),
        "the quantified opponents must never become the token controllers"
    );
}
