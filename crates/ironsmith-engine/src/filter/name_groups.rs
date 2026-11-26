//! Reductions over objects with zero, one, or multiple names (CR 201.2a–b).
use super::{ObjectNames, interchangeable_name_key, is_nonlegendary_creature_card_name};
use crate::{GameState, ObjectId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct NameSet {
    literals: Vec<String>,
    all_creatures: bool,
}

impl From<ObjectNames<'_>> for NameSet {
    fn from(names: ObjectNames<'_>) -> Self {
        let mut literals: Vec<_> = names
            .literals()
            .map(|s| interchangeable_name_key(s).to_owned())
            .collect();
        literals.sort_unstable();
        literals.dedup();
        Self {
            literals,
            all_creatures: names.creature_names,
        }
    }
}

pub(crate) fn distinct_count(
    sets: impl IntoIterator<Item = NameSet>,
    count_objects: bool,
    game: &GameState,
) -> i64 {
    if count_objects {
        return differently_named(sets, game) as i64;
    }
    let mut names = BTreeSet::new();
    let mut include_creature_names = false;
    for set in sets {
        include_creature_names |= set.all_creatures;
        names.extend(set.literals);
    }
    if include_creature_names {
        names.extend(super::all_nonlegendary_creature_card_names(game).map(str::to_owned));
    }
    names.len() as i64
}

/// Each group shares one specific Oracle name. Preserve deterministic order
/// and coalesce names that would prompt for exactly the same permanents.
pub(crate) fn shared_groups(
    objects: &[(ObjectId, NameSet)],
    game: &GameState,
) -> Vec<(String, Vec<ObjectId>)> {
    let mut groups: Vec<(String, Vec<ObjectId>)> = Vec::new();
    let mut indices = BTreeMap::new();
    let mut wild = Vec::new();
    for (id, names) in objects {
        for name in &names.literals {
            let index = *indices.entry(name).or_insert_with(|| {
                groups.push((name.clone(), Vec::new()));
                groups.len() - 1
            });
            groups[index].1.push(*id);
        }
        if names.all_creatures {
            wild.push(*id);
        }
    }
    if wild.is_empty() && objects.iter().all(|(_, set)| set.literals.len() <= 1) {
        return groups;
    }
    let order: BTreeMap<_, _> = objects
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, i))
        .collect();
    for (name, members) in &mut groups {
        if !wild.is_empty() && is_nonlegendary_creature_card_name(name, game) {
            members.extend(&wild);
            members.sort_unstable_by_key(|id| order[id]);
            members.dedup();
        }
    }
    if !wild.is_empty() {
        groups.push(("a shared nonlegendary creature name".to_owned(), wild));
    }
    let mut seen = BTreeSet::new();
    groups.retain(|(_, ids)| seen.insert(ids.clone()));
    groups
}

/// Find the largest group of objects with pairwise disjoint sets of names.
/// Count unique singleton names in linear passes; branch only on overlaps
/// between the remaining multi-name objects.
pub(crate) fn differently_named(
    sets: impl IntoIterator<Item = NameSet>,
    game: &GameState,
) -> usize {
    let distinct: BTreeSet<_> = sets
        .into_iter()
        .filter(|s| !s.literals.is_empty() || s.all_creatures)
        .collect();
    let singles: BTreeSet<_> = distinct
        .iter()
        .filter(|s| !s.all_creatures && s.literals.len() == 1)
        .map(|s| &s.literals[0])
        .collect();
    let single_creature = distinct.iter().any(|s| s.all_creatures)
        && singles
            .iter()
            .any(|s| is_nonlegendary_creature_card_name(s, game));
    let multiple: Vec<_> = distinct
        .iter()
        .filter(|s| {
            (s.all_creatures || s.literals.len() > 1)
                && !(single_creature && s.all_creatures)
                && !s.literals.iter().any(|n| singles.contains(n))
        })
        .collect();
    let mut conflicts = vec![Vec::new(); multiple.len()];
    for (i, a) in multiple.iter().enumerate() {
        for (j, b) in multiple.iter().enumerate().skip(i + 1) {
            if a.literals
                .iter()
                .any(|name| b.literals.binary_search(name).is_ok())
                || (a.all_creatures && b.all_creatures)
                || (a.all_creatures
                    && b.literals
                        .iter()
                        .any(|n| is_nonlegendary_creature_card_name(n, game)))
                || (b.all_creatures
                    && a.literals
                        .iter()
                        .any(|n| is_nonlegendary_creature_card_name(n, game)))
            {
                conflicts[i].push(j);
                conflicts[j].push(i);
            }
        }
    }
    // Disjoint components are independent decisions. Within each component,
    // taking a member excludes its neighbors, while skipping it excludes just itself.
    fn largest_disjoint(members: &[usize], conflicts: &[Vec<usize>]) -> usize {
        let Some((degree, pivot)) = members
            .iter()
            .map(|&id| {
                (
                    conflicts[id]
                        .iter()
                        .filter(|v| members.binary_search(v).is_ok())
                        .count(),
                    id,
                )
            })
            .max()
        else {
            return 0;
        };
        if degree == 0 {
            return members.len();
        }
        if degree == members.len() - 1
            && members.iter().all(|&id| {
                members
                    .iter()
                    .all(|&other| id == other || conflicts[id].binary_search(&other).is_ok())
            })
        {
            return 1;
        }
        let skip: Vec<_> = members.iter().copied().filter(|&id| id != pivot).collect();
        let take: Vec<_> = skip
            .iter()
            .copied()
            .filter(|v| conflicts[pivot].binary_search(v).is_err())
            .collect();
        (1 + largest_disjoint(&take, conflicts)).max(largest_disjoint(&skip, conflicts))
    }
    let mut seen = vec![false; multiple.len()];
    let mut total = singles.len();
    for root in 0..multiple.len() {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut pending = vec![root];
        let mut component = Vec::new();
        while let Some(id) = pending.pop() {
            component.push(id);
            for &next in &conflicts[id] {
                if !seen[next] {
                    seen[next] = true;
                    pending.push(next);
                }
            }
        }
        component.sort_unstable();
        total += largest_disjoint(&component, &conflicts);
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(literals: &[&str]) -> NameSet {
        let mut literals: Vec<_> = literals.iter().map(|&s| s.to_string()).collect();
        literals.sort();
        NameSet {
            literals,
            all_creatures: false,
        }
    }

    #[test]
    fn the_number_of_names_and_the_number_of_differently_named_objects_can_differ() {
        let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let sets = [names(&["A", "B"]), names(&["A", "C"]), names(&["B", "D"])];
        assert_eq!(distinct_count(sets.clone(), false, &game), 4);
        assert_eq!(differently_named(sets, &game), 2);
    }

    #[test]
    fn discard_duplicate_legend_choices_for_objects_sharing_multiple_names() {
        let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let objects = [
            (ObjectId::from_raw(1), names(&["A", "B"])),
            (ObjectId::from_raw(2), names(&["A", "B"])),
        ];
        assert_eq!(shared_groups(&objects, &game).len(), 1);
    }
}
