//! The grants actions of `SubjectVerbActionAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum GrantActionAst {
    GrantProtectionChoice {
        target: TargetAst,
        chooser: PlayerAst,
        allow_colorless: bool,
        allow_artifacts: bool,
        choose_card_type: bool,
    },
    GrantPlayTaggedUntilEndOfTurn {
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        while_on_top_of_library: bool,
        free_cast_from_current_zone: bool,
        /// Use the source-exile event boundary instead of end of turn.
        until_source_exiles_another: bool,
        /// Total plays shared by the tagged collection across the duration.
        max_plays: Option<u32>,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    },
    GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
        tag: TagRef,
        player: PlayerAst,
    },
    GrantPlayTaggedUntilYourNextTurn {
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        until_next_end_step: bool,
        /// Total plays shared by the tagged collection across the duration.
        max_plays: Option<u32>,
    },
    GrantPlayTaggedForAsLongAsExiled {
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        filter: Option<ObjectFilter>,
        /// Restrict the persistent permission to turns in which this counter
        /// type was put on the ability source.
        during_turns_counter_put_on_source: Option<crate::object::CounterType>,
        /// Additional mana cost for nonland cards cast through this exact
        /// permission.
        spell_cost_increase: Option<ManaCost>,
        /// Whether lands played through this exact permission enter tapped.
        lands_enter_tapped: bool,
    },
    GrantPlayTaggedForAsLongAsYouControlSource {
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    },
    GrantAbilitiesAll {
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: Option<PredicateAst>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
        /// CR 611.2c normally fixes the affected set when a resolving effect
        /// starts. Some rules effects instead create a continuous rule for a
        /// filter for the stated duration and must also affect later entrants.
        lock_filter_at_resolution: bool,
    },
    GrantAbilitiesChoiceAll {
        filter: ObjectFilter,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    GrantAbilitiesToTarget {
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: Option<PredicateAst>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    },
    GrantToTarget {
        target: TargetAst,
        grantable: Box<crate::model::CompilerGrantableCore>,
        duration: crate::grant::GrantDuration,
    },
    GrantBySpec {
        spec: Box<crate::model::CompilerGrantSpecCore>,
        player: PlayerAst,
        duration: crate::grant::GrantDuration,
    },
    GrantAbilitiesChoiceToTarget {
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
    },
    GrantAbilityToSource {
        ability: Box<ParsedAbility>,
        duration: Until,
    },
    GrantNextSpellAbilityThisTurn {
        filter: ObjectFilter,
        ability: Box<GrantedAbilityAst>,
    },
}
