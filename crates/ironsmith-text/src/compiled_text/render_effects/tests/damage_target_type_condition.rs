use super::*;

const TEXT: &str = "Searing Barb deals 2 damage to any target. If it's a creature, it can't block this turn. Incubate 1.";

#[test]
fn damage_target_type_condition_survives_prevention() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Searing Barb")
        .card_types(vec![CardType::Sorcery])
        .parse_text(TEXT)
        .unwrap();
    for player_target in [false, true] {
        for prevented in [false, true] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
            let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(3, 5))
                .build();
            let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
            let outsider = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
            let attacker = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
            let resolved = if player_target {
                crate::effects::ResolvedTarget::Player(bob)
            } else {
                crate::effects::ResolvedTarget::Object(target)
            };
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_targets(vec![resolved]);
            ctx.snapshot_targets(&game);
            if prevented {
                let prevention = Effect::new(crate::effects::PreventAllDamageToTargetEffect::new(
                    if player_target {
                        ChooseSpec::SpecificPlayer(bob)
                    } else {
                        ChooseSpec::SpecificObject(target)
                    },
                    Until::EndOfTurn,
                ));
                crate::effects::execute_effect(&mut game, &prevention, &mut ctx).unwrap();
            }
            for effect in definition.spell_effect.as_ref().unwrap() {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            game.update_cant_effects();
            assert_eq!(
                game.damage_on(target),
                if !player_target && !prevented { 2 } else { 0 }
            );
            assert_eq!(
                game.player(bob).unwrap().life,
                if player_target && !prevented { 18 } else { 20 }
            );
            assert_eq!(
                game.can_block_attacker(target, attacker),
                player_target,
                "player={player_target} prevented={prevented}"
            );
            assert!(game.can_block_attacker(outsider, attacker));
            let incubators: Vec<_> = game
                .battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|o| o.name == "Incubator")
                .collect();
            assert_eq!(incubators.len(), 1);
            assert_eq!(game.controller_of(incubators[0]), alice);
            assert_eq!(
                incubators[0]
                    .counters
                    .get(&crate::CounterType::PlusOnePlusOne)
                    .copied(),
                Some(1)
            );
            game.cleanup_restrictions_end_of_turn();
            game.update_cant_effects();
            assert!(game.can_block_attacker(target, attacker));
        }
    }
}

#[test]
fn damage_target_type_condition_renders_target_identity() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Searing Barb")
        .card_types(vec![CardType::Sorcery])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        TEXT
    );
}
