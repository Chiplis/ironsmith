//! "You may reselect which player or permanent target attacking creature is
//! attacking." (Portal Mage): the attacking creature's controller chooses a
//! new player, planeswalker, or battle it could attack (CR 506.4, 508.1b).
//! The creature stays attacking and keeps any blocked status.
//!
//! With `attacked_player`, the new attack is fixed instead of chosen: "Those
//! creatures are now attacking that player." (Portal Manipulator). A creature
//! whose controller could not attack that player keeps its attack.
use crate::tag::TagKeyWalk;
use crate::target_model::ChooseSpec;
use crate::PlayerFilter;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct ReselectAttackTargetEffect {
    /// The attacking creature or creatures whose attack is redirected.
    pub target: ChooseSpec,
    /// "reselect which player": only players may be chosen.
    pub players_only: bool,
    /// "are now attacking that player": the player each creature now
    /// attacks, with no choice (CR 506.4).
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub attacked_player: Option<PlayerFilter>,
    /// "tap it, and it's attacking that player" (Midnight Crusader Shuttle):
    /// a creature that isn't attacking becomes an attacking creature attacking
    /// `attacked_player` (CR 506.3d), without having been declared as an
    /// attacker. Requires `attacked_player`.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "std::ops::Not::not"))]
    pub enters_combat: bool,
    /// "choose a player or planeswalker that opponent is attacking. <It> is
    /// attacking that player or planeswalker" (Tahngarth, First Mate): the
    /// effect's controller chooses among what this player's other attacking
    /// creatures are attacking. With `enters_combat`, a creature not in
    /// combat becomes attacking the chosen one (CR 506.3d). Appended with a
    /// serde default.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub among_attacked_by: Option<PlayerFilter>,
}

impl ReselectAttackTargetEffect {
    pub fn new(target: ChooseSpec, players_only: bool) -> Self {
        Self {
            target,
            players_only,
            attacked_player: None,
            enters_combat: false,
            among_attacked_by: None,
        }
    }

    /// "<it> is attacking a player or planeswalker <player> is attacking".
    pub fn attacking_one_attacked_by(mut self, player: PlayerFilter) -> Self {
        self.players_only = false;
        self.attacked_player = None;
        self.enters_combat = true;
        self.among_attacked_by = Some(player);
        self
    }

    /// "it's attacking that player": a creature not in combat becomes an
    /// attacking creature attacking this player; one already attacking is
    /// redirected to them.
    pub fn attacking_player(mut self, player: PlayerFilter) -> Self {
        self.players_only = true;
        self.attacked_player = Some(player);
        self.enters_combat = true;
        self
    }

    /// Redirect each creature's attack to this player.
    pub fn now_attacking(mut self, player: PlayerFilter) -> Self {
        self.players_only = true;
        self.attacked_player = Some(player);
        self
    }
}

/// "If each of those creatures could block all creatures that the other is
/// blocking, remove both of them from combat. Each one then blocks all
/// creatures the other was blocking." (Sorrow's Path): two blocking creatures
/// trade the attackers they block, provided each could legally block every
/// creature the other blocks. Neither is declared as a blocker again, so no
/// block triggers fire, and every blocked attacker stays blocked (CR 509.1h).
///
/// With `between_attackers` (General Jarkeld) the pair are two blocked
/// attacking creatures instead: if each could be blocked by all creatures the
/// other is blocked by, every creature blocking exactly one of them stops
/// blocking it and blocks the other. Appended with a serde default; omitted
/// from Debug when false.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, PartialEq, TagKeyWalk)]
pub struct ExchangeBlocksEffect {
    /// The two blocking creatures (or blocked attackers).
    pub target: ChooseSpec,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "std::ops::Not::not"))]
    pub between_attackers: bool,
}

impl std::fmt::Debug for ExchangeBlocksEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ExchangeBlocksEffect");
        debug.field("target", &self.target);
        if self.between_attackers {
            debug.field("between_attackers", &true);
        }
        debug.finish()
    }
}

impl ExchangeBlocksEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target, between_attackers: false }
    }

    /// The pair are blocked attackers that trade their blockers.
    pub fn between_attackers(mut self) -> Self {
        self.between_attackers = true;
        self
    }
}

/// "Creatures it was blocking that had become blocked by only this creature
/// this combat become unblocked." (Ydwen Efreet): each attacker whose only
/// blocker this combat was `blocker`, and that `blocker` no longer blocks,
/// stops being blocked (it is neither blocked nor declared unblocked again,
/// so no block triggers fire).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct UnblockSoleBlockedAttackersEffect {
    /// The creature that was the attackers' only blocker.
    pub blocker: ChooseSpec,
}

impl UnblockSoleBlockedAttackersEffect {
    pub fn new(blocker: ChooseSpec) -> Self {
        Self { blocker }
    }
}

/// "You may have it block an attacking creature of your choice." (False
/// Orders): after blockers are declared, `blocker` becomes a blocking
/// creature blocking one attacking creature matching `attackers` that is
/// attacking its controller, chosen by the effect's controller. It is not
/// declared as a blocker, so no block triggers fire, and the chosen attacker
/// becomes blocked (CR 509.1h). With `optional`, the controller may choose
/// none.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct BlockAttackerOfChoiceEffect {
    /// The creature that starts blocking.
    pub blocker: ChooseSpec,
    /// The attacking creatures it may be made to block.
    pub attackers: crate::filter_model::ObjectFilter,
    /// "You may have ...": choosing no attacker is allowed.
    pub optional: bool,
}

impl BlockAttackerOfChoiceEffect {
    pub fn new(
        blocker: ChooseSpec,
        attackers: crate::filter_model::ObjectFilter,
        optional: bool,
    ) -> Self {
        Self {
            blocker,
            attackers,
            optional,
        }
    }
}
