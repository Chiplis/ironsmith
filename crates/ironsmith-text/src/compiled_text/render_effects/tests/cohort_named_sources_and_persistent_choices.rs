use super::*;
use crate::ability::AbilityKind;
use crate::card::PowerToughness;
use crate::game_state::{GameState, StackEntry, Target, TargetAssignment};
use crate::ids::CardId;

#[test]
fn cohort_named_regeneration_keeps_source_identity_and_consumes_one_shield() {
    for (name, reference) in [
        ("Voranel the Patient", "Voranel"),
        ("Zeravin", "Zeravin"),
        ("Toravin", "this creature"),
    ] {
        let oracle = format!("{{R}}{{R}}{{R}}: Regenerate {reference}.");
        let card = crate::CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .parse_text(&oracle)
            .unwrap();
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&card).join("\n"),
            oracle
        );
        let ability = card
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Activated(a) => Some(a),
                _ => None,
            })
            .unwrap();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let other = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        game.mark_damage(source, 2);
        game.stack
            .push(StackEntry::ability(source, alice, ability.effects.clone()));
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            crate::events::processing::process_destroy_full(&mut game, source, None),
            crate::events::processing::DestroyResult::Replaced
        );
        assert!(game.battlefield.contains(&source));
        assert!(game.is_tapped(source));
        assert_eq!(game.damage_on(source), 0);
        assert!(!game.is_tapped(other));
        let mut ctx = crate::effects::EffectContext::new_default(other, alice);
        crate::effects::execute_effect(
            &mut game,
            &Effect::destroy(ChooseSpec::SpecificObject(source)),
            &mut ctx,
        )
        .unwrap();
        assert!(
            !game.battlefield.contains(&source),
            "the shield must be consumed once"
        );
        assert!(game.battlefield.contains(&other));
    }
}

#[test]
fn cohort_fixed_counter_and_keyword_choice_share_optional_target() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Veralyn")
        .card_types(vec![CardType::Planeswalker])
        .parse_text("+1: Choose up to one target creature. Put a +1/+1 counter and a counter from among flying, first strike, lifelink, or vigilance on it.").unwrap();
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
            assert_eq!(ctx.options.len(), 4);
            vec![self.index]
        }
    }
    for (index, counter, keyword) in [
        (
            0,
            crate::CounterType::Flying,
            crate::static_abilities::StaticAbilityId::Flying,
        ),
        (
            1,
            crate::CounterType::FirstStrike,
            crate::static_abilities::StaticAbilityId::FirstStrike,
        ),
        (
            2,
            crate::CounterType::Lifelink,
            crate::static_abilities::StaticAbilityId::Lifelink,
        ),
        (
            3,
            crate::CounterType::Vigilance,
            crate::static_abilities::StaticAbilityId::Vigilance,
        ),
    ] {
        for state in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
            let body = crate::CardDefinitionBuilder::new(CardId::new(), "Zaravel")
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
            assert_eq!(requirements.len(), 1);
            let selected = state != 0;
            let mut entry = StackEntry::ability(source, alice, ability.effects.clone())
                .with_targets(if selected {
                    vec![Target::Object(target)]
                } else {
                    vec![]
                });
            entry.target_assignments = vec![TargetAssignment {
                spec: requirements[0].spec.clone(),
                range: 0..usize::from(selected),
            }];
            game.stack.push(entry);
            if state == 2 {
                game.move_object_by_effect(target, Zone::Graveyard).unwrap();
            }
            let mut pick = Pick { index, called: 0 };
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut pick).unwrap();
            assert_eq!(
                game.counter_count(target, crate::CounterType::PlusOnePlusOne),
                u32::from(state == 1)
            );
            assert_eq!(game.counter_count(target, counter), u32::from(state == 1));
            if state == 1 {
                assert!(game.current_has_static_ability_id(target, keyword));
                assert_eq!(game.current_power(target), Some(3));
            }
            assert_eq!(pick.called, usize::from(state == 1));
        }
    }
}

#[test]
fn cohort_cumulative_upkeep_repeats_opponent_token_payment_or_sacrifices() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Varanel").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3,4))
        .parse_text("Cumulative upkeep—Have an opponent create a 1/1 red Survivor creature token.\nTrample; rampage 1").unwrap();
    assert!(
        card.spell_effect.is_none(),
        "upkeep payment must not become a creature spell effect"
    );
    struct Pay {
        accept: bool,
        next: usize,
    }
    impl crate::decision::DecisionMaker for Pay {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.accept
        }
        fn decide_options(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(
                ctx.options.len(),
                2,
                "the controller chooses between both opponents"
            );
            let index = self.next % 2;
            self.next += 1;
            vec![index]
        }
    }
    for accept in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let (alice, bob, carol) = (game.players[0].id, game.players[1].id, game.players[2].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let mut pay = Pay { accept, next: 0 };
        for turn in 1..=3 {
            if !game.battlefield.contains(&source) {
                break;
            }
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::BeginningOfUpkeepEvent::new(alice),
                crate::provenance::ProvNodeId::default(),
            );
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(triggers.len(), 1);
            let mut queue = crate::triggers::TriggerQueue::new();
            for trigger in triggers {
                queue.add(trigger);
            }
            crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut pay).unwrap();
            assert_eq!(game.battlefield.contains(&source), accept);
            let tokens: Vec<_> = game
                .battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|o| matches!(o.kind, crate::object::ObjectKind::Token))
                .collect();
            let expected = if accept { turn * (turn + 1) / 2 } else { 0 };
            assert_eq!(tokens.len(), expected);
            if accept {
                assert_eq!(
                    game.counter_count(source, crate::CounterType::Age),
                    turn as u32
                );
                assert_eq!(
                    tokens
                        .iter()
                        .filter(|o| game.controller_of_id(o.id) == Some(bob))
                        .count(),
                    (expected + 1) / 2
                );
                assert_eq!(
                    tokens
                        .iter()
                        .filter(|o| game.controller_of_id(o.id) == Some(carol))
                        .count(),
                    expected / 2
                );
                for token in tokens {
                    assert_eq!(game.current_power(token.id), Some(1));
                    assert_eq!(game.current_toughness(token.id), Some(1));
                    assert!(token.subtypes.contains(&Subtype::Survivor));
                }
            }
        }
    }
}

#[test]
fn cohort_chosen_creature_identity_persists_across_entry_static_and_leave_abilities() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Voranel").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(6,6))
        .parse_text("Flying, trample\nAs this creature enters, choose another creature you control.\nThe chosen creature gets +3/+3 and has flying.\nWhen this creature leaves the battlefield, sacrifice the chosen creature.").unwrap();
    let body = crate::CardDefinitionBuilder::new(CardId::new(), "Zaravin")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    for case in [4, 0, 1, 2, 3] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let enemy = game.create_object_from_definition(&body, bob, Zone::Battlefield);
        let chosen = (case != 0)
            .then(|| game.create_object_from_definition(&body, alice, Zone::Battlefield));
        let hand = game.create_object_from_definition(&card, alice, Zone::Hand);
        let source = game
            .move_object_with_etb_processing(hand, Zone::Battlefield)
            .unwrap()
            .new_id;
        let later = game.create_object_from_definition(&body, alice, Zone::Battlefield);
        assert_eq!(game.current_power(source), Some(6));
        assert_eq!(game.current_power(enemy), Some(2));
        assert_eq!(game.current_power(later), Some(2));
        assert_eq!(game.chosen_object(source).is_some(), chosen.is_some());
        let mut returned_choice = None;
        if let Some(chosen) = chosen {
            assert_eq!(game.current_power(chosen), Some(5));
            assert!(game.current_has_static_ability_id(
                chosen,
                crate::static_abilities::StaticAbilityId::Flying
            ));
            if case == 2 {
                game.set_current_controller(chosen, bob);
                assert_eq!(
                    game.current_power(chosen),
                    Some(5),
                    "the chosen identity survives a control change"
                );
            }
            if case == 3 {
                let hand = game.move_object_by_effect(chosen, Zone::Hand).unwrap();
                let returned = game
                    .move_object_with_etb_processing(hand, Zone::Battlefield)
                    .unwrap()
                    .new_id;
                returned_choice = Some(returned);
                assert_eq!(
                    game.current_power(returned),
                    Some(2),
                    "a new zone incarnation is not the chosen object"
                );
            }
            if case == 4 {
                let remove_type = crate::effects::ApplyContinuousEffect::new(
                    crate::continuous::EffectTarget::Filter(ObjectFilter::specific(chosen)),
                    crate::continuous::Modification::RemoveCardTypes(vec![CardType::Creature]),
                    Until::EndOfTurn,
                );
                let mut ctx = crate::effects::EffectContext::new_default(source, alice);
                crate::effects::execute_effect(&mut game, &Effect::new(remove_type), &mut ctx)
                    .unwrap();
                assert!(!game.current_is_creature(chosen));
                assert!(
                    game.current_has_static_ability_id(
                        chosen,
                        crate::static_abilities::StaticAbilityId::Flying
                    ),
                    "the chosen object's identity survives a type change (CR 700.7)"
                );
            }
        }
        game.take_pending_trigger_events();
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let mut queue = crate::triggers::TriggerQueue::new();
        crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(
            game.stack.len(),
            1,
            "the source leaving must retain its linked trigger"
        );
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.battlefield.contains(&later));
        assert!(game.battlefield.contains(&enemy));
        if let Some(returned) = returned_choice {
            assert!(
                game.battlefield.contains(&returned),
                "the leave trigger cannot sacrifice a new incarnation of the chosen card"
            );
        }
        if let Some(chosen) = chosen {
            assert_eq!(
                game.battlefield.contains(&chosen),
                case == 2,
                "chosen sacrifice case {case}"
            );
            if case == 2 {
                assert_eq!(game.current_power(chosen), Some(2));
            }
        }
    }
}

#[test]
fn cohort_counter_condition_protects_player_and_other_subtype_members() {
    let card=crate::CardDefinitionBuilder::new(CardId::new(),"Zoravin, the Guardian").card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Hero]).power_toughness(PowerToughness::fixed(3,3))
        .parse_text("First strike\nZoravin enters with a shield counter on him.\nAs long as Zoravin has a shield counter on him, you and other Heroes you control have hexproof.").unwrap();
    let hero = crate::CardDefinitionBuilder::new(CardId::new(), "Varanel")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Hero])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let ordinary = crate::CardDefinitionBuilder::new(CardId::new(), "Taravel")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let spell = crate::CardDefinitionBuilder::new(CardId::new(), "Zarevin")
        .card_types(vec![CardType::Instant])
        .build();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let (alice, bob) = (game.players[0].id, game.players[1].id);
    let hand = game.create_object_from_definition(&card, alice, Zone::Hand);
    let source = game
        .move_object_with_etb_processing(hand, Zone::Battlefield)
        .unwrap()
        .new_id;
    assert_eq!(game.counter_count(source, crate::CounterType::Shield), 1);
    let ours = game.create_object_from_definition(&hero, alice, Zone::Battlefield);
    let theirs = game.create_object_from_definition(&hero, bob, Zone::Battlefield);
    let plain = game.create_object_from_definition(&ordinary, alice, Zone::Battlefield);
    let own_spell = game.create_object_from_definition(&spell, alice, Zone::Stack);
    let enemy_spell = game.create_object_from_definition(&spell, bob, Zone::Stack);
    game.stack.push(StackEntry::new(own_spell, alice));
    game.stack.push(StackEntry::new(enemy_spell, bob));
    for enabled in [true, false, true] {
        if !enabled {
            game.remove_counters(source, crate::CounterType::Shield, 1, None, None)
                .unwrap();
        } else if game.counter_count(source, crate::CounterType::Shield) == 0 {
            game.add_counters(source, crate::CounterType::Shield, 1);
        }
        game.update_cant_effects();
        assert_eq!(
            game.can_target_player_from_source(alice, enemy_spell),
            !enabled
        );
        assert!(game.can_target_player_from_source(alice, own_spell));
        assert!(game.can_target_player_from_source(bob, own_spell));
        for (target, protected) in [
            (source, false),
            (ours, enabled),
            (theirs, false),
            (plain, false),
        ] {
            assert_eq!(
                crate::targeting::can_target_object(&game, target, enemy_spell, bob).is_legal(),
                !protected
            );
            assert!(
                crate::targeting::can_target_object(&game, target, own_spell, alice).is_legal()
            );
        }
    }
}

#[test]
fn cohort_looked_permanent_choice_keeps_mana_limit_shield_and_bottom_partition() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Veralyn")
        .card_types(vec![CardType::Planeswalker])
        .parse_text("-3: Look at the top seven cards of your library. You may put a permanent card with mana value 3 or less from among them onto the battlefield with a shield counter on it. Put the rest on the bottom of your library in a random order.")
        .unwrap();
    let ability = card
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(ability),
            _ => None,
        })
        .unwrap();
    struct Pick {
        accept: bool,
        selected: crate::ids::ObjectId,
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
            let legal = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect::<Vec<_>>();
            assert_eq!(
                legal,
                vec![self.selected],
                "only a qualifying permanent among the seven looked cards can enter"
            );
            if self.accept {
                vec![self.selected]
            } else {
                vec![]
            }
        }
    }
    for kind in [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
    ] {
        for accept in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
            game.add_counters(source, crate::CounterType::Loyalty, 5);
            let build = |name: &str, kind, mana| {
                crate::CardDefinitionBuilder::new(CardId::new(), name.to_owned())
                    .card_types(vec![kind])
                    .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                        crate::mana::ManaSymbol::Generic(mana),
                    ]))
                    .power_toughness(PowerToughness::fixed(1, 1))
                    .build()
            };
            let unlooked = game.create_object_from_definition(
                &build("Unlooked", CardType::Artifact, 0),
                alice,
                Zone::Library,
            );
            let selected = game.create_object_from_definition(
                &build("Selected", kind, 3),
                alice,
                Zone::Library,
            );
            game.create_object_from_definition(
                &build("Too costly", CardType::Creature, 4),
                alice,
                Zone::Library,
            );
            for index in 0..5 {
                game.create_object_from_definition(
                    &build(&format!("Nonpermanent {index}"), CardType::Sorcery, 1),
                    alice,
                    Zone::Library,
                );
            }
            let mut pick = Pick { accept, selected };
            game.stack
                .push(StackEntry::ability(source, alice, ability.effects.clone()));
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut pick).unwrap();
            let entered = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.name == "Selected")
                })
                .collect::<Vec<_>>();
            assert_eq!(entered.len(), usize::from(accept));
            if let Some(entered) = entered.first() {
                assert_eq!(game.counter_count(*entered, crate::CounterType::Shield), 1);
            }
            let library = &game.player(alice).unwrap().library;
            assert_eq!(library.len(), 8 - usize::from(accept));
            assert_eq!(
                library.last(),
                Some(&unlooked),
                "the unlooked card stays above the cards put on the bottom"
            );
            assert!(game.player(alice).unwrap().hand.is_empty());
        }
    }
}
