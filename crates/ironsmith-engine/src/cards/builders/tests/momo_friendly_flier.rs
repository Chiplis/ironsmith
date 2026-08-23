#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn momo_keeps_the_first_matching_spell_and_own_turn_restrictions() {
    let definition = parse_oracle_card_definition("Momo, Friendly Flier");
    let lines = canonical_compiled_lines(&definition);
    assert_eq!(
        lines.get(1).map(String::as_str),
        Some(
            "The first non-Lemur creature spell with flying you cast during each of your turns costs {1} less to cast."
        ),
        "{definition:#?}"
    );

    let reduction = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.cost_reduction(),
            _ => None,
        })
        .next()
        .expect("Momo should have a typed cost reduction");
    assert!(
        reduction.filter.first_spell_cast_each_turn,
        "{reduction:#?}"
    );
    assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
    assert_eq!(reduction.filter.card_types, [CardType::Creature]);
    assert_eq!(reduction.filter.excluded_subtypes, [Subtype::Lemur]);
    assert_eq!(reduction.filter.static_abilities, [StaticAbilityId::Flying]);
    assert_eq!(reduction.condition, Some(crate::ConditionExpr::YourTurn));
}
