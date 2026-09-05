//! Counts inputs on which a ranked registry's order, not its grammar, chose
//! the reading.
//!
//! A ranked registry (see `ironsmith_compiler::registry::RANKED_REGISTRIES`)
//! runs every viable rule and keeps registration order as its tie-break while
//! its overlaps are being resolved. This audit runs the corpus with the
//! overlap ledger on and reports, per registry, how many sampled cards hit an
//! overlap and which rule pairs, with example cards and text. The report
//! prints the ranked-registry count first (one ratchet) and the overlap count
//! second (another).
//!
//! `--every N` samples every Nth card (default 20); `--all` runs the corpus;
//! `--registries-only` prints the ranked-registry count without compiling.

use ironsmith_tools::{
    compile_strict_snapshot_from_payload, default_cards_path, load_canonical_cards,
};
use std::collections::BTreeMap;

fn main() {
    let mut every = 20usize;
    let mut registries_only = false;
    let mut cards_path = default_cards_path().display().to_string();
    let mut only_name: Option<String> = None;
    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--every" => {
                every = iter
                    .next()
                    .and_then(|value| value.parse().ok())
                    .expect("--every requires a positive integer");
            }
            "--all" => every = 1,
            "--registries-only" => registries_only = true,
            "--cards" => cards_path = iter.next().expect("--cards requires a path"),
            "--name" => only_name = Some(iter.next().expect("--name requires a card name")),
            other => panic!("unknown argument {other}"),
        }
    }

    let ranked = ironsmith_compiler::registry::RANKED_REGISTRIES;
    println!("ranked registries: {}", ranked.len());
    for registry in ranked {
        println!("  {registry}");
    }
    if registries_only {
        return;
    }

    let cards = load_canonical_cards(&cards_path)
        .unwrap_or_else(|error| panic!("failed to load cards from {cards_path}: {error}"));

    let mut sampled = 0usize;
    let mut cards_with_overlaps = 0usize;
    let mut total = 0usize;
    // (registry, rules) -> (count, example card, example text)
    let mut by_pair: BTreeMap<(String, String), (usize, String, String)> = BTreeMap::new();
    let mut by_registry: BTreeMap<String, usize> = BTreeMap::new();
    for (index, (name, payload)) in cards.iter().enumerate() {
        if let Some(only) = &only_name {
            if name != only {
                continue;
            }
        } else if index % every != 0 {
            continue;
        }
        sampled += 1;
        ironsmith_compiler::overlap_ledger::begin();
        let _snapshot = compile_strict_snapshot_from_payload(payload);
        let mut overlaps = ironsmith_compiler::overlap_ledger::end();
        overlaps.sort();
        overlaps.dedup();
        if overlaps.is_empty() {
            continue;
        }
        cards_with_overlaps += 1;
        for overlap in overlaps {
            total += 1;
            *by_registry.entry(overlap.registry.to_string()).or_default() += 1;
            let entry = by_pair
                .entry((overlap.registry.to_string(), overlap.rules.join(" | ")))
                .or_insert_with(|| (0, name.clone(), overlap.text.clone()));
            entry.0 += 1;
        }
    }

    println!("cards sampled: {sampled} (every {every})");
    println!("cards with overlaps: {cards_with_overlaps}");
    println!("registry overlaps: {total}");
    for (registry, count) in &by_registry {
        println!("  {count:6}  {registry}");
    }
    println!("rule pairs: {}", by_pair.len());
    let mut rows: Vec<_> = by_pair.iter().collect();
    rows.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(b.0)));
    for ((registry, rules), (count, card, text)) in rows {
        println!("{count:6}  {registry}: {rules}  e.g. [{card}] \"{text}\"");
    }
}
