//! Tithe: "Search your library for a Plains card. If target opponent controls
//! more lands than you, you may search your library for an additional Plains
//! card. Reveal those cards, put them into your hand, then shuffle."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Subtype, Supertype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Tithe",
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

/// Targets Bob, accepts the optional second search, and takes the first
/// legal Plains each time; records every search's legal candidates.
struct Caster {
    searches: Vec<Vec<ObjectId>>,
}

impl DecisionMaker for Caster {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        true
    }

    fn decide_targets(&mut self, _game: &GameState, _ctx: &TargetsContext) -> Vec<Target> {
        vec![Target::Player(PlayerId::from_index(1))]
    }

    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal: Vec<ObjectId> = ctx.candidates.iter().filter(|c| c.legal).map(|c| c.id).collect();
        self.searches.push(legal.clone());
        legal.into_iter().take(1).collect()
    }
}

fn land(name: &str, subtype: Option<Subtype>) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic]);
    if let Some(subtype) = subtype {
        builder = builder.subtypes(vec![subtype]);
    }
    builder.build()
}

/// Alice controls `alice_lands` lands, Bob controls `bob_lands`; Alice's
/// library holds three Plains and a Forest. Returns Alice's hand after
/// Tithe resolves and the number of searches.
fn cast(alice_lands: usize, bob_lands: usize) -> (Vec<String>, usize) {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    for _ in 0..alice_lands {
        game.create_object_from_definition(&land("Wastes", None), alice, Zone::Battlefield);
    }
    for _ in 0..bob_lands {
        game.create_object_from_definition(&land("Wastes", None), bob, Zone::Battlefield);
    }
    for name in ["Plains A", "Plains B", "Plains C"] {
        game.create_object_from_definition(&land(name, Some(Subtype::Plains)), alice, Zone::Library);
    }
    game.create_object_from_definition(&land("Forest", Some(Subtype::Forest)), alice, Zone::Library);
    let spell = game.create_object_from_definition(
        &ironsmith_tools::compile_definition_from_payload(&payload()).unwrap(),
        alice,
        Zone::Hand,
    );
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::White, 1);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Caster { searches: Vec::new() };
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
    let mut hand: Vec<String> = game
        .player(alice)
        .unwrap()
        .hand
        .iter()
        .map(|id| game.object(*id).unwrap().name.to_string())
        .collect();
    hand.sort();
    if let [first, second] = dm.searches.as_slice() {
        assert!(
            !second.iter().any(|id| first.first() == Some(id)),
            "the additional search cannot find the first card again"
        );
    }
    (hand, dm.searches.len())
}

#[test]
fn opponent_with_more_lands_allows_an_additional_plains() {
    let (hand, searches) = cast(1, 3);
    assert_eq!(searches, 2);
    assert_eq!(hand.len(), 2, "{hand:?}");
    assert!(hand.iter().all(|name| name.starts_with("Plains")), "{hand:?}");
}

#[test]
fn otherwise_only_one_plains() {
    let (hand, searches) = cast(3, 3);
    assert_eq!(searches, 1);
    assert_eq!(hand.len(), 1);
    assert!(hand[0].starts_with("Plains"));
}
