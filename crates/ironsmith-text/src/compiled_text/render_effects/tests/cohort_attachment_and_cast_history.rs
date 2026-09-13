use super::*;

struct OptionalSequence {
    accept: bool,
    decisions: usize,
}
impl crate::decision::DecisionMaker for OptionalSequence {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.decisions += 1;
        self.accept
    }
}

fn resolve_event(
    game: &mut crate::game_state::GameState,
    event: &crate::triggers::TriggerEvent,
) -> usize {
    let triggers = crate::triggers::check_triggers(game, event);
    let count = triggers.len();
    for trigger in triggers {
        let mut ctx =
            crate::effects::EffectContext::new_default(trigger.source, trigger.controller);
        ctx.triggering_event = Some(trigger.triggering_event);
        for effect in trigger.ability.effects.flattened_default_effects() {
            crate::effects::execute_effect(game, effect, &mut ctx).unwrap();
        }
    }
    count
}

#[test]
fn cohort_optional_return_then_attach_is_one_choice_and_keeps_entering_creature() {
    let equipment = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Returning Blade")
        .card_types(vec![CardType::Artifact]).subtypes(vec![crate::types::Subtype::Equipment])
        .parse_text("Whenever a 1/1 creature you control enters, you may return this card from your graveyard to the battlefield, then attach it to that creature.").unwrap();
    for (accept, leaves) in [(false, false), (true, false), (true, true)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&equipment, alice, Zone::Graveyard);
        let stable = game.object(source).unwrap().stable_id;
        let creature =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Small creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(1, 1))
                .build();
        let entered = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                entered,
                Zone::Hand,
                Zone::Battlefield,
                crate::events::cause::EventCause::effect(),
                None,
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
        if leaves {
            game.move_object_by_effect(entered, Zone::Graveyard);
        }
        let mut choices = OptionalSequence {
            accept,
            decisions: 0,
        };
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(
            choices.decisions, 1,
            "return and attachment are one optional sequence"
        );
        let returned = game
            .battlefield
            .iter()
            .copied()
            .find(|id| game.object(*id).unwrap().stable_id == stable);
        assert_eq!(returned.is_some(), accept);
        if let Some(returned) = returned {
            assert_eq!(
                game.object(returned).unwrap().attached_to,
                (!leaves).then_some(crate::object::AttachmentTarget::Object(entered))
            );
            assert!(
                crate::triggers::check_triggers(&game, &event).is_empty(),
                "return trigger operates only from graveyard"
            );
        }
    }
}

#[test]
fn cohort_exchange_neither_checks_the_exchanged_set_despite_other_owned_creatures() {
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Memory Trade")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exchange control of two target creatures controlled by different players. If you control neither creature, draw three cards.").unwrap();
    for own_one in [false, true] {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let [alice, bob, carol] = std::array::from_fn(|i| game.players[i].id);
        let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
        let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let owner1 = if own_one { alice } else { bob };
        let first = game.create_object_from_definition(&creature, owner1, Zone::Battlefield);
        let second = game.create_object_from_definition(&creature, carol, Zone::Battlefield);
        game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        for _ in 0..3 {
            game.create_object_from_definition(&creature, alice, Zone::Library);
        }
        let mut ctx = crate::effects::EffectContext::new_default(source, alice).with_targets(vec![
            crate::effects::ResolvedTarget::Object(first),
            crate::effects::ResolvedTarget::Object(second),
        ]);
        for effect in spell
            .spell_effect
            .as_ref()
            .unwrap()
            .flattened_default_effects()
        {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.controller_of(game.object(first).unwrap()), carol);
        assert_eq!(game.controller_of(game.object(second).unwrap()), owner1);
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            if own_one { 0 } else { 3 }
        );
    }
    assert!(
        crate::compiled_text::compiled_text_lines(&spell)
            .join("\n")
            .contains("neither creature")
    );
}

#[test]
fn cohort_another_spell_cast_history_excludes_source_and_handles_uncast_copies() {
    let spell = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sequence Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Sequence Probe deals 2 damage to any target. If you've cast another spell this turn, draw a card.").unwrap();
    for source_was_cast in [false, true] {
        for own_other in [false, true] {
            for opponent_other in [false, true] {
                let mut game =
                    crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let bob = game.players[1].id;
                let source = game.create_object_from_definition(&spell, alice, Zone::Stack);
                let other =
                    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Other spell")
                        .card_types(vec![CardType::Instant])
                        .build();
                for (id, player, cast) in [
                    (source, alice, source_was_cast),
                    (
                        game.create_object_from_definition(&other, alice, Zone::Stack),
                        alice,
                        own_other,
                    ),
                    (
                        game.create_object_from_definition(&other, bob, Zone::Stack),
                        bob,
                        opponent_other,
                    ),
                ] {
                    if cast {
                        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
                            game.object(id).unwrap(),
                            &game,
                        );
                        let event = crate::triggers::TriggerEvent::new_with_provenance(
                            crate::events::SpellCastEvent::new_with_snapshot(
                                id,
                                player,
                                Zone::Hand,
                                snapshot,
                            ),
                            crate::provenance::ProvNodeId::default(),
                        );
                        game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event);
                    }
                    if id != source {
                        game.move_object_by_effect(id, Zone::Graveyard);
                    }
                }
                game.create_object_from_definition(&other, alice, Zone::Library);
                let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                    .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);
                for effect in spell
                    .spell_effect
                    .as_ref()
                    .unwrap()
                    .flattened_default_effects()
                {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
                assert_eq!(game.player(bob).unwrap().life, 18);
                assert_eq!(
                    game.player(alice).unwrap().hand.len(),
                    usize::from(own_other),
                    "source_cast={source_was_cast} own_other={own_other} opponent_other={opponent_other}"
                );
            }
        }
    }
    assert!(
        crate::compiled_text::compiled_text_lines(&spell)
            .join("\n")
            .contains("you've cast another spell this turn")
    );
}

#[test]
fn cohort_granted_vanishing_uses_time_counter_upkeep_and_last_counter_events() {
    let card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Changing Djinn")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature is turned face up, put two time counters on it and it gains vanishing.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let face_up = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::TurnedFaceUpEvent::new(source, alice),
        crate::provenance::ProvNodeId::default(),
    );
    assert_eq!(resolve_event(&mut game, &face_up), 1);
    assert_eq!(
        game.object(source)
            .unwrap()
            .counters
            .get(&crate::CounterType::Time),
        Some(&2)
    );
    let upkeep = |player| {
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::BeginningOfUpkeepEvent::new(player),
            crate::provenance::ProvNodeId::default(),
        )
    };
    assert_eq!(resolve_event(&mut game, &upkeep(bob)), 0);
    assert_eq!(resolve_event(&mut game, &upkeep(alice)), 1);
    assert_eq!(
        game.object(source)
            .unwrap()
            .counters
            .get(&crate::CounterType::Time),
        Some(&1)
    );
    game.add_counters(source, crate::CounterType::PlusOnePlusOne, 1);
    let (_, other_counter) = game
        .remove_counters(source, crate::CounterType::PlusOnePlusOne, 1, None, None)
        .unwrap();
    assert_eq!(resolve_event(&mut game, &other_counter), 0);
    let (_, last_time) = game
        .remove_counters(source, crate::CounterType::Time, 1, None, None)
        .unwrap();
    assert_eq!(resolve_event(&mut game, &upkeep(alice)), 0);
    assert_eq!(resolve_event(&mut game, &last_time), 1);
    assert!(!game.battlefield.contains(&source));
    assert!(
        crate::compiled_text::compiled_text_lines(&card)
            .join("\n")
            .contains("it gains vanishing")
    );
}

struct StickerChoices {
    accept: bool,
    options: usize,
    target_max: Option<usize>,
    target: crate::ids::ObjectId,
}
impl crate::decision::DecisionMaker for StickerChoices {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.accept
    }
    fn decide_options(
        &mut self,
        _: &crate::game_state::GameState,
        _: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.options += 1;
        vec![0]
    }
    fn decide_targets(
        &mut self,
        game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<crate::game_state::Target> {
        assert_eq!(
            self.options, 2,
            "reflexive targets follow sticker and position choices"
        );
        self.target_max = ctx.requirements[0].max_targets;
        assert!(game.current_name(ctx.source).unwrap().starts_with("AEe-Y"));
        if self.target_max == Some(0) {
            vec![]
        } else {
            vec![crate::game_state::Target::Object(self.target)]
        }
    }
}

#[test]
fn cohort_name_sticker_reflexive_targets_use_the_chosen_stickers_unique_vowels() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sticker Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 3))
        .parse_text("When this creature enters, you may put a name sticker on it. When you do, up to X target creatures each get -1/-1 until end of turn, where X is the number of unique vowels on that sticker.").unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("number of unique vowels on that sticker"),
        "{rendered}"
    );
    for (accept, available) in [(false, true), (true, false), (true, true)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        if available {
            game.add_accessible_name_sticker(alice, "AEe-Y");
        }
        let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Target")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(4, 4))
            .build();
        let target = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                source,
                Zone::Hand,
                Zone::Battlefield,
                crate::events::cause::EventCause::effect(),
                None,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let mut queue = crate::triggers::TriggerQueue::new();
        for trigger in crate::triggers::check_triggers(&game, &event) {
            queue.add(trigger);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        let mut choices = StickerChoices {
            accept,
            options: 0,
            target_max: None,
            target,
        };
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(choices.target_max, (accept && available).then_some(3));
        if accept && available {
            let next = game.add_accessible_name_sticker(alice, "Iou");
            game.apply_available_name_sticker(alice, source, next, 0)
                .unwrap();
            assert_eq!(
                game.stack.len(),
                1,
                "later sticker does not create another reflexive trigger"
            );
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert_eq!(
                game.calculated_characteristics(target).unwrap().power,
                Some(3)
            );
        } else {
            assert!(game.stack.is_empty());
        }
    }
}

#[test]
fn cohort_name_stickers_track_physical_identity_public_zones_and_name_layers() {
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let card =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Wolf in _____ Clothing")
            .card_types(vec![CardType::Creature])
            .build();
    let object = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let foreign = game.create_object_from_definition(&card, bob, Zone::Battlefield);
    let first = game.add_accessible_name_sticker(alice, "AEe-Y");
    let duplicate = game.add_accessible_name_sticker(alice, "AEe-Y");
    assert_ne!(first, duplicate);
    assert!(
        game.apply_available_name_sticker(alice, foreign, first, 2)
            .is_none()
    );
    game.apply_available_name_sticker(alice, object, first, 2)
        .unwrap();
    assert_eq!(
        game.current_name(object).as_deref(),
        Some("Wolf in AEe-Y Clothing")
    );
    assert!(
        game.apply_available_name_sticker(alice, object, first, 0)
            .is_none()
    );
    assert_eq!(
        game.available_name_stickers(alice),
        vec![(duplicate, "AEe-Y".to_string())]
    );
    let graveyard = game.move_object_by_effect(object, Zone::Graveyard).unwrap();
    assert_eq!(
        game.current_name(graveyard).as_deref(),
        Some("Wolf in AEe-Y Clothing")
    );
    let returned = game
        .move_object_by_effect(graveyard, Zone::Battlefield)
        .unwrap();
    assert_eq!(
        game.current_name(returned).as_deref(),
        Some("Wolf in AEe-Y Clothing")
    );
    let rename = crate::effects::ApplyContinuousEffect::new(
        crate::continuous::EffectTarget::Specific(returned),
        crate::continuous::Modification::SetName("Fox".into()),
        crate::effect::Until::EndOfTurn,
    );
    let mut ctx = crate::effects::EffectContext::new_default(returned, alice);
    crate::effects::EffectExecutor::execute(&rename, &mut game, &mut ctx).unwrap();
    assert_eq!(game.current_name(returned).as_deref(), Some("Fox"));
    game.apply_available_name_sticker(alice, returned, duplicate, 1)
        .unwrap();
    assert_eq!(game.current_name(returned).as_deref(), Some("Fox AEe-Y"));
    let hand = game.move_object_by_effect(returned, Zone::Hand).unwrap();
    assert_eq!(
        game.current_name(hand).as_deref(),
        Some("Wolf in _____ Clothing")
    );
    assert_eq!(game.available_name_stickers(alice).len(), 2);
    assert_eq!(crate::game_state::name_sticker_unique_vowels("AEe-Y"), 3);
    assert_eq!(crate::game_state::name_sticker_unique_vowels("rhythm"), 1);
    assert_eq!(crate::game_state::name_sticker_unique_vowels("tsk"), 0);
}
