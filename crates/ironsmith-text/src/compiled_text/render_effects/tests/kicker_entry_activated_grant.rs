use super::*;
const ORACLE: &str = "Kicker {1}{U} and/or {B}\nIf this creature was kicked with its {1}{U} kicker, it enters with two +1/+1 counters on it and with flying.\nIf this creature was kicked with its {B} kicker, it enters with a +1/+1 counter on it and with \"Pay 3 life: Regenerate this creature.\"";
#[test]
fn kicker_entry_activated_grant_keeps_each_payment_independent() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Anavolver")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .parse_text(ORACLE)
        .unwrap();
    assert!(definition.spell_effect.is_none());
    assert_eq!(definition.optional_costs.len(), 2);
    for mask in 0..4 {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut paid = crate::cost::OptionalCostsPaid::from_costs(&definition.optional_costs);
        for index in 0..2 {
            if mask & (1 << index) != 0 {
                paid.pay(index);
            }
        }
        game.object_mut(source).unwrap().optional_costs_paid = paid;
        let entered = game
            .move_object_with_etb_processing(source, Zone::Battlefield)
            .unwrap()
            .new_id;
        let blue = mask & 1 != 0;
        let black = mask & 2 != 0;
        let counters = u32::from(blue) * 2 + u32::from(black);
        assert_eq!(
            game.object(entered)
                .unwrap()
                .counters
                .get(&crate::object::CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            counters,
            "mask={mask}"
        );
        assert_eq!(
            game.current_has_static_ability_id(
                entered,
                crate::static_abilities::StaticAbilityId::Flying
            ),
            blue
        );
        let abilities = game.current_abilities(entered).unwrap();
        let activated = abilities
            .iter()
            .filter_map(|a| {
                if let AbilityKind::Activated(a) = &a.kind {
                    Some(a)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(activated.len(), usize::from(black));
        assert_eq!(
            game.player(alice).unwrap().life,
            20,
            "entry must not pay the activation cost"
        );
        if black {
            assert_eq!(activated[0].mana_cost.display(), "Pay 3 life");
            let mut ctx = crate::effects::EffectContext::new_default(entered, alice);
            for segment in &activated[0].effects.segments {
                for effect in &segment.default_effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            crate::effects::execute_effect(
                &mut game,
                &Effect::destroy(ChooseSpec::SpecificObject(entered)),
                &mut ctx,
            )
            .unwrap();
            assert!(
                game.object(entered).is_some(),
                "granted regeneration replaces destruction"
            );
            assert!(game.is_tapped(entered));
        }
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state();
        assert_eq!(
            game.current_has_static_ability_id(
                entered,
                crate::static_abilities::StaticAbilityId::Flying
            ),
            blue,
            "entry grants persist"
        );
        assert_eq!(
            game.current_abilities(entered)
                .unwrap()
                .iter()
                .filter(|a| matches!(&a.kind, AbilityKind::Activated(_)))
                .count(),
            usize::from(black)
        );
        let mut ctx = crate::effects::EffectContext::new_default(entered, alice);
        crate::effects::execute_effect(
            &mut game,
            &Effect::destroy(ChooseSpec::SpecificObject(entered)),
            &mut ctx,
        )
        .unwrap();
        assert!(
            game.object(entered).is_none(),
            "regeneration shield protects against only one destruction"
        );
    }
}

#[test]
fn kicker_entry_activated_grant_renders_discriminated_cost_and_ability() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Anavolver")
        .card_types(vec![CardType::Creature])
        .parse_text(ORACLE)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        ORACLE
    );
}
