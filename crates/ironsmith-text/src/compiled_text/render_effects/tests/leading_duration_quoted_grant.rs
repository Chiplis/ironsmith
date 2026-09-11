use super::*;

#[test]
fn leading_duration_covers_both_pump_and_quoted_death_trigger() {
    for amount in [2, 3] {
        let oracle = format!(
            "When this creature enters, until end of turn, another target creature you control gets +{amount}/+0 and gains \"When this creature dies, return it to the battlefield tapped under its owner's control.\""
        );
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Temporary Grant Probe")
                .card_types(vec![CardType::Creature])
                .parse_text(&oracle)
                .unwrap();
        let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind
        else {
            panic!("entry trigger")
        };
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 2))
            .build();
        let target = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let other = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let entry = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::ZoneChangeEvent::with_cause(
                source,
                Zone::Stack,
                Zone::Battlefield,
                crate::events::cause::EventCause::effect(),
                Some(crate::snapshot::ObjectSnapshot::from_object(
                    game.object(source).unwrap(),
                    &game,
                )),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_triggering_event(entry)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        ctx.snapshot_targets(&game);
        for segment in &triggered.effects.segments {
            for effect in &segment.default_effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        assert_eq!(game.current_power(target), Some(1 + amount));
        assert_eq!(game.current_power(other), Some(1));
        let abilities = game.current_abilities(target).unwrap();
        assert_eq!(abilities.len(), 1);
        let crate::ability::AbilityKind::Triggered(granted) = &abilities[0].kind else {
            panic!("granted death trigger")
        };
        assert!(format!("{granted:#?}").contains("tapped: true"));
        assert!(game.current_abilities(other).unwrap().is_empty());
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state();
        assert_eq!(game.current_power(target), Some(1));
        assert!(game.current_abilities(target).unwrap().is_empty());
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [oracle]
        );
    }
}
