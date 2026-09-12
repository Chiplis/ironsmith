//! Compile a newline-delimited card-name cohort and export ability zones.
//! cargo run -p ironsmith-tools --example audit_functional_zones -- cards.json names.txt output.json
use ironsmith::AbilityKind;
use ironsmith_tools::{compile_definition_from_payload, load_card_payloads_by_names};
use serde_json::json;

fn main() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}

fn run() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 4, "expected cards.json names.txt output.json");
    let names: Vec<String> = std::fs::read_to_string(&args[2])
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect();
    let payloads = load_card_payloads_by_names(&args[1], &names).unwrap();
    let mut rows = Vec::new();
    for faces in payloads.values() {
        for payload in faces {
            match compile_definition_from_payload(payload) {
                Ok(definition) => {
                    let abilities: Vec<_> = definition.abilities.iter().map(|ability| {
                        let kind = match ability.kind {
                            AbilityKind::Activated(_) => "activated",
                            AbilityKind::Triggered(_) => "triggered",
                            AbilityKind::Static(_) => "static",
                        };
                        json!({"kind": kind, "zones": ability.functional_zones.iter().map(|zone| format!("{zone:?}")).collect::<Vec<_>>(), "text": ironsmith::compiled_text::ability_surface_text(ability)})
                    }).collect();
                    rows.push(json!({"name": payload.name, "oracle": payload.raw_oracle_text, "abilities": abilities}));
                }
                Err(error) => rows.push(json!({"name": payload.name, "oracle": payload.raw_oracle_text, "error": error})),
            }
        }
    }
    std::fs::write(&args[3], serde_json::to_string_pretty(&rows).unwrap()).unwrap();
    println!("Audited {} card faces", rows.len());
}
