#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn plural_result_cards_keep_the_authored_they_subject() {
    let mut failures = Vec::new();
    for name in [
        "Relive the Past",
        "Rhino, Terrible Trampler",
        "Tezzeret, Cruel Machinist",
        "Furygale Flocking",
        "Cybership",
    ] {
        let definition = parse_oracle_card_definition(name);
        let rendered = canonical_compiled_lines(&definition).join("\n");
        let normalized = rendered.to_ascii_lowercase();
        if !normalized.contains("they ") && !normalized.contains("they're ") {
            failures.push(format!("{name} lost its plural result subject: {rendered}"));
        }
        if normalized.contains("it gains ")
            || normalized.contains("it's ")
            || normalized.contains("it becomes ")
        {
            failures.push(format!(
                "{name} collapsed a plural result to singular: {rendered}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
