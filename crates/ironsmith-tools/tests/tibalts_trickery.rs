//! Tibalt's Trickery: "Counter target spell. Choose 1, 2, or 3 at random. Its
//! controller mills that many cards, then exiles cards from the top of their
//! library until they exile a nonland card with a different name than that
//! spell. They may cast that card without paying its mana cost. Then they put
//! the exiled cards on the bottom of their library in a random order."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::{StackEntry, Target};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Subtype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Tibalt's Trickery",
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

struct Choices {
    target: ObjectId,
    cast_free: bool,
}

impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert!(
            ctx.requirements[0]
                .legal_targets
                .contains(&Target::Object(self.target))
        );
        vec![Target::Object(self.target)]
    }

    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.cast_free
    }
}

fn creature(name: &str, cost: u8) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(cost)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn plains() -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Plains")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Plains])
        .build()
}

fn names_in(game: &GameState, player: PlayerId, zone: Zone) -> Vec<String> {
    let p = game.player(player).unwrap();
    let ids: Vec<ObjectId> = match zone {
        Zone::Graveyard => p.graveyard.iter().copied().collect(),
        Zone::Library => p.library.iter().copied().collect(),
        _ => unreachable!(),
    };
    ids.iter()
        .map(|id| game.object(*id).unwrap().name.to_string())
        .collect()
}

/// Bob's Grizzly Bears is on the stack. His library, top first: three Plains,
/// another Grizzly Bears, Hill Giant, then Bottom Card. Alice casts Tibalt's
/// Trickery on the Bears and it resolves. Returns the game.
fn run(cast_free: bool, seed: u64) -> GameState {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(seed);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    // Library is listed bottom-first.
    for card in [
        creature("Bottom Card", 1),
        creature("Hill Giant", 4),
        creature("Grizzly Bears", 2),
        plains(),
        plains(),
        plains(),
    ] {
        game.create_object_from_definition(&card, bob, Zone::Library);
    }
    let bears = game.create_object_from_definition(&creature("Grizzly Bears", 2), bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(bears, bob));
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 2);
    let hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand))
        .expect("castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = Choices {
        target: bears,
        cast_free,
    };
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..16 {
        if game.stack.len() == 2 || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    assert_eq!(game.stack.len(), 2, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    game
}

fn check(game: &GameState, hill_giant_cast: bool) {
    let bob = PlayerId::from_index(1);
    let graveyard = names_in(game, bob, Zone::Graveyard);
    assert!(
        graveyard.contains(&"Grizzly Bears".to_string()),
        "countered: {graveyard:?}"
    );
    let milled = graveyard.iter().filter(|name| *name == "Plains").count();
    assert!(
        (1..=3).contains(&milled),
        "milled 1, 2, or 3: {graveyard:?}"
    );
    assert_eq!(
        graveyard.len(),
        milled + 1,
        "only milled cards and the spell: {graveyard:?}"
    );

    let library = names_in(game, bob, Zone::Library);
    // The exiled cards (the rest of the Plains, the same-named Bears, and the
    // Giant if it wasn't cast) go under Bottom Card; nothing stays exiled.
    let exiled_count = (3 - milled) + 1 + usize::from(!hill_giant_cast);
    assert_eq!(library.len(), 1 + exiled_count, "{library:?}");
    assert_eq!(
        library[exiled_count], "Bottom Card",
        "exiled cards on the bottom: {library:?}"
    );
    assert!(
        !game
            .exile
            .iter()
            .any(|id| game.object(*id).is_some_and(|o| o.owner == bob)),
        "nothing stays exiled"
    );
    assert_eq!(
        game.stack.iter().any(|entry| game
            .object(entry.object_id)
            .is_some_and(|o| o.name == "Hill Giant")),
        hill_giant_cast,
        "a nonland card with a different name than Grizzly Bears was found"
    );
}

#[test]
fn skips_lands_and_same_named_cards_then_offers_a_free_cast() {
    for seed in [1, 2, 3, 4, 5, 6] {
        let game = run(true, seed);
        check(&game, true);
    }
}

#[test]
fn declining_the_cast_puts_the_hit_on_the_bottom_too() {
    let game = run(false, 7);
    check(&game, false);
}

#[test]
fn random_choice_covers_one_two_and_three() {
    let bob = PlayerId::from_index(1);
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..40 {
        let game = run(false, seed);
        seen.insert(
            names_in(&game, bob, Zone::Graveyard)
                .iter()
                .filter(|n| *n == "Plains")
                .count(),
        );
    }
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), vec![1, 2, 3]);
}
