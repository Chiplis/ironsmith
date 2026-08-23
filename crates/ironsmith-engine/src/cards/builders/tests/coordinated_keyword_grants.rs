#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn coordinated_keyword_grant_cards_render_exactly() {
    let cases = [
        (
            "Midnight Mayhem",
            "Create three 1/1 red Gremlin creature tokens. Gremlins you control gain menace, lifelink, and haste until end of turn.",
        ),
        (
            "Case of the Shattered Pact",
            "When this Case enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle.\nTo solve — There are five colors among permanents you control.\nSolved — At the beginning of combat on your turn, target creature you control gains flying, double strike, and vigilance until end of turn.",
        ),
        (
            "Cosmic Spider-Man",
            "Flying, first strike, trample, lifelink, haste\nAt the beginning of combat on your turn, other Spiders you control gain flying, first strike, trample, lifelink, and haste until end of turn.",
        ),
    ];

    let failures = cases
        .into_iter()
        .filter_map(|(name, oracle)| {
            let definition = parse_oracle_card_definition(name);
            let compiled = canonical_compiled_lines(&definition).join("\n");
            (compiled != oracle)
                .then(|| format!("{name}:\n  compiled: {compiled:?}\n  oracle:   {oracle:?}"))
        })
        .collect::<Vec<_>>();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
