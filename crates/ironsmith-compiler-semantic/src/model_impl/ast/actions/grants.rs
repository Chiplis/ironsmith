//! The grants actions of `SubjectVerbActionAst`.

use super::*;
use ironsmith_compiler_ast::TagRef;

#[derive(Clone, PartialEq, TagKeyWalk)]
pub enum GrantActionAst {
    GrantProtectionChoice {
        target: TargetAst,
        chooser: PlayerAst,
        allow_colorless: bool,
        allow_artifacts: bool,
        choose_card_type: bool,
        /// "you and each permanent you control gain protection from the
        /// color of your choice" (Faith's Shield): a quantified object set
        /// that gains the same chosen protection alongside `target`. One
        /// choice covers both recipients.
        also_each: Option<ObjectFilter>,
    },
    GrantPlayTaggedUntilEndOfTurn {
        spell_filter: Option<ObjectFilter>,
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
        /// Mana reduction attached to using this permission.
        spell_cost_reduction: Option<ManaCost>,
        /// "If that spell doesn't target <filter>, it costs {N} more to cast
        /// this way": the surcharge and the target filter that waives it.
        spell_cost_increase_unless_targets: Option<(ManaCost, ObjectFilter)>,
        alternative_cost: Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
        /// "You may play target snow permanent card from your graveyard
        /// this turn. If you do, it enters tapped." (Kaldring, the
        /// Rimestaff): the land played or permanent spell cast through this
        /// permission enters the battlefield tapped.
        permanents_enter_tapped: bool,
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
        until_next_turn_start: bool,
        /// "their" duration: use the permission holder, not the effect controller.
        duration_is_holder_relative: bool,
        /// "During your next turn, you may play ...": usable only during
        /// that turn, not for the rest of the current one.
        during_next_turn: bool,
        /// "Until end of combat on your next turn, you may play ..." (Brazen
        /// Cannonade): through the combat phase of that turn.
        until_end_of_combat_next_turn: bool,
        spell_filter: Option<ObjectFilter>,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
        /// Total plays shared by the tagged collection across the duration.
        max_plays: Option<u32>,
        /// "you may play cards this way without paying their mana costs"
        /// (Ignite the Future): spells cast through this permission are cast
        /// without paying their mana costs (CR 118.9).
        without_paying_mana_cost: bool,
    },
    GrantPlayTaggedForAsLongAsExiled {
        permission_bound_mana: bool,
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
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
        /// Restrict the persistent permission to turns in which its player
        /// attacked with enough matching creatures.
        during_turns_attacked_with: Option<ironsmith_core::effect::AttackedWithTurnCondition>,
        /// Restrict the persistent permission to turns in which its player
        /// has already cast another spell ("as long as you've cast another
        /// spell this turn").
        during_turns_cast_another_spell: bool,
        /// "That spell has undaunted." (Durnan of the Yawning Portal):
        /// generic reduction per opponent for spells cast this way.
        spell_cost_reduction_per_opponent: u32,
    },
    GrantPlayTaggedForAsLongAsYouControlSource {
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        allow_any_color_for_cast: ironsmith_core::value_model::ManaSpendMode,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
        /// "you may play one of the exiled cards" (Kotose): a shared play
        /// budget over the tagged collection; `None` grants each card.
        max_plays: Option<u32>,
        /// "... during your turn": usable only during the player's turns.
        during_your_turns: bool,
        /// "by waterbending {X} rather than paying its mana cost": the
        /// required replacement price for a spell cast this way.
        alternative_cost: Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>,
        /// "..., where X is its mana value": X in `alternative_cost` is the
        /// granted card's mana value.
        alternative_cost_x_is_mana_value: bool,
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
    GrantActivatedAbilitiesFrom {
        target: TargetAst,
        source: TargetAst,
        duration: Until,
        /// "each Horror you control gains ...": every object matching the
        /// recipient filter at resolution receives the copies.
        each_recipient: bool,
        /// "You may spend blue mana as though it were mana of any color to
        /// activate those abilities": only the copies' costs widen.
        symbol_as_any_color: Option<ironsmith_core::ManaSymbol>,
    },
    GrantAbilitiesToTarget {
        target: TargetAst,
        abilities: Vec<GrantedAbilityAst>,
        duration: Until,
        condition: Option<PredicateAst>,
        set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
        /// "If it doesn't have \"...,\" it gains that ability" (Musician):
        /// a recipient that already has the granted abilities is skipped.
        only_if_absent: bool,
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
        mode: ironsmith_core::NextSpellGrantMode,
    },
    /// A resolving permission with a fixed beneficiary and exact source lifetime.
    GrantPlayTaggedWhileSourceOnBattlefield {
        tag: TagRef,
        player: PlayerAst,
        allow_land: bool,
        without_paying_mana_cost: bool,
        surface: Option<ironsmith_core::GrantPlayTaggedSurface>,
    },

}
