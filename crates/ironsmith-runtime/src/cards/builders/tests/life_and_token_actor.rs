#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn life_change_and_token_cards_render_the_shared_actor_once() {
    let cases = [
        (
            "Bitterblossom",
            "At the beginning of your upkeep, you lose 1 life and create a 1/1 black Faerie Rogue creature token with flying.",
        ),
        (
            "Lotho, Corrupt Shirriff",
            "Whenever a player casts their second spell each turn, you lose 1 life and create a Treasure token.",
        ),
        (
            "Sporeweb Weaver",
            "Reach, hexproof from blue\nWhenever this creature is dealt damage, you gain 1 life and create a 1/1 green Saproling creature token.",
        ),
        (
            "Life Insurance",
            "Extort\nWhenever a nontoken creature dies, you lose 1 life and create a Treasure token.",
        ),
        (
            "Static Net",
            "When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield.\nWhen this enchantment enters, you gain 2 life and create a tapped Powerstone token.",
        ),
        (
            "Kamber, the Plunderer",
            "Partner with Laurine, the Diversion\nLifelink\nWhenever a creature an opponent controls dies, you gain 1 life and create a Blood token.",
        ),
        (
            "Bitterbloom Bearer",
            "Flash\nFlying\nAt the beginning of your upkeep, you lose 1 life and create a 1/1 blue and black Faerie creature token with flying.",
        ),
        (
            "Biotransference",
            "Creatures you control are artifacts in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.\nWhenever you cast an artifact spell, you lose 1 life and create a 2/2 black Necron Warrior artifact creature token.",
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
