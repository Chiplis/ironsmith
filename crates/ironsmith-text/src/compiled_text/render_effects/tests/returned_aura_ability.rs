use super::*;
const TEXT: &str = "{G}{W}: This creature gains indestructible until end of turn.\nWhen this creature dies, return it to the battlefield. It's an Aura enchantment with enchant creature you control and \"{G}{W}: Enchanted creature gains indestructible until end of turn,\" and it loses all other abilities.";
#[test]
fn returned_aura_ability_retains_atomic_return_and_grant() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Bronzehide Lion")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Cat])
            .power_toughness(crate::card::PowerToughness::fixed(3, 3))
            .parse_text(TEXT)
            .unwrap();
    let debug = format!("{definition:#?}");
    assert!(
        !debug.contains("as_aura: None"),
        "Aura return payload missing: {debug}"
    );
    assert!(
        debug.contains("AddAbilityGeneric"),
        "Aura grant missing: {debug}"
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn returned_aura_ability_belongs_to_aura_and_protects_attachment() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Bronzehide Lion")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Cat])
            .power_toughness(crate::card::PowerToughness::fixed(3, 3))
            .parse_text(TEXT)
            .unwrap();
    for (entry_text, enters_tapped) in [
        ("Creatures enter the battlefield tapped.", false),
        ("Enchantments enter the battlefield tapped.", true),
    ] {
        for host_available in [true, false] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let host_def = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Host")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .parse_text("Vigilance")
                .unwrap();
            let host = host_available
                .then(|| game.create_object_from_definition(&host_def, alice, Zone::Battlefield));
            let other = game.create_object_from_definition(&host_def, bob, Zone::Battlefield);
            let entry_rule = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Entry Rule")
                .card_types(vec![CardType::Enchantment]).build();
            let rule = game.create_object_from_card(&entry_rule, bob, Zone::Battlefield);
            let filter = if enters_tapped { ObjectFilter::enchantment() } else { ObjectFilter::creature() };
            game.object_mut(rule).unwrap().abilities_mut().push(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::enters_tapped_for_filter(filter)));
            game.refresh_continuous_state();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let snapshot =
                crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::ZoneChangeEvent::with_cause(
                    source,
                    Zone::Battlefield,
                    Zone::Graveyard,
                    crate::events::cause::EventCause::effect(),
                    Some(snapshot),
                ),
                crate::provenance::ProvNodeId::default(),
            );
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(triggers.len(), 1);
            let grave = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_triggering_event(event);
            for segment in &triggers[0].ability.effects.segments {
                for effect in &segment.default_effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(
                        |error| {
                            panic!(
                                "host_available={host_available}, effect={:?}, error={error:?}",
                                format!("{effect:?}").chars().take(300).collect::<String>()
                            )
                        },
                    );
                }
            }
            assert!(game.current_has_static_ability_id(
                other,
                crate::static_abilities::StaticAbilityId::Vigilance
            ));
            let returned = game
                .battlefield
                .iter()
                .copied()
                .find(|id| game.object(*id).unwrap().name == "Bronzehide Lion");
            assert_eq!(returned.is_some(), host_available);
            if let Some(host) = host {
                let aura = returned.unwrap();
                assert!(game.current_has_subtype(aura, Subtype::Aura));
                assert_eq!(
                    game.is_tapped(aura),
                    enters_tapped,
                    "replacement must see an Aura enchantment: {entry_text}"
                );
                assert_eq!(
                    game.object(aura).unwrap().attached_to,
                    Some(crate::object::AttachmentTarget::Object(host))
                );
                assert!(game.current_has_static_ability_id(
                    host,
                    crate::static_abilities::StaticAbilityId::Vigilance
                ));
                assert!(
                    game.current_abilities(host)
                        .unwrap()
                        .iter()
                        .all(|a| !matches!(a.kind, crate::ability::AbilityKind::Activated(_)))
                );
                let abilities = game.current_abilities(aura).unwrap();
                let activated: Vec<_> = abilities
                    .iter()
                    .filter_map(|a| match &a.kind {
                        crate::ability::AbilityKind::Activated(a) => Some(a),
                        _ => None,
                    })
                    .collect();
                assert_eq!(activated.len(), 1, "{abilities:#?}");
                assert!(
                    !abilities
                        .iter()
                        .any(|a| matches!(a.kind, crate::ability::AbilityKind::Triggered(_)))
                );
                let mut ctx = crate::effects::EffectContext::new_default(aura, alice);
                for effect in &activated[0].effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
                assert!(game.current_has_static_ability_id(
                    host,
                    crate::static_abilities::StaticAbilityId::Indestructible
                ));
                assert!(!game.current_has_static_ability_id(
                    aura,
                    crate::static_abilities::StaticAbilityId::Indestructible
                ));
                assert!(!game.current_has_static_ability_id(
                    other,
                    crate::static_abilities::StaticAbilityId::Indestructible
                ));
                game.effect_store.continuous_effects.cleanup_end_of_turn();
                game.refresh_continuous_state();
                assert!(!game.current_has_static_ability_id(
                    host,
                    crate::static_abilities::StaticAbilityId::Indestructible
                ));
            } else {
                assert_eq!(game.object(grave).unwrap().zone, Zone::Graveyard);
            }
        }
    }
}
