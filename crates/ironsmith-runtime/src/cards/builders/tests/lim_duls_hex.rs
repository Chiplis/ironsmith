#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn lim_duls_hex_compiles_to_its_exact_oracle_text() {
    let definition = parse_oracle_card_definition("Lim-Dûl's Hex");

    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "At the beginning of your upkeep, for each player, this enchantment deals 1 damage to that player unless they pay {B} or {3}.".to_string(),
        ]
    );
}

#[test]
fn lim_duls_hex_keeps_one_payment_choice_per_iterated_player() {
    let definition = parse_oracle_card_definition("Lim-Dûl's Hex");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Lim-Dûl's Hex should have an upkeep trigger");
    let [outer] = triggered.effects.segments[0].default_effects.as_slice() else {
        panic!("expected one outer effect, got {:#?}", triggered.effects);
    };
    let for_players = outer
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("Lim-Dûl's Hex should iterate players before offering payment");
    let [per_player] = for_players.effects.as_slice() else {
        panic!("expected one effect per player, got {for_players:#?}");
    };
    let unless_pays = per_player
        .downcast_ref::<crate::effects::UnlessPaysEffect>()
        .expect("each player should receive an independent payment choice");

    assert_eq!(for_players.filter, PlayerFilter::Any);
    assert_eq!(unless_pays.player, PlayerFilter::IteratedPlayer);
    assert!(
        matches!(
            unless_pays.cost.kind(),
            ironsmith_core::TotalCostKind::OneOf(branches) if branches.len() == 2
        ),
        "expected the {{B}}/{{3}} alternatives, got {:#?}",
        unless_pays.cost
    );
}
