use super::*;
const TEXT: &str = "Whenever you cast a red spell, untap Chandra.\n{T}: Chandra deals 1 damage to target player or planeswalker. If Chandra has dealt 3 or more damage this turn, exile her, then return her to the battlefield transformed under her owner's control.";
#[test]
fn source_damage_transform_counts_damage_and_returns_to_owner() {
    let front_id = crate::ids::CardId::new();
    let back_id = crate::ids::CardId::new();
    let mut front = crate::CardDefinitionBuilder::new(front_id, "Chandra, Fire of Kaladesh")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .parse_text(TEXT)
        .unwrap();
    let mut back = crate::CardDefinitionBuilder::new(back_id, "Chandra, Roaring Flame")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build();
    front.card.other_face = Some(back_id);
    front.card.other_face_name = Some(back.card.name.clone());
    front.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;
    back.card.other_face = Some(front_id);
    back.card.other_face_name = Some(front.card.name.clone());
    back.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;
    let ability = front
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
    for prior in [1, 2] {
        for prevent_final in [false, true] {
            let mut game = crate::game_state::GameState::new(
                vec!["Alice".into(), "Bob".into(), "Carol".into()],
                20,
            );
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let carol = game.players[2].id;
            game.create_object_from_definition(&back, alice, Zone::Exile);
            let source = game.create_object_from_definition(&front, alice, Zone::Battlefield);
            game.set_current_controller(source, bob);
            let mut ctx = crate::effects::EffectContext::new_default(source, bob);
            crate::effects::execute_effect(
                &mut game,
                &Effect::deal_damage(prior, ChooseSpec::SpecificPlayer(carol)),
                &mut ctx,
            )
            .unwrap();
            if prevent_final {
                crate::effects::execute_effect(
                    &mut game,
                    &Effect::new(crate::effects::PreventAllDamageToTargetEffect::new(
                        ChooseSpec::SpecificPlayer(carol),
                        Until::EndOfTurn,
                    )),
                    &mut ctx,
                )
                .unwrap();
            }
            drop(ctx);
            let mut ctx = crate::effects::EffectContext::new_default(source, bob)
                .with_targets(vec![crate::effects::ResolvedTarget::Player(carol)]);
            for segment in &ability.effects.segments {
                for effect in &segment.default_effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            drop(ctx);
            let transforms = prior == 2 && !prevent_final;
            let returned = game
                .battlefield
                .iter()
                .find(|id| game.object(**id).unwrap().name == "Chandra, Roaring Flame");
            assert_eq!(
                returned.is_some(),
                transforms,
                "prior={prior}, prevented={prevent_final}"
            );
            if let Some(&id) = returned {
                assert_eq!(game.controller_of_id(id), Some(alice));
                assert_ne!(id, source);
            } else {
                assert!(game.battlefield.contains(&source));
            }
            assert_eq!(
                game.player(carol).unwrap().life,
                20 - prior - i32::from(!prevent_final)
            );
        }
    }
}
#[test]
fn source_damage_transform_preserves_names_and_pronouns() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Chandra, Fire of Kaladesh")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn source_damage_transform_untaps_only_for_its_controllers_red_spell() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Chandra, Fire of Kaladesh")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .parse_text(TEXT)
            .unwrap();
    for own_spell in [false, true] {
        for red in [false, true] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let other = game.create_object_from_card(
                &crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other Creature")
                    .card_types(vec![CardType::Creature])
                    .build(),
                alice,
                Zone::Battlefield,
            );
            game.tap(source);
            game.tap(other);
            let caster = if own_spell { alice } else { bob };
            let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Spell Probe")
                .card_types(vec![CardType::Instant])
                .mana_cost(crate::mana::ManaCost::from_symbols(vec![if red {
                    crate::mana::ManaSymbol::Red
                } else {
                    crate::mana::ManaSymbol::Blue
                }]))
                .build();
            let spell = game.create_object_from_card(&card, caster, Zone::Stack);
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::SpellCastEvent::new(spell, caster, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            );
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(triggers.len(), usize::from(own_spell && red));
            for trigger in triggers {
                let mut ctx = crate::effects::EffectContext::new_default(trigger.source, alice)
                    .with_triggering_event(event.clone());
                for effect in &trigger.ability.effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            assert_eq!(game.is_tapped(source), !(own_spell && red));
            assert!(game.is_tapped(other));
        }
    }
}
