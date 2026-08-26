#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn nested_grant_by_spec(
    effect: &crate::effect::Effect,
) -> Option<&crate::effects::GrantBySpecEffect> {
    if let Some(grant) = effect.downcast_ref::<crate::effects::GrantBySpecEffect>() {
        return Some(grant);
    }
    effect
        .downcast_ref::<crate::effects::SequenceEffect>()?
        .effects
        .iter()
        .find_map(nested_grant_by_spec)
}

#[test]
fn the_grim_captains_locker_keeps_the_temporary_fixed_escape_grant() {
    let definition = parse_oracle_card_definition("The Grim Captain's Locker");
    let grant = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .flat_map(|activated| &activated.effects.segments)
        .flat_map(|segment| &segment.default_effects)
        .find_map(nested_grant_by_spec)
        .expect("the second activated ability should grant a typed escape method");

    assert_eq!(grant.duration, crate::grant::GrantDuration::UntilEndOfTurn);
    assert_eq!(grant.spec.zone, Zone::Graveyard);
    assert_eq!(grant.spec.beneficiary, PlayerFilter::You);
    assert_eq!(grant.player, PlayerFilter::You);
    assert_eq!(grant.spec.filter.owner, Some(PlayerFilter::You));
    assert_eq!(grant.spec.filter.card_types, [CardType::Creature]);
    assert!(matches!(
        &grant.spec.grantable,
        crate::grant::Grantable::AlternativeCast(
            crate::alternative_cast::AlternativeCastingMethod::Escape {
                cost: Some(cost),
                exile_count: 4,
                ..
            }
        ) if cost.to_oracle() == "{3}{B}"
    ));
}

#[test]
fn the_grim_captains_locker_renders_the_public_card_exactly() {
    let definition = parse_oracle_card_definition("The Grim Captain's Locker");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "{T}: Surveil 1.",
            "{T}: Until end of turn, each creature card in your graveyard gains \"Escape—{3}{B}, Exile four other cards from your graveyard.\"",
        ]
    );
}
