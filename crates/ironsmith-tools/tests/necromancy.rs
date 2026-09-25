//! Necromancy: flash-with-cleanup-sacrifice rider, then "When this enchantment
//! enters, if it's on the battlefield, it becomes an Aura with 'enchant
//! creature put onto the battlefield with Necromancy.' Put target creature card
//! from a graveyard onto the battlefield under your control and attach this
//! enchantment to it. When this enchantment leaves the battlefield, that
//! creature's controller sacrifices it."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::ids::{CardId, StableId};
use ironsmith::object::AttachmentTarget;
use ironsmith::triggers::{TriggerQueue, check_delayed_triggers, check_triggers};
use ironsmith::{CardType, GameState, PlayerId, Subtype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Necromancy",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
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
}

struct Resolved {
    game: GameState,
    queue: TriggerQueue,
    necro: StableId,
    creature: StableId,
}

fn find(game: &GameState, stable: StableId) -> ironsmith::ObjectId {
    game.find_object_by_stable_id(stable).unwrap()
}

/// Casts Necromancy (on Alice's main phase or at instant speed on Bob's turn),
/// resolves it and its enter trigger, targeting the only creature card, which
/// sits in `graveyard_owner`'s graveyard.
fn cast_and_resolve(instant_speed: bool, graveyard_owner: PlayerId) -> Resolved {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = if instant_speed { bob } else { alice };
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let creature_def = CardDefinitionBuilder::new(CardId::new(), "Buried Giant")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(4, 4))
        .build();
    let buried =
        game.create_object_from_definition(&creature_def, graveyard_owner, Zone::Graveyard);
    let creature = game.object(buried).unwrap().stable_id;
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ironsmith::mana::ManaSymbol::Black, 3);
    let hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let necro = game.object(hand).unwrap().stable_id;
    let action = ironsmith::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)
        })
        .expect("Necromancy is castable at this timing");
    let mut queue = TriggerQueue::new();
    let mut state = ironsmith::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &ironsmith::game_loop::PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..12 {
        if !game.stack.is_empty() {
            break;
        }
        let ironsmith::decision::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        )
        .unwrap();
    }
    assert_eq!(game.stack.len(), 1);
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(
        game.object(find(&game, necro)).unwrap().zone,
        Zone::Battlefield
    );
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1, "the enter trigger");
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    Resolved {
        game,
        queue,
        necro,
        creature,
    }
}

fn assert_reanimated_and_attached(resolved: &mut Resolved) {
    let alice = PlayerId::from_index(0);
    let game = &resolved.game;
    let necro = find(game, resolved.necro);
    let creature = find(game, resolved.creature);
    assert_eq!(game.object(creature).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        game.controller_of_id(creature),
        Some(alice),
        "under Necromancy's controller"
    );
    assert_eq!(
        game.object(necro).unwrap().attached_to,
        Some(AttachmentTarget::Object(creature))
    );
    let chars = game.current_characteristics(necro).unwrap();
    assert!(chars.subtypes.contains(&Subtype::Aura), "it became an Aura");
    assert!(chars.card_types.contains(&CardType::Enchantment));
    assert!(game.was_put_onto_battlefield_with_source(necro, creature));
    ironsmith::game_loop::check_and_apply_sbas(&mut resolved.game, &mut resolved.queue).unwrap();
    let game = &resolved.game;
    assert_eq!(
        game.object(find(game, resolved.necro)).unwrap().zone,
        Zone::Battlefield,
        "a legal Aura attachment survives state-based actions"
    );
}

fn run_cleanup(game: &mut GameState, queue: &mut TriggerQueue) -> usize {
    game.turn.phase = ironsmith::game_state::Phase::Ending;
    game.turn.step = Some(ironsmith::game_state::Step::Cleanup);
    let mut fired = 0;
    for event in ironsmith::triggers::generate_step_trigger_events_for_active_players(game) {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
            fired += 1;
        }
        for entry in check_delayed_triggers(game, &event) {
            queue.add(entry);
            fired += 1;
        }
    }
    fired
}

#[test]
fn sorcery_speed_cast_reanimates_from_any_graveyard_and_stays() {
    for owner in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let mut resolved = cast_and_resolve(false, owner);
        assert_reanimated_and_attached(&mut resolved);
        assert_eq!(
            run_cleanup(&mut resolved.game, &mut resolved.queue),
            0,
            "no cleanup sacrifice"
        );
    }
}

#[test]
fn leaving_the_battlefield_makes_the_creatures_controller_sacrifice_it() {
    let mut resolved = cast_and_resolve(false, PlayerId::from_index(1));
    let game = &mut resolved.game;
    let creature = find(game, resolved.creature);
    game.set_current_controller(creature, PlayerId::from_index(1));
    let necro = find(game, resolved.necro);
    game.move_object_by_effect(necro, Zone::Graveyard).unwrap();
    ironsmith::game_loop::put_triggers_on_stack(game, &mut resolved.queue).unwrap();
    assert_eq!(game.stack.len(), 1, "one leaves trigger");
    ironsmith::game_loop::resolve_stack_entry(game).unwrap();
    assert_eq!(
        game.object(find(game, resolved.creature)).unwrap().zone,
        Zone::Graveyard
    );
}

#[test]
fn instant_speed_cast_is_sacrificed_at_the_next_cleanup_and_takes_the_creature() {
    let mut resolved = cast_and_resolve(true, PlayerId::from_index(1));
    assert_reanimated_and_attached(&mut resolved);
    let Resolved {
        mut game,
        mut queue,
        necro,
        creature,
    } = resolved;
    assert_eq!(
        run_cleanup(&mut game, &mut queue),
        1,
        "the delayed cleanup sacrifice"
    );
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(
        game.object(find(&game, necro)).unwrap().zone,
        Zone::Graveyard
    );
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(
        game.stack.len(),
        1,
        "Necromancy leaving triggers the creature sacrifice"
    );
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(
        game.object(find(&game, creature)).unwrap().zone,
        Zone::Graveyard
    );
}
