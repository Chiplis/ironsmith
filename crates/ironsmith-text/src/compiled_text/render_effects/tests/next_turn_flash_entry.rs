use super::*;
const TEXT: &str = "+1: Until your next turn, you may cast creature spells as though they had flash, and each creature you control enters with an additional +1/+1 counter on it.";
#[test]
fn next_turn_flash_entry_expires_at_actual_next_turn_start() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Arlinn, the Pack's Hope")
            .card_types(vec![CardType::Planeswalker])
            .parse_text(TEXT)
            .unwrap();
    let AbilityKind::Activated(ability) = &definition.abilities[0].kind else {
        panic!("activated")
    };
    for schedule in 0..4 {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature spell")
            .card_types(vec![CardType::Creature])
            .build();
        let hand = game.create_object_from_card(&creature, alice, Zone::Hand);
        let opposing = game.create_object_from_card(&creature, bob, Zone::Hand);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        for effect in &ability.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx)
                .unwrap_or_else(|error| panic!("{error:?}: {effect:?}"));
        }
        let flash = crate::static_abilities::StaticAbility::flash();
        assert!(game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            hand,
            Zone::Hand,
            alice,
            &flash
        ));
        assert!(!game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            opposing,
            Zone::Hand,
            bob,
            &flash
        ));
        match schedule {
            1 => game.turn_store.extra_turns.push(alice),
            2 => game.turn_store.extra_turns.push(bob),
            3 => {
                game.turn_store.skip_next_turn.insert(alice);
            }
            _ => {}
        }
        let mut expired = false;
        for _ in 0..8 {
            game.next_turn();
            expired |= game.is_active_player(alice);
            assert_eq!(
                game.effect_store.grant_registry.card_has_granted_ability(
                    &game,
                    hand,
                    Zone::Hand,
                    alice,
                    &flash
                ),
                !expired,
                "schedule={schedule} active={:?} turn={}",
                game.turn.active_player,
                game.turn.turn_number
            );
        }
    }
}

#[test]
fn next_turn_entry_counter_rule_applies_to_future_entries_without_flash_antecedent() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Entry rule")
        .card_types(vec![CardType::Planeswalker])
        .parse_text("+1: Until your next turn, each creature you control enters with an additional +1/+1 counter on it.")
        .unwrap();
    let AbilityKind::Activated(ability) = &definition.abilities[0].kind else {
        panic!("activated")
    };
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = crate::effects::EffectContext::new_default(source, alice);
    for effect in &ability.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .unwrap_or_else(|error| panic!("{error:?}: {effect:?}"));
    }
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Future creature")
        .card_types(vec![CardType::Creature])
        .build();
    for owner in [alice, bob] {
        for zone in [Zone::Hand, Zone::Graveyard, Zone::Exile] {
            let object = game.create_object_from_card(&creature, owner, zone);
            let mut decisions = crate::decision::SelectFirstDecisionMaker;
            let entered = game
                .move_object_with_etb_processing_with_dm(object, Zone::Battlefield, &mut decisions)
                .unwrap()
                .new_id;
            assert_eq!(
                game.counter_count(entered, CounterType::PlusOnePlusOne),
                u32::from(owner == alice),
                "owner={owner:?} origin={zone:?}"
            );
        }
    }
}

#[test]
fn next_turn_flash_entry_shares_correct_start_duration_in_text() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Arlinn, the Pack's Hope")
            .card_types(vec![CardType::Planeswalker])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        TEXT
    );
}
