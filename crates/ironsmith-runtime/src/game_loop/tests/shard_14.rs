#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_everflowing_chalice_two_kicks() {
    use crate::cards::definitions::everflowing_chalice;
    use crate::cost::OptionalCostsPaid;
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::object::CounterType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Everflowing Chalice directly on battlefield
    let chalice_def = everflowing_chalice();
    let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

    // Simulate that it entered with 2 kicks
    let mut paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
    paid.pay_times(0, 2); // Pay multikicker twice
    let mut ctx = ExecutionContext::new_default(chalice_id, alice)
        .with_optional_costs_paid(paid)
        .with_targets(vec![ResolvedTarget::Object(chalice_id)]);

    // Execute the ETB effect
    let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
    execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

    // Should have 2 charge counters
    let chalice = game.object(chalice_id).unwrap();
    assert_eq!(
        chalice.counters.get(&CounterType::Charge),
        Some(&2),
        "Should have 2 charge counters with 2 kicks"
    );

    // Tap for mana - should produce 2 colorless
    let mana_effect = Effect::add_colorless_mana(Value::CountersOnSource(CounterType::Charge));
    let mut mana_ctx = ExecutionContext::new_default(chalice_id, alice);
    execute_effect(&mut game, &mana_effect, &mut mana_ctx).unwrap();

    assert_eq!(
        game.player(alice).unwrap().mana_pool.colorless,
        2,
        "Should produce 2 colorless mana with 2 counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_everflowing_chalice_etb_trigger_uses_object_kick_count() {
    // This test verifies that when an ETB trigger fires, it can read
    // the kick count from the permanent that entered (not from ctx)
    use crate::cards::definitions::everflowing_chalice;
    use crate::cost::OptionalCostsPaid;
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::object::CounterType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Everflowing Chalice directly on battlefield
    let chalice_def = everflowing_chalice();
    let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

    // Set the optional_costs_paid on the object itself (simulating what
    // resolve_stack_entry does when a permanent enters)
    {
        let chalice = game.object_mut(chalice_id).unwrap();
        let mut paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
        paid.pay_times(0, 3); // 3 kicks
        chalice.optional_costs_paid = paid;
    }

    // Now execute the ETB effect with an EMPTY context (simulating a trigger)
    // The effect should still read the kick count from the source object
    let mut ctx = ExecutionContext::new_default(chalice_id, alice)
        .with_targets(vec![ResolvedTarget::Object(chalice_id)]);
    // Note: ctx.optional_costs_paid is empty, but the source object has it

    let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
    execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

    // Should have 3 charge counters (read from source object)
    let chalice = game.object(chalice_id).unwrap();
    assert_eq!(
        chalice.counters.get(&CounterType::Charge),
        Some(&3),
        "Should have 3 charge counters (read from object's optional_costs_paid)"
    );
}

// =========================================================================
// Force of Will / Alternative Cost Tests
// =========================================================================

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_force_of_will_alternative_cost_available() {
    use crate::cards::definitions::force_of_will;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up - alice needs something to counter
    // Put a spell on the stack that bob cast
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Create a spell on the stack for alice to counter
    use crate::cards::definitions::lightning_bolt;
    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
    game.stack.push(StackEntry::new(bolt_id, bob));

    // Give alice Force of Will in hand
    let fow_def = force_of_will();
    let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

    // Give alice another blue card in hand to exile (an Island counts as blue for this test)
    // Actually, lands are colorless. Let's use a Counterspell instead.
    use crate::cards::definitions::counterspell;
    let cs_def = counterspell();
    let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

    // Give alice 20 life (default)
    game.player_mut(alice).unwrap().life = 20;

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should find alternative cost option
    let alt_cost_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == fow_id
        )
    });

    assert!(
        alt_cost_action.is_some(),
        "Should be able to cast Force of Will with alternative cost when blue card available"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_force_of_will_alternative_cost_casting_flow() {
    use crate::alternative_cast::CastingMethod;
    use crate::cards::definitions::force_of_will;
    use crate::triggers::TriggerQueue;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up - alice needs something to counter
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Create a spell on the stack for alice to counter
    use crate::cards::definitions::lightning_bolt;
    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
    game.stack.push(StackEntry::new(bolt_id, bob));

    // Give alice Force of Will in hand
    let fow_def = force_of_will();
    let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

    // Give alice another blue card in hand to exile
    use crate::cards::definitions::counterspell;
    let cs_def = counterspell();
    let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

    // Give alice 20 life
    game.player_mut(alice).unwrap().life = 20;

    // Verify alternative method has non-mana costs but no mana cost
    let fow_obj = game.object(fow_id).unwrap();
    assert_eq!(fow_obj.alternative_casts.len(), 1);
    let method = &fow_obj.alternative_casts[0];
    assert!(
        !method.non_mana_costs().is_empty(),
        "Force of Will should have non-mana costs"
    );
    assert!(
        method.mana_cost().is_none(),
        "Force of Will alternative should NOT have a mana cost"
    );

    // Now test the casting flow
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();

    // Execute the CastSpell action via apply_priority_response
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: fow_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Alternative(0),
    });

    let result = apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
    assert!(result.is_ok(), "CastSpell action should succeed");

    // The result should be a target selection decision
    let progress = result.unwrap();
    match &progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_)) => {
            // Good - now let's choose the target (Lightning Bolt)
        }
        _ => {
            panic!(
                "Expected Targets context decision after casting Force of Will, got {:?}",
                progress
            );
        }
    }

    // Now handle the target selection
    let pending = state.pending_cast.take().unwrap();
    let target = Target::Object(bolt_id);
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let result = continue_to_mana_payment(
        &mut game,
        &mut trigger_queue,
        &mut state,
        pending,
        vec![target],
        &mut dm,
    );

    let next_cost_ctx = match result {
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        )) => ctx,
        other => panic!(
            "expected next-cost chooser for Force of Will alternative cost, got {:?}",
            other
        ),
    };
    let exile_cost_index = next_cost_ctx
        .options
        .iter()
        .find(|opt| opt.description.to_ascii_lowercase().contains("exile"))
        .map(|opt| opt.index)
        .expect("expected an exile cost option");

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let choose_exile_cost = PriorityResponse::NextCostChoice(exile_cost_index);
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &choose_exile_cost,
        &mut dm,
    )
    .expect("should choose exile cost first");

    match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_),
        ) => {}
        other => panic!(
            "expected exile-from-hand chooser after selecting Force of Will exile cost, got {:?}",
            other
        ),
    }

    let blue_card_id = game
        .player(alice)
        .expect("Alice exists")
        .hand
        .iter()
        .copied()
        .find(|&id| id != fow_id)
        .expect("expected another blue card in hand");
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::CardCostChoice(blue_card_id),
        &mut dm,
    )
    .expect("should finish paying Force of Will after exiling a blue card");

    // Verify the alternative costs were paid
    // - Life should have decreased by 1
    let life = game.player(alice).unwrap().life;
    assert_eq!(life, 19, "Alice should have paid 1 life (got {})", life);

    // - The blue card should have been exiled
    // Note: move_object changes the ObjectId, so we need to look in exile
    let exiled_blue_card = game.exile.iter().any(|&id| {
        if let Some(obj) = game.object(id) {
            obj.name == "Counterspell"
        } else {
            false
        }
    });
    assert!(
        exiled_blue_card,
        "Blue card (Counterspell) should be in exile"
    );

    // - Force of Will should be on the stack
    assert!(
        game.stack.iter().any(|e| {
            if let Some(obj) = game.object(e.object_id) {
                obj.name == "Force of Will"
            } else {
                false
            }
        }),
        "Force of Will should be on the stack"
    );
    let force_entry = game
        .stack
        .iter()
        .find(|e| {
            game.object(e.object_id)
                .is_some_and(|obj| obj.name == "Force of Will")
        })
        .expect("Force of Will stack entry should exist");
    let exiled = force_entry
        .tagged_objects
        .get(&crate::tag::TagKey::from("exile_cost"))
        .expect("Force of Will stack entry should keep the exiled-card tag");
    assert_eq!(exiled.len(), 1);
    assert_eq!(exiled[0].name, "Counterspell");
}

#[test]
pub(super) fn test_non_mana_only_flashback_does_not_require_printed_mana_cost() {
    use crate::alternative_cast::{AlternativeCastingMethod, CastingMethod};
    use crate::cost::TotalCost;
    use crate::costs::Cost;
    use crate::decision::compute_legal_actions;
    use crate::effect::ChoiceCount;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let flashback_spell = CardDefinitionBuilder::new(CardId::new(), "Non-Mana Flashback Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::return_from_graveyard_to_battlefield(
            crate::target::ChooseSpec::Target(Box::new(crate::target::ChooseSpec::Object(
                ObjectFilter::creature()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You),
            ))),
            false,
        )])
        .alternative_cast(AlternativeCastingMethod::Flashback {
            total_cost: TotalCost::from_costs(vec![
                Cost::validated_effect(Effect::choose_objects(
                    ObjectFilter::creature().you_control(),
                    ChoiceCount::exactly(3),
                    PlayerFilter::You,
                    "flashback_sacrifice_cost".to_string(),
                )),
                Cost::validated_effect(Effect::sacrifice(
                    ObjectFilter::tagged("flashback_sacrifice_cost"),
                    3,
                )),
            ]),
        })
        .build();
    let spell_id = game.create_object_from_definition(&flashback_spell, alice, Zone::Graveyard);

    let creature = CardBuilder::new(CardId::new(), "Cost Creature")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(0)]]))
        .card_types(vec![CardType::Creature])
        .build();
    for _ in 0..3 {
        game.create_object_from_card(&creature, alice, Zone::Battlefield);
    }
    game.create_object_from_card(&creature, alice, Zone::Graveyard);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::Alternative(0),
            } if *id == spell_id
        )),
        "non-mana-only flashback should be legal without paying printed mana"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_force_of_will_alternative_cost_not_available_without_card() {
    use crate::cards::definitions::force_of_will;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Create a spell on the stack for alice to counter
    use crate::cards::definitions::lightning_bolt;
    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
    game.stack.push(StackEntry::new(bolt_id, bob));

    // Give alice Force of Will in hand (this is her ONLY card)
    let fow_def = force_of_will();
    let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

    // Give alice 20 life
    game.player_mut(alice).unwrap().life = 20;

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should NOT find alternative cost option (no other blue card to exile)
    let alt_cost_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == fow_id
        )
    });

    assert!(
        alt_cost_action.is_none(),
        "Should NOT be able to use alternative cost without another blue card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_force_of_negation_resolution_counters_and_exiles_target_spell() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::cards::definitions::lightning_bolt;
    use crate::game_loop::resolve_stack_entry;
    use crate::ids::CardId;
    use crate::types::CardType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
    let bolt_stable_id = game
        .object(bolt_id)
        .expect("bolt should exist on the stack")
        .stable_id;
    game.push_to_stack(StackEntry::new(bolt_id, bob));

    let fon_def = CardDefinitionBuilder::new(CardId::new(), "Force of Negation Test")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target noncreature spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.",
        )
        .expect("Force of Negation test text should parse");
    let fon_id = game.create_object_from_definition(&fon_def, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(fon_id, alice).with_targets(vec![Target::Object(bolt_id)]));

    resolve_stack_entry(&mut game).expect("Force of Negation should resolve");

    let moved_bolt_id = game
        .find_object_by_stable_id(bolt_stable_id)
        .expect("countered spell should still be findable by stable id");
    assert_eq!(
        game.object(moved_bolt_id)
            .expect("countered spell should still exist after resolution")
            .zone,
        Zone::Exile,
        "Force of Negation should exile the spell it counters"
    );
    assert!(
        !game.stack.iter().any(|entry| entry.object_id == bolt_id),
        "countered spell should no longer be on the stack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_force_of_negation_exiles_nexus_of_fate_instead_of_shuffling_it() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::game_loop::resolve_stack_entry;
    use crate::ids::CardId;
    use crate::types::CardType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let nexus_def = CardDefinitionBuilder::new(CardId::new(), "Nexus of Fate Test")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(5),
            ManaSymbol::Blue,
            ManaSymbol::Blue,
        ]))
        .parse_text(
            "Take an extra turn after this one.\nIf Nexus of Fate would be put into a graveyard from anywhere, reveal Nexus of Fate and shuffle it into its owner's library instead.",
        )
        .expect("Nexus of Fate test text should parse");
    let nexus_id = game.create_object_from_definition(&nexus_def, bob, Zone::Stack);
    let nexus_stable_id = game
        .object(nexus_id)
        .expect("Nexus should exist on the stack")
        .stable_id;
    game.push_to_stack(StackEntry::new(nexus_id, bob));

    let library_before = game.player(bob).expect("bob should exist").library.len();

    let fon_def = CardDefinitionBuilder::new(CardId::new(), "Force of Negation Test")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target noncreature spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.",
        )
        .expect("Force of Negation test text should parse");
    let fon_id = game.create_object_from_definition(&fon_def, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(fon_id, alice).with_targets(vec![Target::Object(nexus_id)]));
    let (_, _, all_targets_invalid) = validate_stack_entry_targets(
        &game,
        game.stack
            .last()
            .expect("Force of Negation should be on the stack"),
    );
    assert!(
        !all_targets_invalid,
        "Nexus of Fate should still be a legal target for Force of Negation at resolution"
    );

    resolve_stack_entry(&mut game).expect("Force of Negation should resolve");

    let moved_nexus_id = game
        .find_object_by_stable_id(nexus_stable_id)
        .expect("countered Nexus should still be findable by stable id");
    assert_eq!(
        game.object(moved_nexus_id)
            .expect("countered Nexus should still exist after resolution")
            .zone,
        Zone::Exile,
        "Force of Negation should exile Nexus of Fate instead of letting it shuffle"
    );
    assert_eq!(
        game.player(bob).expect("bob should exist").library.len(),
        library_before,
        "Nexus of Fate should not get shuffled into its owner's library"
    );
    assert!(
        !game.stack.iter().any(|entry| entry.object_id == nexus_id),
        "countered Nexus should no longer be on the stack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn invasive_surgery_test_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Invasive Surgery")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target sorcery spell.\nDelirium — If there are four or more card types among cards in your graveyard, search the graveyard, hand, and library of that spell's controller for any number of cards with the same name as that spell, exile those cards, then that player shuffles.",
        )
        .expect("Invasive Surgery should parse strictly for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn invasive_surgery_target_sorcery(name: &str) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Black]))
        .card_types(vec![CardType::Sorcery])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_invasive_surgery_delirium_cards(
    game: &mut GameState,
    controller: PlayerId,
    count: usize,
) {
    let type_sets = [
        ("Delirium Artifact", vec![CardType::Artifact]),
        ("Delirium Creature", vec![CardType::Creature]),
        ("Delirium Enchantment", vec![CardType::Enchantment]),
        ("Delirium Land", vec![CardType::Land]),
    ];
    for (name, card_types) in type_sets.into_iter().take(count) {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .build();
        game.create_object_from_definition(&def, controller, Zone::Graveyard);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[derive(Default)]
pub(super) struct InvasiveSurgeryDecisionMaker {
    pub(super) object_selection_calls: usize,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for InvasiveSurgeryDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.object_selection_calls += 1;
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_invasive_surgery_with_delirium_exiles_same_name_cards_from_controller_zones() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    put_invasive_surgery_delirium_cards(&mut game, alice, 4);

    let target_def = invasive_surgery_target_sorcery("Duplicated Sorcery");
    let stack_copy = game.create_object_from_definition(&target_def, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(stack_copy, bob));
    game.create_object_from_definition(&target_def, bob, Zone::Graveyard);
    game.create_object_from_definition(&target_def, bob, Zone::Hand);
    game.create_object_from_definition(&target_def, bob, Zone::Library);
    let other_def = invasive_surgery_target_sorcery("Different Sorcery");
    game.create_object_from_definition(&other_def, bob, Zone::Library);

    let invasive_surgery = invasive_surgery_test_definition();
    let invasive_id = game.create_object_from_definition(&invasive_surgery, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(invasive_id, alice).with_targets(vec![Target::Object(stack_copy)]),
    );

    let mut decisions = InvasiveSurgeryDecisionMaker::default();
    resolve_stack_entry_with(&mut game, &mut decisions).expect("Invasive Surgery should resolve");

    assert_eq!(
        decisions.object_selection_calls, 1,
        "delirium branch should perform one same-name multi-zone search"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Exile, "Duplicated Sorcery"),
        4,
        "delirium branch should exile the countered spell and same-name cards from graveyard, hand, and library"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Graveyard, "Duplicated Sorcery"),
        0,
        "same-name graveyard cards should be exiled after the counter resolves"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Hand, "Duplicated Sorcery"),
        0,
        "same-name hand cards should be exiled"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Library, "Duplicated Sorcery"),
        0,
        "same-name library cards should be exiled"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Library, "Different Sorcery"),
        1,
        "cards with a different name should remain in the searched player's library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_invasive_surgery_without_delirium_only_counters_target_sorcery() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    put_invasive_surgery_delirium_cards(&mut game, alice, 3);

    let target_def = invasive_surgery_target_sorcery("Duplicated Sorcery");
    let stack_copy = game.create_object_from_definition(&target_def, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(stack_copy, bob));
    game.create_object_from_definition(&target_def, bob, Zone::Graveyard);
    game.create_object_from_definition(&target_def, bob, Zone::Hand);
    game.create_object_from_definition(&target_def, bob, Zone::Library);

    let invasive_surgery = invasive_surgery_test_definition();
    let invasive_id = game.create_object_from_definition(&invasive_surgery, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(invasive_id, alice).with_targets(vec![Target::Object(stack_copy)]),
    );

    let mut decisions = InvasiveSurgeryDecisionMaker::default();
    resolve_stack_entry_with(&mut game, &mut decisions).expect("Invasive Surgery should resolve");

    assert_eq!(
        decisions.object_selection_calls, 0,
        "non-delirium branch should not perform the same-name search"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Exile, "Duplicated Sorcery"),
        0,
        "without delirium, Invasive Surgery should not exile same-name cards"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Graveyard, "Duplicated Sorcery"),
        2,
        "without delirium, only the target sorcery should be countered into the graveyard alongside the existing copy"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Hand, "Duplicated Sorcery"),
        1,
        "same-name hand card should remain when delirium is not satisfied"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Library, "Duplicated Sorcery"),
        1,
        "same-name library card should remain when delirium is not satisfied"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_force_of_will_alternative_cost_not_available_with_only_nonblue_card() {
    use crate::cards::definitions::force_of_will;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Create a spell on the stack for alice to counter
    use crate::cards::definitions::lightning_bolt;
    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
    game.stack.push(StackEntry::new(bolt_id, bob));

    // Give alice Force of Will in hand
    let fow_def = force_of_will();
    let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

    // Give alice a non-blue card (Lightning Bolt is red)
    let red_card_def = lightning_bolt();
    let _red_card_id = game.create_object_from_definition(&red_card_def, alice, Zone::Hand);

    // Give alice 20 life
    game.player_mut(alice).unwrap().life = 20;

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should NOT find alternative cost option (no blue card to exile)
    let alt_cost_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == fow_id
        )
    });

    assert!(
        alt_cost_action.is_none(),
        "Should NOT be able to use alternative cost with only non-blue cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_force_of_will_normal_cast_available_with_mana() {
    use crate::cards::definitions::force_of_will;
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Create a spell on the stack for alice to counter
    use crate::cards::definitions::lightning_bolt;
    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
    game.stack.push(StackEntry::new(bolt_id, bob));

    // Give alice Force of Will in hand
    let fow_def = force_of_will();
    let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

    // Give alice enough mana to cast normally: {3}{U}{U}
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 3);

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should find normal cast option
    let normal_cast = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *spell_id == fow_id
        )
    });

    assert!(
        normal_cast.is_some(),
        "Should be able to cast Force of Will normally with 3UU"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_force_of_will_both_options_available() {
    use crate::cards::definitions::{counterspell, force_of_will, lightning_bolt};
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Create a spell on the stack for alice to counter
    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
    game.stack.push(StackEntry::new(bolt_id, bob));

    // Give alice Force of Will in hand
    let fow_def = force_of_will();
    let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

    // Give alice another blue card (for alternative cost)
    let cs_def = counterspell();
    let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

    // Give alice enough mana to cast normally: {3}{U}{U}
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 3);

    // Give alice 20 life
    game.player_mut(alice).unwrap().life = 20;

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Legal-action generation now exposes each available native casting method directly.
    let normal_cast = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *spell_id == fow_id
        )
    });

    let alt_cast = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == fow_id
        )
    });

    assert!(normal_cast.is_some(), "Should be able to cast normally");
    assert!(
        alt_cast.is_some(),
        "Alternative should be exposed as a separate legal action when available"
    );

    // Count total CastSpell actions for Force of Will from hand
    let fow_cast_count = actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    ..
                } if *spell_id == fow_id
            )
        })
        .count();
    assert_eq!(
        fow_cast_count, 2,
        "Should have normal and alternative CastSpell actions for Force of Will"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_choose_casting_method_flow() {
    use crate::cards::definitions::{counterspell, force_of_will, lightning_bolt};
    use crate::decision::GameProgress;
    use crate::mana::ManaSymbol;
    use crate::triggers::TriggerQueue;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Create a spell on the stack for alice to counter
    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
    game.stack.push(StackEntry::new(bolt_id, bob));

    // Give alice Force of Will in hand
    let fow_def = force_of_will();
    let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Hand);

    // Give alice another blue card (for alternative cost)
    let cs_def = counterspell();
    let _blue_card_id = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

    // Give alice enough mana to cast normally: {3}{U}{U}
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 3);

    // Give alice 20 life
    game.player_mut(alice).unwrap().life = 20;

    // Now test the ChooseCastingMethod flow
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();

    // Cast using Normal method - should trigger ChooseCastingMethod since both methods available
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: fow_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });

    let result = apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);

    // Should get a ChooseCastingMethod decision
    match result {
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        )) => {
            assert_eq!(ctx.player, alice);
            assert_eq!(ctx.source, Some(fow_id));
            assert_eq!(ctx.options.len(), 2, "Should have 2 casting method options");
            assert!(ctx.description.contains("Choose casting method"));
        }
        other => panic!(
            "Expected SelectOptions context for casting method, got {:?}",
            other
        ),
    }

    // Now choose the alternative cost (index 1)
    let method_response = PriorityResponse::CastingMethodChoice(1);
    let result =
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &method_response);

    // Should get ChooseTargets decision next (Force of Will targets a spell)
    // After targets, it will ask for card to exile
    match result {
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(ctx),
        )) => {
            assert_eq!(ctx.player, alice, "Should be alice choosing targets");
        }
        other => panic!(
            "Expected Targets context decision after method choice, got {:?}",
            other
        ),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn brain_in_a_jar_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(93_000), "Brain in a Jar")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{1}, {T}: Put a charge counter on this artifact, then you may cast an instant or sorcery spell with mana value equal to the number of charge counters on this artifact from your hand without paying its mana cost.\n{3}, {T}, Remove X charge counters from this artifact: Scry X.",
        )
        .expect("Brain in a Jar should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn brain_in_a_jar_ability_index(
    game: &GameState,
    brain_id: ObjectId,
    needle: &str,
) -> usize {
    game.object(brain_id)
        .expect("Brain in a Jar should exist")
        .abilities
        .iter()
        .position(|ability| {
            if let AbilityKind::Activated(activated) = &ability.kind {
                format!("{:?}", activated.effects).contains(needle)
            } else {
                false
            }
        })
        .unwrap_or_else(|| {
            panic!("Brain in a Jar should have activated ability containing {needle}")
        })
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn surtland_elementalist_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(93_100), "Surtland Elementalist")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Giant, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(8, 8))
        .parse_text(
            "As an additional cost to cast this spell, reveal a Giant card from your hand or pay {2}.\nWhenever this creature attacks, you may cast an instant or sorcery spell from your hand without paying its mana cost.",
        )
        .expect("Surtland Elementalist should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) enum SurtlandAdditionalCostMode {
    Reveal,
    Pay,
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct SurtlandDecisionMaker {
    pub(super) cast_free_spell: bool,
    pub(super) additional_cost_mode: Option<SurtlandAdditionalCostMode>,
    pub(super) object_choice: Option<ObjectId>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for SurtlandDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.cast_free_spell
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if let Some(mode) = self.additional_cost_mode.as_ref() {
            let needle = match mode {
                SurtlandAdditionalCostMode::Reveal => "reveal",
                SurtlandAdditionalCostMode::Pay => "pay {2}",
            };
            if let Some(option) = ctx.options.iter().find(|option| {
                option.legal && option.description.to_ascii_lowercase().contains(needle)
            }) {
                return vec![option.index];
            }
        }

        ctx.options
            .iter()
            .filter(|option| option.legal)
            .map(|option| option.index)
            .take(ctx.min)
            .collect()
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if let Some(object_id) = self.object_choice
            && ctx
                .candidates
                .iter()
                .any(|candidate| candidate.legal && candidate.id == object_id)
        {
            return vec![object_id];
        }

        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(ctx.min)
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn attack_with_surtland(game: &mut GameState, surtland_id: ObjectId) -> TriggerQueue {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.remove_summoning_sickness(surtland_id);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);

    let mut combat = CombatState::default();
    let mut trigger_queue = TriggerQueue::new();
    apply_attacker_declarations(
        game,
        &mut combat,
        &mut trigger_queue,
        &[AttackerDeclaration {
            creature: surtland_id,
            target: AttackTarget::Player(bob),
        }],
    )
    .expect("Surtland Elementalist should be able to attack");
    trigger_queue
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn stack_contains_named_object(game: &GameState, name: &str) -> bool {
    game.stack.iter().any(|entry| {
        game.object(entry.object_id)
            .is_some_and(|object| object.name == name)
    })
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn finish_surtland_cast(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut progress: crate::decision::GameProgress,
    dm: &mut SurtlandDecisionMaker,
) {
    for _ in 0..32 {
        if stack_contains_named_object(game, "Surtland Elementalist") {
            return;
        }
        progress = match progress {
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ) => {
                let choice = dm.object_choice.unwrap_or_else(|| {
                    ctx.candidates
                        .iter()
                        .find(|candidate| candidate.legal)
                        .expect("Surtland additional cost should have a legal object choice")
                        .id
                });
                apply_priority_response_with_dm(
                    game,
                    trigger_queue,
                    state,
                    &PriorityResponse::CardCostChoice(choice),
                    dm,
                )
                .expect("Surtland additional cost object choice should be accepted")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) => {
                let choice = dm
                    .decide_options(game, &ctx)
                    .first()
                    .copied()
                    .unwrap_or_else(|| {
                        ctx.options
                            .iter()
                            .find(|option| option.legal)
                            .expect("Surtland cast should have a legal option")
                            .index
                    });
                let description = ctx.description.to_ascii_lowercase();
                let response = if description.starts_with("choose the next cost to pay") {
                    PriorityResponse::NextCostChoice(choice)
                } else {
                    PriorityResponse::ManaPayment(choice)
                };
                apply_priority_response_with_dm(game, trigger_queue, state, &response, dm)
                    .expect("Surtland cast option should be accepted")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            )
            | crate::decision::GameProgress::Continue => return,
            other => panic!("unexpected Surtland cast flow state: {other:?}"),
        };
    }
    panic!("Surtland cast flow did not finish after repeated decisions");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn surtland_elementalist_attack_trigger_casts_matching_hand_spell_for_free() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let surtland = surtland_elementalist_definition();
    let surtland_id = game.create_object_from_definition(&surtland, alice, Zone::Battlefield);
    let matching_spell = CardBuilder::new(CardId::from_raw(93_101), "Surtland Matching Instant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(9)]))
        .build();
    let nonmatching_type = CardBuilder::new(CardId::from_raw(93_102), "Surtland Hand Creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .build();
    let opponent_spell = CardBuilder::new(CardId::from_raw(93_103), "Opponent Surtland Instant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .build();
    game.create_object_from_card(&matching_spell, alice, Zone::Hand);
    let nonmatching_id = game.create_object_from_card(&nonmatching_type, alice, Zone::Hand);
    let opponent_spell_id = game.create_object_from_card(&opponent_spell, bob, Zone::Hand);

    let mut trigger_queue = attack_with_surtland(&mut game, surtland_id);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Surtland Elementalist should trigger when it attacks"
    );
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Surtland Elementalist attack trigger should go on the stack");

    let mut dm = SurtlandDecisionMaker {
        cast_free_spell: true,
        additional_cost_mode: None,
        object_choice: None,
    };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Surtland Elementalist attack trigger should resolve");

    assert!(
        stack_contains_named_object(&game, "Surtland Matching Instant"),
        "the matching instant should be cast onto the stack without paying its mana cost"
    );
    assert_eq!(
        game.object(nonmatching_id)
            .expect("nonmatching creature should still exist")
            .zone,
        Zone::Hand,
        "the trigger should not cast a non-instant, non-sorcery card"
    );
    assert_eq!(
        game.object(opponent_spell_id)
            .expect("opponent spell should still exist")
            .zone,
        Zone::Hand,
        "the trigger should not cast a spell from another player's hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn surtland_elementalist_attack_trigger_can_be_declined() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let surtland = surtland_elementalist_definition();
    let surtland_id = game.create_object_from_definition(&surtland, alice, Zone::Battlefield);
    let matching_spell = CardBuilder::new(CardId::from_raw(93_104), "Declined Surtland Instant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(9)]))
        .build();
    let matching_id = game.create_object_from_card(&matching_spell, alice, Zone::Hand);

    let mut trigger_queue = attack_with_surtland(&mut game, surtland_id);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Surtland Elementalist attack trigger should go on the stack");

    let mut dm = SurtlandDecisionMaker {
        cast_free_spell: false,
        additional_cost_mode: None,
        object_choice: None,
    };
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Surtland Elementalist attack trigger should resolve when declined");

    assert_eq!(
        game.object(matching_id)
            .expect("declined spell should still exist")
            .zone,
        Zone::Hand,
        "declining the may trigger should leave the matching spell in hand"
    );
    assert!(
        !stack_contains_named_object(&game, "Declined Surtland Instant"),
        "declining the may trigger should not cast the spell"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn surtland_elementalist_additional_cost_can_reveal_giant_instead_of_paying_two() {
    use crate::decision::LegalAction;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let surtland = surtland_elementalist_definition();
    let surtland_id = game.create_object_from_definition(&surtland, alice, Zone::Hand);
    let giant = CardBuilder::new(CardId::from_raw(93_105), "Revealed Giant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Giant])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let giant_id = game.create_object_from_card(&giant, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 2);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 5);

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SurtlandDecisionMaker {
        cast_free_spell: false,
        additional_cost_mode: Some(SurtlandAdditionalCostMode::Reveal),
        object_choice: Some(giant_id),
    };
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: surtland_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
        &mut dm,
    )
    .expect(
        "Surtland Elementalist should cast by revealing a Giant with only its mana cost available",
    );
    finish_surtland_cast(&mut game, &mut trigger_queue, &mut state, progress, &mut dm);

    assert!(
        stack_contains_named_object(&game, "Surtland Elementalist"),
        "Surtland Elementalist should be on the stack after revealing the Giant additional cost"
    );
    assert_eq!(
        game.object(giant_id)
            .expect("revealed Giant should remain in hand")
            .zone,
        Zone::Hand,
        "revealing a Giant should not move it out of hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn surtland_elementalist_additional_cost_can_pay_two_without_giant() {
    use crate::decision::LegalAction;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let surtland = surtland_elementalist_definition();
    let surtland_id = game.create_object_from_definition(&surtland, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 2);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 7);

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SurtlandDecisionMaker {
        cast_free_spell: false,
        additional_cost_mode: Some(SurtlandAdditionalCostMode::Pay),
        object_choice: None,
    };
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: surtland_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
        &mut dm,
    )
    .expect("Surtland Elementalist should cast by paying {2} with no Giant in hand");
    finish_surtland_cast(&mut game, &mut trigger_queue, &mut state, progress, &mut dm);

    assert!(
        stack_contains_named_object(&game, "Surtland Elementalist"),
        "Surtland Elementalist should be on the stack after paying {{2}} additional cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_omniscience_grants_free_cast_from_hand_without_mana() {
    use crate::cards::definitions::lightning_bolt;
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let omniscience = CardDefinitionBuilder::new(CardId::from_raw(9001), "Omniscience Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text("You may cast spells from your hand without paying their mana costs.")
        .expect("Omniscience text should parse");
    game.create_object_from_definition(&omniscience, alice, Zone::Battlefield);

    let bolt = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Hand);

    let actions = compute_legal_actions(&game, alice);
    let free_cast = actions.iter().find(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::PlayFrom {
                    zone: Zone::Hand,
                    use_alternative: Some(_),
                    ..
                },
            } if *spell_id == bolt_id
        )
    });

    assert!(
        free_cast.is_some(),
        "Omniscience should expose a free cast action from hand without available mana"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_omniscience_choose_casting_method_includes_free_option() {
    use crate::cards::definitions::{basic_mountain, lightning_bolt};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let omniscience = CardDefinitionBuilder::new(CardId::from_raw(9002), "Omniscience Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text("You may cast spells from your hand without paying their mana costs.")
        .expect("Omniscience text should parse");
    game.create_object_from_definition(&omniscience, alice, Zone::Battlefield);

    let mountain = basic_mountain();
    game.create_object_from_definition(&mountain, alice, Zone::Battlefield);

    let bolt = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Hand);

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: bolt_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });

    let result = apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);

    match result {
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        )) => {
            assert_eq!(ctx.player, alice);
            assert_eq!(ctx.source, Some(bolt_id));
            assert_eq!(
                ctx.options.len(),
                2,
                "Should offer normal and free cast methods"
            );
            assert!(
                ctx.options.iter().any(|option| {
                    option
                        .description
                        .to_ascii_lowercase()
                        .contains("without paying mana cost")
                        || option.description.to_ascii_lowercase().contains("free")
                }),
                "expected a free-cast option in ChooseCastingMethod, got {:?}",
                ctx.options
            );
        }
        other => panic!(
            "Expected SelectOptions context for Omniscience casting choice, got {:?}",
            other
        ),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_omniscience_does_not_bypass_sorcery_timing_restrictions() {
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.active_player = bob;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    game.turn.priority_player = Some(alice);

    let omniscience = CardDefinitionBuilder::new(CardId::from_raw(9003), "Omniscience Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text("You may cast spells from your hand without paying their mana costs.")
        .expect("Omniscience text should parse");
    game.create_object_from_definition(&omniscience, alice, Zone::Battlefield);

    let sorcery = CardBuilder::new(CardId::from_raw(9004), "Omniscience Sorcery Test")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(crate::mana::ManaCost::from_symbols(vec![
            crate::mana::ManaSymbol::Blue,
        ]))
        .build();
    let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

    let actions = compute_legal_actions(&game, alice);
    let free_cast = actions.iter().find(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::PlayFrom {
                    zone: Zone::Hand,
                    use_alternative: Some(_),
                    ..
                },
            } if *spell_id == sorcery_id
        )
    });

    assert!(
        free_cast.is_none(),
        "Omniscience should not let sorceries ignore normal timing restrictions"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_brain_in_a_jar_first_ability_casts_matching_mana_value_spell_for_free() {
    fn find_matching_cast_filter(
        effect: &crate::effect::Effect,
    ) -> Option<crate::filter::ObjectFilter> {
        if let Some(cast) =
            effect.downcast_ref::<crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect>()
        {
            return Some(cast.filter.clone());
        }

        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find_matching_cast_filter(child);
            }
        });
        found
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let brain = brain_in_a_jar_definition();
    let matching_filter = brain
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(&activated.effects),
            _ => None,
        })
        .flat_map(|program| program.all_effects())
        .find_map(find_matching_cast_filter)
        .expect("Brain in a Jar should expose its counter-derived cast filter");
    let brain_id = game.create_object_from_definition(&brain, alice, Zone::Battlefield);
    let matching_spell = CardBuilder::new(CardId::from_raw(93_001), "One-Mana Instant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .build();
    let nonmatching_spell = CardBuilder::new(CardId::from_raw(93_002), "Two-Mana Instant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
        .build();
    let matching_id = game.create_object_from_card(&matching_spell, alice, Zone::Hand);
    let matching_stable_id = game
        .object(matching_id)
        .expect("matching spell should exist")
        .stable_id;
    let nonmatching_id = game.create_object_from_card(&nonmatching_spell, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    let ability_index = brain_in_a_jar_ability_index(
        &game,
        brain_id,
        "MayCastMatchingSpellWithoutPayingManaCostEffect",
    );
    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == brain_id && *idx == ability_index
            )
        })
        .expect("Brain in a Jar first ability should be legal with one mana and untapped source");

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
    .expect("Brain in a Jar activation should be put on the stack");

    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Brain in a Jar first ability should resolve");

    assert_eq!(
        game.counter_count(brain_id, crate::object::CounterType::Charge),
        1,
        "Brain in a Jar should put a charge counter on itself before checking the free-cast gate"
    );
    let filter_ctx = game
        .filter_context_for(alice, Some(brain_id))
        .with_caster(Some(alice));
    let matching_current_id = game
        .find_object_by_stable_id(matching_stable_id)
        .expect("the matching spell should retain stable identity across the cast");
    assert!(
        game.object(matching_current_id).is_some_and(|candidate| {
            crate::filter::ObjectFilterExt::matches(&matching_filter, candidate, &filter_ctx, &game)
        }),
        "the compiled counter-derived filter must match the one-mana instant after the counter is added: {matching_filter:#?}"
    );
    assert!(
        game.stack.iter().any(|entry| {
            game.object(entry.object_id)
                .is_some_and(|object| object.name == "One-Mana Instant")
        }),
        "the one-mana instant should be cast onto the stack without paying its mana cost"
    );
    assert_eq!(
        game.object(nonmatching_id)
            .expect("nonmatching spell should remain in hand")
            .zone,
        Zone::Hand,
        "the two-mana instant should not match one charge counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_brain_in_a_jar_first_ability_casts_nothing_without_matching_mana_value() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let brain = brain_in_a_jar_definition();
    let brain_id = game.create_object_from_definition(&brain, alice, Zone::Battlefield);
    let nonmatching_spell = CardBuilder::new(CardId::from_raw(93_003), "Two-Mana Sorcery")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
        .build();
    let nonmatching_type = CardBuilder::new(CardId::from_raw(93_004), "One-Mana Creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .build();
    let opponent_spell = CardBuilder::new(CardId::from_raw(93_005), "Opponent One-Mana Instant")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .build();
    let nonmatching_id = game.create_object_from_card(&nonmatching_spell, alice, Zone::Hand);
    let nonmatching_type_id = game.create_object_from_card(&nonmatching_type, alice, Zone::Hand);
    let opponent_spell_id = game.create_object_from_card(&opponent_spell, bob, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    let ability_index = brain_in_a_jar_ability_index(
        &game,
        brain_id,
        "MayCastMatchingSpellWithoutPayingManaCostEffect",
    );
    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == brain_id && *idx == ability_index
            )
        })
        .expect("Brain in a Jar first ability should be legal");

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
    .expect("Brain in a Jar activation should be put on the stack");

    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Brain in a Jar first ability should resolve without a matching spell");

    assert_eq!(
        game.counter_count(brain_id, crate::object::CounterType::Charge),
        1,
        "Brain in a Jar should still get its charge counter"
    );
    assert_eq!(
        game.object(nonmatching_id)
            .expect("nonmatching spell should remain in hand")
            .zone,
        Zone::Hand,
        "a spell with the wrong mana value should not be cast"
    );
    assert_eq!(
        game.object(nonmatching_type_id)
            .expect("nonmatching card type should remain in hand")
            .zone,
        Zone::Hand,
        "a non-instant, non-sorcery card with matching mana value should not be cast"
    );
    assert_eq!(
        game.object(opponent_spell_id)
            .expect("opponent's matching spell should remain in hand")
            .zone,
        Zone::Hand,
        "a matching instant in another player's hand should not be cast"
    );
    assert!(
        game.stack
            .iter()
            .all(|entry| match game.object(entry.object_id) {
                Some(object) => !matches!(
                    object.name.as_str(),
                    "Two-Mana Sorcery" | "One-Mana Creature" | "Opponent One-Mana Instant"
                ),
                None => true,
            }),
        "nonmatching or non-owned cards should not be on the stack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_brain_in_a_jar_second_ability_removes_x_charge_counters_for_scry() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let brain = brain_in_a_jar_definition();
    let brain_id = game.create_object_from_definition(&brain, alice, Zone::Battlefield);
    game.add_counters(brain_id, crate::object::CounterType::Charge, 2)
        .expect("charge counters should be addable to Brain in a Jar");
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 3);

    let ability_index = brain_in_a_jar_ability_index(&game, brain_id, "ScryEffect");
    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == brain_id && *idx == ability_index
            )
        })
        .expect("Brain in a Jar scry ability should be legal with three mana and counters");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Brain in a Jar scry activation should ask for X");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Number(ctx),
        ) => {
            assert_eq!(ctx.player, alice);
            assert_eq!(ctx.source, Some(brain_id));
        }
        other => panic!("expected X choice for Brain in a Jar scry activation, got {other:?}"),
    }

    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::XValue(2),
        &mut dm,
    )
    .expect("choosing X should finish paying Brain in a Jar activation costs");

    for _ in 0..8 {
        progress = match progress {
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) if ctx.description.starts_with("Choose the next cost to pay") => {
                let option = ctx
                    .options
                    .iter()
                    .find(|option| {
                        let description = option.description.to_ascii_lowercase();
                        option.legal && description.contains("remove")
                    })
                    .or_else(|| {
                        ctx.options.iter().find(|option| {
                            let description = option.description.to_ascii_lowercase();
                            option.legal && description.contains("tap")
                        })
                    })
                    .or_else(|| ctx.options.iter().find(|option| option.legal))
                    .expect("Brain in a Jar should have a legal cost option");
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::NextCostChoice(option.index),
                    &mut dm,
                )
                .expect("chosen Brain in a Jar cost should be payable")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) => {
                let option = ctx
                    .options
                    .iter()
                    .find(|option| option.legal)
                    .expect("Brain in a Jar mana payment should have a legal option");
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::ManaPayment(option.index),
                    &mut dm,
                )
                .expect("Brain in a Jar mana payment should succeed")
            }
            crate::decision::GameProgress::Continue => break,
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => break,
            other => panic!("unexpected Brain in a Jar activation progress: {other:?}"),
        };
    }

    assert_eq!(
        game.counter_count(brain_id, crate::object::CounterType::Charge),
        0,
        "the X charge counters should be removed as an activation cost"
    );

    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Brain in a Jar scry ability should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_backdraft_cast_from_hand_uses_blasphemous_act_damage_history() {
    struct ScriptedBackdraftDecisionMaker {
        cast_blasphemous_act: bool,
        cast_backdraft: bool,
    }

    impl DecisionMaker for ScriptedBackdraftDecisionMaker {
        fn decide_priority(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::PriorityContext,
        ) -> LegalAction {
            if !self.cast_blasphemous_act
                && let Some(action) = ctx.actions.iter().find(|action| {
                    matches!(
                        action,
                        LegalAction::CastSpell { spell_id, .. }
                            if game
                                .object(*spell_id)
                                .is_some_and(|object| object.name == "Blasphemous Act")
                    )
                })
            {
                self.cast_blasphemous_act = true;
                return action.clone();
            }

            if !self.cast_backdraft
                && game.stack.is_empty()
                && let Some(action) = ctx.actions.iter().find(|action| {
                    matches!(
                        action,
                        LegalAction::CastSpell { spell_id, .. }
                            if game
                                .object(*spell_id)
                                .is_some_and(|object| object.name == "Backdraft")
                    )
                })
            {
                self.cast_backdraft = true;
                return action.clone();
            }

            ctx.actions
                .iter()
                .find(|action| matches!(action, LegalAction::PassPriority))
                .cloned()
                .unwrap_or_else(|| {
                    ctx.actions
                        .first()
                        .cloned()
                        .unwrap_or(LegalAction::PassPriority)
                })
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if let Some(option) = ctx
                .options
                .iter()
                .find(|option| option.legal && option.description.contains("Alice"))
            {
                return vec![option.index];
            }

            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();

    game.turn.active_player = alice;
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let omniscience = CardDefinitionBuilder::new(CardId::from_raw(9100), "Omniscience Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text("You may cast spells from your hand without paying their mana costs.")
        .expect("Omniscience text should parse");
    game.create_object_from_definition(&omniscience, alice, Zone::Battlefield);

    for idx in 0..3 {
        let ornithopter =
            CardBuilder::new(CardId::from_raw(9101 + idx), format!("Ornithopter {idx}"))
                .card_types(vec![CardType::Artifact, CardType::Creature])
                .mana_cost(ManaCost::from_symbols(vec![]))
                .power_toughness(PowerToughness::fixed(0, 2))
                .build();
        game.create_object_from_card(&ornithopter, alice, Zone::Battlefield);
    }

    let blasphemous_act = CardDefinitionBuilder::new(CardId::from_raw(9110), "Blasphemous Act")
        .card_types(vec![CardType::Sorcery])
        .parse_text("This spell deals 13 damage to each creature.")
        .expect("Blasphemous Act should parse");
    let backdraft = CardDefinitionBuilder::new(CardId::from_raw(9111), "Backdraft")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.",
        )
        .expect("Backdraft should parse");

    game.create_object_from_definition(&blasphemous_act, alice, Zone::Hand);
    game.create_object_from_definition(&backdraft, alice, Zone::Hand);

    let alice_life_before = game.player(alice).expect("alice should exist").life;
    let mut dm = ScriptedBackdraftDecisionMaker {
        cast_blasphemous_act: false,
        cast_backdraft: false,
    };

    let result = run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm)
        .expect("priority loop should resolve both spells successfully");

    assert!(
        matches!(result, GameProgress::Continue),
        "priority loop should finish after resolving Backdraft, got {result:?}"
    );
    assert_eq!(
        game.player(alice).expect("alice should exist").life,
        alice_life_before - 19,
        "Backdraft should deal half of Blasphemous Act's 39 damage when cast through the normal hand-to-stack flow"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_dauthi_voidwalker_activation_makes_void_counter_card_castable_from_exile_for_free()
 {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::events::processing::ZoneChangeOutcome;
    use crate::object::CounterType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let dauthi = CardDefinitionBuilder::new(CardId::new(), "Dauthi Voidwalker Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Shadow\nIf a card would be put into an opponent's graveyard from anywhere, instead exile it with a void counter on it.\n{T}, Sacrifice this creature: Choose an exiled card an opponent owns with a void counter on it. You may play it this turn without paying its mana cost.",
        )
        .expect("Dauthi text should parse");
    let dauthi_id = game.create_object_from_definition(&dauthi, alice, Zone::Battlefield);
    game.remove_summoning_sickness(dauthi_id);

    let bears = crate::cards::definitions::grizzly_bears();
    let bears_id = game.create_object_from_definition(&bears, bob, Zone::Battlefield);
    let bears_stable_id = game
        .object(bears_id)
        .expect("grizzly bears should exist")
        .stable_id;

    let mut dm = SelectFirstDecisionMaker;
    let zone_change = crate::events::processing::process_zone_change(
        &mut game,
        bears_id,
        Zone::Battlefield,
        Zone::Graveyard,
        crate::events::cause::EventCause::from_sba(),
        &mut dm,
    );
    assert!(
        matches!(zone_change, ZoneChangeOutcome::Replaced),
        "expected Dauthi replacement to exile the creature, got {zone_change:?}"
    );

    let exiled_bears_id = game
        .find_object_by_stable_id(bears_stable_id)
        .expect("exiled Grizzly Bears should be findable by stable id");
    assert_eq!(
        game.object(exiled_bears_id)
            .expect("exiled bears should exist")
            .zone,
        Zone::Exile,
        "Grizzly Bears should be exiled by Dauthi's replacement effect"
    );
    assert_eq!(
        game.counter_count(exiled_bears_id, CounterType::Void),
        1,
        "exiled Grizzly Bears should have a void counter"
    );

    let actions_before = compute_legal_actions(&game, alice);
    assert!(
        !actions_before.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    ..
                } if *spell_id == exiled_bears_id
            )
        }),
        "card should not be castable from exile before Dauthi's activation resolves"
    );

    let activated = game
        .object(dauthi_id)
        .expect("Dauthi should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("Dauthi should have an activated ability");

    let mut ctx = ExecutionContext::new(dauthi_id, alice, &mut dm);
    for effect in &activated.effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Dauthi activation effect should resolve");
    }

    let actions_after = compute_legal_actions(&game, alice);
    assert!(
        actions_after.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::PlayFrom {
                        zone: Zone::Exile,
                        use_alternative: Some(_),
                        ..
                    },
                } if *spell_id == exiled_bears_id
            )
        }),
        "Dauthi activation should make the exiled void-counter card castable for free, got {actions_after:?}"
    );
}

// =========================================================================
// Underworld Breach / Granted Escape Tests
// =========================================================================

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_underworld_breach_grants_escape_to_graveyard_cards() {
    use crate::cards::definitions::{lightning_bolt, underworld_breach};
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Put Underworld Breach on battlefield
    let breach_def = underworld_breach();
    let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

    // Put Lightning Bolt in graveyard
    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

    // Add 3 more cards to graveyard (for escape cost)
    let _bolt2_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
    let _bolt3_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
    let _bolt4_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

    // Give alice enough mana to cast Lightning Bolt (R)
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 1);

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should find a GrantedEscape cast option for Lightning Bolt
    let escape_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::GrantedEscape { .. },
            } if *spell_id == bolt_id
        )
    });

    assert!(
        escape_action.is_some(),
        "Should be able to cast Lightning Bolt with granted escape from graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_underworld_breach_no_escape_without_enough_cards_to_exile() {
    use crate::cards::definitions::{lightning_bolt, underworld_breach};
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Put Underworld Breach on battlefield
    let breach_def = underworld_breach();
    let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

    // Put Lightning Bolt in graveyard (ONLY card)
    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

    // Only 1 card in graveyard - need 3 MORE to exile for escape
    // So escape should not be available

    // Give alice enough mana to cast Lightning Bolt (R)
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 1);

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should NOT find escape option (not enough cards to exile)
    let escape_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::GrantedEscape { .. },
            } if *spell_id == bolt_id
        )
    });

    assert!(
        escape_action.is_none(),
        "Should NOT be able to use escape without enough cards to exile"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_underworld_breach_escape_needs_3_other_cards() {
    // Regression test: with 3 cards in graveyard, you can only exile 2 OTHER cards,
    // so escape (which requires exiling 3) should NOT be available
    use crate::cards::definitions::{counterspell, force_of_will, think_twice, underworld_breach};
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Put Underworld Breach on battlefield
    let breach_def = underworld_breach();
    let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

    // Put 3 cards in graveyard
    let think_twice_def = think_twice();
    let fow_def = force_of_will();
    let cs_def = counterspell();
    let think_twice_id =
        game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);
    let _fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Graveyard);
    let _cs_id = game.create_object_from_definition(&cs_def, alice, Zone::Graveyard);

    // Give alice enough mana to cast any of these
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 5);

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Escape requires exiling 3 OTHER cards - but with only 3 total,
    // each card has only 2 other cards available, so NO escape should be available
    let escape_actions: Vec<_> = actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                LegalAction::CastSpell {
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::GrantedEscape { .. },
                    ..
                }
            )
        })
        .collect();

    assert!(
        escape_actions.is_empty(),
        "Should NOT be able to use escape with only 3 cards in graveyard (need 3 OTHER cards). Found {} escape actions: {:?}",
        escape_actions.len(),
        escape_actions
            .iter()
            .map(|a| if let LegalAction::CastSpell { spell_id, .. } = a {
                game.object(*spell_id)
                    .map(|o| o.name.to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            })
            .collect::<Vec<_>>()
    );

    // Flashback for Think Twice SHOULD still be available though
    let flashback_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == think_twice_id
        )
    });
    assert!(
        flashback_action.is_some(),
        "Think Twice's intrinsic flashback should still be available"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_underworld_breach_doesnt_grant_escape_to_lands() {
    use crate::cards::definitions::{basic_mountain, underworld_breach};
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Put Underworld Breach on battlefield
    let breach_def = underworld_breach();
    let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

    // Put a land in graveyard
    let mountain_def = basic_mountain();
    let mountain_id = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);

    // Add 3 more cards to graveyard (for potential escape cost)
    use crate::cards::definitions::lightning_bolt;
    let bolt_def = lightning_bolt();
    let _bolt2_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
    let _bolt3_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
    let _bolt4_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should NOT find escape option for the land
    let escape_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::GrantedEscape { .. },
            } if *spell_id == mountain_id
        )
    });

    assert!(
        escape_action.is_none(),
        "Underworld Breach should NOT grant escape to lands"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_underworld_breach_no_escape_without_breach_on_battlefield() {
    use crate::cards::definitions::lightning_bolt;
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // NO Underworld Breach on battlefield

    // Put Lightning Bolt in graveyard
    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

    // Add 3 more cards to graveyard
    let _bolt2_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
    let _bolt3_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
    let _bolt4_id = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

    // Give alice mana
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 1);

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should NOT find escape option (no Underworld Breach)
    let escape_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::GrantedEscape { .. },
            } if *spell_id == bolt_id
        )
    });

    assert!(
        escape_action.is_none(),
        "Should NOT be able to use escape without Underworld Breach on battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_force_of_will_cannot_use_alt_cost_when_escaping() {
    // This tests a tricky interaction:
    // Force of Will has an alternative cost (pay 1 life, exile a blue card from hand)
    // Underworld Breach grants escape (pay mana cost + exile 3 cards from graveyard)
    //
    // According to MTG rules, you CANNOT combine alternative costs.
    // When casting via granted escape, you must pay the escape cost (card's mana cost + exile 3).
    // You cannot use Force of Will's own alternative cost from the graveyard.

    use crate::cards::definitions::{
        counterspell, force_of_will, lightning_bolt, underworld_breach,
    };
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up main phase with something on the stack to counter
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Put a spell on the stack for alice to counter
    let bolt_def = lightning_bolt();
    let bolt_stack_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
    game.stack.push(StackEntry::new(bolt_stack_id, bob));

    // Put Underworld Breach on battlefield
    let breach_def = underworld_breach();
    let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

    // Put Force of Will in GRAVEYARD
    let fow_def = force_of_will();
    let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Graveyard);

    // Add 3 more cards to graveyard (for escape cost)
    let _extra1 = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
    let _extra2 = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);
    let _extra3 = game.create_object_from_definition(&bolt_def, alice, Zone::Graveyard);

    // Give alice a blue card in hand (would be needed for FoW's own alternative cost)
    let cs_def = counterspell();
    let _blue_card_in_hand = game.create_object_from_definition(&cs_def, alice, Zone::Hand);

    // Give alice enough mana to cast Force of Will normally (3UU = 5 mana)
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 3);

    // Give alice 20 life
    game.player_mut(alice).unwrap().life = 20;

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should find granted escape option (from graveyard via Underworld Breach)
    let granted_escape_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::GrantedEscape { .. },
            } if *spell_id == fow_id
        )
    });

    assert!(
        granted_escape_action.is_some(),
        "Should be able to cast Force of Will via granted escape from graveyard"
    );

    // Should NOT find Force of Will's own alternative cost from graveyard
    // (Alternative cost says "from hand", not "from graveyard")
    let fow_alt_cost_from_graveyard = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == fow_id
        )
    });

    assert!(
        fow_alt_cost_from_graveyard.is_none(),
        "Should NOT be able to use Force of Will's own alternative cost from graveyard - \
             alternative costs cannot be combined, and FoW's alt cost requires casting from hand"
    );

    // Also verify: no weird hybrid option that combines both costs
    // (There shouldn't be any action that lets you pay "1 life + exile blue card + exile 3 from GY")
    // This is implicitly tested by the above - we only have GrantedEscape, not Alternative(0)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_underworld_breach_escape_works_with_4_cards() {
    // With 4 cards in graveyard, escape SHOULD be available (3 other cards to exile)
    // This tests the positive case - escape IS legal when there are enough cards
    use crate::cards::definitions::{basic_mountain, think_twice, underworld_breach};
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Put Underworld Breach on battlefield
    let breach_def = underworld_breach();
    let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

    // Put 4 cards in graveyard: Think Twice + 3 others (Mountain is a land but can still be exiled)
    let think_twice_def = think_twice();
    let mountain_def = basic_mountain();
    let think_twice_id =
        game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);
    let _m1 = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);
    let _m2 = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);
    let _m3 = game.create_object_from_definition(&mountain_def, alice, Zone::Graveyard);

    // Give alice enough mana for flashback (2U = 3 mana, more expensive than escape's 1U)
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should find Think Twice [Escape] option
    let escape_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::GrantedEscape { .. },
            } if *spell_id == think_twice_id
        )
    });

    assert!(
        escape_action.is_some(),
        "Think Twice [Escape] should be available with 4 cards in graveyard (3 other cards to exile)"
    );

    // Also verify Think Twice's normal Flashback is still available
    let flashback_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == think_twice_id
        )
    });

    assert!(
        flashback_action.is_some(),
        "Think Twice's intrinsic Flashback should also be available"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_force_of_will_escape_with_spell_on_stack() {
    // Simulates:
    // - Player 1 has Underworld Breach, 5 Islands, Force of Will + 3 cards in graveyard
    // - Player 2 casts Lightning Bolt
    // - Player 1 should be able to counter with Force of Will via Escape
    use crate::cards::definitions::{
        basic_mountain, counterspell, force_of_will, lightning_bolt, think_twice, underworld_breach,
    };
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Set up - it's Player 2's turn, main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = bob;

    // Player 1's setup: Underworld Breach + 5 Islands on battlefield
    let breach_def = underworld_breach();
    let _breach_id = game.create_object_from_definition(&breach_def, alice, Zone::Battlefield);

    // Player 1's graveyard: Force of Will, Counterspell, Think Twice, Mountain (4 cards)
    let fow_def = force_of_will();
    let cs_def = counterspell();
    let tt_def = think_twice();
    let mtn_def = basic_mountain();
    let fow_id = game.create_object_from_definition(&fow_def, alice, Zone::Graveyard);
    let _cs_id = game.create_object_from_definition(&cs_def, alice, Zone::Graveyard);
    let _tt_id = game.create_object_from_definition(&tt_def, alice, Zone::Graveyard);
    let _mtn_id = game.create_object_from_definition(&mtn_def, alice, Zone::Graveyard);

    // Player 2 casts Lightning Bolt targeting Player 1
    let bolt_def = lightning_bolt();
    let bolt_id = game.create_object_from_definition(&bolt_def, bob, Zone::Stack);
    let mut bolt_entry = StackEntry::new(bolt_id, bob);
    bolt_entry.targets = vec![Target::Player(alice)];
    game.stack.push(bolt_entry);

    // Now Player 1 has priority to respond
    game.turn.priority_player = Some(alice);

    // Give Player 1 mana to cast Force of Will (3UU = 5 mana)
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 5);

    // Compute legal actions for Player 1
    let actions = compute_legal_actions(&game, alice);

    // Should find Force of Will [Escape] option - there's a spell on the stack to counter!
    let fow_escape_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::GrantedEscape { .. },
            } if *spell_id == fow_id
        )
    });

    assert!(
        fow_escape_action.is_some(),
        "Force of Will [Escape] should be available when there's a spell on the stack to counter. \
             Graveyard has 4 cards (3 others to exile), and Lightning Bolt is on the stack as a legal target."
    );

    // Now actually cast Force of Will via Escape
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2); // 2 players

    // Cast the spell
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: fow_id,
        from_zone: Zone::Graveyard,
        casting_method: CastingMethod::GrantedEscape {
            source: game
                .battlefield
                .iter()
                .find(|&&id| {
                    game.object(id)
                        .map(|o| o.name == "Underworld Breach")
                        .unwrap_or(false)
                })
                .copied()
                .unwrap(),
            exile_count: 3,
        },
    });

    let progress =
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);

    // Should need to choose targets (Lightning Bolt is the only legal target)
    assert!(
        matches!(
            progress,
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(_)
            ))
        ),
        "Should prompt for targets after casting Force of Will. Got: {:?}",
        progress
    );

    // Provide the target (Lightning Bolt on stack - spells are objects)
    let targets_response = PriorityResponse::Targets(vec![Target::Object(bolt_id)]);
    let mut progress2 =
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &targets_response);

    assert!(
        progress2.is_ok(),
        "Targeting should succeed. Got: {:?}",
        progress2
    );

    // Granted Escape now pays its typed exile component through the ordinary
    // interactive CR 601 cost transaction rather than auto-exiling the first
    // three graveyard cards in finalization.
    let mut dm = SelectFirstDecisionMaker;
    for _ in 0..12 {
        let progress = progress2.expect("granted Escape cost payment should remain valid");
        progress2 = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => break,
            GameProgress::NeedsDecisionCtx(context) => apply_decision_context_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &context,
                &mut dm,
            ),
            GameProgress::Continue => break,
            other => panic!("unexpected granted Escape payment progress: {other:?}"),
        };
    }

    // Verify the escape cost was paid:
    // - Force of Will should now be on the stack
    // - 3 cards should have been exiled from Alice's graveyard
    // - Alice's graveyard should now have only 0 cards (FoW moved to stack, 3 exiled)

    let alice_graveyard_count = game.player(alice).unwrap().graveyard.len();
    assert_eq!(
        alice_graveyard_count, 0,
        "Alice's graveyard should be empty after casting FoW via escape (1 cast + 3 exiled). Got: {}",
        alice_graveyard_count
    );

    // Verify 3 cards were exiled
    let alice_exile_count = game
        .exile
        .iter()
        .filter(|&&id| game.object(id).map(|o| o.owner == alice).unwrap_or(false))
        .count();
    assert_eq!(
        alice_exile_count, 3,
        "3 cards should have been exiled from Alice's graveyard for escape cost. Got: {}",
        alice_exile_count
    );

    // Verify Force of Will is on the stack
    let fow_on_stack = game.stack.iter().any(|entry| {
        game.object(entry.object_id)
            .map(|o| o.name == "Force of Will")
            .unwrap_or(false)
    });
    assert!(fow_on_stack, "Force of Will should be on the stack");
}

// ============================================================================
// Affinity for Artifacts Tests
// ============================================================================

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_affinity_reduces_mana_cost() {
    // Frogmite costs {4} with affinity for artifacts
    // With 4 artifacts in play, it should cost {0}
    use crate::cards::definitions::frogmite;
    use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Create 4 artifacts on the battlefield
    for i in 0..4 {
        let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&artifact, alice, Zone::Battlefield);
    }

    // Put Frogmite in hand with NO mana in pool
    let frogmite_def = frogmite();
    let frogmite_id = game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

    // Compute legal actions - Frogmite should be castable with 0 mana
    let actions = compute_legal_actions(&game, alice);

    let can_cast_frogmite = actions.iter().any(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                ..
            } if *spell_id == frogmite_id
        )
    });

    assert!(
        can_cast_frogmite,
        "Should be able to cast Frogmite for free with 4 artifacts in play"
    );

    // Verify the effective cost is 0
    let frogmite_obj = game.object(frogmite_id).unwrap();
    let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
    let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
    assert_eq!(
        effective_cost.mana_value(),
        0,
        "Effective cost should be 0 with 4 artifacts"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_affinity_partial_reduction() {
    // Frogmite costs {4} with affinity for artifacts
    // With 2 artifacts in play, it should cost {2}
    use crate::cards::definitions::frogmite;
    use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Create 2 artifacts on the battlefield
    for i in 0..2 {
        let artifact = CardBuilder::new(CardId::new(), &format!("Artifact {}", i))
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&artifact, alice, Zone::Battlefield);
    }

    // Put Frogmite in hand
    let frogmite_def = frogmite();
    let frogmite_id = game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

    // Verify the effective cost is 2
    let frogmite_obj = game.object(frogmite_id).unwrap();
    let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
    let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
    assert_eq!(
        effective_cost.mana_value(),
        2,
        "Effective cost should be 2 with 2 artifacts"
    );

    // With only 1 mana, should NOT be able to cast
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    let actions = compute_legal_actions(&game, alice);
    let can_cast = actions.iter().any(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                ..
            } if *spell_id == frogmite_id
        )
    });
    assert!(
        !can_cast,
        "Should NOT be able to cast Frogmite with only 1 mana when cost is 2"
    );

    // With 2 mana, should be able to cast
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    let actions = compute_legal_actions(&game, alice);
    let can_cast = actions.iter().any(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                ..
            } if *spell_id == frogmite_id
        )
    });
    assert!(
        can_cast,
        "Should be able to cast Frogmite with 2 mana when cost is 2"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_affinity_only_counts_own_artifacts() {
    // Affinity only counts artifacts YOU control
    use crate::cards::definitions::frogmite;
    use crate::decision::calculate_effective_mana_cost;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Create 2 artifacts controlled by Alice
    for i in 0..2 {
        let artifact = CardBuilder::new(CardId::new(), &format!("Alice Artifact {}", i))
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&artifact, alice, Zone::Battlefield);
    }

    // Create 2 artifacts controlled by Bob (should NOT count)
    for i in 10..12 {
        let artifact = CardBuilder::new(CardId::new(), &format!("Bob Artifact {}", i))
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&artifact, bob, Zone::Battlefield);
    }

    // Put Frogmite in Alice's hand
    let frogmite_def = frogmite();
    let frogmite_id = game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

    // Verify effective cost is 2 (only Alice's artifacts count)
    let frogmite_obj = game.object(frogmite_id).unwrap();
    let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
    let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
    assert_eq!(
        effective_cost.mana_value(),
        2,
        "Effective cost should be 2 - only Alice's 2 artifacts count, not Bob's"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_frogmite_counts_as_artifact_for_affinity_when_on_battlefield() {
    // Frogmite is an artifact creature, so once on battlefield it counts for other affinity costs
    use crate::cards::definitions::frogmite;
    use crate::decision::calculate_effective_mana_cost;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Put one Frogmite on the battlefield
    let frogmite_def = frogmite();
    let _battlefield_frogmite_id =
        game.create_object_from_definition(&frogmite_def, alice, Zone::Battlefield);

    // Put another Frogmite in hand
    let frogmite_in_hand_id = game.create_object_from_definition(&frogmite_def, alice, Zone::Hand);

    // The first Frogmite on battlefield should count as an artifact
    let frogmite_obj = game.object(frogmite_in_hand_id).unwrap();
    let base_cost = frogmite_obj.mana_cost.as_ref().unwrap();
    let effective_cost = calculate_effective_mana_cost(&game, alice, frogmite_obj, base_cost);
    assert_eq!(
        effective_cost.mana_value(),
        3,
        "Effective cost should be 3 - one artifact (the other Frogmite) on battlefield"
    );
}

// ============================================================================
// Delve Tests
// ============================================================================

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_delve_reduces_mana_cost() {
    // Treasure Cruise costs {7}{U} with Delve
    // With 7 cards in graveyard, it should cost just {U}
    use crate::cards::definitions::treasure_cruise;
    use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Put 7 cards in graveyard
    for i in 0..7 {
        let card = CardBuilder::new(CardId::new(), &format!("Graveyard Card {}", i))
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }

    // Put Treasure Cruise in hand
    let tc_def = treasure_cruise();
    let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

    // Give alice just 1 blue mana (the {U} part)
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);

    // Verify the effective cost is just {U} (mana value 1)
    let tc_obj = game.object(tc_id).unwrap();
    let base_cost = tc_obj.mana_cost.as_ref().unwrap();
    let effective_cost = calculate_effective_mana_cost(&game, alice, tc_obj, base_cost);
    assert_eq!(
        effective_cost.mana_value(),
        1,
        "Effective cost should be 1 (just U) with 7 cards in graveyard to delve"
    );

    // Compute legal actions - Treasure Cruise should be castable with 1 blue mana
    let actions = compute_legal_actions(&game, alice);

    let can_cast_tc = actions.iter().any(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                ..
            } if *spell_id == tc_id
        )
    });

    assert!(
        can_cast_tc,
        "Should be able to cast Treasure Cruise with 7 cards to delve and 1 blue mana"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_delve_partial_reduction() {
    // Treasure Cruise costs {7}{U} with Delve
    // With 3 cards in graveyard, it should cost {4}{U}
    use crate::cards::definitions::treasure_cruise;
    use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Put 3 cards in graveyard
    for i in 0..3 {
        let card = CardBuilder::new(CardId::new(), &format!("Graveyard Card {}", i))
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }

    // Put Treasure Cruise in hand
    let tc_def = treasure_cruise();
    let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

    // Verify effective cost is {4}{U} = 5
    let tc_obj = game.object(tc_id).unwrap();
    let base_cost = tc_obj.mana_cost.as_ref().unwrap();
    let effective_cost = calculate_effective_mana_cost(&game, alice, tc_obj, base_cost);
    assert_eq!(
        effective_cost.mana_value(),
        5,
        "Effective cost should be 5 (4U) with 3 cards to delve"
    );

    // With only 3 mana (not enough), should NOT be able to cast
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    let actions = compute_legal_actions(&game, alice);
    let can_cast = actions.iter().any(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                ..
            } if *spell_id == tc_id
        )
    });
    assert!(
        !can_cast,
        "Should NOT be able to cast with only 3 mana when effective cost is 5"
    );

    // With 5 mana, should be able to cast
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    let actions = compute_legal_actions(&game, alice);
    let can_cast = actions.iter().any(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                ..
            } if *spell_id == tc_id
        )
    });
    assert!(
        can_cast,
        "Should be able to cast with 5 mana when effective cost is 5"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_delve_exiles_cards_on_cast() {
    // When casting with Delve, cards should be exiled from graveyard
    use crate::cards::definitions::treasure_cruise;
    use crate::decision::LegalAction;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Put 7 cards in graveyard
    let graveyard_names = [
        "Delve Card 0",
        "Delve Card 1",
        "Delve Card 2",
        "Delve Card 3",
        "Delve Card 4",
        "Delve Card 5",
        "Delve Card 6",
    ];
    for name in graveyard_names {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }

    // Put Treasure Cruise in hand
    let tc_def = treasure_cruise();
    let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

    // Give alice 1 blue mana
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);

    // Verify initial state
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 7);
    assert_eq!(game.exile.len(), 0);

    let mut dm = NamedCastCostDecisionMaker::new(graveyard_names);
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        tc_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("Treasure Cruise Delve transaction should execute")
    .expect("Treasure Cruise should commit after seven chosen Delve payments");

    // Verify 7 cards were exiled from graveyard
    assert_eq!(
        game.player(alice).unwrap().graveyard.len(),
        0,
        "Graveyard should be empty after delving 7 cards"
    );
    assert_eq!(
        game.exile.len(),
        7,
        "7 cards should be in exile after delving"
    );

    // Treasure Cruise should be on the stack
    assert!(game.stack.iter().any(|entry| entry.object_id == stack_id));
}

#[test]
pub(super) fn delve_chooses_an_exact_card_and_can_stop_below_the_maximum() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Selective Delve Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .delve()
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
    for name in ["Delve Choice A", "Delve Choice B", "Delve Choice C"] {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    let mut dm = NamedCastCostDecisionMaker::new(["Delve Choice C"]);
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("selective Delve transaction should execute")
    .expect("one Delve payment plus available mana should commit");

    assert_eq!(dm.cost_prompts.len(), 1);
    let exiled_choice = game
        .exile
        .iter()
        .copied()
        .find(|&id| {
            game.object(id)
                .is_some_and(|object| object.name == "Delve Choice C")
        })
        .expect("the exact selected Delve card should be exiled");
    assert_eq!(game.exile.len(), 1, "Delve must not force its maximum");
    assert!(
        game.get_exiled_with_source_links(stack_id)
            .contains(&exiled_choice)
    );
    let entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == stack_id)
        .expect("Delve spell should be on the stack");
    assert!(
        entry
            .tagged_objects
            .get(&crate::tag::TagKey::from(crate::tag::SOURCE_EXILED_TAG))
            .is_some_and(|cards| cards.iter().any(|card| card.name == "Delve Choice C"))
    );
    for name in ["Delve Choice A", "Delve Choice B"] {
        assert!(
            game.player(alice)
                .expect("Alice exists")
                .graveyard
                .iter()
                .any(|&id| game.object(id).is_some_and(|object| object.name == name))
        );
    }
}

#[test]
pub(super) fn delve_can_be_declined_entirely_when_mana_covers_the_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Declined Delve Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .delve()
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
    for name in ["Kept Delve Card A", "Kept Delve Card B"] {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    let mut dm = NamedCastCostDecisionMaker::default();
    let result = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("declined Delve transaction should execute");

    assert!(result.is_some());
    assert!(dm.cost_prompts.is_empty());
    assert!(game.exile.is_empty());
    assert_eq!(game.player(alice).expect("Alice exists").graveyard.len(), 2);
}

#[test]
pub(super) fn cancelling_partial_delve_payment_rolls_back_every_exile() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Rollback Delve Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Sorcery])
        .delve()
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
    for name in ["Rollback Delve A", "Rollback Delve B", "Rollback Delve C"] {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }

    let mut dm = NamedCastCostDecisionMaker::new(["Rollback Delve B", "<cancel>"]);
    let result = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    )
    .expect("cancelled Delve transaction should roll back cleanly");

    assert!(result.is_none());
    assert!(game.stack_is_empty());
    assert!(game.exile.is_empty());
    assert!(
        game.object(spell_id)
            .is_some_and(|spell| spell.zone == Zone::Hand)
    );
    assert_eq!(game.player(alice).expect("Alice exists").graveyard.len(), 3);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_delve_cannot_cast_without_enough_graveyard_or_mana() {
    // Treasure Cruise costs {7}{U}
    // With 0 cards in graveyard and only 3 mana, should NOT be castable
    use crate::cards::definitions::treasure_cruise;
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Empty graveyard
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 0);

    // Put Treasure Cruise in hand
    let tc_def = treasure_cruise();
    let tc_id = game.create_object_from_definition(&tc_def, alice, Zone::Hand);

    // Give alice 3 mana (not enough for {7}{U})
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    // Should NOT be able to cast
    let actions = compute_legal_actions(&game, alice);
    let can_cast = actions.iter().any(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                ..
            } if *spell_id == tc_id
        )
    });

    assert!(
        !can_cast,
        "Should NOT be able to cast Treasure Cruise with no graveyard and only 3 mana"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_convoke_reduces_mana_cost_with_creatures() {
    // Stoke the Flames costs {2}{R}{R} with Convoke
    // With 2 untapped creatures (one red), it should cost {1}{R}
    use crate::cards::definitions::stoke_the_flames;
    use crate::color::ColorSet;
    use crate::decision::{calculate_effective_mana_cost, compute_legal_actions};
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.turn.active_player = alice;

    // Create 2 untapped creatures on battlefield (one red, one colorless)
    let red_creature = CardBuilder::new(CardId::from_raw(800), "Red Creature")
        .card_types(vec![CardType::Creature])
        .color_indicator(ColorSet::RED)
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let colorless_creature = CardBuilder::new(CardId::from_raw(801), "Colorless Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let red_id = game.create_object_from_card(&red_creature, alice, Zone::Battlefield);
    let colorless_id = game.create_object_from_card(&colorless_creature, alice, Zone::Battlefield);

    // Mark them as not summoning sick
    game.remove_summoning_sickness(red_id);
    game.remove_summoning_sickness(colorless_id);

    // Put Stoke the Flames in hand
    let stoke_def = stoke_the_flames();
    let stoke_id = game.create_object_from_definition(&stoke_def, alice, Zone::Hand);

    // Give alice {1}{R} mana (red creature pays one {R}, colorless pays {1} of the {2})
    // Use Colorless for generic since Generic(1) doesn't add to pool
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 1);

    // Verify the effective cost is reduced
    let stoke_obj = game.object(stoke_id).unwrap();
    let base_cost = stoke_obj.mana_cost.as_ref().unwrap();
    let effective_cost = calculate_effective_mana_cost(&game, alice, stoke_obj, base_cost);

    // With red creature paying {R} and colorless paying {1}, remaining should be {1}{R}
    assert_eq!(
        effective_cost.mana_value(),
        2,
        "Effective cost should be 2 (1 generic + 1 red) with 2 creatures to convoke"
    );

    // Compute legal actions - Stoke should be castable
    let actions = compute_legal_actions(&game, alice);

    let can_cast_stoke = actions.iter().any(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                ..
            } if *spell_id == stoke_id
        )
    });

    assert!(
        can_cast_stoke,
        "Should be able to cast Stoke the Flames with 2 creatures to convoke and 2 mana"
    );
}
