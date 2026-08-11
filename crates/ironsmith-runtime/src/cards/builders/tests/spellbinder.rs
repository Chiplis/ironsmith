#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::SelectFirstDecisionMaker;
use crate::effects::ExecutionContext;

#[test]
fn spellbinder_imprint_and_combat_trigger_cast_a_copy_of_the_linked_exiled_card() {
    let definition = parse_oracle_card_definition("Spellbinder");
    let triggers = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect::<Vec<_>>();
    let imprint = triggers
        .iter()
        .copied()
        .find(|triggered| format!("{:#?}", triggered.effects).contains("ImprintFromHandEffect"))
        .expect("Spellbinder should have its Imprint enters trigger");
    let copy_cast = triggers
        .iter()
        .copied()
        .find(|triggered| format!("{:#?}", triggered.effects).contains("CastTaggedEffect"))
        .expect("Spellbinder should have its combat-damage copy/cast trigger");
    let copy_debug = format!("{copy_cast:#?}");
    assert!(
        copy_debug.contains("DealsCombatDamageToPlayer"),
        "the copy ability must remain a combat-damage trigger: {copy_debug}"
    );
    assert!(copy_debug.contains(crate::tag::SOURCE_EXILED_TAG));
    assert!(copy_debug.contains("as_copy: true"));
    assert!(copy_debug.contains("without_paying_mana_cost: true"));
    assert!(
        !copy_debug.contains("CopySpellEffect"),
        "the linked exiled card is not a spell on the stack: {copy_debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let spellbinder = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let instant_definition = CardDefinitionBuilder::new(CardId::new(), "Imprinted Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let instant = game.create_object_from_definition(&instant_definition, alice, Zone::Hand);
    let instant_stable = game.object(instant).expect("instant exists").stable_id;

    let mut imprint_decisions = SelectFirstDecisionMaker;
    let mut imprint_ctx = ExecutionContext::new(spellbinder, alice, &mut imprint_decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut imprint_ctx,
        alice,
        spellbinder,
        &imprint.effects,
        None,
        &[],
    )
    .expect("Spellbinder's Imprint trigger should resolve");

    let exiled_instant = game
        .find_object_by_stable_id(instant_stable)
        .expect("the imprinted card should retain stable identity");
    assert_eq!(
        game.object(exiled_instant)
            .expect("imprinted card exists")
            .zone,
        Zone::Exile
    );
    assert!(
        game.get_exiled_with_source_links(spellbinder)
            .contains(&exiled_instant),
        "the Imprint trigger must link the exiled card to Spellbinder"
    );

    let mut copy_decisions = SelectFirstDecisionMaker;
    let mut copy_ctx = ExecutionContext::new(spellbinder, alice, &mut copy_decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut copy_ctx,
        alice,
        spellbinder,
        &copy_cast.effects,
        None,
        &[],
    )
    .expect("Spellbinder's linked copy/cast trigger should resolve");

    assert_eq!(
        game.object(exiled_instant)
            .expect("original card remains")
            .zone,
        Zone::Exile,
        "casting the copy must not move the original imprinted card"
    );
    let copied_spell = game
        .stack
        .iter()
        .find(|entry| {
            game.object(entry.object_id)
                .is_some_and(|object| object.name == "Imprinted Instant")
        })
        .expect("the free-cast copy should be on the stack");
    assert_ne!(copied_spell.object_id, exiled_instant);
}
