use super::*;
use crate::card::PowerToughness;
use crate::game_state::{GameState, StackEntry};
use crate::ids::CardId;

#[test]
fn cohort_threshold_damage_replacement_keeps_announced_objects_and_players() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Cinder Cascade")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Cinder Cascade deals 2 damage to each of up to three targets.\nThreshold — Cinder Cascade deals 4 damage to each of those permanents and/or players instead if there are seven or more cards in your graveyard.").unwrap();
    for grave_count in [0, 6, 7, 8] {
        for target_count in 0..=3 {
            for remove_creature in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let (alice, bob) = (game.players[0].id, game.players[1].id);
                let source = game.create_object_from_definition(&card, alice, Zone::Stack);
                let body = crate::CardDefinitionBuilder::new(CardId::new(), "Veralon")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(2, 10))
                    .build();
                let creature = game.create_object_from_definition(&body, bob, Zone::Battlefield);
                let bystander = game.create_object_from_definition(&body, bob, Zone::Battlefield);
                for _ in 0..grave_count {
                    game.create_object_from_definition(&body, alice, Zone::Graveyard);
                }
                let requirements =
                    crate::game_loop::extract_target_requirements_from_program_with_modes(
                        &game,
                        card.spell_effect.as_ref().unwrap(),
                        alice,
                        Some(source),
                        None,
                    );
                assert_eq!(
                    requirements.len(),
                    1,
                    "both damage amounts must share one target declaration"
                );
                let targets = [
                    crate::game_state::Target::Object(creature),
                    crate::game_state::Target::Player(alice),
                    crate::game_state::Target::Player(bob),
                ][..target_count]
                    .to_vec();
                let mut entry = StackEntry::new(source, alice).with_targets(targets);
                entry.target_assignments = vec![crate::game_state::TargetAssignment {
                    spec: requirements[0].spec.clone(),
                    range: 0..target_count,
                }];
                game.stack.push(entry);
                if remove_creature {
                    game.move_object_by_effect(creature, Zone::Exile).unwrap();
                }
                crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                let amount = if grave_count >= 7 { 4 } else { 2 };
                if !remove_creature {
                    assert_eq!(
                        game.damage_on(creature),
                        if target_count >= 1 { amount } else { 0 },
                        "grave={grave_count}, targets={target_count}"
                    );
                }
                assert_eq!(game.damage_on(bystander), 0);
                assert_eq!(
                    game.player(alice).unwrap().life,
                    20 - if target_count >= 2 { amount as i32 } else { 0 }
                );
                assert_eq!(
                    game.player(bob).unwrap().life,
                    20 - if target_count >= 3 { amount as i32 } else { 0 }
                );
            }
        }
    }
}

#[test]
fn cohort_attached_first_strike_checks_current_combat_partner_and_each_combat_untap() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Vesran Blade")
        .card_types(vec![CardType::Artifact]).subtypes(vec![Subtype::Equipment])
        .parse_text("Equipped creature gets +1/+1 and has haste.\nAt the beginning of each combat, untap equipped creature.\nEquipped creature has first strike as long as it's blocking or blocked by a Goblin or Orc.\nEquip {2}").unwrap();
    assert!(crate::compiled_text::compiled_text_lines(&card).join("\n")
        .contains("Equipped creature has first strike as long as it's blocking or blocked by a Goblin or Orc"));
    for host_attacks in [false, true] {
        for host_goblin in [false, true] {
            for partner_type in [Subtype::Goblin, Subtype::Orc, Subtype::Elf] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let (alice, bob) = (game.players[0].id, game.players[1].id);
                let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
                let body = |name: &str, subtype| {
                    crate::CardDefinitionBuilder::new(CardId::new(), name.to_owned())
                        .card_types(vec![CardType::Creature])
                        .subtypes(vec![subtype])
                        .power_toughness(PowerToughness::fixed(2, 2))
                        .build()
                };
                let host = game.create_object_from_definition(
                    &body(
                        "Zeravin",
                        if host_goblin {
                            Subtype::Goblin
                        } else {
                            Subtype::Human
                        },
                    ),
                    alice,
                    Zone::Battlefield,
                );
                let partner = game.create_object_from_definition(
                    &body("Taravin", partner_type),
                    bob,
                    Zone::Battlefield,
                );
                let unrelated = game.create_object_from_definition(
                    &body("Meravin", Subtype::Goblin),
                    bob,
                    Zone::Battlefield,
                );
                game.attach_object_to_target(source, crate::object::AttachmentTarget::Object(host));
                assert_eq!(game.current_power(host), Some(3));
                assert!(game.current_has_static_ability_id(
                    host,
                    crate::static_abilities::StaticAbilityId::Haste
                ));
                assert!(!game.current_has_static_ability_id(
                    host,
                    crate::static_abilities::StaticAbilityId::FirstStrike
                ));
                for active in [alice, bob] {
                    game.tap(host);
                    game.turn.active_player = active;
                    let event = crate::triggers::TriggerEvent::new_with_provenance(
                        crate::events::BeginningOfCombatEvent::new(active),
                        crate::provenance::ProvNodeId::default(),
                    );
                    let triggers = crate::triggers::check_triggers(&game, &event);
                    assert_eq!(triggers.len(), 1);
                    let mut queue = crate::triggers::TriggerQueue::new();
                    for trigger in triggers {
                        queue.add(trigger);
                    }
                    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                    assert!(!game.is_tapped(host));
                }
                let (attacker, blocker, defender) = if host_attacks {
                    (host, partner, bob)
                } else {
                    (partner, host, alice)
                };
                game.combat = Some(crate::combat_state::CombatState {
                    attackers: vec![crate::combat_state::AttackerInfo {
                        creature: attacker,
                        target: crate::combat_state::AttackTarget::Player(defender),
                    }],
                    blockers: std::collections::HashMap::from([(attacker, vec![blocker])]),
                    ..Default::default()
                });
                game.attach_object_to_target(source, crate::object::AttachmentTarget::Object(host));
                assert_eq!(
                    game.current_has_static_ability_id(
                        host,
                        crate::static_abilities::StaticAbilityId::FirstStrike
                    ),
                    partner_type != Subtype::Elf,
                    "host attacks={host_attacks}, host Goblin={host_goblin}, partner={partner_type:?}"
                );
                assert!(!game.current_has_static_ability_id(
                    partner,
                    crate::static_abilities::StaticAbilityId::FirstStrike
                ));
                assert!(!game.current_has_static_ability_id(
                    unrelated,
                    crate::static_abilities::StaticAbilityId::FirstStrike
                ));
                game.move_object_by_effect(partner, Zone::Graveyard)
                    .unwrap();
                assert!(
                    !game.current_has_static_ability_id(
                        host,
                        crate::static_abilities::StaticAbilityId::FirstStrike
                    ),
                    "a departed combat partner must stop satisfying the condition"
                );
                game.combat = None;
                game.attach_object_to_target(source, crate::object::AttachmentTarget::Object(host));
                assert!(!game.current_has_static_ability_id(
                    host,
                    crate::static_abilities::StaticAbilityId::FirstStrike
                ));
            }
        }
    }
}

#[test]
fn cohort_attack_attachment_count_uses_resolution_state_and_departed_source_lki() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Zeravin, Steel Seeker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Whenever Zeravin attacks, draw a card for each Aura and Equipment attached to it.",
        )
        .unwrap();
    for (change, leaves, expected) in [
        (0, false, 2),
        (1, false, 3),
        (2, false, 1),
        (0, true, 2),
        (1, true, 3),
        (2, true, 1),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let blank = crate::CardDefinitionBuilder::new(CardId::new(), "Velron")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let other = game.create_object_from_definition(&blank, alice, Zone::Battlefield);
        for _ in 0..12 {
            game.create_object_from_definition(&blank, alice, Zone::Library);
        }
        let mut attachments = Vec::new();
        for (kind, subtype, controller, host) in [
            (CardType::Enchantment, Subtype::Aura, bob, source),
            (CardType::Artifact, Subtype::Equipment, alice, source),
            (CardType::Artifact, Subtype::Equipment, alice, other),
            (CardType::Enchantment, Subtype::Aura, alice, other),
        ] {
            let attachment = crate::CardDefinitionBuilder::new(CardId::new(), "Teralis")
                .card_types(vec![kind])
                .subtypes(vec![subtype])
                .parse_text(if subtype == Subtype::Aura {
                    "Enchant creature"
                } else {
                    ""
                })
                .unwrap();
            let id = game.create_object_from_definition(&attachment, controller, Zone::Battlefield);
            assert!(
                game.attach_object_to_target(id, crate::object::AttachmentTarget::Object(host))
            );
            attachments.push(id);
        }
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::CreatureAttackedEvent::new(
                source,
                crate::triggers::AttackEventTarget::Player(bob),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        let mut queue = crate::triggers::TriggerQueue::new();
        for trigger in triggers {
            queue.add(trigger);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        if change == 1 {
            game.attach_object_to_target(
                attachments[2],
                crate::object::AttachmentTarget::Object(source),
            );
        }
        if change == 2 {
            game.detach_object_from_current_target(attachments[1]);
        }
        if leaves {
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            let snapshot = game.stack.last().unwrap().source_snapshot.as_ref().unwrap();
            assert_eq!(
                snapshot.attachments.len(),
                expected,
                "departed source attachment ids"
            );
            assert_eq!(
                snapshot.attachment_snapshots.len(),
                expected,
                "departed source attachment snapshots"
            );
            // The Aura also leaves before resolution; its former attachment
            // still contributed when the source last existed on the battlefield.
            game.move_object_by_effect(attachments[0], Zone::Graveyard)
                .unwrap();
        }
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            expected,
            "change={change}, leaves={leaves}"
        );
    }
}

#[test]
fn cohort_source_exile_keeps_counter_bound_for_battlefield_and_graveyard_filters() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Erasure Orb")
        .card_types(vec![CardType::Artifact])
        .parse_text("At the beginning of your upkeep, exile this artifact, all creatures and planeswalkers with mana value less than or equal to the number of void counters on it, and all creature and planeswalker cards in graveyards with mana value less than or equal to the number of void counters on it.").unwrap();
    let ability = card
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            crate::ability::AbilityKind::Triggered(a) => Some(a),
            _ => None,
        })
        .unwrap();
    for (counters, source_left) in [0, 2, 4]
        .into_iter()
        .flat_map(|count| [(count, false), (count, true)])
    {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        game.add_counters(source, crate::CounterType::Void, 5);
        let mut cases = Vec::new();
        for owner in [alice, bob] {
            for zone in [Zone::Battlefield, Zone::Graveyard] {
                for kind in [
                    CardType::Creature,
                    CardType::Planeswalker,
                    CardType::Artifact,
                ] {
                    for mv in [0, 2, 4, 5] {
                        let definition = crate::CardDefinitionBuilder::new(CardId::new(), "Eravon")
                            .card_types(vec![kind])
                            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                                crate::mana::ManaSymbol::Generic(mv),
                            ]]))
                            .power_toughness(PowerToughness::fixed(2, 2))
                            .build();
                        let id = game.create_object_from_definition(&definition, owner, zone);
                        cases.push((id, kind != CardType::Artifact && u32::from(mv) <= counters));
                    }
                }
            }
        }
        let mut entry = StackEntry::ability(source, alice, ability.effects.clone());
        entry.source_snapshot = Some(crate::snapshot::ObjectSnapshot::from_object(
            game.object(source).unwrap(),
            &game,
        ));
        game.stack.push(entry);
        // The limit is determined during resolution, not when the upkeep
        // trigger's initial snapshot was captured.
        game.remove_counters(source, crate::CounterType::Void, 5, None, None);
        game.add_counters(source, crate::CounterType::Void, counters);
        if source_left {
            game.move_object_by_effect(source, Zone::Exile).unwrap();
        }
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.object(source).is_none(), "the source must be exiled");
        for (id, exiled) in cases {
            assert_eq!(
                game.object(id).is_none(),
                exiled,
                "void counters={counters}, source left={source_left}, object={id:?}"
            );
        }
    }
}

#[test]
fn cohort_multiword_source_or_own_subtype_entry_retains_both_trigger_arms() {
    for (name, alias) in [
        ("Marshal Velran of Greyhold", "Marshal Velran"),
        ("Captain Zeravin of the Reach", "Captain Zeravin"),
    ] {
        let text = format!("Whenever {alias} or another Human you control enters, draw a card.");
        let card = crate::CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .power_toughness(PowerToughness::fixed(3, 3))
            .parse_text(&text)
            .unwrap();
        assert!(
            crate::compiled_text::compiled_text_lines(&card)
                .join("\n")
                .contains("or another Human you control enters")
        );
        for (source_enters, human, own, expected) in [
            (true, true, true, 1),
            (false, true, true, 1),
            (false, true, false, 0),
            (false, false, true, 0),
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let (alice, bob) = (game.players[0].id, game.players[1].id);
            let blank = crate::CardDefinitionBuilder::new(CardId::new(), "Meravin")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![if human { Subtype::Human } else { Subtype::Elf }])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
            for _ in 0..3 {
                game.create_object_from_definition(&blank, alice, Zone::Library);
            }
            let entrant = if source_enters {
                game.create_object_from_definition(&card, alice, Zone::Hand)
            } else {
                game.create_object_from_definition(&card, alice, Zone::Battlefield);
                game.create_object_from_definition(
                    &blank,
                    if own { alice } else { bob },
                    Zone::Hand,
                )
            };
            game.take_pending_trigger_events();
            game.move_object_with_etb_processing(entrant, Zone::Battlefield)
                .unwrap();
            let mut queue = crate::triggers::TriggerQueue::new();
            crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
            crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert_eq!(
                game.stack.len(),
                expected,
                "source={source_enters}, human={human}, own={own}"
            );
            if expected != 0 {
                crate::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
            assert_eq!(game.player(alice).unwrap().hand.len(), expected);
        }
    }
}
