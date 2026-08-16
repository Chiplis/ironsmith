#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use crate::compiled_text::canonical_compiled_lines;

#[test]
fn land_entry_chants_keep_the_entering_player_and_effect_backed_payment_linked() {
    for (name, land_type, mana) in [
        ("Thelon's Chant", "Swamp", "{G}"),
        ("Tourach's Chant", "Forest", "{B}"),
    ] {
        let definition = parse_oracle_card_definition(name);
        let expected = format!(
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {mana}.\nWhenever a player puts a {land_type} onto the battlefield, this enchantment deals 3 damage to that player unless they put a -1/-1 counter on a creature they control."
        );
        assert_eq!(
            canonical_compiled_lines(&definition).join("\n"),
            expected,
            "{definition:#?}"
        );

        let debug = format!("{definition:#?}");
        assert!(debug.contains("ZoneChangeTrigger"), "{debug}");
        assert!(debug.contains("AliasedControllerOf"), "{debug}");
        assert!(debug.contains("UnlessPaysEffect"), "{debug}");
        assert!(debug.contains("PutCountersEffect"), "{debug}");
        assert!(debug.contains("MinusOneMinusOne"), "{debug}");
    }
}
