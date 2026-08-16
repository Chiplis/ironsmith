#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn single_counter_target_referents_render_exactly() {
    let cases = [
        (
            "Stony Strength",
            "Put a +1/+1 counter on target creature you control. Untap that creature.",
        ),
        (
            "Vedalken Anatomist",
            "{2}{U}, {T}: Put a -1/-1 counter on target creature. You may tap or untap that creature.",
        ),
        (
            "Hunt the Weak",
            "Put a +1/+1 counter on target creature you control. Then that creature fights target creature you don't control.",
        ),
        (
            "Growth Curve",
            "Put a +1/+1 counter on target creature you control, then double the number of +1/+1 counters on that creature.",
        ),
        (
            "Invigorating Surge",
            "Put a +1/+1 counter on target creature you control, then double the number of +1/+1 counters on that creature.",
        ),
        (
            "Invasion of Muraganda",
            "When this Siege enters, put a +1/+1 counter on target creature you control. Then that creature fights up to one target creature you don't control.",
        ),
        (
            "Savage Stomp",
            "This spell costs {2} less to cast if it targets a Dinosaur you control.\nPut a +1/+1 counter on target creature you control. Then that creature fights target creature you don't control.",
        ),
        (
            "Struggle for Skemfar",
            "Put a +1/+1 counter on target creature you control. Then that creature fights up to one target creature you don't control.\nForetell {G}",
        ),
        (
            "Hunger of the Howlpack",
            "Put a +1/+1 counter on target creature.\nMorbid — Put three +1/+1 counters on that creature instead if a creature died this turn.",
        ),
        (
            "Might Beyond Reason",
            "Put two +1/+1 counters on target creature.\nDelirium — Put three +1/+1 counters on that creature instead if there are four or more card types among cards in your graveyard.",
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
