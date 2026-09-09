use super::*;

const TEXT: &str = "The first spell you cast with {X} in its mana cost each turn costs {1} less to cast for each +1/+1 counter on Zimone.\nWhenever you cast your first spell with {X} in its mana cost each turn, put two +1/+1 counters on Zimone.";

#[test]
fn first_x_counter_discount_compiles_complete_named_source() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zimone, Infinite Analyst")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn first_x_counter_discount_uses_source_counters_and_first_matching_cast() {
    use crate::mana::{ManaCost, ManaSymbol};
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zimone, Infinite Analyst")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    let trigger = definition
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t),
            _ => None,
        })
        .unwrap();
    for (initial, x_value) in [0, 3, 9].into_iter().flat_map(|n| [0, 4].map(|x| (n, x))) {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.add_counters(source, CounterType::PlusOnePlusOne, initial);
        game.add_counters(source, CounterType::Charge, 10);
        let other = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let other = game.create_object_from_card(&other, alice, Zone::Battlefield);
        game.add_counters(other, CounterType::PlusOnePlusOne, 20);
        for turn in 0..2 {
            if turn == 1 {
                game.next_turn();
            }
            for (caster, has_x, first_matching) in [
                (alice, false, false),
                (bob, true, false),
                (alice, true, true),
                (alice, true, false),
            ] {
                let mut symbols = vec![ManaSymbol::Generic(5), ManaSymbol::Green];
                if has_x {
                    symbols.insert(0, ManaSymbol::X);
                }
                let cost = ManaCost::from_symbols(symbols);
                let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Cast Probe")
                    .card_types(vec![CardType::Sorcery])
                    .mana_cost(cost)
                    .build();
                let spell = game.create_object_from_card(&card, caster, Zone::Stack);
                game.push_to_stack(
                    crate::game_state::StackEntry::new(spell, caster)
                        .with_casting_method(crate::alternative_cast::CastingMethod::Normal),
                );
                // The payable cost has X already expanded; the printed card
                // still carries X for filtering.
                let payable = ManaCost::from_symbols(vec![
                    ManaSymbol::Generic(if has_x { 5 + x_value } else { 5 }),
                    ManaSymbol::Green,
                ]);
                let reduction = if first_matching {
                    initial + 2 * turn
                } else {
                    0
                };
                let remaining = (if has_x { 5 + x_value as i32 } else { 5 }) - reduction as i32;
                let expected = if remaining <= 0 {
                    "{G}".to_string()
                } else {
                    format!("{{{remaining}}}{{G}}")
                };
                let in_hand = game.create_object_from_card(&card, caster, Zone::Hand);
                assert_eq!(
                    crate::decision::calculate_effective_mana_cost(
                        &game,
                        caster,
                        game.object(in_hand).unwrap(),
                        &payable
                    )
                    .to_oracle(),
                    expected,
                    "hand preview: initial={initial}, turn={turn}, caster={caster:?}, has_x={has_x}, first={first_matching}"
                );
                assert_eq!(
                    crate::decision::calculate_effective_mana_cost(
                        &game,
                        caster,
                        game.object(spell).unwrap(),
                        &payable
                    )
                    .to_oracle(),
                    expected,
                    "initial={initial}, turn={turn}, caster={caster:?}, has_x={has_x}, first={first_matching}"
                );
                let snapshot = crate::snapshot::ObjectSnapshot::from_object(
                    game.object(spell).unwrap(),
                    &game,
                );
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::SpellCastEvent::new_with_snapshot(
                        spell,
                        caster,
                        Zone::Hand,
                        snapshot,
                    ),
                    crate::provenance::ProvNodeId::default(),
                );
                game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event.clone());
                assert_eq!(
                    crate::triggers::check_triggers(&game, &event).len(),
                    usize::from(first_matching),
                    "initial={initial}, turn={turn}, caster={caster:?}, has_x={has_x}, first={first_matching}"
                );
                if first_matching {
                    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                        .with_triggering_event(event);
                    for effect in &trigger.effects {
                        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    }
                    assert_eq!(
                        game.object(source)
                            .unwrap()
                            .counters
                            .get(&CounterType::PlusOnePlusOne)
                            .copied()
                            .unwrap_or(0),
                        initial + 2 * (turn + 1)
                    );
                }
            }
        }
    }
}

#[test]
fn named_counter_discounts_scale_typed_and_untyped_source_counters() {
    use crate::mana::{ManaCost, ManaSymbol};
    for (descriptor, expected_generic) in [("charge counter", 6), ("counter", 2)] {
        let text = format!(
            "Spells you cast cost {{2}} less to cast for each {descriptor} on Counter Engine."
        );
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter Engine")
                .card_types(vec![CardType::Artifact])
                .parse_text(&text)
                .unwrap();
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.add_counters(source, CounterType::Charge, 2);
        game.add_counters(source, CounterType::PlusOnePlusOne, 2);
        let cost = ManaCost::from_symbols(vec![ManaSymbol::Generic(10), ManaSymbol::Blue]);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Ordinary Spell")
            .card_types(vec![CardType::Instant])
            .mana_cost(cost.clone())
            .build();
        let spell = game.create_object_from_card(&card, alice, Zone::Hand);
        assert_eq!(
            crate::decision::calculate_effective_mana_cost(
                &game,
                alice,
                game.object(spell).unwrap(),
                &cost
            )
            .to_oracle(),
            format!("{{{expected_generic}}}{{U}}")
        );
    }
}
