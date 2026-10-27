//! The keywordactions actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq, TagKeyWalk)]
pub enum KeywordActionAst {
    Scry {
        count: Value,
    },
    Surveil {
        count: Value,
    },
    Proliferate {
        count: Value,
    },
    Investigate {
        count: Value,
    },
    Incubate {
        descriptive: bool,
        amount: Value,
        count: Value,
    },
    Learn,
    /// Copy the source for its controller and a chosen opponent, then retarget each copy.
    OptionalSourceCopiesForControllerAndOpponent,
    ReturnUnblockedAttackerForCost,
    EnterAttackingReturnedDefender,
    PairUnpairedCreatures,
    ExileResolvedSpellForCombatCopies,
    CopiesAssignedToOpponents,
    SetPermanentDesignation { designation: ironsmith_core::PermanentDesignation },
    EmitKeywordAction {
        action: crate::events::KeywordActionKind,
        amount: u32,
    },
    Reconfigure {
        target: TargetAst,
    },
    CumulativeUpkeep {
        cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    },
    Casualty {
        power: u32,
    },
    EmpowerJace { amount: Value },
    CollectEvidence { amount: Value },
    Amass {
        subtype: Option<Subtype>,
        amount: Value,
        descriptive: bool,
    },
    Bolster {
        amount: Value,
    },
    Support {
        amount: Value,
    },
    Adapt {
        amount: u32,
    },
    Monstrosity {
        amount: Value,
    },
    Discover {
        count: Value,
    },
    Fateseal {
        count: Value,
    },
    Populate {
        count: Value,
        enters_tapped: bool,
        enters_attacking: bool,
        has_haste: bool,
        sacrifice_at_next_end_step: bool,
        exile_at_next_end_step: bool,
        next_end_step_player: PlayerFilter,
        exile_at_end_of_combat: bool,
        sacrifice_at_end_of_combat: bool,
    },
    Airbend {
        target: TargetAst,
        descriptive_cost: Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>,
    },
    Explore {
        target: TargetAst,
        descriptive: bool,
    },
    Endure {
        target: TargetAst,
        amount: Value,
    },
    Exploit,
    Connive {
        target: TargetAst,
        count: Value,
        descriptive: bool,
    },
    ConniveIterated,
    OpenAttraction {
        reminder: bool,
    },
    /// "Roll to visit your Attractions" (CR 701.52).
    RollToVisitAttractions,
    /// "Roll the planar die" (CR 901.9).
    RollPlanarDie,
    ManifestCardFromHand,
    ManifestDread,
    Earthbend {
        counters: Value,
        descriptive: bool,
    },
    Behold {
        subtype: Subtype,
        count: u32,
        descriptive: bool,
    },
    Fight {
        creature1: TargetAst,
        creature2: TargetAst,
        mutual_surface: bool,
    },
    FightIterated {
        creature2: TargetAst,
    },
    Clash {
        opponent: ClashOpponentAst,
        descriptive: bool,
    },
    Meld {
        result_name: String,
        enters_tapped: bool,
        enters_attacking: bool,
    },
    UnlockRoomDoor,
    RingTemptsYou,
    VentureIntoDungeon {
        undercity_if_no_active: bool,
        descriptive: bool,
    },
    TakeInitiative,
    Detain {
        target: TargetAst,
    },
    Goad {
        target: TargetAst,
        duration: Until,
        /// The card spelled the requirement out ("attack each combat if able
        /// and attack a player other than you if able") instead of saying
        /// "goad" (Kardur, Doomscourge).
        spelled_out_requirement: bool,
    },
    BecomePlotted {
        target: TargetAst,
    },
    /// "It becomes foretold." (The Foretold Soldier): an exiled card gains the
    /// foretold designation (CR 702.143).
    BecomeForetold {
        target: TargetAst,
        /// "Its foretell cost is its mana cost reduced by {N}." (Ethereal
        /// Valkyrie): the generic reduction forming the granted foretell cost.
        foretell_cost_reduction: Option<u32>,
    },
    /// "unlock a locked door of up to one target Room you control" (CR 709.5f).
    /// With `allow_lock`: "lock or unlock a door of target Room you control"
    /// (CR 709.5c).
    UnlockTargetRoomDoor {
        target: TargetAst,
        allow_lock: bool,
    },
    /// "<creature> attacks <player> this turn if able" (CR 508.1d).
    MustAttackPlayerThisTurn {
        target: TargetAst,
        player: TargetAst,
        /// "attacks during its controller's next combat phase if able":
        /// `player` is unused.
        controllers_next_combat: bool,
    },
    Suspect {
        target: TargetAst,
    },
    /// The Prepared keyword action: the subject becomes prepared, or with
    /// `unprepare`, stops being prepared ("becomes unprepared").
    Prepare {
        target: TargetAst,
        unprepare: bool,
    },
    ClearSuspected {
        target: Option<TargetAst>,
    },
    ClearGoad {
        target: Option<TargetAst>,
    },
    Regenerate {
        target: TargetAst,
        follow_up_effects: Vec<EffectAst>,
        /// "The next time <target> would be destroyed this turn, remove all
        /// damage marked on it instead." (Pyramids): only the damage
        /// removal, which is not regeneration.
        remove_damage_only: bool,
        ordinary_destruction_replacement: bool,
    },
    RegenerateAll {
        filter: ObjectFilter,
    },
}
