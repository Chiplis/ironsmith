use super::*;
const TEXT: &str = "{W}{U}{B}{R}{G}: This creature gets +2/+2 and gains fear until end of turn. Target creature gets -2/-2 until end of turn. Activate only during your turn.";
#[test]
fn source_pump_keyword_target_debuff_keeps_objects_and_duration() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Fleshformer")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .parse_text(TEXT)
        .unwrap();
    let AbilityKind::Activated(ability) = &definition.abilities[0].kind else {
        panic!("activated")
    };
    for self_target in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let other = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let target = if self_target { source } else { other };
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        for effect in &ability.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        game.refresh_continuous_state();
        assert_eq!(
            game.current_power(source),
            Some(if self_target { 2 } else { 4 })
        );
        assert_eq!(
            game.current_toughness(source),
            Some(if self_target { 2 } else { 4 })
        );
        assert_eq!(
            game.current_power(other),
            Some(if self_target { 2 } else { 0 })
        );
        assert!(
            game.current_has_static_ability_id(
                source,
                crate::static_abilities::StaticAbilityId::Fear
            )
        );
        assert!(
            !game.current_has_static_ability_id(
                other,
                crate::static_abilities::StaticAbilityId::Fear
            )
        );
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state();
        for id in [source, other] {
            assert_eq!(game.current_power(id), Some(2));
            assert_eq!(game.current_toughness(id), Some(2));
            assert!(
                !game.current_has_static_ability_id(
                    id,
                    crate::static_abilities::StaticAbilityId::Fear
                )
            );
        }
    }
}
#[test]
fn source_pump_keyword_target_debuff_compacts_shared_duration() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Fleshformer")
        .card_types(vec![CardType::Creature])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}
