use super::*;

struct ChooseCopy(bool);
impl crate::decision::DecisionMaker for ChooseCopy {
    fn decide_options(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        let choices = ctx
            .options
            .iter()
            .filter(|option| option.legal)
            .collect::<Vec<_>>();
        choices
            .get(if self.0 {
                choices.len().saturating_sub(1)
            } else {
                0
            })
            .map(|option| vec![option.index])
            .unwrap_or_default()
    }
}

#[test]
fn cohort_copy_exceptions_apply_supertypes_and_conditional_counters_as_it_enters() {
    let mirror = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Snow Mirror")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(0, 0))
        .parse_text("Changeling\nYou may have this creature enter as a copy of a permanent you control, except it's legendary and snow in addition to its other types and, if it's a creature, it enters with two additional +1/+1 counters on it and has changeling.").unwrap();
    assert!(mirror.abilities.iter().any(|ability| matches!(&ability.kind, crate::ability::AbilityKind::Static(ability) if ability.enter_as_copy_as_enters().is_some())));
    for (creature, animated) in [(false, false), (false, true), (true, false)] {
        for copy in [false, true] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Copy Candidate")
                    .card_types(vec![if creature {
                        CardType::Creature
                    } else {
                        CardType::Artifact
                    }])
                    .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                    .parse_text(if creature {
                        "This creature enters with three +1/+1 counters on it."
                    } else {
                        ""
                    })
                    .unwrap();
            let candidate = game.create_object_from_definition(&source, alice, Zone::Battlefield);
            if animated {
                let animation = crate::effects::ApplyContinuousEffect::new(
                    crate::continuous::EffectTarget::Filter(crate::target::ObjectFilter::specific(
                        candidate,
                    )),
                    crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
                    crate::effect::Until::EndOfTurn,
                );
                let mut ctx = crate::effects::EffectContext::new_default(candidate, alice);
                crate::effects::execute_effect(
                    &mut game,
                    &crate::effect::Effect::new(animation),
                    &mut ctx,
                )
                .unwrap();
                assert!(game.current_is_creature(candidate));
            }
            let entrant = game.create_object_from_definition(&mirror, alice, Zone::Hand);
            let entered = game
                .move_object_with_etb_processing_with_dm(
                    entrant,
                    Zone::Battlefield,
                    &mut ChooseCopy(copy),
                )
                .unwrap()
                .new_id;
            let object = game.object(entered).unwrap();
            assert_eq!(
                object.name.as_ref(),
                if copy {
                    "Copy Candidate"
                } else {
                    "Snow Mirror"
                }
            );
            assert_eq!(
                object
                    .counters
                    .get(&crate::object::CounterType::PlusOnePlusOne)
                    .copied()
                    .unwrap_or(0),
                if copy && creature { 5 } else { 0 }
            );
            for supertype in [
                crate::types::Supertype::Legendary,
                crate::types::Supertype::Snow,
            ] {
                assert_eq!(object.supertypes.contains(&supertype), copy);
            }
            assert_eq!(
                game.current_has_static_ability_id(
                    entered,
                    crate::static_abilities::StaticAbilityId::Changeling
                ),
                !copy || creature
            );
            if copy {
                // Copy exceptions are themselves copiable; entry counters are not.
                let descendant = crate::CardDefinitionBuilder::new(
                    crate::ids::CardId::new(),
                    "Second Mirror",
                )
                .card_types(vec![CardType::Creature])
                .parse_text(
                    "You may have this creature enter as a copy of a permanent you control.",
                )
                .unwrap();
                let other = game.create_object_from_definition(&descendant, alice, Zone::Hand);
                let other = game
                    .move_object_with_etb_processing_with_dm(
                        other,
                        Zone::Battlefield,
                        &mut ChooseCopy(true),
                    )
                    .unwrap()
                    .new_id;
                let object = game.object(other).unwrap();
                assert!(
                    object
                        .supertypes
                        .contains(&crate::types::Supertype::Legendary)
                );
                assert!(object.supertypes.contains(&crate::types::Supertype::Snow));
                assert_eq!(
                    object
                        .counters
                        .get(&crate::object::CounterType::PlusOnePlusOne)
                        .copied()
                        .unwrap_or(0),
                    if creature { 3 } else { 0 }
                );
            }
        }
    }
}

#[test]
fn cohort_life_history_target_requires_loss_during_the_current_turn() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "History Mage")
        .card_types(vec![CardType::Creature])
        .parse_text("{B}{R}: Target player who lost life this turn loses 1 life.")
        .unwrap();
    let crate::ability::AbilityKind::Activated(ability) = &card.abilities[0].kind else {
        panic!("activated");
    };
    let target = ability
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::TargetOnlyEffect>())
        .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    assert!(
        crate::game_loop::compute_legal_targets(&game, &target.target, alice, Some(source))
            .is_empty()
    );
    let mut ctx = crate::effects::EffectContext::new_default(source, alice);
    let result = crate::effects::execute_effect(
        &mut game,
        &crate::effect::Effect::lose_life_player(2, crate::target::PlayerFilter::Specific(bob)),
        &mut ctx,
    )
    .unwrap();
    for event in result.events {
        game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event);
    }
    assert_eq!(
        crate::game_loop::compute_legal_targets(&game, &target.target, alice, Some(source)),
        vec![crate::game_state::Target::Player(bob)]
    );
    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);
    for effect in ability.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.player(bob).unwrap().life, 17);
    assert_eq!(game.player(alice).unwrap().life, 20);
    game.turn_store.turn_history.clear_for_new_turn();
    assert!(
        crate::game_loop::compute_legal_targets(&game, &target.target, alice, Some(source))
            .is_empty()
    );
}

#[test]
fn cohort_shared_color_or_mana_value_trigger_uses_its_own_exiled_card() {
    use crate::mana::{ManaCost, ManaSymbol};
    let prison = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Imprint Prison")
        .card_types(vec![CardType::Artifact])
        .parse_text("Whenever a player casts a spell that shares a color or mana value with the exiled card, this artifact deals 2 damage to that player.").unwrap();
    for linked in [false, true] {
        for color_matches in [false, true] {
            for value_matches in [false, true] {
                for own_cast in [false, true] {
                    let mut game =
                        crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    let alice = game.players[0].id;
                    let bob = game.players[1].id;
                    let source =
                        game.create_object_from_definition(&prison, alice, Zone::Battlefield);
                    let reference = crate::CardDefinitionBuilder::new(
                        crate::ids::CardId::new(),
                        "Reference Spell",
                    )
                    .card_types(vec![CardType::Sorcery])
                    .mana_cost(ManaCost::from_symbols(vec![
                        ManaSymbol::Generic(2),
                        ManaSymbol::Red,
                    ]))
                    .build();
                    let reference =
                        game.create_object_from_definition(&reference, bob, Zone::Exile);
                    let other =
                        game.create_object_from_definition(&prison, alice, Zone::Battlefield);
                    game.add_exiled_with_source_link(
                        if linked { source } else { other },
                        reference,
                    );
                    // Only observe the first prison, while keeping an unrelated exile link.
                    game.object_mut(other).unwrap().abilities_mut().clear();
                    let spell =
                        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Test Spell")
                            .card_types(vec![CardType::Instant])
                            .mana_cost(ManaCost::from_symbols(vec![
                                ManaSymbol::Generic(if value_matches { 2 } else { 4 }),
                                if color_matches {
                                    ManaSymbol::Red
                                } else {
                                    ManaSymbol::Blue
                                },
                            ]))
                            .build();
                    let caster = if own_cast { alice } else { bob };
                    let spell = game.create_object_from_definition(&spell, caster, Zone::Stack);
                    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
                        game.object(spell).unwrap(),
                        &game,
                    );
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
                    let fires = linked && (color_matches || value_matches);
                    assert_eq!(
                        triggers.len(),
                        usize::from(fires),
                        "linked={linked} color={color_matches} value={value_matches}"
                    );
                    let mut queue = crate::triggers::TriggerQueue::new();
                    for trigger in triggers {
                        queue.add(trigger);
                    }
                    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                    if fires {
                        assert_eq!(game.stack.len(), 1);
                        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                    }
                    assert_eq!(
                        game.player(caster).unwrap().life,
                        if fires { 18 } else { 20 }
                    );
                    assert_eq!(
                        game.player(if own_cast { bob } else { alice })
                            .unwrap()
                            .life,
                        20
                    );
                }
            }
        }
    }
}

#[test]
fn cohort_each_mixed_target_gets_its_own_exiled_cards_damage_and_all_cards_remain_playable() {
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Library Eruption")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Choose any number of target creatures, planeswalkers, and/or players. For each of them, exile the top card of your library, then this spell deals damage equal to that card's mana value to that permanent or player. You may play the exiled cards until the end of your next turn.").unwrap();
    for empty in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
        let body = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(10, 10))
            .build();
        let first = game.create_object_from_definition(&body, alice, Zone::Battlefield);
        let second = game.create_object_from_definition(&body, bob, Zone::Battlefield);
        let mut stable_ids = Vec::new();
        for value in (2..=5).rev() {
            let card = crate::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                format!("Library {value}"),
            )
            .card_types(vec![CardType::Sorcery])
            .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::Generic(value),
            ]))
            .build();
            let id = game.create_object_from_definition(&card, alice, Zone::Library);
            stable_ids.push(game.object(id).unwrap().stable_id);
        }
        let targets = if empty {
            vec![]
        } else {
            vec![
                crate::effects::ResolvedTarget::Player(alice),
                crate::effects::ResolvedTarget::Player(bob),
                crate::effects::ResolvedTarget::Object(first),
                crate::effects::ResolvedTarget::Object(second),
            ]
        };
        let mut ctx =
            crate::effects::EffectContext::new_default(source, alice).with_targets(targets);
        for effect in spell
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects()
        {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(
            game.player(alice).unwrap().life,
            if empty { 20 } else { 18 }
        );
        assert_eq!(game.player(bob).unwrap().life, if empty { 20 } else { 17 });
        assert_eq!(game.damage_on(first), if empty { 0 } else { 4 });
        assert_eq!(game.damage_on(second), if empty { 0 } else { 5 });
        assert_eq!(
            game.player(alice).unwrap().library.len(),
            if empty { 4 } else { 0 }
        );
        if !empty {
            for stable in stable_ids {
                let id = game
                    .exile
                    .iter()
                    .copied()
                    .find(|id| game.object(*id).is_some_and(|o| o.stable_id == stable))
                    .unwrap();
                assert!(
                    game.effect_store.grant_registry.card_can_play_from_zone(
                        &game,
                        id,
                        Zone::Exile,
                        alice
                    ),
                    "every iteration contributes to the play permission"
                );
                assert!(!game.effect_store.grant_registry.card_can_play_from_zone(
                    &game,
                    id,
                    Zone::Exile,
                    bob
                ));
                let created_turn = game.turn.turn_number;
                for offset in 1..=3 {
                    game.turn.turn_number = created_turn + offset;
                    game.turn.active_player = if offset == 2 { alice } else { bob };
                    assert_eq!(
                        game.effect_store.grant_registry.card_can_play_from_zone(
                            &game,
                            id,
                            Zone::Exile,
                            alice
                        ),
                        offset <= 2
                    );
                }
                game.turn.turn_number = created_turn;
                game.turn.active_player = alice;
            }
        }
    }
}

#[test]
fn cohort_combined_combat_and_activation_restrictions_apply_only_to_the_selected_creature() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Authority")
        .card_types(vec![CardType::Artifact])
        .parse_text("{1}, {T}: Target creature can't attack this turn. Put a brick counter on this artifact.\n{1}, {T}: Until your next turn, target creature can't attack or block and its activated abilities can't be activated. Activate only if there are three or more brick counters on this artifact.").unwrap();
    let crate::ability::AbilityKind::Activated(first_ability) = &card.abilities[0].kind else {
        panic!("activated");
    };
    let counter_effect = first_ability
        .effects
        .flattened_default_effects()
        .iter()
        .find(|effect| {
            effect
                .downcast_ref::<crate::effects::PutCountersEffect>()
                .is_some()
        })
        .unwrap();
    let counter_type = counter_effect
        .downcast_ref::<crate::effects::PutCountersEffect>()
        .unwrap()
        .counter_type;
    let crate::ability::AbilityKind::Activated(ability) = &card.abilities[1].kind else {
        panic!("activated");
    };
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let body = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Creature")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Add {G}.")
        .unwrap();
    let target = game.create_object_from_definition(&body, bob, Zone::Battlefield);
    let bystander = game.create_object_from_definition(&body, bob, Zone::Battlefield);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(crate::mana::ManaSymbol::Colorless, 1);
    for counters in [0, 2, 3, 4] {
        game.object_mut(source)
            .unwrap()
            .counters
            .remove(&counter_type);
        for _ in 0..counters {
            let mut ctx = crate::effects::EffectContext::new_default(source, alice);
            crate::effects::execute_effect(&mut game, counter_effect, &mut ctx).unwrap();
        }
        game.refresh_continuous_state();
        let actions = crate::decision::compute_actions_for_source(&game, alice, Some(source));
        assert_eq!(actions.iter().any(|action| matches!(action, crate::decision::LegalAction::ActivateAbility { source: id, ability_index: 1 } if *id == source)), counters >= 3);
    }
    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    for effect in ability.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    game.update_cant_effects();
    assert!(game.effect_store.cant_effects.cant_attack.contains(&target));
    assert!(game.effect_store.cant_effects.cant_block.contains(&target));
    assert!(
        game.effect_store
            .cant_effects
            .cant_activate_abilities_of
            .contains(&target)
    );
    assert!(
        !game
            .effect_store
            .cant_effects
            .cant_attack
            .contains(&bystander)
    );
    assert!(
        !game
            .effect_store
            .cant_effects
            .cant_block
            .contains(&bystander)
    );
    assert!(
        !game
            .effect_store
            .cant_effects
            .cant_activate_abilities_of
            .contains(&bystander)
    );
    game.turn.turn_number += 1;
    game.turn.active_player = bob;
    game.update_cant_effects();
    assert!(!game.can_activate_abilities_of(target));
    game.turn.turn_number += 1;
    game.turn.active_player = alice;
    game.update_cant_effects();
    assert!(game.can_activate_abilities_of(target));
    assert!(!game.effect_store.cant_effects.cant_attack.contains(&target));
    assert!(!game.effect_store.cant_effects.cant_block.contains(&target));
}
