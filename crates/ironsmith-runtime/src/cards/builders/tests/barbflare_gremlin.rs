#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const BARBFLARE_TEXT: &str = "First strike, haste\nWhenever a player taps a land for mana, if this creature is tapped, that player adds one mana of any type that land produced. Then that land deals 1 damage to that player.";

fn barbflare_triggered(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::TapForManaTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Barbflare Gremlin should have its tap-for-mana trigger")
}

fn mana_event(
    game: &crate::GameState,
    land: ObjectId,
    player: PlayerId,
) -> crate::triggers::TriggerEvent {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(land).expect("triggering land should exist"),
        game,
    );
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ManaAddedEvent::new(
            land,
            player,
            player,
            vec![crate::mana::ManaSymbol::Green],
        )
        .with_snapshot(Some(snapshot))
        .with_production_provenance(
            crate::events::mana::ManaProductionProvenance::TappedSourceForMana,
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

#[test]
fn barbflare_keeps_triggering_land_source_and_leading_then_surface() {
    let definition = parse_oracle_card_definition("Barbflare Gremlin");
    assert_eq!(compiled_text_lines(&definition).join("\n"), BARBFLARE_TEXT);

    let triggered = barbflare_triggered(&definition);
    assert_eq!(
        triggered.intervening_if,
        Some(crate::ConditionExpr::SourceIsTapped)
    );
    let [mana_segment, damage_segment] = triggered.effects.segments.as_slice() else {
        panic!(
            "the two authored sentences should remain separate resolution segments: {:#?}",
            triggered.effects
        );
    };
    assert!(matches!(
        mana_segment.default_effects.as_slice(),
        [effect]
            if effect
                .downcast_ref::<crate::effects::AddManaOfLandProducedTypesEffect>()
                .is_some()
    ));
    let [leading_then] = damage_segment.default_effects.as_slice() else {
        panic!("the damage sentence should have one typed wrapper: {damage_segment:#?}");
    };
    let sequence = leading_then
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the second sentence should retain leading-Then provenance");
    assert_eq!(
        sequence.surface,
        ironsmith_core::SequenceSurface::SentenceLeadingThen
    );
    let [damage_effect] = sequence.effects.as_slice() else {
        panic!("the second sentence should contain one damage action: {sequence:#?}");
    };
    let with_source = damage_effect
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        .expect("the triggering land, not the Gremlin, should execute the damage");
    assert!(matches!(
        with_source.source.base(),
        ChooseSpec::Tagged(tag) if tag.as_str() == "triggering"
    ));
    assert_eq!(
        with_source.source.source_reference_surface(),
        Some(&crate::target::SourceReferenceSurface::ThisPermanentType(
            "that land".to_string()
        ))
    );
    let damage = with_source
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .expect("the source wrapper should contain typed damage");
    assert_eq!(damage.amount, crate::effect::Value::Fixed(1));
    assert!(matches!(
        damage.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ));
}

#[test]
fn tapped_barbflare_credits_the_land_as_the_damage_source() {
    let definition = parse_oracle_card_definition("Barbflare Gremlin");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let gremlin = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let land_definition = CardDefinitionBuilder::new(CardId::new(), "Barbflare Test Land")
        .card_types(vec![CardType::Land])
        .build();
    let land = game.create_object_from_definition(&land_definition, bob, Zone::Battlefield);
    game.tap(gremlin);

    let event = mana_event(&game, land, bob);
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == gremlin)
    {
        queue.add(entry);
    }
    assert_eq!(queue.entries.len(), 1);

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("Barbflare's triggered mana ability should resolve");
    if !game.stack.is_empty() {
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
            .expect("a non-immediate Barbflare trigger should resolve from the stack");
    }

    assert_eq!(game.player(bob).expect("Bob should exist").life, 19);
    assert_eq!(
        game.player(bob).expect("Bob should exist").mana_pool.green,
        1
    );
    assert!(
        game.turn_store
            .turn_history
            .source_dealt_damage_to_player_this_turn(land, None, bob),
        "the land must be the recorded damage source"
    );
    assert!(
        !game
            .turn_store
            .turn_history
            .source_dealt_damage_to_player_this_turn(gremlin, None, bob),
        "the Gremlin must not be substituted as the damage source"
    );
}

#[test]
fn untapped_barbflare_is_an_executable_intervening_if_near_miss() {
    let definition = parse_oracle_card_definition("Barbflare Gremlin");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let gremlin = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let land_definition = CardDefinitionBuilder::new(CardId::new(), "Untapped Barbflare Land")
        .card_types(vec![CardType::Land])
        .build();
    let land = game.create_object_from_definition(&land_definition, bob, Zone::Battlefield);

    let event = mana_event(&game, land, bob);
    let barbflare_entries = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == gremlin)
        .count();
    assert_eq!(barbflare_entries, 0);
    assert_eq!(game.player(bob).expect("Bob should exist").life, 20);
    assert!(
        !game
            .turn_store
            .turn_history
            .source_dealt_damage_to_player_this_turn(land, None, bob)
    );
}
