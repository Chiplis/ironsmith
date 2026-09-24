//! Berserk: "Cast this spell only before the combat damage step. Target
//! creature gains trample and gets +X/+0 until end of turn, where X is its
//! power. At the beginning of the next end step, destroy that creature if it
//! attacked this turn."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::{CardId, StableId};
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Berserk",
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

fn setup(phase: ironsmith::game_state::Phase, step: Option<ironsmith::game_state::Step>) -> (GameState, ObjectId, ObjectId) {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = phase;
    game.turn.step = step;
    let creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let spell = game.create_object_from_definition(
        &ironsmith_tools::compile_definition_from_payload(&payload()).unwrap(),
        alice,
        Zone::Hand,
    );
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Green, 1);
    (game, spell, creature)
}

fn castable(game: &GameState, spell: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, PlayerId::from_index(0))
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
}

/// Casts Berserk on the Bear in the first main phase; optionally the Bear
/// then attacks. Returns (power after resolving, has trample, zone after the
/// end step).
fn berserk(attacks: bool) -> (Option<i32>, bool, Zone) {
    let alice = PlayerId::from_index(0);
    let (mut game, spell, creature) = setup(ironsmith::game_state::Phase::FirstMain, None);
    let creature_stable: StableId = game.object(creature).unwrap().stable_id;
    let action = castable(&game, spell).expect("castable before combat damage");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm);
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    game.refresh_continuous_state();
    let power = game.calculated_power(creature);
    let trample = game.object_has_static_ability_id(creature, StaticAbilityId::Trample);
    if attacks {
        game.mark_creature_attacked_this_turn(creature);
    }
    let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::BeginningOfEndStepEvent::new(alice),
        Default::default(),
    );
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for entry in ironsmith::triggers::check_delayed_triggers(&mut game, &event) {
        queue.add(entry);
    }
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    }
    let zone = game
        .object(game.find_object_by_stable_id(creature_stable).unwrap())
        .unwrap()
        .zone;
    (power, trample, zone)
}

#[test]
fn doubles_power_grants_trample_and_destroys_it_if_it_attacked() {
    let (power, trample, zone) = berserk(true);
    assert_eq!(power, Some(6), "+X/+0 where X is its power");
    assert!(trample);
    assert_eq!(zone, Zone::Graveyard, "it attacked this turn");
}

#[test]
fn a_creature_that_did_not_attack_survives() {
    let (_, _, zone) = berserk(false);
    assert_eq!(zone, Zone::Battlefield);
}

#[test]
fn cannot_be_cast_after_combat_damage() {
    let (game, spell, _) = setup(
        ironsmith::game_state::Phase::Combat,
        Some(ironsmith::game_state::Step::CombatDamage),
    );
    assert!(castable(&game, spell).is_none(), "only before the combat damage step");
    let (game, spell, _) = setup(ironsmith::game_state::Phase::Combat, Some(ironsmith::game_state::Step::DeclareBlockers));
    assert!(castable(&game, spell).is_some(), "declare blockers is before combat damage");
}
