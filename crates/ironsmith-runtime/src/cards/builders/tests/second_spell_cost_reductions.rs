#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn second_spell_cost_reduction_cards_keep_exact_ordinal() {
    for (name, oracle) in [
        (
            "Uthros Psionicist",
            "The second spell you cast each turn costs {2} less to cast.",
        ),
        (
            "Highspire Bell-Ringer",
            "Flying\nThe second spell you cast each turn costs {1} less to cast.",
        ),
        (
            "Raging Battle Mouse",
            "The second spell you cast each turn costs {1} less to cast.\nCelebration — At the beginning of combat on your turn, if two or more nonland permanents entered the battlefield under your control this turn, target creature you control gets +1/+1 until end of turn.",
        ),
        (
            "Alisaie Leveilleur",
            "Partner with Alphinaud Leveilleur\nFirst strike\nDualcast — The second spell you cast each turn costs {2} less to cast.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let compiled = canonical_compiled_lines(&definition).join("\n");
        assert_eq!(compiled, oracle, "{name}: {definition:#?}");
    }
}
