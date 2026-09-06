//! The permanentstate actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum PermanentStateActionAst {
    TurnFaceUp {
        target: TargetAst,
    },
    Tap {
        target: TargetAst,
    },
    Untap {
        target: TargetAst,
    },
    TapAll {
        filter: ObjectFilter,
    },
    UntapAll {
        filter: ObjectFilter,
    },
    TapOrUntap {
        target: TargetAst,
    },
    TapOrUntapAll {
        tap_filter: ObjectFilter,
        untap_filter: ObjectFilter,
    },
    PhaseOut {
        target: TargetAst,
        duration: crate::effects::PhaseOutDuration,
        source_surface: Option<SourceReferenceSurface>,
    },
    PhaseOutAll {
        filter: ObjectFilter,
        duration: crate::effects::PhaseOutDuration,
        source_surface: Option<SourceReferenceSurface>,
    },
    PhaseIn {
        target: TargetAst,
    },
    PhaseInAll {
        filter: ObjectFilter,
    },
    Transform {
        target: TargetAst,
    },
    Convert {
        target: TargetAst,
    },
    SwitchPowerToughness {
        target: TargetAst,
        duration: Until,
    },
    ScalePowerToughnessAll {
        filter: ObjectFilter,
        power: bool,
        toughness: bool,
        multiplier: i32,
        duration: Until,
    },
    RemoveFromCombat {
        target: TargetAst,
    },
    Flip {
        target: TargetAst,
    },
}
