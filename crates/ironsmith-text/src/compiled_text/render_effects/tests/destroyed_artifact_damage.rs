use super::*;
const TEXT: &str = "{R}, {T}: Destroy target artifact. That artifact deals damage equal to its mana value to this creature.";
#[test]
fn destroyed_artifact_remains_the_damage_source() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Goblin Tinkerer")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 2))
            .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::Generic(1),
                crate::mana::ManaSymbol::Red,
            ]))
            .parse_text(TEXT)
            .unwrap();
    let ability = definition
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    for indestructible in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut artifact =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Damaging Artifact")
                .card_types(vec![CardType::Artifact])
                .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                    crate::mana::ManaSymbol::Generic(3),
                ]))
                .with_ability(Ability::static_ability(
                    crate::static_abilities::StaticAbility::lifelink(),
                ));
        if indestructible {
            artifact = artifact.with_ability(Ability::static_ability(
                crate::static_abilities::StaticAbility::indestructible(),
            ));
        }
        let target = game.create_object_from_definition(&artifact.build(), bob, Zone::Battlefield);
        let stable = game.object(target).unwrap().stable_id;
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        ctx.snapshot_targets(&game);
        for effect in &ability.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            let current = game.find_object_by_stable_id(stable).unwrap();
            if game.object(current).unwrap().zone == Zone::Graveyard {
                // The next instruction uses battlefield LKI, even if the new
                // graveyard object no longer has the damage-source ability.
                game.object_mut(current).unwrap().abilities = std::sync::Arc::new(vec![]);
            }
        }
        assert_eq!(
            game.damage_on(source),
            3,
            "artifact mana value, indestructible={indestructible}"
        );
        assert_eq!(
            game.player(bob).unwrap().life,
            23,
            "artifact's lifelink identifies the damage source, indestructible={indestructible}"
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        let final_id = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(final_id).unwrap().zone,
            if indestructible {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
    }
}
#[test]
fn destroyed_artifact_damage_preserves_both_object_references() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Goblin Tinkerer")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}
