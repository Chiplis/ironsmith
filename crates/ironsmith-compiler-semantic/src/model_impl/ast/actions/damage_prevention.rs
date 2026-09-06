//! The damage-prevention and redirection actions of `SubjectVerbActionAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
    },
    PreventAllCombatDamageFromSourceFilter {
        duration: Until,
        source_filter: ObjectFilter,
        excluded_source_target: Option<TargetAst>,
    },
    PreventAllCombatDamageToPlayers {
        duration: Until,
    },
    PreventAllCombatDamageToYou {
        duration: Until,
    },
    PreventNextTimeDamage {
        source: PreventNextTimeDamageSourceAst,
        target: PreventNextTimeDamageTargetAst,
        reflect_damage_to_source_controller: bool,
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
        source_of_your_choice: bool,
        source_choice_shares_activation_mana_color: bool,
        source_target: Option<TargetAst>,
    },
    PreventAllDamageToTargetFromSourceFilter {
        target: TargetAst,
        duration: Until,
        source_filter: ObjectFilter,
    },
    PreventAllDamageFromSourceFilter {
        duration: Until,
        source_filter: ObjectFilter,
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
    },
}
