use super::*;
use crate::ability::AbilityKind;
use crate::card::PowerToughness;
use crate::game_state::{GameState, Target};
use crate::ids::CardId;

fn creature(name: &str, power: i32) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, power))
        .build()
}

#[test]
fn cohort_labeled_celebration_affects_only_source_and_counts_entry_history() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Celebrating Visitor")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Celebration — This creature gets +1/+1 and has trample as long as two or more nonland permanents entered the battlefield under your control this turn.").unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let (alice, bob) = (game.players[0].id, game.players[1].id);
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let other = game.create_object_from_definition(&creature("Other", 2), alice, Zone::Battlefield);
    let opponent =
        game.create_object_from_definition(&creature("Opponent", 2), bob, Zone::Battlefield);
    game.turn_store.turn_history.clear_for_new_turn();
    let check = |game: &GameState, enabled| {
        assert_eq!(
            game.current_characteristics(source).unwrap().power,
            Some(if enabled { 3 } else { 2 })
        );
        assert_eq!(
            game.current_has_static_ability_id(
                source,
                crate::static_abilities::StaticAbilityId::Trample
            ),
            enabled
        );
        for id in [other, opponent] {
            assert_eq!(game.current_characteristics(id).unwrap().power, Some(2));
            assert!(!game.current_has_static_ability_id(
                id,
                crate::static_abilities::StaticAbilityId::Trample
            ));
        }
    };
    check(&game, false);
    let land = crate::CardDefinitionBuilder::new(CardId::new(), "Land")
        .card_types(vec![CardType::Land])
        .build();
    let id = game.create_object_from_definition(&land, alice, Zone::Hand);
    game.move_object_by_effect(id, Zone::Battlefield).unwrap();
    let id = game.create_object_from_definition(&creature("Opponent entrant", 1), bob, Zone::Hand);
    game.move_object_by_effect(id, Zone::Battlefield).unwrap();
    check(&game, false);
    let first =
        game.create_object_from_definition(&creature("First entrant", 1), alice, Zone::Hand);
    let first = game
        .move_object_by_effect(first, Zone::Battlefield)
        .unwrap();
    check(&game, false);
    game.move_object_by_effect(first, Zone::Graveyard).unwrap();
    let second =
        game.create_object_from_definition(&creature("Second entrant", 1), alice, Zone::Hand);
    let second = game
        .move_object_by_effect(second, Zone::Battlefield)
        .unwrap();
    check(&game, true);
    game.move_object_by_effect(second, Zone::Graveyard).unwrap();
    check(&game, true);
    game.turn_store.turn_history.clear_for_new_turn();
    check(&game, false);
}

#[test]
fn cohort_greatest_power_target_includes_ties_and_rechecks_on_resolution() {
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Zorvik")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile target creature with the greatest power among creatures on the battlefield.",
        )
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&card),
        ["Exile target creature with the greatest power among creatures on the battlefield."]
    );
    for became_smaller in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Stack);
        let chosen =
            game.create_object_from_definition(&creature("Chosen", 5), alice, Zone::Battlefield);
        let tied = game.create_object_from_definition(&creature("Tied", 5), bob, Zone::Battlefield);
        let smaller =
            game.create_object_from_definition(&creature("Smaller", 4), bob, Zone::Battlefield);
        game.create_object_from_definition(&creature("In graveyard", 20), alice, Zone::Graveyard);
        let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
            &game,
            card.spell_effect.as_ref().unwrap(),
            alice,
            Some(source),
            None,
        );
        assert_eq!(requirements.len(), 1);
        assert!(
            requirements[0]
                .legal_targets
                .contains(&Target::Object(chosen))
        );
        assert!(
            requirements[0]
                .legal_targets
                .contains(&Target::Object(tied))
        );
        assert!(
            !requirements[0]
                .legal_targets
                .contains(&Target::Object(smaller))
        );
        if became_smaller {
            game.object_mut(tied)
                .unwrap()
                .counters
                .insert(crate::CounterType::PlusOnePlusOne, 1);
        }
        game.stack.push(
            crate::game_state::StackEntry::new(source, alice)
                .with_targets(vec![Target::Object(chosen)])
                .with_target_assignments(vec![crate::game_state::TargetAssignment {
                    spec: requirements[0].spec.clone(),
                    range: 0..1,
                }]),
        );
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.object(chosen).is_some(), became_smaller);
        assert!(game.object(tied).is_some());
    }
}

#[test]
fn cohort_random_discard_returns_only_creature_unless_any_player_pays() {
    struct Pay {
        payer: Option<crate::ids::PlayerId>,
        asked: Vec<crate::ids::PlayerId>,
    }
    impl crate::decision::DecisionMaker for Pay {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.asked.push(ctx.player);
            self.payer == Some(ctx.player)
        }
        fn decide_objects(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<crate::ids::ObjectId> {
            panic!("random discard must not ask a player to select the card");
        }
    }
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Merovin")
        .card_types(vec![CardType::Enchantment])
        .parse_text("At the beginning of your upkeep, discard a card at random. If you discard a creature card this way, return it from your graveyard to the battlefield unless any player pays 5 life.").unwrap();
    for kind in [None, Some(CardType::Creature), Some(CardType::Land)] {
        for payer_index in [None, Some(0), Some(1), Some(2)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            let alice = game.players[0].id;
            let _source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
            let discarded = kind.map(|kind| {
                let def = crate::CardDefinitionBuilder::new(CardId::new(), "Discarded candidate")
                    .card_types(vec![kind])
                    .power_toughness(PowerToughness::fixed(2, 2))
                    .build();
                let id = game.create_object_from_definition(&def, alice, Zone::Hand);
                game.object(id).unwrap().stable_id
            });
            let other = game.create_object_from_definition(
                &creature("Already in graveyard", 2),
                alice,
                Zone::Graveyard,
            );
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
            let mut pay = Pay {
                payer: payer_index.map(|i| game.players[i].id),
                asked: Vec::new(),
            };
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut pay).unwrap();
            assert!(game.player(alice).unwrap().hand.is_empty());
            assert_eq!(game.object(other).unwrap().zone, Zone::Graveyard);
            let creature_discarded = kind == Some(CardType::Creature);
            if let Some(stable) = discarded {
                let id = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(
                    game.object(id).unwrap().zone,
                    if creature_discarded && payer_index.is_none() {
                        Zone::Battlefield
                    } else {
                        Zone::Graveyard
                    }
                );
            }
            assert_eq!(
                pay.asked.len(),
                if creature_discarded {
                    payer_index.map_or(3, |i| i + 1)
                } else {
                    0
                }
            );
            for (i, player) in game.players.iter().enumerate() {
                assert_eq!(
                    player.life,
                    if creature_discarded && payer_index == Some(i) {
                        15
                    } else {
                        20
                    }
                );
            }
        }
    }
}

#[test]
fn cohort_next_qualifying_spell_copy_uses_current_power_and_is_one_shot() {
    use crate::effects::{EffectContext, execute_effect};
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Myravon")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("{1}, {T}: When you next cast an instant or sorcery spell with mana value less than or equal to this creature's power this turn, copy that spell. You may choose new targets for the copy.").unwrap();
    let ability = card
        .abilities
        .iter()
        .find_map(|a| match &a.kind {
            AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    for expires in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (alice, bob) = (game.players[0].id, game.players[1].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        for _ in 0..2 {
            let mut ctx = EffectContext::new_default(source, alice);
            for effect in ability.effects.flattened_default_effects() {
                execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        assert_eq!(game.effect_store.delayed_triggers.len(), 2);
        let cast = |game: &mut GameState, kind, caster, mana| {
            let spell = crate::CardDefinitionBuilder::new(CardId::new(), "Candidate")
                .card_types(vec![kind])
                .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                    crate::mana::ManaSymbol::Generic(mana),
                ]))
                .build();
            let id = game.create_object_from_definition(&spell, caster, Zone::Stack);
            game.stack
                .push(crate::game_state::StackEntry::new(id, caster));
            let snapshot =
                crate::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(), game);
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::SpellCastEvent::new_with_snapshot(id, caster, Zone::Hand, snapshot),
                crate::provenance::ProvNodeId::default(),
            );
            (id, crate::triggers::check_delayed_triggers(game, &event))
        };
        for (kind, caster, mana) in [
            (CardType::Creature, alice, 1),
            (CardType::Instant, bob, 1),
            (CardType::Sorcery, alice, 4),
        ] {
            let (_, triggered) = cast(&mut game, kind, caster, mana);
            assert!(triggered.is_empty());
            assert_eq!(game.effect_store.delayed_triggers.len(), 2);
            game.stack.clear();
        }
        game.add_counters(source, crate::CounterType::PlusOnePlusOne, 2);
        if expires {
            game.turn.turn_number += 1;
        }
        let (original, triggered) = cast(&mut game, CardType::Instant, alice, 4);
        assert_eq!(triggered.len(), if expires { 0 } else { 2 });
        assert!(game.effect_store.delayed_triggers.is_empty());
        if !expires {
            // Resolve the two independently scheduled abilities. Each places
            // exactly one copy on the stack above the original spell.
            for trigger in triggered {
                let mut queue = crate::triggers::TriggerQueue::new();
                queue.add(trigger);
                crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                crate::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
            assert_eq!(game.stack.len(), 3);
            assert_eq!(game.stack[0].object_id, original);
            for entry in game.stack.iter().skip(1) {
                assert_ne!(entry.object_id, original);
                assert_eq!(entry.controller, alice);
                assert_eq!(
                    game.object(entry.object_id).unwrap().card_types,
                    vec![CardType::Instant]
                );
            }
        }
        let (_, triggered) = cast(&mut game, CardType::Sorcery, alice, 1);
        assert!(triggered.is_empty());
    }
}

#[test]
fn cohort_hand_cast_consult_returns_exiled_pool_to_casters_library_bottom() {
    struct MayCast {
        caster: crate::ids::PlayerId,
        accept: bool,
    }
    impl crate::decision::DecisionMaker for MayCast {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            assert_eq!(ctx.player, self.caster);
            self.accept
        }
    }
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Varelis")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever a player casts a spell from their hand, that player exiles it, then exiles cards from the top of their library until they exile a card that shares a card type with it. That player may cast that card without paying its mana cost. Then they put all cards exiled with this enchantment on the bottom of their library in a random order.").unwrap();
    let mut observed_orders = std::collections::HashSet::new();
    for seed in 1..=4 {
        for has_match in [false, true] {
            for accept in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.set_random_seed(seed);
                let (alice, bob) = (game.players[0].id, game.players[1].id);
                let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
                let candidate = |name: &str, kind| {
                    crate::CardDefinitionBuilder::new(CardId::new(), name)
                        .card_types(vec![kind])
                        .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                            crate::mana::ManaSymbol::Generic(7),
                        ]))
                        .build()
                };
                let original = game.create_object_from_definition(
                    &candidate("Original", CardType::Instant),
                    bob,
                    Zone::Stack,
                );
                let original_stable = game.object(original).unwrap().stable_id;
                game.stack
                    .push(crate::game_state::StackEntry::new(original, bob));
                game.stack.push(crate::game_state::StackEntry::ability(
                    original,
                    bob,
                    Vec::<Effect>::new(),
                ));
                let own_library = game.create_object_from_definition(
                    &candidate("Own library", CardType::Land),
                    alice,
                    Zone::Library,
                );
                let unrelated = game.create_object_from_definition(
                    &candidate("Unrelated exile", CardType::Instant),
                    bob,
                    Zone::Exile,
                );
                let unseen = game.create_object_from_definition(
                    &candidate("Unseen", CardType::Artifact),
                    bob,
                    Zone::Library,
                );
                let matching = has_match.then(|| {
                    game.create_object_from_definition(
                        &candidate("Matching", CardType::Instant),
                        bob,
                        Zone::Library,
                    )
                });
                let match_stable = matching.map(|id| game.object(id).unwrap().stable_id);
                game.create_object_from_definition(
                    &candidate("Nonmatching", CardType::Sorcery),
                    bob,
                    Zone::Library,
                );
                game.create_object_from_definition(
                    &candidate("Top land", CardType::Land),
                    bob,
                    Zone::Library,
                );
                let snapshot = crate::snapshot::ObjectSnapshot::from_object(
                    game.object(original).unwrap(),
                    &game,
                );
                for origin in [Zone::Exile, Zone::Graveyard] {
                    let event = crate::triggers::TriggerEvent::new_with_provenance(
                        crate::events::SpellCastEvent::new_with_snapshot(
                            original,
                            bob,
                            origin,
                            snapshot.clone(),
                        ),
                        crate::provenance::ProvNodeId::default(),
                    );
                    assert!(crate::triggers::check_triggers(&game, &event).is_empty());
                }
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::SpellCastEvent::new_with_snapshot(
                        original,
                        bob,
                        Zone::Hand,
                        snapshot,
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
                crate::game_loop::resolve_stack_entry_with(
                    &mut game,
                    &mut MayCast {
                        caster: bob,
                        accept,
                    },
                )
                .unwrap();
                assert_eq!(game.object(unrelated).unwrap().zone, Zone::Exile);
                assert_eq!(game.player(alice).unwrap().library, vec![own_library]);
                let library = &game.player(bob).unwrap().library;
                assert_eq!(library.len(), if has_match && !accept { 5 } else { 4 });
                assert!(
                    library
                        .iter()
                        .any(|id| game.object(*id).unwrap().stable_id == original_stable)
                );
                if has_match {
                    assert_eq!(
                        library.last(),
                        Some(&unseen),
                        "returned cards must stay below the unexposed library"
                    );
                }
                assert_eq!(
                    game.stack
                        .iter()
                        .filter(|entry| entry.is_ability && entry.object_id == original)
                        .count(),
                    1,
                    "an ability survives its source spell leaving the stack"
                );
                let spells = game
                    .stack
                    .iter()
                    .filter(|entry| !entry.is_ability)
                    .collect::<Vec<_>>();
                assert_eq!(
                    spells.len(),
                    usize::from(has_match && accept),
                    "match={has_match} accept={accept}"
                );
                if has_match && accept {
                    assert_eq!(spells[0].controller, bob);
                    assert_eq!(
                        game.object(spells[0].object_id).unwrap().stable_id,
                        match_stable.unwrap()
                    );
                    assert_eq!(game.player(bob).unwrap().mana_pool.total(), 0);
                }
                assert!(game.get_exiled_with_source_links(source).iter().all(|id| {
                    game.object(*id)
                        .is_none_or(|object| object.zone != Zone::Exile)
                }));
                if has_match && !accept {
                    observed_orders.insert(
                        library
                            .iter()
                            .map(|id| game.object(*id).unwrap().name.to_string())
                            .collect::<Vec<_>>(),
                    );
                }
            }
        }
    }
    assert!(
        observed_orders.len() > 1,
        "random bottom ordering must use the game's random stream"
    );
}

#[test]
fn cohort_activated_source_characteristics_and_counters_keep_authored_short_name() {
    let text = "{1}, {T}: When you next cast an instant or sorcery spell with mana value less than or equal to Nyvora's power this turn, copy that spell. You may choose new targets for the copy.\n{4}{R}: Put two +1/+1 counters on Nyvora.";
    let card = crate::CardDefinitionBuilder::new(CardId::new(), "Nyvora Silverbranch")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(text)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&card).join("\n"),
        text
    );
}
