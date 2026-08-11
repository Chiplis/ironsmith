#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::*;

#[test]
pub(super) fn triggering_permanent_controller_chooses_targets_using_ability_controller_legality() {
    #[derive(Debug)]
    struct RecordingChooser {
        expected_target: ObjectId,
        context: Option<crate::decisions::context::TargetsContext>,
    }

    impl DecisionMaker for RecordingChooser {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.context = Some(ctx.clone());
            let target = Target::Object(self.expected_target);
            ctx.requirements
                .first()
                .is_some_and(|requirement| requirement.legal_targets.contains(&target))
                .then_some(target)
                .into_iter()
                .collect()
        }
    }

    let mut game = GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source_def = CardDefinitionBuilder::new(CardId::new(), "Chooser Source")
        .card_types(vec![CardType::Enchantment])
        .build();
    let hexproof_def = CardDefinitionBuilder::new(CardId::new(), "Alice Hexproof Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .hexproof()
        .build();
    let creature_def = CardDefinitionBuilder::new(CardId::new(), "Plain Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
    let alice_hexproof =
        game.create_object_from_definition(&hexproof_def, alice, Zone::Battlefield);
    let bob_creature = game.create_object_from_definition(&creature_def, bob, Zone::Battlefield);
    let charlie_creature =
        game.create_object_from_definition(&creature_def, charlie, Zone::Battlefield);
    let entering = game.create_object_from_definition(&creature_def, bob, Zone::Battlefield);
    game.refresh_continuous_state();

    let triggering_ref = crate::target::ObjectRef::Tagged(crate::tag::TagKey::from("triggering"));
    let chooser = PlayerFilter::ControllerOf(triggering_ref.clone());
    let mut filter = ObjectFilter::permanent();
    filter.controller = Some(PlayerFilter::excluding(PlayerFilter::Any, chooser.clone()));
    filter
        .tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: crate::tag::TagKey::from("triggering"),
            relation: crate::target::TaggedOpbjectRelation::SharesCardType,
        });
    let target = ChooseSpec::target(ChooseSpec::Object(filter));
    let ability = crate::ability::TriggeredAbility {
        trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::new(crate::effects::TagTriggeringObjectEffect::new("triggering")),
            Effect::new(crate::effects::TargetOnlyEffect::explicit(target).with_chooser(chooser)),
        ]),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };
    let event = TriggerEvent::new_with_provenance(
        crate::events::zones::EnterBattlefieldEvent::new(entering, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    let source_stable_id = game.object(source).expect("source").stable_id;
    let mut trigger_queue = TriggerQueue::new();
    trigger_queue.add(TriggeredAbilityEntry {
        source,
        controller: alice,
        x_value: None,
        event_value_amount: None,
        ability: ability.clone(),
        triggering_event: event,
        source_stable_id,
        source_name: "Chooser Source".to_string(),
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: crate::triggers::TriggeredAbilitySourceKind::Object,
        trigger_identity: crate::triggers::compute_trigger_identity(&ability),
    });

    let mut dm = RecordingChooser {
        expected_target: alice_hexproof,
        context: None,
    };
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("trigger should be stacked");

    let context = dm.context.expect("target prompt");
    assert_eq!(
        context.player, bob,
        "the entering permanent's controller chooses"
    );
    let legal = &context.requirements[0].legal_targets;
    assert!(
        legal.contains(&Target::Object(alice_hexproof)),
        "hexproof is checked against Alice, the ability controller"
    );
    assert!(legal.contains(&Target::Object(charlie_creature)));
    assert!(!legal.contains(&Target::Object(bob_creature)));
    assert!(!legal.contains(&Target::Object(entering)));
    assert_eq!(
        game.stack.last().map(|entry| entry.targets.as_slice()),
        Some(&[Target::Object(alice_hexproof)][..])
    );
}

#[test]
pub(super) fn test_triggered_mana_ability_resolves_immediately_without_stack() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let alice = PlayerId::from_index(0);

    let swamp_card = CardBuilder::new(CardId::new(), "Test Swamp")
        .card_types(vec![CardType::Land])
        .subtypes(vec![crate::types::Subtype::Swamp])
        .build();
    let swamp_id = game.create_object_from_card(&swamp_card, alice, Zone::Battlefield);
    if let Some(swamp) = game.object_mut(swamp_id) {
        swamp.abilities_mut().push(Ability::mana(
            crate::cost::TotalCost::free(),
            vec![crate::mana::ManaSymbol::Black],
        ));
    }

    let enchantment_card = CardBuilder::new(CardId::new(), "Mana Echo")
        .card_types(vec![CardType::Enchantment])
        .build();
    let enchantment_id = game.create_object_from_card(&enchantment_card, alice, Zone::Battlefield);
    if let Some(enchantment) = game.object_mut(enchantment_id) {
        enchantment.abilities_mut().push(Ability::triggered(
            Trigger::player_taps_for_mana(
                crate::target::PlayerFilter::You,
                crate::filter::ObjectFilter::land().with_subtype(crate::types::Subtype::Swamp),
            ),
            vec![Effect::add_mana(vec![crate::mana::ManaSymbol::Black])],
        ));
    }

    let snapshot =
        crate::snapshot::ObjectSnapshot::from_object(game.object(swamp_id).expect("swamp"), &game);
    let event = crate::events::ManaAddedEvent::new(
        swamp_id,
        alice,
        alice,
        vec![crate::mana::ManaSymbol::Black],
    )
    .with_snapshot(Some(snapshot))
    .with_production_provenance(crate::events::mana::ManaProductionProvenance::TappedSourceForMana)
    .into_trigger_event();
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("the triggered mana ability should resolve in the trigger-processing window");

    assert!(
        trigger_queue.is_empty(),
        "triggered mana ability should resolve immediately"
    );
    assert!(
        game.stack.is_empty(),
        "triggered mana abilities should not use the stack"
    );
    assert_eq!(
        game.player(alice).expect("alice").mana_pool.black,
        1,
        "triggered mana ability should add mana immediately"
    );
}

#[test]
pub(super) fn tap_for_mana_trigger_requires_tapped_source_provenance_and_credits_land_controller() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let land_card = CardBuilder::new(CardId::new(), "Triggered Mana Land")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land_card, bob, Zone::Battlefield);

    let enchantment_card = CardBuilder::new(CardId::new(), "Additional Mana Aura")
        .card_types(vec![CardType::Enchantment])
        .build();
    let enchantment_id = game.create_object_from_card(&enchantment_card, alice, Zone::Battlefield);
    let triggering_tag = crate::TagKey::from("triggering");
    if let Some(enchantment) = game.object_mut(enchantment_id) {
        enchantment.abilities_mut().push(Ability::triggered(
            Trigger::player_taps_for_mana(PlayerFilter::Any, crate::filter::ObjectFilter::land()),
            vec![
                Effect::tag_triggering_object(triggering_tag.clone()),
                Effect::add_mana_of_any_color_player(
                    Value::Fixed(1),
                    PlayerFilter::ControllerOf(ObjectRef::Tagged(triggering_tag)),
                ),
            ],
        ));
    }

    let snapshot =
        crate::snapshot::ObjectSnapshot::from_object(game.object(land_id).expect("land"), &game);
    let event =
        crate::events::ManaAddedEvent::new(land_id, bob, bob, vec![crate::mana::ManaSymbol::Green])
            .with_snapshot(Some(snapshot.clone()))
            .into_trigger_event();
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);
    assert!(
        trigger_queue.is_empty() && game.stack.is_empty(),
        "a non-tap mana ability must not queue the tap-for-mana trigger"
    );
    assert_eq!(
        game.player(bob).expect("bob").mana_pool.white,
        0,
        "a non-tap mana ability must not receive additional mana"
    );

    let event =
        crate::events::ManaAddedEvent::new(land_id, bob, bob, vec![crate::mana::ManaSymbol::Green])
            .with_snapshot(Some(snapshot))
            .with_production_provenance(
                crate::events::mana::ManaProductionProvenance::TappedSourceForMana,
            )
            .into_trigger_event();
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("the triggered mana ability should resolve in the trigger-processing window");
    assert!(
        trigger_queue.is_empty() && game.stack.is_empty(),
        "the triggered mana ability should resolve immediately"
    );
    assert_eq!(
        game.player(alice).expect("alice").mana_pool.white,
        0,
        "the Aura controller must not receive the land's additional mana"
    );
    assert_eq!(
        game.player(bob).expect("bob").mana_pool.white,
        1,
        "the triggering land's controller should receive one mana of the chosen color"
    );
}

#[test]
pub(super) fn test_mana_added_event_triggers_mana_ability_immediately() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_card = CardBuilder::new(CardId::new(), "Mana Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

    let echo_card = CardBuilder::new(CardId::new(), "Mana Echo")
        .card_types(vec![CardType::Enchantment])
        .build();
    let echo_id = game.create_object_from_card(&echo_card, alice, Zone::Battlefield);
    if let Some(echo) = game.object_mut(echo_id) {
        echo.abilities_mut().push(Ability::triggered(
            Trigger::mana_added(crate::target::PlayerFilter::You),
            vec![Effect::add_mana_player(
                vec![crate::mana::ManaSymbol::Black],
                crate::target::PlayerFilter::Specific(bob),
            )],
        ));
    }

    let event = crate::events::ManaAddedEvent::trigger_event(
        source_id,
        alice,
        alice,
        vec![crate::mana::ManaSymbol::Green],
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("mana-added trigger processing should succeed");

    assert!(
        trigger_queue.is_empty(),
        "triggered mana ability should resolve immediately"
    );
    assert!(
        game.stack.is_empty(),
        "triggered mana abilities should not use the stack"
    );
    assert_eq!(
        game.player(bob).expect("bob").mana_pool.black,
        1,
        "mana-added triggered mana ability should add mana immediately"
    );
}

#[test]
pub(super) fn test_triggered_mana_ability_target_requirement_uses_stack() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_card = CardBuilder::new(CardId::new(), "Mana Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

    let echo_card = CardBuilder::new(CardId::new(), "Targeting Mana Echo")
        .card_types(vec![CardType::Enchantment])
        .build();
    let echo_id = game.create_object_from_card(&echo_card, alice, Zone::Battlefield);
    if let Some(echo) = game.object_mut(echo_id) {
        echo.abilities_mut().push(Ability {
            kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: Trigger::mana_added(crate::target::PlayerFilter::You),
                effects: vec![Effect::add_mana_player(
                    vec![crate::mana::ManaSymbol::Black],
                    crate::target::PlayerFilter::Specific(bob),
                )]
                .into(),
                choices: vec![crate::target::ChooseSpec::target(
                    crate::target::ChooseSpec::Player(crate::target::PlayerFilter::Any),
                )],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        });
    }

    let event = crate::events::ManaAddedEvent::trigger_event(
        source_id,
        alice,
        alice,
        vec![crate::mana::ManaSymbol::Green],
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("targeted trigger should be stacked normally");

    assert_eq!(
        game.stack.len(),
        1,
        "triggered abilities that require targets are not mana abilities"
    );
    assert_eq!(
        game.player(bob).expect("bob").mana_pool.black,
        0,
        "targeted trigger should not resolve immediately"
    );
}

#[test]
pub(super) fn test_triggered_mana_ability_ignores_non_target_choices() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_card = CardBuilder::new(CardId::new(), "Mana Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

    let echo_card = CardBuilder::new(CardId::new(), "Choosing Mana Echo")
        .card_types(vec![CardType::Enchantment])
        .build();
    let echo_id = game.create_object_from_card(&echo_card, alice, Zone::Battlefield);
    if let Some(echo) = game.object_mut(echo_id) {
        echo.abilities_mut().push(Ability {
            kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: Trigger::mana_added(crate::target::PlayerFilter::You),
                effects: vec![Effect::add_mana_player(
                    vec![crate::mana::ManaSymbol::Black],
                    crate::target::PlayerFilter::Specific(bob),
                )]
                .into(),
                choices: vec![crate::target::ChooseSpec::SpecificPlayer(alice)],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        });
    }

    let event = crate::events::ManaAddedEvent::trigger_event(
        source_id,
        alice,
        alice,
        vec![crate::mana::ManaSymbol::Green],
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("non-target choice trigger should resolve immediately");

    assert!(game.stack.is_empty());
    assert_eq!(game.player(bob).expect("bob").mana_pool.black, 1);
}

#[test]
pub(super) fn test_activated_mana_ability_emits_mana_added_event_for_triggers() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let land_card = CardBuilder::new(CardId::new(), "Event Land")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land_card, alice, Zone::Battlefield);
    if let Some(land) = game.object_mut(land_id) {
        land.abilities_mut().push(Ability::mana(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            vec![crate::mana::ManaSymbol::Black],
        ));
    }

    let echo_card = CardBuilder::new(CardId::new(), "Mana Added Echo")
        .card_types(vec![CardType::Enchantment])
        .build();
    let echo_id = game.create_object_from_card(&echo_card, alice, Zone::Battlefield);
    if let Some(echo) = game.object_mut(echo_id) {
        echo.abilities_mut().push(Ability::triggered(
            Trigger::mana_added(crate::target::PlayerFilter::You),
            vec![Effect::add_mana_player(
                vec![crate::mana::ManaSymbol::Green],
                crate::target::PlayerFilter::Specific(bob),
            )],
        ));
    }

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateManaAbility {
                    source,
                    ability_index: _
                } if *source == land_id
            )
        })
        .expect("mana ability activation should be legal");

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("mana ability activation should succeed");

    let alice_pool = &game.player(alice).expect("alice").mana_pool;
    assert_eq!(alice_pool.black, 1, "land should add its fixed black mana");
    assert_eq!(
        game.player(bob).expect("bob").mana_pool.green,
        1,
        "mana-added triggered mana ability should resolve from the emitted event"
    );
    assert!(game.stack.is_empty());
    assert!(trigger_queue.is_empty());
}

#[test]
pub(super) fn test_non_mana_tap_for_mana_trigger_still_uses_stack() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let alice = PlayerId::from_index(0);

    let swamp_card = CardBuilder::new(CardId::new(), "Test Swamp")
        .card_types(vec![CardType::Land])
        .subtypes(vec![crate::types::Subtype::Swamp])
        .build();
    let swamp_id = game.create_object_from_card(&swamp_card, alice, Zone::Battlefield);
    if let Some(swamp) = game.object_mut(swamp_id) {
        swamp.abilities_mut().push(Ability::mana(
            crate::cost::TotalCost::free(),
            vec![crate::mana::ManaSymbol::Black],
        ));
    }

    let enchantment_card = CardBuilder::new(CardId::new(), "Mana Barbs Test")
        .card_types(vec![CardType::Enchantment])
        .build();
    let enchantment_id = game.create_object_from_card(&enchantment_card, alice, Zone::Battlefield);
    if let Some(enchantment) = game.object_mut(enchantment_id) {
        enchantment.abilities_mut().push(Ability::triggered(
            Trigger::player_taps_for_mana(
                crate::target::PlayerFilter::Any,
                crate::filter::ObjectFilter::land(),
            ),
            vec![Effect::lose_life_player(
                1,
                crate::target::PlayerFilter::Specific(alice),
            )],
        ));
    }

    let snapshot =
        crate::snapshot::ObjectSnapshot::from_object(game.object(swamp_id).expect("swamp"), &game);
    let event = crate::events::ManaAddedEvent::new(
        swamp_id,
        alice,
        alice,
        vec![crate::mana::ManaSymbol::Black],
    )
    .with_snapshot(Some(snapshot))
    .with_production_provenance(crate::events::mana::ManaProductionProvenance::TappedSourceForMana)
    .into_trigger_event();
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

    assert!(
        !trigger_queue.is_empty(),
        "non-mana trigger should remain queued"
    );
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("should put trigger on stack");
    assert_eq!(
        game.stack.len(),
        1,
        "non-mana tap-for-mana trigger should use stack"
    );
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn emrakul_cast_trigger_prompts_for_opponent_in_four_player_game() {
    #[derive(Debug, Default)]
    struct RecordingTargetDecisionMaker {
        targets_ctx: Option<crate::decisions::context::TargetsContext>,
    }

    impl DecisionMaker for RecordingTargetDecisionMaker {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.targets_ctx = Some(ctx.clone());
            ctx.requirements
                .iter()
                .filter_map(|requirement| requirement.legal_targets.first().copied())
                .collect()
        }
    }

    let mut game = GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
    );
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = RecordingTargetDecisionMaker::default();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let dana = PlayerId::from_index(3);

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let emrakul_id =
        game.create_object_from_definition(&emrakul_the_promised_end(), alice, Zone::Stack);
    let (emrakul_stable_id, emrakul_name) = game
        .object(emrakul_id)
        .map(|object| (object.stable_id, object.name.to_string()))
        .expect("Emrakul spell object should exist");
    game.push_to_stack(
        StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
    );

    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Emrakul should queue its cast trigger from the stack"
    );

    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Emrakul trigger should go on the stack");

    let targets_ctx = dm
        .targets_ctx
        .expect("Emrakul trigger should request target selection");
    assert_eq!(
        targets_ctx.player, alice,
        "the caster should choose Emrakul's target opponent"
    );
    assert_eq!(
        targets_ctx.requirements.len(),
        1,
        "Emrakul should ask for exactly one target requirement"
    );

    let legal_players: Vec<PlayerId> = targets_ctx.requirements[0]
        .legal_targets
        .iter()
        .filter_map(|target| match target {
            Target::Player(player) => Some(*player),
            Target::Object(_) => None,
        })
        .collect();
    assert_eq!(
        legal_players,
        vec![bob, charlie, dana],
        "all opponents should be legal Emrakul targets"
    );
    assert_eq!(
        game.stack.len(),
        2,
        "Emrakul's trigger should be pushed on top of the spell"
    );
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn emrakul_cast_trigger_prompt_does_not_autofill_or_stack_before_choice() {
    #[derive(Debug, Default)]
    struct PromptOnlyDecisionMaker {
        targets_ctx: Option<crate::decisions::context::TargetsContext>,
    }

    impl DecisionMaker for PromptOnlyDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.targets_ctx.is_some()
        }

        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.targets_ctx = Some(ctx.clone());
            Vec::new()
        }
    }

    let mut game = GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
    );
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = PromptOnlyDecisionMaker::default();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let dana = PlayerId::from_index(3);

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let emrakul_id =
        game.create_object_from_definition(&emrakul_the_promised_end(), alice, Zone::Stack);
    let (emrakul_stable_id, emrakul_name) = game
        .object(emrakul_id)
        .map(|object| (object.stable_id, object.name.to_string()))
        .expect("Emrakul spell object should exist");
    game.push_to_stack(
        StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
    );

    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Emrakul trigger prompt should not fail");

    let targets_ctx = dm
        .targets_ctx
        .expect("Emrakul trigger should request target selection");
    let legal_players: Vec<PlayerId> = targets_ctx.requirements[0]
        .legal_targets
        .iter()
        .filter_map(|target| match target {
            Target::Player(player) => Some(*player),
            Target::Object(_) => None,
        })
        .collect();

    assert_eq!(
        legal_players,
        vec![bob, charlie, dana],
        "all opponents should remain legal Emrakul targets while the prompt is open"
    );
    assert_eq!(
        game.stack.len(),
        1,
        "the trigger should not be pushed until the player actually chooses a target"
    );
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "the unresolved trigger should stay queued while waiting for player input"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn last_rites_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(27_376), "Last Rites")
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Black,
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Discard any number of cards. Target player reveals their hand, then you choose a nonland card from it for each card discarded this way. That player discards those cards.",
        )
        .expect("Last Rites should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn last_rites_test_card(name: &str, card_types: Vec<CardType>) -> crate::card::Card {
    CardBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct LastRitesDecisionMaker {
    pub(super) target: PlayerId,
    pub(super) discard_from_hand: Vec<ObjectId>,
    pub(super) choose_from_target: Vec<ObjectId>,
    pub(super) reveal_calls: Vec<(PlayerId, PlayerId, bool, Vec<ObjectId>)>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for LastRitesDecisionMaker {
    fn decide_targets(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        vec![Target::Player(self.target)]
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let legal_ids = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        let scripted = if self
            .discard_from_hand
            .iter()
            .any(|id| legal_ids.contains(id))
        {
            &self.discard_from_hand
        } else {
            &self.choose_from_target
        };
        scripted
            .iter()
            .copied()
            .filter(|id| legal_ids.contains(id))
            .collect()
    }

    fn view_cards(
        &mut self,
        _game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        self.reveal_calls
            .push((viewer, ctx.subject, ctx.public, cards.to_vec()));
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn last_rites_discards_chosen_nonlands_from_revealed_target_hand() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let alice_discard_one = game.create_object_from_card(
        &last_rites_test_card("Alice Discard One", vec![CardType::Instant]),
        alice,
        Zone::Hand,
    );
    let alice_discard_two = game.create_object_from_card(
        &last_rites_test_card("Alice Discard Two", vec![CardType::Creature]),
        alice,
        Zone::Hand,
    );
    let _alice_keeps = game.create_object_from_card(
        &last_rites_test_card("Alice Keeps", vec![CardType::Sorcery]),
        alice,
        Zone::Hand,
    );
    let bob_nonland_one = game.create_object_from_card(
        &last_rites_test_card("Bob Nonland One", vec![CardType::Instant]),
        bob,
        Zone::Hand,
    );
    let bob_nonland_two = game.create_object_from_card(
        &last_rites_test_card("Bob Nonland Two", vec![CardType::Creature]),
        bob,
        Zone::Hand,
    );
    let bob_land = game.create_object_from_card(
        &last_rites_test_card("Bob Land", vec![CardType::Land]),
        bob,
        Zone::Hand,
    );

    let last_rites = last_rites_definition();
    let spell_id = game.create_object_from_definition(&last_rites, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice).with_targets(vec![Target::Player(bob)]));

    let mut dm = LastRitesDecisionMaker {
        target: bob,
        discard_from_hand: vec![alice_discard_one, alice_discard_two],
        choose_from_target: vec![bob_nonland_one, bob_nonland_two],
        reveal_calls: Vec::new(),
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Last Rites should resolve");

    let public_reveals = dm
        .reveal_calls
        .iter()
        .filter(|(_, _, public, _)| *public)
        .collect::<Vec<_>>();
    assert_eq!(
        public_reveals.len(),
        game.players.len(),
        "Last Rites should reveal the target player's hand publicly"
    );
    for (_viewer, subject, public, cards) in public_reveals {
        assert_eq!(*subject, bob);
        assert!(*public);
        assert!(cards.contains(&bob_nonland_one));
        assert!(cards.contains(&bob_nonland_two));
        assert!(cards.contains(&bob_land));
    }

    assert!(!player_zone_contains_named(
        &game,
        alice,
        Zone::Hand,
        "Alice Discard One"
    ));
    assert!(!player_zone_contains_named(
        &game,
        alice,
        Zone::Hand,
        "Alice Discard Two"
    ));
    assert!(player_zone_contains_named(
        &game,
        alice,
        Zone::Hand,
        "Alice Keeps"
    ));

    assert!(!player_zone_contains_named(
        &game,
        bob,
        Zone::Hand,
        "Bob Nonland One"
    ));
    assert!(!player_zone_contains_named(
        &game,
        bob,
        Zone::Hand,
        "Bob Nonland Two"
    ));
    assert!(player_zone_contains_named(
        &game,
        bob,
        Zone::Hand,
        "Bob Land"
    ));

    assert!(player_zone_contains_named(
        &game,
        alice,
        Zone::Graveyard,
        "Alice Discard One"
    ));
    assert!(player_zone_contains_named(
        &game,
        alice,
        Zone::Graveyard,
        "Alice Discard Two"
    ));
    assert!(player_zone_contains_named(
        &game,
        bob,
        Zone::Graveyard,
        "Bob Nonland One"
    ));
    assert!(player_zone_contains_named(
        &game,
        bob,
        Zone::Graveyard,
        "Bob Nonland Two"
    ));
    assert!(!player_zone_contains_named(
        &game,
        bob,
        Zone::Graveyard,
        "Bob Land"
    ));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn last_rites_discards_available_nonlands_when_more_cards_were_discarded() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let alice_discard_one = game.create_object_from_card(
        &last_rites_test_card("Alice Discard One", vec![CardType::Instant]),
        alice,
        Zone::Hand,
    );
    let alice_discard_two = game.create_object_from_card(
        &last_rites_test_card("Alice Discard Two", vec![CardType::Creature]),
        alice,
        Zone::Hand,
    );
    let alice_discard_three = game.create_object_from_card(
        &last_rites_test_card("Alice Discard Three", vec![CardType::Sorcery]),
        alice,
        Zone::Hand,
    );
    let bob_nonland = game.create_object_from_card(
        &last_rites_test_card("Bob Only Nonland", vec![CardType::Instant]),
        bob,
        Zone::Hand,
    );
    let _bob_land = game.create_object_from_card(
        &last_rites_test_card("Bob Land", vec![CardType::Land]),
        bob,
        Zone::Hand,
    );

    let last_rites = last_rites_definition();
    let spell_id = game.create_object_from_definition(&last_rites, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice).with_targets(vec![Target::Player(bob)]));

    let mut dm = LastRitesDecisionMaker {
        target: bob,
        discard_from_hand: vec![alice_discard_one, alice_discard_two, alice_discard_three],
        choose_from_target: vec![bob_nonland],
        reveal_calls: Vec::new(),
    };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Last Rites should resolve with fewer target nonlands than discarded cards");

    assert!(!player_zone_contains_named(
        &game,
        bob,
        Zone::Hand,
        "Bob Only Nonland"
    ));
    assert!(player_zone_contains_named(
        &game,
        bob,
        Zone::Hand,
        "Bob Land"
    ));
    assert!(player_zone_contains_named(
        &game,
        bob,
        Zone::Graveyard,
        "Bob Only Nonland"
    ));
    assert!(!player_zone_contains_named(
        &game,
        bob,
        Zone::Graveyard,
        "Bob Land"
    ));
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn kodama_of_the_center_tree_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(74_086), "Kodama of the Center Tree")
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(4),
            ManaSymbol::Green,
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spirit])
        .power_toughness(PowerToughness::new(PtValue::Star, PtValue::Star))
        .parse_text(
            "Kodama of the Center Tree's power and toughness are each equal to the number of Spirits you control.\n\
             Kodama of the Center Tree has soulshift X, where X is the number of Spirits you control. (When this creature dies, you may return target Spirit card with mana value X or less from your graveyard to your hand.)",
        )
        .expect("Kodama of the Center Tree should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn spirit_card_with_mana_value(name: &str, mana_value: u8) -> crate::card::Card {
    CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
            mana_value,
        )]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn player_zone_contains_named(
    game: &GameState,
    player: PlayerId,
    zone: Zone,
    name: &str,
) -> bool {
    let object_ids = match zone {
        Zone::Hand => &game.player(player).expect("player exists").hand,
        Zone::Graveyard => &game.player(player).expect("player exists").graveyard,
        _ => panic!("unsupported zone check {zone:?}"),
    };
    object_ids.iter().any(|id| {
        game.object(*id)
            .is_some_and(|object| object.name == name && object.zone == zone)
    })
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kodama_of_the_center_tree_dynamic_soulshift_counts_itself_when_dying() {
    let def = kodama_of_the_center_tree_definition();
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let alice = PlayerId::from_index(0);
    let kodama = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let other_spirit_one = game.create_object_from_card(
        &spirit_card_with_mana_value("Battlefield Spirit One", 1),
        alice,
        Zone::Battlefield,
    );
    let other_spirit_two = game.create_object_from_card(
        &spirit_card_with_mana_value("Battlefield Spirit Two", 1),
        alice,
        Zone::Battlefield,
    );
    let target = game.create_object_from_card(
        &spirit_card_with_mana_value("Returned Spirit", 3),
        alice,
        Zone::Graveyard,
    );

    game.mark_damage(kodama, 99);
    check_and_apply_sbas(&mut game, &mut trigger_queue)
        .expect("lethal damage should put Kodama into the graveyard and queue soulshift");
    assert_eq!(trigger_queue.entries.len(), 1, "Kodama should trigger once");

    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Kodama soulshift should go on the stack");
    let entry = game.stack.last().expect("soulshift should be on the stack");
    assert!(
        entry.targets.contains(&Target::Object(target)),
        "two other Spirits plus dying Kodama should make a mana value 3 Spirit legal"
    );

    game.move_object_by_effect(other_spirit_one, Zone::Graveyard);
    game.move_object_by_effect(other_spirit_two, Zone::Graveyard);

    resolve_stack_entry_with(&mut game, &mut dm).expect("Kodama soulshift should resolve");

    assert!(
        player_zone_contains_named(&game, alice, Zone::Hand, "Returned Spirit"),
        "soulshift should use the pre-death value instead of drifting with later battlefield changes"
    );
    assert!(
        !player_zone_contains_named(&game, alice, Zone::Graveyard, "Returned Spirit"),
        "returned Spirit should no longer be in the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kodama_of_the_center_tree_dynamic_soulshift_rejects_above_predeath_count() {
    let def = kodama_of_the_center_tree_definition();
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let alice = PlayerId::from_index(0);
    let kodama = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.create_object_from_card(
        &spirit_card_with_mana_value("Battlefield Spirit", 1),
        alice,
        Zone::Battlefield,
    );
    let target = game.create_object_from_card(
        &spirit_card_with_mana_value("Too Large Spirit", 3),
        alice,
        Zone::Graveyard,
    );

    game.mark_damage(kodama, 99);
    check_and_apply_sbas(&mut game, &mut trigger_queue)
        .expect("lethal damage should put Kodama into the graveyard and queue soulshift");
    assert_eq!(trigger_queue.entries.len(), 1, "Kodama should trigger once");

    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Kodama soulshift should go on the stack even when choosing no target");
    let entry = game.stack.last().expect("soulshift should be on the stack");
    assert!(
        !entry.targets.contains(&Target::Object(target)),
        "one other Spirit plus dying Kodama should cap soulshift at mana value 2"
    );
    assert!(
        player_zone_contains_named(&game, alice, Zone::Graveyard, "Too Large Spirit"),
        "over-cap Spirit should stay in the graveyard"
    );
    assert!(
        !player_zone_contains_named(&game, alice, Zone::Hand, "Too Large Spirit"),
        "over-cap Spirit should not move to hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn infernal_kirin_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(74_377), "Infernal Kirin")
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Black,
            ManaSymbol::Black,
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Kirin, Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Flying\nWhenever you cast a Spirit or Arcane spell, target player reveals their hand and discards all cards with that spell's mana value.",
        )
        .expect("Infernal Kirin should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn test_card_with_mana_value(
    name: &str,
    mana: Vec<ManaSymbol>,
    card_types: Vec<CardType>,
) -> crate::card::Card {
    CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_symbols(mana))
        .card_types(card_types)
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn infernal_kirin_discards_only_cards_matching_triggering_spell_mana_value() {
    struct ChooseBobAndAllDiscardCards {
        bob: PlayerId,
        reveal_calls: Vec<(PlayerId, PlayerId, bool, Vec<ObjectId>)>,
    }

    impl DecisionMaker for ChooseBobAndAllDiscardCards {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            vec![Target::Player(self.bob)]
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect()
        }

        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.reveal_calls
                .push((viewer, ctx.subject, ctx.public, cards.to_vec()));
        }
    }

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&infernal_kirin_definition(), alice, Zone::Battlefield);

    let matching_one = game.create_object_from_card(
        &test_card_with_mana_value(
            "Bob Matching One",
            vec![ManaSymbol::Generic(1), ManaSymbol::Blue],
            vec![CardType::Instant],
        ),
        bob,
        Zone::Hand,
    );
    let matching_two = game.create_object_from_card(
        &test_card_with_mana_value(
            "Bob Matching Two",
            vec![ManaSymbol::Black, ManaSymbol::Black],
            vec![CardType::Sorcery],
        ),
        bob,
        Zone::Hand,
    );
    let nonmatching = game.create_object_from_card(
        &test_card_with_mana_value(
            "Bob Nonmatching",
            vec![ManaSymbol::Generic(2), ManaSymbol::Green],
            vec![CardType::Instant],
        ),
        bob,
        Zone::Hand,
    );
    let alice_matching = game.create_object_from_card(
        &test_card_with_mana_value(
            "Alice Matching",
            vec![ManaSymbol::Generic(1), ManaSymbol::Red],
            vec![CardType::Instant],
        ),
        alice,
        Zone::Hand,
    );

    let triggering_spell = CardBuilder::new(CardId::new(), "Triggering Spirit")
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(1),
            ManaSymbol::White,
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spirit])
        .build();
    let spell_id = game.create_object_from_card(&triggering_spell, alice, Zone::Stack);
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Infernal Kirin should trigger from a Spirit spell"
    );

    let mut dm = ChooseBobAndAllDiscardCards {
        bob,
        reveal_calls: Vec::new(),
    };
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Infernal Kirin trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Infernal Kirin trigger should resolve");

    assert_eq!(
        dm.reveal_calls.len(),
        game.players.len(),
        "revealing a hand should show it publicly to every player"
    );
    for (_viewer, subject, public, cards) in &dm.reveal_calls {
        assert_eq!(*subject, bob);
        assert!(
            *public,
            "Infernal Kirin should reveal the targeted player's hand"
        );
        assert!(cards.contains(&matching_one));
        assert!(cards.contains(&matching_two));
        assert!(cards.contains(&nonmatching));
    }

    let bob_hand = &game.player(bob).expect("bob exists").hand;
    assert!(!bob_hand.contains(&matching_one));
    assert!(!bob_hand.contains(&matching_two));
    assert!(bob_hand.contains(&nonmatching));
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .contains(&alice_matching),
        "Infernal Kirin should only discard from the targeted player's hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn infernal_kirin_triggers_for_arcane_spell() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    game.create_object_from_definition(&infernal_kirin_definition(), alice, Zone::Battlefield);

    let arcane_spell = CardBuilder::new(CardId::new(), "Arcane Probe")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red]))
        .card_types(vec![CardType::Instant])
        .subtypes(vec![Subtype::Arcane])
        .build();
    let spell_id = game.create_object_from_card(&arcane_spell, alice, Zone::Stack);
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Infernal Kirin should trigger from an Arcane spell"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn infernal_kirin_does_not_trigger_for_non_spirit_non_arcane_spell() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    game.create_object_from_definition(&infernal_kirin_definition(), alice, Zone::Battlefield);

    let ordinary_spell = CardBuilder::new(CardId::new(), "Ordinary Instant")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_card(&ordinary_spell, alice, Zone::Stack);
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);

    assert!(
        trigger_queue.entries.is_empty(),
        "Infernal Kirin should not trigger from a spell that is neither Spirit nor Arcane"
    );
}

#[test]
pub(super) fn put_triggers_on_stack_uses_controller_selected_order_for_simultaneous_triggers() {
    use crate::ability::TriggeredAbility;
    use crate::events::phase::BeginningOfUpkeepEvent;
    use crate::target::PlayerFilter;

    #[derive(Debug, Default)]
    struct TriggerOrderDecisionMaker {
        prompts: Vec<String>,
    }

    impl DecisionMaker for TriggerOrderDecisionMaker {
        fn decide_order(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::OrderContext,
        ) -> Vec<ObjectId> {
            self.prompts.push(ctx.description.clone());
            ctx.items.iter().rev().map(|(id, _)| *id).collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let upkeep_event = TriggerEvent::new_with_provenance(
        BeginningOfUpkeepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );

    let alpha_id = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Alpha Trigger")
            .card_types(vec![CardType::Enchantment])
            .build(),
        alice,
        Zone::Battlefield,
    );
    let beta_id = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Beta Trigger")
            .card_types(vec![CardType::Enchantment])
            .build(),
        alice,
        Zone::Battlefield,
    );

    let alpha_stable_id = game.object(alpha_id).expect("alpha exists").stable_id;
    let beta_stable_id = game.object(beta_id).expect("beta exists").stable_id;
    let ability = TriggeredAbility {
        trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
        effects: crate::resolution::ResolutionProgram::default(),
        choices: vec![],
        intervening_if: None,
        presentation_label: None,
    };

    let mut trigger_queue = TriggerQueue::new();
    trigger_queue.add(TriggeredAbilityEntry {
        source: alpha_id,
        controller: alice,
        x_value: None,
        event_value_amount: None,
        ability: ability.clone(),
        triggering_event: upkeep_event.clone(),
        source_stable_id: alpha_stable_id,
        source_name: "Alpha Trigger".to_string(),
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: crate::triggers::TriggeredAbilitySourceKind::Object,
        trigger_identity: crate::triggers::compute_trigger_identity(&ability),
    });
    trigger_queue.add(TriggeredAbilityEntry {
        source: beta_id,
        controller: alice,
        x_value: None,
        event_value_amount: None,
        ability: ability.clone(),
        triggering_event: upkeep_event,
        source_stable_id: beta_stable_id,
        source_name: "Beta Trigger".to_string(),
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: crate::triggers::TriggeredAbilitySourceKind::Object,
        trigger_identity: crate::triggers::compute_trigger_identity(&ability),
    });

    let mut dm = TriggerOrderDecisionMaker::default();
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("trigger ordering should succeed");

    assert_eq!(dm.prompts.len(), 1, "expected a trigger ordering prompt");
    assert!(
        dm.prompts[0].contains("leftmost item becomes the top"),
        "trigger ordering prompt should explain stack direction"
    );
    assert_eq!(game.stack.len(), 2, "both triggers should be stacked");
    assert_eq!(
        game.stack
            .last()
            .and_then(|entry| entry.source_name.as_deref()),
        Some("Beta Trigger"),
        "the selected first trigger should become the top of the stack"
    );
}

#[test]
pub(super) fn put_triggers_on_stack_orders_each_controller_in_apnap_order() {
    use crate::ability::TriggeredAbility;
    use crate::events::phase::BeginningOfUpkeepEvent;
    use crate::target::PlayerFilter;

    #[derive(Debug, Default)]
    struct RecordingTriggerOrderDecisionMaker {
        players: Vec<PlayerId>,
    }

    impl DecisionMaker for RecordingTriggerOrderDecisionMaker {
        fn decide_order(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::OrderContext,
        ) -> Vec<ObjectId> {
            self.players.push(ctx.player);
            ctx.items.iter().rev().map(|(id, _)| *id).collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;

    let upkeep_event = TriggerEvent::new_with_provenance(
        BeginningOfUpkeepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    let ability = TriggeredAbility {
        trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
        effects: crate::resolution::ResolutionProgram::default(),
        choices: vec![],
        intervening_if: None,
        presentation_label: None,
    };

    let make_trigger = |game: &mut GameState, name: &str, controller: PlayerId| {
        let object_id = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Enchantment])
                .build(),
            controller,
            Zone::Battlefield,
        );
        let stable_id = game
            .object(object_id)
            .expect("trigger source exists")
            .stable_id;
        TriggeredAbilityEntry {
            source: object_id,
            controller,
            x_value: None,
            event_value_amount: None,
            ability: ability.clone(),
            triggering_event: upkeep_event.clone(),
            source_stable_id: stable_id,
            source_name: name.to_string(),
            source_snapshot: None,
            tagged_objects: std::collections::HashMap::new(),
            source_kind: crate::triggers::TriggeredAbilitySourceKind::Object,
            trigger_identity: crate::triggers::compute_trigger_identity(&ability),
        }
    };

    let mut trigger_queue = TriggerQueue::new();
    trigger_queue.add(make_trigger(&mut game, "Alice One", alice));
    trigger_queue.add(make_trigger(&mut game, "Alice Two", alice));
    trigger_queue.add(make_trigger(&mut game, "Bob One", bob));
    trigger_queue.add(make_trigger(&mut game, "Bob Two", bob));

    let mut dm = RecordingTriggerOrderDecisionMaker::default();
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("trigger ordering should succeed");

    assert_eq!(
        dm.players,
        vec![alice, bob],
        "controllers should order their triggers in APNAP order"
    );
    let stack_names: Vec<&str> = game
        .stack
        .iter()
        .map(|entry| entry.source_name.as_deref().unwrap_or("?"))
        .collect();
    assert_eq!(
        stack_names,
        vec!["Alice One", "Alice Two", "Bob One", "Bob Two"],
        "each controller's chosen order should be preserved within APNAP stacking"
    );
    assert_eq!(
        game.stack
            .last()
            .and_then(|entry| entry.source_name.as_deref()),
        Some("Bob Two"),
        "the non-active player's first chosen trigger should resolve first"
    );
}

#[test]
pub(super) fn test_drain_pending_events_checks_delayed_zone_change_triggers() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let stangg_id = create_creature(&mut game, "Stangg", alice, 3, 4);
    let twin_id = create_creature(&mut game, "Stangg Twin", alice, 3, 4);

    game.effect_store
        .delayed_triggers
        .push(crate::triggers::DelayedTrigger {
            trigger: Trigger::this_leaves_battlefield(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::move_to_zone(ChooseSpec::SpecificObject(twin_id), Zone::Exile, true),
            ]),
            one_shot: true,
            x_value: None,
            not_before_turn: None,
            expires_at_turn: None,
            expires_before_controller_turn_after: None,
            expires_at_end_of_combat: false,
            while_any_tagged_object_in_zone: None,
            target_objects: vec![stangg_id],
            ability_source: None,
            ability_source_stable_id: None,
            ability_source_name: None,
            ability_source_snapshot: None,
            controller: alice,
            choices: vec![],
            tagged_objects: std::collections::HashMap::new(),
            tagged_players: std::collections::HashMap::new(),
            prepayment: None,
            prevention_shield: None,
        });

    let moved = game.move_object_by_effect(stangg_id, Zone::Graveyard);
    assert!(moved.is_some(), "expected Stangg to move to graveyard");
    assert!(
        !game.effect_store.pending_trigger_events.is_empty(),
        "moving Stangg off battlefield should queue a zone-change trigger event"
    );

    drain_pending_trigger_events(&mut game, &mut trigger_queue);

    assert!(
        !trigger_queue.entries.is_empty(),
        "pending zone-change events should check delayed triggers"
    );
    assert_eq!(
        trigger_queue.entries[0].source, stangg_id,
        "delayed trigger source should be the leaving permanent"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("delayed trigger should be put on stack");
    assert_eq!(
        game.stack.len(),
        1,
        "expected delayed trigger ability on stack"
    );

    resolve_stack_entry(&mut game).expect("delayed trigger should resolve");

    assert!(
        !game.battlefield.contains(&twin_id),
        "Stangg Twin should no longer be on battlefield after delayed exile resolves"
    );
}

#[test]
pub(super) fn test_pending_zone_change_still_drives_non_delayed_triggered_abilities() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let stangg_id = create_creature(&mut game, "Stangg", alice, 3, 4);
    let twin_id = create_creature(&mut game, "Stangg Twin", alice, 3, 4);
    if let Some(twin) = game.object_mut(twin_id) {
        twin.kind = ObjectKind::Token;
    }

    if let Some(stangg) = game.object_mut(stangg_id) {
        let filter = ObjectFilter::default().token().named("Stangg Twin");
        stangg.abilities_mut().push(Ability::triggered(
            Trigger::leaves_battlefield(filter),
            vec![Effect::sacrifice_source()],
        ));
    } else {
        panic!("expected Stangg object to exist");
    }

    let moved = game.move_object_by_effect(twin_id, Zone::Graveyard);
    assert!(moved.is_some(), "expected Stangg Twin to move to graveyard");

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("triggered ability should be put on stack");
    assert_eq!(
        game.stack.len(),
        1,
        "expected sacrifice trigger on stack after Stangg Twin left"
    );

    resolve_stack_entry(&mut game).expect("sacrifice trigger should resolve");

    assert!(
        !game.battlefield.contains(&stangg_id),
        "Stangg should be sacrificed when Stangg Twin leaves the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn matter_reshaper_definition() -> crate::cards::CardDefinition {
    let def = CardDefinitionBuilder::new(CardId::from_raw(72_806), "Matter Reshaper")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Colorless],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi])
        .power_toughness(PowerToughness::fixed(3, 2))
        .parse_text(
            "({C} represents colorless mana.)\n\
             When this creature dies, reveal the top card of your library. You may put that card onto the battlefield if it's a permanent card with mana value 3 or less. Otherwise, put that card into your hand.",
        )
        .expect("Matter Reshaper should parse for runtime tests");
    def
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn matter_reshaper_test_card(
    raw_id: u32,
    name: &str,
    card_types: Vec<CardType>,
    mana_value: u8,
) -> crate::card::Card {
    let mut builder = CardBuilder::new(CardId::from_raw(raw_id), name).card_types(card_types);
    if mana_value > 0 {
        builder = builder.mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]));
    }
    builder.build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_matter_reshaper_death_trigger(
    game: &mut GameState,
    reshaper_id: ObjectId,
    dm: &mut dyn DecisionMaker,
) {
    let mut trigger_queue = TriggerQueue::new();
    game.move_object_by_effect(reshaper_id, Zone::Graveyard)
        .expect("Matter Reshaper should move to graveyard");
    drain_pending_trigger_events(game, &mut trigger_queue);
    put_triggers_on_stack(game, &mut trigger_queue)
        .expect("Matter Reshaper death trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "Matter Reshaper death should create one trigger"
    );
    resolve_stack_entry_with(game, dm).expect("Matter Reshaper death trigger should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn matter_reshaper_accepts_small_permanent_card_onto_battlefield() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let reshaper = matter_reshaper_definition();
    let reshaper_id = game.create_object_from_definition(&reshaper, alice, Zone::Battlefield);
    let top_card = matter_reshaper_test_card(72_807, "Revealed Land", vec![CardType::Land], 0);
    let top_id = game.create_object_from_card(&top_card, alice, Zone::Library);
    let top_stable = game.object(top_id).expect("top card exists").stable_id;

    let mut dm = SelectFirstDecisionMaker;
    resolve_matter_reshaper_death_trigger(&mut game, reshaper_id, &mut dm);

    let revealed_id = game
        .find_object_by_stable_id(top_stable)
        .expect("revealed card should still exist");
    assert!(
        game.battlefield.contains(&revealed_id),
        "accepted small permanent card should move to the battlefield"
    );
    assert!(
        !game
            .player(alice)
            .expect("alice exists")
            .hand
            .contains(&revealed_id),
        "accepted small permanent card should not also move to hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn matter_reshaper_declined_small_permanent_card_goes_to_hand() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let reshaper = matter_reshaper_definition();
    let reshaper_id = game.create_object_from_definition(&reshaper, alice, Zone::Battlefield);
    let top_card = matter_reshaper_test_card(72_808, "Declined Land", vec![CardType::Land], 0);
    let top_id = game.create_object_from_card(&top_card, alice, Zone::Library);
    let top_stable = game.object(top_id).expect("top card exists").stable_id;

    let mut dm = AutoPassDecisionMaker;
    resolve_matter_reshaper_death_trigger(&mut game, reshaper_id, &mut dm);

    let revealed_id = game
        .find_object_by_stable_id(top_stable)
        .expect("revealed card should still exist");
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .contains(&revealed_id),
        "declining the optional battlefield move should put the card into hand"
    );
    assert!(
        !game.battlefield.contains(&revealed_id),
        "declined card should not move to the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn matter_reshaper_nonpermanent_card_goes_to_hand() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let reshaper = matter_reshaper_definition();
    let reshaper_id = game.create_object_from_definition(&reshaper, alice, Zone::Battlefield);
    let top_card =
        matter_reshaper_test_card(72_809, "Revealed Instant", vec![CardType::Instant], 2);
    let top_id = game.create_object_from_card(&top_card, alice, Zone::Library);
    let top_stable = game.object(top_id).expect("top card exists").stable_id;

    let mut dm = SelectFirstDecisionMaker;
    resolve_matter_reshaper_death_trigger(&mut game, reshaper_id, &mut dm);

    let revealed_id = game
        .find_object_by_stable_id(top_stable)
        .expect("revealed card should still exist");
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .contains(&revealed_id),
        "nonpermanent revealed card should go to hand"
    );
    assert!(
        !game.battlefield.contains(&revealed_id),
        "nonpermanent revealed card should not move to the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn matter_reshaper_large_permanent_card_goes_to_hand() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let reshaper = matter_reshaper_definition();
    let reshaper_id = game.create_object_from_definition(&reshaper, alice, Zone::Battlefield);
    let top_card = matter_reshaper_test_card(72_810, "Large Creature", vec![CardType::Creature], 4);
    let top_id = game.create_object_from_card(&top_card, alice, Zone::Library);
    let top_stable = game.object(top_id).expect("top card exists").stable_id;

    let mut dm = SelectFirstDecisionMaker;
    resolve_matter_reshaper_death_trigger(&mut game, reshaper_id, &mut dm);

    let revealed_id = game
        .find_object_by_stable_id(top_stable)
        .expect("revealed card should still exist");
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .contains(&revealed_id),
        "permanent card with mana value greater than three should go to hand"
    );
    assert!(
        !game.battlefield.contains(&revealed_id),
        "large permanent card should not move to the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_portcullis_exiles_entrying_creature_and_returns_it_when_it_leaves() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let portcullis = CardDefinitionBuilder::new(CardId::from_raw(91_200), "Portcullis")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.",
        )
        .expect("Portcullis should parse");
    let portcullis_id = game.create_object_from_definition(&portcullis, alice, Zone::Battlefield);

    create_creature(&mut game, "Existing Creature One", alice, 2, 2);
    create_creature(&mut game, "Existing Creature Two", bob, 2, 2);

    let entering = CardBuilder::new(CardId::from_raw(91_201), "Portcullis Test Entrant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let entering_id = game.create_object_from_card(&entering, bob, Zone::Hand);

    assert!(
        game.move_object_by_effect(entering_id, Zone::Battlefield)
            .is_some(),
        "the creature should enter the battlefield"
    );

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Portcullis should trigger on the third creature entering"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Portcullis trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Portcullis trigger should resolve");

    let exiled = game.get_exiled_with_source_links(portcullis_id).to_vec();
    assert_eq!(
        exiled.len(),
        1,
        "Portcullis should track exactly one exiled creature"
    );
    assert!(
        game.object(exiled[0])
            .is_some_and(|obj| obj.zone == Zone::Exile),
        "the entering creature should be exiled while Portcullis is on the battlefield"
    );

    assert!(
        game.move_object_by_effect(portcullis_id, Zone::Graveyard)
            .is_some(),
        "Portcullis should leave the battlefield"
    );
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Portcullis return trigger should go on the stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("Portcullis return trigger should resolve");
    }

    assert!(
        game.battlefield.iter().any(|&id| {
            game.object(id).is_some_and(|obj| {
                obj.name == "Portcullis Test Entrant" && game.controller_of(obj) == bob
            })
        }),
        "the exiled creature should return to the battlefield under its owner's control"
    );
    assert!(
        game.get_exiled_with_source_links(portcullis_id).is_empty(),
        "Portcullis should no longer track exiled cards after it leaves"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_portcullis_does_not_trigger_without_two_other_creatures() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let portcullis = CardDefinitionBuilder::new(CardId::from_raw(91_202), "Portcullis")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.",
        )
        .expect("Portcullis should parse");
    let portcullis_id = game.create_object_from_definition(&portcullis, alice, Zone::Battlefield);

    create_creature(&mut game, "Lonely Creature", alice, 2, 2);

    let entering = CardBuilder::new(CardId::from_raw(91_203), "Portcullis Non-Trigger Entrant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let entering_id = game.create_object_from_card(&entering, alice, Zone::Hand);

    assert!(
        game.move_object_by_effect(entering_id, Zone::Battlefield)
            .is_some(),
        "the creature should enter the battlefield"
    );

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Portcullis trigger queue should resolve cleanly");

    assert!(
        game.get_exiled_with_source_links(portcullis_id).is_empty(),
        "Portcullis should not exile a creature when fewer than two other creatures are present"
    );
    assert!(
        game.battlefield.iter().any(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Portcullis Non-Trigger Entrant")
        }),
        "the entering creature should remain on the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_optional_trigger_target_stacks_without_legal_targets() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let gate_def = CardDefinitionBuilder::new(CardId::from_raw(91_000), "Optional Target Gate")
        .card_types(vec![CardType::Land])
        .parse_text("When this land enters, up to one target creature phases out.")
        .expect("optional ETB target should parse");
    let gate_id = game.create_object_from_definition(&gate_def, alice, Zone::Hand);

    let moved = game.move_object_by_effect(gate_id, Zone::Battlefield);
    assert!(
        moved.is_some(),
        "expected the land to enter the battlefield"
    );

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("optional ETB trigger should be put on the stack");

    assert_eq!(
        game.stack.len(),
        1,
        "optional target trigger should still go on the stack with no legal targets"
    );
    assert!(
        game.stack
            .last()
            .expect("trigger should be on the stack")
            .targets
            .is_empty(),
        "optional target trigger should not invent a target"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_optional_trigger_target_can_be_skipped_even_with_legal_targets() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    create_creature(&mut game, "Legal Target", bob, 2, 2);

    let gate_def = CardDefinitionBuilder::new(CardId::from_raw(91_001), "Optional Target Gate")
        .card_types(vec![CardType::Land])
        .parse_text("When this land enters, up to one target creature phases out.")
        .expect("optional ETB target should parse");
    let gate_id = game.create_object_from_definition(&gate_def, alice, Zone::Hand);

    let moved = game.move_object_by_effect(gate_id, Zone::Battlefield);
    assert!(
        moved.is_some(),
        "expected the land to enter the battlefield"
    );

    drain_pending_trigger_events(&mut game, &mut trigger_queue);

    let mut dm = DeclineOptionalTriggerTargetsDecisionMaker {
        seen_min_targets: None,
        seen_max_targets: None,
    };
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("optional ETB trigger should be put on the stack");

    assert_eq!(
        dm.seen_min_targets,
        Some(0),
        "optional trigger targeting should advertise zero required targets"
    );
    assert_eq!(
        dm.seen_max_targets,
        Some(Some(1)),
        "optional trigger targeting should preserve the one-target upper bound"
    );
    assert_eq!(game.stack.len(), 1, "expected the trigger on the stack");
    assert!(
        game.stack
            .last()
            .expect("trigger should be on the stack")
            .targets
            .is_empty(),
        "declining an optional trigger target should leave the trigger untargeted"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_toggo_landfall_creates_a_rock_token_with_an_activated_ability() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let toggo = CardDefinitionBuilder::new(CardId::new(), "Toggo, Goblin Weaponsmith")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Goblin, crate::types::Subtype::Artificer])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Landfall — Whenever a land you control enters, create a colorless Equipment artifact token named Rock with \"Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'\" and equip {1}.\nPartner (You can have two commanders if both have partner.)",
        )
        .expect("Toggo should parse");
    let toggo_id = game.create_object_from_definition(&toggo, alice, Zone::Battlefield);
    game.remove_summoning_sickness(toggo_id);

    let land = CardBuilder::new(CardId::from_raw(91_102), "Toggo Landfall Land")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, alice, Zone::Hand);
    assert!(
        game.move_object_by_effect(land_id, Zone::Battlefield)
            .is_some(),
        "the land should enter the battlefield"
    );

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Toggo's landfall trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Toggo's landfall trigger should resolve");

    let rock_id = game
        .battlefield
        .iter()
        .copied()
        .find(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Rock" && game.controller_of(obj) == alice)
        })
        .expect("Toggo should create a Rock token");
    let rock = game.object(rock_id).expect("Rock token should exist");
    assert_eq!(rock.name, "Rock");
    let activated_texts = rock
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(_) => crate::ability::ability_surface_text_for_tests(ability),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        activated_texts.iter().any(|text| text == "Equip {1}"),
        "Rock should keep its equip ability, got {activated_texts:?}"
    );
    assert!(
        rock.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::AttachedAbilityGrant
            )
        }),
        "Rock should keep the static grant for the quoted damage ability, got {:?}",
        rock.abilities
    );

    if let Some(rock) = game.object_mut(rock_id) {
        rock.attached_to = Some(crate::object::AttachmentTarget::Object(toggo_id));
    }
    if let Some(toggo) = game.object_mut(toggo_id) {
        toggo.attachments.push(rock_id);
    }
    let toggo_chars = game
        .calculated_characteristics(toggo_id)
        .expect("equipped Toggo should have calculated characteristics");
    let granted_activated_texts = toggo_chars
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(_) => crate::ability::ability_surface_text_for_tests(ability),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        granted_activated_texts.iter().any(|text| {
            text.contains("Sacrifice")
                && text.contains("This creature deals 2 damage to any target")
        }),
        "Equipped creature should gain the quoted Rock damage ability, got {granted_activated_texts:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cloud_ex_soldier_etb_allows_skipping_optional_equipment_target() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let cloud = CardDefinitionBuilder::new(CardId::from_raw(91_101), "Cloud, Ex-SOLDIER")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Haste\nWhen Cloud enters, attach up to one target Equipment you control to it.\nWhenever Cloud attacks, draw a card for each equipped attacking creature you control. Then if Cloud has power 7 or greater, create two Treasure tokens.",
        )
        .expect("Cloud, Ex-SOLDIER should parse");

    let equipment = CardBuilder::new(CardId::from_raw(91_102), "Bronze Blade")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    game.create_object_from_card(&equipment, alice, Zone::Battlefield);

    let cloud_id = game.create_object_from_definition(&cloud, alice, Zone::Hand);
    assert!(
        game.move_object_by_effect(cloud_id, Zone::Battlefield)
            .is_some(),
        "Cloud should enter the battlefield"
    );

    drain_pending_trigger_events(&mut game, &mut trigger_queue);

    let mut dm = DeclineOptionalTriggerTargetsDecisionMaker {
        seen_min_targets: None,
        seen_max_targets: None,
    };
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Cloud ETB trigger should go on stack");

    assert_eq!(dm.seen_min_targets, Some(0));
    assert_eq!(dm.seen_max_targets, Some(Some(1)));
    assert_eq!(game.stack.len(), 1, "Cloud ETB trigger should be queued");
    assert!(
        game.stack
            .last()
            .expect("Cloud ETB trigger should exist")
            .targets
            .is_empty(),
        "declining optional target should leave Cloud ETB untargeted"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cloud_ex_soldier_attack_trigger_draws_for_equipped_attackers_and_makes_treasures_at_power_7()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let cloud = CardDefinitionBuilder::new(CardId::from_raw(91_111), "Cloud, Ex-SOLDIER")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Haste\nWhen Cloud enters, attach up to one target Equipment you control to it.\nWhenever Cloud attacks, draw a card for each equipped attacking creature you control. Then if Cloud has power 7 or greater, create two Treasure tokens.",
        )
        .expect("Cloud, Ex-SOLDIER should parse");
    let cloud_id = game.create_object_from_definition(&cloud, alice, Zone::Battlefield);
    game.remove_summoning_sickness(cloud_id);
    game.object_mut(cloud_id)
        .expect("Cloud should exist")
        .add_counters(crate::object::CounterType::PlusOnePlusOne, 3);

    let wingman_id = create_creature(&mut game, "Attacking Wingman", alice, 2, 2);
    game.remove_summoning_sickness(wingman_id);

    let cloud_equipment = CardBuilder::new(CardId::from_raw(91_112), "Cloud Blade")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let cloud_equipment_id =
        game.create_object_from_card(&cloud_equipment, alice, Zone::Battlefield);
    let wingman_equipment = CardBuilder::new(CardId::from_raw(91_113), "Wingman Blade")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let wingman_equipment_id =
        game.create_object_from_card(&wingman_equipment, alice, Zone::Battlefield);

    if let Some(equipment) = game.object_mut(cloud_equipment_id) {
        equipment.attached_to = Some(crate::object::AttachmentTarget::Object(cloud_id));
    }
    game.object_mut(cloud_id)
        .expect("Cloud should exist")
        .attachments
        .push(cloud_equipment_id);

    if let Some(equipment) = game.object_mut(wingman_equipment_id) {
        equipment.attached_to = Some(crate::object::AttachmentTarget::Object(wingman_id));
    }
    game.object_mut(wingman_id)
        .expect("wingman should exist")
        .attachments
        .push(wingman_equipment_id);

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![
        AttackerDeclaration {
            creature: cloud_id,
            target: AttackTarget::Player(bob),
        },
        AttackerDeclaration {
            creature: wingman_id,
            target: AttackTarget::Player(bob),
        },
    ];
    apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations)
        .expect("attackers should be legal");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Cloud attack trigger should go on stack");
    assert_eq!(
        game.stack.len(),
        1,
        "Cloud should create one attack trigger"
    );
    resolve_stack_entry(&mut game).expect("Cloud attack trigger should resolve");

    let treasure_count = game
        .battlefield
        .iter()
        .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Treasure"))
        .count();
    assert_eq!(
        treasure_count, 2,
        "Cloud should create two Treasures at power 7 or greater"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cloud_ex_soldier_attack_trigger_skips_treasures_below_power_7() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let cloud = CardDefinitionBuilder::new(CardId::from_raw(91_121), "Cloud, Ex-SOLDIER")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Haste\nWhen Cloud enters, attach up to one target Equipment you control to it.\nWhenever Cloud attacks, draw a card for each equipped attacking creature you control. Then if Cloud has power 7 or greater, create two Treasure tokens.",
        )
        .expect("Cloud, Ex-SOLDIER should parse");
    let cloud_id = game.create_object_from_definition(&cloud, alice, Zone::Battlefield);
    game.remove_summoning_sickness(cloud_id);
    game.object_mut(cloud_id)
        .expect("Cloud should exist")
        .add_counters(crate::object::CounterType::PlusOnePlusOne, 2);

    let wingman_id = create_creature(&mut game, "Attacking Wingman", alice, 2, 2);
    game.remove_summoning_sickness(wingman_id);

    let cloud_equipment = CardBuilder::new(CardId::from_raw(91_122), "Cloud Blade")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let cloud_equipment_id =
        game.create_object_from_card(&cloud_equipment, alice, Zone::Battlefield);
    let wingman_equipment = CardBuilder::new(CardId::from_raw(91_123), "Wingman Blade")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let wingman_equipment_id =
        game.create_object_from_card(&wingman_equipment, alice, Zone::Battlefield);

    if let Some(equipment) = game.object_mut(cloud_equipment_id) {
        equipment.attached_to = Some(crate::object::AttachmentTarget::Object(cloud_id));
    }
    game.object_mut(cloud_id)
        .expect("Cloud should exist")
        .attachments
        .push(cloud_equipment_id);

    if let Some(equipment) = game.object_mut(wingman_equipment_id) {
        equipment.attached_to = Some(crate::object::AttachmentTarget::Object(wingman_id));
    }
    game.object_mut(wingman_id)
        .expect("wingman should exist")
        .attachments
        .push(wingman_equipment_id);

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    let declarations = vec![
        AttackerDeclaration {
            creature: cloud_id,
            target: AttackTarget::Player(bob),
        },
        AttackerDeclaration {
            creature: wingman_id,
            target: AttackTarget::Player(bob),
        },
    ];
    apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations)
        .expect("attackers should be legal");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Cloud attack trigger should go on stack");
    assert_eq!(
        game.stack.len(),
        1,
        "Cloud should create one attack trigger"
    );
    resolve_stack_entry(&mut game).expect("Cloud attack trigger should resolve");

    let treasure_count = game
        .battlefield
        .iter()
        .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Treasure"))
        .count();
    assert_eq!(
        treasure_count, 0,
        "Cloud should not create Treasures below power 7"
    );
}

pub(super) fn bridge_from_below_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(472), "Bridge from Below")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a nontoken creature is put into your graveyard from the battlefield, if this card is in your graveyard, create a 2/2 black Zombie creature token.\nWhen a creature is put into an opponent's graveyard from the battlefield, if this card is in your graveyard, exile this card.",
        )
        .expect("Bridge from Below should parse")
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn test_bridge_from_below_triggers_from_graveyard_on_your_creature_dying() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let bridge_id =
        game.create_object_from_definition(&bridge_from_below_definition(), alice, Zone::Graveyard);
    let victim_id = create_creature(&mut game, "Butcher Ghoul Test", alice, 1, 1);

    let moved = game.move_object_by_effect(victim_id, Zone::Graveyard);
    assert!(moved.is_some(), "expected creature to move to graveyard");

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Bridge from Below should trigger from the graveyard: {:?}",
        trigger_queue.entries
    );
    assert_eq!(
        trigger_queue.entries[0].source, bridge_id,
        "Bridge from Below should be the trigger source"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Bridge trigger should be put on stack");
    resolve_stack_entry(&mut game).expect("Bridge trigger should resolve");

    let zombie_count = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| game.controller_of(obj) == alice && obj.name == "Zombie")
        .count();
    assert_eq!(zombie_count, 1, "Bridge should create one Zombie token");
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn test_bridge_from_below_exiles_itself_when_opponents_creature_dies() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let _bridge_id =
        game.create_object_from_definition(&bridge_from_below_definition(), alice, Zone::Graveyard);
    let victim_id = create_creature(&mut game, "Opponent Creature Test", bob, 2, 2);

    let moved = game.move_object_by_effect(victim_id, Zone::Graveyard);
    assert!(
        moved.is_some(),
        "expected opponent creature to move to graveyard"
    );

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Bridge exile trigger should fire from the graveyard"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Bridge exile trigger should be put on stack");
    resolve_stack_entry(&mut game).expect("Bridge exile trigger should resolve");

    let exiled_bridge = game
        .exile
        .iter()
        .filter_map(|&id| game.object(id))
        .any(|obj| game.controller_of(obj) == alice && obj.name == "Bridge from Below");
    assert!(exiled_bridge, "Bridge should exile itself");
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn test_bridge_from_below_token_trigger_fizzles_if_bridge_leaves_graveyard() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let bridge_id =
        game.create_object_from_definition(&bridge_from_below_definition(), alice, Zone::Graveyard);
    let victim_id = create_creature(&mut game, "Butcher Ghoul Test", alice, 1, 1);

    let moved = game.move_object_by_effect(victim_id, Zone::Graveyard);
    assert!(moved.is_some(), "expected creature to move to graveyard");

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Bridge token trigger should fire before Bridge leaves the graveyard"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Bridge trigger should be put on stack");
    let moved_bridge = game.move_object_by_effect(bridge_id, Zone::Exile);
    assert!(
        moved_bridge.is_some(),
        "expected Bridge to leave the graveyard before the trigger resolves"
    );

    resolve_stack_entry(&mut game).expect("Bridge trigger should resolve cleanly");

    let zombie_count = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| game.controller_of(obj) == alice && obj.name == "Zombie")
        .count();
    assert_eq!(
        zombie_count, 0,
        "Bridge should not create a Zombie after it leaves the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resize_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(46_512), "Resize")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature gets +3/+3 until end of turn.\nRecover {1}{G}")
        .expect("Resize should parse with recover")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn resize_recover_paid_returns_resize_from_graveyard_to_hand() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let resize_id =
        game.create_object_from_definition(&resize_definition(), alice, Zone::Graveyard);
    let victim_id = create_creature(&mut game, "Resize Recover Victim", alice, 1, 1);
    {
        let player = game.player_mut(alice).expect("alice exists");
        player.mana_pool.add(ManaSymbol::Colorless, 1);
        player.mana_pool.add(ManaSymbol::Green, 1);
    }

    assert!(
        game.move_object_by_effect(victim_id, Zone::Graveyard)
            .is_some(),
        "owned creature should move to graveyard"
    );
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Resize recover should trigger"
    );
    assert_eq!(trigger_queue.entries[0].source, resize_id);

    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Resize recover trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Resize recover trigger should resolve");

    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .any(|&id| game.object(id).is_some_and(|obj| obj.name == "Resize")),
        "paying recover should return Resize to hand"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").mana_pool.total(),
        0,
        "recover payment should spend {{1}}{{G}}"
    );
    assert!(
        game.exile
            .iter()
            .all(|&id| !game.object(id).is_some_and(|obj| obj.name == "Resize")),
        "paid recover should not exile Resize"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn resize_recover_declined_exiles_resize_from_graveyard() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let resize_id =
        game.create_object_from_definition(&resize_definition(), alice, Zone::Graveyard);
    let victim_id = create_creature(&mut game, "Resize Decline Victim", alice, 1, 1);

    assert!(
        game.move_object_by_effect(victim_id, Zone::Graveyard)
            .is_some(),
        "owned creature should move to graveyard"
    );
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Resize recover should trigger"
    );
    assert_eq!(trigger_queue.entries[0].source, resize_id);

    let mut dm = AutoPassDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Resize recover trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Resize recover trigger should resolve");

    assert!(
        game.exile
            .iter()
            .any(|&id| game.object(id).is_some_and(|obj| obj.name == "Resize")),
        "declining recover should exile Resize"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .all(|&id| !game.object(id).is_some_and(|obj| obj.name == "Resize")),
        "declined recover should not return Resize to hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn resize_recover_does_not_exile_resize_if_it_left_graveyard_before_resolution() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let resize_id =
        game.create_object_from_definition(&resize_definition(), alice, Zone::Graveyard);
    let resize_stable_id = game.object(resize_id).expect("Resize exists").stable_id;
    let victim_id = create_creature(&mut game, "Resize Moved Victim", alice, 1, 1);

    assert!(
        game.move_object_by_effect(victim_id, Zone::Graveyard)
            .is_some(),
        "owned creature should move to graveyard"
    );
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Resize recover should trigger"
    );

    let mut dm = AutoPassDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Resize recover trigger should go on the stack");
    let moved_resize_id = game
        .move_object_by_effect(resize_id, Zone::Hand)
        .expect("Resize should move to hand before recover resolves");
    resolve_stack_entry_with(&mut game, &mut dm).expect("Resize recover trigger should resolve");

    assert_eq!(
        game.find_object_by_stable_id(resize_stable_id),
        Some(moved_resize_id),
        "Resize should still be the moved object after the trigger resolves"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .any(|&id| id == moved_resize_id),
        "recover should not exile Resize from hand after it left the graveyard"
    );
    assert!(
        game.exile
            .iter()
            .all(|&id| !game.object(id).is_some_and(|obj| obj.name == "Resize")),
        "recover should not exile Resize from another zone"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn resize_recover_does_not_trigger_for_opponents_creature() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let resize_id =
        game.create_object_from_definition(&resize_definition(), alice, Zone::Graveyard);
    let victim_id = create_creature(&mut game, "Opponent Resize Victim", bob, 1, 1);

    assert!(
        game.move_object_by_effect(victim_id, Zone::Graveyard)
            .is_some(),
        "opponent creature should move to graveyard"
    );
    drain_pending_trigger_events(&mut game, &mut trigger_queue);

    assert!(
        trigger_queue.entries.is_empty(),
        "Resize recover should not trigger for an opponent-owned creature dying"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .graveyard
            .iter()
            .any(|&id| id == resize_id && game.object(id).is_some_and(|obj| obj.name == "Resize")),
        "Resize should remain in the graveyard when recover does not trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn skeleton_crew_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(640_045), "Skeleton Crew")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Skeleton, Subtype::Pirate])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Each other creature you control that's a Skeleton or Pirate gets +1/+1.\n\
             Whenever one or more creature cards leave your graveyard, create a 2/2 black Skeleton Pirate creature token. (This ability triggers only from the battlefield.)\n\
             {5}{B}: Return this card from your graveyard to the battlefield tapped.",
        )
        .expect("Skeleton Crew should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_skeleton_crew_test_card(
    game: &mut GameState,
    name: &str,
    owner: PlayerId,
    zone: Zone,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    power: i32,
    toughness: i32,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .subtypes(subtypes)
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build();
    game.create_object_from_card(&card, owner, zone)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn skeleton_crew_anthem_buffs_other_skeletons_and_pirates_only() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let crew_id =
        game.create_object_from_definition(&skeleton_crew_definition(), alice, Zone::Battlefield);
    let skeleton_id = create_skeleton_crew_test_card(
        &mut game,
        "Ally Skeleton",
        alice,
        Zone::Battlefield,
        vec![CardType::Creature],
        vec![Subtype::Skeleton],
        2,
        2,
    );
    let pirate_id = create_skeleton_crew_test_card(
        &mut game,
        "Ally Pirate",
        alice,
        Zone::Battlefield,
        vec![CardType::Creature],
        vec![Subtype::Pirate],
        2,
        2,
    );
    let bear_id = create_skeleton_crew_test_card(
        &mut game,
        "Ally Bear",
        alice,
        Zone::Battlefield,
        vec![CardType::Creature],
        vec![Subtype::Bear],
        2,
        2,
    );
    let opponent_pirate_id = create_skeleton_crew_test_card(
        &mut game,
        "Opponent Pirate",
        bob,
        Zone::Battlefield,
        vec![CardType::Creature],
        vec![Subtype::Pirate],
        2,
        2,
    );

    assert_eq!(game.calculated_power(skeleton_id), Some(3));
    assert_eq!(game.calculated_toughness(skeleton_id), Some(3));
    assert_eq!(game.calculated_power(pirate_id), Some(3));
    assert_eq!(game.calculated_toughness(pirate_id), Some(3));
    assert_eq!(game.calculated_power(bear_id), Some(2));
    assert_eq!(game.calculated_toughness(bear_id), Some(2));
    assert_eq!(game.calculated_power(opponent_pirate_id), Some(2));
    assert_eq!(game.calculated_toughness(opponent_pirate_id), Some(2));
    assert_eq!(game.calculated_power(crew_id), Some(3));
    assert_eq!(game.calculated_toughness(crew_id), Some(3));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn skeleton_crew_creates_token_when_your_creature_card_leaves_graveyard() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    game.create_object_from_definition(&skeleton_crew_definition(), alice, Zone::Battlefield);
    let creature_card_id = create_skeleton_crew_test_card(
        &mut game,
        "Graveyard Creature",
        alice,
        Zone::Graveyard,
        vec![CardType::Creature],
        vec![Subtype::Zombie],
        2,
        2,
    );

    let moved = game.move_object_by_effect(creature_card_id, Zone::Battlefield);
    assert!(moved.is_some(), "creature card should leave your graveyard");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Skeleton Crew should trigger once when a creature card leaves your graveyard"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Skeleton Crew trigger should be put on stack");
    resolve_stack_entry(&mut game).expect("Skeleton Crew trigger should resolve");

    let skeleton_pirate_tokens = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id).is_some_and(|object| {
                object.kind == ObjectKind::Token
                    && game.calculated_subtypes(id).contains(&Subtype::Skeleton)
                    && game.calculated_subtypes(id).contains(&Subtype::Pirate)
            })
        })
        .count();
    assert_eq!(
        skeleton_pirate_tokens, 1,
        "Skeleton Crew should create one Skeleton Pirate token"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn skeleton_crew_one_or_more_graveyard_leave_batches_once() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    game.create_object_from_definition(&skeleton_crew_definition(), alice, Zone::Battlefield);
    let first_id = create_skeleton_crew_test_card(
        &mut game,
        "First Graveyard Creature",
        alice,
        Zone::Graveyard,
        vec![CardType::Creature],
        vec![Subtype::Zombie],
        2,
        2,
    );
    let second_id = create_skeleton_crew_test_card(
        &mut game,
        "Second Graveyard Creature",
        alice,
        Zone::Graveyard,
        vec![CardType::Creature],
        vec![Subtype::Zombie],
        2,
        2,
    );
    let lookback_source_snapshots = game.trigger_source_lookback_snapshots();
    let snapshots = [first_id, second_id]
        .into_iter()
        .map(|id| {
            let object = game.object(id).expect("graveyard creature should exist");
            crate::snapshot::ObjectSnapshot::from_object(object, &game)
        })
        .collect();
    let event = TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::batch_with_snapshots(
            vec![first_id, second_id],
            Zone::Graveyard,
            Zone::Exile,
            crate::events::cause::EventCause::effect(),
            snapshots,
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(lookback_source_snapshots);

    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Skeleton Crew should trigger once when multiple creature cards leave your graveyard together"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn skeleton_crew_does_not_trigger_for_noncreatures_opponents_graveyard_or_off_battlefield()
 {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let battlefield_crew_id =
        game.create_object_from_definition(&skeleton_crew_definition(), alice, Zone::Battlefield);
    let noncreature_id = create_skeleton_crew_test_card(
        &mut game,
        "Graveyard Artifact",
        alice,
        Zone::Graveyard,
        vec![CardType::Artifact],
        vec![],
        0,
        0,
    );
    let opponent_creature_id = create_skeleton_crew_test_card(
        &mut game,
        "Opponent Graveyard Creature",
        bob,
        Zone::Graveyard,
        vec![CardType::Creature],
        vec![Subtype::Zombie],
        2,
        2,
    );

    assert!(
        game.move_object_by_effect(noncreature_id, Zone::Battlefield)
            .is_some()
    );
    assert!(
        game.move_object_by_effect(opponent_creature_id, Zone::Battlefield)
            .is_some()
    );
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "Skeleton Crew should ignore noncreature cards and opponents' graveyards"
    );

    assert!(
        game.move_object_by_effect(battlefield_crew_id, Zone::Exile)
            .is_some()
    );
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    trigger_queue.entries.clear();

    let graveyard_crew_id =
        game.create_object_from_definition(&skeleton_crew_definition(), alice, Zone::Graveyard);
    let second_creature_id = create_skeleton_crew_test_card(
        &mut game,
        "Second Graveyard Creature",
        alice,
        Zone::Graveyard,
        vec![CardType::Creature],
        vec![Subtype::Zombie],
        2,
        2,
    );

    assert!(
        game.move_object_by_effect(graveyard_crew_id, Zone::Exile)
            .is_some()
    );
    assert!(
        game.move_object_by_effect(second_creature_id, Zone::Battlefield)
            .is_some()
    );
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "Skeleton Crew's creature-card trigger should function only from the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn skeleton_crew_graveyard_activation_returns_it_tapped() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let crew_id =
        game.create_object_from_definition(&skeleton_crew_definition(), alice, Zone::Graveyard);
    let crew_stable_id = game
        .object(crew_id)
        .expect("Skeleton Crew should exist")
        .stable_id;
    {
        let player = game.player_mut(alice).expect("Alice should exist");
        player.mana_pool.add(ManaSymbol::Colorless, 5);
        player.mana_pool.add(ManaSymbol::Black, 1);
    }

    let ability_index = game
        .object(crew_id)
        .expect("Skeleton Crew should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Skeleton Crew should have a graveyard activated ability");
    let activate_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == crew_id && *idx == ability_index
            )
        })
        .expect("Skeleton Crew graveyard activation should be legal with mana available");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Skeleton Crew activation should be put on the stack");
    resolve_stack_entry(&mut game).expect("Skeleton Crew activation should resolve");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "Skeleton Crew should not trigger from seeing itself leave the graveyard"
    );

    let returned_id = game
        .find_object_by_stable_id(crew_stable_id)
        .expect("Skeleton Crew should still be tracked after changing zones");
    assert!(
        game.battlefield.contains(&returned_id),
        "Skeleton Crew should return to the battlefield"
    );
    assert!(
        game.is_tapped(returned_id),
        "Skeleton Crew should return tapped"
    );
}

#[test]
pub(super) fn test_mortuary_triggers_for_owned_creatures_even_if_control_changed() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let mortuary_id = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Mortuary Probe")
            .card_types(vec![CardType::Enchantment])
            .build(),
        alice,
        Zone::Battlefield,
    );
    if let Some(obj) = game.object_mut(mortuary_id) {
        obj.abilities_mut().push(Ability::triggered(
            Trigger::dies(crate::target::ObjectFilter::creature().owned_by(PlayerFilter::You)),
            crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::tag_triggering_object("triggering"),
                Effect::move_to_zone(
                    crate::target::ChooseSpec::Tagged("triggering".into()),
                    Zone::Library,
                    true,
                ),
            ]),
        ));
    }

    let alice_owned_creature = create_creature(&mut game, "Alice Creature", alice, 2, 2);
    game.set_current_controller(alice_owned_creature, bob);

    let moved = game.move_object_by_effect(alice_owned_creature, Zone::Graveyard);
    assert!(moved.is_some(), "owned creature should move to graveyard");

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Mortuary should trigger when a creature you own dies, even if another player controlled it"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("Mortuary trigger should stack");
    resolve_stack_entry(&mut game).expect("Mortuary trigger should resolve");

    assert_eq!(
        game.player(alice)
            .expect("alice exists")
            .library
            .last()
            .and_then(|id| game.object(*id))
            .map(|object| object.name.as_str()),
        Some("Alice Creature"),
        "Mortuary should put the owned creature onto the top of your library"
    );

    let bob_owned_creature = create_creature(&mut game, "Bob Creature", bob, 2, 2);
    game.set_current_controller(bob_owned_creature, alice);

    let moved = game.move_object_by_effect(bob_owned_creature, Zone::Graveyard);
    assert!(moved.is_some(), "bob creature should move to graveyard");

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "Mortuary should not trigger when a creature you don't own dies, even if you controlled it"
    );
    assert!(
        game.player(bob)
            .expect("bob exists")
            .graveyard
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "Bob Creature")),
        "the non-owned creature should remain in its owner's graveyard"
    );
}

#[test]
pub(super) fn destroy_all_batches_dies_events_for_sources_destroyed_at_the_same_time() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut decision_maker = SelectFirstDecisionMaker;

    let harvester = create_creature(&mut game, "Harvester Probe", alice, 5, 5);
    if let Some(obj) = game.object_mut(harvester) {
        obj.abilities_mut().push(Ability::triggered(
            Trigger::dies(crate::target::ObjectFilter::creature().nontoken().other()),
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
        ));
    }
    create_creature(&mut game, "Alice Creature", alice, 2, 2);
    create_creature(&mut game, "Bob Creature", bob, 2, 2);

    let destroy_all = Effect::destroy_all(crate::target::ObjectFilter::creature());
    let mut ctx = crate::effects::ExecutionContext::new(harvester, alice, &mut decision_maker);
    crate::effects::execute_effect(&mut game, &destroy_all, &mut ctx)
        .expect("destroy all should resolve");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);

    assert_eq!(
        trigger_queue.entries.len(),
        2,
        "a source destroyed with other creatures should trigger once for each other nontoken creature"
    );
}

#[test]
pub(super) fn simultaneous_sba_deaths_use_source_lki_for_all_dying_objects() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let necromancer = create_creature(&mut game, "Necromancer Probe", alice, 2, 2);
    if let Some(obj) = game.object_mut(necromancer) {
        obj.subtypes.push(Subtype::Human);
        obj.abilities_mut().push(Ability::triggered(
            Trigger::either(
                Trigger::this_dies(),
                Trigger::dies(
                    crate::target::ObjectFilter::creature()
                        .with_subtype(Subtype::Human)
                        .you_control(),
                ),
            ),
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
        ));
    }

    let human_a = create_creature(&mut game, "Human A", alice, 1, 1);
    let human_b = create_creature(&mut game, "Human B", alice, 1, 1);
    for id in [human_a, human_b] {
        if let Some(obj) = game.object_mut(id) {
            obj.subtypes.push(Subtype::Human);
        }
    }

    game.mark_damage(necromancer, 2);
    game.mark_damage(human_a, 1);
    game.mark_damage(human_b, 1);

    let mut trigger_queue = TriggerQueue::new();
    check_and_apply_sbas(&mut game, &mut trigger_queue).unwrap();

    assert_eq!(
        trigger_queue.entries.len(),
        3,
        "a dies-trigger source should see all simultaneous SBA deaths using LKI"
    );
}

#[test]
pub(super) fn test_parsed_mortuary_moves_owned_creature_from_graveyard_to_library_top() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let mortuary = CardDefinitionBuilder::new(CardId::from_raw(257_401), "Mortuary")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a creature is put into your graveyard from the battlefield, put that card on top of your library.",
        )
        .expect("Mortuary should parse");
    let ability_debug = format!("{:#?}", mortuary.abilities);
    let ability_debug_compact = ability_debug.split_whitespace().collect::<String>();
    assert!(
        ability_debug_compact.contains("ZoneChangeTrigger")
            && ability_debug_compact.contains("owner:Some(You")
            && ability_debug_compact.contains("MoveToZoneEffect")
            && ability_debug_compact.contains("zone:Library")
            && ability_debug_compact.contains("to_top:true"),
        "expected parsed Mortuary to build the owned-creature graveyard trigger, got {ability_debug}"
    );

    game.create_object_from_definition(&mortuary, alice, Zone::Battlefield);

    let alice_owned_creature =
        create_creature(&mut game, "Borrowed Mortuary Creature", alice, 2, 2);
    game.set_current_controller(alice_owned_creature, bob);

    let moved = game.move_object_by_effect(alice_owned_creature, Zone::Graveyard);
    assert!(moved.is_some(), "owned creature should move to graveyard");

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "parsed Mortuary should trigger for a creature you own even when another player controlled it"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("Mortuary trigger should stack");
    resolve_stack_entry(&mut game).expect("Mortuary trigger should resolve");

    assert_eq!(
        game.player(alice)
            .expect("alice exists")
            .library
            .last()
            .and_then(|id| game.object(*id))
            .map(|object| object.name.as_str()),
        Some("Borrowed Mortuary Creature"),
        "parsed Mortuary should put the owned creature card on top of your library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_stangg_linked_twin_sacrifice_survives_legend_rule_for_other_twin() {
    use crate::ability::AbilityKind;
    use crate::cards::CardDefinitionBuilder;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::events::zones::EnterBattlefieldEvent;
    use crate::ids::CardId;
    use crate::triggers::TriggerEvent;
    use crate::zone::Zone;

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let oracle = "When Stangg enters, create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. Exile that token when Stangg leaves the battlefield. Sacrifice Stangg when that token leaves the battlefield.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Stangg")
        .parse_text(oracle)
        .expect("parse stangg text");
    let etb = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{:?}", triggered.effects).contains("CreateTokenEffect") =>
            {
                Some(triggered.clone())
            }
            _ => None,
        })
        .expect("expected stangg ETB trigger");

    let stangg_a = create_creature(&mut game, "Stangg", alice, 3, 4);
    let stangg_b = create_creature(&mut game, "Stangg", alice, 3, 4);

    for source in [stangg_a, stangg_b] {
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let event = TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::new(source, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_triggering_event(event);
        for effect in &etb.effects {
            execute_effect(&mut game, effect, &mut ctx).expect("stangg ETB effect should resolve");
        }
    }

    let twins_before_sba = game
        .battlefield
        .iter()
        .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Stangg Twin"))
        .count();
    assert_eq!(
        twins_before_sba, 2,
        "expected two Stangg Twin tokens before legend rule applies"
    );

    check_and_apply_sbas(&mut game, &mut trigger_queue).expect("apply SBAs");

    let twins_after_sba = game
        .battlefield
        .iter()
        .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Stangg Twin"))
        .count();
    assert_eq!(twins_after_sba, 1, "legend rule should keep only one Twin");

    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("queue triggered abilities");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("resolve trigger");
    }

    let stangg_after_resolution = game
        .battlefield
        .iter()
        .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Stangg"))
        .count();
    assert_eq!(
        stangg_after_resolution, 1,
        "only the Stangg linked to the Twin that left should be sacrificed"
    );
}

#[test]
pub(super) fn test_turn_face_up_action_puts_turned_face_up_trigger_on_stack() {
    use crate::decision::{LegalAction, SelectFirstDecisionMaker};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::triggers::Trigger;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let card = CardBuilder::new(CardId::from_raw(42), "Face-Up Trigger Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

    if let Some(obj) = game.object_mut(creature_id) {
        obj.abilities_mut()
            .push(Ability::static_ability(StaticAbility::morph(
                crate::cost::TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Green]])),
            )));
        obj.abilities_mut().push(Ability::triggered(
            Trigger::this_is_turned_face_up(),
            vec![Effect::draw(1)],
        ));
    }
    game.set_face_down(creature_id);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 1);

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let response = PriorityResponse::PriorityAction(LegalAction::TurnFaceUp {
        creature_id,
        method: crate::special_actions::TurnFaceUpMethod::TurnFaceUpAbility,
    });
    let result = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &response,
        &mut dm,
    );
    assert!(result.is_ok(), "turn-face-up action should succeed");

    let top = game
        .stack
        .last()
        .expect("triggered ability should be on stack");
    assert!(
        top.is_ability,
        "turned-face-up trigger should use ability stack entry"
    );
    assert_eq!(top.object_id, creature_id);
    assert!(
        top.triggering_event
            .as_ref()
            .is_some_and(|event| event.kind() == EventKind::TurnedFaceUp),
        "trigger stack entry should carry TurnedFaceUp trigger event"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kadenas_silencer_turn_face_up_trigger_counters_only_opponent_abilities() {
    use crate::decision::{LegalAction, SelectFirstDecisionMaker};
    use crate::special_actions::TurnFaceUpMethod;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let kadena = CardDefinitionBuilder::new(CardId::new(), "Kadena's Silencer")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Snake, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text(
            "When this creature is turned face up, counter all abilities your opponents control.\nMegamorph {1}{U}",
        )
        .expect("Kadena's Silencer should parse for runtime test");
    let kadena_id = game.create_object_from_definition(&kadena, alice, Zone::Battlefield);
    game.set_face_down(kadena_id);

    let opponent_first_ability_source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Opponent Ability Source A")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    let opponent_second_ability_source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Opponent Ability Source B")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    let your_ability_source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Your Ability Source")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );
    let opponent_spell = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Opponent Spell")
            .card_types(vec![CardType::Instant])
            .build(),
        bob,
        Zone::Stack,
    );

    game.push_to_stack(StackEntry::ability(
        opponent_first_ability_source,
        bob,
        vec![Effect::draw(1)],
    ));
    game.push_to_stack(StackEntry::ability(
        opponent_second_ability_source,
        bob,
        vec![Effect::draw(1)],
    ));
    game.push_to_stack(StackEntry::ability(
        your_ability_source,
        alice,
        vec![Effect::draw(1)],
    ));
    game.push_to_stack(StackEntry::new(opponent_spell, bob));

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let response = PriorityResponse::PriorityAction(LegalAction::TurnFaceUp {
        creature_id: kadena_id,
        method: TurnFaceUpMethod::MegamorphAbility,
    });
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &response,
        &mut dm,
    )
    .expect("turning Kadena's Silencer face up should succeed");

    let top = game
        .stack
        .last()
        .expect("Kadena's Silencer trigger should be on the stack");
    assert_eq!(top.object_id, kadena_id);
    assert!(top.is_ability);
    resolve_stack_entry(&mut game).expect("Kadena's Silencer trigger should resolve");

    assert!(
        !game
            .stack
            .iter()
            .any(|entry| entry.object_id == opponent_first_ability_source),
        "first opponent ability should be countered"
    );
    assert!(
        !game
            .stack
            .iter()
            .any(|entry| entry.object_id == opponent_second_ability_source),
        "second opponent ability should be countered"
    );
    assert!(
        game.stack
            .iter()
            .any(|entry| entry.object_id == your_ability_source && entry.is_ability),
        "your ability should not be countered"
    );
    assert!(
        game.stack
            .iter()
            .any(|entry| entry.object_id == opponent_spell && !entry.is_ability),
        "opponent spell should not be countered"
    );
}
