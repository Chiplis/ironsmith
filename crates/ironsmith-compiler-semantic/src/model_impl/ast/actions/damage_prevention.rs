//! The damage-prevention and redirection actions of `SubjectVerbActionAst`.

use super::*;
use ironsmith_compiler_ast::TagRef;

#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct TimedDamageRedirectionAst {
    pub source_filter: ObjectFilter,
    pub source_target: Option<TargetAst>,
    pub protected_target: Option<TargetAst>,
    pub player_filter: Option<PlayerFilter>,
    pub object_filter: Option<ObjectFilter>,
    pub combat_only: bool,
    pub destination: ironsmith_core::TimedDamageRedirectDestination,
    pub mode: ironsmith_core::ReplacementApplyMode,
    pub display: String,
}

#[derive(Clone, PartialEq, TagKeyWalk)]
pub enum DamagePreventionActionAst {
    PreventAllCombatDamage {
        duration: Until,
    },
    AssignNoCombatDamage {
        source: TargetAst,
        duration: Until,
    },
    PreventAllCombatDamageFromSource {
        duration: Until,
        source: TargetAst,
        source_would_deal_surface: bool,
        /// "that would be dealt to and dealt by <source>": also prevent the
        /// combat damage dealt to it.
        dealt_to_and_by: bool,
    },
    PreventAllCombatDamageFromSourceFilter {
        duration: Until,
        source_filter: ObjectFilter,
        excluded_source_target: Option<TargetAst>,
        /// "a creature of your choice would deal": one matching source is
        /// chosen as the effect resolves, and only its damage is prevented.
        source_of_your_choice: bool,
    },
    PreventAllCombatDamageToPlayers {
        duration: Until,
    },
    PreventAllCombatDamageToYou {
        duration: Until,
        /// "For each 1 damage prevented this way, ..." (Inkshield): run as
        /// each damage event is prevented, reading the prevented amount.
        follow_up_effects: Vec<EffectAst>,
    },
    PreventNextTimeDamage {
        source: PreventNextTimeDamageSourceAst,
        target: PreventNextTimeDamageTargetAst,
        reflect_damage_to_source_controller: bool,
        /// "If damage from a red source is prevented this way, ...": the
        /// reflected damage happens only when the prevented damage's source
        /// matches this filter at that time.
        reflect_source_filter: Option<ObjectFilter>,
        follow_up_effects: Vec<EffectAst>,
    },
    ReplaceNextDamageToTarget {
        target: TargetAst,
        damage_target_tag: TagRef,
        replacement_effects: Vec<EffectAst>,
    },
    PreventDamage {
        amount: Value,
        target: TargetAst,
        duration: Until,
        source_of_your_choice: bool,
        protect_you_and_permanents_you_control: bool,
        follow_up_effects: Vec<EffectAst>,
    },
    PreventAllDamageToTarget {
        target: TargetAst,
        duration: Until,
        combat_only: bool,
        source_of_your_choice: bool,
        source_choice_shares_activation_mana_color: bool,
        source_target: Option<TargetAst>,
        /// The same declared source is also protected against incoming damage.
        protect_source_target: bool,
        follow_up_effects: Vec<EffectAst>,
    },
    PreventAllDamageToTargetFromSourceFilter {
        target: TargetAst,
        duration: Until,
        source_filter: ObjectFilter,
    },
    PreventAllDamageFromSourceFilter {
        duration: Until,
        source_filter: ObjectFilter,
        /// "sources of the color of your choice": the color is chosen as the
        /// effect resolves and narrows `source_filter`.
        of_chosen_color: bool,
    },
    PreventDamageToTargetPutCounters {
        amount: Option<Value>,
        target: TargetAst,
        duration: Until,
        counter_type: CounterType,
    },
    PreventDamageEach {
        amount: Value,
        filter: ObjectFilter,
        duration: Until,
    },
    RedirectNextDamageFromSourceToTarget {
        amount: Value,
        protected_target: Option<TargetAst>,
        destination: RedirectNextTimeDamageDestinationAst,
        destination_target: Option<TargetAst>,
    },
    RedirectNextTimeDamageToSource {
        source: PreventNextTimeDamageSourceAst,
        target: TargetAst,
        destination: RedirectNextTimeDamageDestinationAst,
        destination_target: Option<TargetAst>,
        all_this_turn: bool,
    },
    RedirectAllDamageThisTurnBySourceToSourceController {
        source: TargetAst,
    },
    RedirectAllDamageThisTurnToTarget {
        player_filter: PlayerFilter,
        object_filter: ObjectFilter,
        target: TargetAst,
        scope: Option<TimedDamageRedirectionAst>,
    },
}
