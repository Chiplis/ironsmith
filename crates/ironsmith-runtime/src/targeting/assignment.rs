use std::collections::HashMap;
use std::ops::Range;

use crate::decisions::context::TargetRequirementContext;
use crate::game_state::Target;

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
        memo: &mut HashMap<(usize, usize), Option<Vec<usize>>>,
    ) -> Option<Vec<usize>> {
        if let Some(cached) = memo.get(&(req_idx, cursor)) {
            return cached.clone();
        }

        let result = if req_idx == requirements.len() {
            if cursor == targets.len() {
                Some(Vec::new())
            } else {
                None
            }
        } else {
            let req = &requirements[req_idx];
            let remaining = targets.len().saturating_sub(cursor);
            let future_min: usize = requirements[req_idx + 1..]
                .iter()
                .map(|next| next.min_targets)
                .sum();
            let min_for_req = if allow_autofill { 0 } else { req.min_targets };
            let max_for_req = req.max_targets.unwrap_or(remaining).min(remaining);
            let mut found = None;

            if min_for_req <= max_for_req {
                for count in (min_for_req..=max_for_req).rev() {
                    if remaining.saturating_sub(count) < future_min {
                        continue;
                    }

                    let slice = &targets[cursor..cursor + count];
                    if slice
                        .iter()
                        .any(|target| !req.legal_targets.contains(target))
                    {
                        continue;
                    }

                    if let Some(mut rest) = recurse(
                        requirements,
                        targets,
                        req_idx + 1,
                        cursor + count,
                        allow_autofill,
                        memo,
                    ) {
                        let mut counts = Vec::with_capacity(rest.len() + 1);
                        counts.push(count);
                        counts.append(&mut rest);
                        found = Some(counts);
                        break;
                    }
                }
            }

            found
        };

        memo.insert((req_idx, cursor), result.clone());
        result
    }

    let mut memo = HashMap::new();
    recurse(requirements, targets, 0, 0, allow_autofill, &mut memo)
}

pub fn normalize_targets_for_requirements(
    requirements: &[TargetRequirementContext],
    proposed: Vec<Target>,
) -> Option<Vec<Target>> {
    let counts = assign_target_counts(requirements, &proposed, true)?;
    let mut out = Vec::new();
    let mut cursor = 0usize;

    for (req, count) in requirements.iter().zip(counts.into_iter()) {
        let mut selected = Vec::new();
        for target in &proposed[cursor..cursor + count] {
            if !selected.contains(target) {
                selected.push(*target);
            }
        }
        cursor += count;

        if selected.len() < req.min_targets {
            for legal in &req.legal_targets {
                if selected.len() >= req.min_targets {
                    break;
                }
                if !selected.contains(legal) {
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

pub fn validate_flat_target_assignment(
    requirements: &[TargetRequirementContext],
    targets: &[Target],
) -> bool {
    assign_target_counts(requirements, targets, false).is_some()
}

#[cfg(test)]
mod tests {
    use super::{
        assigned_target_ranges, normalize_targets_for_requirements, validate_flat_target_assignment,
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
                min_targets: 0,
                max_targets: None,
            },
            TargetRequirementContext {
                description: "final target".to_string(),
                legal_targets: vec![d],
                min_targets: 1,
                max_targets: Some(1),
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
            min_targets: 1,
            max_targets: Some(1),
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
                min_targets: 1,
                max_targets: Some(1),
            },
            TargetRequirementContext {
                description: "second".to_string(),
                legal_targets: vec![b],
                min_targets: 1,
                max_targets: Some(1),
            },
        ];

        assert!(!validate_flat_target_assignment(&requirements, &[b, a]));
    }
}
