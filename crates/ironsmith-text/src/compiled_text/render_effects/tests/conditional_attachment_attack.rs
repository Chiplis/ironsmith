use super::*;

#[test]
fn attachment_condition_grants_defender_permission_only_to_its_source() {
    let oracle = "Defender\nAs long as this creature is enchanted or equipped, it can attack as though it didn't have defender.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Conditional Attack Probe")
        .card_types(vec![CardType::Creature]).parse_text(oracle).unwrap();
    assert!(definition.spell_effect.is_none(), "an intrinsic static condition must not become a resolving attachment effect");
    for equipment in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let first = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let second = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let attachment = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Attachment Probe")
            .card_types(vec![if equipment { CardType::Artifact } else { CardType::Enchantment }])
            .subtypes(vec![if equipment { Subtype::Equipment } else { Subtype::Aura }]).build();
        let attached = game.create_object_from_card(&attachment, if equipment { alice } else { bob }, Zone::Battlefield);
        for destination in [None, Some(first), Some(second), None] {
            if let Some(target) = destination {
                assert!(game.attach_object_to_target(attached, crate::object::AttachmentTarget::Object(target)));
            } else if game.object(attached).unwrap().attached_to.is_some() {
                assert!(game.detach_object_from_current_target(attached));
            }
            game.refresh_continuous_state();
            for source in [first, second] {
                assert!(game.current_has_static_ability_id(source, crate::static_abilities::StaticAbilityId::Defender));
                assert_eq!(game.current_has_static_ability_id(source,
                    crate::static_abilities::StaticAbilityId::CanAttackAsThoughNoDefender), destination == Some(source));
            }
        }
    }
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), oracle);
}
