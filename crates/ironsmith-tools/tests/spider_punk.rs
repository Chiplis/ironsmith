use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::ids::CardId;
use ironsmith::object::CounterType;
use ironsmith::{CardType, GameState, PlayerId, Subtype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Spider-Punk",
    )
    .unwrap()
    .remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{:?}",
        snapshot.parse_error
    );
    assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented && snapshot.parse_error.is_none());
    assert!(
        snapshot.similarity_score >= 0.99,
        "{}: {:?}",
        snapshot.similarity_score,
        snapshot.compiled_text
    );
}

#[test]
fn riot_counter_is_present_before_entry_finishes_and_is_granted_to_other_spiders() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let entered = game
        .move_object_with_etb_processing_with_dm(
            source,
            Zone::Battlefield,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap()
        .new_id;
    assert_eq!(
        game.object(entered)
            .unwrap()
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        1,
        "riot replaces entry; it must not wait for a trigger to resolve"
    );
    for (controller, subtype, expected) in [
        (alice, Subtype::Spider, 1),
        (bob, Subtype::Spider, 0),
        (alice, Subtype::Human, 0),
    ] {
        let card = CardDefinitionBuilder::new(CardId::new(), "Riot recipient")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![subtype])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
            .build();
        let hand = game.create_object_from_definition(&card, controller, Zone::Hand);
        let creature = game
            .move_object_with_etb_processing_with_dm(
                hand,
                Zone::Battlefield,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap()
            .new_id;
        assert_eq!(
            game.object(creature)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            expected
        );
    }
}

#[test]
fn riot_haste_choice_is_immediate_and_survives_cleanup() {
    struct ChooseLast;
    impl ironsmith::decision::DecisionMaker for ChooseLast {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .rev()
                .find(|option| option.legal)
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let entered = game
        .move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut ChooseLast)
        .unwrap()
        .new_id;
    assert_eq!(
        game.object(entered)
            .unwrap()
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        0
    );
    assert!(game.object_has_static_ability_id(
        entered,
        ironsmith::static_abilities::StaticAbilityId::Haste
    ));
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert!(
        game.object_has_static_ability_id(
            entered,
            ironsmith::static_abilities::StaticAbilityId::Haste
        ),
        "riot's haste has no turn duration"
    );
}

fn counter(
    game: &mut GameState,
    controller: PlayerId,
    source: ironsmith::ObjectId,
    target: ironsmith::ObjectId,
) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(source, controller, &mut dm);
    ironsmith::effects::execute_effect(
        game,
        &ironsmith::effect::Effect::new(ironsmith::effects::CounterEffect::new(
            ironsmith::target::ChooseSpec::SpecificObject(target),
        )),
        &mut ctx,
    )
    .unwrap();
}

#[test]
fn battlefield_counter_restriction_does_not_protect_its_own_spell() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let spell = game.create_object_from_definition(&definition(), alice, Zone::Stack);
    game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, alice));
    let counter_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Counter source")
            .card_types(vec![CardType::Instant])
            .build(),
        bob,
        Zone::Stack,
    );
    game.update_cant_effects();
    counter(&mut game, bob, counter_source, spell);
    assert!(
        game.stack.is_empty(),
        "a battlefield static restriction is inactive on the stack"
    );
}

#[test]
fn both_players_spells_and_abilities_are_protected_only_while_source_is_present() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for controller in [alice, bob] {
        for kind in 0..3 {
            let is_ability = kind != 0;
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source =
                game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
            let card = CardDefinitionBuilder::new(CardId::new(), "Stack object source")
                .card_types(vec![CardType::Artifact])
                .build();
            let object = game.create_object_from_definition(
                &card,
                controller,
                if is_ability {
                    Zone::Battlefield
                } else {
                    Zone::Stack
                },
            );
            let mut entry = if is_ability {
                ironsmith::game_state::StackEntry::ability(
                    object,
                    controller,
                    vec![ironsmith::effect::Effect::draw(1)],
                )
            } else {
                ironsmith::game_state::StackEntry::new(object, controller)
            };
            if kind == 2 {
                entry.triggering_event = Some(ironsmith::triggers::TriggerEvent::new(
                    ironsmith::events::LifeGainEvent::new(controller, 1),
                    game.provenance_graph_mut()
                        .alloc_root_event(ironsmith::events::EventKind::LifeGain),
                ));
            }
            game.push_to_stack(entry);
            game.update_cant_effects();
            counter(&mut game, bob, source, object);
            assert_eq!(
                game.stack.len(),
                1,
                "controller={controller:?}, ability={is_ability}"
            );
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            game.update_cant_effects();
            counter(&mut game, bob, object, object);
            assert!(
                game.stack.is_empty(),
                "protection must end when its source leaves"
            );
        }
    }
}

#[test]
fn damage_ignores_prevention_while_source_is_present_and_prevention_resumes_afterward() {
    use ironsmith::effect::{Effect, Until};
    use ironsmith::target::{ChooseSpec, PlayerFilter};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for damaged in [alice, bob] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
        let damage_source = game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::new(), "Damage source")
                .card_types(vec![CardType::Instant])
                .build(),
            bob,
            Zone::Stack,
        );
        let target = ChooseSpec::Player(PlayerFilter::Specific(damaged));
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ironsmith::effects::EffectContext::new(damage_source, bob, &mut dm);
        ironsmith::effects::execute_effect(
            &mut game,
            &Effect::prevent_all_damage_to_target(target.clone(), Until::EndOfTurn),
            &mut ctx,
        )
        .unwrap();
        game.update_cant_effects();
        ironsmith::effects::execute_effect(
            &mut game,
            &Effect::deal_damage(3, target.clone()),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(game.player(damaged).unwrap().life, 17);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        game.update_cant_effects();
        ironsmith::effects::execute_effect(&mut game, &Effect::deal_damage(3, target), &mut ctx)
            .unwrap();
        assert_eq!(
            game.player(damaged).unwrap().life,
            17,
            "an unconsumed prevention shield applies after the prohibition ends"
        );
    }
}

#[test]
fn printed_and_granted_riot_are_separate_entry_choices() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition = definition();
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let entered = game
        .move_object_with_etb_processing_with_dm(
            hand,
            Zone::Battlefield,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap()
        .new_id;
    assert_eq!(
        game.object(entered)
            .unwrap()
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        2,
        "each riot instance must offer a separate replacement choice"
    );
}

#[test]
fn runtime_builder_riot_matches_compiler_entry_and_duration_semantics() {
    struct PickMode(bool);
    impl ironsmith::decision::DecisionMaker for PickMode {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            let choices: Vec<_> = ctx.options.iter().filter(|option| option.legal).collect();
            let choice = if self.0 {
                choices.last()
            } else {
                choices.first()
            };
            choice.map(|option| vec![option.index]).unwrap_or_default()
        }
    }
    let alice = PlayerId::from_index(0);
    for haste in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = CardDefinitionBuilder::new(CardId::new(), "Riot builder probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
            .riot()
            .build();
        let hand = game.create_object_from_definition(&card, alice, Zone::Hand);
        let creature = game
            .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut PickMode(haste))
            .unwrap()
            .new_id;
        assert_eq!(
            game.object(creature)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            if haste { 0 } else { 1 }
        );
        assert_eq!(
            game.object_has_static_ability_id(
                creature,
                ironsmith::static_abilities::StaticAbilityId::Haste
            ),
            haste
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(
            game.object_has_static_ability_id(
                creature,
                ironsmith::static_abilities::StaticAbilityId::Haste
            ),
            haste
        );
    }
}
