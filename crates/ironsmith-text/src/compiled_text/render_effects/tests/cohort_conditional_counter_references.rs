use super::*;

#[test]
fn cohort_conditional_counter_placements_keep_the_original_commander() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Commander Forge")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Choose target commander that entered this turn. Put a +1/+1 counter on it if it's a creature and a loyalty counter on it if it's a planeswalker.").unwrap();
    let crate::ability::AbilityKind::Activated(activated) = &card.abilities[0].kind else {
        panic!("activated");
    };
    for types in [
        vec![CardType::Creature],
        vec![CardType::Planeswalker],
        vec![CardType::Creature, CardType::Planeswalker],
        vec![CardType::Enchantment],
    ] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let commander =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Type-changing commander")
                .card_types(types.clone())
                .power_toughness(crate::card::PowerToughness::fixed(3, 3))
                .build();
        let commander = game.create_object_from_definition(&commander, alice, Zone::Hand);
        game.set_as_commander(commander, alice);
        let commander = game
            .move_object_by_effect(commander, Zone::Battlefield)
            .unwrap();
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(commander)]);
        for effect in activated.effects.flattened_default_effects() {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(
            game.counter_count(commander, crate::object::CounterType::PlusOnePlusOne),
            u32::from(types.contains(&CardType::Creature)),
            "{types:?}"
        );
        assert_eq!(
            game.counter_count(commander, crate::object::CounterType::Loyalty),
            u32::from(types.contains(&CardType::Planeswalker)),
            "{types:?}"
        );
    }
}

#[test]
fn cohort_counter_trigger_draws_or_loses_life_for_the_actual_creatures_controller() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter Witness")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever one or more -1/-1 counters are put on a creature, draw a card if you control that creature. If you don't control it, its controller loses 1 life.").unwrap();
    for own in [false, true] {
        for amount in [1, 2] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
            let creature =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter recipient")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(5, 5))
                    .build();
            let target = game.create_object_from_definition(
                &creature,
                if own { alice } else { bob },
                Zone::Battlefield,
            );
            game.create_object_from_definition(&creature, alice, Zone::Library);
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
            let counters = crate::effects::PutCountersEffect::new(
                crate::object::CounterType::MinusOneMinusOne,
                amount,
                crate::target::ChooseSpec::target(crate::target::ChooseSpec::creature()),
            );
            let result =
                crate::effects::EffectExecutor::execute(&counters, &mut game, &mut ctx).unwrap();
            let mut fired = 0;
            for event in result.events {
                for trigger in crate::triggers::check_triggers(&game, &event) {
                    fired += 1;
                    let mut ctx = crate::effects::EffectContext::new_default(
                        trigger.source,
                        trigger.controller,
                    );
                    ctx.triggering_event = Some(trigger.triggering_event);
                    for effect in trigger.ability.effects.flattened_default_effects() {
                        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    }
                }
            }
            assert_eq!(fired, 1, "one-or-more is one trigger for the batch");
            assert_eq!(game.player(alice).unwrap().hand.len(), usize::from(own));
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(bob).unwrap().life, if own { 20 } else { 19 });
        }
    }
}

#[test]
fn cohort_named_permanent_replacement_replaces_instead_of_stacking_stat_penalties() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Dying Newt")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .parse_text("When this creature dies, target creature an opponent controls gets -1/-1 until end of turn. That creature gets -4/-4 instead if you control a creature named Bogbrew Witch.").unwrap();
    for witch_owner in [None, Some(0), Some(1)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Large target")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(10, 10))
            .build();
        let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        if let Some(owner) = witch_owner {
            let witch =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Bogbrew Witch")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(1, 3))
                    .build();
            game.create_object_from_definition(&witch, game.players[owner].id, Zone::Battlefield);
        }
        game.take_pending_trigger_events();
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let mut queue = crate::triggers::TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for trigger in crate::triggers::check_triggers(&game, &event) {
                queue.add(trigger);
            }
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        let mut choices = crate::decision::SelectFirstDecisionMaker;
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(
            game.calculated_characteristics(target).unwrap().power,
            Some(if witch_owner == Some(0) { 6 } else { 9 })
        );
    }
}

#[test]
fn cohort_zombie_or_token_creature_union_is_inclusive_and_controller_scoped() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Winged Union")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Creatures you control that are Zombies and/or tokens get +1/+1 and have flying.",
        )
        .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    game.create_object_from_definition(&card, alice, Zone::Battlefield);
    for controller in [alice, bob] {
        for zombie in [false, true] {
            for token in [false, true] {
                let mut builder =
                    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Union member")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(crate::card::PowerToughness::fixed(2, 2));
                if zombie {
                    builder = builder.subtypes(vec![crate::types::Subtype::Zombie]);
                }
                if token {
                    builder = builder.token();
                }
                let object = game.create_object_from_definition(
                    &builder.build(),
                    controller,
                    Zone::Battlefield,
                );
                let qualifies = controller == alice && (zombie || token);
                assert_eq!(
                    game.calculated_characteristics(object).unwrap().power,
                    Some(if qualifies { 3 } else { 2 }),
                    "zombie={zombie} token={token} controller={controller:?}"
                );
                assert_eq!(
                    game.current_has_static_ability_id(
                        object,
                        crate::static_abilities::StaticAbilityId::Flying
                    ),
                    qualifies
                );
            }
        }
    }
}

#[test]
fn cohort_convoke_cast_trigger_damages_only_opponents_and_the_battles_they_protect() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Battle Sculptor")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever you cast a spell that has convoke, this creature deals 1 damage to each opponent and each battle they protect.").unwrap();
    for convoke in [false, true] {
        for own_cast in [false, true] {
            let mut game = crate::game_state::GameState::new(
                vec!["Alice".into(), "Bob".into(), "Carol".into()],
                20,
            );
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let carol = game.players[2].id;
            game.create_object_from_definition(&card, alice, Zone::Battlefield);
            let battle_card =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Protected battle")
                    .card_types(vec![CardType::Battle])
                    .subtypes(vec![crate::types::Subtype::Siege])
                    .defense(10)
                    .build();
            let mut battles = Vec::new();
            for protector in [alice, bob, carol] {
                let controller = if protector == alice { bob } else { alice };
                let battle =
                    game.create_object_from_definition(&battle_card, controller, Zone::Battlefield);
                assert!(game.set_battle_protector(battle, protector));
                game.object_mut(battle)
                    .unwrap()
                    .counters
                    .insert(crate::object::CounterType::Defense, 10);
                battles.push((battle, protector));
            }
            let spell_builder =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cast spell")
                    .card_types(vec![CardType::Sorcery]);
            let spell = if convoke {
                spell_builder.parse_text("Convoke").unwrap()
            } else {
                spell_builder.build()
            };
            let caster = if own_cast { alice } else { bob };
            let spell = game.create_object_from_definition(&spell, caster, Zone::Stack);
            let snapshot =
                crate::snapshot::ObjectSnapshot::from_object(game.object(spell).unwrap(), &game);
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::SpellCastEvent::new_with_snapshot(
                    spell,
                    caster,
                    Zone::Hand,
                    snapshot,
                ),
                crate::provenance::ProvNodeId::default(),
            );
            let triggers = crate::triggers::check_triggers(&game, &event);
            let fires = convoke && own_cast;
            assert_eq!(triggers.len(), usize::from(fires));
            for trigger in triggers {
                let mut ctx =
                    crate::effects::EffectContext::new_default(trigger.source, trigger.controller);
                ctx.triggering_event = Some(trigger.triggering_event);
                for effect in trigger.ability.effects.flattened_default_effects() {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            assert_eq!(game.player(alice).unwrap().life, 20);
            for opponent in [bob, carol] {
                assert_eq!(
                    game.player(opponent).unwrap().life,
                    if fires { 19 } else { 20 }
                );
            }
            for (battle, protector) in battles {
                assert_eq!(
                    game.counter_count(battle, crate::object::CounterType::Defense),
                    if fires && protector != alice { 9 } else { 10 },
                    "protector={protector:?}"
                );
            }
        }
    }
}
