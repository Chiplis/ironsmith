//! Hammerheim: "{T}: Add {R}. {T}: Target creature loses all landwalk
//! abilities until end of turn."
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};

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
        "Hammerheim",
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

struct Aim(ObjectId);

impl DecisionMaker for Aim {
    fn decide_targets(&mut self, _game: &GameState, _ctx: &TargetsContext) -> Vec<Target> {
        vec![Target::Object(self.0)]
    }
}

fn has(game: &GameState, id: ObjectId, ability: StaticAbilityId) -> bool {
    game.object_has_static_ability_id(id, ability)
}

#[test]
fn target_creature_loses_every_landwalk_ability_until_end_of_turn() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let hammerheim = game.create_object_from_definition(&load("Hammerheim"), alice, Zone::Battlefield);
    // Bog Wraith: swampwalk.
    let wraith = game.create_object_from_definition(&load("Bog Wraith"), bob, Zone::Battlefield);
    let angel = game.create_object_from_definition(&load("Serra Angel"), bob, Zone::Battlefield);
    game.refresh_continuous_state();
    assert!(has(&game, wraith, StaticAbilityId::Landwalk), "Bog Wraith has swampwalk");

    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| {
            matches!(a, LegalAction::ActivateAbility { source, .. } if *source == hammerheim)
                && !matches!(a, LegalAction::ActivateManaAbility { .. })
        })
        .expect("targeted ability is activatable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Aim(wraith);
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..8 {
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
    assert!(!has(&game, wraith, StaticAbilityId::Landwalk), "swampwalk is gone");
    assert!(has(&game, angel, StaticAbilityId::Flying), "other creatures are untouched");
    // The effect ends at end of turn.
    ironsmith::turn::execute_cleanup_step(&mut game);
    game.refresh_continuous_state();
    assert!(has(&game, wraith, StaticAbilityId::Landwalk), "swampwalk returns after the turn");
}
