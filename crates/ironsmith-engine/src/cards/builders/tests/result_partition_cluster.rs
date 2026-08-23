#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn mismatch(name: &str, oracle: &str) -> Option<String> {
    let definition = parse_oracle_card_definition(name);
    let compiled = canonical_compiled_lines(&definition).join("\n");
    (compiled != oracle)
        .then(|| format!("{name}:\n  compiled: {compiled:?}\n  oracle:   {oracle:?}"))
}

#[test]
fn result_partition_cluster_renders_exactly() {
    let cases = [
        (
            "Death or Glory",
            "Separate all creature cards in your graveyard into two piles. Exile the pile of an opponent's choice and return the other to the battlefield.",
        ),
        (
            "Shape Anew",
            "The controller of target artifact sacrifices it, then reveals cards from the top of their library until they reveal an artifact card. That player puts that card onto the battlefield, then shuffles all other cards revealed this way into their library.",
        ),
        (
            "Fact or Fiction",
            "Reveal the top five cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard.",
        ),
        (
            "Sphinx of Uthuun",
            "Flying\nWhen this creature enters, reveal the top five cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard.",
        ),
        (
            "Unesh, Criosphinx Sovereign",
            "Flying\nSphinx spells you cast cost {2} less to cast.\nWhenever Unesh or another Sphinx you control enters, reveal the top four cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard.",
        ),
        (
            "Sphinx of Clear Skies",
            "Flying, ward {2}\nDomain — Whenever this creature deals combat damage to a player, reveal the top X cards of your library, where X is the number of basic land types among lands you control. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard.",
        ),
        (
            "Scrap Mastery",
            "Each player exiles all artifact cards from their graveyard, then sacrifices all artifacts they control, then puts all cards they exiled this way onto the battlefield.",
        ),
        (
            "Living Death",
            "Each player exiles all creature cards from their graveyard, then sacrifices all creatures they control, then puts all cards they exiled this way onto the battlefield.",
        ),
        (
            "Living End",
            "Suspend 3—{2}{B}{B}\nEach player exiles all creature cards from their graveyard, then sacrifices all creatures they control, then puts all cards they exiled this way onto the battlefield.",
        ),
        (
            "Strongbox Raider",
            "Raid — When this creature enters, if you attacked this turn, exile the top two cards of your library. Choose one of them. Until the end of your next turn, you may play that card.",
        ),
        (
            "Riverwheel Sweep",
            "Tap target creature. Put three stun counters on it.\nExile the top two cards of your library. Choose one of them. Until the end of your next turn, you may play that card.",
        ),
        (
            "Mishra's Research Desk",
            "{1}, {T}, Sacrifice this artifact: Exile the top two cards of your library. Choose one of them. Until the end of your next turn, you may play that card.\nUnearth {1}{R}.",
        ),
    ];

    let failures = cases
        .into_iter()
        .filter_map(|(name, oracle)| mismatch(name, oracle))
        .collect::<Vec<_>>();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
