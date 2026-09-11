use super::*;
const ORACLE: &str = "Flying, deathtouch\nWhenever Scriv enters or attacks, create a white Aura enchantment token named Contract attached to target creature an opponent controls. The token has enchant creature and \"Whenever enchanted creature attacks, it gets +2/+0 until end of turn if it's attacking one of your opponents. Otherwise, its controller loses 2 life.\"";

#[test]
fn attached_token_attack_contract_preserves_creation_and_both_branches() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Scriv, the Obligator")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 3))
            .parse_text(ORACLE)
            .unwrap();
    for entry_trigger in [false, true] {
        for destination in 0..3 {
            let mut game = crate::game_state::GameState::new(
                vec!["Alice".into(), "Bob".into(), "Carol".into()],
                20,
            );
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let carol = game.players[2].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Recipient")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(1, 3))
                .build();
            let target = game.create_object_from_card(&creature, bob, Zone::Battlefield);
            let other = game.create_object_from_card(&creature, carol, Zone::Battlefield);
            let event = if entry_trigger {
                crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::ZoneChangeEvent::with_cause(
                        source,
                        Zone::Stack,
                        Zone::Battlefield,
                        crate::events::cause::EventCause::effect(),
                        Some(crate::snapshot::ObjectSnapshot::from_object(
                            game.object(source).unwrap(),
                            &game,
                        )),
                    ),
                    crate::provenance::ProvNodeId::default(),
                )
            } else {
                crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::CreatureAttackedEvent::new(
                        source,
                        crate::events::AttackEventTarget::Player(bob),
                    ),
                    crate::provenance::ProvNodeId::default(),
                )
            };
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(triggers.len(), 1, "outer entry/attack trigger");
            let entry = &triggers[0];
            assert_eq!(entry.source, source);
            let requirements =
                crate::game_loop::extract_target_requirements_from_program_with_modes(
                    &game,
                    &entry.ability.effects,
                    alice,
                    Some(source),
                    None,
                );
            assert_eq!(requirements.len(), 1);
            assert!(
                requirements[0]
                    .legal_targets
                    .contains(&crate::Target::Object(target))
            );
            assert!(
                !requirements[0]
                    .legal_targets
                    .contains(&crate::Target::Object(source))
            );
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_triggering_event(event)
                .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
            ctx.snapshot_targets(&game);
            for segment in &entry.ability.effects.segments {
                for effect in &segment.default_effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            let aura = *game
                .battlefield
                .iter()
                .find(|id| game.object(**id).unwrap().name == "Contract")
                .expect("create the Contract token");
            let token = game.object(aura).unwrap();
            assert_eq!(token.kind, crate::object::ObjectKind::Token);
            assert_eq!(token.card_types.as_slice(), &[CardType::Enchantment]);
            assert!(token.subtypes.contains(&Subtype::Aura));
            assert_eq!(
                game.current_colors(aura),
                Some(crate::color::ColorSet::WHITE)
            );
            assert_eq!(game.controller_of_id(aura), Some(alice));
            assert_eq!(
                token.attached_to,
                Some(crate::object::AttachmentTarget::Object(target))
            );
            assert!(
                matches!(token.aura_attach_filter_owned(), Some(crate::object::AuraAttachmentFilter::Object(filter)) if filter.card_types == [CardType::Creature])
            );
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            let (attack_target, event_target) = match destination {
                0 => (
                    crate::combat_state::AttackTarget::Player(alice),
                    crate::events::AttackEventTarget::Player(alice),
                ),
                1 => (
                    crate::combat_state::AttackTarget::Player(carol),
                    crate::events::AttackEventTarget::Player(carol),
                ),
                _ => {
                    let walker = crate::card::CardBuilder::new(
                        crate::ids::CardId::new(),
                        "Defending walker",
                    )
                    .card_types(vec![CardType::Planeswalker])
                    .build();
                    let id = game.create_object_from_card(&walker, carol, Zone::Battlefield);
                    (
                        crate::combat_state::AttackTarget::Planeswalker(id),
                        crate::events::AttackEventTarget::Planeswalker(id),
                    )
                }
            };
            let mut combat = crate::combat_state::CombatState::default();
            combat.attackers.push(crate::combat_state::AttackerInfo {
                creature: target,
                target: attack_target,
            });
            game.combat = Some(combat);
            let unrelated = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::CreatureAttackedEvent::new(other, event_target.clone()),
                crate::provenance::ProvNodeId::default(),
            );
            assert!(crate::triggers::check_triggers(&game, &unrelated).is_empty());
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::CreatureAttackedEvent::new(target, event_target),
                crate::provenance::ProvNodeId::default(),
            );
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(triggers.len(), 1);
            let entry = &triggers[0];
            assert_eq!(entry.source, aura);
            let mut ctx = crate::effects::EffectContext::new_default(aura, alice)
                .with_triggering_event(event);
            for segment in &entry.ability.effects.segments {
                for effect in &segment.default_effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            assert_eq!(
                game.current_power(target),
                Some(if destination == 1 { 3 } else { 1 }),
                "entry={entry_trigger} destination={destination}"
            );
            assert_eq!(
                game.player(bob).unwrap().life,
                if destination == 1 { 20 } else { 18 }
            );
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(carol).unwrap().life, 20);
            assert_eq!(game.current_power(other), Some(1));
            game.effect_store.continuous_effects.cleanup_end_of_turn();
            game.refresh_continuous_state();
            assert_eq!(game.current_power(target), Some(1));
        }
    }
}

#[test]
fn attached_token_attack_contract_renders_outer_creation_and_token_rules() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Scriv, the Obligator")
            .card_types(vec![CardType::Creature])
            .parse_text(ORACLE)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        ORACLE
    );
}
