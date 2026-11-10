//! Bounded, branch-aware mutation history for derived indexes.
//!
//! Cursors are local cache identities, never gameplay IDs or serialized state.
//! Keeping an old cursor cannot keep an unbounded chain of game states alive.
use std::sync::Arc;

/// A position in one particular history branch.
#[derive(Clone, Debug)]
pub struct ChangeCursor(Arc<()>);

#[derive(Clone, Debug)]
struct Entry<T> {
    before: ChangeCursor,
    value: T,
}

/// A persistent journal with cheap checkpoint clones and bounded retention.
#[derive(Clone, Debug)]
pub struct ChangeJournal<T: Clone, const LIMIT: usize = 4096> {
    current: ChangeCursor,
    entries: im::Vector<Entry<T>>,
}

impl<T: Clone, const LIMIT: usize> Default for ChangeJournal<T, LIMIT> {
    fn default() -> Self {
        Self {
            current: ChangeCursor(Arc::new(())),
            entries: im::Vector::new(),
        }
    }
}

impl<T: Clone, const LIMIT: usize> ChangeJournal<T, LIMIT> {
    pub fn cursor(&self) -> ChangeCursor {
        self.current.clone()
    }

    pub fn record(&mut self, value: T) {
        let before = self.current.clone();
        self.current = ChangeCursor(Arc::new(()));
        if LIMIT != 0 {
            self.entries.push_back(Entry { before, value });
            if self.entries.len() > LIMIT {
                self.entries.pop_front();
            }
        }
    }

    /// Returns only the new changes, in mutation order. `None` requires a
    /// rebuild: the cursor expired, is from another game, or is on a sibling
    /// checkpoint branch. Equal numeric mutation counts cannot alias branches.
    pub fn since(&self, cursor: &ChangeCursor) -> Option<Vec<T>> {
        if Arc::ptr_eq(&self.current.0, &cursor.0) {
            return Some(Vec::new());
        }
        let mut changes = Vec::new();
        for entry in self.entries.iter().rev() {
            changes.push(entry.value.clone());
            if Arc::ptr_eq(&entry.before.0, &cursor.0) {
                changes.reverse();
                return Some(changes);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deltas_preserve_order_and_expire_at_the_retention_boundary() {
        let mut journal = ChangeJournal::<u32, 2>::default();
        let start = journal.cursor();
        assert_eq!(journal.since(&start), Some(vec![]));
        journal.record(1);
        let one = journal.cursor();
        journal.record(2);
        assert_eq!(journal.since(&start), Some(vec![1, 2]));
        journal.record(3);
        assert_eq!(journal.since(&start), None);
        assert_eq!(journal.since(&one), Some(vec![2, 3]));
        assert_eq!(journal.entries.len(), 2);
    }

    #[test]
    fn checkpoint_branches_cannot_alias_and_common_ancestors_work() {
        let mut original = ChangeJournal::<u32>::default();
        original.record(1);
        let ancestor = original.cursor();
        let mut fork = original.clone();
        original.record(2);
        let future = original.cursor();
        fork.record(3);
        assert_eq!(fork.since(&future), None);
        assert_eq!(original.since(&fork.cursor()), None);
        assert_eq!(fork.since(&ancestor), Some(vec![3]));
        assert_eq!(original.since(&ancestor), Some(vec![2]));
        assert_eq!(ChangeJournal::<u32>::default().since(&ancestor), None);
    }
}

/// Mutation-tracked persistent object membership for derived rule restrictions.
#[derive(Debug, Clone, Default)]
pub struct ObjectSet {
    members: im::HashSet<crate::ids::ObjectId>,
    changes: ChangeJournal<crate::ids::ObjectId>,
}
impl ObjectSet {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(&mut self, id: crate::ids::ObjectId) -> bool {
        if self.members.contains(&id) {
            return false;
        }
        self.members.insert(id);
        self.changes.record(id);
        true
    }
    pub fn remove(&mut self, id: &crate::ids::ObjectId) -> bool {
        if self.members.remove(id).is_none() {
            return false;
        }
        self.changes.record(*id);
        true
    }
    pub fn clear(&mut self) {
        let ids: Vec<_> = self.members.iter().copied().collect();
        for id in ids {
            self.remove(&id);
        }
    }
    pub fn retain(&mut self, mut keep: impl FnMut(&crate::ids::ObjectId) -> bool) {
        let ids: Vec<_> = self
            .members
            .iter()
            .filter(|id| !keep(id))
            .copied()
            .collect();
        for id in ids {
            self.remove(&id);
        }
    }
    pub fn cursor(&self) -> ChangeCursor {
        self.changes.cursor()
    }
    pub fn changes_since(&self, cursor: &ChangeCursor) -> Option<Vec<crate::ids::ObjectId>> {
        self.changes.since(cursor)
    }
}
impl std::ops::Deref for ObjectSet {
    type Target = im::HashSet<crate::ids::ObjectId>;
    fn deref(&self) -> &Self::Target {
        &self.members
    }
}
impl Extend<crate::ids::ObjectId> for ObjectSet {
    fn extend<T: IntoIterator<Item = crate::ids::ObjectId>>(&mut self, iter: T) {
        for id in iter {
            self.insert(id);
        }
    }
}
impl IntoIterator for ObjectSet {
    type Item = crate::ids::ObjectId;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.members.iter().copied().collect::<Vec<_>>().into_iter()
    }
}

impl PartialEq for ChangeCursor {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ChangeCursor {}

/// An opaque identity for public mutable containers whose clients need a broad
/// invalidation rather than individual deltas. Mutable access always advances
/// identity, including direct slice/index mutation through `DerefMut`.
#[derive(Debug)]
pub struct TrackedValue<T: Clone> {
    value: Arc<T>,
    cursor: ChangeCursor,
    /// Whether a retained derivation read this value (see
    /// `GameState::derived_tap_read_scope`). Never cleared by a read.
    derived_read: std::sync::atomic::AtomicBool,
}

thread_local! {
    /// Depth of running derivations whose results may be retained, across
    /// every game on this thread. A read of any tracked value while one runs
    /// is recorded on that value; a nested or cloned game only over-records.
    static DERIVED_READ_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

pub(crate) fn enter_derived_reads() {
    DERIVED_READ_DEPTH.with(|depth| depth.set(depth.get().saturating_add(1)));
}

pub(crate) fn exit_derived_reads() {
    DERIVED_READ_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
}

fn derived_read_in_progress() -> bool {
    DERIVED_READ_DEPTH.with(|depth| depth.get() > 0)
}

impl<T: Clone> Clone for TrackedValue<T> {
    fn clone(&self) -> Self {
        // A copy taken inside a derivation can be read in place of this value.
        if derived_read_in_progress() {
            self.derived_read.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Self {
            value: Arc::clone(&self.value),
            cursor: self.cursor.clone(),
            derived_read: std::sync::atomic::AtomicBool::new(self.derived_read()),
        }
    }
}
impl<T: Clone + Default> Default for TrackedValue<T> {
    fn default() -> Self {
        T::default().into()
    }
}
impl<T: Clone> From<T> for TrackedValue<T> {
    fn from(value: T) -> Self {
        Self {
            value: Arc::new(value),
            cursor: ChangeCursor(Arc::new(())),
            derived_read: std::sync::atomic::AtomicBool::new(false),
        }
    }
}
impl<T: Clone> TrackedValue<T> {
    /// Whether a derivation has read this value since the flag was last taken.
    pub(crate) fn derived_read(&self) -> bool {
        self.derived_read.load(std::sync::atomic::Ordering::Relaxed)
    }
    pub(crate) fn take_derived_read(&self) -> bool {
        self.derived_read.swap(false, std::sync::atomic::Ordering::Relaxed)
    }
    pub(crate) fn note_derived_read(&self) {
        self.derived_read.store(true, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn cursor(&self) -> ChangeCursor {
        self.cursor.clone()
    }
    /// Derived player fields are refreshed under an existing invalidation.
    /// They must not invalidate that refresh merely by acquiring a mutable view.
    pub(crate) fn get_mut_for_derived_update(&mut self) -> &mut T {
        Arc::make_mut(&mut self.value)
    }
}
impl<T: Clone> std::ops::Deref for TrackedValue<T> {
    type Target = T;
    fn deref(&self) -> &T {
        if derived_read_in_progress() {
            self.derived_read.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        &self.value
    }
}
impl<T: Clone> std::ops::DerefMut for TrackedValue<T> {
    fn deref_mut(&mut self) -> &mut T {
        if derived_read_in_progress() {
            self.derived_read.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.cursor = ChangeCursor(Arc::new(()));
        Arc::make_mut(&mut self.value)
    }
}
impl<'a, T: Clone> IntoIterator for &'a TrackedValue<Vec<T>> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, T: Clone> IntoIterator for &'a mut TrackedValue<Vec<T>> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
impl<T: Clone> IntoIterator for TrackedValue<Vec<T>> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        Arc::unwrap_or_clone(self.value).into_iter()
    }
}

impl<T: Clone> FromIterator<T> for TrackedValue<Vec<T>> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().collect::<Vec<_>>().into()
    }
}

impl<T: Clone + PartialEq> PartialEq for TrackedValue<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.value, &other.value) || self.value == other.value
    }
}
impl<T: Clone + Eq> Eq for TrackedValue<T> {}
