//! Read-only corpus probe. Run from the repo root, with an output directory.
#[path = "../tests/support/semantic_audit_probes.rs"]
mod probes;
use rayon::prelude::*;
use serde_json::json;
use std::{
    collections::BTreeSet,
    io::{BufWriter, Write},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
fn main() {
    let out = std::env::args().nth(1).expect("output directory");
    let names: Vec<String> = std::fs::read_to_string(format!("{out}/names.txt"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    let loaded = ironsmith_tools::load_card_payloads_by_names("cards.json", &names).unwrap();
    let loaded_names: BTreeSet<_> = loaded.keys().cloned().collect();
    let missing: Vec<_> = names
        .iter()
        .filter(|n| !loaded_names.contains(&ironsmith_tools::normalize_lookup_name(n)))
        .collect();
    std::fs::write(
        format!("{out}/excluded-names.json"),
        serde_json::to_string_pretty(&missing).unwrap(),
    )
    .unwrap();
    let payloads: Vec<_> = loaded.into_values().flatten().collect();
    eprintln!(
        "loaded {} payloads; {} names excluded by loader",
        payloads.len(),
        missing.len()
    );
    let writer = Mutex::new(BufWriter::new(
        std::fs::File::create(format!("{out}/corpus.jsonl")).unwrap(),
    ));
    let done = AtomicUsize::new(0);
    let verify = std::env::var_os("RENDERER_AUDIT_VERIFY").is_some();
    let light = std::env::var_os("RENDERER_AUDIT_LIGHT").is_some();
    rayon::ThreadPoolBuilder::new().num_threads(8).stack_size(64*1024*1024).build().unwrap().install(|| {
        payloads.par_iter().for_each(|p| {
            let row=match ironsmith_tools::compile_runtime_definition_from_payload(p) {
                // RENDERER_AUDIT_LIGHT: rendered + semantic-audit lines only,
                // for corpus regression diffs and audit-marker inventories.
                Ok(def) if verify => {
                    let audit = ironsmith::compiled_text::semantic_audit_lines(&def);
                    let poisoned = ironsmith::compiled_text::semantic_audit_lines(&probes::poison_definition(&def));
                    let mutations = probes::executable_mutations(&def);
                    let hidden: Vec<_> = mutations.iter().filter_map(|(index, kind, changed)| {
                        let after = ironsmith::compiled_text::semantic_audit_lines(changed);
                        (after == audit).then(|| json!({"ability":index,"kind":kind,
                            "model_before":format!("{:?}",def.abilities[*index]),
                            "model_after":format!("{:?}",changed.abilities[*index])}))
                    }).collect();
                    json!({"name":p.name,"oracle":p.oracle_text,
                        "rendered":ironsmith::compiled_text::compiled_text_lines(&def),
                        "audit":audit,"support_issues":ironsmith::cards::generated_definition_unsupported_mechanics_message(&def),
                        "poison_changed":audit!=poisoned,"poison_after":if audit!=poisoned {Some(poisoned)} else {None},
                        "static_abilities":def.abilities.iter().filter(|a| matches!(a.kind, ironsmith::ability::AbilityKind::Static(_))).count(),
                        "mutation_count":mutations.len(),"mutation_kinds":mutations.iter().map(|(_,kind,_)|kind).collect::<Vec<_>>(),
                        "hidden_mutations":hidden})
                },
                Ok(def) if light => json!({"name":p.name,
                    "rendered":ironsmith::compiled_text::compiled_text_lines(&def),
                    "support_issues":ironsmith::cards::generated_definition_unsupported_mechanics_message(&def),
                    "audit":ironsmith::compiled_text::semantic_audit_lines(&def)}),
                Ok(def) => json!({"name":p.name,"oracle":p.oracle_text,"parse_input":p.parse_input,
                    "rendered":ironsmith::compiled_text::compiled_text_lines(&def),
                    "support_issues":ironsmith::cards::generated_definition_unsupported_mechanics_message(&def),
                    "definition":format!("{def:#?}")}),
                Err(error) => json!({"name":p.name,"oracle":p.oracle_text,"error":error}),
            };
            writeln!(writer.lock().unwrap(),"{}",row).unwrap();
            let n=done.fetch_add(1,Ordering::Relaxed)+1;
            if n%500==0 {eprintln!("{n}/{}",payloads.len());}
        });
    });
    writer.into_inner().unwrap().flush().unwrap();
    eprintln!("complete: {}", done.load(Ordering::Relaxed));
}
