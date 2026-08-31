use super::*;

pub(super) fn branch_has_scoped_state(filter: &ObjectFilter) -> bool {
    filter.other
        || filter.zone.is_some()
        || filter.controller.is_some()
        || filter.owner.is_some()
        || filter.attacking
        || filter.attacking_alone
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

pub(super) fn contains_other_than_exclusion(tokens: &[OwnedLexToken]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        token.is_word("other")
            && tokens
                .get(index + 1)
                .is_some_and(|next| next.is_word("than"))
    })
}

pub(super) fn propagate_leading_shared_state(
    tokens: &[OwnedLexToken],
    branches: &mut [ObjectFilter],
) {
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

pub(super) fn factor_common_domain_scope(branches: &mut [ObjectFilter], union: &mut ObjectFilter) {
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
    let common_other = branches.iter().all(|branch| branch.other);
    let common_nontoken = branches.iter().all(|branch| branch.nontoken);

    union.zone = common_zone;
    union.controller = common_controller.clone();
    union.owner = common_owner.clone();
    union.other = common_other;
    union.nontoken = common_nontoken;
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
        if common_other {
            branch.other = false;
        }
        if common_nontoken {
            branch.nontoken = false;
        }
    }
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
pub(super) fn contains_relative_characteristic_union(tokens: &[OwnedLexToken]) -> bool {
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

/// A connective in the comparison side of a shared-characteristic relation
/// joins comparison domains, not candidate object-filter branches.
///
/// For example, in
///
/// `a creature spell that doesn't share a creature type with a creature you
/// control or a creature card in your graveyard`
///
/// the spell remains one stack-object filter. Splitting on the inner `or`
/// makes the graveyard comparison arm look like the spell's cast-origin
/// domain and silently changes the trigger to "cast from your graveyard."
pub(super) fn contains_shared_characteristic_comparison_union(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    words.iter().enumerate().any(|(share_idx, word)| {
        if !matches!(*word, "share" | "shares") {
            return false;
        }
        let Some(with_offset) =
            crate::word_primitives::parse_sequence_start(&words[share_idx + 1..], &["with"])
        else {
            return false;
        };
        let with_idx = share_idx + 1 + with_offset;
        words[share_idx + 1..with_idx]
            .iter()
            .any(|word| matches!(*word, "type" | "types" | "color" | "colors"))
            && words[with_idx + 1..]
                .iter()
                .any(|word| matches!(*word, "or" | "and/or"))
    })
}

/// Parse an elided shared selector whose objects are counted across two zones.
///
/// Oracle commonly writes the selector once after the domains, as in
/// `cards you own in exile and in your graveyard that are instant cards ...`.
/// Keep the shared owner/type facts on the outer filter and represent only the
/// two disjoint locations as union arms. This uses the existing `ObjectFilter`
/// union semantics and remains generic for other selectors and zone pairs.
pub fn parse_elided_shared_domain_union(
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

        let leading_scope = crate::grammar::primitives::probe_shape(parse_object_filter(
            &tokens[..first_in],
            other,
        ));
        let mut shared_tokens = Vec::with_capacity(tokens.len());
        shared_tokens.extend_from_slice(&tokens[..first_in]);
        shared_tokens.extend_from_slice(&tokens[after_second..]);
        let Ok(mut outer) = parse_object_filter(&shared_tokens, other) else {
            continue;
        };
        if !outer.any_of.is_empty() {
            let Some(leading_scope) = leading_scope.as_ref() else {
                continue;
            };
            let Some(flattened) =
                flatten_elided_shared_characteristic_selector(leading_scope, outer)
            else {
                continue;
            };
            outer = flattened;
        }
        if outer.owner.is_none() {
            let leading_words = TokenWordView::new(&tokens[..first_in]).word_refs();
            let typed_owner = (0..leading_words.len()).find_map(|start| {
                let mut relation = ObjectFilter::default();
                try_apply_player_relation_clause(
                    &mut relation,
                    &leading_words[start..],
                    &PlayerFilter::IteratedPlayer,
                )
                .and(relation.owner)
            });
            outer.owner = leading_scope
                .as_ref()
                .and_then(|scope| scope.owner.clone())
                .or(typed_owner);
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
