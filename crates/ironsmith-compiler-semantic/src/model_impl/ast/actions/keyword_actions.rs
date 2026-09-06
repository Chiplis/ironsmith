//! The keywordactions actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
        amount: Value,
        count: Value,
    },
    Learn,
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
    Amass {
        subtype: Option<Subtype>,
        amount: Value,
    },
    Bolster {
        amount: u32,
    },
    Support {
        amount: u32,
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
    Explore {
        target: TargetAst,
    },
    Endure {
        target: TargetAst,
        amount: Value,
    },
    Exploit,
    Connive {
        target: TargetAst,
        count: Value,
    },
    ConniveIterated,
    OpenAttraction {
        reminder: bool,
    },
    ManifestCardFromHand,
    ManifestDread,
    Earthbend {
        counters: u32,
    },
    Behold {
        subtype: Subtype,
        count: u32,
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
    },
    TakeInitiative,
    Detain {
        target: TargetAst,
    },
    Goad {
        target: TargetAst,
        duration: Until,
    },
    Suspect {
        target: TargetAst,
    },
    ClearSuspected {
        target: Option<TargetAst>,
    },
    Regenerate {
        target: TargetAst,
        follow_up_effects: Vec<EffectAst>,
    },
    RegenerateAll {
        filter: ObjectFilter,
    },
}
