//! Loki, God of Mischief: "Whenever a player or permanent becomes the target
//! of an ability you control, draw a card. This ability triggers only once
//! each turn."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

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
        "Loki, God of Mischief",
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

struct Aim(Target);

impl DecisionMaker for Aim {
    fn decide_targets(&mut self, _game: &GameState, _ctx: &TargetsContext) -> Vec<Target> {
        vec![self.0]
    }
}

fn setup() -> GameState {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.create_object_from_definition(&load("Loki, God of Mischief"), alice, Zone::Battlefield);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Filler")
        .card_types(vec![CardType::Sorcery])
        .build();
    for _ in 0..5 {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }
    game
}

/// Takes `action`, puts any triggers on the stack, and resolves everything.
fn perform(game: &mut GameState, action: LegalAction, target: Target) {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Aim(target);
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
        result = ironsmith::game_loop::apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm);
    }
    assert!(!game.stack.is_empty(), "{result:?}");
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
    }
}

fn activate_pinger(game: &mut GameState, pinger: ObjectId, target: Target) {
    let action = compute_legal_actions(game, PlayerId::from_index(0))
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == pinger))
        .expect("pinger activatable");
    perform(game, action, target);
}

#[test]
fn targeting_with_an_ability_draws_once_each_turn() {
    let mut game = setup();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let first = game.create_object_from_definition(&load("Prodigal Sorcerer"), alice, Zone::Battlefield);
    let second = game.create_object_from_definition(&load("Prodigal Sorcerer"), alice, Zone::Battlefield);
    let bear = game.create_object_from_definition(&load("Grizzly Bears"), bob, Zone::Battlefield);
    game.remove_summoning_sickness(first);
    game.remove_summoning_sickness(second);
    activate_pinger(&mut game, first, Target::Player(bob));
    assert_eq!(game.player(alice).unwrap().hand.len(), 1, "targeting a player draws");
    assert_eq!(game.player(bob).unwrap().life, 19);
    activate_pinger(&mut game, second, Target::Object(bear));
    assert_eq!(game.player(alice).unwrap().hand.len(), 1, "only once each turn");
}

#[test]
fn targeting_a_permanent_with_an_ability_draws() {
    let mut game = setup();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let pinger = game.create_object_from_definition(&load("Prodigal Sorcerer"), alice, Zone::Battlefield);
    let bear = game.create_object_from_definition(&load("Grizzly Bears"), bob, Zone::Battlefield);
    game.remove_summoning_sickness(pinger);
    activate_pinger(&mut game, pinger, Target::Object(bear));
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
}

#[test]
fn targeting_with_a_spell_does_not_trigger() {
    let mut game = setup();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let shock = game.create_object_from_definition(&load("Shock"), alice, Zone::Hand);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 1);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == shock))
        .expect("Shock castable");
    perform(&mut game, action, Target::Player(bob));
    assert_eq!(game.player(bob).unwrap().life, 18);
    assert_eq!(game.player(alice).unwrap().hand.len(), 0, "a spell is not an ability");
}
