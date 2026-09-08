//! Persistent ordered zone membership with explicit incremental changes.
use crate::ids::ObjectId;
use crate::incremental::{ChangeCursor, ChangeJournal};
use std::ops::Deref;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZoneChange {
    Insert {
        index: usize,
        id: ObjectId,
    },
    Remove {
        index: usize,
        id: ObjectId,
    },
    /// A bulk permutation/replacement needs rebuilding order, not object views.
    Reorder,
}

#[derive(Debug, Default)]
pub struct ZoneSequence {
    ids: im::Vector<ObjectId>,
    order_labels: im::Vector<u128>,
    positions: im::HashMap<ObjectId, im::OrdSet<u128>>,
    changes: ChangeJournal<ZoneChange>,
    contiguous: OnceLock<Vec<ObjectId>>,
}

impl Clone for ZoneSequence {
    fn clone(&self) -> Self {
        Self {
            ids: self.ids.clone(),
            order_labels: self.order_labels.clone(),
            positions: self.positions.clone(),
            changes: self.changes.clone(),
            contiguous: OnceLock::new(),
        }
    }
}
impl PartialEq for ZoneSequence {
    fn eq(&self, rhs: &Self) -> bool {
        self.ids == rhs.ids
    }
}
impl Eq for ZoneSequence {}
impl PartialEq<Vec<ObjectId>> for ZoneSequence {
    fn eq(&self, rhs: &Vec<ObjectId>) -> bool {
        self.iter().eq(rhs.iter())
    }
}
impl PartialEq<ZoneSequence> for Vec<ObjectId> {
    fn eq(&self, rhs: &ZoneSequence) -> bool {
        self.iter().eq(rhs.iter())
    }
}
impl ZoneSequence {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.ids.len()
    }
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
    pub fn iter(&self) -> im::vector::Iter<'_, ObjectId> {
        self.ids.iter()
    }
    pub fn first(&self) -> Option<&ObjectId> {
        self.ids.front()
    }
    pub fn last(&self) -> Option<&ObjectId> {
        self.ids.back()
    }
    pub fn get(&self, index: usize) -> Option<&ObjectId> {
        self.ids.get(index)
    }
    pub fn contains(&self, id: &ObjectId) -> bool {
        self.positions.contains_key(id)
    }
    pub fn cursor(&self) -> ChangeCursor {
        self.changes.cursor()
    }
    pub fn changes_since(&self, cursor: &ChangeCursor) -> Option<Vec<ZoneChange>> {
        self.changes.since(cursor)
    }
    pub fn to_vec(&self) -> Vec<ObjectId> {
        self.iter().copied().collect()
    }
    pub fn as_slice(&self) -> &[ObjectId] {
        self.contiguous.get_or_init(|| self.to_vec())
    }
    fn record(&mut self, change: ZoneChange) {
        self.contiguous.take();
        self.changes.record(change);
    }
    fn rebuild_positions(&mut self) {
        self.order_labels.clear();
        self.positions.clear();
        for (index, id) in self.ids.iter().copied().enumerate() {
            let label = ((index as u128) + 1) << 64;
            self.order_labels.push_back(label);
            self.positions.entry(id).or_default().insert(label);
        }
    }
    fn forget_position(&mut self, id: ObjectId, label: u128) {
        let empty = if let Some(labels) = self.positions.get_mut(&id) {
            labels.remove(&label);
            labels.is_empty()
        } else {
            false
        };
        if empty {
            self.positions.remove(&id);
        }
    }
    fn index_of_label(&self, label: u128) -> usize {
        let (mut low, mut high) = (0, self.len());
        while low < high {
            let middle = low + (high - low) / 2;
            if self.order_labels[middle] < label {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        debug_assert_eq!(self.order_labels.get(low), Some(&label));
        low
    }
    /// Locate a known object without scanning the zone. Labels remain stable
    /// as surrounding members leave, so rank lookup is logarithmic in tree depth.
    pub fn position_of(&self, id: ObjectId) -> Option<usize> {
        self.positions
            .get(&id)
            .and_then(|labels| labels.get_min())
            .map(|label| self.index_of_label(*label))
    }
    /// Remove all occurrences, preserving the old retain-by-ID semantics even
    /// for a malformed duplicate membership supplied by an embedding caller.
    pub fn remove_id(&mut self, id: ObjectId) -> bool {
        let labels: Vec<_> = self
            .positions
            .get(&id)
            .into_iter()
            .flat_map(|labels| labels.iter().copied())
            .collect();
        let changed = !labels.is_empty();
        for label in labels {
            self.remove(self.index_of_label(label));
        }
        changed
    }
    pub fn push(&mut self, id: ObjectId) {
        self.insert(self.len(), id);
    }
    pub fn pop(&mut self) -> Option<ObjectId> {
        let id = self.ids.pop_back()?;
        let label = self.order_labels.pop_back().expect("parallel zone labels");
        self.forget_position(id, label);
        self.record(ZoneChange::Remove {
            index: self.len(),
            id,
        });
        Some(id)
    }
    pub fn insert(&mut self, index: usize, id: ObjectId) {
        assert!(index <= self.len());
        let bounds = |labels: &im::Vector<u128>| {
            (
                index
                    .checked_sub(1)
                    .and_then(|i| labels.get(i))
                    .copied()
                    .unwrap_or(0),
                labels.get(index).copied().unwrap_or(u128::MAX),
            )
        };
        let (mut left, mut right) = bounds(&self.order_labels);
        if right - left <= 1 {
            self.rebuild_positions();
            (left, right) = bounds(&self.order_labels);
        }
        let label = left + (right - left).min(1u128 << 64) / 2;
        self.ids.insert(index, id);
        self.order_labels.insert(index, label);
        self.positions.entry(id).or_default().insert(label);
        self.record(ZoneChange::Insert { index, id });
    }
    pub fn remove(&mut self, index: usize) -> ObjectId {
        let id = self.ids.remove(index);
        let label = self.order_labels.remove(index);
        self.forget_position(id, label);
        self.record(ZoneChange::Remove { index, id });
        id
    }
    pub fn retain(&mut self, mut keep: impl FnMut(&ObjectId) -> bool) {
        let removed: Vec<_> = self
            .iter()
            .enumerate()
            .filter_map(|(i, id)| (!keep(id)).then_some(i))
            .collect();
        for index in removed.into_iter().rev() {
            self.remove(index);
        }
    }
    pub fn clear(&mut self) {
        if !self.is_empty() {
            self.ids.clear();
            self.order_labels.clear();
            self.positions.clear();
            self.record(ZoneChange::Reorder);
        }
    }
    pub fn truncate(&mut self, len: usize) {
        while self.len() > len {
            self.pop();
        }
    }
    /// Bulk algorithms (shuffle, sort, splice) explicitly invalidate order.
    pub fn with_vec_mut<R>(&mut self, edit: impl FnOnce(&mut Vec<ObjectId>) -> R) -> R {
        let mut ids = self.to_vec();
        let result = edit(&mut ids);
        self.ids = ids.into_iter().collect();
        self.rebuild_positions();
        self.record(ZoneChange::Reorder);
        result
    }
    pub fn swap(&mut self, a: usize, b: usize) {
        if a == b {
            assert!(a < self.len());
            return;
        }
        let (a, b) = (a.min(b), a.max(b));
        let (left, right) = (self.ids[a], self.ids[b]);
        self.remove(b);
        self.remove(a);
        self.insert(a, right);
        self.insert(b, left);
    }
    pub fn reverse(&mut self) {
        self.with_vec_mut(|ids| ids.reverse());
    }
    pub fn sort_unstable(&mut self) {
        self.with_vec_mut(|ids| ids.sort_unstable());
    }
    pub fn extend_from_slice(&mut self, ids: &[ObjectId]) {
        self.extend(ids.iter().copied());
    }
}
impl Deref for ZoneSequence {
    type Target = [ObjectId];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
impl From<Vec<ObjectId>> for ZoneSequence {
    fn from(ids: Vec<ObjectId>) -> Self {
        ids.into_iter().collect()
    }
}
impl FromIterator<ObjectId> for ZoneSequence {
    fn from_iter<T: IntoIterator<Item = ObjectId>>(iter: T) -> Self {
        let mut zone = Self {
            ids: iter.into_iter().collect(),
            ..Self::default()
        };
        zone.rebuild_positions();
        zone
    }
}
impl Extend<ObjectId> for ZoneSequence {
    fn extend<T: IntoIterator<Item = ObjectId>>(&mut self, iter: T) {
        for id in iter {
            self.push(id);
        }
    }
}
impl<'a> IntoIterator for &'a ZoneSequence {
    type Item = &'a ObjectId;
    type IntoIter = im::vector::Iter<'a, ObjectId>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl IntoIterator for ZoneSequence {
    type Item = ObjectId;
    type IntoIter = im::vector::ConsumingIter<ObjectId>;
    fn into_iter(self) -> Self::IntoIter {
        self.ids.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(n: u64) -> ObjectId {
        ObjectId::from_raw(n)
    }

    #[test]
    fn mutations_match_vec_and_emit_replayable_ordered_deltas() {
        let mut zone: ZoneSequence = (0..200).map(id).collect();
        let mut reference = zone.to_vec();
        let cursor = zone.cursor();
        zone.push(id(300));
        reference.push(id(300));
        zone.insert(63, id(301));
        reference.insert(63, id(301));
        zone.remove(100);
        reference.remove(100);
        zone.retain(|id| id.0 % 7 != 0);
        reference.retain(|id| id.0 % 7 != 0);
        zone.pop();
        reference.pop();
        assert_eq!(zone.to_vec(), reference);
        let mut replay: Vec<_> = (0..200).map(id).collect();
        for change in zone.changes_since(&cursor).unwrap() {
            match change {
                ZoneChange::Insert { index, id } => replay.insert(index, id),
                ZoneChange::Remove { index, id } => assert_eq!(replay.remove(index), id),
                ZoneChange::Reorder => panic!("local operations must emit local changes"),
            }
        }
        assert_eq!(replay, reference);
    }

    #[test]
    fn slices_and_forks_do_not_observe_sibling_changes() {
        let mut original: ZoneSequence = (0..100).map(id).collect();
        assert_eq!(original.as_slice().len(), 100);
        let mut fork = original.clone();
        let fork_cursor = fork.cursor();
        original.push(id(100));
        fork.with_vec_mut(|ids| ids.reverse());
        assert_eq!(original.first(), Some(&id(0)));
        assert_eq!(fork.first(), Some(&id(99)));
        assert_eq!(original.as_slice().last(), Some(&id(100)));
        assert_eq!(
            fork.changes_since(&fork_cursor),
            Some(vec![ZoneChange::Reorder])
        );
        assert_eq!(original.changes_since(&fork.cursor()), None);
    }
}

/// Stable order labels for a derived per-object index. Ordinary removals and
/// appends never renumber surviving members. Sparse labels also handle inserts
/// into reserved zone positions; only exhausted gaps require relabeling.
#[derive(Clone, Debug, Default)]
pub struct ZoneOrder {
    sequence: im::Vector<ObjectId>,
    cursor: Option<ChangeCursor>,
    ordered: im::OrdMap<u128, ObjectId>,
    labels: im::HashMap<ObjectId, u128>,
}

impl ZoneOrder {
    pub fn label(&self, id: ObjectId) -> Option<u128> {
        self.labels.get(&id).copied()
    }
    pub fn iter(&self) -> impl Iterator<Item = (u128, ObjectId)> + '_ {
        self.ordered.iter().map(|(label, id)| (*label, *id))
    }
    /// `None` means labels were rebuilt; otherwise only these memberships changed.
    pub fn synchronize(&mut self, zone: &ZoneSequence) -> Option<Vec<ObjectId>> {
        let changes = self
            .cursor
            .as_ref()
            .and_then(|cursor| zone.changes_since(cursor));
        let mut touched = Vec::new();
        let mut rebuild = changes.is_none();
        if let Some(changes) = changes {
            for change in changes {
                match change {
                    ZoneChange::Remove { id, index } => {
                        let removed = self.sequence.remove(index);
                        debug_assert_eq!(removed, id);
                        if let Some(label) = self.labels.remove(&id) {
                            self.ordered.remove(&label);
                        }
                        touched.push(id);
                    }
                    ZoneChange::Insert { index, id } => {
                        let (left, right) = if index == self.ordered.len() {
                            (
                                self.ordered.get_max().map(|(k, _)| *k).unwrap_or(0),
                                u128::MAX,
                            )
                        } else {
                            let right = self
                                .sequence
                                .get(index)
                                .and_then(|id| self.labels.get(id))
                                .copied();
                            let left = index
                                .checked_sub(1)
                                .and_then(|i| {
                                    self.sequence
                                        .get(i)
                                        .and_then(|id| self.labels.get(id))
                                        .copied()
                                })
                                .unwrap_or(0);
                            (left, right.unwrap_or(u128::MAX))
                        };
                        let gap = right.saturating_sub(left);
                        if gap <= 1 || self.labels.contains_key(&id) {
                            rebuild = true;
                            break;
                        }
                        // Leave ample room at the tail rather than repeatedly
                        // halving the remaining u128 space on token storms.
                        let label = left + gap.min(1u128 << 64) / 2;
                        self.sequence.insert(index, id);
                        self.labels.insert(id, label);
                        self.ordered.insert(label, id);
                        touched.push(id);
                    }
                    ZoneChange::Reorder => {
                        rebuild = true;
                        break;
                    }
                }
            }
        }
        if rebuild {
            self.sequence = zone.ids.clone();
            self.ordered.clear();
            self.labels.clear();
            for (i, id) in zone.iter().copied().enumerate() {
                let label = ((i as u128) + 1) << 64;
                self.labels.insert(id, label);
                self.ordered.insert(label, id);
            }
        }
        self.cursor = Some(zone.cursor());
        if rebuild { None } else { Some(touched) }
    }
}

#[cfg(test)]
mod order_tests {
    use super::*;

    #[test]
    fn sparse_order_matches_zone_through_inserts_removals_reorder_and_forks() {
        let mut zone = ZoneSequence::new();
        let mut order = ZoneOrder::default();
        assert!(order.synchronize(&zone).is_none());
        for n in 0..300 {
            let id = ObjectId::from_raw(n);
            match n % 4 {
                0 => zone.push(id),
                1 => zone.insert(0, id),
                2 => zone.insert(zone.len() / 2, id),
                _ => {
                    if !zone.is_empty() {
                        zone.remove(zone.len() / 2);
                    }
                }
            }
            order.synchronize(&zone);
            assert_eq!(
                order.iter().map(|(_, id)| id).collect::<Vec<_>>(),
                zone.to_vec()
            );
        }
        let saved = zone.clone();
        zone.reverse();
        assert!(order.synchronize(&zone).is_none());
        assert_eq!(
            order.iter().map(|(_, id)| id).collect::<Vec<_>>(),
            zone.to_vec()
        );
        zone = saved;
        zone.push(ObjectId::from_raw(1000));
        assert!(order.synchronize(&zone).is_none());
        assert_eq!(
            order.iter().map(|(_, id)| id).collect::<Vec<_>>(),
            zone.to_vec()
        );
    }
}

#[cfg(test)]
mod position_tests {
    use super::*;
    #[test]
    fn membership_positions_follow_swaps_bulk_edits_and_duplicate_removal() {
        let mut zone: ZoneSequence = (0..256).map(ObjectId::from_raw).collect();
        let mut expected = zone.to_vec();
        for n in 0..400usize {
            let index = n % (zone.len() + 1);
            let id = ObjectId::from_raw((n % 31) as u64);
            zone.insert(index, id);
            expected.insert(index, id);
            if n % 3 == 0 {
                let a = n % zone.len();
                let b = (n * 7) % zone.len();
                zone.swap(a, b);
                expected.swap(a, b);
            }
            if n % 5 == 0 {
                zone.remove_id(id);
                expected.retain(|candidate| *candidate != id);
            }
            assert_eq!(zone.to_vec(), expected);
            for value in [0, 7, 19, 255, 999] {
                let id = ObjectId::from_raw(value);
                assert_eq!(zone.contains(&id), expected.contains(&id));
                assert_eq!(
                    zone.position_of(id),
                    expected.iter().position(|candidate| *candidate == id)
                );
            }
        }
        zone.reverse();
        expected.reverse();
        assert_eq!(
            zone.position_of(ObjectId::from_raw(255)),
            expected.iter().position(|id| id.0 == 255)
        );
        let before = zone.clone();
        zone.clear();
        assert!(!zone.contains(&ObjectId::from_raw(255)));
        assert!(before.contains(&ObjectId::from_raw(255)));
    }
}
