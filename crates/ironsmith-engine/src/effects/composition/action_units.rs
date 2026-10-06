//! Shared authored action-unit layout for compound schedulers.
//!
//! Layout describes ordering, scope boundaries and read-only selection preludes.
//! Actual proposals, original receipts and completions remain with their owners.

use crate::effect::Effect;

/// Partition an ordered program without executing or preparing its children.
/// Begin markers feed the following unit; end markers finish the preceding
/// unit and own a boundary. Scope changes cannot carry an unfinished prelude
/// into a different scope. Read-only preludes feed the following action, while
/// a mutator always ends its unit. Later consumers prepare only after its
/// completed result and tags have been published for every participant.
pub(super) fn partition_action_units(
    effects: &[Effect],
    marker: impl Fn(usize) -> Option<bool>,
    same_scope: impl Fn(usize, usize) -> bool,
) -> Vec<Vec<usize>> {
    let mut units = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    for (index, effect) in effects.iter().enumerate() {
        if let Some(begin) = marker(index) {
            if begin {
                current.push(index);
            } else {
                if !current.is_empty() {
                    units.push(std::mem::take(&mut current));
                }
                units.push(vec![index]);
            }
            continue;
        }
        if current
            .last()
            .is_some_and(|previous| marker(*previous).is_none() && !same_scope(*previous, index))
        {
            units.push(std::mem::take(&mut current));
        }
        current.push(index);
        if effect.0.is_read_only_simultaneous_player_action() {
            continue;
        }

        units.push(std::mem::take(&mut current));
    }
    if !current.is_empty() {
        units.push(current);
    }
    units
}
