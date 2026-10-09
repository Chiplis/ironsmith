//! Source-zone defaults are resolved at ability construction, never at loading.
//! Explicit `Ability::in_zones` restrictions always supersede these defaults.

use crate::{ColorSet, StaticAbility, StaticAbilityId, StaticAbilityPayload, Zone};

pub trait StaticAbilityFunctionalZones {
    fn default_functional_zones(&self) -> Vec<Zone>;
}

/// Shared policy for compiled abilities and native programmatic builders.
/// `source_only` describes the affected object, not an event's origin.
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
        // Commander tax is determined as the commander is cast from the
        // command zone (CR 903.8, 601.2f).
        Some(CommanderTaxLifeSubstitution) => vec![Zone::Command],
        Some(ExileToExileInsteadOfGraveyard | ExileWouldDieInstead) if source_only => vec![
            Zone::Battlefield,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Hand,
            Zone::Library,
            Zone::Exile,
            Zone::Command,
        ],
        Some(SetColors | AddColors | MakeColorless | AddSubtypes | AddAllSubtypesOfFamily) if source_only => all_zones(),
        Some(
            CharacteristicDefiningPT
            | ShuffleIntoLibraryFromGraveyard
            | CountersRemainAcrossZoneChanges
            | SpellManaSpendingRestriction,
        ) => all_zones(),
        _ => vec![Zone::Battlefield],
    }
}

fn all_zones() -> Vec<Zone> {
    vec![Zone::Battlefield, Zone::Hand, Zone::Stack, Zone::Graveyard,
        Zone::Exile, Zone::Library, Zone::Command, Zone::Ante, Zone::OutsideGame]
}

impl<T, E, C, Cond, ICond> StaticAbility<T, E, C, Cond, ICond> {
    /// Subtypes an unconditional source-only ability defines (CR 604.3).
    /// Printed/copied origin is checked separately from this structural fact.
    pub fn characteristic_defining_subtypes(&self) -> Option<&[crate::Subtype]> {
        match &self.payload {
            StaticAbilityPayload::AddSubtypes { filter, subtypes }
                if filter.is_source_only() && !subtypes.is_empty() => Some(subtypes),
            _ => None,
        }
    }

    /// An unconditional source-only definition of every subtype in a family.
    pub fn is_characteristic_defining_subtype_family(&self) -> bool {
        matches!(&self.payload, StaticAbilityPayload::AddAllSubtypesOfFamily { filter, .. }
            if filter.is_source_only())
    }

    /// Printed color CDA contribution, independent of its gameplay color.
    pub fn color_identity_contribution(&self) -> Option<ColorSet> {
        if matches!(&self.payload, StaticAbilityPayload::SetColors {
            exclude_from_color_identity: true, ..
        }) { return None; }
        self.characteristic_defining_colors()
    }

    /// Colors a printed, unconditional source-only ability defines (CR 604.3).
    /// Callers must separately establish that the ability belongs to the
    /// object's rules text; an ordinary grant is not a CDA.
    pub fn characteristic_defining_colors(&self) -> Option<ColorSet> {
        match &self.payload {
            StaticAbilityPayload::SetColors { filter, colors, .. }
            | StaticAbilityPayload::AddColors { filter, colors }
                if filter.is_source_only() => Some(*colors),
            StaticAbilityPayload::MakeColorless(filter) if filter.is_source_only() =>
                Some(ColorSet::COLORLESS),
            _ => None,
        }
    }
}

impl<T, E, C, Cond, ICond> StaticAbilityFunctionalZones for StaticAbility<T, E, C, Cond, ICond> {
    fn default_functional_zones(&self) -> Vec<Zone> {
        // A conditional size assignment is not a characteristic-defining ability.
        if matches!(&self.payload, StaticAbilityPayload::Conditional { .. })
            && self.id == Some(StaticAbilityId::CharacteristicDefiningPT)
        {
            return vec![Zone::Battlefield];
        }
        let source_only = match &self.payload {
            StaticAbilityPayload::ExileToExileInsteadOfGraveyard { filter, .. }
            | StaticAbilityPayload::ExileWouldDieInstead { filter, .. } => filter.source,
            _ => self.characteristic_defining_colors().is_some()
                || self.characteristic_defining_subtypes().is_some()
                || self.is_characteristic_defining_subtype_family(),
        };
        let source_grant_zone = match &self.payload {
            StaticAbilityPayload::Grants(spec) if spec.filter.source => Some(spec.zone),
            _ => None,
        };
        static_ability_zone_defaults(self.id, source_only, source_grant_zone)
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use crate::ObjectFilter;
    type TestStaticAbility = StaticAbility<(), (), (), (), crate::Condition>;

    #[test]
    fn only_unconditional_source_colors_receive_all_zone_defaults() {
        let source = TestStaticAbility::set_colors(ObjectFilter::source(), ColorSet::RED);
        assert_eq!(source.default_functional_zones(), all_zones());
        assert_eq!(source.characteristic_defining_colors(), Some(ColorSet::RED));
        let ordinary = TestStaticAbility::set_colors(ObjectFilter::permanent(), ColorSet::RED);
        assert_eq!(ordinary.default_functional_zones(), vec![Zone::Battlefield]);
        assert_eq!(ordinary.characteristic_defining_colors(), None);
        let conditional = source.with_condition(crate::Condition::YourTurn);
        assert_eq!(conditional.default_functional_zones(), vec![Zone::Battlefield]);
        assert_eq!(conditional.characteristic_defining_colors(), None);
        let restricted = ObjectFilter::source().in_zone(Zone::Battlefield);
        assert!(!restricted.is_source_only());
    }
}

#[cfg(test)]
mod subtype_tests {
    use super::*;
    use crate::{ObjectFilter, Subtype};
    type TestStaticAbility = StaticAbility<(), (), (), (), crate::Condition>;
    #[test]
    fn only_unconditional_source_subtype_additions_have_all_zone_defaults() {
        let source = TestStaticAbility::add_subtypes(ObjectFilter::source(), vec![Subtype::Wizard]);
        assert_eq!(source.default_functional_zones(), all_zones());
        assert_eq!(source.characteristic_defining_subtypes(), Some(&[Subtype::Wizard][..]));
        let ordinary = TestStaticAbility::add_subtypes(ObjectFilter::creature(), vec![Subtype::Wizard]);
        assert_eq!(ordinary.default_functional_zones(), vec![Zone::Battlefield]);
        let conditional = source.with_condition(crate::Condition::YourTurn);
        assert_eq!(conditional.default_functional_zones(), vec![Zone::Battlefield]);
        assert_eq!(conditional.characteristic_defining_subtypes(), None);
        let restricted = TestStaticAbility::add_subtypes(ObjectFilter::source().in_zone(Zone::Battlefield), vec![Subtype::Wizard]);
        assert_eq!(restricted.default_functional_zones(), vec![Zone::Battlefield]);
        let chosen = TestStaticAbility::add_chosen_creature_type(ObjectFilter::source(), "chosen type");
        assert_eq!(chosen.default_functional_zones(), vec![Zone::Battlefield]);
        assert_eq!(chosen.characteristic_defining_subtypes(), None);
    }
}

#[cfg(test)]
mod cda_gap_tests {
    use super::*;
    use crate::{ObjectFilter, SubtypeFamily};
    type TestAbility = StaticAbility<(), (), (), (), crate::Condition>;

    #[test]
    fn identity_exception_preserves_color_definition_and_all_zone_scope() {
        let ability = TestAbility::set_colors_without_color_identity(ObjectFilter::source(), ColorSet::RED);
        assert_eq!(ability.color_identity_contribution(), None);
        assert_eq!(ability.characteristic_defining_colors(), Some(ColorSet::RED));
        assert_eq!(ability.default_functional_zones(), all_zones());
        assert_eq!(TestAbility::set_colors(ObjectFilter::source(), ColorSet::RED)
            .color_identity_contribution(), Some(ColorSet::RED));
    }

    #[test]
    fn subtype_family_cda_scope_excludes_groups_conditions_and_explicit_restrictions() {
        let ability = TestAbility::add_all_subtypes_of_family(ObjectFilter::source(), SubtypeFamily::Creature);
        assert_eq!(ability.default_functional_zones(), all_zones());
        for filter in [ObjectFilter::creature(), ObjectFilter::source().in_zone(Zone::Battlefield)] {
            assert_eq!(TestAbility::add_all_subtypes_of_family(filter, SubtypeFamily::Creature)
                .default_functional_zones(), vec![Zone::Battlefield]);
        }
        assert_eq!(ability.with_condition(crate::Condition::YourTurn).default_functional_zones(),
            vec![Zone::Battlefield]);
    }
}
