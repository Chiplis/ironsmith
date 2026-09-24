//! Gilded Drake: "Flying. When this creature enters, exchange control of this
//! creature and up to one target creature an opponent controls. If you don't
//! or can't make an exchange, sacrifice this creature. This ability still
//! resolves if its target becomes illegal."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::events::cause::EventCause;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::{CardId, StableId};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Gilded Drake",
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

/// Chooses `target` for the enter trigger (or no target).
struct Choose(Option<ObjectId>);

impl DecisionMaker for Choose {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        match self.0 {
            Some(id) => {
                assert!(ctx.requirements[0].legal_targets.contains(&Target::Object(id)));
                vec![Target::Object(id)]
            }
            None => {
                assert_eq!(ctx.requirements[0].min_targets, 0, "up to one");
                Vec::new()
            }
        }
    }
}

struct Board {
    game: GameState,
    queue: TriggerQueue,
    drake: StableId,
    ogre: ObjectId,
}

/// Alice casts Gilded Drake; Bob controls a 3/3 Ogre. Returns with the enter
/// trigger on the stack targeting `target_ogre ? Ogre : nothing`.
fn cast_drake(target_ogre: bool) -> Board {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Blue, 2);
    let ogre_def = CardDefinitionBuilder::new(CardId::new(), "Ogre")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let ogre = game.create_object_from_definition(&ogre_def, bob, Zone::Battlefield);
    let hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let drake = game.object(hand).unwrap().stable_id;
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand))
        .expect("castable");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Choose(target_ogre.then_some(ogre));
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..12 {
        if !game.stack.is_empty() || progress.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = progress else {
            break;
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(game.stack.len(), 1, "{progress:?}");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1, "the enter trigger");
    Board {
        game,
        queue,
        drake,
        ogre,
    }
}

fn drake_id(board: &Board) -> ObjectId {
    board.game.find_object_by_stable_id(board.drake).unwrap()
}

#[test]
fn exchanges_control_with_the_target_creature() {
    let mut board = cast_drake(true);
    ironsmith::game_loop::resolve_stack_entry(&mut board.game).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let drake = drake_id(&board);
    assert_eq!(board.game.object(drake).unwrap().zone, Zone::Battlefield);
    assert_eq!(board.game.controller_of_id(drake), Some(bob), "Bob now controls the Drake");
    assert_eq!(board.game.controller_of_id(board.ogre), Some(alice), "Alice now controls the Ogre");
    let _ = &mut board.queue;
}

#[test]
fn choosing_no_target_sacrifices_the_drake() {
    let mut board = cast_drake(false);
    ironsmith::game_loop::resolve_stack_entry(&mut board.game).unwrap();
    let drake = drake_id(&board);
    assert_eq!(board.game.object(drake).unwrap().zone, Zone::Graveyard);
    assert_eq!(board.game.controller_of_id(board.ogre), Some(PlayerId::from_index(1)));
}

#[test]
fn illegal_target_still_resolves_and_sacrifices_the_drake() {
    let mut board = cast_drake(true);
    // The Ogre leaves before the trigger resolves: its only target is illegal.
    board
        .game
        .move_object(board.ogre, Zone::Graveyard, EventCause::effect());
    ironsmith::game_loop::resolve_stack_entry(&mut board.game).unwrap();
    let drake = drake_id(&board);
    assert_eq!(
        board.game.object(drake).unwrap().zone,
        Zone::Graveyard,
        "the ability resolves anyway, can't exchange, and sacrifices the Drake"
    );
}
