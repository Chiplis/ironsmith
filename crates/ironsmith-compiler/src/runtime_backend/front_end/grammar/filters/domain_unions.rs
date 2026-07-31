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
    !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.supertypes.is_empty()
        || filter.token
        || filter.is_commander
        || filter.has_explicit_card_noun()
}

fn branch_has_scoped_state(filter: &ObjectFilter) -> bool {
    filter.controller.is_some()
        || filter.owner.is_some()
        || filter.attacking
        || filter.blocking
        || filter.blocked
        || filter.unblocked
        || filter.tapped
        || filter.untapped
        || filter.power.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || !filter.tagged_constraints.is_empty()
}

fn contains_other_than_exclusion(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .windows(2)
        .any(|window| window[0].is_word("other") && window[1].is_word("than"))
}

fn contains_attacking_player_or_planeswalker_relation(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let Some(attacking) = words.iter().position(|word| *word == "attacking") else {
        return false;
    };
    words[attacking..].windows(3).any(|window| {
        window[0] == "or" && matches!(window[1], "a" | "an" | "the") && window[2] == "planeswalker"
    })
}

fn propagate_leading_shared_state(tokens: &[OwnedLexToken], branches: &mut [ObjectFilter]) {
    let first_word = tokens.iter().find_map(OwnedLexToken::as_word);
    match first_word {
        Some("tapped") => {
            for branch in branches {
                branch.tapped = true;
            }
        }
        Some("untapped") => {
            for branch in branches {
                branch.untapped = true;
            }
        }
        _ => {}
    }
}

fn propagate_trailing_shared_player_scope(branches: &mut [ObjectFilter]) {
    let Some((last, preceding)) = branches.split_last_mut() else {
        return;
    };

    if let Some(controller) = last.controller.clone()
        && preceding.iter().all(|branch| branch.controller.is_none())
    {
        for branch in preceding.iter_mut() {
            branch.controller = Some(controller.clone());
        }
    }
    // A trailing owner is usually a possessive on the last arm's own zone
    // ("... and each creature card in your graveyard"), not a scope stated once
    // for the whole list. An arm that already names its own controller
    // ("each creature you control") therefore keeps that scope alone: adding the
    // owner too would both narrow the match to permanents you own and render as
    // "creature you both own and control".
    if let Some(owner) = last.owner.clone()
        && preceding
            .iter()
            .all(|branch| branch.owner.is_none() && branch.controller.is_none())
    {
        for branch in preceding.iter_mut() {
            branch.owner = Some(owner.clone());
        }
    }
}

fn propagate_trailing_shared_attachment_scope(branches: &mut [ObjectFilter]) {
    let Some((last, preceding)) = branches.split_last_mut() else {
        return;
    };
    let [constraint] = last.tagged_constraints.as_slice() else {
        return;
    };
    if !matches!(
        constraint.relation,
        TaggedOpbjectRelation::AttachedToTaggedObject
            | TaggedOpbjectRelation::WasAttachedToTaggedObject
    ) || preceding
        .iter()
        .any(|branch| !branch.tagged_constraints.is_empty())
    {
        return;
    }

    for branch in preceding {
        branch.tagged_constraints.push(constraint.clone());
    }
}

fn factor_common_domain_scope(branches: &mut [ObjectFilter], union: &mut ObjectFilter) {
    let Some(first) = branches.first() else {
        return;
    };
    let common_zone = branches
        .iter()
        .all(|branch| branch.zone == first.zone)
        .then_some(first.zone)
        .flatten();
    let common_controller = branches
        .iter()
        .all(|branch| branch.controller == first.controller)
        .then(|| first.controller.clone())
        .flatten();
    let common_owner = branches
        .iter()
        .all(|branch| branch.owner == first.owner)
        .then(|| first.owner.clone())
        .flatten();

    union.zone = common_zone;
    union.controller = common_controller.clone();
    union.owner = common_owner.clone();
    for branch in branches {
        if common_zone.is_some() {
            branch.zone = None;
        }
        if common_controller.is_some() {
            branch.controller = None;
        }
        if common_owner.is_some() {
            branch.owner = None;
        }
    }
}

fn contains_target_player_or_planeswalker_controller_relation(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    (0..words.len()).any(|start| {
        let mut relation = ObjectFilter::default();
        try_apply_player_relation_clause(
            &mut relation,
            &words[start..],
            &PlayerFilter::IteratedPlayer,
        )
        .is_some()
            && relation.controller == Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
    })
}

/// A relative characteristic list has one shared object domain before the
/// copula and selector alternatives after it:
///
/// `other creature you control that's a token or a Rabbit`
/// `creature that isn't an Insect, Rat, Spider, or Squirrel`
///
/// Splitting that sentence at `or` would incorrectly scope `other creature
/// you control` to only the token arm, and splitting the negative form would
/// turn the excluded characteristics back into positive union arms. Leave
/// both grammars to the ordinary filter parser, which retains the common
/// domain and the copula's scope.
fn contains_relative_characteristic_union(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let Some(characteristic_start) = words
        .iter()
        .enumerate()
        .find_map(|(idx, word)| match *word {
            "that's" | "thats" => Some(idx + 1),
            "isn't" | "isnt" | "aren't" | "arent" => Some(idx + 1),
            _ if matches!(
                words.get(idx..idx + 3),
                Some(["that", "is", "not"] | ["that", "are", "not"])
            ) =>
            {
                Some(idx + 3)
            }
            _ if matches!(
                words.get(idx..idx + 2),
                Some(["that", "is"] | ["that", "are"])
            ) =>
            {
                Some(idx + 2)
            }
            _ => None,
        })
    else {
        return false;
    };
    let Some(characteristics) = words.get(characteristic_start..) else {
        return false;
    };
    if !characteristics
        .iter()
        .any(|word| matches!(*word, "or" | "and/or"))
    {
        return false;
    }

    let mut selectors = 0;
    for word in characteristics {
        if matches!(
            *word,
            "a" | "an" | "the" | "and" | "or" | "and/or" | "card" | "cards"
        ) {
            continue;
        }
        if matches!(*word, "token" | "tokens")
            || parse_card_type(word).is_some()
            || parse_subtype_flexible(word).is_some()
        {
            selectors += 1;
            continue;
        }
        return false;
    }
    selectors >= 2
}

/// `creature that blocked or was blocked by a Zombie this turn` is one
/// historical relation with a nested partner filter, not an object-domain
/// union. Splitting at `or` flattens it into the nonsensical pair "blocked
/// creature or blocked Zombie" before the reference/tag grammar can retain
/// the partner characteristics.
fn contains_historical_block_partner_relation(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    words.windows(5).any(|window| {
        window == ["blocked", "or", "was", "blocked", "by"]
            && words.windows(2).any(|tail| tail == ["this", "turn"])
    })
}

/// `creature blocking or blocked by this creature` describes one creature
/// related to the source, not a union between a blocking creature and a
/// blocked creature. Leave the connective for the reference/tag grammar so it
/// can retain the source-relative combat constraint.
fn contains_current_block_partner_relation(tokens: &[OwnedLexToken]) -> bool {
    TokenWordView::new(tokens)
        .word_refs()
        .windows(4)
        .any(|window| window == ["blocking", "or", "blocked", "by"])
}

/// Parse a union whose branches each name their own object class and may
/// carry independent qualifiers.
///
/// Flattening `Creatures and Vehicles you control` produces the impossible
/// intersection "Vehicle creatures"; flattening `an attacking creature you
/// control or a blocking creature an opponent controls` loses both
/// branch-scoped controllers and combat states. Keep independently nouned
/// branches as `any_of` selectors, while factoring a genuinely shared
/// trailing controller/owner scope onto the outer filter.
pub(crate) fn parse_branch_scoped_object_filter_union_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    if other {
        return None;
    }
    // A comma before an authored value-definition clause is not a list
    // separator. For example, in
    //
    // `creature with toughness X or less, where X is the number of Shrines
    // you control`
    //
    // the comparison grammar owns the entire suffix. Treating the comma as a
    // union boundary invents a second "Shrine you control" object-filter arm
    // and can even leak its controller onto the target creature.
    if tokens
        .windows(2)
        .any(|window| window[0].is_comma() && window[1].is_word("where"))
    {
        return None;
    }
    if contains_relative_characteristic_union(tokens) {
        return None;
    }
    if contains_other_than_exclusion(tokens) {
        return None;
    }
    if contains_attacking_player_or_planeswalker_relation(tokens) {
        return None;
    }
    if contains_historical_block_partner_relation(tokens) {
        return None;
    }
    if contains_current_block_partner_relation(tokens) {
        return None;
    }
    if contains_target_player_or_planeswalker_controller_relation(tokens) {
        return None;
    }
    if parse_elided_shared_domain_union(tokens, other).is_some() {
        return None;
    }

    let has_plain_or = tokens.iter().any(|token| token.is_word("or"));
    let has_and_or = tokens.iter().any(|token| token.is_word("and/or"));
    let has_plain_and = tokens.iter().any(|token| token.is_word("and"));
    let comma_segments = split_lexed_slices_on_comma(tokens);
    let segments = if comma_segments.len() >= 2 {
        comma_segments
            .into_iter()
            .map(|segment| {
                segment
                    .first()
                    .is_some_and(|token| {
                        token.is_word("and") || token.is_word("or") || token.is_word("and/or")
                    })
                    .then(|| segment.get(1..).unwrap_or_default())
                    .unwrap_or(segment)
            })
            .collect()
    } else if has_plain_or {
        split_lexed_slices_on_or(tokens)
    } else {
        split_lexed_slices_on_list_conjunction(tokens)
    };
    if segments.len() < 2 {
        return None;
    }

    let branches = segments
        .into_iter()
        .map(|segment| {
            let segment = segment
                .first()
                .is_some_and(|token| token.is_word("all") || token.is_word("each"))
                .then(|| segment.get(1..).unwrap_or_default())
                .unwrap_or(segment);
            parse_object_filter(segment, false).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    let mut branches = branches
        .into_iter()
        .flat_map(|mut branch| {
            let nested = std::mem::take(&mut branch.any_of);
            if nested.is_empty() {
                vec![branch]
            } else if branch == ObjectFilter::default()
                && branch.union_connective() == ObjectFilterUnionConnective::Or
            {
                nested
            } else {
                // Preserve an opaque nested selector so the explicit-selector
                // guard below rejects an unsafe flattening.
                branch.any_of = nested;
                vec![branch]
            }
        })
        .collect::<Vec<_>>();
    if branches
        .iter()
        .any(|branch| !branch_has_explicit_object_selector(branch))
    {
        return None;
    }

    propagate_trailing_shared_player_scope(&mut branches);
    propagate_trailing_shared_attachment_scope(&mut branches);
    propagate_leading_shared_state(tokens, &mut branches);
    let mut union = ObjectFilter::default();
    factor_common_domain_scope(&mut branches, &mut union);
    let mixes_card_type_and_subtype = branches
        .iter()
        .any(|branch| !branch.card_types.is_empty() || !branch.all_card_types.is_empty())
        && branches.iter().any(|branch| !branch.subtypes.is_empty());
    if !mixes_card_type_and_subtype && !branches.iter().any(branch_has_scoped_state) {
        // Common zone/controller/owner scope does not make otherwise bare
        // card-type arms branch-local. Let the ordinary filter grammar flatten
        // lists such as "artifact, creature, land, or planeswalker you
        // control" into one typed selector so downstream cost renderers retain
        // the authored list instead of treating it as a generic permanent.
        return None;
    }

    union.any_of = branches;
    if has_and_or {
        union.set_union_connective(ObjectFilterUnionConnective::AndOr);
    } else if has_plain_and && !has_plain_or {
        union.set_conjunctive_set_surface(true);
    }
    Some(union)
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

fn flatten_elided_shared_characteristic_selector(
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

/// Parse an elided shared selector whose objects are counted across two zones.
///
/// Oracle commonly writes the selector once after the domains, as in
/// `cards you own in exile and in your graveyard that are instant cards ...`.
/// Keep the shared owner/type facts on the outer filter and represent only the
/// two disjoint locations as union arms. This uses the existing `ObjectFilter`
/// union semantics and remains generic for other selectors and zone pairs.
pub(crate) fn parse_elided_shared_domain_union(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
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
            let Ok(leading_scope) = parse_object_filter(&tokens[..first_in], other) else {
                continue;
            };
            let Some(flattened) =
                flatten_elided_shared_characteristic_selector(&leading_scope, outer)
            else {
                continue;
            };
            outer = flattened;
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
pub(crate) fn parse_domain_union_object_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    if contains_other_than_exclusion(tokens)
        || contains_attacking_player_or_planeswalker_relation(tokens)
    {
        return None;
    }
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

    // Heterogeneous multi-zone target lists ("target spell, nonland
    // permanent, or card in a graveyard") union arms whose domains are all
    // explicit and pairwise distinct; picking one arm silently drops the
    // others.
    // Every arm must carry its own explicit zone: a battlefield-default arm
    // usually shares a trailing zone with its siblings ("Aura and/or
    // Equipment cards from your graveyard") and must not union.
    let distinct_zones: std::collections::HashSet<_> =
        branches.iter().map(|branch| branch.zone).collect();
    let heterogeneous_multi_zone = branches.iter().all(|branch| branch.zone.is_some())
        && distinct_zones.len() == branches.len();

    if !heterogeneous_multi_zone {
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

/// Parse independently scoped domains that repeat the same object selector.
///
/// This narrower entry point lets callers distinguish
/// `creatures you control and creature cards in your graveyard` from a
/// characteristic list with one shared terminal noun such as
/// `instant and sorcery cards in your graveyard`.
pub(crate) fn parse_repeated_selector_domain_union_lexed(
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
        let filter = parse_object_filter(&tokens, false).unwrap();

        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(filter.any_of[0].card_types, vec![CardType::Creature]);
        assert_eq!(filter.any_of[0].zone, Some(Zone::Battlefield));
        assert_eq!(filter.any_of[0].controller, Some(PlayerFilter::You));
        assert_eq!(filter.any_of[1].card_types, vec![CardType::Creature]);
        assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
        assert_eq!(filter.any_of[1].owner, Some(PlayerFilter::You));
    }

    #[test]
    fn flattens_owned_nonbattlefield_zone_set_beside_a_controlled_battlefield_set() {
        let tokens = lex_line(
            "lands you control and land cards you own that aren't on the battlefield",
            0,
        )
        .unwrap();
        let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false)
            .expect("the two authored domains should remain independently scoped");

        assert_eq!(filter.any_of.len(), 6, "{filter:#?}");
        assert!(filter.any_of.iter().any(|branch| {
            branch.zone == Some(Zone::Battlefield)
                && branch.controller == Some(PlayerFilter::You)
                && branch.owner.is_none()
                && branch.card_types == [CardType::Land]
        }));
        for zone in [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Command,
        ] {
            assert!(filter.any_of.iter().any(|branch| {
                branch.zone == Some(zone)
                    && branch.owner == Some(PlayerFilter::You)
                    && branch.controller.is_none()
                    && branch.card_types == [CardType::Land]
            }));
        }
    }

    #[test]
    fn parses_shared_controller_type_subtype_conjunction_as_scoped_union() {
        let tokens = lex_line("Creatures and Vehicles you control", 0).unwrap();
        let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(filter.has_conjunctive_set_surface());
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| branch.card_types == [CardType::Creature]),
            "{filter:#?}"
        );
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| branch.subtypes == [Subtype::Vehicle]),
            "{filter:#?}"
        );
        assert_eq!(filter.description(), "a creature and Vehicle you control");
    }

    #[test]
    fn trailing_attachment_provenance_scopes_every_coordinated_noun() {
        for (text, relation) in [
            (
                "Auras and Equipment that were attached to it",
                TaggedOpbjectRelation::WasAttachedToTaggedObject,
            ),
            (
                "Auras and Equipment attached to it",
                TaggedOpbjectRelation::AttachedToTaggedObject,
            ),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

            assert_eq!(filter.any_of.len(), 2, "{text}: {filter:#?}");
            assert!(
                filter.any_of.iter().all(|branch| {
                    matches!(
                        branch.tagged_constraints.as_slice(),
                        [constraint]
                            if constraint.tag == crate::TagKey::from(crate::cards::builders::IT_TAG)
                                && constraint.relation == relation
                    )
                }),
                "{text}: {filter:#?}"
            );
        }
    }

    #[test]
    fn relative_characteristic_union_is_not_split_from_its_common_domain() {
        let tokens = lex_line("other creature you control that's a token or a Rabbit", 0).unwrap();

        assert!(
            contains_relative_characteristic_union(&tokens),
            "the relative selector list should be recognized lexically"
        );
        assert!(
            parse_branch_scoped_object_filter_union_lexed(&tokens, false).is_none(),
            "the shared creature/controller domain belongs outside both selector arms"
        );
    }

    #[test]
    fn historical_block_partner_relation_is_not_split_as_an_or_union() {
        let tokens = lex_line(
            "creature that blocked or was blocked by a Zombie this turn",
            0,
        )
        .unwrap();

        assert!(contains_historical_block_partner_relation(&tokens));
        assert!(
            parse_branch_scoped_object_filter_union_lexed(&tokens, false).is_none(),
            "the Zombie is the nested combat partner, not a second object-domain arm"
        );
    }

    #[test]
    fn current_block_partner_relation_is_not_split_as_an_or_union() {
        let tokens = lex_line("creature blocking or blocked by this creature", 0).unwrap();

        assert!(contains_current_block_partner_relation(&tokens));
        assert!(
            parse_branch_scoped_object_filter_union_lexed(&tokens, false).is_none(),
            "blocking and blocked describe the two directions of one source-relative relation"
        );
        let filter = parse_object_filter(&tokens, false).unwrap();
        assert!(filter.any_of.is_empty(), "{filter:#?}");
        assert!(filter.in_combat_with_source, "{filter:#?}");
        assert_eq!(
            filter.description(),
            "creature blocking or blocked by this creature"
        );
    }

    #[test]
    fn shared_controller_card_type_list_defers_to_flat_filter_grammar() {
        let tokens = lex_line(
            "an artifact, creature, land, or planeswalker you control",
            0,
        )
        .unwrap();

        assert!(
            parse_branch_scoped_object_filter_union_lexed(&tokens, false).is_none(),
            "a common controller does not make otherwise bare card types branch-local"
        );
        let filter = parse_object_filter(&tokens, false).unwrap();
        assert!(filter.any_of.is_empty(), "{filter:#?}");
        assert_eq!(
            filter.card_types,
            [
                CardType::Artifact,
                CardType::Creature,
                CardType::Land,
                CardType::Planeswalker,
            ]
        );
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(
            filter.description(),
            "an artifact, creature, land, or planeswalker you control"
        );
    }

    #[test]
    fn mirrored_owner_or_controller_scope_keeps_one_shared_object_noun() {
        let tokens = lex_line("permanent you own or control", 0).unwrap();
        let filter =
            crate::runtime_backend::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(
                &tokens,
                false,
            )
            .unwrap();

        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert_eq!(filter.description(), "a permanent you own or control");
    }

    #[test]
    fn nontoken_comma_modifier_is_not_a_standalone_union_arm() {
        let tokens = lex_line("a nontoken, non-Angel creature you control", 0).unwrap();

        assert!(
            parse_branch_scoped_object_filter_union_lexed(&tokens, false).is_none(),
            "`nontoken` is an adjective modifying the shared creature noun"
        );
        let filter = parse_object_filter(&tokens, false).unwrap();
        assert!(filter.any_of.is_empty(), "{filter:#?}");
        assert!(filter.nontoken);
        assert_eq!(filter.excluded_subtypes, [Subtype::Angel]);
        assert_eq!(filter.card_types, [CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(
            filter.description(),
            "a nontoken non-angel creature you control"
        );
    }

    #[test]
    fn preserves_equipped_state_on_only_its_conjunctive_union_arm() {
        let tokens = lex_line("equipped creatures and Equipment you control", 0).unwrap();
        let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(filter.has_conjunctive_set_surface());
        let equipped_creature = filter
            .any_of
            .iter()
            .find(|branch| branch.card_types == [CardType::Creature])
            .expect("equipped creature branch");
        assert!(
            equipped_creature
                .tagged_constraints
                .iter()
                .any(|constraint| {
                    constraint.tag.as_str() == "equipped"
                        && constraint.relation == crate::TaggedOpbjectRelation::IsTaggedObject
                })
        );
        let equipment = filter
            .any_of
            .iter()
            .find(|branch| branch.subtypes == [Subtype::Equipment])
            .expect("Equipment branch");
        assert!(equipment.tagged_constraints.is_empty(), "{equipment:#?}");
        assert_eq!(
            filter.description(),
            "an equipped creature and Equipment you control"
        );
    }

    #[test]
    fn preserves_branch_local_combat_state_and_controllers_in_or_union() {
        let tokens = lex_line(
            "an attacking creature you control or a blocking creature an opponent controls",
            0,
        )
        .unwrap();
        let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.controller, None);
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(filter.any_of.iter().any(|branch| {
            branch.attacking && branch.controller == Some(PlayerFilter::You) && !branch.blocking
        }));
        assert!(filter.any_of.iter().any(|branch| {
            branch.blocking
                && branch.controller == Some(PlayerFilter::Opponent)
                && !branch.attacking
        }));
        assert_eq!(
            filter.description(),
            "an attacking creature you control or a blocking creature an opponent controls"
        );
    }

    #[test]
    fn comma_list_preserves_internal_ownership_conjunction_and_attachment_scopes() {
        let tokens = lex_line(
            "enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control",
            0,
        )
        .unwrap();
        let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.controller, None);
        assert!(filter.has_conjunctive_set_surface());
        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");

        let enchantments = filter
            .any_of
            .iter()
            .find(|branch| branch.card_types == [CardType::Enchantment])
            .expect("enchantment branch");
        assert_eq!(enchantments.controller, Some(PlayerFilter::You));
        assert_eq!(enchantments.owner, None);

        let aura_branches = filter
            .any_of
            .iter()
            .filter(|branch| branch.subtypes == [Subtype::Aura])
            .collect::<Vec<_>>();
        assert_eq!(aura_branches.len(), 2, "{filter:#?}");
        let controlled_host = aura_branches
            .iter()
            .find(|branch| {
                branch.attached_to_object.as_deref().is_some_and(|host| {
                    !host.attacking && host.controller == Some(PlayerFilter::You)
                })
            })
            .expect("Aura attached to a controlled permanent");
        assert_eq!(controlled_host.owner, None);
        assert!(
            controlled_host
                .attached_to_object
                .as_deref()
                .is_some_and(ObjectFilter::has_plural_object_noun_surface)
        );

        let opposing_attacker = aura_branches
            .iter()
            .filter_map(|branch| branch.attached_to_object.as_deref())
            .find(|host| host.attacking && host.controller == Some(PlayerFilter::Opponent));
        assert!(opposing_attacker.is_some(), "{filter:#?}");
    }

    #[test]
    fn comma_list_keeps_other_and_controller_scope_on_each_destroy_branch() {
        let tokens = lex_line(
            "other enchantments you control, all other Auras attached to permanents you control, and all other Auras attached to attacking creatures your opponents control",
            0,
        )
        .unwrap();
        let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.owner, None);
        assert_eq!(filter.controller, None);
        assert!(filter.has_conjunctive_set_surface());
        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
        assert!(filter.any_of.iter().all(|branch| branch.other));

        let enchantments = filter
            .any_of
            .iter()
            .find(|branch| branch.card_types == [CardType::Enchantment])
            .expect("other enchantment branch");
        assert_eq!(enchantments.controller, Some(PlayerFilter::You));

        assert!(filter.any_of.iter().any(|branch| {
            branch.subtypes == [Subtype::Aura]
                && branch.attached_to_object.as_deref().is_some_and(|host| {
                    !host.attacking && host.controller == Some(PlayerFilter::You)
                })
        }));
        assert!(filter.any_of.iter().any(|branch| {
            branch.subtypes == [Subtype::Aura]
                && branch.attached_to_object.as_deref().is_some_and(|host| {
                    host.attacking && host.controller == Some(PlayerFilter::Opponent)
                })
        }));
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

    #[test]
    fn parses_elided_owned_selector_with_repeated_card_alternatives() {
        let tokens = lex_line(
            "card you own in exile and in your graveyard that's an instant card, a sorcery card, or a card that has an Adventure",
            0,
        )
        .unwrap();
        let filter =
            crate::runtime_backend::object_filters::parse_object_filter_lexed(&tokens, false)
                .unwrap();

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
        assert!(
            filter.any_of.iter().all(|branch| branch.owner.is_none()),
            "{filter:#?}"
        );
    }
}
