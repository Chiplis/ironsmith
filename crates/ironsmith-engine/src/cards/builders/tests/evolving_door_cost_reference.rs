#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "{1}, {T}, Sacrifice a creature: Count the colors of the sacrificed creature, then search your library for a creature card that's exactly that many colors plus one. Exile that card, then shuffle. You may cast the exiled card. Activate only as a sorcery.";

#[test]
fn evolving_door_search_count_uses_the_sacrificed_cost_object() {
    let definition = parse_oracle_card_definition("Evolving Door");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [ORACLE],
        "{definition:#?}"
    );

    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Evolving Door should have an activated ability");

    let sacrifice_tag = activated
        .mana_cost
        .costs()
        .iter()
        .filter_map(|cost| cost.effect_ref())
        .find_map(|effect| effect.downcast_ref::<crate::effects::TaggedEffect>())
        .filter(|tagged| {
            tagged
                .effect
                .downcast_ref::<crate::effects::SacrificeEffect>()
                .is_some()
        })
        .map(|tagged| tagged.tag.clone())
        .expect("the sacrifice cost should export its chosen object snapshot");
    assert!(sacrifice_tag.as_str().starts_with("sacrifice_cost_"));

    let search = activated
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
        .filter(|choose| choose.is_search)
        .expect("the ability should retain its library search");
    let Some(crate::filter::Comparison::EqualExpr(value)) = &search.filter.color_count else {
        panic!("the search must retain its exact dynamic color count: {search:#?}");
    };
    let Value::Add(left, right) = value.as_ref() else {
        panic!("the search count must be the paid creature's colors plus one: {value:#?}");
    };
    assert_eq!(right.unhinted(), &Value::Fixed(1));
    let Value::ColorsAmong(sacrificed) = left.unhinted() else {
        panic!("the search count must inspect the sacrificed object: {left:#?}");
    };
    assert!(sacrificed.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag == sacrifice_tag
    }));
}
