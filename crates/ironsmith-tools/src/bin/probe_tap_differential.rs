//! Differential check for the local tap/untap invalidation fast path.
//!
//! For every permanent card in the corpus, builds a small board, then taps and
//! untaps permanents one at a time through `GameState::tap`/`untap`. After
//! each step it compares every battlefield object's calculated
//! characteristics, controller, and the can't-effect tracker against a copy
//! whose continuous state was discarded and recomputed from scratch.
//!
//!   cargo build --release -p ironsmith-tools --bin probe_tap_differential
//!   probe_tap_differential <out_dir> [limit]

use ironsmith::cards::{CardDefinition, generated_definition_has_unimplemented_content};
use ironsmith::game_state::{GameState, Phase};
use ironsmith::ids::{CardId, ObjectId, PlayerId};
use ironsmith::zone::Zone;
use ironsmith_registry::card::PowerToughness;
use ironsmith_registry::cards::builders::CardDefinitionBuilder;
use ironsmith_registry::types::CardType;
use ironsmith_tools::{compile_definition_from_payload, default_cards_path, load_registry_cards};
use rayon::prelude::*;
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

fn vanilla(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn is_permanent_type(type_line: &str) -> bool {
    let head = type_line.split('—').next().unwrap_or(type_line);
    ["Artifact", "Creature", "Enchantment", "Land", "Planeswalker", "Battle"]
        .iter()
        .any(|t| head.contains(t))
}

/// Everything a tap could make stale, rendered for comparison.
fn fingerprint(game: &GameState) -> Vec<(ObjectId, String)> {
    let mut ids: Vec<ObjectId> = game.battlefield.iter().copied().collect();
    ids.sort_by_key(|id| id.0);
    let mut out: Vec<_> = ids
        .into_iter()
        .map(|id| {
            let chars = game.calculated_characteristics(id);
            (
                id,
                format!("{:?} | controller={:?}", chars, game.current_controller(id)),
            )
        })
        .collect();
    out.push((
        ObjectId::from_raw(0),
        format!("cant={:?}", game.effect_store.cant_effects),
    ));
    out
}

fn first_difference(a: &str, b: &str) -> String {
    let at = a
        .char_indices()
        .zip(b.chars())
        .find(|((_, x), y)| x != y)
        .map(|((i, _), _)| i)
        .unwrap_or(a.len().min(b.len()));
    let start = at.saturating_sub(120);
    let end_a = (at + 160).min(a.len());
    let end_b = (at + 160).min(b.len());
    format!(
        "fast: …{}… | full: …{}…",
        a.get(start..end_a).unwrap_or(""),
        b.get(start..end_b).unwrap_or("")
    )
}

struct Outcome {
    local_steps: usize,
    global_steps: usize,
    mismatch: Option<String>,
}

fn refresh(game: &mut GameState) -> Result<(), String> {
    for _ in 0..3 {
        game.refresh_continuous_state()
            .map_err(|e| format!("refresh_err:{e:?}"))?;
        if game.continuous_state_is_clean_public() {
            return Ok(());
        }
    }
    Err("never_clean".into())
}

fn run(forest: &CardDefinition, bear: &CardDefinition, card: &CardDefinition) -> Result<Outcome, String> {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let p0 = PlayerId::from_index(0);
    let p1 = PlayerId::from_index(1);
    let mut order = Vec::new();
    for player in [p0, p1] {
        let card_id = game.create_object_from_definition(card, player, Zone::Battlefield);
        game.remove_summoning_sickness(card_id);
        let land = game.create_object_from_definition(forest, player, Zone::Battlefield);
        let bear_id = game.create_object_from_definition(bear, player, Zone::Battlefield);
        game.remove_summoning_sickness(bear_id);
        order.extend([card_id, land, bear_id]);
    }
    game.turn.active_player = p0;
    game.turn.priority_player = Some(p0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    refresh(&mut game)?;

    let mut outcome = Outcome { local_steps: 0, global_steps: 0, mismatch: None };
    // Tap each in turn, then untap each in turn: covers tapped sources,
    // tapped recipients, and counts of tapped permanents in both directions.
    let steps: Vec<(ObjectId, bool)> = order
        .iter()
        .map(|&id| (id, true))
        .chain(order.iter().map(|&id| (id, false)))
        .collect();
    for (id, tap) in steps {
        if game.object(id).is_none_or(|object| object.zone != Zone::Battlefield) {
            continue;
        }
        // Warm every cached result the fast path must keep correct.
        let _ = fingerprint(&game);
        let before = game.work_counters();
        if tap {
            game.tap(id);
        } else {
            game.untap(id);
        }
        let after = game.work_counters();
        if after.continuous_global_invalidations > before.continuous_global_invalidations {
            outcome.global_steps += 1;
        } else {
            outcome.local_steps += 1;
        }
        refresh(&mut game)?;
        let fast = fingerprint(&game);
        let full_state = game.clone();
        full_state.invalidate_continuous_state_public();
        let mut full_state = full_state;
        refresh(&mut full_state)?;
        let full = fingerprint(&full_state);
        if let Some(((object, a), (_, b))) = fast.iter().zip(&full).find(|(a, b)| a != b) {
            let name = game
                .object(*object)
                .map(|o| o.name.to_string())
                .unwrap_or_else(|| "<tracker>".into());
            outcome.mismatch = Some(format!(
                "{} {:?} → object {name}: {}",
                if tap { "tap" } else { "untap" },
                game.object(id).map(|o| o.name.to_string()),
                first_difference(a, b)
            ));
            return Ok(outcome);
        }
        if fast.len() != full.len() {
            outcome.mismatch = Some("battlefield size differs".into());
            return Ok(outcome);
        }
    }
    Ok(outcome)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_dir = args.get(1).map(String::as_str).unwrap_or("tap-differential");
    let limit: Option<usize> = args.get(2).and_then(|s| s.parse().ok());
    let only: Option<Vec<String>> = std::env::var("ONLY_CARDS")
        .ok()
        .map(|v| v.split('|').map(str::to_string).collect());
    std::fs::create_dir_all(out_dir).unwrap();
    let t0 = Instant::now();
    let records = load_registry_cards(default_cards_path().to_str().unwrap()).unwrap();
    let mut items: Vec<_> = records
        .into_values()
        .filter(|r| r.type_line.as_deref().is_some_and(is_permanent_type))
        .filter(|r| only.as_ref().is_none_or(|names| names.contains(&r.payload.name)))
        .collect();
    items.sort_by(|a, b| a.payload.name.cmp(&b.payload.name));
    if let Some(n) = limit {
        items.truncate(n);
    }
    let forest = {
        let recs = ironsmith_tools::load_card_payloads_by_name(
            default_cards_path().to_str().unwrap(),
            "Forest",
        )
        .unwrap();
        compile_definition_from_payload(&recs[0]).unwrap()
    };
    let bear = vanilla("Probe Bear");
    std::panic::set_hook(Box::new(|_| {}));
    let done = AtomicUsize::new(0);
    let total = items.len();
    let pool = rayon::ThreadPoolBuilder::new().stack_size(256 << 20).build().unwrap();
    let rows: Vec<(String, String, usize, usize, String)> = pool.install(|| {
        items
            .par_iter()
            .map(|rec| {
                let n = done.fetch_add(1, Ordering::Relaxed);
                if n % 2000 == 0 {
                    eprintln!("{n}/{total} {:?}", t0.elapsed());
                }
                let name = rec.payload.name.clone();
                let def = match catch_unwind(AssertUnwindSafe(|| compile_definition_from_payload(&rec.payload))) {
                    Ok(Ok(def)) => def,
                    _ => return (name, "compile_failed".into(), 0, 0, String::new()),
                };
                if generated_definition_has_unimplemented_content(&def) {
                    return (name, "unimplemented".into(), 0, 0, String::new());
                }
                match catch_unwind(AssertUnwindSafe(|| run(&forest, &bear, &def))) {
                    Ok(Ok(o)) => match o.mismatch {
                        Some(m) => (name, "MISMATCH".into(), o.local_steps, o.global_steps, m),
                        None => (name, "ok".into(), o.local_steps, o.global_steps, String::new()),
                    },
                    Ok(Err(e)) => (name, e, 0, 0, String::new()),
                    Err(_) => (name, "panic".into(), 0, 0, String::new()),
                }
            })
            .collect()
    });
    let mut csv = std::fs::File::create(format!("{out_dir}/differential.csv")).unwrap();
    writeln!(csv, "card\tstatus\tlocal_steps\tglobal_steps\tdetail").unwrap();
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    let (mut local, mut global) = (0usize, 0usize);
    let mut all_local_cards = 0usize;
    for (name, status, l, g, detail) in &rows {
        writeln!(csv, "{name}\t{status}\t{l}\t{g}\t{}", detail.replace('\t', " ").replace('\n', " ")).unwrap();
        let key = if status.starts_with("refresh_err") { "refresh_err".to_string() } else { status.clone() };
        *counts.entry(key).or_default() += 1;
        local += l;
        global += g;
        if status == "ok" && *g == 0 {
            all_local_cards += 1;
        }
    }
    println!("cards: {counts:?}");
    println!(
        "tap/untap steps: local={local} global={global} ({:.1}% local); cards whose every step stayed local: {all_local_cards}",
        100.0 * local as f64 / (local + global).max(1) as f64
    );
    for (name, status, _, _, detail) in rows.iter().filter(|r| r.1 == "MISMATCH").take(40) {
        println!("{status} {name}: {detail}");
    }
    println!("elapsed {:?}", t0.elapsed());
}
