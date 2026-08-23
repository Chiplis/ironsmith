use super::shard_16::{
    assert_oracle_card_parses_strict, oracle_text_by_name, parse_oracle_card_definition,
};
use super::*;

#[test]
fn refreshed_command_zone_cards_keep_typed_zone_provenance() {
    for name in [
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
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            canonical_compiled_lines(&definition).join("\n"),
            oracle_text_by_name()[name],
            "{name}: {definition:#?}"
        );
    }
}
