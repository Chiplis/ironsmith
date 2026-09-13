use super::*;
use crate::ability::AbilityKind;
use crate::card::PowerToughness;
use crate::game_state::{GameState, StackEntry, Target, TargetAssignment};
use crate::ids::CardId;

#[test]
fn cohort_counter_kind_choice_is_made_on_resolution_after_one_target_is_announced() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Vezarin")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{R}, {T}, Discard a card: Put a +0/+1 counter or a +1/+0 counter on target creature.",
        )
        .unwrap();
    let ability = card
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    struct Pick {
        index: usize,
        called: usize,
    }
    impl crate::decision::DecisionMaker for Pick {
        fn decide_options(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.called += 1;
            assert_eq!(ctx.options.len(), 2);
            vec![self.index]
        }
    }
    for index in [0, 1] {
        for removed in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
            let body = crate::CardDefinitionBuilder::new(CardId::new(), "Target")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
            let target = game.create_object_from_definition(&body, alice, Zone::Battlefield);
            let requirements =
                crate::game_loop::extract_target_requirements_from_program_with_modes(
                    &game,
                    &ability.effects,
                    alice,
                    Some(source),
                    None,
                );
            assert_eq!(
                requirements.len(),
                1,
                "counter choice must announce one shared target"
            );
            let mut entry = StackEntry::ability(source, alice, ability.effects.clone())
                .with_targets(vec![Target::Object(target)]);
            entry.target_assignments = vec![TargetAssignment {
                spec: requirements[0].spec.clone(),
                range: 0..1,
            }];
            game.stack.push(entry);
            if removed {
                game.move_object_by_effect(target, Zone::Graveyard).unwrap();
            }
            let mut pick = Pick { index, called: 0 };
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut pick).unwrap();
            assert_eq!(pick.called, usize::from(!removed));
            if !removed {
                let chars = game.current_characteristics(target).unwrap();
                assert_eq!(
                    (chars.power, chars.toughness),
                    if index == 0 {
                        (Some(2), Some(3))
                    } else {
                        (Some(3), Some(2))
                    }
                );
                assert_eq!(
                    game.counter_count(target, crate::CounterType::PlusZeroPlusOne),
                    u32::from(index == 0)
                );
                assert_eq!(
                    game.counter_count(target, crate::CounterType::PlusOnePlusZero),
                    u32::from(index == 1)
                );
            }
        }
    }
}

#[test]
fn cohort_conditional_attack_requirement_is_a_live_static_ability_and_excludes_source() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Zarovin")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Ally])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("This creature attacks each combat if able unless you control another Ally.")
        .unwrap();
    assert!(card.spell_effect.is_none());
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let (alice, bob) = (game.players[0].id, game.players[1].id);
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let required = |game: &GameState| {
        game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::MustAttack,
        )
    };
    assert!(required(&game));
    let ally = crate::CardDefinitionBuilder::new(CardId::new(), "Vezra")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Ally])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let opponents_ally = game.create_object_from_definition(&ally, bob, Zone::Battlefield);
    assert!(required(&game));
    let other = game.create_object_from_definition(&ally, alice, Zone::Battlefield);
    assert!(!required(&game));
    game.move_object_by_effect(other, Zone::Graveyard).unwrap();
    assert!(required(&game));
    game.set_current_controller(source, bob);
    assert!(!required(&game));
    game.set_current_controller(opponents_ally, alice);
    assert!(required(&game));
    let mut control_ctx = crate::effects::EffectContext::new_default(source, alice);
    crate::effects::execute_effect(
        &mut game,
        &Effect::gain_control_with_duration(ChooseSpec::Source, Until::Forever),
        &mut control_ctx,
    )
    .unwrap();
    assert_eq!(game.controller_of_id(source), Some(alice));
    assert!(!required(&game));
}

#[test]
fn cohort_first_kicked_spell_cost_reduction_and_trigger_ignore_unkicked_casts() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Voravin").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2))
        .parse_text("The first kicked spell you cast each turn costs {1} less to cast.\nWhenever you cast a kicked spell, put a +1/+1 counter on this creature.").unwrap();
    for keyword in ["Kicker", "Multikicker"] {
        let spell = crate::CardDefinitionBuilder::new(CardId::new(), "Zelvarin")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                crate::mana::ManaSymbol::Generic(3),
            ]))
            .parse_text(&format!("{keyword} {{1}}"))
            .unwrap();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let mut counters = 0;
        for (caster, kicked, expected_cost, next_turn) in [
            (alice, false, 3, false),
            (bob, true, 4, false),
            (alice, true, 3, false),
            (alice, false, 3, false),
            (alice, true, 4, false),
            (alice, true, 3, true),
        ] {
            if next_turn {
                game.turn_store.turn_history.clear_for_new_turn();
                game.turn.turn_number += 1;
                game.turn.active_player = bob;
            }
            let id = game.create_object_from_definition(&spell, caster, Zone::Stack);
            game.object_mut(id).unwrap().optional_costs_paid =
                crate::cost::OptionalCostsPaid::from_costs(&spell.optional_costs);
            if kicked {
                game.object_mut(id).unwrap().optional_costs_paid.pay(0);
            }
            assert_eq!(
                game.object(id).unwrap().optional_costs_paid.was_kicked(),
                kicked
            );
            game.stack.push(StackEntry::new(id, caster));
            let base = crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(
                if kicked { 4 } else { 3 },
            )]);
            let cost = crate::decision::calculate_effective_mana_cost(
                &game,
                caster,
                game.object(id).unwrap(),
                &base,
            );
            assert_eq!(
                cost.generic_mana_total(),
                expected_cost,
                "caster={caster:?}, kicked={kicked}, next_turn={next_turn}"
            );
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::SpellCastEvent::new_with_snapshot(
                    id,
                    caster,
                    Zone::Hand,
                    crate::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(), &game),
                ),
                crate::provenance::ProvNodeId::default(),
            );
            game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event.clone());
            game.take_pending_trigger_events();
            let triggers = crate::triggers::check_triggers(&game, &event);
            let expected = caster == alice && kicked;
            assert_eq!(triggers.len(), usize::from(expected));
            if expected {
                let mut queue = crate::triggers::TriggerQueue::new();
                for trigger in triggers {
                    queue.add(trigger);
                }
                crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                counters += 1;
            }
            assert_eq!(
                game.counter_count(source, crate::CounterType::PlusOnePlusOne),
                counters
            );
            game.move_object_by_effect(id, Zone::Graveyard).unwrap();
        }
    }
}

#[test]
fn cohort_life_lock_and_permanent_protection_share_duration_and_expire() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Valenra")
        .card_types(vec![CardType::Instant])
        .parse_text("You may sacrifice a nontoken white creature rather than pay this spell's mana cost.\nUntil end of turn, your life total can't change, and permanents you control gain hexproof and indestructible.").unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let (alice, bob) = (game.players[0].id, game.players[1].id);
    let creature = crate::CardDefinitionBuilder::new(CardId::new(), "Zalarin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let artifact = crate::CardDefinitionBuilder::new(CardId::new(), "Valoran")
        .card_types(vec![CardType::Artifact])
        .build();
    let own_creature = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let own_artifact = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
    let opposing = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let source = game.create_object_from_definition(&card, alice, Zone::Stack);
    let mut ctx = crate::effects::EffectContext::new_default(source, alice);
    for effect in card
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
    {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    for (id, expected) in [
        (own_creature, true),
        (own_artifact, true),
        (opposing, false),
    ] {
        for ability in [
            crate::static_abilities::StaticAbilityId::Hexproof,
            crate::static_abilities::StaticAbilityId::Indestructible,
        ] {
            assert_eq!(game.current_has_static_ability_id(id, ability), expected);
        }
    }
    assert_eq!(game.gain_life(alice, 3), 0);
    assert_eq!(game.lose_life(alice, 4), 0);
    crate::effects::execute_effect(&mut game, &Effect::set_life_total(7), &mut ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.gain_life(bob, 3), 3);
    assert_eq!(game.lose_life(bob, 4), 4);
    let mut own_ctx = crate::effects::EffectContext::new_default(own_creature, alice);
    crate::effects::execute_effect(
        &mut game,
        &Effect::new(crate::effects::DestroyEffect::with_spec(ChooseSpec::Source)),
        &mut own_ctx,
    )
    .unwrap();
    assert_eq!(game.object(own_creature).unwrap().zone, Zone::Battlefield);
    // An ordinary continuous grant fixes its set of recipients at resolution.
    let later = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    assert!(
        !game.current_has_static_ability_id(
            later,
            crate::static_abilities::StaticAbilityId::Hexproof
        )
    );
    game.cleanup_restrictions_end_of_turn();
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.turn.turn_number += 1;
    game.update_cant_effects();
    assert_eq!(game.gain_life(alice, 3), 3);
    assert_eq!(game.lose_life(alice, 4), 4);
    assert_eq!(game.player(alice).unwrap().life, 19);
    for id in [own_creature, own_artifact] {
        assert!(
            !game.current_has_static_ability_id(
                id,
                crate::static_abilities::StaticAbilityId::Hexproof
            )
        );
        assert!(!game.current_has_static_ability_id(
            id,
            crate::static_abilities::StaticAbilityId::Indestructible
        ));
    }
    crate::effects::execute_effect(
        &mut game,
        &Effect::new(crate::effects::DestroyEffect::with_spec(ChooseSpec::Source)),
        &mut own_ctx,
    )
    .unwrap();
    assert!(game.object(own_creature).is_none());
}

#[test]
fn cohort_optional_milled_spell_return_uses_only_the_milled_card_and_legendary_tap_cost() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Zanoril")
        .card_types(vec![CardType::Artifact])
        .parse_text("{1}, {T}: Mill a card. You may put an instant or sorcery card milled this way into your hand.\nTap an untapped legendary creature you control: Untap this artifact.").unwrap();
    let abilities = card
        .abilities
        .iter()
        .filter_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .collect::<Vec<_>>();
    struct Pick {
        accept: bool,
        forbidden: crate::ids::ObjectId,
        expected: Option<crate::ids::ObjectId>,
    }
    impl crate::decision::DecisionMaker for Pick {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.accept
        }
        fn decide_objects(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<crate::ids::ObjectId> {
            assert!(
                !ctx.candidates
                    .iter()
                    .any(|c| c.legal && c.id == self.forbidden)
            );
            if let Some(expected) = self.expected {
                assert_eq!(
                    ctx.candidates
                        .iter()
                        .filter(|c| c.legal)
                        .map(|c| c.id)
                        .collect::<Vec<_>>(),
                    vec![expected]
                );
            }
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .take(ctx.max.unwrap_or(ctx.min))
                .map(|c| c.id)
                .collect()
        }
    }
    for kind in [
        None,
        Some(CardType::Instant),
        Some(CardType::Sorcery),
        Some(CardType::Artifact),
    ] {
        for accept in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
            let old = crate::CardDefinitionBuilder::new(CardId::new(), "Old choice")
                .card_types(vec![CardType::Instant])
                .build();
            let old_id = game.create_object_from_definition(&old, alice, Zone::Graveyard);
            let stable = kind.map(|kind| {
                let top = crate::CardDefinitionBuilder::new(CardId::new(), "Milled choice")
                    .card_types(vec![kind])
                    .build();
                let id = game.create_object_from_definition(&top, alice, Zone::Library);
                game.object(id).unwrap().stable_id
            });
            let mut pick = Pick {
                accept,
                forbidden: old_id,
                expected: None,
            };
            game.stack.push(StackEntry::ability(
                source,
                alice,
                abilities[0].effects.clone(),
            ));
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut pick).unwrap();
            assert_eq!(game.object(old_id).unwrap().zone, Zone::Graveyard);
            assert!(game.player(alice).unwrap().library.is_empty());
            if let Some(stable) = stable {
                let returned =
                    accept && matches!(kind, Some(CardType::Instant | CardType::Sorcery));
                let in_hand = game
                    .player(alice)
                    .unwrap()
                    .hand
                    .iter()
                    .any(|id| game.object(*id).unwrap().stable_id == stable);
                let in_grave = game
                    .player(alice)
                    .unwrap()
                    .graveyard
                    .iter()
                    .any(|id| game.object(*id).unwrap().stable_id == stable);
                assert_eq!(in_hand, returned, "{kind:?}, accept={accept}");
                assert_eq!(in_grave, !returned);
            }
        }
    }
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let (alice, bob) = (game.players[0].id, game.players[1].id);
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    game.tap(source);
    let legend = crate::CardDefinitionBuilder::new(CardId::new(), "Vazara")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let own = game.create_object_from_definition(&legend, alice, Zone::Battlefield);
    let opposing = game.create_object_from_definition(&legend, bob, Zone::Battlefield);
    let plain = crate::CardDefinitionBuilder::new(CardId::new(), "Zeravin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let plain = game.create_object_from_definition(&plain, alice, Zone::Battlefield);
    let mut pick = Pick {
        accept: true,
        forbidden: opposing,
        expected: Some(own),
    };
    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut pick);
    let costs = abilities[1].mana_cost.costs();
    let choose = costs[0]
        .effect_ref()
        .unwrap()
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .unwrap();
    assert_eq!(choose.filter.zone, Some(Zone::Battlefield));
    assert_eq!(choose.filter.controller, Some(PlayerFilter::You));
    assert!(choose.filter.untapped);
    assert_eq!(choose.filter.supertypes, [Supertype::Legendary]);
    for cost in costs {
        crate::effects::execute_effect(&mut game, cost.effect_ref().unwrap(), &mut ctx).unwrap();
    }
    assert!(game.is_tapped(own));
    for effect in abilities[1].effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert!(!game.is_tapped(source));
    assert!(!game.is_tapped(opposing));
    assert!(!game.is_tapped(plain));
}
