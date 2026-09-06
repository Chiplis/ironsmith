//! Counts the reference keys the corpus's effects use without a symbol bound
//! for them in their scope (item 6 of the parser architecture migration).
//!
//! Every key the grammar mints through `CompilerReferenceTag::bind()` is bound
//! in the line's symbol scope; a key that reaches the effects by any other
//! path (`.key()` without a binding, a raw `TagKey`) resolves to no symbol.
//! The canonical resolver records both; this audit sums the unbound ones over
//! the registry corpus so the remaining producer sites can be found by the
//! keys they leave behind.
//!
//! `--db-path <file>` (default `reports/engine-status.sqlite3`),
//! `--cards <n>` lists the n cards with the most unbound keys,
//! `--keys <n>` lists the n most frequent unbound keys (default 40).

use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Mutex;

use ironsmith::ids::CardId;
use ironsmith_compiler::CardDefinitionBuilder;
use ironsmith_tools::{CardPayload, CardStatusDb, DEFAULT_DB_PATH};
use rayon::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db_path = DEFAULT_DB_PATH.to_string();
    let mut top_cards = 0usize;
    let mut top_keys = 40usize;
    let mut card: Option<String> = None;
    let mut dump = false;
    let mut runtime = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--card" => card = Some(args.next().expect("--card takes a value")),
            "--dump" => dump = true,
            "--runtime" => runtime = true,
            "--db-path" => db_path = args.next().expect("--db-path takes a value"),
            "--cards" => top_cards = args.next().expect("--cards takes a value").parse()?,
            "--keys" => top_keys = args.next().expect("--keys takes a value").parse()?,
            other => panic!("unknown argument {other}"),
        }
    }
    let mut db = CardStatusDb::open(&db_path)?;
    let payloads = db.registry_card_payloads()?;
    if payloads.is_empty() {
        return Err(format!("no registry_card rows found in {db_path}").into());
    }
    if let Some(name) = card {
        let payload = payloads
            .iter()
            .find(|payload| payload.name == name)
            .ok_or_else(|| format!("no card named {name}"))?;
        if runtime {
            explain_runtime_card(payload);
        } else {
            explain_card(payload, dump);
        }
        return Ok(());
    }
    let by_key: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
    let by_card: Mutex<Vec<(String, usize)>> = Mutex::new(Vec::new());
    let totals: Mutex<(usize, usize, usize, usize)> = Mutex::new((0, 0, 0, 0));
    payloads.par_iter().for_each(|payload| {
        let keyed = if runtime {
            runtime_keyed_references(payload)
        } else {
            keyed_references(payload)
        };
        let Some(keyed) = keyed else {
            totals.lock().unwrap().3 += 1;
            return;
        };
        let (bound, unbound): (Vec<_>, Vec<_>) =
            keyed.into_iter().partition(|(_, bound)| *bound);
        {
            let mut totals = totals.lock().unwrap();
            totals.0 += 1;
            totals.1 += bound.len();
            totals.2 += unbound.len();
        }
        if unbound.is_empty() {
            return;
        }
        let mut by_key = by_key.lock().unwrap();
        for (key, _) in &unbound {
            *by_key.entry(key.clone()).or_default() += 1;
        }
        by_card.lock().unwrap().push((payload.name.clone(), unbound.len()));
    });
    let (parsed, bound, unbound, unparsed) = *totals.lock().unwrap();
    println!("cards parsed: {parsed} (not parsed: {unparsed})");
    println!("keyed references: {bound} bound, {unbound} unbound");
    let mut by_card = by_card.into_inner().unwrap();
    by_card.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    println!("cards with unbound keys: {}", by_card.len());
    let mut by_key: Vec<(String, usize)> = by_key.into_inner().unwrap().into_iter().collect();
    by_key.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    for (key, count) in by_key.iter().take(top_keys) {
        println!("{count:>7}  {key}");
    }
    for (card, count) in by_card.iter().take(top_cards) {
        println!("{count:>7}  {card}");
    }
    Ok(())
}

/// Every keyed reference of the card's AST as `(key, bound)`, or `None` when
/// the card does not parse.
fn keyed_references(payload: &CardPayload) -> Option<Vec<(String, bool)>> {
    let parse_name = payload.parse_name.as_deref().unwrap_or(&payload.name);
    let builder = CardDefinitionBuilder::new(CardId::from_raw(1), parse_name);
    let text = payload.parse_input.clone();
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut context =
            ironsmith_compiler::parse_context_for_builder(&builder.card_builder, &text, false);
        let (card, _seed) = builder.split_face();
        ironsmith_compiler::canonical_pipeline::parse_card_ast_with_context(&mut context, card, text)
    }));
    let ast = result.ok()?.ok()?;
    Some(
        ast.reference_resolution
            .keyed
            .iter()
            .map(|keyed| (keyed.key.as_str().to_string(), keyed.symbol.is_some()))
            .collect(),
    )
}

/// Prints one card's symbol scopes, keyed bindings and keyed references.
fn explain_card(payload: &CardPayload, dump: bool) {
    let parse_name = payload.parse_name.as_deref().unwrap_or(&payload.name);
    let builder = CardDefinitionBuilder::new(CardId::from_raw(1), parse_name);
    let text = payload.parse_input.clone();
    println!("{}", text);
    let mut context =
        ironsmith_compiler::parse_context_for_builder(&builder.card_builder, &text, false);
    let (card, _seed) = builder.split_face();
    let ast = match ironsmith_compiler::canonical_pipeline::parse_card_ast_with_context(
        &mut context,
        card,
        text,
    ) {
        Ok(ast) => ast,
        Err(error) => {
            println!("parse failed: {error}");
            return;
        }
    };
    if dump {
        println!("--- items\n{:#?}", ast.items);
    }
    println!("--- scopes");
    for scope in ast.symbols.scopes() {
        println!("{:?} parent={:?} kind={:?}", scope.id, scope.parent, scope.kind);
    }
    println!("--- keyed bindings");
    for binding in ast.symbols.bindings() {
        if let Some(key) = &binding.key {
            println!("{:?} scope={:?} key={} role={:?}", binding.id, binding.scope, key.as_str(), binding.role);
        }
    }
    println!("--- keyed references");
    for keyed in &ast.reference_resolution.keyed {
        println!(
            "{} use_scope={:?} symbol={:?}",
            keyed.key.as_str(),
            keyed.use_scope,
            keyed.symbol
        );
    }
}

/// Every reference key the lowered runtime definition carries as
/// `(key, bound)`, bound meaning the lowered document's symbol table has a
/// binding for the key; `None` when the card does not compile.
fn runtime_keyed_references(payload: &CardPayload) -> Option<Vec<(String, bool)>> {
    let parse_name = payload.parse_name.as_deref().unwrap_or(&payload.name);
    let builder = CardDefinitionBuilder::new(CardId::from_raw(1), parse_name);
    let text = payload.parse_input.clone();
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut context =
            ironsmith_compiler::parse_context_for_builder(&builder.card_builder, &text, false);
        ironsmith_compiler::compiler_pipeline::parse_text_with_annotations_lowered_with_facts_context(
            &mut context,
            builder,
            text,
        )
    }));
    let lowered = result.ok()?.ok()?;
    if std::env::var_os("RUNTIME_DUMP").is_some() {
        println!("--- definition\n{:#?}", lowered.definition);
    }
    let bound_keys: std::collections::HashSet<String> = lowered
        .symbols
        .bindings()
        .iter()
        .filter_map(|binding| binding.key.as_ref().map(|key| key.as_str().to_string()))
        .collect();
    let mut seen = std::collections::HashSet::new();
    Some(
        ironsmith_tools::serde_tag_keys::tag_keys_of_serializable(&lowered.definition)
            .into_iter()
            .filter(|key| seen.insert(key.clone()))
            .map(|key| {
                let bound = bound_keys.contains(key.as_str());
                (key, bound)
            })
            .collect(),
    )
}

/// Prints one card's runtime keys with their binding state.
fn explain_runtime_card(payload: &CardPayload) {
    println!("{}", payload.parse_input);
    match runtime_keyed_references(payload) {
        None => println!("card does not compile"),
        Some(keyed) => {
            println!("--- runtime keys");
            for (key, bound) in keyed {
                println!("{key} bound={bound}");
            }
        }
    }
}
