#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn balls_of_fire_preserves_sticker_trigger_and_name_character_count() {
    let definition = parse_oracle_card_definition("_____ Balls of Fire");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "When this enchantment enters, you may put a name sticker on it.",
            "Whenever you put a sticker on this enchantment, it deals damage equal to the number of o's in name stickers on this enchantment to any target.",
        ],
        "{definition:#?}"
    );

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("NameStickerCharacterCountOnSource") && debug.contains("character: 'o'"),
        "{debug}"
    );
    assert!(
        debug.contains("KeywordActionTrigger") && debug.contains("action: Sticker"),
        "{debug}"
    );
}
