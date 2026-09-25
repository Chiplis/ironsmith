//! Biomancer's Familiar: "Activated abilities of creatures you control cost {2}
//! less to activate. This effect can't reduce the mana in that cost to less
//! than one mana. {T}: The next time target creature adapts this turn, it
//! adapts as though it had no +1/+1 counters on it."
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::{CounterType, GameState, ObjectId, PlayerId, Zone};

fn load(name: &str) -> ironsmith::cards::CardDefinition {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        name,
    )
    .unwrap()
    .remove(0);
    ironsmith_tools::compile_definition_from_payload(&payload).unwrap()
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Biomancer's Familiar",
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
}

struct TargetIt(Option<ObjectId>);

impl DecisionMaker for TargetIt {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let id = self.0.expect("targeted activation");
        assert!(
            ctx.requirements[0]
                .legal_targets
                .contains(&Target::Object(id))
        );
        vec![Target::Object(id)]
    }
}

fn activate(game: &mut GameState, source: ObjectId, target: Option<ObjectId>) {
    let alice = PlayerId::from_index(0);
    let action = compute_legal_actions(game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source))
        .expect("ability is activatable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = TargetIt(target);
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
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
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
}

/// Alice controls Biomancer's Familiar and a Growth-Chamber Guardian ({2}{G}:
/// Adapt 2) that already has one +1/+1 counter.
fn board() -> (GameState, ObjectId, ObjectId) {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let familiar =
        game.create_object_from_definition(&load("Biomancer's Familiar"), alice, Zone::Battlefield);
    let guardian = game.create_object_from_definition(
        &load("Growth-Chamber Guardian"),
        alice,
        Zone::Battlefield,
    );
    game.object_mut(guardian)
        .unwrap()
        .add_counters(CounterType::PlusOnePlusOne, 1);
    for id in [familiar, guardian] {
        game.remove_summoning_sickness(id);
    }
    (game, familiar, guardian)
}

fn counters(game: &GameState, id: ObjectId) -> u32 {
    game.counter_count(id, CounterType::PlusOnePlusOne)
}

#[test]
fn marked_creature_adapts_once_despite_existing_counters_at_reduced_cost() {
    let (mut game, familiar, guardian) = board();
    let alice = PlayerId::from_index(0);
    activate(&mut game, familiar, Some(guardian));
    assert!(game.is_tapped(familiar));

    // {2}{G} costs {2} less: only {G} is needed.
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Green, 1);
    activate(&mut game, guardian, None);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    assert_eq!(
        counters(&game, guardian),
        3,
        "adapted as though it had no counters"
    );

    // The effect applies only to the next adapt.
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Green, 1);
    activate(&mut game, guardian, None);
    assert_eq!(
        counters(&game, guardian),
        3,
        "a second adapt sees the counters again"
    );
}

#[test]
fn without_the_familiars_tap_adapt_does_nothing_with_counters() {
    let (mut game, _familiar, guardian) = board();
    let alice = PlayerId::from_index(0);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Green, 1);
    activate(&mut game, guardian, None);
    assert_eq!(counters(&game, guardian), 1);
}

#[test]
fn the_mark_expires_at_end_of_turn() {
    let (mut game, familiar, guardian) = board();
    let alice = PlayerId::from_index(0);
    activate(&mut game, familiar, Some(guardian));
    game.turn.turn_number += 2;
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Green, 1);
    activate(&mut game, guardian, None);
    assert_eq!(counters(&game, guardian), 1, "only this turn");
}
