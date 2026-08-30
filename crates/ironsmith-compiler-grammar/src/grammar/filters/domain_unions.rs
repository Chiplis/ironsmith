use crate::filter::ObjectFilterUnionConnective;
use crate::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation, Zone};

use super::super::super::lexer::OwnedLexToken;
use super::super::primitives::{
    split_lexed_slices_on_comma, split_lexed_slices_on_list_conjunction, split_lexed_slices_on_or,
};
use super::{
    TokenWordView, parse_card_type, parse_object_filter, parse_simple_object_filter_lexed,
    parse_subtype_flexible, try_apply_player_relation_clause,
};

/// Parse a shared-terminal card selector whose coordinated qualities belong
/// to different characteristic axes, such as `Forest and green card`.
///
/// The terminal `card` scopes both qualities. Flattening the phrase through
/// the ordinary object-filter grammar intersects the subtype and color,
/// incorrectly counting only green Forests. Keep the qualities as `any_of`
/// arms while preserving the authored conjunctive presentation surface.
pub fn parse_subtype_color_shared_card_union_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let words = TokenWordView::new(tokens).word_refs();
    let [first, "and", second, noun] = words.as_slice() else {
        return None;
    };
    if !matches!(*noun, "card" | "cards") {
        return None;
    }

    let first_subtype = parse_subtype_flexible(first);
    let second_subtype = parse_subtype_flexible(second);
    let first_color = crate::util::parse_color(first);
    let second_color = crate::util::parse_color(second);
    let (subtype, color, subtype_first) =
        match (first_subtype, first_color, second_subtype, second_color) {
            (Some(subtype), None, None, Some(color)) => (subtype, color, true),
            (None, Some(color), Some(subtype), None) => (subtype, color, false),
            _ => return None,
        };

    let subtype_branch = ObjectFilter {
        subtypes: vec![subtype],
        ..ObjectFilter::default()
    };
    let color_branch = ObjectFilter {
        colors: Some(color),
        ..ObjectFilter::default()
    };
    let branches = if subtype_first {
        vec![subtype_branch, color_branch]
    } else {
        vec![color_branch, subtype_branch]
    };
    let mut union = ObjectFilter {
        any_of: branches,
        other,
        ..ObjectFilter::default()
    };
    union.set_explicit_card_noun(true);
    union.set_conjunctive_set_surface(true);
    Some(union)
}

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

fn branch_has_explicit_object_selector(filter: &ObjectFilter) -> bool {
    filter.stack_kind.is_some()
        || (filter.zone == Some(Zone::Stack) && filter.has_mana_cost)
        || !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.supertypes.is_empty()
        || filter.token
        || filter.is_commander
        || filter.has_explicit_card_noun()
}

#[cfg(test)]
#[path = "domain_unions_inline_tests.rs"]
mod tests;

#[path = "domain_unions/domain_unions_choice.rs"]
mod domain_unions_choice_programs;
use domain_unions_choice_programs::flatten_elided_shared_characteristic_selector;
pub use domain_unions_choice_programs::parse_repeated_selector_domain_union_lexed;
#[path = "domain_unions/domain_unions_reference.rs"]
mod domain_unions_reference_programs;
use domain_unions_reference_programs::{
    contains_target_player_or_planeswalker_controller_relation,
    propagate_trailing_shared_player_scope, trailing_player_scope_is_shared,
};
pub use domain_unions_reference_programs::{
    parse_branch_scoped_object_filter_union_lexed, parse_domain_union_object_filter_lexed,
};
#[path = "domain_unions/domain_unions_core.rs"]
mod domain_unions_core_programs;
pub use domain_unions_core_programs::parse_elided_shared_domain_union;
use domain_unions_core_programs::{
    branch_has_scoped_state, contains_other_than_exclusion, contains_relative_characteristic_union,
    contains_shared_characteristic_comparison_union, factor_common_domain_scope,
    propagate_leading_shared_state,
};
#[path = "domain_unions/domain_unions_zone.rs"]
mod domain_unions_zone_programs;
use domain_unions_zone_programs::{parse_in_zone_at, possessive_zone_owner};
#[path = "domain_unions/domain_unions_combat.rs"]
mod domain_unions_combat_programs;
use domain_unions_combat_programs::{
    contains_attacking_player_or_planeswalker_relation, contains_current_block_partner_relation,
    contains_historical_block_partner_relation,
};
#[path = "domain_unions/domain_unions_object_action.rs"]
mod domain_unions_object_action_programs;
use domain_unions_object_action_programs::propagate_trailing_shared_attachment_scope;
#[path = "domain_unions/domain_unions_library.rs"]
mod domain_unions_library_programs;
use domain_unions_library_programs::propagate_trailing_shared_card_zone_scope;
#[path = "domain_unions/domain_unions_condition.rs"]
mod domain_unions_condition_programs;
use domain_unions_condition_programs::propagate_leading_shared_set_modifiers;
