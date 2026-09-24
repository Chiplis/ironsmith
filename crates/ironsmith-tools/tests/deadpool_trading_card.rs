use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::ids::CardId;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::{Ability, CardType, GameState, PlayerId, Zone};

fn definition() -> ironsmith::cards::CardDefinition {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Deadpool, Trading Card",
    )
    .unwrap()
    .remove(0);
    ironsmith_tools::compile_definition_from_payload(&payload).unwrap()
}

#[test]
fn entry_exchanges_source_with_a_nontarget_creature_and_preserves_other_characteristics() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let partner = CardDefinitionBuilder::new(CardId::new(), "Exchange partner")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 4))
        .with_ability(Ability::static_ability(StaticAbility::flying()))
        .with_ability(Ability::static_ability(StaticAbility::shroud()))
        .build();
    let partner = game.create_object_from_definition(&partner, bob, Zone::Battlefield);
    let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .unwrap()
        .new_id;
    assert!(
        game.object_has_static_ability_id(entered, StaticAbilityId::Flying),
        "entering object must receive the exchanged text before entry completes"
    );
    assert!(game.object_has_static_ability_id(entered, StaticAbilityId::Shroud));
    assert!(!game.object_has_static_ability_id(partner, StaticAbilityId::Flying));
    let chars = game.calculated_characteristics(entered).unwrap();
    assert_eq!(chars.power, Some(5));
    assert_eq!(chars.toughness, Some(3));
    assert_eq!(game.current_controller(partner), Some(bob));
    game.move_object_by_effect(entered, Zone::Graveyard)
        .unwrap();
    assert!(
        !game.object_has_static_ability_id(partner, StaticAbilityId::Flying),
        "the other creature retains its exchanged text after the source leaves"
    );
}

#[test]
fn exchange_is_optional_and_no_other_creature_is_required_for_entry() {
    struct Decline;
    impl ironsmith::decision::DecisionMaker for Decline {}
    let alice = PlayerId::from_index(0);
    for has_partner in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let partner = has_partner.then(|| {
            let card = CardDefinitionBuilder::new(CardId::new(), "Optional partner")
                .card_types(vec![CardType::Creature])
                .with_ability(Ability::static_ability(StaticAbility::flying()))
                .build();
            game.create_object_from_definition(&card, alice, Zone::Battlefield)
        });
        let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
        let mut dm = Decline;
        let entered = game
            .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
            .unwrap()
            .new_id;
        assert!(!game.object_has_static_ability_id(entered, StaticAbilityId::Flying));
        if let Some(partner) = partner {
            assert!(game.object_has_static_ability_id(partner, StaticAbilityId::Flying));
        }
    }
}

#[test]
fn acquired_entry_replacement_applies_during_the_same_entry() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let partner = CardDefinitionBuilder::new(CardId::new(), "Tapped entry partner")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::static_ability(
            StaticAbility::enters_tapped_ability(),
        ))
        .build();
    game.create_object_from_definition(&partner, alice, Zone::Battlefield);
    let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .unwrap()
        .new_id;
    assert!(
        game.is_tapped(entered),
        "acquired entry replacement must affect the still-pending entry event"
    );
}

#[test]
fn acquired_entry_counter_replacement_applies_once() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let counter = ironsmith::object::CounterType::PlusOnePlusOne;
    let partner = CardDefinitionBuilder::new(CardId::new(), "Counter entry partner")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::static_ability(
            StaticAbility::enters_with_counters_value(counter, ironsmith::effect::Value::Fixed(3)),
        ))
        .build();
    game.create_object_from_definition(&partner, alice, Zone::Battlefield);
    let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .unwrap()
        .new_id;
    assert_eq!(
        game.object(entered)
            .unwrap()
            .counters
            .get(&counter)
            .copied(),
        Some(3)
    );
}

#[test]
fn acquired_entry_choice_is_made_for_the_new_object() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let partner = CardDefinitionBuilder::new(CardId::new(), "Color choice partner")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::static_ability(
            StaticAbility::choose_color_as_enters(
                None,
                "As this creature enters, choose a color.".into(),
            ),
        ))
        .build();
    let partner = game.create_object_from_definition(&partner, alice, Zone::Battlefield);
    game.set_chosen_color(partner, ironsmith::color::Color::Red);
    let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .unwrap()
        .new_id;
    assert!(
        game.chosen_color(entered).is_some(),
        "gained entry choice must run for the entering object"
    );
    assert_ne!(
        game.chosen_color(entered),
        Some(ironsmith::color::Color::Red),
        "the former object's choice is not copied"
    );
}

#[test]
fn exchanged_enter_trigger_fires_and_the_upkeep_drawback_moves_to_partner() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let partner = CardDefinitionBuilder::new(CardId::new(), "Triggered partner")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::triggered(
            ironsmith::triggers::Trigger::enters_battlefield(
                ironsmith::target::ObjectFilter {
                    source: true,
                    ..Default::default()
                },
                None,
            ),
            vec![ironsmith::effect::Effect::gain_life(2)],
        ))
        .build();
    let partner = game.create_object_from_definition(&partner, bob, Zone::Battlefield);
    let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .unwrap()
        .new_id;
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(
        game.stack.len(),
        1,
        "the acquired trigger must see the actual entry"
    );
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 22);
    let upkeep = |player| {
        ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfUpkeepEvent::new(player),
            Default::default(),
        )
    };
    assert!(ironsmith::triggers::check_triggers(&game, &upkeep(alice)).is_empty());
    let entries = ironsmith::triggers::check_triggers(&game, &upkeep(bob));
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, partner);
    for entry in entries {
        queue.add(entry);
    }
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(bob).unwrap().life, 17);
    assert_eq!(
        game.object(entered).unwrap().name.as_ref(),
        "Deadpool, Trading Card"
    );
}

#[test]
fn acquired_entry_program_executes_once() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let program = vec![ironsmith::effect::Effect::gain_life(3)].into();
    let model = ironsmith::static_abilities::CompiledStaticAbility::as_enters_effect_program(
        program,
        "this creature",
        false,
        false,
        None,
    );
    let partner = CardDefinitionBuilder::new(CardId::new(), "Entry program partner")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::static_ability(StaticAbility::from_model(model)))
        .build();
    game.create_object_from_definition(&partner, alice, Zone::Battlefield);
    let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .unwrap();
    assert_eq!(
        game.player(alice).unwrap().life,
        23,
        "the new entry program must execute once during the same entry"
    );
}

#[test]
fn swapped_sacrifice_ability_pays_its_cost_and_draws_for_each_other_player() {
    use ironsmith::decision::LegalAction;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let fixture = CardDefinitionBuilder::new(CardId::new(), "Vanilla partner")
        .card_types(vec![CardType::Creature])
        .build();
    let partner = game.create_object_from_definition(&fixture, bob, Zone::Battlefield);
    for player in [alice, bob, charlie] {
        game.create_object_from_definition(&fixture, player, Zone::Library);
    }
    let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .unwrap();
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(bob)
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Colorless, 2);
    let is_activation = |action: &LegalAction| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == partner);
    // Actions open a mana-ability window before checking exact payment.
    assert!(
        ironsmith::decision::compute_legal_actions(&game, bob)
            .iter()
            .any(is_activation)
    );
    game.player_mut(bob)
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Colorless, 1);
    let action = ironsmith::decision::compute_legal_actions(&game, bob)
        .into_iter()
        .find(is_activation)
        .unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = ironsmith::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &ironsmith::game_loop::PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..16 {
        if !game.stack.is_empty() {
            break;
        }
        let ironsmith::decision::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("{progress:?}");
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &context, &mut dm,
        )
        .unwrap();
    }
    assert_eq!(game.stack.len(), 1);
    assert!(
        game.object(partner).is_none(),
        "sacrifice is paid before resolution"
    );
    assert_eq!(
        game.player(bob).unwrap().mana_pool.total(),
        0,
        "all three mana must be paid before resolution"
    );
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(game.player(bob).unwrap().hand.len(), 0);
    assert_eq!(game.player(charlie).unwrap().hand.len(), 1);
}

#[test]
fn entry_without_candidates_succeeds_and_multiple_candidates_choose_one() {
    let alice = PlayerId::from_index(0);
    for count in [0, 2] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = CardDefinitionBuilder::new(CardId::new(), "Selectable partner")
            .card_types(vec![CardType::Creature])
            .with_ability(Ability::static_ability(StaticAbility::flying()))
            .build();
        let partners: Vec<_> = (0..count)
            .map(|_| game.create_object_from_definition(&card, alice, Zone::Battlefield))
            .collect();
        let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let entered = game
            .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
            .unwrap()
            .new_id;
        assert_eq!(
            game.object_has_static_ability_id(entered, StaticAbilityId::Flying),
            count > 0
        );
        let exchanged = partners
            .iter()
            .filter(|id| !game.object_has_static_ability_id(**id, StaticAbilityId::Flying))
            .count();
        assert_eq!(
            exchanged,
            usize::from(count > 0),
            "exchange selects exactly one available partner"
        );
    }
}

#[test]
fn acquired_copy_replacement_changes_the_pending_entry() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut filter = ironsmith::target::ObjectFilter::creature();
    filter.name = Some("Copy destination".into());
    let spec = ironsmith::static_abilities::EnterAsCopyAsEntersSpec {
        filter: filter.clone(),
        affected_filter: None,
        may: false,
        enters_tapped_if_chosen: false,
        copy_duration: None,
        linked_exile_pair: None,
        copy_source_self: false,
        copy_source_enchanted: false,
        name_override: None,
        added_colors: ironsmith::color::ColorSet::new(),
        added_card_types: vec![],
        removes_other_card_types: false,
        added_supertypes: vec![],
        removed_supertypes: vec![],
        added_subtypes: vec![],
        added_abilities: vec![],
        set_base_power_toughness: None,
        additional_counters: vec![],
        additional_counters_source_filter: None,
        added_abilities_source_filter: None,
        set_base_power_toughness_from_self: false,
        conditional_additional_counters: vec![],
    };
    let partner = CardDefinitionBuilder::new(CardId::new(), "Copy replacement partner")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::static_ability(
            StaticAbility::with_enter_as_copy_as_enters(
                spec,
                "Enter as a copy of the matching creature.".into(),
            ),
        ))
        .with_ability(Ability::static_ability(
            StaticAbility::enters_with_counters_for_filter(
                filter,
                ironsmith::object::CounterType::Charge,
                1,
            ),
        ))
        .build();
    game.create_object_from_definition(&partner, alice, Zone::Battlefield);
    let copy = CardDefinitionBuilder::new(CardId::new(), "Copy destination")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(7, 8))
        .build();
    game.create_object_from_definition(&copy, alice, Zone::Battlefield);
    let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .unwrap()
        .new_id;
    let chars = game.calculated_characteristics(entered).unwrap();
    assert_eq!(
        chars.power,
        Some(7),
        "newly acquired copy replacement must run"
    );
    assert_eq!(chars.toughness, Some(8));
    assert_eq!(
        game.object(entered)
            .unwrap()
            .counters
            .get(&ironsmith::object::CounterType::Charge)
            .copied(),
        None,
        "the former source lost its replacement and the entrant's gained global replacement does not affect its own entry (CR 614.12)"
    );
    assert_eq!(
        game.object(entered).unwrap().name.as_ref(),
        "Copy destination"
    );
}

#[test]
fn text_exchange_is_not_copied_and_ends_for_each_object_on_leaving() {
    struct Decline;
    impl ironsmith::decision::DecisionMaker for Decline {}
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardDefinitionBuilder::new(CardId::new(), "Printed flying partner")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::static_ability(StaticAbility::flying()))
        .build();
    let partner = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let mut accept = ironsmith::decision::SelectFirstDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut accept)
        .unwrap()
        .new_id;
    for (original, expected_flying) in [(partner, true), (entered, false)] {
        let previous = game.battlefield.clone();
        let mut decline = Decline;
        let mut context = ironsmith::effects::EffectContext::new(entered, alice, &mut decline);
        ironsmith::effects::execute_effect(
            &mut game,
            &ironsmith::effect::Effect::create_token_copy(
                ironsmith::target::ChooseSpec::SpecificObject(original),
            ),
            &mut context,
        )
        .unwrap();
        let created: Vec<_> = game
            .battlefield
            .iter()
            .filter(|id| !previous.contains(id))
            .copied()
            .collect();
        assert_eq!(created.len(), 1);
        assert_eq!(
            game.object_has_static_ability_id(created[0], StaticAbilityId::Flying),
            expected_flying,
            "copying uses layer-1 values, excluding the text exchange"
        );
    }
    let buried_partner = game
        .move_object_by_effect(partner, Zone::Graveyard)
        .unwrap();
    assert!(game.object_has_static_ability_id(entered, StaticAbilityId::Flying));
    let mut decline = Decline;
    let returned_partner = game
        .move_object_with_etb_processing_with_dm(buried_partner, Zone::Battlefield, &mut decline)
        .unwrap()
        .new_id;
    assert!(game.object_has_static_ability_id(returned_partner, StaticAbilityId::Flying));
    let buried_source = game
        .move_object_by_effect(entered, Zone::Graveyard)
        .unwrap();
    let returned_source = game
        .move_object_with_etb_processing_with_dm(buried_source, Zone::Battlefield, &mut decline)
        .unwrap()
        .new_id;
    assert!(!game.object_has_static_ability_id(returned_source, StaticAbilityId::Flying));
}

#[test]
fn strict_snapshot_has_supported_structure_and_unrounded_score() {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Deadpool, Trading Card",
    )
    .unwrap()
    .remove(0);
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload);
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
    println!("Unrounded similarity: {:?}", snapshot.similarity_score);
}

#[test]
fn controller_can_order_text_exchange_before_an_external_entry_replacement() {
    struct ChooseOrder {
        preferred: ironsmith::ObjectId,
        offered: bool,
    }
    impl ironsmith::decision::DecisionMaker for ChooseOrder {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
        fn decide_objects(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::SelectObjectsContext,
        ) -> Vec<ironsmith::ObjectId> {
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .take(1)
                .map(|c| c.id)
                .collect()
        }
        fn decide_options(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if let Some(option) = ctx
                .options
                .iter()
                .find(|o| o.legal && o.object_id == Some(self.preferred))
            {
                self.offered = true;
                vec![option.index]
            } else {
                ctx.options
                    .iter()
                    .find(|o| o.legal)
                    .map(|o| vec![o.index])
                    .unwrap_or_default()
            }
        }
    }
    let alice = PlayerId::from_index(0);
    for exchange_first in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card =
            CardDefinitionBuilder::new(CardId::new(), "Global tapped partner")
                .card_types(vec![CardType::Creature])
                .with_ability(Ability::static_ability(
                    StaticAbility::enters_tapped_for_filter(
                        ironsmith::target::ObjectFilter::creature(),
                    ),
                ))
                .build();
        let partner = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
        let mut dm = ChooseOrder {
            preferred: if exchange_first { hand } else { partner },
            offered: false,
        };
        let entered = game
            .move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
            .unwrap()
            .new_id;
        assert!(
            dm.offered,
            "both ordinary replacements must be offered to the affected controller (CR616.1e)"
        );
        assert_eq!(
            game.is_tapped(entered),
            !exchange_first,
            "exchanging first removes the external tapped replacement; the gained global ability cannot affect its own entry"
        );
    }
}

#[test]
fn copying_deadpool_before_entry_exchanges_the_copied_text_box() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let partner = CardDefinitionBuilder::new(CardId::new(), "Flying partner")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::static_ability(StaticAbility::flying()))
        .build();
    let partner = game.create_object_from_definition(&partner, alice, Zone::Battlefield);
    game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
    let mut filter = ironsmith::target::ObjectFilter::creature();
    filter.name = Some("Deadpool, Trading Card".into());
    let spec = ironsmith::static_abilities::EnterAsCopyAsEntersSpec {
        filter,
        affected_filter: None,
        may: false,
        enters_tapped_if_chosen: false,
        copy_duration: None,
        linked_exile_pair: None,
        copy_source_self: false,
        copy_source_enchanted: false,
        name_override: None,
        added_colors: ironsmith::color::ColorSet::new(),
        added_card_types: vec![],
        removes_other_card_types: false,
        added_supertypes: vec![],
        removed_supertypes: vec![],
        added_subtypes: vec![],
        added_abilities: vec![],
        set_base_power_toughness: None,
        additional_counters: vec![],
        additional_counters_source_filter: None,
        added_abilities_source_filter: None,
        set_base_power_toughness_from_self: false,
        conditional_additional_counters: vec![],
    };
    let clone = CardDefinitionBuilder::new(CardId::new(), "Unprinted replica")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::static_ability(StaticAbility::with_enter_as_copy_as_enters(
            spec, "Enter as a copy of the matching creature.".into(),
        )))
        .build();
    let hand = game.create_object_from_definition(&clone, alice, Zone::Hand);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let entered = game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm).unwrap().new_id;
    assert!(game.object_has_static_ability_id(entered, StaticAbilityId::Flying));
    let partner_abilities = game.calculated_characteristics(partner).unwrap().abilities.clone();
    assert_eq!(partner_abilities.len(), 3, "the partner receives Deadpool's copied text, not the replica's printed copy replacement");
    assert!(partner_abilities.iter().any(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_))));
    assert!(partner_abilities.iter().any(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))));
}
