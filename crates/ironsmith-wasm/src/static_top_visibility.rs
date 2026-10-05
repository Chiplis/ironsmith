//! The static top-card view cannot reveal a changed top during an action's
//! announcement/payment. This is derived from the exact local action snapshot,
//! never from a peer assertion or a public state hash.
use super::*;

#[derive(Clone, Copy, Default)]
pub(crate) struct StaticLibraryTopVisibilityWindow<'a> {
    announcing: bool,
    before: Option<&'a GameState>,
}
impl StaticLibraryTopVisibilityWindow<'_> {
    pub(crate) fn allows(&self, game: &GameState, player: PlayerId) -> bool {
        // A resolving spell may start a cast/land play using a local native
        // priority state. Its exact decision clone carries this owner even
        // when the host has no cancelable action or pending-cast cursor.
        if game.has_library_top_announcement() {
            return game.static_library_top_visible_during_announcements(player);
        }
        if !self.announcing { return true; }
        let Some(before) = self.before else { return false; };
        if before.library_top_revision(player) != game.library_top_revision(player)
            || before.player(player).and_then(|p| p.library.last())
                != game.player(player).and_then(|p| p.library.last())
        {
            return false;
        }
        // A shuffle is a new hidden-position question even when an offline
        // deterministic permutation happens to leave the same card on top.
        !game.crypto_audit_operations_since(before.crypto_audit_checkpoint()).iter().any(|operation|
            matches!(operation, HiddenInfoOperation::LibraryShuffle {player: owner, ..} if *owner == player))
    }
}
impl WasmGame {
    pub(super) fn static_library_top_visibility_window(&self) -> StaticLibraryTopVisibilityWindow<'_> {
        let before = self.pending_action_checkpoint.as_ref().map(|checkpoint| checkpoint.game.as_ref())
            .or(self.priority_state.checkpoint.as_ref().filter(|_| self.priority_state.has_pending_action()));
        StaticLibraryTopVisibilityWindow {
            announcing: before.is_some() || self.priority_state.has_pending_action(),
            before,
        }
    }

    pub(super) fn static_library_top_visibility_hash(&self) -> u64 {
        let window = self.static_library_top_visibility_window();
        let game = self.pending_decision_game.as_deref().unwrap_or(&self.game);
        hash_debug_value(&(window.announcing, game.players.iter()
            .map(|player| (player.id, window.allows(game, player.id))).collect::<Vec<_>>()))
    }
}
