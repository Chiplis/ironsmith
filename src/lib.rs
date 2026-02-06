pub mod ability;
pub mod alternative_cast;
pub mod card;
pub mod cards;
pub mod color;
pub mod combat_state;
pub mod continuous;
pub mod cost;
pub mod costs;
pub mod decision;
pub mod decisions;
pub mod dependency;
pub mod effect;
pub mod effects;
pub mod event_processor;
pub mod events;
pub mod executor;
pub mod filter;
pub mod game_event;
pub mod game_loop;
pub mod game_state;
pub mod grant;
pub mod grant_registry;
pub mod ids;
pub mod mana;
pub mod marker;
#[cfg(feature = "net")]
pub mod net;
pub mod object;
pub mod player;
pub mod prevention;
pub mod replacement;
pub mod replacement_ability_processor;
pub mod rules;
pub mod snapshot;
pub mod special_actions;
pub mod static_abilities;
pub mod static_ability_processor;
pub mod tag;
pub mod target;
pub mod targeting;
pub mod triggers;
pub mod turn;
pub mod types;
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub mod wasm_api;
pub mod zone;

#[cfg(test)]
mod tests;

pub use ability::{Ability, AbilityKind, ActivatedAbility, ManaAbility, TriggeredAbility};
pub use alternative_cast::{AlternativeCastingMethod, CastingMethod, TrapCondition};
pub use card::{Card, CardBuilder, PowerToughness, PtValue};
pub use color::{Color, ColorSet};
pub use continuous::{
    ContinuousEffect, ContinuousEffectId, ContinuousEffectManager, EffectSourceType, Layer,
    Modification, PtSublayer,
};
pub use cost::{OptionalCost, OptionalCostsPaid, PermanentFilter, TotalCost};
pub use effect::{ChoiceCount, Effect, Until, Value};
pub use effects::{DealDamageEffect, EffectExecutor};
pub use event_processor::{
    DestroyResult,
    DrawResult,
    EtbEventResult,
    ReplacementPriority,
    TraitEventResult,
    ZoneChangeResult,
    process_damage_with_event,
    process_destroy_full,
    process_dies_with_event,
    process_draw_full,
    process_etb_with_event,
    process_event_with_chosen_replacement_trait,
    process_life_gain_with_event,
    process_put_counters_with_event,
    // Event-based processing functions
    process_trait_event,
    process_zone_change_full,
    process_zone_change_with_event,
};
pub use filter::{
    Comparison, FilterContext, ObjectFilter, PlayerFilter, TaggedObjectConstraint,
    TaggedOpbjectRelation,
};
pub use game_event::{DamageTarget as GameEventDamageTarget, ObjectSnapshot};
pub use game_state::{CantEffectTracker, GameState, Phase, StackEntry, Step, Target, TurnState};
pub use ids::{CardId, ObjectId, PlayerId};
pub use mana::{ManaCost, ManaSymbol};
pub use object::{CounterType, Object, ObjectKind};
pub use player::{ManaPool, Player};
pub use prevention::{
    DamageFilter, PreventionEffectManager, PreventionShield, PreventionShieldId, PreventionTarget,
};
pub use replacement::{
    ReplacementAction, ReplacementEffect, ReplacementEffectId, ReplacementEffectManager,
};
pub use static_abilities::StaticAbility;
pub use tag::TagKey;
pub use target::ChooseSpec;
pub use types::{CardType, Subtype, Supertype};
#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
pub use wasm_api::WasmGame;
pub use zone::Zone;

// Phase 4 exports
pub use cards::{CardDefinition, CardDefinitionBuilder, CardRegistry};
pub use combat_state::{
    AttackTarget, AttackerInfo, CombatError, CombatState, attackers_targeting_planeswalker,
    attackers_targeting_player, declare_attackers, declare_blockers, end_combat, get_attack_target,
    get_blocked_attacker, get_blockers, get_damage_assignment_order, is_attacking, is_blocked,
    is_blocking, is_unblocked, new_combat, set_damage_assignment_order,
};
pub use decision::{
    AttackerDeclaration, AttackerOption, AutoPassDecisionMaker, BlockerDeclaration, BlockerOption,
    ChoiceOption, DecisionMaker, DecisionRouter, GameProgress, GameResult, LegalAction,
    ManaPaymentOption, ManaPipPaymentAction, ManaPipPaymentOption, ModeOption,
    NumericInputDecisionMaker, OptionalCostOption, ReplacementOption, ResponseError,
    TargetRequirement, compute_legal_actions, compute_legal_attackers, compute_legal_blockers,
};
pub use decisions::context::{
    AttackerOptionContext, AttackersContext, BlockerOptionContext, BlockersContext, BooleanContext,
    ColorsContext, CountersContext, DecisionContext, DistributeContext, DistributeTarget,
    NumberContext, OrderContext, PartitionContext, PriorityContext, ProliferateContext,
    SelectObjectsContext, SelectOptionsContext, SelectableObject, SelectableOption,
};
pub use executor::{
    ExecutionContext, ExecutionError, ResolvedTarget, TargetError, execute_effect, resolve_value,
    validate_target,
};
pub use game_loop::{
    CombatDamageEvent, GameLoopError, PriorityLoopState, PriorityResponse, advance_priority,
    apply_attacker_declarations, apply_blocker_declarations, apply_priority_response,
    check_and_apply_sbas, compute_legal_targets, execute_combat_damage_step, execute_turn_with,
    extract_target_spec, generate_and_queue_step_triggers, get_declare_attackers_decision,
    get_declare_blockers_decision, put_triggers_on_stack, requires_target_selection,
    resolve_stack_entry, run_priority_loop_with, spell_has_legal_targets,
};
pub use rules::{
    DamageResult, DamageTarget, StateBasedAction, apply_state_based_actions, calculate_damage,
    calculate_trample_excess, can_attack, can_block, check_state_based_actions, has_vigilance,
    is_lethal, minimum_blockers, must_attack,
};
pub use snapshot::ObjectSnapshot as UnifiedObjectSnapshot;
pub use special_actions::{ActionError, SpecialAction};
pub use targeting::{
    PendingWardCost, TargetingInvalidReason, TargetingResult, WardCost, WardPaymentResult,
    can_target_object, collect_ward_costs, get_ward_cost, has_protection_from_source,
    source_matches_protection,
};
pub use triggers::{
    AttackEventTarget, DamageEventTarget, TriggerEvent, TriggerQueue, TriggeredAbilityEntry,
    check_triggers, generate_step_trigger_events, player_filter_matches_with_context,
};
pub use turn::{
    PriorityResult, PriorityTracker, TurnError, advance_phase, advance_step,
    current_phase_description, execute_cleanup_step, execute_draw_step, execute_untap_step,
    first_step_of_phase, has_priority, is_combat_phase, is_main_phase, is_no_priority_step,
    is_sorcery_timing, next_phase, next_step, pass_priority, priority_holder, reset_priority,
};

// Trait-based events module re-exports
pub use events::{
    // Matchers
    CombatDamageMatcher,
    // Event types
    DamageEvent,
    DamageFromSourceMatcher,
    DamageToObjectMatcher,
    DamageToPlayerMatcher,
    DamageToSelfMatcher,
    DestroyEvent,
    DiscardEvent,
    DrawEvent,
    EnterBattlefieldEvent,
    Event,
    // Core traits
    EventContext,
    EventKind,
    GameEventType,
    LifeGainEvent,
    LifeLossEvent,
    MoveCountersEvent,
    NoncombatDamageMatcher,
    PutCountersEvent,
    RegenerationShieldMatcher,
    RemoveCountersEvent,
    ReplacementMatcher,
    ReplacementPriority as NewReplacementPriority,
    SacrificeEvent,
    TapEvent,
    ThisWouldBeDestroyedMatcher,
    ThisWouldDieMatcher,
    ThisWouldEnterBattlefieldMatcher,
    UntapEvent,
    WouldBeDestroyedMatcher,
    WouldBeExiledMatcher,
    WouldBeSacrificedMatcher,
    WouldBecomeTappedMatcher,
    WouldBecomeUntappedMatcher,
    WouldDieMatcher,
    WouldDiscardMatcher,
    WouldDrawCardMatcher,
    WouldDrawFirstCardMatcher,
    WouldEnterBattlefieldMatcher,
    WouldGainLifeMatcher,
    WouldGoToGraveyardMatcher,
    WouldLeaveBattlefieldMatcher,
    WouldLoseLifeMatcher,
    WouldPutCountersMatcher,
    WouldRemoveCountersMatcher,
    ZoneChangeEvent,
    // Helper functions
    downcast_event,
};
