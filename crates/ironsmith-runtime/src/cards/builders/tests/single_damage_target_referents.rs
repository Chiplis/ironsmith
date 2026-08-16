#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn single_damage_target_referents_render_exactly() {
    let cases = [
        (
            "Stun Sniper",
            "{1}, {T}: This creature deals 1 damage to target creature. Tap that creature.",
        ),
        (
            "Dinosaur Hunter",
            "Whenever this creature deals damage to a Dinosaur, destroy that creature.",
        ),
        (
            "Vampire Slayer",
            "Whenever this creature deals damage to a Vampire, destroy that creature.",
        ),
        (
            "Puncture Bolt",
            "Puncture Bolt deals 1 damage to target creature. Put a -1/-1 counter on that creature.",
        ),
        (
            "East-Mark Cavalier",
            "Vigilance\nWhenever this creature deals damage to a Goblin or Orc, destroy that creature.",
        ),
        (
            "Kashi-Tribe Warriors",
            "Whenever this creature deals combat damage to a creature, tap that creature and it doesn't untap during its controller's next untap step.",
        ),
        (
            "Orochi Ranger",
            "Whenever this creature deals combat damage to a creature, tap that creature and it doesn't untap during its controller's next untap step.",
        ),
        (
            "Lowland Basilisk",
            "Whenever this creature deals damage to a creature, destroy that creature at end of combat.",
        ),
        (
            "Plague Fiend",
            "Whenever this creature deals combat damage to a creature, destroy that creature unless its controller pays {2}.",
        ),
        (
            "Creepy Doll",
            "Indestructible\nWhenever this creature deals combat damage to a creature, flip a coin. If you win the flip, destroy that creature.",
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
