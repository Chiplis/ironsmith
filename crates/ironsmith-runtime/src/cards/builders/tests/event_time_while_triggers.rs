use super::shard_16::{
    assert_oracle_card_parses_strict, oracle_text_by_name, parse_oracle_card_definition,
};
use super::*;

#[test]
fn event_time_while_triggers_round_trip_to_oracle() {
    for name in [
        "Pugnacious Hammerskull",
        "Brazen Blademaster",
        "Burning Sun Cavalry",
        "The Chief Warg",
        "Preacher of the Schism",
        "Ancestral Communion",
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
