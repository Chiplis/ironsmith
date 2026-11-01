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

impl ThisSpellCastTiming {
    pub fn to_oracle(self) -> String {
        match self {
            Self::DuringDeclareAttackersStep => "during the declare attackers step".into(),
            Self::DuringCombat => "during combat".into(),
            Self::DuringCombatBeforeBlockersAreDeclared => "during combat before blockers are declared".into(),
            Self::DuringCombatAfterBlockersAreDeclared => "during combat after blockers are declared".into(),
            Self::DuringCombatOnYourTurnBeforeBlockersAreDeclared => "during combat on your turn before blockers are declared".into(),
            Self::DuringCombatOnOpponentsTurn => "during combat on an opponent’s turn".into(),
            Self::BeforeAttackersAreDeclared => "before attackers are declared".into(),
            Self::BeforeCombatDamageStep => "before the combat damage step".into(),
            Self::DuringOpponentsUpkeep => "during an opponent’s upkeep".into(),
            Self::DuringOpponentsTurnAfterUpkeep => "during an opponent’s turn after their upkeep step".into(),
            Self::DuringYourEndStep => "during your end step".into(),
            Self::AfterCombat => "after combat".into(),
            Self::DuringDeclareBlockersStep => "during the declare blockers step".into(),
            Self::DuringCombatOnYourTurn => "during combat on your turn".into(),
            Self::DuringYourTurn => "during your turn".into(),
            Self::DuringOpponentsTurn => "during an opponent’s turn".into(),
            Self::BeforeBlockersAreDeclared => "before blockers are declared".into(),
            Self::DuringYourDeclareAttackersStep => "during your declare attackers step".into(),
            Self::DuringDeclareBlockersStepOnOpponentsTurn => "during the declare blockers step on an opponent’s turn".into(),
            Self::DuringOpponentsTurnBeforeAttackersAreDeclared => "during an opponent’s turn before attackers are declared".into(),
            Self::NotDuringYourFirstTurns(count) => format!("not during your first {count} turns"),
        }
    }
}
