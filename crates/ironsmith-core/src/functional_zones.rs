//! Source-zone defaults are resolved at ability construction, never at loading.
//! Explicit `Ability::in_zones` restrictions always supersede these defaults.

use crate::{StaticAbility, StaticAbilityId, StaticAbilityPayload, Zone};

pub trait StaticAbilityFunctionalZones {
    fn default_functional_zones(&self) -> Vec<Zone>;
}

/// Shared policy for compiled abilities and native programmatic builders.
/// `source_only` describes the object being replaced, not the event's origin.
pub fn static_ability_zone_defaults(
    id: Option<StaticAbilityId>,
    source_only: bool,
    source_grant_zone: Option<Zone>,
) -> Vec<Zone> {
    use StaticAbilityId::*;
    if id == Some(Grants)
        && let Some(zone) = source_grant_zone
    {
        return vec![zone];
    }
    match id {
        Some(Dredge) => vec![Zone::Graveyard],
        Some(ExileToExileInsteadOfGraveyard | ExileWouldDieInstead) if source_only => vec![
            Zone::Battlefield,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Hand,
            Zone::Library,
            Zone::Exile,
            Zone::Command,
        ],
        Some(ShuffleIntoLibraryFromGraveyard | CountersRemainAcrossZoneChanges) => vec![
            Zone::Battlefield,
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
            Zone::Ante,
            Zone::OutsideGame,
        ],
        _ => vec![Zone::Battlefield],
    }
}

impl<T, E, C, Cond, ICond> StaticAbilityFunctionalZones for StaticAbility<T, E, C, Cond, ICond> {
    fn default_functional_zones(&self) -> Vec<Zone> {
        let source_only = match &self.payload {
            StaticAbilityPayload::ExileToExileInsteadOfGraveyard { filter, .. }
            | StaticAbilityPayload::ExileWouldDieInstead { filter, .. } => filter.source,
            _ => false,
        };
        let source_grant_zone = match &self.payload {
            StaticAbilityPayload::Grants(spec) if spec.filter.source => Some(spec.zone),
            _ => None,
        };
        static_ability_zone_defaults(self.id, source_only, source_grant_zone)
    }
}
