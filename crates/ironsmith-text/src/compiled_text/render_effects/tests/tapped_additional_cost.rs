use super::*;
const TEXT: &str = "As an additional cost to cast this spell, tap an untapped creature you control. Exile target tapped creature. Put a +1/+1 counter on the creature tapped to pay this spell's additional cost.";

#[test]
fn swallow_whole_cast_preserves_paid_creature() {
    for scenario in 0..5 {
        let invalidate_target = scenario == 1;
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let payer = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let victim = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        game.tap(victim);
        if scenario == 4 {
            game.tap(payer);
        }
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Swallow Whole")
                .card_types(vec![CardType::Sorcery])
                .parse_text(TEXT)
                .unwrap();
        let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let snapshot =
            crate::snapshot::ObjectSnapshot::from_object(game.object(spell).unwrap(), &game);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = crate::effects::EffectContext::new(spell, alice, &mut dm);
        ctx.set_tagged_objects("spell", vec![snapshot]);
        let effect = Effect::new(
            crate::effects::CastTaggedEffect::new("spell", PlayerFilter::You)
                .without_paying_mana_cost(),
        );
        let cast_result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
        if scenario == 4 {
            assert!(cast_result.is_err(), "the unpaid mandatory cost must reject casting");
            assert!(
                game.stack.is_empty(),
                "an already tapped creature cannot pay the cost"
            );
            assert_eq!(game.object(spell).unwrap().zone, Zone::Hand);
            assert_eq!(game.calculated_power(payer), Some(2));
            continue;
        }
        cast_result.unwrap();
        assert_eq!(game.stack.len(), 1, "the spell must actually be cast");
        assert!(game.is_tapped(payer), "additional tap cost must be paid");
        assert_eq!(game.stack[0].targets, vec![crate::Target::Object(victim)]);
        if invalidate_target {
            game.untap(victim);
        }
        let final_payer = if scenario == 2 {
            let graveyard = game.move_object_by_effect(payer, Zone::Graveyard).unwrap();
            game.move_object_by_effect(graveyard, Zone::Battlefield)
                .unwrap()
        } else {
            if scenario == 3 {
                game.set_current_controller(payer, bob);
                game.untap(payer);
            }
            payer
        };
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            game.calculated_power(final_payer),
            Some(if invalidate_target || scenario == 2 {
                2
            } else {
                3
            }),
            "scenario={scenario}: cost reference must retain object identity, independent of current tap state or controller"
        );
        assert_eq!(
            game.object(victim).map(|o| o.zone),
            if invalidate_target {
                Some(Zone::Battlefield)
            } else {
                None
            }
        );
        assert_eq!(game.exile.len(), usize::from(!invalidate_target));
    }
}

#[test]
fn swallow_whole_renders_explicit_cost_reference() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Swallow Whole")
        .card_types(vec![CardType::Sorcery])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        TEXT
    );
}
