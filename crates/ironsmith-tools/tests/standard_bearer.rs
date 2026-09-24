//! Standard Bearer: "Flagbearer (While an opponent is choosing targets as
//! part of casting a spell they control or activating an ability they
//! control, that player must choose at least one Flagbearer on the
//! battlefield if able.)"
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
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
        "Standard Bearer",
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

/// Records the offered targets and gives up without choosing.
struct Record(Vec<Target>);

impl DecisionMaker for Record {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.0 = ctx.requirements[0].legal_targets.clone();
        vec![self.0[0]]
    }
}

/// Alice controls Standard Bearer and a Bear. `caster` casts Shock; returns
/// the legal targets offered.
fn shock_targets(caster: PlayerId) -> (Vec<Target>, ObjectId, ObjectId) {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = caster;
    game.turn.priority_player = Some(caster);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let bearer = game.create_object_from_definition(&load("Standard Bearer"), alice, Zone::Battlefield);
    let bear = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let shock = game.create_object_from_definition(&load("Shock"), caster, Zone::Hand);
    game.player_mut(caster).unwrap().mana_pool.add(ManaSymbol::Red, 1);
    let action = compute_legal_actions(&game, caster)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == shock))
        .expect("Shock castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Record(Vec::new());
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
    (dm.0, bearer, bear)
}

#[test]
fn an_opponent_must_target_the_flagbearer() {
    let (targets, bearer, _) = shock_targets(PlayerId::from_index(1));
    assert_eq!(targets, vec![Target::Object(bearer)]);
}

#[test]
fn its_controller_targets_freely() {
    let (targets, bearer, bear) = shock_targets(PlayerId::from_index(0));
    assert!(targets.contains(&Target::Object(bearer)));
    assert!(targets.contains(&Target::Object(bear)));
    assert!(targets.contains(&Target::Player(PlayerId::from_index(1))));
}
