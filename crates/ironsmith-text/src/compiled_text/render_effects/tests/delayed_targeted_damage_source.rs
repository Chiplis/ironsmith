use super::*;

const TEXT: &str = "Flying, haste\nWhenever this creature or another Dinosaur you control with flying enters, gain control of target creature an opponent controls until end of turn. Untap that creature. It gains flying and haste until end of turn. At the beginning of the next end step, target land deals 3 damage to that creature.";

fn definition() -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Swooping Pteranodon")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dinosaur])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text(TEXT)
        .unwrap()
}

#[test]
fn delayed_damage_uses_the_later_targeted_land_and_keeps_the_original_creature() {
    let definition = definition();
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .unwrap();
    for removal in 0..4 {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Stolen Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(5, 5))
            .build();
        let victim = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        let unrelated = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        game.tap(victim);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(victim)]);
        ctx.snapshot_targets(&game);
        for effect in &triggered.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx)
                .unwrap_or_else(|error| panic!("immediate effect failed: {error:?}: {effect:#?}"));
        }
        assert_eq!(game.controller_of(game.object(victim).unwrap()), alice);
        assert!(!game.is_tapped(victim));
        assert_eq!(game.damage_on(victim), 0);
        // The land did not exist when the original ability resolved.
        let land_card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Later Land")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_card(&land_card, bob, Zone::Battlefield);
        if removal == 2 {
            game.move_object_by_effect(victim, Zone::Graveyard).unwrap();
        }
        if removal == 3 {
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        }
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfEndStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let mut queue = crate::triggers::TriggerQueue::new();
        for trigger in crate::triggers::check_delayed_triggers(&mut game, &event) {
            queue.add(trigger);
        }
        assert_eq!(
            queue.entries.len(),
            1,
            "the delayed ability must survive its source: removal={removal}"
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        assert!(
            game.stack[0]
                .targets
                .contains(&crate::game_state::Target::Object(land))
        );
        if removal == 1 {
            game.move_object_by_effect(land, Zone::Graveyard).unwrap();
        }
        let mut pass = crate::decision::AutoPassDecisionMaker;
        crate::game_loop::run_priority_loop_with(&mut game, &mut queue, &mut pass).unwrap();
        let deals_damage = removal == 0 || removal == 3;
        if removal != 2 {
            assert_eq!(game.damage_on(victim), if deals_damage { 3 } else { 0 });
        }
        assert_eq!(game.damage_on(unrelated), 0);
        assert_eq!(game.damage_on(land), 0);
        if deals_damage {
            assert!(game.source_dealt_damage_to_object_this_game(land, victim));
            assert!(!game.source_dealt_damage_to_object_this_game(victim, victim));
        }
    }
}

#[test]
fn delayed_targeted_damage_renders_the_land_as_the_source() {
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition()).join("\n"),
        TEXT
    );
}

#[test]
fn delayed_damage_renders_other_source_types_and_amounts() {
    for (source, amount) in [("artifact", 5), ("creature", 2), ("nonbasic land", 4)] {
        let text = TEXT.replace(
            "target land deals 3",
            &format!("target {source} deals {amount}"),
        );
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Delayed Damage Probe")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Dinosaur])
                .parse_text(&text)
                .unwrap();
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition).join("\n"),
            text
        );
    }
}

#[test]
fn delayed_targets_do_not_become_targets_of_the_registering_spell() {
    for timing in [
        "At the beginning of the next end step",
        "At the beginning of the next upkeep",
        "At the beginning of the next draw step",
    ] {
        let text = format!("You gain 1 life. {timing}, destroy target artifact.");
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Delayed Target Probe")
                .card_types(vec![CardType::Sorcery])
                .parse_text(&text)
                .unwrap();
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        for effect in definition.spell_effect.as_ref().unwrap() {
            crate::effects::execute_effect(&mut game, effect, &mut ctx)
                .unwrap_or_else(|error| panic!("{timing}: {error:?}: {effect:#?}"));
        }
    }
}
