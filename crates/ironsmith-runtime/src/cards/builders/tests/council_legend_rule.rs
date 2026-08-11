#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn council_keeps_a_creature_filtered_controller_legend_rule_exemption() {
    let definition = parse_oracle_card_definition("Council of Reeds");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "The \"legend rule\" doesn't apply to creatures you control.",
            "At the beginning of combat on your turn, if you've cast a noncreature spell this turn, create a token that's a copy of Council of Reeds.",
        ]
    );

    let model = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.compiled_model(),
            _ => None,
        })
        .expect("Council should retain a typed static model");
    let ironsmith_core::StaticAbilityPayload::LegendRuleDoesntApplyToController { filter } =
        &model.payload
    else {
        panic!("expected filtered legend-rule payload: {model:#?}");
    };
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert!(!filter.token);
}
