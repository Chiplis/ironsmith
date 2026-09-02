//! Counts the spans recognition hands to the sentence rule more than once.
//!
//! Composition is fine: a multi-sentence recognizer parses each sentence once.
//! Redundancy is not: probing a span and parsing it again later, or two sibling
//! recognizers each parsing the same tail. The grammar's tracker records both
//! callers for every repeat, so the report groups by call-site pair rather than
//! by card.
//!
//! `--every N` samples every Nth card (default 20); `--all` runs the corpus.
//! `--fail-on-findings` exits non-zero when any repeat is found.

use ironsmith_tools::{
    compile_strict_snapshot_from_payload, default_cards_path, load_canonical_cards,
};
use std::collections::BTreeMap;

fn main() {
    let mut every = 20usize;
    let mut fail_on_findings = false;
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
            "--fail-on-findings" => fail_on_findings = true,
            "--cards" => cards_path = iter.next().expect("--cards requires a path"),
            "--name" => only_name = Some(iter.next().expect("--name requires a card name")),
            other => panic!("unknown argument {other}"),
        }
    }

    let cards = load_canonical_cards(&cards_path)
        .unwrap_or_else(|error| panic!("failed to load cards from {cards_path}: {error}"));

    let mut sampled = 0usize;
    let mut total_calls = 0usize;
    let mut total_hits = 0usize;
    let mut total_repeats = 0usize;
    let mut reentrant: BTreeMap<(String, String), (usize, String, String)> = BTreeMap::new();
    let mut cards_with_repeats = 0usize;
    let mut by_pair: BTreeMap<(String, String), (usize, String, String)> = BTreeMap::new();

    for (index, (name, payload)) in cards.iter().enumerate() {
        if let Some(only) = &only_name {
            if name != only {
                continue;
            }
        } else if index % every != 0 {
            continue;
        }
        sampled += 1;
        ironsmith_compiler::parse_ledger::begin();
        let snapshot = compile_strict_snapshot_from_payload(payload);
        let report = ironsmith_compiler::parse_ledger::end();
        if only_name.is_some() {
            println!("parse_input:\n{}", payload.parse_input);
            println!(
                "compiled_text:\n{}",
                snapshot.compiled_text.as_deref().unwrap_or("<none>")
            );
        }
        total_calls += report.parses;
        total_hits += report.hits;
        if !report.repeats.is_empty() {
            cards_with_repeats += 1;
            if report.resets > 1 {
                println!(
                    "note: [{name}] recognized {} times (memo reset {} times); its {} repeats are one parse per pass",
                    report.resets,
                    report.resets,
                    report.repeats.len()
                );
            }
        }
        total_repeats += report.repeats.len();
        for call in report.reentrant {
            let entry = reentrant
                .entry((call.first_caller.clone(), call.repeat_caller.clone()))
                .or_insert_with(|| (0, name.clone(), call.text.clone()));
            entry.0 += 1;
        }
        for repeat in report.repeats {
            let entry = by_pair
                .entry((repeat.first_caller.clone(), repeat.repeat_caller.clone()))
                .or_insert_with(|| (0, name.clone(), repeat.text.clone()));
            entry.0 += 1;
        }
    }

    println!("redundant sentence parses audit");
    println!("cards sampled: {sampled} (every {every})");
    println!("sentence-rule parses: {total_calls}");
    println!("sentence-rule memo hits: {total_hits}");
    println!("redundant parses: {total_repeats}");
    println!("cards with redundant parses: {cards_with_repeats}");
    println!(
        "re-entrant calls (guarded recursion, not repeats): {}",
        reentrant.values().map(|v| v.0).sum::<usize>()
    );
    for ((first, again), (count, card, text)) in &reentrant {
        println!("{count:6}  reentrant outer {first}  inner {again}  e.g. [{card}] \"{text}\"");
    }
    println!("call-site pairs: {}", by_pair.len());
    let mut pairs: Vec<_> = by_pair.into_iter().collect();
    pairs.sort_by(|left, right| right.1.0.cmp(&left.1.0).then_with(|| left.0.cmp(&right.0)));
    for ((first, repeat), (count, card, text)) in pairs {
        println!("{count:6}  first {first}  again {repeat}  e.g. [{card}] \"{text}\"");
    }

    if fail_on_findings && total_repeats > 0 {
        std::process::exit(1);
    }
}
