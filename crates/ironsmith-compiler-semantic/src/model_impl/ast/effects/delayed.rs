//! The delayed actions of `EffectAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum DelayedEffectAst {
    DelayedUntilNextEndStep {
        player: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextCleanupStep {
        player: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextUntapStep {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextUpkeep {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextDrawStep {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextMainPhase {
        player: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    DelayedUntilNextFirstMainPhase {
        player: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    DelayedUntilEndStepOfExtraTurn {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilEndOfCombat {
        effects: Vec<EffectAst>,
    },
    DelayedTriggerThisTurn {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
        one_shot: bool,
        until_end_of_combat: bool,
        attach_to_previous_ability: bool,
    },
    /// Register a repeating or one-shot delayed trigger with an explicit
    /// duration. This is distinct from granting an object a temporary
    /// triggered ability: the registration captures referenced objects when
    /// this effect resolves and then watches them independently.
    DelayedTriggerForDuration {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
        one_shot: bool,
        duration: Until,
        either_of_watched_objects: bool,
        /// Keep the registration active only while at least one object from
        /// the captured tag remains in this zone.
        while_any_tagged_object_in_zone: Option<(TagRef, Zone)>,
    },
    DelayedWhenLastObjectDiesThisTurn {
        filter: Option<ObjectFilter>,
        effects: Vec<EffectAst>,
    },
    /// A delayed trigger tied to the object selected or created by the
    /// immediately preceding effect. Unlike the dies-this-turn form, this
    /// trigger has no turn-based expiry.
    DelayedWhenLastObjectLeavesBattlefield {
        filter: ObjectFilter,
        effects: Vec<EffectAst>,
    },
}
