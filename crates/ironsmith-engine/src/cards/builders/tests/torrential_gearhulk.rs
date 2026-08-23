#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::{AutoPassDecisionMaker, SelectFirstDecisionMaker};
use crate::effects::{ExecutionContext, ResolvedTarget};
use crate::mana::{ManaCost, ManaSymbol};

const ORACLE_TEXT: &str = "Flash\nWhen this creature enters, you may cast target instant card from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead.";

fn torrential_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Torrential Gearhulk should have its enters trigger")
}

fn expensive_instant(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(7)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .build()
}

fn resolve_trigger(
    game: &mut crate::GameState,
    trigger: &crate::ability::TriggeredAbility,
    gearhulk: ObjectId,
    target: ObjectId,
    decisions: &mut dyn crate::decision::DecisionMaker,
) {
    let alice = PlayerId::from_index(0);
    let mut ctx = ExecutionContext::new(gearhulk, alice, decisions)
        .with_targets(vec![ResolvedTarget::Object(target)]);
    crate::game_loop::execute_resolution_program(
        game,
        &mut ctx,
        alice,
        gearhulk,
        &trigger.effects,
        None,
        &[],
    )
    .expect("Torrential Gearhulk's enters trigger should resolve");
}

#[test]
fn torrential_gearhulk_targets_only_an_instant_in_its_controllers_graveyard() {
    let definition = parse_oracle_card_definition("Torrential Gearhulk");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        ORACLE_TEXT
    );
    let trigger = torrential_trigger(&definition);
    let trigger_debug = format!("{trigger:#?}");
    assert!(
        trigger_debug.contains("CastTaggedEffect"),
        "{trigger_debug}"
    );
    assert!(
        trigger_debug.contains("without_paying_mana_cost: true"),
        "{trigger_debug}"
    );
    assert!(
        trigger_debug.contains("RegisterFutureZoneReplacementEffect"),
        "{trigger_debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let gearhulk = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let own_instant = game.create_object_from_definition(
        &expensive_instant("Alice Graveyard Instant"),
        alice,
        Zone::Graveyard,
    );
    let own_sorcery = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Alice Graveyard Sorcery")
            .card_types(vec![CardType::Sorcery])
            .build(),
        alice,
        Zone::Graveyard,
    );
    let opposing_instant = game.create_object_from_definition(
        &expensive_instant("Bob Graveyard Instant"),
        bob,
        Zone::Graveyard,
    );
    let hand_instant = game.create_object_from_definition(
        &expensive_instant("Alice Hand Instant"),
        alice,
        Zone::Hand,
    );

    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &trigger.effects,
        alice,
        Some(gearhulk),
        None,
    );
    let [requirement] = requirements.as_slice() else {
        panic!("expected one mandatory target requirement: {requirements:#?}");
    };
    assert_eq!(requirement.min_targets, 1);
    assert_eq!(requirement.max_targets, Some(1));
    assert_eq!(
        requirement.legal_targets,
        vec![crate::game_state::Target::Object(own_instant)],
        "only an instant card in Gearhulk's controller's graveyard is legal"
    );
    for illegal in [own_sorcery, opposing_instant, hand_instant] {
        assert!(
            !requirement
                .legal_targets
                .contains(&crate::game_state::Target::Object(illegal))
        );
    }
}

#[test]
fn torrential_gearhulk_may_decline_to_cast_its_target() {
    let definition = parse_oracle_card_definition("Torrential Gearhulk");
    let trigger = torrential_trigger(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let gearhulk = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let instant = game.create_object_from_definition(
        &expensive_instant("Declined Gearhulk Instant"),
        alice,
        Zone::Graveyard,
    );
    let stable_id = game
        .object(instant)
        .expect("instant should exist")
        .stable_id;

    let mut decisions = AutoPassDecisionMaker;
    resolve_trigger(&mut game, trigger, gearhulk, instant, &mut decisions);

    let declined = game
        .find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .expect("declined instant should still exist");
    assert_eq!(declined.zone, Zone::Graveyard);
    assert!(game.stack.is_empty(), "declining must not cast the target");
}

#[test]
fn torrential_gearhulk_casts_for_free_and_exiles_the_spell_on_resolution() {
    let definition = parse_oracle_card_definition("Torrential Gearhulk");
    let trigger = torrential_trigger(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let gearhulk = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let instant = game.create_object_from_definition(
        &expensive_instant("Expensive Gearhulk Instant"),
        alice,
        Zone::Graveyard,
    );
    let stable_id = game
        .object(instant)
        .expect("instant should exist")
        .stable_id;
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .total(),
        0,
        "the free-cast scenario must not provide mana for the eight-mana instant"
    );

    let mut decisions = SelectFirstDecisionMaker;
    resolve_trigger(&mut game, trigger, gearhulk, instant, &mut decisions);

    let stack_id = game
        .find_object_by_stable_id(stable_id)
        .expect("the targeted instant should retain stable identity");
    assert_eq!(
        game.object(stack_id)
            .expect("cast instant should exist")
            .zone,
        Zone::Stack,
        "an eight-mana instant must be cast despite an empty mana pool"
    );
    assert!(
        game.stack
            .iter()
            .any(|entry| entry.object_id == stack_id && entry.controller == alice)
    );

    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("the Gearhulk-cast instant should resolve");
    let resolved = game
        .find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .expect("resolved instant should still exist");
    assert_eq!(
        resolved.zone,
        Zone::Exile,
        "the linked one-shot replacement must exile this spell instead of returning it to the graveyard"
    );
    assert!(
        !game
            .player(alice)
            .expect("Alice should exist")
            .graveyard
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|object| object.stable_id == stable_id))
    );
}
