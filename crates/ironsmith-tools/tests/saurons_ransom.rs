//! Sauron's Ransom: "Choose an opponent. They look at the top four cards of
//! your library and separate them into a face-down pile and a face-up pile.
//! Put one pile into your hand and the other into your graveyard. The Ring
//! tempts you."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{SelectObjectsContext, SelectOptionsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Sauron's Ransom",
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

/// Bob builds the face-down pile from `face_down`; Alice takes the face-down
/// pile when `take_face_down`, else the face-up pile.
struct Choices {
    face_down: Vec<&'static str>,
    take_face_down: bool,
    divider: Option<PlayerId>,
    offered: Vec<String>,
}

impl DecisionMaker for Choices {
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.divider = Some(ctx.player);
        self.offered = ctx.candidates.iter().map(|c| c.name.to_string()).collect();
        ctx.candidates
            .iter()
            .filter(|c| self.face_down.contains(&c.name.as_str()))
            .map(|c| c.id)
            .collect()
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let wanted = if self.take_face_down { "face-down" } else { "face-up" };
        let picked: Vec<usize> = ctx
            .options
            .iter()
            .filter(|o| o.legal && o.description.contains(wanted))
            .map(|o| o.index)
            .take(1)
            .collect();
        if picked.is_empty() {
            ctx.options.iter().filter(|o| o.legal).take(ctx.min.max(1)).map(|o| o.index).collect()
        } else {
            picked
        }
    }
}

fn card(name: &str) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Sorcery])
        .build()
}

fn names(game: &GameState, ids: impl Iterator<Item = ObjectId>) -> Vec<String> {
    let mut names: Vec<String> = ids.map(|id| game.object(id).unwrap().name.to_string()).collect();
    names.sort();
    names
}

fn run(take_face_down: bool) -> (GameState, Choices) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    // Library listed bottom-first: A is on top.
    for name in ["Bottom", "D", "C", "B", "A"] {
        game.create_object_from_definition(&card(name), alice, Zone::Library);
    }
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Black, 2);
    let hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand))
        .expect("castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Choices {
        face_down: vec!["A", "B"],
        take_face_down,
        divider: None,
        offered: Vec::new(),
    };
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
    (game, dm)
}

#[test]
fn opponent_divides_the_top_four_and_you_take_the_face_down_pile() {
    let (game, dm) = run(true);
    let alice = PlayerId::from_index(0);
    assert_eq!(dm.divider, Some(PlayerId::from_index(1)), "the chosen opponent divides");
    let mut offered = dm.offered.clone();
    offered.sort();
    assert_eq!(offered, vec!["A", "B", "C", "D"], "only the top four");
    let player = game.player(alice).unwrap();
    assert_eq!(names(&game, player.hand.iter().copied()), vec!["A", "B"]);
    assert_eq!(
        names(&game, player.graveyard.iter().copied()),
        vec!["C", "D", "Sauron's Ransom"]
    );
    assert_eq!(names(&game, player.library.iter().copied()), vec!["Bottom"]);
}

#[test]
fn taking_the_face_up_pile_sends_the_face_down_pile_to_the_graveyard() {
    let (game, _) = run(false);
    let alice = PlayerId::from_index(0);
    let player = game.player(alice).unwrap();
    assert_eq!(names(&game, player.hand.iter().copied()), vec!["C", "D"]);
    assert_eq!(
        names(&game, player.graveyard.iter().copied()),
        vec!["A", "B", "Sauron's Ransom"]
    );
}

#[test]
fn the_ring_tempts_you() {
    let (game, _) = run(true);
    let alice = PlayerId::from_index(0);
    assert_eq!(game.ring_temptations(alice), 1);
}
