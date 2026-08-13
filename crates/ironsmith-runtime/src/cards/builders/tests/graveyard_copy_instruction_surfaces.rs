use super::shard_16::{
    assert_oracle_card_parses_strict, oracle_text_by_name, parse_oracle_card_definition,
};
use super::*;

#[test]
fn authored_graveyard_copy_instruction_boundaries_round_trip() {
    for name in [
        "Reenact the Crime",
        "Roving Actuator",
        "Flawless Forgery",
        "Shiko, Paragon of the Way",
        "Demilich",
        "Founding the Third Path",
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            canonical_compiled_lines(&definition).join("\n"),
            oracle_text_by_name()[name],
            "{name}"
        );
    }
}
