//! Times continuous-state refreshes, legal-action listing and priority passes
//! on a small 2-player board whose static abilities read other permanents'
//! characteristics ("as long as you control another Dwarf").
//!
//!   cargo build --release -p ironsmith-tools --bin probe_static_conditions
//!   probe_static_conditions [runs]
//!
//! OPP_CARDS / OWN_CARDS / HAND_CARDS ('|' separated) override the board.
//! PHASE=refresh|legal|pass loops one measurement forever (for `sample`).

use ironsmith::cards::CardDefinition;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm};
use ironsmith::game_state::{GameState, Phase, WorkCounterSnapshot};
use ironsmith::ids::PlayerId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::zone::Zone;
use ironsmith_tools::{compile_definition_from_payload, default_cards_path};
use std::collections::BTreeMap;
use std::time::Instant;

fn list(var: &str, default: &str) -> Vec<String> {
    std::env::var(var)
        .unwrap_or_else(|_| default.into())
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn opp_cards() -> Vec<String> {
    list(
        "OPP_CARDS",
        "Colossus Hammer|Dragonfire Blade|Leyline Axe|Battlefield Forge|Battlefield Forge|Sacred Foundry|Dáin's Company|Sram, Senior Edificer",
    )
}

fn own_cards() -> Vec<String> {
    list("OWN_CARDS", "Spirebluff Canal|Spirebluff Canal")
}

fn hand_cards() -> Vec<String> {
    list(
        "HAND_CARDS",
        "Screaming Nemesis|Flow State|Burst Lightning|Reckless Rage|Island|Shivan Reef|Riverglide Pathway",
    )
}

fn build(defs: &BTreeMap<String, CardDefinition>) -> GameState {
    let mut game = GameState::new(vec!["P0".into(), "P1".into()], 20);
    let p0 = PlayerId::from_index(0);
    let p1 = PlayerId::from_index(1);
    for name in opp_cards() {
        let id = game.create_object_from_definition(&defs[&name], p1, Zone::Battlefield);
        game.remove_summoning_sickness(id);
    }
    for name in own_cards() {
        let id = game.create_object_from_definition(&defs[&name], p0, Zone::Battlefield);
        game.remove_summoning_sickness(id);
    }
    for name in hand_cards() {
        game.create_object_from_definition(&defs[&name], p0, Zone::Hand);
    }
    for _ in 0..20 {
        game.create_object_from_definition(&defs["Island"], p0, Zone::Library);
        game.create_object_from_definition(&defs["Island"], p1, Zone::Library);
    }
    game.turn.turn_number = 6;
    game.turn.active_player = p0;
    game.turn.priority_player = Some(p0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.refresh_continuous_state().unwrap();
    game
}

type Sample = (f64, WorkCounterSnapshot, WorkCounterSnapshot);

/// One global invalidation followed by a refresh: the floor every mutation pays.
fn run_refresh(base: &GameState) -> Sample {
    let mut g = base.clone();
    let c0 = g.work_counters();
    let t = Instant::now();
    g.invalidate_continuous_state_public();
    g.refresh_continuous_state().unwrap();
    (t.elapsed().as_secs_f64() * 1e3, c0, g.work_counters())
}

/// Legal actions after a fresh invalidation, as the priority menu is built.
fn run_legal(base: &GameState) -> Sample {
    let mut g = base.clone();
    g.invalidate_continuous_state_public();
    g.refresh_continuous_state().unwrap();
    let c0 = g.work_counters();
    let t = Instant::now();
    let _ = compute_legal_actions(&g, PlayerId::from_index(0)).unwrap();
    (t.elapsed().as_secs_f64() * 1e3, c0, g.work_counters())
}

/// Play a land, then list legal actions again (the slow action in the report).
fn run_pass(base: &GameState) -> Sample {
    let p0 = PlayerId::from_index(0);
    let mut g = base.clone();
    let mut queue = TriggerQueue::default();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let c0 = g.work_counters();
    let t = Instant::now();
    let action = compute_legal_actions(&g, p0)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::PlayLand { .. }))
        .expect("land play");
    apply_priority_response_with_dm(
        &mut g,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    let _ = compute_legal_actions(&g, p0).unwrap();
    (t.elapsed().as_secs_f64() * 1e3, c0, g.work_counters())
}

fn report(label: &str, runs: usize, f: impl Fn() -> Sample) {
    let mut best = f64::INFINITY;
    let mut counters = None;
    for _ in 0..runs {
        let (ms, c0, c1) = f();
        best = best.min(ms);
        counters = Some((c0, c1));
    }
    let (a, b) = counters.unwrap();
    println!(
        "{label:8} {best:9.2} ms | global={} recomputes={} cache_hits={} effects={} dep_sorts={} pairs={} regens={}",
        b.continuous_global_invalidations - a.continuous_global_invalidations,
        b.characteristics_full_recomputes - a.characteristics_full_recomputes,
        b.characteristics_cache_hits - a.characteristics_cache_hits,
        b.effects_considered - a.effects_considered,
        b.dependency_sorts - a.dependency_sorts,
        b.dependency_pairs_probed - a.dependency_pairs_probed,
        b.static_ability_regens - a.static_ability_regens,
    );
}

fn main() {
    let runs: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);
    let mut wanted = vec!["Island".to_string()];
    wanted.extend(opp_cards());
    wanted.extend(own_cards());
    wanted.extend(hand_cards());
    wanted.sort();
    wanted.dedup();
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
    let base = build(&defs);
    println!(
        "board: {} permanents; opp: {}",
        base.battlefield.len(),
        opp_cards().join(", ")
    );
    match std::env::var("PHASE").as_deref() {
        Ok("refresh") => loop {
            run_refresh(&base);
        },
        Ok("legal") => loop {
            run_legal(&base);
        },
        Ok("pass") => loop {
            run_pass(&base);
        },
        _ => {}
    }
    report("refresh", runs, || run_refresh(&base));
    report("legal", runs, || run_legal(&base));
    report("land+legal", runs, || run_pass(&base));
}
