//! CR audit (in-review completion pass, package q02): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, EffectExecutor, GameState, PlayerId};

use ironsmith::DecisionMaker;

fn q02_creature(game: &mut GameState, zone: ironsmith::Zone, p: i32, t: i32) -> ironsmith::ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    let definition = CardDefinitionBuilder::new(ironsmith::CardId::new(), "Q02 creature")
        .card_types(vec![ironsmith::CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(p, t))
        .build();
    game.create_object_from_definition(&definition, PlayerId::from_index(0), zone)
}

fn q02_complete_action_with_dm<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    action: ironsmith::LegalAction,
    dm: &mut D,
    queue: &mut ironsmith::triggers::TriggerQueue,
) -> Result<(), String> {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    let mut state = PriorityLoopState::new(game.players_in_game());
    assert_eq!(game.turn.priority_player, Some(player));
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..16 {
        if result.is_err()
            || (state.pending_cast.is_none()
                && state.pending_method_selection.is_none()
                && state.pending_activation.is_none())
        {
            break;
        }
        let context = match &result {
            Ok(ironsmith::decision::GameProgress::NeedsDecisionCtx(context)) => context.clone(),
            other => return Err(format!("pending native action has no decision: {other:?}")),
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            game, queue, &mut state, &context, dm,
        );
    }
    result.map_err(|error| format!("{error:?}"))?;
    Ok(())
}

#[test]
fn cr_115_6_activated_up_to_one_target_with_zero_chosen_is_untargeted_and_resolves() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decisions::context::TargetsContext;
    use ironsmith::effects::PutCountersEffect;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{
        Ability, CardId, CardType, ChoiceCount, ChooseSpec, CounterType, Effect, LegalAction,
        ObjectFilter, Phase, Target, Zone,
    };
    struct Choose {
        wanted: usize,
        seen: usize,
    }
    impl DecisionMaker for Choose {
        fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
            self.seen += 1;
            let req = &ctx.requirements[0];
            assert_eq!(req.min_targets, 0, "CR 115.6: up to one allows zero");
            req.legal_targets[..self.wanted].to_vec()
        }
    }
    for wanted in [0usize, 1] {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        let creature = q02_creature(&mut game, Zone::Battlefield, 2, 2);
        let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
        let mut ability = Ability::activated(
            ironsmith::cost::TotalCost::free(),
            vec![
                Effect::new(
                    PutCountersEffect::new(CounterType::Charge, 1, spec.clone())
                        .with_target_count(ChoiceCount::up_to(1)),
                ),
                Effect::gain_life(2),
            ],
        );
        if let AbilityKind::Activated(activated) = &mut ability.kind {
            activated.choices = vec![spec];
        }
        let source_def = CardDefinitionBuilder::new(CardId::new(), "Optional-target activator")
            .card_types(vec![CardType::Artifact])
            .with_ability(ability)
            .build();
        let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
        let action = LegalAction::ActivateAbility {
            source,
            ability_index: 0,
        };
        assert!(
            ironsmith::compute_legal_actions(&game, alice)
                .unwrap()
                .contains(&action),
            "CR 115.6: ability with optional targets is activatable"
        );
        let mut dm = Choose { wanted, seen: 0 };
        let mut queue = TriggerQueue::new();
        q02_complete_action_with_dm(&mut game, alice, action, &mut dm, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        assert_eq!(
            game.stack[0].targets.len(),
            wanted,
            "CR 115.6: ability is targeted only if a target was chosen"
        );
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            game.player(alice).unwrap().life,
            22,
            "CR 115.6/608.2b: zero-target ability still resolves its untargeted instruction (wanted={wanted})"
        );
        assert_eq!(
            game.counter_count(creature, CounterType::Charge),
            wanted as u32,
            "CR 115.6: only chosen targets are affected"
        );
    }
}

#[test]
fn cr_116_2e_printed_circling_vultures_offers_instant_speed_discard_special_action() {
    use ironsmith::special_actions::SpecialAction;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{LegalAction, Phase, PriorityLoopState, PriorityResponse, Step, Zone};
    let compiled = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Circling Vultures",
        "Mana cost: {B}\nType: Creature — Bird\nPower/Toughness: 3/2\nFlying\nYou may discard this card any time you could cast an instant.\nAt the beginning of your upkeep, sacrifice this creature unless you exile the top creature card of your graveyard.",
        false,
    );
    assert!(
        compiled.is_ok(),
        "CR 116.2e: current Oracle Circling Vultures must compile: {:?}",
        compiled.as_ref().err()
    );
    let def = compiled.unwrap();
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "CR 116.2e: printed Circling Vultures must compile without unimplemented content"
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    // Opponent's turn, combat: only instant-speed timing applies.
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::BeginCombat);
    let card = game.create_object_from_definition(&def, alice, Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let action = LegalAction::SpecialAction(SpecialAction::DiscardFromHand { card_id: card });
    assert!(
        ironsmith::compute_legal_actions(&game, alice)
            .unwrap()
            .contains(&action),
        "CR 116.2e: printed Circling Vultures discard is offered whenever its owner has priority"
    );
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    let moved = game.find_object_by_stable_id(stable);
    assert!(
        moved.is_some_and(|id| game.object(id).unwrap().zone == Zone::Graveyard),
        "CR 116.2e: discarding Circling Vultures moves it to the graveyard"
    );
    assert!(game.stack.is_empty(), "CR 116.2e: special action does not use the stack");
    assert_eq!(
        game.turn.priority_player,
        Some(alice),
        "CR 116.3: the player retains priority"
    );
}

#[test]
fn cr_120_5_planeswalker_and_battle_damage_waits_for_state_based_actions() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::{CardId, CardType, ChooseSpec, CounterType, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    for battle in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = q02_creature(&mut game, Zone::Battlefield, 1, 1);
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Damage recipient");
        builder = if battle {
            builder
                .card_types(vec![CardType::Battle])
                .subtypes(vec![Subtype::Siege])
                .defense(3)
        } else {
            builder.card_types(vec![CardType::Planeswalker]).loyalty(3)
        };
        let recipient =
            game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
        let mut ctx = EffectContext::new_default(source, alice);
        DealDamageEffect::new(3, ChooseSpec::SpecificObject(recipient))
            .execute(&mut game, &mut ctx)
            .unwrap();
        let kind = if battle {
            CounterType::Defense
        } else {
            CounterType::Loyalty
        };
        assert_eq!(game.counter_count(recipient, kind), 0, "CR 120.3c/h");
        assert!(
            game.battlefield.contains(&recipient),
            "CR 120.5: damage itself does not destroy or remove the permanent (battle={battle})"
        );
        ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
        assert!(
            !game.battlefield.contains(&recipient),
            "CR 120.5/704.5i/704.5v: state-based actions put the permanent into the graveyard (battle={battle})"
        );
    }
}

#[test]
fn cr_120_6_damage_equal_to_toughness_is_lethal_and_one_less_is_not() {
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::{ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    for (damage, dies) in [(3, false), (4, true), (5, true)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = q02_creature(&mut game, Zone::Battlefield, 1, 1);
        let target = q02_creature(&mut game, Zone::Battlefield, 3, 4);
        DealDamageEffect::new(damage, ChooseSpec::SpecificObject(target))
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
        assert_eq!(
            !game.battlefield.contains(&target),
            dies,
            "CR 120.6: damage {damage} on toughness 4 lethal={dies}"
        );
    }
}

#[test]
fn cr_121_1_turn_based_draw_step_draws_the_top_library_card() {
    use ironsmith::{Phase, Step, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.turn_number = 3;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Draw);
    for _ in 0..3 {
        q02_creature(&mut game, Zone::Library, 1, 1);
    }
    let top = *game.player(alice).unwrap().library.last().unwrap();
    let top_card = game.object(top).unwrap().card;
    ironsmith::turn::execute_draw_step(&mut game).unwrap();
    let hand = &game.player(alice).unwrap().hand;
    assert_eq!(hand.len(), 1, "CR 121.1/504.1: draw step draws one card");
    assert_eq!(
        game.object(hand[0]).unwrap().card,
        top_card,
        "CR 121.1: the top card of the library is drawn"
    );
    assert_eq!(game.player(alice).unwrap().library.len(), 2);
    assert_eq!(
        game.turn_store.turn_history.cards_drawn_by_player(alice),
        1,
        "CR 121.1: the turn-based draw is recorded as a draw"
    );
    assert!(game.player(PlayerId::from_index(1)).unwrap().hand.is_empty());
}

#[test]
fn cr_121_5_searching_library_into_hand_is_not_a_draw() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decisions::context::SelectObjectsContext;
    use ironsmith::effects::SearchLibraryEffect;
    use ironsmith::events::cards::matchers::WouldDrawCardMatcher;
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::{
        CardId, CardType, Effect, ObjectFilter, ObjectId, PlayerFilter, StackEntry, Zone,
    };
    struct Pick(ObjectId);
    impl DecisionMaker for Pick {
        fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
            if ctx.candidates.iter().any(|c| c.id == self.0 && c.legal) {
                vec![self.0]
            } else {
                ctx.candidates
                    .iter()
                    .filter(|c| c.legal)
                    .take(ctx.min)
                    .map(|c| c.id)
                    .collect()
            }
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source_def = CardDefinitionBuilder::new(CardId::new(), "Tutor source")
        .card_types(vec![CardType::Artifact])
        .build();
    let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
    let card = q02_creature(&mut game, Zone::Library, 1, 1);
    game.effect_store
        .replacement_effects
        .add_resolution_effect(ReplacementEffect::with_matcher(
            source,
            alice,
            WouldDrawCardMatcher::you(),
            ReplacementAction::Instead(vec![Effect::gain_life(5)]),
        ));
    let search = SearchLibraryEffect::to_hand(
        ObjectFilter::creature().in_zone(Zone::Library),
        PlayerFilter::You,
        false,
    );
    game.stack
        .push(StackEntry::ability(source, alice, vec![Effect::new(search)]));
    let mut dm = Pick(card);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(game.player(alice).unwrap().hand.len(), 1, "fixture: card found");
    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "CR 121.5: a search into hand is not a draw, so a draw replacement must not apply"
    );
    assert_eq!(
        game.turn_store.turn_history.cards_drawn_by_player(alice),
        0,
        "CR 121.5: searching is not recorded as drawing"
    );
}

#[test]
fn cr_122_1c_shield_counter_does_not_stop_state_based_lethal_damage_destruction() {
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::{ChooseSpec, CounterType, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = q02_creature(&mut game, Zone::Battlefield, 1, 1);
    let creature = q02_creature(&mut game, Zone::Battlefield, 3, 4);
    DealDamageEffect::new(4, ChooseSpec::SpecificObject(creature))
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
    assert_eq!(game.damage_on(creature), 4);
    // The shield counter arrives after lethal damage is marked but before SBAs are checked
    // (e.g. between two instructions of one resolving spell).
    game.add_counters(creature, CounterType::Shield, 1);
    ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
    assert!(
        !game.battlefield.contains(&creature),
        "CR 122.1c/704.5g: shield replaces destruction only as the result of an effect; lethal-damage SBA destruction is not replaced"
    );
}

#[test]
fn cr_122_2_counters_cease_when_moving_between_non_battlefield_zones() {
    use ironsmith::{CounterType, Zone};
    for (origin, destination) in [
        (Zone::Exile, Zone::Graveyard),
        (Zone::Exile, Zone::Hand),
        (Zone::Graveyard, Zone::Exile),
        (Zone::Hand, Zone::Exile),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = q02_creature(&mut game, origin, 2, 2);
        game.add_counters(card, CounterType::Time, 3);
        assert_eq!(game.counter_count(card, CounterType::Time), 3, "fixture");
        game.take_pending_trigger_events();
        let moved = game
            .move_object_by_effect(card, destination)
            .expect("zone move succeeds");
        assert!(
            game.object(moved).unwrap().counters.is_empty(),
            "CR 122.2: counters cease to exist on a {origin:?}->{destination:?} move"
        );
        let events = game.take_pending_trigger_events();
        assert!(
            !events
                .iter()
                .any(|e| e.kind() == ironsmith::events::EventKind::MarkersChanged),
            "CR 122.2: ceasing to exist is not counter removal ({origin:?}->{destination:?})"
        );
    }
}
