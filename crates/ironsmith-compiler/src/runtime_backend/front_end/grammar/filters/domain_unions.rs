use crate::filter::ObjectFilterUnionConnective;
use crate::{ObjectFilter, PlayerFilter, Zone};

use super::super::super::lexer::OwnedLexToken;
use super::super::primitives::split_lexed_slices_on_list_conjunction;
use super::{parse_object_filter, parse_simple_object_filter_lexed};

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

fn possessive_zone_owner(token: &OwnedLexToken) -> Option<PlayerFilter> {
    match token.as_word()? {
        "your" => Some(PlayerFilter::You),
        "their" => Some(PlayerFilter::IteratedPlayer),
        _ => None,
    }
}

fn parse_in_zone_at(
    tokens: &[OwnedLexToken],
    start: usize,
) -> Option<(Zone, Option<PlayerFilter>, usize)> {
    if !tokens.get(start)?.is_word("in") {
        return None;
    }
    let first = tokens.get(start + 1)?;
    if let Some(zone) = first
        .as_word()
        .and_then(crate::runtime_backend::front_end::shared::util::parse_zone_word)
    {
        return Some((zone, None, start + 2));
    }
    let owner = possessive_zone_owner(first)?;
    let zone = tokens
        .get(start + 2)?
        .as_word()
        .and_then(crate::runtime_backend::front_end::shared::util::parse_zone_word)?;
    Some((zone, Some(owner), start + 3))
}

/// Parse an elided shared selector whose objects are counted across two zones.
///
/// Oracle commonly writes the selector once after the domains, as in
/// `cards you own in exile and in your graveyard that are instant cards ...`.
/// Keep the shared owner/type facts on the outer filter and represent only the
/// two disjoint locations as union arms. This uses the existing `ObjectFilter`
/// union semantics and remains generic for other selectors and zone pairs.
fn parse_elided_shared_domain_union(tokens: &[OwnedLexToken], other: bool) -> Option<ObjectFilter> {
    for first_in in 0..tokens.len() {
        let Some((first_zone, first_owner, after_first)) = parse_in_zone_at(tokens, first_in)
        else {
            continue;
        };
        if !tokens
            .get(after_first)
            .is_some_and(|token| token.is_word("and"))
        {
            continue;
        }
        let Some((second_zone, second_owner, after_second)) =
            parse_in_zone_at(tokens, after_first + 1)
        else {
            continue;
        };
        if first_zone == second_zone || first_in == 0 {
            continue;
        }

        let mut shared_tokens = Vec::with_capacity(tokens.len());
        shared_tokens.extend_from_slice(&tokens[..first_in]);
        shared_tokens.extend_from_slice(&tokens[after_second..]);
        let Ok(mut outer) = parse_object_filter(&shared_tokens, other) else {
            continue;
        };
        if !outer.any_of.is_empty() {
            continue;
        }
        // The shared selector is parsed without its two authored domains.
        // Object-filter parsing therefore applies its ordinary battlefield
        // default for typed objects (and even for some explicit card nouns).
        // The union arms below own the complete zone semantics, so remove that
        // inferred default before composing them with the shared constraints.
        outer.zone = None;
        if first_owner
            .as_ref()
            .is_some_and(|owner| outer.owner.as_ref().is_some_and(|outer| outer != owner))
            || second_owner
                .as_ref()
                .is_some_and(|owner| outer.owner.as_ref().is_some_and(|outer| outer != owner))
        {
            continue;
        }

        let mut first_branch = ObjectFilter::default();
        first_branch.zone = Some(first_zone);
        first_branch.owner = first_owner.filter(|owner| outer.owner.as_ref() != Some(owner));
        let mut second_branch = ObjectFilter::default();
        second_branch.zone = Some(second_zone);
        second_branch.owner = second_owner.filter(|owner| outer.owner.as_ref() != Some(owner));
        outer.any_of = vec![first_branch, second_branch];
        return Some(outer);
    }
    None
}

/// Parses a conjunction of independently scoped instances of the same object
/// selector. This keeps battlefield/controller and card-zone/owner facts on
/// separate `any_of` arms instead of collapsing them onto one filter.
pub(super) fn parse_domain_union_object_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    if let Some(filter) = parse_elided_shared_domain_union(tokens, other) {
        return Some(filter);
    }
    let segments = split_lexed_slices_on_list_conjunction(tokens);
    if segments.len() < 2 {
        return None;
    }

    let branches = segments
        .into_iter()
        .map(|segment| {
            let segment = segment
                .first()
                .is_some_and(|token| token.is_word("each"))
                .then(|| segment.get(1..).unwrap_or_default())
                .unwrap_or(segment);
            parse_simple_object_filter_lexed(segment, other)
        })
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

    let mut union = ObjectFilter {
        any_of: branches,
        ..ObjectFilter::default()
    };
    if tokens.iter().any(|token| token.is_word("and/or")) {
        union.set_union_connective(ObjectFilterUnionConnective::AndOr);
    }
    Some(union)
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
    fn parses_and_or_scoped_domains_as_one_semantic_union() {
        let tokens = lex_line(
            "creatures you control and/or creature cards in your graveyard",
            0,
        )
        .unwrap();
        let filter = parse_domain_union_object_filter_lexed(&tokens, false).unwrap();

        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(
            filter.union_connective(),
            ObjectFilterUnionConnective::AndOr
        );
        assert_eq!(filter.any_of[0].zone, Some(Zone::Battlefield));
        assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parses_repeated_each_quantifiers_across_scoped_domains() {
        let tokens = lex_line("Caves you control and each Cave card in your graveyard", 0).unwrap();
        let filter = parse_domain_union_object_filter_lexed(&tokens, false).unwrap();

        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(filter.any_of[0].subtypes, vec![Subtype::Cave]);
        assert_eq!(filter.any_of[0].zone, Some(Zone::Battlefield));
        assert_eq!(filter.any_of[0].controller, Some(PlayerFilter::You));
        assert_eq!(filter.any_of[1].subtypes, vec![Subtype::Cave]);
        assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
        assert_eq!(filter.any_of[1].owner, Some(PlayerFilter::You));
    }

    #[test]
    fn does_not_reinterpret_different_object_selectors_as_domain_union() {
        let tokens = lex_line("artifacts and creatures you control", 0).unwrap();
        assert!(parse_domain_union_object_filter_lexed(&tokens, false).is_none());
    }

    #[test]
    fn parses_elided_owned_selector_across_exile_and_graveyard() {
        let tokens = lex_line(
            "cards you own in exile and in your graveyard that are instant cards, are sorcery cards, and/or have an Adventure",
            0,
        )
        .unwrap();
        let filter = parse_domain_union_object_filter_lexed(&tokens, false).unwrap();

        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(
            filter.card_types,
            vec![crate::CardType::Instant, crate::CardType::Sorcery]
        );
        assert_eq!(filter.subtypes, vec![Subtype::Adventure]);
        assert!(filter.type_or_subtype_union);
        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(filter.any_of[0].zone, Some(Zone::Exile));
        assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
        assert_eq!(
            domain_selector_signature(&filter.any_of[0]),
            Some(ObjectFilter::default())
        );
        assert_eq!(
            domain_selector_signature(&filter.any_of[1]),
            Some(ObjectFilter::default())
        );
    }
}
