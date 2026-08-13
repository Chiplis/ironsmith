use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

#[test]
fn root_refreshed_regression_gate_reports_every_remaining_mismatch() {
    let names = [
        "Reenact the Crime",
        "Roving Actuator",
        "Flawless Forgery",
        "Shiko, Paragon of the Way",
        "Demilich",
        "Founding the Third Path",
        "Pugnacious Hammerskull",
        "Brazen Blademaster",
        "Burning Sun Cavalry",
        "The Chief Warg",
        "Preacher of the Schism",
        "Ancestral Communion",
        "Skull Storm",
        "Arahbo, Roar of the World",
        "Liesa, Shroud of Dusk",
        "Henzie \"Toolbox\" Torre",
        "Hatut Zeraze Strike Force",
        "Empyrial Storm",
        "Genesis Storm",
        "Echo Storm",
        "Sidar Jabari of Zhalfir",
        "Inalla, Archmage Ritualist",
        "Command Beacon",
        "Fury Storm",
        "Edgar Markov",
        "Netherborn Altar",
    ];
    let oracle = oracle_text_by_name();
    let mut failures = Vec::new();
    for name in names {
        let definition = parse_oracle_card_definition(name);
        let actual = canonical_compiled_lines(&definition).join("\n");
        let expected = &oracle[name];
        if &actual != expected {
            failures.push(format!(
                "{name}\n  compiled: {actual:?}\n  oracle:   {expected:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "remaining refreshed mismatches:\n{}",
        failures.join("\n\n")
    );
}
