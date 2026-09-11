use super::*;
const TEXT: &str = "Whenever you cast a creature spell, if {S} of any of that spell's colors was spent to cast it, that creature enters with an additional +1/+1 counter on it.";
#[test]
fn snow_cast_entry_renders_a_future_entry_modification() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Boreal Outrider")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}
#[test]
fn ordinary_mana_does_not_satisfy_a_snow_mana_condition() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Green Creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Green,
        ]))
        .build();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.object_mut(source).unwrap().mana_spent_to_cast.green = 1;
    let ctx = crate::effects::EffectContext::new_default(source, alice);
    assert!(
        !crate::condition_eval::evaluate_condition_resolution(
            &game,
            &Condition::SnowManaOfAnySpellColorSpentToCastThisSpell,
            &ctx
        )
        .unwrap()
    );
}

#[test]
fn snow_cast_entry_requires_matching_snow_mana_and_survives_source_removal() {
    use crate::mana::{ManaCost, ManaSymbol};
    for (snow, symbol, spell_symbols, expected) in [
        (
            false,
            ManaSymbol::Green,
            vec![ManaSymbol::Generic(1), ManaSymbol::Green],
            false,
        ),
        (
            true,
            ManaSymbol::Blue,
            vec![ManaSymbol::Generic(1), ManaSymbol::Green],
            false,
        ),
        (
            true,
            ManaSymbol::Colorless,
            vec![ManaSymbol::Generic(1), ManaSymbol::Green],
            false,
        ),
        (
            true,
            ManaSymbol::Green,
            vec![ManaSymbol::Generic(1), ManaSymbol::Green],
            true,
        ),
        (true, ManaSymbol::Green, vec![ManaSymbol::Generic(2)], false),
        (
            true,
            ManaSymbol::Blue,
            vec![ManaSymbol::Green, ManaSymbol::Blue],
            true,
        ),
    ] {
        for remove_source in [false, true] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let definition =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Boreal Outrider")
                    .card_types(vec![CardType::Creature])
                    .parse_text(TEXT)
                    .unwrap();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let cost = ManaCost::from_symbols(spell_symbols.clone());
            let spell_def =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Green Creature")
                    .card_types(vec![CardType::Creature])
                    .mana_cost(cost.clone())
                    .build();
            let spell = game.create_object_from_definition(&spell_def, alice, Zone::Stack);
            game.stack
                .push(crate::game_state::StackEntry::new(spell, alice));
            for (is_snow, produced) in [(false, ManaSymbol::Green), (snow, symbol)] {
                let mut builder =
                    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Mana Source")
                        .card_types(vec![CardType::Land]);
                if is_snow {
                    builder = builder.supertypes(vec![crate::types::Supertype::Snow]);
                }
                let mana_source =
                    game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
                let mut mana_ctx = crate::effects::EffectContext::new_default(mana_source, alice);
                crate::effects::execute_effect(
                    &mut game,
                    &crate::effect::Effect::add_mana(vec![produced]),
                    &mut mana_ctx,
                )
                .unwrap();
                // Mana keeps its production-time snow property after its source leaves.
                game.move_object_by_effect(mana_source, Zone::Graveyard)
                    .unwrap();
            }
            let spent = game.player(alice).unwrap().mana_pool.clone();
            assert!(game.try_pay_mana_cost_with_reason(
                alice,
                Some(spell),
                &cost,
                0,
                crate::costs::PaymentReason::CastSpell
            ));
            game.object_mut(spell).unwrap().mana_spent_to_cast = spent;
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::SpellCastEvent::new_with_snapshot(
                    spell,
                    alice,
                    Zone::Hand,
                    crate::snapshot::ObjectSnapshot::from_object(
                        game.object(spell).unwrap(),
                        &game,
                    ),
                ),
                crate::provenance::ProvNodeId::default(),
            );
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(
                triggers.len(),
                usize::from(expected),
                "snow={snow}, symbol={symbol:?}"
            );
            if remove_source {
                game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            }
            if expected {
                let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                    .with_triggering_event(event);
                let condition = triggers[0].ability.intervening_if.as_ref().unwrap();
                assert!(
                    crate::condition_eval::evaluate_condition_resolution(&game, condition, &ctx)
                        .unwrap()
                );
                game.object_mut(spell).unwrap().color_override =
                    Some(crate::color::ColorSet::default());
                game.refresh_continuous_state();
                assert!(
                    !crate::condition_eval::evaluate_condition_resolution(&game, condition, &ctx)
                        .unwrap(),
                    "intervening condition must use the spell's current colors"
                );
                game.object_mut(spell).unwrap().color_override = None;
                game.refresh_continuous_state();
                assert!(
                    crate::condition_eval::evaluate_condition_resolution(&game, condition, &ctx)
                        .unwrap()
                );
                for effect in &triggers[0].ability.effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            assert_eq!(
                game.counter_count(spell, CounterType::PlusOnePlusOne),
                0,
                "counter must wait for entry"
            );
            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let entered = game
                .move_object_with_etb_processing_with_dm(spell, Zone::Battlefield, &mut dm)
                .unwrap();
            assert_eq!(
                game.counter_count(entered.new_id, CounterType::PlusOnePlusOne),
                u32::from(expected)
            );
        }
    }
}
