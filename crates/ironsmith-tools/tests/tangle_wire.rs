//! Tangle Wire: "Fading 4. At the beginning of each player's upkeep, that
//! player taps an untapped artifact, creature, or land they control for each
//! fade counter on this artifact."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::ids::CardId;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{CardType, CounterType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Tangle Wire",
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

/// Records who chooses and prefers candidates by name order.
struct Chooser {
    prefer: Vec<&'static str>,
    prompts: Vec<(PlayerId, Vec<String>, usize, Option<usize>)>,
}

impl DecisionMaker for Chooser {
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let legal: Vec<_> = ctx.candidates.iter().filter(|c| c.legal).collect();
        self.prompts.push((
            ctx.player,
            legal.iter().map(|c| c.name.clone()).collect(),
            ctx.min,
            ctx.max,
        ));
        let want = ctx.max.unwrap_or(legal.len()).min(legal.len());
        let mut picked: Vec<ObjectId> = self
            .prefer
            .iter()
            .filter_map(|name| legal.iter().find(|c| c.name == *name).map(|c| c.id))
            .take(want)
            .collect();
        for candidate in &legal {
            if picked.len() >= want {
                break;
            }
            if !picked.contains(&candidate.id) {
                picked.push(candidate.id);
            }
        }
        picked
    }
}

fn permanent(name: &str, card_type: CardType) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(1, 1));
    }
    builder.build()
}

struct Board {
    game: GameState,
    wire: ObjectId,
    bob_permanents: Vec<(ObjectId, &'static str)>,
}

/// Alice controls Tangle Wire with `fade` counters. Bob controls a land, a
/// creature, an artifact, an enchantment, and an already tapped land.
fn board(fade: u32) -> Board {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 4;
    game.turn.active_player = bob;
    game.turn.phase = ironsmith::game_state::Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
    let wire = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.object_mut(wire)
        .unwrap()
        .add_counters(CounterType::Fade, fade);
    let mut bob_permanents = Vec::new();
    for (name, card_type) in [
        ("Bob Land", CardType::Land),
        ("Bob Bear", CardType::Creature),
        ("Bob Relic", CardType::Artifact),
        ("Bob Aura", CardType::Enchantment),
        ("Bob Tapped Land", CardType::Land),
    ] {
        let id =
            game.create_object_from_definition(&permanent(name, card_type), bob, Zone::Battlefield);
        bob_permanents.push((id, name));
    }
    let tapped = bob_permanents[4].0;
    game.tap(tapped);
    Board {
        game,
        wire,
        bob_permanents,
    }
}

fn run_upkeep(game: &mut GameState, dm: &mut Chooser) -> usize {
    let mut queue = TriggerQueue::new();
    for event in ironsmith::triggers::generate_step_trigger_events_for_active_players(game) {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    let count = queue.entries.len();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
    }
    count
}

fn tapped_names(board: &Board) -> Vec<&'static str> {
    let mut names: Vec<_> = board
        .bob_permanents
        .iter()
        .filter(|(id, _)| board.game.is_tapped(*id))
        .map(|(_, name)| *name)
        .collect();
    names.sort();
    names
}

#[test]
fn opponent_taps_one_permanent_per_fade_counter_and_chooses_them() {
    let mut board = board(2);
    let bob = PlayerId::from_index(1);
    let mut dm = Chooser {
        prefer: vec!["Bob Relic", "Bob Land"],
        prompts: Vec::new(),
    };
    assert_eq!(
        run_upkeep(&mut board.game, &mut dm),
        1,
        "only the each-player upkeep trigger on Bob's turn"
    );
    let (chooser, candidates, min, max) = dm.prompts.last().unwrap().clone();
    assert_eq!(chooser, bob, "that player chooses what to tap");
    assert_eq!((min, max), (2, Some(2)));
    let mut candidates = candidates;
    candidates.sort();
    assert_eq!(
        candidates,
        vec!["Bob Bear", "Bob Land", "Bob Relic"],
        "only untapped artifacts, creatures, and lands that player controls"
    );
    assert_eq!(
        tapped_names(&board),
        vec!["Bob Land", "Bob Relic", "Bob Tapped Land"]
    );
    assert!(
        !board.game.is_tapped(board.wire),
        "Alice's permanents are untouched"
    );
}

#[test]
fn taps_everything_eligible_when_counters_exceed_permanents() {
    let mut board = board(4);
    let mut dm = Chooser {
        prefer: Vec::new(),
        prompts: Vec::new(),
    };
    run_upkeep(&mut board.game, &mut dm);
    assert_eq!(
        tapped_names(&board),
        vec!["Bob Bear", "Bob Land", "Bob Relic", "Bob Tapped Land"]
    );
    assert!(
        !board.game.is_tapped(board.bob_permanents[3].0),
        "enchantments are not eligible"
    );
}

#[test]
fn controllers_own_upkeep_removes_a_counter_and_taps_for_the_rest() {
    let mut board = board(2);
    let alice = PlayerId::from_index(0);
    board.game.turn.active_player = alice;
    let land = board.game.create_object_from_definition(
        &permanent("Alice Land", CardType::Land),
        alice,
        Zone::Battlefield,
    );
    let mut dm = Chooser {
        prefer: vec!["Alice Land"],
        prompts: Vec::new(),
    };
    run_upkeep(&mut board.game, &mut dm);
    let fade = board
        .game
        .object(board.wire)
        .unwrap()
        .counters
        .get(&CounterType::Fade)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        fade, 1,
        "fading removes one counter on its controller's upkeep"
    );
    let (chooser, _, min, _) = dm.prompts.last().unwrap().clone();
    assert_eq!(chooser, alice);
    assert!((1..=2).contains(&min), "counted at resolution: {min}");
    assert!(
        board.game.is_tapped(land),
        "Alice taps her own permanents on her upkeep"
    );
    assert!(
        tapped_names(&board) == vec!["Bob Tapped Land"],
        "Bob is unaffected on Alice's upkeep"
    );
}
