#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::types::{CardType, Subtype};

#[test]
fn shelob_keeps_filtered_damager_history_and_food_copy_exception() {
    let definition = parse_oracle_card_definition("Shelob, Child of Ungoliant");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Deathtouch, ward {2}",
            "Other Spiders you control have deathtouch and ward {2}.",
            "Whenever another creature dealt damage this turn by a Spider you controlled dies, create a token that's a copy of that creature, except it's a Food artifact with \"{2}, {T}, Sacrifice this token: You gain 3 life,\" and it loses all other card types.",
        ]
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::DiesDamagedByFilteredSourceThisTurnTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Shelob should use the filtered-damager history trigger");
    let matcher = triggered
        .trigger
        .downcast_ref::<crate::triggers::DiesDamagedByFilteredSourceThisTurnTrigger>()
        .expect("the trigger matcher should remain typed");
    assert!(matcher.victim.other);
    assert_eq!(matcher.victim.card_types, [CardType::Creature]);
    assert_eq!(matcher.victim.controller, None);
    assert!(matcher.victim.subtypes.is_empty());
    assert_eq!(matcher.damager_filter.subtypes, [Subtype::Spider]);
    assert_eq!(matcher.damager_filter.controller, Some(PlayerFilter::You));

    let copy = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<CreateTokenCopyEffect>())
        .expect("Shelob should create a typed token copy");
    assert_eq!(
        copy.set_card_types.as_deref(),
        Some([CardType::Artifact].as_slice())
    );
    assert_eq!(
        copy.set_subtypes.as_deref(),
        Some([Subtype::Food].as_slice())
    );
    let [granted] = copy.granted_static_abilities.as_slice() else {
        panic!("the Food copy should carry exactly one activated ability: {copy:#?}");
    };
    let model = granted
        .compiled_model()
        .expect("the granted token ability should retain its typed model");
    let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) = &model.payload
    else {
        panic!("expected source-bound granted ability, got {model:#?}");
    };
    assert!(matches!(
        grant.ability.kind,
        ironsmith_core::AbilityKind::Activated(_)
    ));
}
