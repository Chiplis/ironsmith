use super::*;

const TEXT: &str = "Whenever you cast a permanent spell with a mana cost that contains {X}, double the value of X.\nWhenever you cast an instant or sorcery spell or activate an ability, if that spell's mana cost or that ability's activation cost contains {X}, copy that spell or ability. You may choose new targets for the copy.";

#[test]
fn unbound_x_cost_trigger_matches_only_your_x_cost_activations() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Unbound Flourishing")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let artifact = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Ability Source")
        .card_types(vec![CardType::Artifact])
        .build();
    for activator in [alice, bob] {
        let source = game.create_object_from_card(&artifact, activator, Zone::Battlefield);
        for has_x in [true, false] {
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::AbilityActivatedEvent::new(source, activator, false)
                    .with_activation_cost_has_x(has_x)
                    .with_x_value(has_x.then_some(3)),
                crate::provenance::ProvNodeId::default(),
            );
            assert_eq!(
                crate::triggers::check_triggers(&game, &event).len(),
                usize::from(activator == alice && has_x),
                "activation by {activator:?}, X in activation cost: {has_x}"
            );
        }
    }
}

#[test]
fn unbound_x_cost_trigger_rejects_spells_without_x_in_the_cost() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Unbound Flourishing")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut mismatches = Vec::new();
    for caster in [alice, bob] {
        for kind in [CardType::Instant, CardType::Sorcery, CardType::Creature] {
            for has_x in [false, true] {
                let cost = crate::mana::ManaCost::from_symbols(vec![if has_x {
                    crate::mana::ManaSymbol::X
                } else {
                    crate::mana::ManaSymbol::Generic(3)
                }]);
                let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Spell Probe")
                    .card_types(vec![kind])
                    .mana_cost(cost)
                    .build();
                let spell = game.create_object_from_card(&card, caster, Zone::Stack);
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::SpellCastEvent::new(spell, caster, Zone::Hand),
                    crate::provenance::ProvNodeId::default(),
                );
                let count = crate::triggers::check_triggers(&game, &event).len();
                let expected = usize::from(caster == alice && has_x);
                if count != expected {
                    mismatches.push(format!(
                        "{caster:?} {kind:?} has_x={has_x}: {count} triggers, expected {expected}"
                    ));
                }
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("; "));
}

struct CopyTarget {
    player: crate::ids::PlayerId,
    target: crate::ids::PlayerId,
    choices: usize,
}
impl crate::decision::DecisionMaker for CopyTarget {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        assert_eq!(ctx.player, self.player);
        true
    }
    fn decide_targets(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<crate::game_state::Target> {
        assert_eq!(ctx.player, self.player);
        self.choices += 1;
        vec![crate::game_state::Target::Player(self.target)]
    }
}

#[test]
fn unbound_x_cost_trigger_copies_x_and_allows_new_targets() {
    for (ability, older_activation, source_leaves, activation_countered) in [
        (false, false, false, false),
        (true, false, false, false),
        (true, true, false, false),
        (true, true, true, false),
        (true, true, false, true),
    ] {
        let unbound =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Unbound Flourishing")
                .card_types(vec![CardType::Enchantment])
                .parse_text(TEXT)
                .unwrap();
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "X Damage Probe")
                .card_types(vec![if ability {
                    CardType::Artifact
                } else {
                    CardType::Instant
                }])
                .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                    crate::mana::ManaSymbol::X,
                ]))
                .parse_text(if ability {
                    "{X}: Deal X damage to any target."
                } else {
                    "Deal X damage to any target."
                })
                .unwrap();
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let carol = game.players[2].id;
        game.create_object_from_definition(&unbound, alice, Zone::Battlefield);
        let object = game.create_object_from_definition(
            &definition,
            alice,
            if ability {
                Zone::Battlefield
            } else {
                Zone::Stack
            },
        );
        let mut entry = if ability {
            let activated = definition
                .abilities
                .iter()
                .find_map(|a| {
                    if let crate::ability::AbilityKind::Activated(a) = &a.kind {
                        Some(a)
                    } else {
                        None
                    }
                })
                .unwrap();
            crate::game_state::StackEntry::ability(object, alice, activated.effects.clone())
        } else {
            game.object_mut(object).unwrap().x_value = Some(3);
            crate::game_state::StackEntry::new(object, alice)
        };
        let activation_provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::AbilityActivated);
        let source_snapshot =
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(object).unwrap(),
                &game,
            );
        entry.source_snapshot = Some(source_snapshot.clone());
        entry.provenance = activation_provenance;
        entry.x_value = Some(3);
        entry.activation_cost_has_x = ability;
        entry.targets = vec![crate::game_state::Target::Player(bob)];
        if older_activation {
            let mut old = entry.clone();
            old.x_value = Some(1);
            old.provenance = game
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::AbilityActivated);
            game.push_to_stack(old);
        }
        game.push_to_stack(entry);
        let event = if ability {
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::AbilityActivatedEvent::new(object, alice, false)
                    .with_activation_cost_has_x(true)
                    .with_x_value(Some(3))
                    .with_stack_entry_provenance(Some(activation_provenance))
                    .with_snapshot(Some(source_snapshot)),
                crate::provenance::ProvNodeId::default(),
            )
        } else {
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::SpellCastEvent::new(object, alice, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            )
        };
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        let mut queue = crate::triggers::TriggerQueue::new();
        for trigger in triggers {
            queue.add(trigger);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        if source_leaves {
            game.move_object_by_effect(object, Zone::Graveyard).unwrap();
        }
        if activation_countered {
            game.stack.remove(usize::from(older_activation));
        }
        let mut dm = CopyTarget {
            player: alice,
            target: carol,
            choices: 0,
        };
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        if activation_countered {
            assert_eq!(
                game.stack.len(),
                1,
                "a removed activation must not redirect the copy to an older one"
            );
            assert_eq!(game.stack[0].x_value, Some(1));
            assert_eq!(dm.choices, 0);
            continue;
        }
        assert_eq!(
            game.stack.len(),
            2 + usize::from(older_activation),
            "ability={ability}"
        );
        let copy = game.stack.last().unwrap();
        assert_eq!(
            copy.x_value,
            Some(3),
            "ability={ability}, older={older_activation}"
        );
        assert_eq!(copy.is_ability, ability);
        assert_eq!(copy.targets, vec![crate::game_state::Target::Player(carol)]);
        assert_eq!(dm.choices, 1);
        assert_eq!(
            game.stack[usize::from(older_activation)].targets,
            vec![crate::game_state::Target::Player(bob)]
        );
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(carol).unwrap().life, 17);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
}

#[test]
fn unbound_x_cost_trigger_doubles_only_the_permanent_spells_x() {
    for x in [0, 3] {
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Unbound Flourishing")
                .card_types(vec![CardType::Enchantment])
                .parse_text(TEXT)
                .unwrap();
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "X Permanent")
            .card_types(vec![CardType::Creature])
            .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::X,
            ]))
            .build();
        let spell = game.create_object_from_card(&card, alice, Zone::Stack);
        game.object_mut(spell).unwrap().x_value = Some(x);
        game.push_to_stack(crate::game_state::StackEntry::new(spell, alice).with_x(x));
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::SpellCastEvent::new(spell, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        let mut queue = crate::triggers::TriggerQueue::new();
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        for trigger in triggers {
            queue.add(trigger);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].x_value, Some(2 * x));
        assert_eq!(game.object(spell).unwrap().x_value, Some(2 * x));
    }
}
