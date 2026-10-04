//! Whole-selection relations shared by choice execution and cost preflight.
use crate::filter::ObjectFilter;
use crate::game_state::GameState;
use crate::ids::ObjectId;

pub(crate) fn has_relations(filter: &ObjectFilter) -> bool {
    filter.distinct_names || filter.shares_name || filter.shares_color
}

fn names(game: &GameState, id: ObjectId) -> Vec<String> {
    let Some(object) = game.object(id) else {
        return Vec::new();
    };
    let primary = game.current_name(id).unwrap_or_else(|| object.name.to_string());
    primary
        .split(" // ")
        .chain(
            object
                .split_other_half_name()
                .into_iter()
                .flat_map(|name| name.split(" // ")),
        )
        .map(str::trim)
        .filter(|name| !crate::filter::name_is_nameless(name))
        .map(str::to_owned)
        .collect()
}

/// Hidden placeholders are unknown, not the literal shared name "Hidden Card".
/// They may supply a potential preflight group; a public selection is checked
/// again after the chosen identities have been opened for replay.
pub(crate) fn allows(
    game: &GameState,
    filter: &ObjectFilter,
    chosen: &[ObjectId],
    allow_unknown: bool,
) -> bool {
    let mut unique = std::collections::HashSet::new();
    if chosen
        .iter()
        .any(|id| !unique.insert(*id) || game.object(*id).is_none())
    {
        return false;
    }
    if !allow_unknown && chosen.iter().any(|id| game.is_hidden_card_placeholder(*id)) {
        return false;
    }
    let known: Vec<_> = chosen
        .iter()
        .copied()
        .filter(|id| !(allow_unknown && game.is_hidden_card_placeholder(*id)))
        .collect();
    if known.is_empty() {
        return true;
    }
    if filter.shares_color {
        let mut colors = game.current_colors(known[0]).unwrap_or_default();
        for id in known.iter().skip(1) {
            colors = colors.intersection(game.current_colors(*id).unwrap_or_default());
        }
        if colors.is_empty() {
            return false;
        }
    }
    if filter.shares_name || filter.distinct_names {
        let names: Vec<_> = known.iter().map(|id| names(game, *id)).collect();
        if names.iter().any(Vec::is_empty) {
            return false;
        }
        if filter.shares_name
            && !names[0].iter().any(|name| {
                names.iter().skip(1).all(|other| {
                    other
                        .iter()
                        .any(|candidate| crate::filter::names_match(name, candidate))
                })
            })
        {
            return false;
        }
        if filter.distinct_names {
            for (index, left) in names.iter().enumerate() {
                if names[index + 1..].iter().any(|right| {
                    left.iter()
                        .any(|a| right.iter().any(|b| crate::filter::names_match(a, b)))
                }) {
                    return false;
                }
            }
        }
    }
    true
}

/// Find an exact legal subset without making a decision or changing state.
/// Prefix incompatibility is monotone for these relations, so invalid branches
/// can be pruned. The count comes from the authored choice, not a search cap.
pub(crate) fn find_group(
    game: &GameState,
    filter: &ObjectFilter,
    candidates: &[ObjectId],
    required: usize,
    allow_unknown: bool,
) -> Option<Vec<ObjectId>> {
    fn search(
        game: &GameState,
        filter: &ObjectFilter,
        rest: &[ObjectId],
        needed: usize,
        allow_unknown: bool,
        selected: &mut Vec<ObjectId>,
    ) -> bool {
        if needed == 0 {
            return true;
        }
        if rest.len() < needed {
            return false;
        }
        for index in 0..=rest.len() - needed {
            if selected.contains(&rest[index]) {
                continue;
            }
            selected.push(rest[index]);
            if allows(game, filter, selected, allow_unknown)
                && search(
                    game,
                    filter,
                    &rest[index + 1..],
                    needed - 1,
                    allow_unknown,
                    selected,
                )
            {
                return true;
            }
            selected.pop();
        }
        false
    }
    let mut selected = Vec::new();
    search(
        game,
        filter,
        candidates,
        required,
        allow_unknown,
        &mut selected,
    )
    .then_some(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::{CardId, CardType, PlayerId, Zone};
    #[test]
    fn shared_names_use_a_common_name_and_distinct_names_respect_split_names() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut card = |name| {
            game.create_object_from_card(
                &CardBuilder::new(CardId::new(), name)
                    .card_types(vec![CardType::Instant])
                    .build(),
                PlayerId(0),
                Zone::Hand,
            )
        };
        let ab = card("Alpha // Beta");
        let bc = card("Beta // Gamma");
        let ca = card("Gamma // Alpha");
        let beta = card("Beta");
        let nameless = card("");
        let shared = ObjectFilter {
            shares_name: true,
            ..Default::default()
        };
        assert!(allows(&game, &shared, &[ab, bc, beta], false));
        assert!(!allows(&game, &shared, &[ab, bc, ca], false));
        assert!(!allows(&game, &shared, &[nameless, nameless], false));
        let distinct = ObjectFilter {
            distinct_names: true,
            ..Default::default()
        };
        assert!(!allows(&game, &distinct, &[ab, beta], false));
        assert!(!allows(&game, &distinct, &[nameless, beta], false));
        assert_eq!(
            find_group(&game, &shared, &[ca, ab, bc, beta], 3, false),
            Some(vec![ab, bc, beta])
        );
    }
    #[test]
    fn public_group_admission_rechecks_individual_filter_after_identities_open() {
        use crate::decisions::context::{SelectObjectsContext, SelectableObject};
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut make = |kind| {
            game.create_object_from_card(
                &CardBuilder::new(CardId::new(), "Shared name")
                    .card_types(vec![kind])
                    .build(),
                PlayerId(0),
                Zone::Hand,
            )
        };
        let first = make(CardType::Artifact);
        let second = make(CardType::Creature);
        let land = make(CardType::Land);
        let filter = ObjectFilter {
            zone: Some(Zone::Hand),
            owner: Some(crate::target::PlayerFilter::You),
            excluded_card_types: vec![CardType::Land],
            shares_name: true,
            ..Default::default()
        };
        // The offered IDs may have been placeholders before the public answer
        // opened them. Candidate membership alone cannot validate nonland.
        let context = SelectObjectsContext::new(
            PlayerId(0),
            Some(first),
            "group",
            [first, second, land]
                .into_iter()
                .map(|id| SelectableObject::new(id, "Shared name"))
                .collect(),
            2,
            Some(2),
        )
        .with_relation_filter(filter);
        assert!(context.selection_satisfies_relation_filter(&game, &[first, second]));
        assert!(!context.selection_satisfies_relation_filter(&game, &[first, land]));
    }
}
