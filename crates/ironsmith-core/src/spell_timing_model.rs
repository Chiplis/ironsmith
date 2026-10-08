use crate::tag::TagKeyWalk;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum ThisSpellCastTiming {
    DuringDeclareAttackersStep,
    DuringCombat,
    DuringCombatBeforeBlockersAreDeclared,
    DuringCombatAfterBlockersAreDeclared,
    DuringCombatOnYourTurnBeforeBlockersAreDeclared,
    DuringCombatOnOpponentsTurn,
    BeforeAttackersAreDeclared,
    BeforeCombatDamageStep,
    DuringOpponentsUpkeep,
    DuringOpponentsTurnAfterUpkeep,
    DuringYourEndStep,
    AfterCombat,
    DuringDeclareBlockersStep,
    DuringCombatOnYourTurn,
    DuringYourTurn,
    DuringOpponentsTurn,
    /// "before blockers are declared" / "before combat or during combat
    /// before blockers are declared": the beginning phase, the precombat
    /// main phase, or combat before the declare blockers step.
    BeforeBlockersAreDeclared,
    /// "during your declare attackers step".
    DuringYourDeclareAttackersStep,
    /// "during the declare blockers step on an opponent's turn".
    DuringDeclareBlockersStepOnOpponentsTurn,
    /// "during an opponent's turn, before attackers are declared": the
    /// opponent's beginning phase, precombat main phase, or beginning of combat.
    DuringOpponentsTurnBeforeAttackersAreDeclared,
    /// Appended. "You can't cast this spell during your first[, second, or
    /// third] turns of the game": prohibited while the caster is the active
    /// player and has taken at most this many turns, counting the current
    /// one (CR 500.1, 500.7 extra turns count as turns taken).
    NotDuringYourFirstTurns(u32),
}
