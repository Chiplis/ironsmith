use super::*;
const TEXT: &str = "When this creature enters, draw a card if you control a creature with a counter on it. If you don't draw a card this way, put a +1/+1 counter on this creature.";
#[test]
fn conditional_draw_fallback_uses_actual_draw_result() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Trade Route Envoy")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .unwrap();
    for counter_location in 0..5 {
        for available in [false, true] {
            for replacement_kind in 0..4 {
                let mut game =
                    crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let bob = game.players[1].id;
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let creature =
                    crate::card::CardBuilder::new(crate::ids::CardId::new(), "Counter creature")
                        .card_types(vec![CardType::Creature])
                        .build();
                let artifact =
                    crate::card::CardBuilder::new(crate::ids::CardId::new(), "Counter artifact")
                        .card_types(vec![CardType::Artifact])
                        .build();
                let your_creature =
                    game.create_object_from_card(&creature, alice, Zone::Battlefield);
                let opposing_creature =
                    game.create_object_from_card(&creature, bob, Zone::Battlefield);
                let your_artifact =
                    game.create_object_from_card(&artifact, alice, Zone::Battlefield);
                if available {
                    game.create_object_from_card(&artifact, alice, Zone::Library);
                }
                game.create_object_from_card(&artifact, bob, Zone::Library);
                let mut ctx = crate::effects::EffectContext::new_default(source, alice);
                if counter_location > 0 {
                    let target = [your_creature, opposing_creature, your_artifact, source]
                        [counter_location - 1];
                    crate::effects::execute_effect(
                        &mut game,
                        &Effect::put_counters(
                            CounterType::Charge,
                            1,
                            ChooseSpec::SpecificObject(target),
                        ),
                        &mut ctx,
                    )
                    .unwrap();
                }
                if replacement_kind > 0 {
                    let effects = match replacement_kind {
                        1 => vec![Effect::gain_life(2)],
                        2 => vec![Effect::target_draws(1, PlayerFilter::Specific(bob))],
                        _ => vec![Effect::target_draws(2, PlayerFilter::You)],
                    };
                    let replacement = crate::effects::RegisterDrawReplacementEffect::new(
                        PlayerFilter::You,
                        effects,
                        crate::effects::ReplacementApplyMode::OneShot,
                    );
                    crate::effects::execute_effect(&mut game, &Effect::new(replacement), &mut ctx)
                        .unwrap();
                }
                for effect in &triggered.effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
                let eligible = matches!(counter_location, 1 | 4);
                let drew = eligible && available && matches!(replacement_kind, 0 | 3);
                assert_eq!(game.player(alice).unwrap().hand.len(), usize::from(drew));
                assert_eq!(
                    game.counter_count(source, CounterType::PlusOnePlusOne),
                    u32::from(!drew),
                    "counter={counter_location}, available={available}, replacement_kind={replacement_kind}"
                );
                assert_eq!(
                    game.player(alice).unwrap().life,
                    20 + if eligible && replacement_kind == 1 {
                        2
                    } else {
                        0
                    }
                );
                assert_eq!(
                    game.player(bob).unwrap().hand.len(),
                    usize::from(eligible && replacement_kind == 2)
                );
                for other in [your_creature, opposing_creature, your_artifact] {
                    assert_eq!(game.counter_count(other, CounterType::PlusOnePlusOne), 0);
                }
            }
        }
    }
}
#[test]
fn conditional_draw_fallback_preserves_failed_draw_and_source_noun() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Trade Route Envoy")
            .card_types(vec![CardType::Creature])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn source_counter_surface_preserves_remove_then_draw_or_mana() {
    let text = "At the beginning of your first main phase, remove all flood counters from this enchantment. If no counters were removed this way, put a flood counter on this enchantment and draw a card. Otherwise, add {C}{G}{U}.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Bounty of the Luxa")
            .card_types(vec![CardType::Enchantment])
            .parse_text(text)
            .unwrap();
    for initial in [0, 1, 3] {
        for available in [false, true] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.add_counters(source, CounterType::Flood, initial);
            if available {
                game.create_object_from_definition(&definition, alice, Zone::Library);
            }
            let mut ctx = crate::effects::EffectContext::new_default(source, alice);
            let triggered = definition
                .abilities
                .iter()
                .find_map(|a| match &a.kind {
                    AbilityKind::Triggered(t) => Some(t),
                    _ => None,
                })
                .unwrap();
            for effect in &triggered.effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            assert_eq!(
                game.counter_count(source, CounterType::Flood),
                u32::from(initial == 0)
            );
            assert_eq!(
                game.player(alice).unwrap().hand.len(),
                usize::from(initial == 0 && available)
            );
            assert_eq!(
                game.player(alice).unwrap().mana_pool.total(),
                if initial == 0 { 0 } else { 3 }
            );
        }
    }
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}

#[test]
fn source_counter_surface_preserves_discarded_creature_count() {
    struct SelectCreatures(usize);
    impl crate::decision::DecisionMaker for SelectCreatures {
        fn decide_objects(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<crate::ids::ObjectId> {
            let legal = ctx
                .candidates
                .iter()
                .filter(|c| c.legal)
                .collect::<Vec<_>>();
            assert!(
                legal
                    .iter()
                    .all(|c| game.current_has_card_type(c.id, CardType::Creature))
            );
            legal.into_iter().take(self.0).map(|c| c.id).collect()
        }
    }
    let text = "When this creature enters, discard any number of creature cards. For each card discarded this way, put two +1/+1 counters on this creature.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Mind Maggots")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .unwrap();
    for count in 0..=3 {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let hand: Vec<_> = (0..3)
            .map(|_| game.create_object_from_definition(&definition, alice, Zone::Hand))
            .collect();
        let artifact = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Noncreature")
            .card_types(vec![CardType::Artifact])
            .build();
        let other = game.create_object_from_card(&artifact, alice, Zone::Hand);
        let mut decisions = SelectCreatures(count);
        let mut ctx = crate::effects::EffectContext::new(source, alice, &mut decisions);
        let triggered = definition
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                AbilityKind::Triggered(t) => Some(t),
                _ => None,
            })
            .unwrap();
        for effect in &triggered.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(
            game.counter_count(source, CounterType::PlusOnePlusOne),
            2 * count as u32
        );
        assert_eq!(game.player(alice).unwrap().hand.len(), 4 - count);
        assert!(game.player(alice).unwrap().hand.contains(&other));
        for id in hand.into_iter().skip(count) {
            assert!(game.player(alice).unwrap().hand.contains(&id));
        }
    }
}
