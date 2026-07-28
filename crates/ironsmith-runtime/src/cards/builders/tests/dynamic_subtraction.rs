use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn compiled_card_text(name: &str) -> String {
    assert_oracle_card_parses_strict(name);
    canonical_compiled_lines(&parse_oracle_card_definition(name)).join("\n")
}

#[test]
fn explicit_dynamic_count_subtraction_survives_card_compilation() {
    let superior = compiled_card_text("Superior Numbers");
    assert!(
        superior.contains(
            "equal to the number of creatures you control in excess of the number of creatures target opponent controls"
        ),
        "{superior}"
    );

    let suspicions = compiled_card_text("Dark Suspicions");
    assert!(
        suspicions.contains(
            "where X is the number of cards in that player's hand minus the number of cards in your hand"
        ),
        "{suspicions}"
    );

    let bulwark = compiled_card_text("Bulwark");
    assert!(
        bulwark.contains(
            "where X is the number of cards in your hand minus the number of cards in that player's hand"
        ),
        "{bulwark}"
    );
}
