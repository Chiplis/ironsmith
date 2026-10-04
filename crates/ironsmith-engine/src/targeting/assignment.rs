use std::collections::{HashMap, HashSet};
use std::ops::Range;

use crate::decisions::context::TargetRequirementContext;
use crate::game_state::Target;

#[derive(Clone, Default)]
struct TargetGroupSelections {
    distinct: HashMap<usize, HashSet<Target>>,
    players: HashMap<usize, crate::ids::PlayerId>,
    selected_by_requirement: Vec<Vec<Target>>,
}

fn selected_targets_satisfy_requirement(
    req: &TargetRequirementContext,
    selected: &[Target],
) -> bool {
    if selected
        .iter()
        .any(|target| !req.legal_targets.contains(target))
    {
        return false;
    }

    let satisfies_set_constraint = req.legal_target_sets.is_empty()
        || req
            .legal_target_sets
            .iter()
            .any(|set| selected.iter().all(|target| set.contains(target)));
    let satisfies_aggregate_constraint = req
        .aggregate_constraint
        .as_ref()
        .is_none_or(|constraint| constraint.allows(selected));

    satisfies_set_constraint && satisfies_aggregate_constraint
}

fn legal_pool_for_selected(
    req: &TargetRequirementContext,
    selected: &[Target],
) -> Option<Vec<Target>> {
    if req.legal_target_sets.is_empty() {
        return Some(req.legal_targets.clone());
    }

    req.legal_target_sets
        .iter()
        .find(|set| selected.iter().all(|target| set.contains(target)))
        .cloned()
        .or_else(|| {
            selected
                .is_empty()
                .then(|| req.legal_target_sets.first().cloned())
                .flatten()
        })
}

fn selected_targets_satisfy_distinct_player_group(
    req: &TargetRequirementContext,
    selected: &[Target],
    used_by_group: &TargetGroupSelections,
    allow_unresolved: bool,
) -> bool {
    if let Some(relation) = &req.shared_player_group {
        if let Some(pair) = &relation.pair_constraint {
            let anchor = used_by_group.selected_by_requirement.get(pair.prior_requirement)
                .and_then(|targets| targets.first());
            if !selected.is_empty() && !anchor.is_some_and(|anchor| selected.iter()
                .all(|target| pair.allowed_pairs.contains(&(*anchor, *target))))
                && !(allow_unresolved && anchor.is_none()) {
                return false;
            }
        } else {
        let mut chosen_player = used_by_group.players.get(&relation.group).copied();
        for target in selected {
            let Some((_, player)) = relation
                .target_players
                .iter()
                .find(|(candidate, _)| candidate == target)
            else {
                return false;
            };
            if chosen_player.is_some_and(|chosen| chosen != *player) {
                return false;
            }
            chosen_player = Some(*player);
        }
        }
    }
    // CR 115.3: one instance of "target" cannot select an object/player
    // twice, even without an authored cross-requirement distinctness group.
    // Separate requirements may still reuse a target unless grouped.
    let already_used = req
        .distinct_player_group
        .and_then(|group| used_by_group.distinct.get(&group));
    let mut selected_in_requirement = HashSet::new();

    selected.iter().all(|target| {
        selected_in_requirement.insert(*target)
            && !already_used.is_some_and(|used| used.contains(target))
    })
}

fn add_distinct_player_group_targets(
    req: &TargetRequirementContext,
    selected: &[Target],
    used_by_group: &mut TargetGroupSelections,
) {
    if let Some(relation) = &req.shared_player_group {
        if relation.pair_constraint.is_none() {
        for target in selected {
            if let Some((_, player)) = relation
                .target_players
                .iter()
                .find(|(candidate, _)| candidate == target)
            {
                used_by_group.players.insert(relation.group, *player);
            }
        }
        }
    }
    used_by_group.selected_by_requirement.push(selected.to_vec());
    if let Some(group) = req.distinct_player_group {
        used_by_group
            .distinct
            .entry(group)
            .or_default()
            .extend(selected.iter().copied());
    }
}

fn assign_target_counts(
    requirements: &[TargetRequirementContext],
    targets: &[Target],
    allow_autofill: bool,
) -> Option<Vec<usize>> {
    fn recurse(
        requirements: &[TargetRequirementContext],
        targets: &[Target],
        req_idx: usize,
        cursor: usize,
        allow_autofill: bool,
        used_by_group: &mut TargetGroupSelections,
    ) -> Option<Vec<usize>> {
        if req_idx == requirements.len() {
            if cursor == targets.len() {
                return Some(Vec::new());
            } else {
                return None;
            }
        }

        let req = &requirements[req_idx];
        let remaining = targets.len().saturating_sub(cursor);
        let future_min: usize = if allow_autofill {
            0
        } else {
            requirements[req_idx + 1..]
                .iter()
                .map(|next| next.min_targets)
                .sum()
        };
        let min_for_req = if allow_autofill { 0 } else { req.min_targets };
        let max_for_req = req.max_targets.unwrap_or(remaining).min(remaining);

        if min_for_req <= max_for_req {
            for count in (min_for_req..=max_for_req).rev() {
                if remaining.saturating_sub(count) < future_min {
                    continue;
                }

                let slice = &targets[cursor..cursor + count];
                if !selected_targets_satisfy_requirement(req, slice)
                    || !selected_targets_satisfy_distinct_player_group(req, slice, used_by_group, allow_autofill)
                {
                    continue;
                }

                let previous_groups = used_by_group.clone();
                add_distinct_player_group_targets(req, slice, used_by_group);
                let result = recurse(
                    requirements,
                    targets,
                    req_idx + 1,
                    cursor + count,
                    allow_autofill,
                    used_by_group,
                );
                if let Some(mut rest) = result {
                    let mut counts = Vec::with_capacity(rest.len() + 1);
                    counts.push(count);
                    counts.append(&mut rest);
                    return Some(counts);
                }
                *used_by_group = previous_groups;
            }
        }

        None
    }

    let mut used_by_group = TargetGroupSelections::default();
    recurse(
        requirements,
        targets,
        0,
        0,
        allow_autofill,
        &mut used_by_group,
    )
}

pub fn normalize_targets_for_requirements(
    requirements: &[TargetRequirementContext],
    proposed: Vec<Target>,
) -> Option<Vec<Target>> {
    let counts = assign_target_counts(requirements, &proposed, true)?;
    // A chooser may provide one target for a repeated `that player`-style
    // reference while the lowered program exposes two compatible target
    // requirements.  When autofilling the second requirement, preserve the
    // chooser's selection as the first preference instead of silently
    // replacing it with the first legal player.
    let proposed_preference = proposed.clone();
    let mut out = Vec::new();
    let mut cursor = 0usize;
    let mut used_by_group = TargetGroupSelections::default();

    for (req, count) in requirements.iter().zip(counts.into_iter()) {
        let mut selected = Vec::new();
        for target in &proposed[cursor..cursor + count] {
            if !selected.contains(target) {
                selected.push(*target);
            }
        }
        cursor += count;

        if selected.len() < req.min_targets {
            let legal_pool = legal_pool_for_selected(req, &selected)?;
            let mut ordered_pool = proposed_preference
                .iter()
                .filter(|target| legal_pool.contains(target))
                .copied()
                .collect::<Vec<_>>();
            for legal in legal_pool.iter().copied() {
                if !ordered_pool.contains(&legal) {
                    ordered_pool.push(legal);
                }
            }
            for legal in &ordered_pool {
                if selected.len() >= req.min_targets {
                    break;
                }
                let aggregate_allows_extension =
                    req.aggregate_constraint.as_ref().is_none_or(|constraint| {
                        let mut extended = selected.clone();
                        extended.push(*legal);
                        constraint.allows(&extended)
                    });
                if !selected.contains(legal)
                    && aggregate_allows_extension
                    && selected_targets_satisfy_distinct_player_group(
                        req,
                        std::slice::from_ref(legal),
                        &used_by_group,
                        false,
                    )
                {
                    selected.push(*legal);
                }
            }
        }

        if selected.len() < req.min_targets {
            return None;
        }
        if let Some(max) = req.max_targets
            && selected.len() > max
        {
            return None;
        }
        if !selected_targets_satisfy_distinct_player_group(req, &selected, &used_by_group, false) {
            return None;
        }
        add_distinct_player_group_targets(req, &selected, &mut used_by_group);

        out.extend(selected);
    }

    Some(out)
}

pub fn assigned_target_ranges(
    requirements: &[TargetRequirementContext],
    assigned: &[Target],
) -> Option<Vec<Range<usize>>> {
    let counts = assign_target_counts(requirements, assigned, false)?;
    let mut cursor = 0usize;
    let mut ranges = Vec::with_capacity(counts.len());

    for count in counts {
        let end = cursor + count;
        ranges.push(cursor..end);
        cursor = end;
    }

    Some(ranges)
}

/// Recover the target-slot ranges from requirement arity without requiring the
/// old targets to remain legal. This is used when changing a spell's targets:
/// the copied or existing assignment was legal when announced, but one or more
/// of those targets may have become illegal before the retarget effect resolves.
pub fn assigned_target_ranges_ignoring_current_legality(
    requirements: &[TargetRequirementContext],
    assigned: &[Target],
) -> Option<Vec<Range<usize>>> {
    fn assign_counts(
        requirements: &[TargetRequirementContext],
        req_idx: usize,
        remaining: usize,
    ) -> Option<Vec<usize>> {
        if req_idx == requirements.len() {
            return (remaining == 0).then(Vec::new);
        }

        let requirement = &requirements[req_idx];
        let future_min = requirements[req_idx + 1..]
            .iter()
            .map(|next| next.min_targets)
            .sum::<usize>();
        let max = requirement.max_targets.unwrap_or(remaining).min(remaining);

        for count in (requirement.min_targets..=max).rev() {
            let after = remaining.saturating_sub(count);
            if after < future_min {
                continue;
            }
            if let Some(mut rest) = assign_counts(requirements, req_idx + 1, after) {
                let mut counts = Vec::with_capacity(rest.len() + 1);
                counts.push(count);
                counts.append(&mut rest);
                return Some(counts);
            }
        }

        None
    }

    let counts = assign_counts(requirements, 0, assigned.len())?;
    let mut cursor = 0usize;
    Some(
        counts
            .into_iter()
            .map(|count| {
                let start = cursor;
                cursor += count;
                start..cursor
            })
            .collect(),
    )
}

pub fn validate_flat_target_assignment(
    requirements: &[TargetRequirementContext],
    targets: &[Target],
) -> bool {
    assign_target_counts(requirements, targets, false).is_some()
}

#[cfg(test)]
mod tests {
    use super::{
        assigned_target_ranges, assigned_target_ranges_ignoring_current_legality,
        normalize_targets_for_requirements, validate_flat_target_assignment,
    };
    use crate::decisions::context::TargetRequirementContext;
    use crate::game_state::Target;
    use crate::ids::ObjectId;

    #[test]
    fn normalize_targets_preserves_unbounded_requirement_prefix() {
        let a = Target::Object(ObjectId::from_raw(1));
        let b = Target::Object(ObjectId::from_raw(2));
        let c = Target::Object(ObjectId::from_raw(3));
        let d = Target::Object(ObjectId::from_raw(4));
        let requirements = vec![
            TargetRequirementContext {
                description: "any number".to_string(),
                legal_targets: vec![a, b, c],
                legal_target_sets: Vec::new(),
                aggregate_constraint: None,
                min_targets: 0,
                max_targets: None,
                distinct_player_group: None,
                shared_player_group: None,
            },
            TargetRequirementContext {
                description: "final target".to_string(),
                legal_targets: vec![d],
                legal_target_sets: Vec::new(),
                aggregate_constraint: None,
                min_targets: 1,
                max_targets: Some(1),
                distinct_player_group: None,
                shared_player_group: None,
            },
        ];

        let normalized =
            normalize_targets_for_requirements(&requirements, vec![a, b, d]).expect("valid");

        assert_eq!(normalized, vec![a, b, d]);
        assert_eq!(
            assigned_target_ranges(&requirements, &normalized).expect("ranges"),
            vec![0..2, 2..3]
        );
    }

    #[test]
    fn normalize_targets_can_autofill_required_target_after_empty_input() {
        let a = Target::Object(ObjectId::from_raw(1));
        let requirements = vec![TargetRequirementContext {
            description: "required".to_string(),
            legal_targets: vec![a],
            legal_target_sets: Vec::new(),
            aggregate_constraint: None,
            min_targets: 1,
            max_targets: Some(1),
            distinct_player_group: None,
            shared_player_group: None,
        }];

        let normalized =
            normalize_targets_for_requirements(&requirements, Vec::new()).expect("valid");

        assert_eq!(normalized, vec![a]);
    }

    #[test]
    fn validate_flat_assignment_rejects_reversed_requirement_order() {
        let a = Target::Object(ObjectId::from_raw(1));
        let b = Target::Object(ObjectId::from_raw(2));
        let requirements = vec![
            TargetRequirementContext {
                description: "first".to_string(),
                legal_targets: vec![a],
                legal_target_sets: Vec::new(),
                aggregate_constraint: None,
                min_targets: 1,
                max_targets: Some(1),
                distinct_player_group: None,
                shared_player_group: None,
            },
            TargetRequirementContext {
                description: "second".to_string(),
                legal_targets: vec![b],
                legal_target_sets: Vec::new(),
                aggregate_constraint: None,
                min_targets: 1,
                max_targets: Some(1),
                distinct_player_group: None,
                shared_player_group: None,
            },
        ];

        assert!(!validate_flat_target_assignment(&requirements, &[b, a]));
    }

    #[test]
    fn target_ranges_can_be_recovered_after_an_old_target_becomes_illegal() {
        let old_target = Target::Object(ObjectId::from_raw(1));
        let new_legal_target = Target::Object(ObjectId::from_raw(2));
        let requirements = vec![TargetRequirementContext {
            description: "replacement target".to_string(),
            legal_targets: vec![new_legal_target],
            legal_target_sets: Vec::new(),
            aggregate_constraint: None,
            min_targets: 1,
            max_targets: Some(1),
            distinct_player_group: None,
            shared_player_group: None,
        }];

        assert!(assigned_target_ranges(&requirements, &[old_target]).is_none());
        assert_eq!(
            assigned_target_ranges_ignoring_current_legality(&requirements, &[old_target]),
            Some(std::iter::once(0..1).collect())
        );
    }

    #[test]
    fn grouped_requirement_rejects_mixed_target_sets() {
        let a = Target::Object(ObjectId::from_raw(1));
        let b = Target::Object(ObjectId::from_raw(2));
        let c = Target::Object(ObjectId::from_raw(3));
        let d = Target::Object(ObjectId::from_raw(4));
        let requirements = vec![TargetRequirementContext {
            description: "same controller targets".to_string(),
            legal_targets: vec![a, b, c, d],
            legal_target_sets: vec![vec![a, b], vec![c, d]],
            aggregate_constraint: None,
            min_targets: 2,
            max_targets: Some(2),
            distinct_player_group: None,
            shared_player_group: None,
        }];

        assert!(validate_flat_target_assignment(&requirements, &[a, b]));
        assert!(!validate_flat_target_assignment(&requirements, &[a, c]));
    }

    #[test]
    fn grouped_requirement_autofills_from_selected_group() {
        let a = Target::Object(ObjectId::from_raw(1));
        let b = Target::Object(ObjectId::from_raw(2));
        let c = Target::Object(ObjectId::from_raw(3));
        let d = Target::Object(ObjectId::from_raw(4));
        let requirements = vec![TargetRequirementContext {
            description: "same controller targets".to_string(),
            legal_targets: vec![a, b, c, d],
            legal_target_sets: vec![vec![a, b], vec![c, d]],
            aggregate_constraint: None,
            min_targets: 2,
            max_targets: Some(2),
            distinct_player_group: None,
            shared_player_group: None,
        }];

        let normalized = normalize_targets_for_requirements(&requirements, vec![c]).expect("valid");

        assert_eq!(normalized, vec![c, d]);
    }

    #[test]
    fn aggregate_requirement_rejects_individually_legal_targets_over_the_total() {
        let four = Target::Object(ObjectId::from_raw(1));
        let three = Target::Object(ObjectId::from_raw(2));
        let two = Target::Object(ObjectId::from_raw(3));
        let requirements = vec![TargetRequirementContext {
            description: "total mana value 6 or less".to_string(),
            legal_targets: vec![four, three, two],
            legal_target_sets: Vec::new(),
            aggregate_constraint: Some(crate::targeting::ResolvedTargetAggregateConstraint {
                metric: crate::effect::ChoiceAggregateMetric::ManaValue,
                maximum: 6,
                target_values: vec![(four, 4), (three, 3), (two, 2)],
            }),
            min_targets: 0,
            max_targets: None,
            distinct_player_group: None,
            shared_player_group: None,
        }];

        assert!(!validate_flat_target_assignment(
            &requirements,
            &[four, three]
        ));
        assert!(validate_flat_target_assignment(&requirements, &[four, two]));
    }
}

/// Match a saved target assignment after an announcement-time chooser has
/// specialized "they control" to that particular player. Every other filter
/// constraint and the target count must still agree.
pub(crate) fn target_spec_matches_chooser_assignment(
    declared: &crate::target::ChooseSpec,
    assigned: &crate::target::ChooseSpec,
) -> bool {
    use crate::target::{ChooseSpec, PlayerFilter};
    if declared.count() != assigned.count() || declared.count_value() != assigned.count_value() {
        return false;
    }
    let (ChooseSpec::Object(declared), ChooseSpec::Object(assigned)) =
        (declared.base(), assigned.base())
    else {
        return false;
    };
    if declared.controller != Some(PlayerFilter::IteratedPlayer)
        || !matches!(assigned.controller, Some(PlayerFilter::Specific(_)))
    {
        return false;
    }
    let mut restored = assigned.clone();
    restored.controller = Some(PlayerFilter::IteratedPlayer);
    declared == &restored
}

#[cfg(test)]
mod chooser_assignment_tests {
    use super::*;
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    #[test]
    fn chooser_assignment_requires_same_filter_and_count() {
        let declared = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
        ));
        let assigned = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature()
                .controlled_by(PlayerFilter::Specific(crate::ids::PlayerId::from_index(2))),
        ));
        assert!(target_spec_matches_chooser_assignment(&declared, &assigned));
        let different_kind = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::artifact()
                .controlled_by(PlayerFilter::Specific(crate::ids::PlayerId::from_index(2))),
        ));
        assert!(!target_spec_matches_chooser_assignment(
            &declared,
            &different_kind
        ));
        let different_count = ChooseSpec::WithCount(
            Box::new(assigned.clone()),
            crate::effect::ChoiceCount::exactly(2),
        );
        assert!(!target_spec_matches_chooser_assignment(
            &declared,
            &different_count
        ));
        let unspecified = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        ));
        assert!(!target_spec_matches_chooser_assignment(
            &declared,
            &unspecified
        ));
        assert!(!target_spec_matches_chooser_assignment(
            &assigned, &declared
        ));
    }
}

#[cfg(test)]
mod prior_target_pair_constraint_tests {
    use super::*;
    use crate::decisions::context::{SharedTargetPlayerGroup,TargetPairConstraint};
    fn t(n:u64)->Target {Target::Object(crate::ids::ObjectId::from_raw(n))}
    fn pair(prior:usize,allowed:Vec<(Target,Target)>)->SharedTargetPlayerGroup {
        SharedTargetPlayerGroup{group:0,target_players:vec![],pair_constraint:Some(TargetPairConstraint{
            prior_requirement:prior,allowed_pairs:allowed})}
    }
    fn requirements()->Vec<TargetRequirementContext> {
        let source=TargetRequirementContext::single("source",vec![t(1),t(2),t(3)]);
        let mut destination=TargetRequirementContext::single("destination",vec![t(1),t(2),t(3)]);
        // Three controller classes: exclusion permits either of the other two.
        destination.shared_player_group=Some(pair(0,vec![(t(1),t(2)),(t(1),t(3)),
            (t(2),t(1)),(t(2),t(3)),(t(3),t(1)),(t(3),t(2))]));
        vec![source,destination]
    }
    #[test]
    fn excluded_controller_pair_accepts_both_other_player_classes() {
        let req=requirements();
        for source in [t(1),t(2),t(3)] {for to in [t(1),t(2),t(3)] {
            assert_eq!(validate_flat_target_assignment(&req,&[source,to]),source!=to);
        }}
    }
    #[test]
    fn pair_dependency_normalization_fills_missing_roles_before_validation() {
        let req=requirements();
        for partial in [vec![],vec![t(1)],vec![t(3)]] {
            let result=normalize_targets_for_requirements(&req,partial).expect("fillable pair");
            assert_eq!(result.len(),2);
            assert!(validate_flat_target_assignment(&req,&result));
            assert_ne!(result[0],result[1]);
        }
    }
    #[test]
    fn empty_prior_role_cannot_borrow_an_unrelated_selection() {
        let unrelated=TargetRequirementContext::single("unrelated",vec![t(1)]);
        let mut source=TargetRequirementContext::single("empty source",vec![]);
        source.min_targets=0;source.max_targets=Some(0);
        let mut destination=TargetRequirementContext::single("destination",vec![t(2)]);
        destination.shared_player_group=Some(pair(1,vec![(t(1),t(2))]));
        let req=vec![unrelated,source,destination];
        assert!(!validate_flat_target_assignment(&req,&[t(1),t(2)]));
        assert!(normalize_targets_for_requirements(&req,vec![t(1),t(2)]).is_none());
    }
    #[test]
    fn chained_pair_constraints_bind_each_authored_prior_role() {
        let mut req=requirements();
        let mut third=TargetRequirementContext::single("third",vec![t(1),t(2),t(3)]);
        third.shared_player_group=Some(pair(1,vec![(t(1),t(1)),(t(2),t(2)),(t(3),t(3))]));
        req.push(third);
        assert!(validate_flat_target_assignment(&req,&[t(1),t(2),t(2)]));
        assert!(!validate_flat_target_assignment(&req,&[t(1),t(2),t(1)]));
        assert!(!validate_flat_target_assignment(&req,&[t(1),t(1),t(1)]));
    }
}
