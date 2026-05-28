#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThisSpellCastTiming {
    DuringDeclareAttackersStep,
    DuringCombat,
    DuringCombatOnYourTurn,
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
