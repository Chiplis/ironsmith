use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "The Wandering Emperor",
    )
    .unwrap()
    .remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}
fn setup(definition: &CardDefinition) -> (GameState, PlayerId, ObjectId) {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = PlayerId::from_index(1);
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let hand = game.create_object_from_definition(definition, alice, Zone::Hand);
    let source = game
        .move_object_with_etb_processing_with_dm(
            hand,
            Zone::Battlefield,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap()
        .new_id;
    (game, alice, source)
}
fn activations(game: &GameState, player: PlayerId, source: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(game, player).into_iter().filter(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).collect()
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
fn instant_loyalty_timing_is_limited_to_entry_turn() {
    let definition = definition();
    for same_turn in [false, true] {
        for own_turn in [false, true] {
            for stack_occupied in [false, true] {
                let (mut game, alice, source) = setup(&definition);
                if !same_turn {
                    game.turn.turn_number += 1;
                    game.turn_store.turn_history.clear_for_new_turn();
                }
                if own_turn {
                    game.turn.active_player = alice;
                }
                if stack_occupied {
                    let spell = CardDefinitionBuilder::new(CardId::new(), "Stack probe")
                        .card_types(vec![CardType::Instant])
                        .build();
                    let id = game.create_object_from_definition(&spell, alice, Zone::Stack);
                    game.push_to_stack(ironsmith::game_state::StackEntry::new(id, alice));
                }
                let permitted = same_turn || (own_turn && !stack_occupied);
                assert_eq!(
                    !activations(&game, alice, source).is_empty(),
                    permitted,
                    "same turn={same_turn}, own turn={own_turn}, occupied stack={stack_occupied}"
                );
            }
        }
    }
}

#[test]
fn instant_permission_does_not_relax_once_per_turn_or_controller_rules() {
    let (mut game, alice, source) = setup(&definition());
    assert!(!activations(&game, alice, source).is_empty());
    assert!(activations(&game, PlayerId::from_index(1), source).is_empty());
    game.record_loyalty_ability_activation(source);
    assert!(
        activations(&game, alice, source).is_empty(),
        "an instant-speed permission does not permit a second loyalty activation"
    );
}

fn drive_action(game: &mut GameState, action: LegalAction) {
    let expected = game.stack.len() + 1;
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = ironsmith::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &ironsmith::game_loop::PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..24 {
        if game.stack.len() == expected {
            return;
        }
        let ironsmith::decision::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &ctx, &mut dm,
        )
        .unwrap();
    }
    panic!("action did not finish paying costs and placing its entry on the stack");
}
fn loyalty_action(
    game: &GameState,
    alice: PlayerId,
    source: ObjectId,
    ordinal: usize,
) -> LegalAction {
    let index = game.object(source).unwrap().abilities.iter().enumerate().filter(|(_, ability)| matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(activated) if activated.is_loyalty_ability())).nth(ordinal).unwrap().0;
    activations(game, alice, source).into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { ability_index, .. } if *ability_index == index)).expect("requested loyalty activation must be offered")
}
fn creature() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Creature probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 3))
        .build()
}

#[test]
fn flash_cast_and_zero_target_loyalty_activation_work_on_opponents_turn() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = PlayerId::from_index(1);
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let hand = game.create_object_from_definition(&definition(), alice, Zone::Hand);
    let identity = game.object(hand).unwrap().stable_id;
    for symbol in [
        ironsmith::mana::ManaSymbol::White,
        ironsmith::mana::ManaSymbol::White,
        ironsmith::mana::ManaSymbol::Colorless,
        ironsmith::mana::ManaSymbol::Colorless,
    ] {
        game.player_mut(alice).unwrap().mana_pool.add(symbol, 1);
    }
    let action = compute_legal_actions(&game, alice).into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method: ironsmith::alternative_cast::CastingMethod::Normal, .. } if *spell_id == hand)).expect("flash allows the actual cast");
    drive_action(&mut game, action);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .unwrap();
    let source = game.find_object_by_stable_id(identity).unwrap();
    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.object(source)
            .unwrap()
            .counters
            .get(&ironsmith::CounterType::Loyalty),
        Some(&3)
    );
    game.turn.priority_player = Some(alice);
    let action = loyalty_action(&game, alice, source, 0);
    drive_action(&mut game, action);
    assert!(game.stack.last().unwrap().targets.is_empty());
    assert_eq!(
        game.object(source)
            .unwrap()
            .counters
            .get(&ironsmith::CounterType::Loyalty),
        Some(&4)
    );
    assert!(activations(&game, alice, source).is_empty());
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .unwrap();
    assert!(activations(&game, alice, source).is_empty());
}

#[test]
fn plus_one_adds_counter_and_temporary_first_strike_to_chosen_creature() {
    let (mut game, alice, source) = setup(&definition());
    let target = game.create_object_from_definition(&creature(), alice, Zone::Battlefield);
    let action = loyalty_action(&game, alice, source, 0);
    drive_action(&mut game, action);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .unwrap();
    assert_eq!(
        game.object(target)
            .unwrap()
            .counters
            .get(&ironsmith::CounterType::PlusOnePlusOne),
        Some(&1)
    );
    assert!(game.object_has_static_ability_id(
        target,
        ironsmith::static_abilities::StaticAbilityId::FirstStrike
    ));
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert!(!game.object_has_static_ability_id(
        target,
        ironsmith::static_abilities::StaticAbilityId::FirstStrike
    ));
    assert_eq!(
        game.object(target)
            .unwrap()
            .counters
            .get(&ironsmith::CounterType::PlusOnePlusOne),
        Some(&1)
    );
}

#[test]
fn minus_one_creates_a_white_samurai_with_vigilance() {
    let (mut game, alice, source) = setup(&definition());
    let action = loyalty_action(&game, alice, source, 1);
    drive_action(&mut game, action);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .unwrap();
    let token = game
        .battlefield
        .iter()
        .copied()
        .find(|id| *id != source)
        .unwrap();
    let chars = game.calculated_characteristics(token).unwrap();
    assert_eq!((chars.power, chars.toughness), (Some(2), Some(2)));
    assert!(chars.subtypes.contains(&ironsmith::Subtype::Samurai));
    assert_eq!(chars.colors, ironsmith::color::ColorSet::WHITE);
    assert_eq!(
        game.object(token).unwrap().kind,
        ironsmith::object::ObjectKind::Token
    );
    assert!(game.object_has_static_ability_id(
        token,
        ironsmith::static_abilities::StaticAbilityId::Vigilance
    ));
    assert_eq!(game.current_controller(token), Some(alice));
    assert_eq!(
        game.object(source)
            .unwrap()
            .counters
            .get(&ironsmith::CounterType::Loyalty),
        Some(&2)
    );
}

#[test]
fn minus_two_requires_a_tapped_creature_and_exiles_it_then_gains_life() {
    let (mut game, alice, source) = setup(&definition());
    let target =
        game.create_object_from_definition(&creature(), PlayerId::from_index(1), Zone::Battlefield);
    assert_eq!(
        activations(&game, alice, source).len(),
        2,
        "untapped creatures cannot satisfy the exile target"
    );
    game.tap(target);
    let identity = game.object(target).unwrap().stable_id;
    let action = loyalty_action(&game, alice, source, 2);
    drive_action(&mut game, action);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .unwrap();
    let exiled = game.find_object_by_stable_id(identity).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    assert_eq!(game.player(alice).unwrap().life, 22);
    assert_eq!(
        game.object(source)
            .unwrap()
            .counters
            .get(&ironsmith::CounterType::Loyalty),
        Some(&1)
    );
}

#[test]
fn exile_ability_fizzles_if_target_untaps_before_resolution() {
    let (mut game, alice, source) = setup(&definition());
    let target = game.create_object_from_definition(&creature(), alice, Zone::Battlefield);
    game.tap(target);
    let action = loyalty_action(&game, alice, source, 2);
    drive_action(&mut game, action);
    game.untap(target);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .unwrap();
    assert_eq!(game.object(target).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "no life gain when the only target is illegal"
    );
    assert_eq!(
        game.object(source)
            .unwrap()
            .counters
            .get(&ironsmith::CounterType::Loyalty),
        Some(&1),
        "cost remains paid"
    );
}

#[test]
fn timing_permission_scope_does_not_leak_and_generic_grants_end_when_source_leaves() {
    let definition = definition();
    let (mut game, alice, source) = setup(&definition);
    let mut ordinary = definition.clone();
    ordinary
        .abilities
        .retain(|ability| matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(_)));
    let other = game.create_object_from_definition(&ordinary, alice, Zone::Battlefield);
    assert!(
        activations(&game, alice, other).is_empty(),
        "self permission does not grant other planeswalkers instant timing"
    );
    let mut granting = creature();
    granting
        .abilities
        .push(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::from_model(
                ironsmith::static_abilities::CompiledStaticAbility::loyalty_abilities_any_time(
                    ironsmith::filter::ObjectFilter::planeswalker().you_control(),
                ),
            ),
        ));
    let grant = game.create_object_from_definition(&granting, alice, Zone::Battlefield);
    assert!(
        !activations(&game, alice, other).is_empty(),
        "a filtered permission works for other matching planeswalkers"
    );
    game.move_object_by_effect(grant, Zone::Graveyard);
    assert!(activations(&game, alice, other).is_empty());
    assert!(!activations(&game, alice, source).is_empty());
    game.object_mut(source)
        .unwrap()
        .counters
        .insert(ironsmith::CounterType::Loyalty, 0);
    assert_eq!(
        activations(&game, alice, source).len(),
        1,
        "permission does not waive negative loyalty costs"
    );
}
