//! Shared allocation feasibility for selected discard payment slots.

use crate::ids::ObjectId;

/// Match each requested discard to a different candidate. Reassign earlier
/// slots when filters overlap; do not depend on greedy candidate order.
pub(crate) fn distinct_discard_assignment_exists(slots: &[Vec<ObjectId>]) -> bool {
    fn assign(
        slot: usize,
        slots: &[Vec<ObjectId>],
        owners: &mut std::collections::HashMap<ObjectId, usize>,
        visited: &mut std::collections::HashSet<ObjectId>,
    ) -> bool {
        for &card in &slots[slot] {
            if !visited.insert(card) {
                continue;
            }
            let previous = owners.get(&card).copied();
            if previous.is_none_or(|other| assign(other, slots, owners, visited)) {
                owners.insert(card, slot);
                return true;
            }
        }
        false
    }
    let mut owners = std::collections::HashMap::new();
    (0..slots.len()).all(|slot| {
        assign(
            slot,
            slots,
            &mut owners,
            &mut std::collections::HashSet::new(),
        )
    })
}
