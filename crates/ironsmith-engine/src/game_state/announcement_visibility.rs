//! Native announcement ownership for continuous top-card visibility. The
//! boundary travels with exact GameState clones, including a resolving effect's
//! local cast/payment state, rather than depending on a host's Undo cursor.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LibraryTopAnnouncement {
    Cast(ObjectId),
    Activation(crate::provenance::ProvNodeId),
    Land(ObjectId),
}
#[derive(Debug, Clone)]
pub(crate) struct LibraryTopVisibilityBoundary {
    tops: HashMap<PlayerId, (Option<ObjectId>, u64)>,
    audit_checkpoint: usize,
}
impl GameState {
    pub(crate) fn capture_library_top_visibility_boundary(&self) -> LibraryTopVisibilityBoundary {
        LibraryTopVisibilityBoundary {
            tops: self.players.iter().map(|player| (player.id,
                (player.library.last().copied(), self.library_top_revision(player.id)))).collect(),
            audit_checkpoint: self.crypto_audit_checkpoint(),
        }
    }
    pub(crate) fn register_library_top_announcement(&mut self, owner: LibraryTopAnnouncement, boundary: LibraryTopVisibilityBoundary) {
        self.runtime_cache.library_top_announcements.entry(owner).or_insert(boundary);
    }
    pub(crate) fn begin_library_top_announcement(&mut self, owner: LibraryTopAnnouncement) {
        if !self.runtime_cache.library_top_announcements.contains_key(&owner) {
            let boundary = self.capture_library_top_visibility_boundary();
            self.register_library_top_announcement(owner, boundary);
        }
    }
    pub(crate) fn finish_library_top_announcement(&mut self, owner: LibraryTopAnnouncement) {
        self.runtime_cache.library_top_announcements.remove(&owner);
    }
    pub fn has_library_top_announcement(&self) -> bool {
        !self.runtime_cache.library_top_announcements.is_empty()
    }
    pub fn static_library_top_visible_during_announcements(&self, player: PlayerId) -> bool {
        let current = self.player(player).and_then(|player| player.library.last().copied());
        self.runtime_cache.library_top_announcements.values().all(|boundary| {
            boundary.tops.get(&player).is_some_and(|(top, revision)|
                *top == current && *revision == self.library_top_revision(player))
                && !self.crypto_audit_operations_since(boundary.audit_checkpoint).iter().any(|operation|
                    matches!(operation, HiddenInfoOperation::LibraryShuffle {player: owner, ..} if *owner == player))
        })
    }
}
