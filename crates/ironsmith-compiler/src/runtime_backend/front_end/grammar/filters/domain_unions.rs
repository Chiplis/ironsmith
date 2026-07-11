use crate::{ObjectFilter, PlayerFilter, Zone};

use super::super::super::lexer::OwnedLexToken;
use super::super::primitives::split_lexed_slices_on_and;
use super::parse_simple_object_filter_lexed;

#[derive(Debug, Clone, PartialEq)]
struct ObjectDomainScope {
    zone: Option<Zone>,
    controller: Option<PlayerFilter>,
    owner: Option<PlayerFilter>,
    single_graveyard: bool,
}

impl ObjectDomainScope {
    fn from_filter(filter: &ObjectFilter) -> Self {
        Self {
            zone: filter.zone,
            controller: filter.controller.clone(),
            owner: filter.owner.clone(),
            single_graveyard: filter.single_graveyard,
        }
    }
}

fn domain_selector_signature(filter: &ObjectFilter) -> Option<ObjectFilter> {
    if !filter.any_of.is_empty() {
        return None;
    }

    let mut signature = filter.clone();
    signature.zone = None;
    signature.controller = None;
    signature.owner = None;
    signature.single_graveyard = false;
    // Oracle can restrict only one domain arm to "other" objects, as in
    // "other Dragons you control and Dragon cards in your graveyard."
    signature.other = false;
    Some(signature)
}

/// Parses a conjunction of independently scoped instances of the same object
/// selector. This keeps battlefield/controller and card-zone/owner facts on
/// separate `any_of` arms instead of collapsing them onto one filter.
pub(super) fn parse_domain_union_object_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let segments = split_lexed_slices_on_and(tokens);
    if segments.len() < 2 {
        return None;
    }

    let branches = segments
        .into_iter()
        .map(|segment| parse_simple_object_filter_lexed(segment, other))
        .collect::<Option<Vec<_>>>()?;

    let first_signature = domain_selector_signature(branches.first()?)?;
    if branches.iter().skip(1).any(|branch| {
        domain_selector_signature(branch)
            .as_ref()
            .is_none_or(|signature| signature != &first_signature)
    }) {
        return None;
    }

    let first_scope = ObjectDomainScope::from_filter(branches.first()?);
    if branches
        .iter()
        .skip(1)
        .all(|branch| ObjectDomainScope::from_filter(branch) == first_scope)
    {
        return None;
    }

    Some(ObjectFilter {
        any_of: branches,
        ..ObjectFilter::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;
    use crate::{CardType, Subtype};

    #[test]
    fn parses_controlled_creature_and_owned_graveyard_card_as_domain_union() {
        let tokens = lex_line(
            "creatures you control and creature cards in your graveyard",
            0,
        )
        .unwrap();
        let filter = parse_domain_union_object_filter_lexed(&tokens, false).unwrap();

        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(filter.any_of[0].card_types, vec![CardType::Creature]);
        assert_eq!(filter.any_of[0].zone, Some(Zone::Battlefield));
        assert_eq!(filter.any_of[0].controller, Some(PlayerFilter::You));
        assert_eq!(filter.any_of[1].card_types, vec![CardType::Creature]);
        assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
        assert_eq!(filter.any_of[1].owner, Some(PlayerFilter::You));
    }

    #[test]
    fn preserves_arm_local_other_qualifier_for_subtype_domain_union() {
        let tokens = lex_line(
            "other Dragons you control and Dragon cards in your graveyard",
            0,
        )
        .unwrap();
        let filter = parse_domain_union_object_filter_lexed(&tokens, false).unwrap();

        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(filter.any_of[0].subtypes, vec![Subtype::Dragon]);
        assert!(filter.any_of[0].other);
        assert_eq!(filter.any_of[0].controller, Some(PlayerFilter::You));
        assert_eq!(filter.any_of[1].subtypes, vec![Subtype::Dragon]);
        assert!(!filter.any_of[1].other);
        assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
        assert_eq!(filter.any_of[1].owner, Some(PlayerFilter::You));
    }

    #[test]
    fn does_not_reinterpret_different_object_selectors_as_domain_union() {
        let tokens = lex_line("artifacts and creatures you control", 0).unwrap();
        assert!(parse_domain_union_object_filter_lexed(&tokens, false).is_none());
    }
}
