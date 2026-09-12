use super::*;

#[test]
fn cohort_static_goad_tracks_power_controller_and_source_lifetime() {
    let text =
        "Creatures your opponents control with power less than this creature's power are goaded.";
    let source_card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Goad Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text(text)
        .unwrap();
    assert!(source_card.spell_effect.is_none());
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&source_card).join("\n"),
        text
    );
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let [alice, bob, carol] = std::array::from_fn(|i| game.players[i].id);
    let source = game.create_object_from_definition(&source_card, alice, Zone::Battlefield);
    let mut creatures = Vec::new();
    for (owner, power) in [(alice, 1), (bob, 2), (bob, 3), (carol, 4)] {
        let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Creature Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(power, 5))
            .build();
        creatures.push(game.create_object_from_definition(&card, owner, Zone::Battlefield));
    }
    let goaded = |game: &crate::game_state::GameState| {
        creatures
            .iter()
            .map(|id| game.is_goaded(*id))
            .collect::<Vec<_>>()
    };
    assert_eq!(goaded(&game), [false, true, false, false]);
    let mut pump_ctx = crate::effects::EffectContext::new_default(source, alice);
    crate::effects::execute_effect(
        &mut game,
        &Effect::new(crate::effects::ModifyPowerToughnessEffect::pump(
            ChooseSpec::Source,
            2,
            Until::EndOfTurn,
        )),
        &mut pump_ctx,
    )
    .unwrap();
    assert_eq!(goaded(&game), [false, true, true, true]);
    game.object_mut(creatures[1])
        .unwrap()
        .add_counters(crate::object::CounterType::PlusOnePlusOne, 3);
    assert_eq!(goaded(&game), [false, false, true, true]);
    game.set_current_controller(source, bob);
    assert_eq!(goaded(&game), [true, false, false, true]);
    assert_eq!(
        game.active_goaders_for(creatures[0]),
        [bob].into_iter().collect()
    );
    let source_abilities = game.object(source).unwrap().abilities.clone();
    game.object_mut(source).unwrap().abilities = Default::default();
    assert_eq!(goaded(&game), [false, false, false, false]);
    game.object_mut(source).unwrap().abilities = source_abilities;
    assert_eq!(goaded(&game), [true, false, false, true]);
    game.move_object_by_effect(source, Zone::Graveyard);
    assert_eq!(goaded(&game), [false, false, false, false]);
}

#[test]
fn cohort_prevention_captures_the_original_object_or_player_for_repeatable_payments() {
    let text = "Prevent the next X damage that would be dealt to any target this turn. Until end of turn, you may pay {1} any time you could cast an instant. If you do, prevent the next 1 damage that would be dealt to that permanent or player this turn.";
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Prevention Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(text)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&spell).join("\n"),
        text
    );
    for protect_player in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let creature =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Protected Creature")
                .card_types(vec![CardType::Creature])
                .build();
        let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
        let resolved = if protect_player {
            crate::effects::ResolvedTarget::Player(bob)
        } else {
            crate::effects::ResolvedTarget::Object(target)
        };
        let mut ctx =
            crate::effects::EffectContext::new_default(source, alice).with_targets(vec![resolved]);
        ctx.x_value = Some(2);
        ctx.snapshot_targets(&game);
        for effect in spell
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects()
        {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.effect_store.repeatable_mana_payment_actions.len(), 1);
        assert_eq!(
            game.effect_store.repeatable_mana_payment_actions[0].targets,
            vec![resolved]
        );
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Blue, 2);
        let action = crate::special_actions::SpecialAction::PerformRepeatableManaPaymentAction {
            action_index: 0,
        };
        let mut choices = crate::decision::SelectFirstDecisionMaker;
        for _ in 0..2 {
            crate::special_actions::perform(action.clone(), &mut game, alice, &mut choices)
                .unwrap();
        }
        let damage_source = game.new_object_id();
        let damage_target = if protect_player {
            crate::events::DamageTarget::Player(bob)
        } else {
            crate::events::DamageTarget::Object(target)
        };
        let (remaining, _) = crate::events::processing::process_damage_with_event(
            &mut game,
            damage_source,
            damage_target,
            5,
            false,
            crate::events::cause::EventCause::effect(),
        );
        assert_eq!(remaining, 1);
        game.turn.turn_number += 1;
        assert!(crate::special_actions::can_perform_check(&action, &game, alice).is_err());
    }
}

#[test]
fn cohort_participant_loot_compares_only_the_discarded_cards_and_accepts_ties() {
    let text = "Whenever this creature attacks, you and defending player each draw a card, then discard a card. Put two +1/+1 counters on this creature if you discarded the card with the greatest mana value among those cards or tied for greatest.";
    let (card, trace) = ironsmith_compiler::parse_trace::capture(|| {
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Loot Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(text)
            .unwrap()
    });
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&card).join("\n"),
        text,
        "{}",
        trace.render()
    );
    for (your_cost, their_cost, expected) in [(5, 3, 2), (5, 5, 2), (3, 5, 0)] {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let [alice, bob, carol] = std::array::from_fn(|i| game.players[i].id);
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        for (player, cost) in [(alice, your_cost), (bob, their_cost), (carol, 9)] {
            let library_card =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Discard Probe")
                    .card_types(vec![CardType::Sorcery])
                    .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                        crate::mana::ManaSymbol::Generic(cost),
                    ]))
                    .build();
            game.create_object_from_definition(&library_card, player, Zone::Hand);
            let drawn_card =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Kept Draw")
                    .card_types(vec![CardType::Sorcery])
                    .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                        crate::mana::ManaSymbol::Generic(9),
                    ]))
                    .build();
            game.create_object_from_definition(&drawn_card, player, Zone::Library);
        }
        let trigger = card
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                crate::ability::AbilityKind::Triggered(trigger) => Some(trigger),
                _ => None,
            })
            .unwrap();
        let mut ctx =
            crate::effects::EffectContext::new_default(source, alice).with_defending_player(bob);
        for effect in trigger.effects.flattened_default_effects() {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(
            game.counter_count(source, crate::object::CounterType::PlusOnePlusOne),
            expected
        );
        for player in [alice, bob] {
            assert_eq!(game.player(player).unwrap().graveyard.len(), 1);
            assert_eq!(game.player(player).unwrap().hand.len(), 1);
            assert!(game.player(player).unwrap().library.is_empty());
        }
        assert_eq!(game.player(carol).unwrap().library.len(), 1);
        assert!(game.player(carol).unwrap().graveyard.is_empty());
    }
}

#[test]
fn cohort_vote_result_ids_preserve_compact_shared_actions() {
    let text = "Flying\nCouncil's dilemma — When this creature enters, starting with you, each player votes for feather or quill. Put a +1/+1 counter on this creature for each feather vote and draw a card for each quill vote. For each card drawn this way, discard a card.";
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Messenger Jays")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&card).join("\n"),
        text.replace("and draw a card", "and you draw a card")
    );
}

#[test]
fn cohort_static_goad_survives_in_last_known_state_for_death_trigger_filters() {
    let source_card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Goad Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text("Creatures your opponents control with power less than this creature's power are goaded.")
        .unwrap();
    let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Creature Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&source_card, alice, Zone::Battlefield);
    let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(target).unwrap(),
        &game,
    );
    assert_eq!(snapshot.goaded, Some(true));
    game.move_object_by_effect(target, Zone::Graveyard);
    game.move_object_by_effect(source, Zone::Graveyard);
    assert!(!game.is_goaded(target));
    let mut filter = ObjectFilter::creature();
    filter.goaded = true;
    let ctx = crate::triggers::TriggerContext::new(
        source,
        alice,
        game.filter_context_for(alice, Some(source)),
        &game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            target,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(crate::triggers::Trigger::dies(filter).matches(&event, &ctx));
}

#[test]
fn cohort_simultaneous_discards_collect_all_choices_before_cards_move() {
    struct CheckChoices {
        choices: usize,
    }
    impl crate::decision::DecisionMaker for CheckChoices {
        fn decide_objects(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<crate::ids::ObjectId> {
            for player in &game.players {
                assert_eq!(
                    player.hand.len(),
                    2,
                    "every choice sees the same pre-discard state"
                );
                assert!(player.graveyard.is_empty());
            }
            self.choices += 1;
            vec![
                ctx.candidates
                    .iter()
                    .find(|candidate| candidate.legal)
                    .unwrap()
                    .id,
            ]
        }
    }
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Discard Probe")
        .card_types(vec![CardType::Sorcery])
        .build();
    let source = game.create_object_from_definition(&card, alice, Zone::Stack);
    let players = game
        .players
        .iter()
        .map(|player| player.id)
        .collect::<Vec<_>>();
    for player in players {
        for _ in 0..2 {
            game.create_object_from_definition(&card, player, Zone::Hand);
        }
    }
    let effect = Effect::new(crate::effects::ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![Effect::new(crate::effects::DiscardEffect::new(
            1,
            PlayerFilter::IteratedPlayer,
            false,
        ))],
    ));
    let mut choices = CheckChoices { choices: 0 };
    {
        let mut ctx = crate::effects::EffectContext::new(source, alice, &mut choices);
        crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    }
    assert_eq!(choices.choices, 2);
    for player in &game.players {
        assert_eq!(player.hand.len(), 1);
        assert_eq!(player.graveyard.len(), 1);
    }
}
