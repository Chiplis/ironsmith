//! Measures how often tapping a permanent forces a *global* continuous-state
//! invalidation, across the whole card corpus, and what that costs.
//!
//!   cargo build --release -p ironsmith-tools --bin probe_tap_invalidation
//!   probe_tap_invalidation corpus <out_dir> [limit]
//!   probe_tap_invalidation cost <globalizer card name> [runs]

use ironsmith::ability::AbilityKind;
use ironsmith::cards::{CardDefinition, generated_definition_has_unimplemented_content};
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
use ironsmith_tools::{
    compile_definition_from_payload, default_cards_path, load_registry_cards,
};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

fn vanilla(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn variant_name(debug: &str) -> String {
    debug
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn static_kind(ability: &ironsmith::static_abilities::StaticAbility) -> String {
    match ability.compiled_model() {
        Some(model) => {
            let debug = format!("{:?}", model.payload);
            let mut kind = variant_name(&debug);
            if matches!(kind.as_str(), "None" | "SourceLineKeywordGroup" | "SelfSubjectSurface") {
                let id = model.id.as_ref().map(|id| variant_name(&format!("{id:?}")));
                kind = format!("{kind}#{}", id.unwrap_or_else(|| "noid".into()));
            }
            if matches!(
                kind.as_str(),
                "Conditional" | "GrantObjectAbilityForFilter" | "Grants"
            ) {
                if let Some(pos) = debug.find("payload: ") {
                    kind = format!("{kind}>{}", variant_name(&debug[pos + 9..]));
                } else if let Some(pos) = debug.find("grantable: ") {
                    kind = format!("{kind}>{}", variant_name(&debug[pos + 11..]));
                }
            }
            kind
        }
        None => format!("NoModel:{}", variant_name(&format!("{ability:?}"))),
    }
}

fn static_kinds(def: &CardDefinition) -> Vec<String> {
    def.abilities
        .iter()
        .filter_map(|a| match &a.kind {
            AbilityKind::Static(s) => Some(static_kind(s)),
            _ => None,
        })
        .collect()
}

struct Fixtures {
    forest: CardDefinition,
    bear: CardDefinition,
}

#[derive(Debug, Default, Clone)]
struct TapVerdict {
    land_global: bool,
    creature_global: bool,
    untap_global: bool,
    note: String,
}

fn delta(a: &WorkCounterSnapshot, b: &WorkCounterSnapshot) -> (u64, u64) {
    (
        b.continuous_global_invalidations - a.continuous_global_invalidations,
        b.continuous_local_invalidations - a.continuous_local_invalidations,
    )
}

fn run_tap_test(fx: &Fixtures, card: Option<&CardDefinition>) -> TapVerdict {
    let mut verdict = TapVerdict::default();
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let p0 = PlayerId::from_index(0);
    let p1 = PlayerId::from_index(1);
    let lands: Vec<ObjectId> = (0..4)
        .map(|_| game.create_object_from_definition(&fx.forest, p0, Zone::Battlefield))
        .collect();
    let bears: Vec<ObjectId> = (0..2)
        .map(|_| game.create_object_from_definition(&fx.bear, p0, Zone::Battlefield))
        .collect();
    for &b in &bears {
        game.remove_summoning_sickness(b);
    }
    game.create_object_from_definition(&fx.forest, p1, Zone::Battlefield);
    if let Some(def) = card {
        game.create_object_from_definition(def, p0, Zone::Battlefield);
    }
    game.turn.active_player = p0;
    game.turn.priority_player = Some(p0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    for _ in 0..3 {
        if let Err(e) = game.refresh_continuous_state() {
            verdict.note = format!("refresh_err:{}", variant_name(&format!("{e:?}")));
            return verdict;
        }
        if game.continuous_state_is_clean_public() {
            break;
        }
    }
    if !game.continuous_state_is_clean_public() {
        verdict.note = "never_clean".into();
        return verdict;
    }
    let c0 = game.work_counters();
    game.tap(lands[0]);
    let c1 = game.work_counters();
    verdict.land_global = delta(&c0, &c1).0 > 0;
    let _ = game.refresh_continuous_state();
    let c0 = game.work_counters();
    game.tap(bears[0]);
    let c1 = game.work_counters();
    verdict.creature_global = delta(&c0, &c1).0 > 0;
    let _ = game.refresh_continuous_state();
    let c0 = game.work_counters();
    game.untap(bears[0]);
    let c1 = game.work_counters();
    verdict.untap_global = delta(&c0, &c1).0 > 0;
    if delta(&c0, &c1) == (0, 0) {
        verdict.note = "untap_noop".into();
    }
    verdict
}

fn only_ability(def: &CardDefinition, keep: usize) -> CardDefinition {
    let mut d = def.clone();
    let labels_match = d.ability_labels.len() == d.abilities.len();
    d.abilities = vec![def.abilities[keep].clone()];
    if labels_match {
        d.ability_labels = vec![def.ability_labels[keep].clone()];
    }
    d
}

fn is_permanent_type(type_line: &str) -> bool {
    let head = type_line.split('—').next().unwrap_or(type_line);
    ["Artifact", "Creature", "Enchantment", "Land", "Planeswalker", "Battle"]
        .iter()
        .any(|t| head.contains(t))
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn corpus(out_dir: &str, limit: Option<usize>) {
    std::fs::create_dir_all(out_dir).unwrap();
    let t0 = Instant::now();
    let records = load_registry_cards(default_cards_path().to_str().unwrap()).unwrap();
    eprintln!("loaded {} records in {:?}", records.len(), t0.elapsed());
    let mut items: Vec<_> = records
        .into_values()
        .filter(|r| r.type_line.as_deref().is_some_and(is_permanent_type))
        .collect();
    let skip: std::collections::HashSet<String> = std::env::var("SKIP_CARDS_FILE")
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default();
    items.retain(|r| !skip.contains(&r.payload.name));
    if let Some(n) = limit {
        items.truncate(n);
    }
    let progress = std::sync::Mutex::new(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(format!("{out_dir}/progress.log"))
            .unwrap(),
    );
    let fx_forest = {
        let recs = ironsmith_tools::load_card_payloads_by_name(
            default_cards_path().to_str().unwrap(),
            "Forest",
        )
        .unwrap();
        compile_definition_from_payload(&recs[0]).unwrap()
    };
    let fx = Fixtures { forest: fx_forest, bear: vanilla("Probe Bear") };
    let baseline = run_tap_test(&fx, None);
    eprintln!("baseline: {baseline:?}");
    assert!(!baseline.land_global, "baseline tap is global; investigate");
    std::panic::set_hook(Box::new(|_| {}));

    #[derive(Default)]
    struct Row {
        name: String,
        type_line: String,
        edhrec_rank: Option<u64>,
        status: String,
        verdict: TapVerdict,
        kinds: Vec<String>,
        culprits: Vec<String>,
    }
    let done = std::sync::atomic::AtomicUsize::new(0);
    let total = items.len();
    let pool = rayon::ThreadPoolBuilder::new()
        .stack_size(256 << 20)
        .build()
        .unwrap();
    let rows: Vec<Row> = pool.install(|| {
        items
            .par_iter()
            .map(|rec| {
                let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if n % 2000 == 0 {
                    eprintln!("{n}/{total} {:?}", t0.elapsed());
                }
                {
                    let mut f = progress.lock().unwrap();
                    let _ = writeln!(f, "S\t{}", rec.payload.name);
                }
                let row = (|| {
                let edhrec_rank = serde_json::from_str::<serde_json::Value>(&rec.raw_card_json)
                    .ok()
                    .and_then(|v| v.get("edhrec_rank").and_then(|r| r.as_u64()));
                let mut row = Row {
                    name: rec.payload.name.clone(),
                    type_line: rec.type_line.clone().unwrap_or_default(),
                    edhrec_rank,
                    ..Default::default()
                };
                let compiled = catch_unwind(AssertUnwindSafe(|| {
                    compile_definition_from_payload(&rec.payload)
                }));
                let def = match compiled {
                    Ok(Ok(def)) => def,
                    Ok(Err(_)) => {
                        row.status = "compile_failed".into();
                        return row;
                    }
                    Err(_) => {
                        row.status = "compile_panic".into();
                        return row;
                    }
                };
                if generated_definition_has_unimplemented_content(&def) {
                    row.status = "unimplemented".into();
                    return row;
                }
                row.kinds = static_kinds(&def);
                match catch_unwind(AssertUnwindSafe(|| run_tap_test(&fx, Some(&def)))) {
                    Ok(v) => {
                        row.status = if v.note.starts_with("refresh_err") || v.note == "never_clean" {
                            v.note.clone()
                        } else {
                            "ok".into()
                        };
                        row.verdict = v;
                    }
                    Err(_) => {
                        row.status = "test_panic".into();
                        return row;
                    }
                }
                if row.verdict.land_global {
                    for (i, a) in def.abilities.iter().enumerate() {
                        if let AbilityKind::Static(s) = &a.kind {
                            let single = only_ability(&def, i);
                            let r = catch_unwind(AssertUnwindSafe(|| run_tap_test(&fx, Some(&single))));
                            if r.is_ok_and(|v| v.land_global) {
                                row.culprits.push(static_kind(s));
                            }
                        }
                    }
                    if row.culprits.is_empty() {
                        let mut stripped = def.clone();
                        stripped.abilities.clear();
                        stripped.ability_labels.clear();
                        let r = catch_unwind(AssertUnwindSafe(|| run_tap_test(&fx, Some(&stripped))));
                        row.culprits.push(if r.is_ok_and(|v| v.land_global) {
                            "NO_ABILITIES_STILL_GLOBAL".into()
                        } else {
                            "COMBINATION_OR_NONSTATIC".into()
                        });
                    }
                }
                row
                })();
                {
                    let mut f = progress.lock().unwrap();
                    let _ = writeln!(f, "E\t{}", rec.payload.name);
                }
                row
            })
            .collect()
    });
    let _ = std::panic::take_hook();
    eprintln!("probed {} cards in {:?}", rows.len(), t0.elapsed());

    let mut f = std::fs::File::create(format!("{out_dir}/tap_verdicts.csv")).unwrap();
    writeln!(f, "card,type_line,edhrec_rank,status,land_tap_global,creature_tap_global,untap_global,culprit_kinds,static_kinds").unwrap();
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    let mut culprit_hist: BTreeMap<String, (usize, f64, Vec<(u64, String)>)> = BTreeMap::new();
    let mut tested = 0usize;
    let mut global = 0usize;
    let mut creature_global = 0usize;
    let mut untap_global = 0usize;
    let mut weight_total = 0f64;
    let mut weight_global = 0f64;
    let mut offenders: Vec<(u64, String, String)> = Vec::new();
    for r in &rows {
        *by_status.entry(r.status.clone()).or_default() += 1;
        writeln!(
            f,
            "{},{},{},{},{},{},{},{},{}",
            csv_escape(&r.name),
            csv_escape(&r.type_line),
            r.edhrec_rank.map(|x| x.to_string()).unwrap_or_default(),
            r.status,
            r.verdict.land_global,
            r.verdict.creature_global,
            r.verdict.untap_global,
            csv_escape(&r.culprits.join("|")),
            csv_escape(&r.kinds.join("|"))
        )
        .unwrap();
        if r.status != "ok" {
            continue;
        }
        tested += 1;
        // Popularity weight: 1/edhrec_rank (rank 1 = most played on EDHREC).
        let w = r.edhrec_rank.map(|x| 1.0 / x as f64).unwrap_or(0.0);
        weight_total += w;
        creature_global += r.verdict.creature_global as usize;
        untap_global += r.verdict.untap_global as usize;
        if r.verdict.land_global {
            global += 1;
            weight_global += w;
            let mut seen = std::collections::BTreeSet::new();
            for c in &r.culprits {
                if !seen.insert(c.clone()) {
                    continue;
                }
                let e = culprit_hist.entry(c.clone()).or_default();
                e.0 += 1;
                e.1 += w;
                e.2.push((r.edhrec_rank.unwrap_or(u64::MAX), r.name.clone()));
            }
            offenders.push((
                r.edhrec_rank.unwrap_or(u64::MAX),
                r.name.clone(),
                r.culprits.join("|"),
            ));
        }
    }
    offenders.sort();
    let mut s = String::new();
    use std::fmt::Write as _;
    writeln!(s, "baseline (no extra card): {baseline:?}").unwrap();
    writeln!(s, "status counts: {by_status:?}").unwrap();
    writeln!(
        s,
        "tested ok: {tested}; land tap global: {global} ({:.2}%); creature tap global: {creature_global}; creature untap global: {untap_global}",
        100.0 * global as f64 / tested.max(1) as f64
    )
    .unwrap();
    writeln!(
        s,
        "popularity-weighted (1/edhrec_rank) share globalizing: {:.2}%",
        100.0 * weight_global / weight_total.max(1e-12)
    )
    .unwrap();
    writeln!(s, "\nculprit payload kinds (attributed by single-static isolation):").unwrap();
    let mut hist: Vec<_> = culprit_hist.into_iter().collect();
    hist.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    for (k, (n, w, mut ex)) in hist {
        ex.sort();
        let ex: Vec<_> = ex.iter().take(4).map(|e| e.1.clone()).collect();
        writeln!(s, "{n:6}  w={:.4}  {k}   e.g. {}", w, ex.join("; ")).unwrap();
    }
    writeln!(s, "\ntop 50 most-played (EDHREC rank) tap-globalizing cards:").unwrap();
    for (rank, name, c) in offenders.iter().take(50) {
        writeln!(s, "{rank:7}  {name}  [{c}]").unwrap();
    }
    std::fs::write(format!("{out_dir}/summary.txt"), &s).unwrap();
    println!("{s}");
}

// ---------------------------------------------------------------- cost probe

struct Board {
    game: GameState,
    p0_lands: Vec<ObjectId>,
    p0_attackers: Vec<ObjectId>,
}

fn build_board(
    defs: &BTreeMap<String, CardDefinition>,
    per_player: usize,
    globalizer: Option<&str>,
) -> Board {
    let names: Vec<String> = (0..4).map(|i| format!("P{i}")).collect();
    let mut game = GameState::new(names, 40);
    let bear = vanilla("Probe Bear");
    let nonland = board_cards();
    let mut p0_lands = Vec::new();
    let mut p0_attackers = Vec::new();
    for p in 0..4 {
        let pid = PlayerId::from_index(p);
        let lands = per_player * 2 / 5;
        for _ in 0..lands {
            let id = game.create_object_from_definition(&defs["Forest"], pid, Zone::Battlefield);
            if p == 0 {
                p0_lands.push(id);
            }
        }
        let mut placed = lands;
        for name in &nonland {
            if placed >= per_player {
                break;
            }
            let id = game.create_object_from_definition(&defs[name.as_str()], pid, Zone::Battlefield);
            game.remove_summoning_sickness(id);
            placed += 1;
        }
        let mut extra_slot = p == 0;
        while placed < per_player {
            let id = if extra_slot {
                extra_slot = false;
                match globalizer {
                    Some(g) => game.create_object_from_definition(&defs[g], pid, Zone::Battlefield),
                    None => game.create_object_from_definition(&bear, pid, Zone::Battlefield),
                }
            } else {
                game.create_object_from_definition(&bear, pid, Zone::Battlefield)
            };
            game.remove_summoning_sickness(id);
            if p == 0
                && game.object(id).is_some_and(|o| o.name.as_ref() == "Probe Bear")
                && p0_attackers.len() < 4
            {
                p0_attackers.push(id);
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
    game.refresh_continuous_state().unwrap();
    assert!(game.continuous_state_is_clean_public());
    Board { game, p0_lands, p0_attackers }
}

fn board_cards() -> Vec<String> {
    std::env::var("BOARD_CARDS")
        .unwrap_or_else(|_| {
            "Llanowar Elves,Elvish Mystic,Glorious Anthem,Elvish Archdruid,Sol Ring,Arcane Signet,Rhystic Study,Smothering Tithe,Intangible Virtue,Lord of Atlantis".into()
        })
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn warm(game: &GameState) -> GameState {
    let mut g = game.clone();
    g.refresh_continuous_state().unwrap();
    let _ = compute_legal_actions(&g, PlayerId::from_index(0));
    g.refresh_continuous_state().unwrap();
    assert!(g.continuous_state_is_clean_public());
    g
}

fn min_ms<F: FnMut() -> f64>(runs: usize, mut f: F) -> f64 {
    (0..runs).map(|_| f()).fold(f64::INFINITY, f64::min)
}

fn cost(globalizer: &str, runs: usize) {
    let mut wanted: Vec<String> = vec!["Forest".to_string()];
    wanted.extend(board_cards());
    wanted.push(globalizer.to_string());
    let payloads =
        ironsmith_tools::load_card_payloads_by_names(default_cards_path().to_str().unwrap(), &wanted)
            .unwrap();
    let defs: BTreeMap<String, CardDefinition> = wanted
        .iter()
        .map(|n| {
            let p = &payloads[&ironsmith_tools::normalize_lookup_name(n)][0];
            (n.clone(), compile_definition_from_payload(p).unwrap())
        })
        .collect();
    let p0 = PlayerId::from_index(0);
    let p1 = PlayerId::from_index(1);
    println!("globalizer = {globalizer}; runs = {runs} (min reported, ms)");
    for per_player in [25usize, 40] {
        for (label, g) in [("local", None), ("global", Some(globalizer))] {
            let board = build_board(&defs, per_player, g);
            let bf = board.game.battlefield.len();
            // Verify the classification of a single land tap on this board.
            let mut probe = board.game.clone();
            let c0 = probe.work_counters();
            probe.tap(board.p0_lands[0]);
            let (gl, lo) = delta(&c0, &probe.work_counters());

            // (a) refresh after one tap
            let refresh = min_ms(runs, || {
                let mut g = warm(&board.game);
                g.tap(board.p0_lands[0]);
                let t = Instant::now();
                g.refresh_continuous_state().unwrap();
                t.elapsed().as_secs_f64() * 1e3
            });
            let refresh_noop = min_ms(runs, || {
                let mut g = warm(&board.game);
                let t = Instant::now();
                g.refresh_continuous_state().unwrap();
                t.elapsed().as_secs_f64() * 1e3
            });
            let mana_probe = {
                let mut g = warm(&board.game);
                let c0 = g.work_counters();
                let actions = compute_legal_actions(&g, p0).unwrap();
                let action = actions
                    .into_iter()
                    .find(|a| matches!(a, LegalAction::ActivateManaAbility { source, .. } if *source == board.p0_lands[0]))
                    .expect("land mana action");
                let c1 = g.work_counters();
                let mut queue = TriggerQueue::default();
                let mut state = PriorityLoopState::new(g.players_in_game());
                apply_priority_response_with_dm(&mut g, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).unwrap();
                let c2 = g.work_counters();
                format!("legal={:?} activate={:?}", delta(&c0, &c1), delta(&c1, &c2))
            };
            let tap_only = min_ms(runs, || {
                let mut g = warm(&board.game);
                let t = Instant::now();
                g.tap(board.p0_lands[0]);
                t.elapsed().as_secs_f64() * 1e3
            });
            let (clean_after_tap, second_refresh) = {
                let mut g = warm(&board.game);
                g.tap(board.p0_lands[0]);
                let rev_before = board.game.effect_store.continuous_effects.revision();
                let rev_after = g.effect_store.continuous_effects.revision();
                let clean = g.continuous_state_is_clean_public();
                let c0 = g.work_counters();
                let t1 = Instant::now();
                g.refresh_continuous_state().unwrap();
                let first = t1.elapsed().as_secs_f64() * 1e3;
                let c1 = g.work_counters();
                let tc = Instant::now();
                let cl = g.clone();
                let clone_ms = tc.elapsed().as_secs_f64() * 1e3;
                drop(cl);
                println!("  first refresh after tap {first:.3}ms; game.clone() {clone_ms:.3}ms; effects revision {rev_before}->{rev_after}");
                let t = Instant::now();
                g.refresh_continuous_state().unwrap();
                (clean, format!("refresh work: regens={} recomputes={} effects={} global_inval_during_refresh={}; second refresh {:.3}ms",
                    c1.static_ability_regens - c0.static_ability_regens,
                    c1.characteristics_full_recomputes - c0.characteristics_full_recomputes,
                    c1.effects_considered - c0.effects_considered,
                    c1.continuous_global_invalidations - c0.continuous_global_invalidations,
                    t.elapsed().as_secs_f64() * 1e3))
            };
            println!("  board={bf} {label}: tap_only={tap_only:.3} clean_after_tap={clean_after_tap} {second_refresh}");
            println!("  board={bf} {label}: refresh_noop={refresh_noop:.3} one priority mana activation (global,local): {mana_probe}");
            // (b) legal actions right after one tap (no explicit refresh)
            let legal_after_tap = min_ms(runs, || {
                let mut g = warm(&board.game);
                g.tap(board.p0_lands[0]);
                let t = Instant::now();
                let _ = compute_legal_actions(&g, p0).unwrap();
                t.elapsed().as_secs_f64() * 1e3
            });
            let legal_clean = min_ms(runs, || {
                let g = warm(&board.game);
                let t = Instant::now();
                let _ = compute_legal_actions(&g, p0).unwrap();
                t.elapsed().as_secs_f64() * 1e3
            });
            // (b') tap + refresh + legal actions, the realistic sequence
            let tap_refresh_legal = min_ms(runs, || {
                let mut g = warm(&board.game);
                let t = Instant::now();
                g.tap(board.p0_lands[0]);
                g.refresh_continuous_state().unwrap();
                let _ = compute_legal_actions(&g, p0).unwrap();
                t.elapsed().as_secs_f64() * 1e3
            });

            // (c) simulated turn
            let mut turn_counts = (0u64, 0u64, 0u64, 0u64);
            let mut phase_ms = [f64::INFINITY; 3];
            let turn_ms = min_ms(runs, || {
                let mut g = board.game.clone();
                for &id in board.p0_lands.iter().chain(&board.p0_attackers) {
                    g.tap(id);
                }
                g.turn.phase = Phase::Beginning;
                g.turn.step = Some(Step::Untap);
                g.turn.priority_player = None;
                g.refresh_continuous_state().unwrap();
                let c0 = g.work_counters();
                let t = Instant::now();
                ironsmith::turn::execute_untap_step(&mut g);
                g.turn.phase = Phase::FirstMain;
                g.turn.step = None;
                g.turn.priority_player = Some(p0);
                g.refresh_continuous_state().unwrap();
                let t_untap = t.elapsed().as_secs_f64() * 1e3;
                let c_untap = g.work_counters();
                let mut queue = TriggerQueue::default();
                let mut state = PriorityLoopState::new(g.players_in_game());
                for &land in board.p0_lands.iter().take(6) {
                    let actions = compute_legal_actions(&g, p0).unwrap();
                    let action = actions
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
                let t_mana = t.elapsed().as_secs_f64() * 1e3;
                let c_mana = g.work_counters();
                g.turn.phase = Phase::Combat;
                g.turn.step = Some(Step::DeclareAttackers);
                g.refresh_continuous_state().unwrap();
                let decls: Vec<_> = board
                    .p0_attackers
                    .iter()
                    .map(|&c| AttackerDeclaration { creature: c, target: AttackTarget::Player(p1) })
                    .collect();
                let mut combat = CombatState::default();
                apply_attacker_declarations(&mut g, &mut combat, &mut queue, &decls).unwrap();
                g.refresh_continuous_state().unwrap();
                let elapsed = t.elapsed().as_secs_f64() * 1e3;
                let c1 = g.work_counters();
                let (ug, ul) = delta(&c0, &c_untap);
                let (mg, ml) = delta(&c_untap, &c_mana);
                let (ag, al) = delta(&c_mana, &c1);
                turn_counts = (ug, mg, ag, ul + ml + al);
                let _ = (ul, ml, al);
                phase_ms[0] = phase_ms[0].min(t_untap);
                phase_ms[1] = phase_ms[1].min(t_mana - t_untap);
                phase_ms[2] = phase_ms[2].min(elapsed - t_mana);
                let regens = c1.static_ability_regens - c0.static_ability_regens;
                let recomputes =
                    c1.characteristics_full_recomputes - c0.characteristics_full_recomputes;
                eprintln!(
                    "  [{label} {bf}] turn global(untap,mana,attack)=({ug},{mg},{ag}) local={} regens={regens} recomputes={recomputes}",
                    ul + ml + al
                );
                elapsed
            });
            println!(
                "board={bf} perms case={label}: one-land-tap global={gl} local={lo} | refresh_after_tap={refresh:.3} | legal_after_tap(no refresh)={legal_after_tap:.3} legal_clean={legal_clean:.3} tap+refresh+legal={tap_refresh_legal:.3} | turn total={turn_ms:.3} (untap={:.3} mana6={:.3} attack4={:.3}) turn global invalidations (untap,mana,attack)=({},{},{}) local={}",
                phase_ms[0], phase_ms[1], phase_ms[2], turn_counts.0, turn_counts.1, turn_counts.2, turn_counts.3
            );
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("corpus") => corpus(
            args.get(2).map(String::as_str).unwrap_or("tap-probe"),
            args.get(3).and_then(|s| s.parse().ok()),
        ),
        Some("cost") => cost(
            args.get(2).map(String::as_str).unwrap_or("Llanowar Elves"),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(7),
        ),
        _ => eprintln!("usage: probe_tap_invalidation corpus <out_dir> [limit] | cost <card> [runs]"),
    }
}
