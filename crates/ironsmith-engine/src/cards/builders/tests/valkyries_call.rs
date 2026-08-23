use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

#[test]
fn valkyries_call_preserves_conjunctive_death_filter_and_return_followup() {
    assert_oracle_card_parses_strict("Valkyrie's Call");
    let definition = parse_oracle_card_definition("Valkyrie's Call");

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Whenever a nontoken non-angel creature you control dies, return that card to the battlefield under its owner's control with a +1/+1 counter on it. It has flying and is an Angel in addition to its other types."
    );

    let filter = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
                .map(|trigger| &trigger.object_filter),
            _ => None,
        })
        .expect("Valkyrie's Call should compile to a zone-change trigger");

    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.excluded_subtypes, vec![Subtype::Angel]);
    assert!(filter.nontoken);
    assert!(
        filter.any_of.is_empty(),
        "comma-separated modifiers must remain conjunctive, got {filter:?}"
    );
}
