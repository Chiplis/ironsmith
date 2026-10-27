//! Strict, reminder-only batch audit. Reads JSONL requests and emits JSONL results.
//! No keyword registry or allow-unsupported fallback is used.
use ironsmith_compiler::{CardDefinitionBuilder, parse_card_text};
use ironsmith_core::ids::CardId;
use serde_json::{Value, json};
use std::io::{self, BufRead};

fn compile(name: &str, text: &str) -> Value {
    let (result, loss) = ironsmith_compiler_api::parse_loss::capture(|| {
        std::panic::catch_unwind(|| {
            parse_card_text(CardDefinitionBuilder::new(CardId(1), name), text)
        })
    });
    match result {
        Ok(Ok(definition)) => {
            let canonical = definition.canonical_text.clone();
            let behavior = format!(
                "{:?}",
                (
                    &definition.abilities,
                    &definition.spell_effect,
                    &definition.aura_attach_filter,
                    &definition.alternative_casts,
                    definition.has_fuse,
                    &definition.optional_costs,
                    &definition.additional_cost,
                    definition.refers_to_ante
                )
            );
            json!({"status":"compiled", "canonical_text":canonical,"behavior":behavior,"loss_count":loss.count(),"loss_diagnostics":loss.reasons_text()})
        }
        Ok(Err(error)) => json!({"status":"rejected", "error":error.to_string()}),
        Err(_) => json!({"status":"panic"}),
    }
}
fn run() {
    for line in io::stdin().lock().lines() {
        let input: Value =
            serde_json::from_str(&line.expect("read request")).expect("JSON request");
        let name = input["name"].as_str().unwrap();
        let metadata = input["metadata"].as_str().unwrap();
        let result = compile(
            name,
            &format!("{}\n{}", metadata, input["text"].as_str().unwrap()),
        );
        let baseline = compile(
            name,
            &format!("{}\n{}", metadata, input["baseline"].as_str().unwrap()),
        );
        let empty = compile(name, metadata);
        let same = result["status"] == "compiled"
            && baseline["status"] == "compiled"
            && result["behavior"] == baseline["behavior"];
        let noop = result["status"] == "compiled" && result["behavior"] == empty["behavior"];
        println!(
            "{}",
            json!({"id":input["id"],"reminder":result,"baseline":baseline,"same_structure":same,"no_op":noop})
        );
    }
}

fn main() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}
