use super::*;
const TEXT: &str = "−7: Chandra deals 6 damage to each opponent. Each player dealt damage this way gets an emblem with \"At the beginning of your upkeep, this emblem deals 3 damage to you.\"";
#[test]
fn damage_emblem_recipients_excludes_controller_and_prevented_damage() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Chandra, Roaring Flame")
            .supertypes(vec![Supertype::Legendary]).card_types(vec![CardType::Planeswalker])
            .parse_text(TEXT)
            .unwrap();
    let ability = definition
        .abilities
        .iter()
        .find_map(|a| {
            if let AbilityKind::Activated(a) = &a.kind {
                Some(a)
            } else {
                None
            }
        })
        .unwrap();
    for prevented in [0, 3, 6] {
        let prevent_bob = prevented == 6;
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let carol = game.players[2].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        // Damage from an earlier action must not award an emblem to Alice.
        crate::effects::execute_effect(&mut game, &Effect::deal_damage(1, ChooseSpec::SpecificPlayer(alice)), &mut ctx).unwrap();
        if prevented > 0 {
            let prevent = Effect::new(crate::effects::PreventDamageEffect::new(
                prevented,
                ChooseSpec::SpecificPlayer(bob),
                Until::EndOfTurn,
            ));
            crate::effects::execute_effect(&mut game, &prevent, &mut ctx).unwrap();
        }
        for segment in &ability.effects.segments {
            for effect in &segment.default_effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        drop(ctx);
        assert_eq!(game.player(alice).unwrap().life, 19);
        assert_eq!(
            game.player(bob).unwrap().life,
            14 + prevented
        );
        assert_eq!(game.player(carol).unwrap().life, 14);
        let mut owners = game
            .command_zone
            .iter()
            .filter_map(|id| {
                let obj = game.object(*id)?;
                matches!(obj.kind, crate::object::ObjectKind::Emblem).then_some(obj.owner)
            })
            .collect::<Vec<_>>();
        owners.sort();
        let mut expected = if prevent_bob {
            vec![carol]
        } else {
            vec![bob, carol]
        };
        expected.sort();
        assert_eq!(owners, expected, "prevent_bob={prevent_bob}");
        for player in [alice, bob, carol] {
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::BeginningOfUpkeepEvent::new(player),
                crate::provenance::ProvNodeId::default(),
            );
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(triggers.len(), usize::from(expected.contains(&player)));
            for trigger in triggers {
                let mut ctx = crate::effects::EffectContext::new_default(trigger.source, player)
                    .with_triggering_event(event.clone());
                for segment in &trigger.ability.effects.segments {
                    for effect in &segment.default_effects {
                        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    }
                }
            }
        }
        assert_eq!(game.player(alice).unwrap().life, 19);
        assert_eq!(
            game.player(bob).unwrap().life,
            if prevent_bob { 20 } else { 11 + prevented }
        );
        assert_eq!(game.player(carol).unwrap().life, 11);
    }
}

#[test]
fn damage_emblem_recipients_renders_damage_condition_and_one_quote_pair() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Chandra, Roaring Flame")
            .supertypes(vec![Supertype::Legendary]).card_types(vec![CardType::Planeswalker])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}
