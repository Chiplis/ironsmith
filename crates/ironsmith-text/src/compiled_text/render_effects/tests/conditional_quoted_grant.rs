use super::*;

struct SacrificeDecision(bool);
impl crate::decision::DecisionMaker for SacrificeDecision {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.0
    }
}

#[test]
fn conditional_quoted_grant_depends_on_defending_players_actual_sacrifice() {
    let oracle = "Whenever this creature attacks, it gains \"this creature can't be blocked\" until end of turn unless defending player sacrifices a creature of their choice.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Conditional Grant Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(3, 1))
            .parse_text(oracle)
            .unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("\" until end of turn unless defending player sacrifices a creature"),
        "{rendered}"
    );
    let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected attack trigger")
    };
    for accepts in [false, true] {
        for available in [false, true] {
            let mut game = crate::game_state::GameState::new(
                vec!["Alice".into(), "Bob".into(), "Carol".into()],
                20,
            );
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let carol = game.players[2].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let creature =
                crate::card::CardBuilder::new(crate::ids::CardId::new(), "Payment Creature")
                    .card_types(vec![CardType::Creature])
                    .build();
            let other = game.create_object_from_card(&creature, carol, Zone::Battlefield);
            if available {
                game.create_object_from_card(&creature, bob, Zone::Battlefield);
            }
            let event = crate::triggers::TriggerEvent::new(
                crate::events::combat::CreatureAttackedEvent::new(
                    source,
                    crate::triggers::event::AttackEventTarget::Player(bob),
                ),
                crate::provenance::ProvNodeId::default(),
            );
            let mut decision = SacrificeDecision(accepts);
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_decision_maker(&mut decision)
                .with_defending_player(bob)
                .with_triggering_event(event);
            for segment in &triggered.effects.segments {
                for effect in &segment.default_effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            let paid = accepts && available;
            assert_eq!(game.player(bob).unwrap().graveyard.len(), usize::from(paid));
            assert_eq!(
                game.can_be_blocked(source),
                paid,
                "accepts={accepts}, available={available}"
            );
            assert_eq!(game.object(other).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            game.effect_store.continuous_effects.cleanup_end_of_turn();
            game.refresh_continuous_state();
            assert!(game.can_be_blocked(source));
        }
    }
}
