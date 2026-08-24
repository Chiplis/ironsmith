use super::*;

pub(super) fn flatten_elided_shared_characteristic_selector(
    leading_scope: &ObjectFilter,
    mut selector: ObjectFilter,
) -> Option<ObjectFilter> {
    let mut scope_shape = leading_scope.clone();
    scope_shape.zone = None;
    scope_shape.controller = None;
    scope_shape.owner = None;
    scope_shape.single_graveyard = false;
    if !scope_shape.any_of.is_empty() || scope_shape != ObjectFilter::default() {
        return None;
    }

    let selector_branches = std::mem::take(&mut selector.any_of);
    if selector_branches.len() < 2 || selector != ObjectFilter::default() {
        return None;
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for mut branch in selector_branches {
        if !matches!(branch.zone, None | Some(Zone::Battlefield))
            || branch
                .controller
                .as_ref()
                .is_some_and(|controller| leading_scope.controller.as_ref() != Some(controller))
            || branch
                .owner
                .as_ref()
                .is_some_and(|owner| leading_scope.owner.as_ref() != Some(owner))
        {
            return None;
        }
        branch.zone = None;
        branch.controller = None;
        branch.owner = None;
        branch.single_graveyard = false;
        let branch_card_types = std::mem::take(&mut branch.card_types);
        let branch_subtypes = std::mem::take(&mut branch.subtypes);
        branch.type_or_subtype_union = false;
        if branch != ObjectFilter::default()
            || (branch_card_types.is_empty() && branch_subtypes.is_empty())
        {
            return None;
        }
        for card_type in branch_card_types {
            if !card_types.contains(&card_type) {
                card_types.push(card_type);
            }
        }
        for subtype in branch_subtypes {
            if !subtypes.contains(&subtype) {
                subtypes.push(subtype);
            }
        }
    }
    if card_types.is_empty() || subtypes.is_empty() {
        return None;
    }

    let mut flattened = leading_scope.clone();
    flattened.zone = None;
    flattened.card_types = card_types;
    flattened.subtypes = subtypes;
    flattened.type_or_subtype_union = true;
    Some(flattened)
}

/// Parse independently scoped domains that repeat the same object selector.
///
/// This narrower entry point lets callers distinguish
/// `creatures you control and creature cards in your graveyard` from a
/// characteristic list with one shared terminal noun such as
/// `instant and sorcery cards in your graveyard`.
pub fn parse_repeated_selector_domain_union_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let union = parse_domain_union_object_filter_lexed(tokens, other)?;
    let first_signature = domain_selector_signature(union.any_of.first()?)?;
    union
        .any_of
        .iter()
        .skip(1)
        .all(|branch| domain_selector_signature(branch).as_ref() == Some(&first_signature))
        .then_some(union)
}
