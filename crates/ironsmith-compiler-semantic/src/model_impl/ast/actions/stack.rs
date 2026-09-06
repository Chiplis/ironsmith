//! The stack actions of `SubjectVerbActionAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum StackActionAst {
    CopySpell {
        target: TargetAst,
        /// Authored kind of a stack-object back-reference. Tagged targets
        /// retain identity but not whether the source text named a spell,
        /// ability, or their union.
        target_reference_kind: Option<crate::filter::StackObjectKind>,
        /// The authored target back-reference was the pronoun `it`.
        ///
        /// This survives reference resolution independently of the semantic
        /// target tag so compiled text can reproduce the original pronoun.
        target_reference_pronoun: bool,
        /// Copy every matching stack object instead of choosing one match.
        ///
        /// This is intentionally part of the typed action rather than inferred
        /// from the target filter: `copy target spell` and `copy all spells`
        /// may otherwise lower to the same `ObjectFilter` and lose the printed
        /// set quantifier before runtime execution.
        all_matches: bool,
        count: Value,
        count_surface: Option<ironsmith_core::effect::CopyCountSurface>,
        player: PlayerAst,
        may_choose_new_targets: bool,
        choose_new_target_singular: bool,
        removed_supertypes: Vec<crate::types::Supertype>,
        /// Colors set by an explicit copy exception, such as
        /// "except that the copy is red."
        set_colors: Option<crate::color::ColorSet>,
        /// Card types added by an explicit copy exception, such as
        /// "except the copy is an artifact in addition to its other types."
        added_card_types: Vec<CardType>,
        /// Subtypes added by an explicit copy exception while retaining the
        /// copied spell's other types.
        added_subtypes: Vec<Subtype>,
        /// Base power and toughness set by an explicit copy exception.
        set_base_power_toughness: Option<(i32, i32)>,
    },
    CopySpellForEachTarget {
        target: TargetAst,
        object_filter: Option<ObjectFilter>,
        player_filter: Option<PlayerFilter>,
        player: PlayerAst,
        exclude_current_targets: bool,
        removed_supertypes: Vec<crate::types::Supertype>,
    },
    ScaleXValue {
        target: TargetAst,
        multiplier: u32,
    },
    CastTagged {
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        as_copy: bool,
        copy_cast_reminder_surface: bool,
        copy_instruction_surface: Option<ironsmith_core::effect::CopyInstructionSurface>,
        without_paying_mana_cost: bool,
        additional_mana_cost: Option<ManaCost>,
        cost_reduction: Option<ManaCost>,
        mana_spend_mode: ironsmith_core::value_model::ManaSpendMode,
    },
    RetargetStackObject {
        target: TargetAst,
        mode: RetargetModeAst,
        require_change: bool,
        /// Preserve authored "the copies" independently of the copied
        /// stack-object tag and the per-event copy count.
        copy_reference_plural: bool,
    },
    Counter {
        target: TargetAst,
    },
    CounterUnlessPays {
        target: TargetAst,
        cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    },
    ReduceNextSpellCostThisTurn {
        filter: ObjectFilter,
        reduction: ManaCost,
    },
    ReduceMatchingSpellCostThisTurn {
        filter: ObjectFilter,
        reduction: Value,
        duration: Until,
        next_only: bool,
    },
}
