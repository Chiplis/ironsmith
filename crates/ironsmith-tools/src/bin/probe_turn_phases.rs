//! Times the tap-heavy turn phases (untap step, mana activations, attack
//! declaration) on a 4-player board and reports work counters per phase.
//!
//!   cargo build --release -p ironsmith-tools --bin probe_turn_phases
//!   probe_turn_phases [per_player] [runs]
//!
//! BOARD_CARDS (comma separated) overrides the nonland cards each player gets;
//! PHASE=untap|mana|attack loops one phase forever (for `sample`).

use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_attacker_declarations,
    apply_priority_response_with_dm,
};
use ironsmith::game_state::{GameState, Phase, Step, WorkCounterSnapshot};
use ironsmith::ids::{CardId, ObjectId, PlayerId};
use ironsmith::triggers::TriggerQueue;
use ironsmith::zone::Zone;
use ironsmith::{AttackTarget, AttackerDeclaration, CombatState};
use ironsmith_registry::card::PowerToughness;
use ironsmith_registry::cards::builders::CardDefinitionBuilder;
use ironsmith_registry::types::CardType;
use ironsmith_tools::{compile_definition_from_payload, default_cards_path};
use std::collections::BTreeMap;
use std::time::Instant;

fn vanilla(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn board_cards() -> Vec<String> {
    std::env::var("BOARD_CARDS")
        .unwrap_or_else(|_| {
            "Llanowar Elves,Elvish Mystic,Glorious Anthem,Elvish Archdruid,Sol Ring,Arcane Signet,Rhystic Study,Smothering Tithe,Intangible Virtue,Lord of Atlantis,Swiftfoot Boots".into()
        })
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

struct Board {
    game: GameState,
    lands: Vec<ObjectId>,
    attackers: Vec<ObjectId>,
}

fn build_board(defs: &BTreeMap<String, CardDefinition>, per_player: usize) -> Board {
    let mut game = GameState::new((0..4).map(|i| format!("P{i}")).collect(), 40);
    let bear = vanilla("Probe Bear");
    let mut lands = Vec::new();
    let mut attackers = Vec::new();
    for p in 0..4 {
        let pid = PlayerId::from_index(p);
        let land_count = per_player * 2 / 5;
        for _ in 0..land_count {
            let id = game.create_object_from_definition(&defs["Forest"], pid, Zone::Battlefield);
            if p == 0 {
                lands.push(id);
            }
        }
        let mut placed = land_count;
        for name in board_cards() {
            if placed >= per_player {
                break;
            }
            let id = game.create_object_from_definition(&defs[&name], pid, Zone::Battlefield);
            game.remove_summoning_sickness(id);
            placed += 1;
        }
        while placed < per_player {
            let id = game.create_object_from_definition(&bear, pid, Zone::Battlefield);
            game.remove_summoning_sickness(id);
            if p == 0 && attackers.len() < 4 {
                attackers.push(id);
            }
            placed += 1;
        }
    }
    let p0 = PlayerId::from_index(0);
    game.turn.turn_number = 5;
    game.turn.active_player = p0;
    game.turn.priority_player = Some(p0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.refresh_continuous_state().unwrap();
    Board { game, lands, attackers }
}

fn untap_ready(board: &Board) -> GameState {
    let mut g = board.game.clone();
    for &id in board.lands.iter().chain(&board.attackers) {
        g.tap(id);
    }
    g.turn.phase = Phase::Beginning;
    g.turn.step = Some(Step::Untap);
    g.turn.priority_player = None;
    g.refresh_continuous_state().unwrap();
    g
}

fn main_ready(board: &Board) -> GameState {
    let mut g = board.game.clone();
    g.refresh_continuous_state().unwrap();
    let _ = compute_legal_actions(&g, PlayerId::from_index(0));
    g
}

fn run_untap(board: &Board) -> (f64, WorkCounterSnapshot, WorkCounterSnapshot) {
    let mut g = untap_ready(board);
    let c0 = g.work_counters();
    let t = Instant::now();
    ironsmith::turn::execute_untap_step(&mut g);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.turn.priority_player = Some(PlayerId::from_index(0));
    g.refresh_continuous_state().unwrap();
    (t.elapsed().as_secs_f64() * 1e3, c0, g.work_counters())
}

/// Activates six land mana abilities through the priority loop, then lists
/// legal actions once more, as a client would after each click.
fn run_mana(board: &Board) -> (f64, WorkCounterSnapshot, WorkCounterSnapshot) {
    let p0 = PlayerId::from_index(0);
    let mut g = main_ready(board);
    let mut queue = TriggerQueue::default();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let c0 = g.work_counters();
    let t = Instant::now();
    for &land in board.lands.iter().take(6) {
        let action = compute_legal_actions(&g, p0)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == land))
            .expect("land mana action");
        apply_priority_response_with_dm(
            &mut g,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
    }
    let _ = compute_legal_actions(&g, p0).unwrap();
    (t.elapsed().as_secs_f64() * 1e3, c0, g.work_counters())
}

fn run_attack(board: &Board) -> (f64, WorkCounterSnapshot, WorkCounterSnapshot) {
    let mut g = main_ready(board);
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::DeclareAttackers);
    g.refresh_continuous_state().unwrap();
    let decls: Vec<_> = board
        .attackers
        .iter()
        .map(|&c| AttackerDeclaration {
            creature: c,
            target: AttackTarget::Player(PlayerId::from_index(1)),
        })
        .collect();
    let mut queue = TriggerQueue::default();
    let mut combat = CombatState::default();
    let c0 = g.work_counters();
    let t = Instant::now();
    apply_attacker_declarations(&mut g, &mut combat, &mut queue, &decls).unwrap();
    g.refresh_continuous_state().unwrap();
    (t.elapsed().as_secs_f64() * 1e3, c0, g.work_counters())
}

fn report(label: &str, runs: usize, f: impl Fn() -> (f64, WorkCounterSnapshot, WorkCounterSnapshot)) {
    let mut best = f64::INFINITY;
    let mut counters = None;
    for _ in 0..runs {
        let (ms, c0, c1) = f();
        best = best.min(ms);
        counters = Some((c0, c1));
    }
    let (a, b) = counters.unwrap();
    println!(
        "{label:7} {best:10.2} ms | global={} local={} regens={} recomputes={} cache_hits={} derived_views={} dep_sorts={}",
        b.continuous_global_invalidations - a.continuous_global_invalidations,
        b.continuous_local_invalidations - a.continuous_local_invalidations,
        b.static_ability_regens - a.static_ability_regens,
        b.characteristics_full_recomputes - a.characteristics_full_recomputes,
        b.characteristics_cache_hits - a.characteristics_cache_hits,
        b.derived_view_rebuilds - a.derived_view_rebuilds,
        b.dependency_sorts - a.dependency_sorts,
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let per_player: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(25);
    let runs: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5);
    let mut wanted = vec!["Forest".to_string()];
    wanted.extend(board_cards());
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        default_cards_path().to_str().unwrap(),
        &wanted,
    )
    .unwrap();
    let defs: BTreeMap<String, CardDefinition> = wanted
        .iter()
        .map(|n| {
            let p = &payloads[&ironsmith_tools::normalize_lookup_name(n)][0];
            (n.clone(), compile_definition_from_payload(p).unwrap())
        })
        .collect();
    let board = build_board(&defs, per_player);
    println!(
        "board: {} permanents, cards: {}",
        board.game.battlefield.len(),
        board_cards().join(", ")
    );
    match std::env::var("PHASE").as_deref() {
        Ok("untap") => loop {
            run_untap(&board);
        },
        Ok("mana") => loop {
            run_mana(&board);
        },
        Ok("attack") => loop {
            run_attack(&board);
        },
        _ => {}
    }
    report("untap", runs, || run_untap(&board));
    report("mana6", runs, || run_mana(&board));
    report("attack4", runs, || run_attack(&board));
}
