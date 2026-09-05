use crate::tag::TagKeyWalk;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
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
}
