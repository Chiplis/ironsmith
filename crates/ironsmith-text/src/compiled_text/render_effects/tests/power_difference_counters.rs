use super::*;
const TEXT: &str = "Whenever a creature dies, if it had power greater than this creature's power, put a number of +1/+1 counters on this creature equal to the difference.";
fn definition() -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Power Difference")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text(TEXT)
        .unwrap()
}
#[test]
fn power_difference_counters_use_death_snapshot_and_current_source_power() {
    for exile_before_resolution in [false, true] {
        for (victim_power, victim_bonus, source_bonus, expected) in [
            (2, 0, 0, 0),
            (3, 0, 0, 0),
            (7, 0, 0, 4),
            (4, 3, 0, 4),
            (7, 0, 2, 2),
            (7, 0, 4, 0),
        ] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let source =
                game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
            let victim_card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Victim")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(victim_power, 3))
                .build();
            let victim = game.create_object_from_card(&victim_card, bob, Zone::Battlefield);
            for _ in 0..victim_bonus {
                game.object_mut(victim)
                    .unwrap()
                    .add_counters(crate::CounterType::PlusOnePlusOne, 1);
            }
            let snapshot =
                crate::snapshot::ObjectSnapshot::from_object(game.object(victim).unwrap(), &game);
            let dead = game.move_object_by_effect(victim, Zone::Graveyard).unwrap();
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::ZoneChangeEvent::with_results(
                    victim,
                    vec![dead],
                    Zone::Battlefield,
                    Zone::Graveyard,
                    crate::events::cause::EventCause::effect(),
                    Some(snapshot),
                ),
                crate::provenance::ProvNodeId::default(),
            );
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(
                triggers.len(),
                usize::from(victim_power + victim_bonus > 3),
                "victim={victim_power}+{victim_bonus}, source+={source_bonus}"
            );
            let mut queue = crate::triggers::TriggerQueue::new();
            for trigger in triggers {
                queue.add(trigger);
            }
            crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            for _ in 0..source_bonus {
                game.object_mut(source)
                    .unwrap()
                    .add_counters(crate::CounterType::PlusOnePlusOne, 1);
            }
            // The departed card can leave its graveyard before resolution; use LKI.
            if exile_before_resolution {
                game.move_object_by_effect(dead, Zone::Exile).unwrap();
            }
            if !game.stack.is_empty() {
                crate::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
            assert_eq!(
                game.object(source)
                    .unwrap()
                    .counters
                    .get(&crate::CounterType::PlusOnePlusOne)
                    .copied()
                    .unwrap_or(0),
                (source_bonus + expected) as u32,
                "victim={victim_power}+{victim_bonus}, source+={source_bonus}"
            );
        }
    }
}

#[test]
fn power_difference_counters_render_the_correlated_difference() {
    let rendered = crate::compiled_text::compiled_text_lines(&definition()).join("\n");
    assert!(rendered.contains("equal to the difference"), "{rendered}");
}

#[test]
fn power_difference_counters_do_not_hide_unrelated_amounts() {
    for amount in [
        Value::Fixed(2),
        Value::Add(
            Box::new(Value::PowerOf(Box::new(ChooseSpec::Tagged(
                "triggering".into(),
            )))),
            Box::new(Value::Fixed(-2)),
        ),
    ] {
        let mut definition = definition();
        let crate::ability::AbilityKind::Triggered(triggered) = &mut definition.abilities[0].kind
        else {
            panic!("trigger");
        };
        let effect = &mut triggered.effects.segments[0].default_effects[1];
        let mut counters = effect
            .downcast_ref::<crate::effects::PutCountersEffect>()
            .unwrap()
            .clone();
        counters.amount = amount;
        *effect = Effect::new(counters);
        triggered.effects =
            ironsmith_core::ResolutionProgram::new(triggered.effects.segments.clone());
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(!rendered.contains("equal to the difference"), "{rendered}");
    }
}
