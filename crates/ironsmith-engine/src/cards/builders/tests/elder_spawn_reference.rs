#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn elder_spawn_keeps_the_sacrificed_source_as_the_damage_source() {
    let oracle = "At the beginning of your upkeep, unless you sacrifice an Island, sacrifice this creature and it deals 6 damage to you.\nThis creature can't be blocked by red creatures.";
    let definition = parse_oracle_card_definition("Elder Spawn");
    let compiled = compiled_text_lines(&definition).join("\n");

    assert_eq!(compiled, oracle);

    let debug = format!("{definition:#?}");
    let compact_debug = debug.split_whitespace().collect::<String>();
    let sacrifice_uses_source = compact_debug.contains("SacrificeTargetEffect{target:Source")
        || compact_debug.contains("SacrificeTargetEffect{target:SurfaceHinted{spec:Source");
    let damage_uses_source = compact_debug.contains("ExecuteWithSourceEffect{source:Source")
        || compact_debug.contains("ExecuteWithSourceEffect{source:SurfaceHinted{spec:Source");
    assert!(
        compact_debug.contains("UnlessPaysEffect")
            && sacrifice_uses_source
            && damage_uses_source
            && compact_debug.contains("amount:Fixed(6"),
        "the unpaid upkeep branch must sacrifice Elder Spawn and use that same object as the damage source: {debug}"
    );
    assert!(
        !compact_debug
            .contains("ExecuteWithSourceEffect{source:Object(ObjectFilter{zone:Some(Hand"),
        "the source pronoun must not widen to a card in the controller's hand: {debug}"
    );
}
