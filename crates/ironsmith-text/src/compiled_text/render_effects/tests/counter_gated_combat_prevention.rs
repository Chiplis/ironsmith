use super::*;
const TEXT: &str = "This creature enters with three ice counters on it.\nAs long as this creature has an ice counter on it, prevent all combat damage it would deal and it has defender.\nWhenever this creature blocks, remove an ice counter from it.";
#[test]
fn counter_gated_combat_prevention_and_defender_apply_only_to_source() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Woolly Razorback")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(7, 7))
        .parse_text(TEXT).unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let source = game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield,
        &mut crate::decision::SelectFirstDecisionMaker).unwrap().new_id;
    let other = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other creature")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
    let own = game.create_object_from_card(&other, alice, Zone::Battlefield);
    let opposing = game.create_object_from_card(&other, bob, Zone::Battlefield);
    assert_eq!(game.object(source).unwrap().counters.get(&crate::CounterType::Ice), Some(&3));
    for remaining in (0..=3).rev() {
        game.refresh_continuous_state();
        assert_eq!(game.current_has_static_ability_id(source, crate::static_abilities::StaticAbilityId::Defender), remaining > 0);
        assert_eq!(game.current_has_static_ability_id(source, crate::static_abilities::StaticAbilityId::PreventAllCombatDamageDealtByThisPermanent), remaining > 0);
        for id in [own, opposing] {
            assert!(!game.current_has_static_ability_id(id, crate::static_abilities::StaticAbilityId::Defender));
            assert!(!game.current_has_static_ability_id(id, crate::static_abilities::StaticAbilityId::PreventAllCombatDamageDealtByThisPermanent));
        }
        for (damage_source, combat, prevented) in [
            (source, true, remaining > 0), (source, false, false), (own, true, false),
        ] {
            let before = game.player(bob).unwrap().life;
            let damage = Effect::new(crate::effects::DealDamageEffect::new(
                Value::Fixed(1), ChooseSpec::SpecificPlayer(bob),
            ).with_combat(combat));
            let mut ctx = crate::effects::EffectContext::new_default(damage_source, alice);
            crate::effects::execute_effect(&mut game, &damage, &mut ctx).unwrap();
            assert_eq!(game.player(bob).unwrap().life, before - i32::from(!prevented),
                "remaining={remaining}, combat={combat}, source={damage_source:?}");
        }
        if remaining > 0 {
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::combat::CreatureBlockedEvent::new(source, opposing),
                crate::provenance::ProvNodeId::default());
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(triggers.len(), 1);
            let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_triggering_event(event);
            for effect in &triggers[0].ability.effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            assert_eq!(game.object(source).unwrap().counters.get(&crate::CounterType::Ice).copied().unwrap_or(0), remaining - 1);
        }
    }
    game.add_counters(source, crate::CounterType::Ice, 1);
    game.refresh_continuous_state();
    assert!(game.current_has_static_ability_id(source, crate::static_abilities::StaticAbilityId::Defender));
}
#[test]
fn counter_gated_combat_prevention_preserves_both_instructions() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Woolly Razorback")
        .card_types(vec![CardType::Creature]).parse_text(TEXT).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), TEXT);
}

#[test]
fn counter_gated_prevention_generalizes_counter_and_keyword() {
    let text = TEXT.replace("ice", "charge").replace("an charge", "a charge").replace("defender", "vigilance");
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Conditional prevention")
        .card_types(vec![CardType::Creature]).parse_text(&text).unwrap();
    // Replacing the counter noun also changes its grammatical article.
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), text);
}
