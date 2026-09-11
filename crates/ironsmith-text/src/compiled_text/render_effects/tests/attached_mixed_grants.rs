use super::*;

#[test]
fn attached_mixed_keyword_and_blocking_grants_follow_current_attachment() {
    for equipment in [false, true] {
        let oracle = if equipment {
            "Equipped creature has hexproof and can't be blocked.\nEquip—{2}, Pay 2 life."
        } else {
            "Enchant creature\nEnchanted creature has hexproof and can't be blocked."
        };
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Attached Mixed Grant Probe",
        )
        .card_types(vec![if equipment {
            CardType::Artifact
        } else {
            CardType::Enchantment
        }])
        .subtypes(vec![if equipment {
            Subtype::Equipment
        } else {
            Subtype::Aura
        }])
        .parse_text(oracle)
        .unwrap();
        if equipment {
            assert!(
                definition.spell_effect.is_none(),
                "equipment bonuses must be static: {definition:#?}"
            );
        }
        let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(
            rendered.contains("hexproof") && rendered.contains("can't be blocked"),
            "{rendered}"
        );
        if equipment {
            let raw = definition
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    crate::ability::AbilityKind::Static(value) => Some(value.display()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(
                rendered.contains("Equipped creature has hexproof and can't be blocked."),
                "{rendered}\nraw static displays: {raw:?}"
            );
        }
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Grant Recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let first = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let second = game.create_object_from_card(&creature, bob, Zone::Battlefield);
        for attached in [Some(first), Some(second), None] {
            if let Some(target) = attached {
                assert!(game.attach_object_to_target(
                    source,
                    crate::object::AttachmentTarget::Object(target)
                ));
            } else {
                assert!(game.detach_object_from_current_target(source));
            }
            game.refresh_continuous_state();
            for recipient in [first, second] {
                let granted = attached == Some(recipient);
                assert_eq!(
                    game.current_has_static_ability_id(
                        recipient,
                        crate::static_abilities::StaticAbilityId::Hexproof
                    ),
                    granted
                );
                assert_eq!(game.can_be_blocked(recipient), !granted);
            }
            assert!(!game.current_has_static_ability_id(
                source,
                crate::static_abilities::StaticAbilityId::Hexproof
            ));
        }
    }
}
