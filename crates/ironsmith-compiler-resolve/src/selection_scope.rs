//! Zone provenance for object selections, not event or characteristic filters.

use crate::cards::builders::CardTextError;
use crate::filter::{ObjectFilter, TaggedOpbjectRelation};
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// A description containing "card" cannot use the implicit battlefield domain.
/// A zone or an identity-bound collection must establish its candidate pool.
/// Only union branches describe alternative candidates; nested characteristic
/// and event predicates must not be interpreted as independent selections.
pub fn validate_card_selection_filter(
    filter: &ObjectFilter,
    source_zone: Option<Zone>,
) -> Result<(), CardTextError> {
    fn unresolved(filter: &ObjectFilter, scoped: bool, card: bool) -> bool {
        let scoped = scoped
            || filter.zone.is_some()
            || filter.source
            || filter.specific.is_some()
            || filter.is_target_object
            || filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject);
        let card = card || filter.has_explicit_card_noun();
        if filter.any_of.is_empty() {
            return card && !scoped;
        }
        filter
            .any_of
            .iter()
            .any(|branch| unresolved(branch, scoped, card))
    }
    if unresolved(filter, source_zone.is_some(), false) {
        return Err(CardTextError::ParseError(
            "card selection has no resolved source zone or referenced collection; refusing an implicit battlefield selection".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_card_selection_spec(spec: &ChooseSpec) -> Result<(), CardTextError> {
    match spec.base() {
        ChooseSpec::Object(filter)
        | ChooseSpec::All(filter)
        | ChooseSpec::ObjectOrPlayer(filter, _) => validate_card_selection_filter(filter, None),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{PlayerFilter, TaggedObjectConstraint};
    use crate::types::CardType;

    fn card() -> ObjectFilter {
        let mut filter = ObjectFilter::default();
        filter.set_explicit_card_noun(true);
        filter
    }

    #[test]
    fn card_selection_requires_a_zone_or_membership_binding() {
        let mut filter = card();
        filter.owner = Some(PlayerFilter::You);
        assert!(validate_card_selection_filter(&filter, None).is_err());
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: "prior".into(),
            relation: TaggedOpbjectRelation::SameControllerAsTagged,
        });
        assert!(validate_card_selection_filter(&filter, None).is_err());
        filter.tagged_constraints[0].relation = TaggedOpbjectRelation::IsTaggedObject;
        assert!(validate_card_selection_filter(&filter, None).is_ok());
        for zone in [Zone::Hand, Zone::Library, Zone::Graveyard, Zone::Exile] {
            assert!(validate_card_selection_filter(&card(), Some(zone)).is_ok());
            assert!(validate_card_selection_filter(&card().in_zone(zone), None).is_ok());
        }
        assert!(validate_card_selection_filter(&ObjectFilter::creature(), None).is_ok());
    }

    #[test]
    fn every_union_branch_must_resolve_its_card_pool() {
        let mut filter = card();
        filter.any_of = vec![
            ObjectFilter::default().in_zone(Zone::Hand),
            ObjectFilter::default(),
        ];
        assert!(validate_card_selection_filter(&filter, None).is_err());
        filter.any_of[1].zone = Some(Zone::Graveyard);
        assert!(validate_card_selection_filter(&filter, None).is_ok());
        filter.any_of[1] = ObjectFilter::default();
        assert!(validate_card_selection_filter(&filter, Some(Zone::Exile)).is_ok());

        let mut mixed = ObjectFilter::default();
        mixed.any_of = vec![ObjectFilter::creature(), card()];
        assert!(validate_card_selection_filter(&mixed, None).is_err());
        mixed.any_of[1].zone = Some(Zone::Graveyard);
        assert!(validate_card_selection_filter(&mixed, None).is_ok());
    }

    #[test]
    fn count_and_target_wrappers_do_not_hide_unscoped_cards() {
        let spec = ChooseSpec::target(ChooseSpec::Object(card()))
            .with_count(ironsmith_core::ChoiceCount::exactly(1));
        assert!(validate_card_selection_spec(&spec).is_err());
        let mut filter = ObjectFilter::creature();
        // A comparison against a card is not itself a selected card pool.
        let mut comparison = card();
        comparison.card_types = vec![CardType::Creature];
        filter.no_shared_creature_types_with.push(comparison);
        assert!(validate_card_selection_filter(&filter, None).is_ok());
    }
}
