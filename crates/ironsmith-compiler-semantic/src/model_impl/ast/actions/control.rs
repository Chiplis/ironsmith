//! The control actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ControlActionAst {
    Attach {
        object: TargetAst,
        target: TargetAst,
    },
    Unattach {
        object: TargetAst,
    },
    Enchant {
        filter: AuraAttachmentFilter,
    },
    ControlCombatChoicesThisTurn {
        attackers: bool,
        blockers: bool,
        this_combat: bool,
    },
    GainControl {
        target: TargetAst,
        duration: Until,
        condition: Option<PredicateAst>,
        /// Explicit object whose controller performs the control change.
        ///
        /// This preserves authored relational subjects such as "that
        /// source's controller" without resolving them through the generic
        /// last-object antecedent.
        controller_reference: Option<ObjectRef>,
        source_reference_surface: Option<SourceReferenceSurface>,
    },
    ControlPlayer {
        player: PlayerFilter,
        duration: ControlDurationAst,
    },
}
