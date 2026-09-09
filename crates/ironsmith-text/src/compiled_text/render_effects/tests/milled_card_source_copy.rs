use super::*;
const TEXT: &str = "Flash\nAt the beginning of your upkeep, each player mills three cards. You may exile a creature card from among the cards milled this way. If you do, this creature becomes a copy of that card, except it has this ability.";
struct CopyChoice(bool);
impl crate::decision::DecisionMaker for CopyChoice {
    fn decide_boolean(&mut self, _: &crate::game_state::GameState,
        _: &crate::decisions::context::BooleanContext) -> bool { self.0 }
}
#[test]
fn milled_card_copy_changes_only_source_and_keeps_the_upkeep_ability() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Shadow Kin")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .parse_text(TEXT).unwrap();
    let trigger = definition.abilities.iter().find_map(|a| match &a.kind {
        AbilityKind::Triggered(t) => Some(t), _ => None,
    }).unwrap();
    for creature_owner in [None, Some(0), Some(1), Some(2)] {
        for accept in [false, true] {
            let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let other_card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other creature")
                .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(3, 3)).build();
            let other = game.create_object_from_card(&other_card, alice, Zone::Battlefield);
            game.create_object_from_card(&other_card, alice, Zone::Graveyard);
            for index in 0..2 {
                let owner = game.players[index].id;
                for slot in 0..3 {
                    let creature = (creature_owner == Some(index) || creature_owner == Some(2)) && slot == 0;
                    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), if creature { "Copy candidate" } else { "Land" })
                        .card_types(vec![if creature { CardType::Creature } else { CardType::Land }])
                        .power_toughness(crate::card::PowerToughness::fixed(5, 6)).build();
                    game.create_object_from_card(&card, owner, Zone::Library);
                }
            }
            let mut dm = CopyChoice(accept);
            let mut ctx = crate::effects::EffectContext::new(source, alice, &mut dm)
                .with_trigger_identity(crate::triggers::compute_trigger_identity(trigger));
            ctx.source_snapshot = Some(crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(), &game));
            for (index, effect) in trigger.effects.iter().enumerate() {
                crate::effects::execute_effect(&mut game, effect, &mut ctx)
                    .unwrap_or_else(|error| panic!("owner={creature_owner:?}, accept={accept}, effect={index}: {error:?}"));
            }
            game.refresh_continuous_state();
            assert!(game.players.iter().all(|player| player.library.is_empty()));
            let copied = accept && creature_owner.is_some();
            assert_eq!(game.calculated_power(source), Some(if copied { 5 } else { 2 }), "owner={creature_owner:?}, accept={accept}");
            assert_eq!(game.calculated_toughness(source), Some(if copied { 6 } else { 2 }));
            assert_eq!(game.calculated_power(other), Some(3));
            assert_eq!(game.exile.len(), usize::from(copied));
            assert_eq!(game.current_has_static_ability_id(source, crate::static_abilities::StaticAbilityId::Flash), !copied,
                "the copy exception retains this upkeep ability, not unrelated Flash");
            assert!(game.current_abilities(source).unwrap().iter().any(|a| matches!(a.kind, AbilityKind::Triggered(_))));
        }
    }
}
#[test]
fn milled_card_copy_preserves_conditional_source_text() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Shadow Kin")
        .card_types(vec![CardType::Creature]).parse_text(TEXT).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), TEXT);
}

#[test]
fn death_triggered_paid_copy_uses_the_dead_creatures_last_known_values() {
    let text = "Whenever a creature dies, you may pay {1}. If you do, this creature becomes a copy of that creature, except it has this ability.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cemetery Puca")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(1, 2))
        .parse_text(text).unwrap();
    let trigger = definition.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Triggered(t) => Some(t), _ => None }).unwrap();
    for (accept, mana) in [(false, 1), (true, 0), (true, 1)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let victim = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Dead creature")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(5, 6)).build();
        let victim = game.create_object_from_card(&victim, bob, Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(victim).unwrap(), &game);
        let graveyard_id = game.move_object_by_effect(victim, Zone::Graveyard).unwrap();
        game.object_mut(graveyard_id).unwrap().name = "Later graveyard instance".into();
        game.player_mut(alice).unwrap().mana_pool.add(crate::mana::ManaSymbol::Colorless, mana);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::ZoneChangeEvent::with_cause(victim, Zone::Battlefield, Zone::Graveyard,
                crate::events::cause::EventCause::effect(), Some(snapshot)),
            crate::provenance::ProvNodeId::default());
        let mut dm = CopyChoice(accept);
        let mut ctx = crate::effects::EffectContext::new(source, alice, &mut dm)
            .with_triggering_event(event).with_trigger_identity(crate::triggers::compute_trigger_identity(trigger));
        ctx.source_snapshot = Some(crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game));
        for effect in &trigger.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx)
                .unwrap_or_else(|error| panic!("accept={accept}, mana={mana}: {error:?}"));
        }
        game.refresh_continuous_state();
        assert_eq!(game.calculated_power(source), Some(if accept && mana > 0 { 5 } else { 1 }));
        if accept && mana > 0 { assert_eq!(game.current_name(source).as_deref(), Some("Dead creature")); }
        assert!(game.current_abilities(source).unwrap().iter().any(|a| matches!(a.kind, AbilityKind::Triggered(_))));
    }
}
