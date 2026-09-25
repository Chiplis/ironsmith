//! Hope of Ghirapur: "Flying. Sacrifice Hope of Ghirapur: Until your next
//! turn, target player who was dealt combat damage by Hope of Ghirapur this
//! turn can't cast noncreature spells."
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
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
        "Hope of Ghirapur",
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

/// Records the legal player targets and targets Bob when legal.
struct Aim {
    legal: Vec<Target>,
}

impl DecisionMaker for Aim {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.legal = ctx.requirements[0].legal_targets.clone();
        let bob = Target::Player(PlayerId::from_index(1));
        if self.legal.contains(&bob) {
            vec![bob]
        } else {
            Vec::new()
        }
    }
}

struct Board {
    game: GameState,
    hope: ObjectId,
    opt: ObjectId,
}

fn setup(hope_hit_bob: bool) -> Board {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::NextMain;
    let hope =
        game.create_object_from_definition(&load("Hope of Ghirapur"), alice, Zone::Battlefield);
    let opt = game.create_object_from_definition(&load("Opt"), bob, Zone::Hand);
    game.player_mut(bob)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    if hope_hit_bob {
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ironsmith::effects::EffectContext::new(hope, alice, &mut dm);
        let mut combat_hit = ironsmith::effects::DealDamageEffect::new(
            1,
            ChooseSpec::Player(PlayerFilter::Specific(bob)),
        );
        combat_hit.source_is_combat = true;
        ironsmith::effects::execute_effect(
            &mut game,
            &ironsmith::Effect::new(combat_hit),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(game.player(bob).unwrap().life, 19);
    }
    Board { game, hope, opt }
}

/// Activates the sacrifice ability; returns the legal targets offered and
/// whether the ability reached the stack.
fn sacrifice(board: &mut Board) -> (Vec<Target>, bool) {
    let alice = PlayerId::from_index(0);
    let hope = board.hope;
    let Some(action) = compute_legal_actions(&board.game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == hope))
    else {
        return (Vec::new(), false);
    };
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(board.game.players_in_game());
    let mut dm = Aim { legal: Vec::new() };
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut board.game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..8 {
        if !board.game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut board.game,
            &mut queue,
            &mut state,
            &ctx,
            &mut dm,
        );
    }
    let on_stack = board.game.stack.len() == 1;
    if on_stack {
        ironsmith::game_loop::resolve_stack_entry_with(&mut board.game, &mut dm).unwrap();
    }
    (dm.legal, on_stack)
}

fn bob_can_cast_opt(board: &Board) -> bool {
    let bob = PlayerId::from_index(1);
    let opt = board.opt;
    let mut game = board.game.clone();
    game.turn.priority_player = Some(bob);
    compute_legal_actions(&game, bob)
        .iter()
        .any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == opt))
}

#[test]
fn only_a_player_it_hit_can_be_targeted_and_then_cant_cast_noncreature_spells() {
    let mut board = setup(true);
    assert!(
        bob_can_cast_opt(&board),
        "Opt is castable before the ability"
    );
    let (legal, on_stack) = sacrifice(&mut board);
    assert!(on_stack);
    assert_eq!(
        legal,
        vec![Target::Player(PlayerId::from_index(1))],
        "Alice was not hit"
    );
    assert!(
        board
            .game
            .object(board.hope)
            .is_none_or(|o| o.zone != Zone::Battlefield),
        "Hope was sacrificed"
    );
    assert!(
        !bob_can_cast_opt(&board),
        "Bob can't cast noncreature spells"
    );
}

#[test]
fn no_legal_target_when_it_dealt_no_combat_damage() {
    let mut board = setup(false);
    let (legal, on_stack) = sacrifice(&mut board);
    assert!(!on_stack, "legal targets offered: {legal:?}");
    assert!(bob_can_cast_opt(&board));
}
