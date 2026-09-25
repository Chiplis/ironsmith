//! Outrageous Robbery: "Target opponent exiles the top X cards of their
//! library face down. You may look at and play those cards for as long as
//! they remain exiled. If you cast a spell this way, you may spend mana as
//! though it were mana of any type to cast it."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{NumberContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::{CardId, StableId};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Supertype, Zone};

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
        "Outrageous Robbery",
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

struct Caster {
    x: u32,
    opponent: PlayerId,
}

impl DecisionMaker for Caster {
    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value { self.x } else { ctx.min }
    }

    fn decide_targets(&mut self, _game: &GameState, _ctx: &TargetsContext) -> Vec<Target> {
        vec![Target::Player(self.opponent)]
    }
}

fn apply(game: &mut GameState, queue: &mut TriggerQueue, action: LegalAction, dm: &mut Caster) {
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result =
            ironsmith::game_loop::apply_decision_context_with_dm(game, queue, &mut state, &ctx, dm);
    }
    assert!(result.is_ok(), "{result:?}");
}

struct Board {
    game: GameState,
    queue: TriggerQueue,
    opt: StableId,
    forest: StableId,
    third: StableId,
}

/// Alice casts Outrageous Robbery for X=2 targeting Bob, whose library (top
/// first) is Opt, Forest, Filler.
fn rob() -> Board {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let filler = CardDefinitionBuilder::new(CardId::new(), "Filler")
        .card_types(vec![CardType::Sorcery])
        .build();
    let forest = CardDefinitionBuilder::new(CardId::new(), "Forest")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic])
        .subtypes(vec![ironsmith::Subtype::Forest])
        .build();
    let third = game.create_object_from_definition(&filler, bob, Zone::Library);
    let forest = game.create_object_from_definition(&forest, bob, Zone::Library);
    let opt = game.create_object_from_definition(&load("Opt"), bob, Zone::Library);
    for _ in 0..3 {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }
    let stable = |game: &GameState, id: ObjectId| game.object(id).unwrap().stable_id;
    let (opt, forest, third) = (
        stable(&game, opt),
        stable(&game, forest),
        stable(&game, third),
    );
    let spell = game.create_object_from_definition(&load("Outrageous Robbery"), alice, Zone::Hand);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Black, 4);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("castable");
    let mut queue = TriggerQueue::new();
    let mut dm = Caster {
        x: 2,
        opponent: bob,
    };
    apply(&mut game, &mut queue, action, &mut dm);
    assert_eq!(game.stack.len(), 1);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    Board {
        game,
        queue,
        opt,
        forest,
        third,
    }
}

fn current(game: &GameState, stable: StableId) -> ObjectId {
    game.find_object_by_stable_id(stable).unwrap()
}

#[test]
fn the_top_x_cards_are_exiled_face_down() {
    let board = rob();
    let game = &board.game;
    for card in [board.opt, board.forest] {
        let object = game.object(current(game, card)).unwrap();
        assert_eq!(object.zone, Zone::Exile);
        assert!(game.is_face_down(object.id), "exiled face down");
    }
    assert_eq!(
        game.object(current(game, board.third)).unwrap().zone,
        Zone::Library
    );
}

#[test]
fn you_may_cast_an_exiled_spell_with_mana_of_any_type() {
    let mut board = rob();
    let alice = PlayerId::from_index(0);
    // Opt costs {U}; Alice has only black mana.
    board
        .game
        .player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Black, 1);
    let opt = current(&board.game, board.opt);
    let action = compute_legal_actions(&board.game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == opt))
        .expect("Opt is castable from exile with black mana");
    let mut dm = Caster {
        x: 0,
        opponent: PlayerId::from_index(1),
    };
    apply(&mut board.game, &mut board.queue, action, &mut dm);
    assert_eq!(board.game.stack.len(), 1, "Opt is on the stack");
    assert_eq!(board.game.player(alice).unwrap().mana_pool.total(), 0);
    ironsmith::game_loop::resolve_stack_entry_with(&mut board.game, &mut dm).unwrap();
    assert_eq!(
        board.game.player(alice).unwrap().hand.len(),
        1,
        "Opt drew Alice a card"
    );
}

#[test]
fn you_may_play_an_exiled_land() {
    let board = rob();
    let alice = PlayerId::from_index(0);
    let forest = current(&board.game, board.forest);
    assert!(
        compute_legal_actions(&board.game, alice)
            .iter()
            .any(|a| matches!(a, LegalAction::PlayLand { land_id } if *land_id == forest)),
        "the exiled Forest can be played"
    );
    assert!(
        !compute_legal_actions(&board.game, PlayerId::from_index(1))
            .iter()
            .any(|a| matches!(a, LegalAction::PlayLand { land_id } if *land_id == forest)),
        "Bob has no permission"
    );
}
