use super::*;

const ORACLE: &str = "This spell costs {X} less to cast, where X is the total toughness of creatures you control.\nDefender\n{2}{U}{U}: Until end of turn, target creature you control gets +1/+0, gains \"Whenever this creature deals combat damage to a player, draw cards equal to its toughness,\" and can attack as though it didn't have defender.";

#[test]
fn aggregate_discount_and_temporary_combat_draw_use_correct_creatures() {
    let cost = crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(10), crate::mana::ManaSymbol::Green]);
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "The Pride of Hull Clade")
        .card_types(vec![CardType::Creature]).mana_cost(cost.clone()).parse_text(ORACLE).unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let mut own = Vec::new();
    for (player, toughness) in [(alice, 2), (alice, 3), (bob, 9)] {
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Recipient")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(1, toughness)).build();
        let id = game.create_object_from_card(&creature, player, Zone::Battlefield);
        if player == alice { own.push(id); }
    }
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let reduced = crate::decision::calculate_effective_mana_cost(&game, alice, game.object(source).unwrap(), &cost);
    assert_eq!(reduced.to_oracle(), "{5}{G}", "sum both controlled creatures, exclude opponent");
    let source = game.move_object_by_effect(source, Zone::Battlefield).unwrap();
    let activation = definition.abilities.iter().find_map(|a| if let AbilityKind::Activated(a)= &a.kind { Some(a) } else { None }).unwrap();
    let target = own[1];
    let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    ctx.snapshot_targets(&game);
    for segment in &activation.effects.segments { for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); } }
    assert_eq!(game.current_power(target), Some(2));
    assert_eq!(game.current_power(own[0]), Some(1));
    assert!(game.current_has_static_ability_id(target, crate::static_abilities::StaticAbilityId::CanAttackAsThoughNoDefender));
    assert!(!game.current_has_static_ability_id(own[0], crate::static_abilities::StaticAbilityId::CanAttackAsThoughNoDefender));
    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Draw fixture").card_types(vec![CardType::Land]).build();
    for _ in 0..10 { game.create_object_from_card(&card, alice, Zone::Library); }
    for (dealer, combat, expected) in [(own[0], true, 0), (target, false, 0), (target, true, 1)] {
        let event = crate::triggers::TriggerEvent::new_with_provenance(crate::events::DamageEvent::with_cause(dealer, crate::events::DamageTarget::Player(bob), 1, combat, crate::events::cause::EventCause::from_sba()), crate::provenance::ProvNodeId::default());
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), expected);
        for entry in triggers {
            let mut ctx = crate::effects::EffectContext::new_default(entry.source, entry.controller).with_triggering_event(entry.triggering_event);
            for segment in &entry.ability.effects.segments { for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); } }
        }
    }
    assert_eq!(game.player(alice).unwrap().hand.len(), 3);
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state();
    assert_eq!(game.current_power(target), Some(1));
    assert!(game.current_abilities(target).unwrap().is_empty());
}

#[test]
fn aggregate_discount_temporary_grant_keeps_leading_duration() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "The Pride of Hull Clade")
        .card_types(vec![CardType::Creature]).parse_text(ORACLE).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), ORACLE);
}
