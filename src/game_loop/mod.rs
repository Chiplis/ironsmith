//! Game loop and integration for MTG.
//!
//! This module provides the main game loop integration including:
//! - Stack resolution
//! - Combat damage execution
//! - Priority loop with player decisions
//! - State-based action integration
//! - Full turn execution

#![allow(unused_imports)]

use crate::ability::AbilityKind;
use crate::alternative_cast::CastingMethod;
use crate::combat_state::{
    AttackTarget, CombatError, CombatState, get_attack_target, get_damage_assignment_order,
    is_blocked, is_unblocked,
};
use crate::cost::OptionalCostsPaid;
use crate::costs::CostContext;
use crate::decision::{
    AlternativePaymentEffect, AttackerDeclaration, BlockerDeclaration, DecisionMaker, GameProgress,
    GameResult, KeywordPaymentContribution, LegalAction, ManaPaymentOption, ManaPipPaymentAction,
    ManaPipPaymentOption, OptionalCostOption, ReplacementOption, ResponseError, TargetRequirement,
    can_activate_ability_with_restrictions, compute_commander_actions, compute_legal_actions,
    compute_legal_attackers, compute_legal_blockers, compute_potential_mana,
};
use crate::effect::Effect;
use crate::events::cause::EventCause;
use crate::events::combat::{
    CreatureAttackedAndUnblockedEvent, CreatureAttackedEvent, CreatureBecameBlockedEvent,
    CreatureBlockedEvent,
};
use crate::events::damage::DamageEvent;
use crate::events::life::{LifeGainEvent, LifeLossEvent};
use crate::events::permanents::SacrificeEvent;
use crate::events::spells::{AbilityActivatedEvent, BecomesTargetedEvent, SpellCastEvent};
use crate::events::zones::EnterBattlefieldEvent;
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ResolvedTarget, execute_effect};
use crate::filter::{FilterContext, ObjectFilter};
use crate::game_event::DamageTarget as EventDamageTarget;
use crate::game_state::{GameState, StackEntry, Step, Target};
use crate::ids::{ObjectId, PlayerId, StableId};
#[cfg(feature = "net")]
use crate::net::{CostPayment, CostStep, GameObjectId, ManaSymbolCode, ManaSymbolSpec, ZoneCode};
#[cfg(not(feature = "net"))]
type CostStep = ();
use crate::object::CounterType;
use crate::player::ManaPool;
use crate::provenance::{ProvNodeId, ProvenanceNodeKind};
use crate::rules::combat::{
    deals_first_strike_damage_with_game, deals_regular_combat_damage_with_game, maximum_blockers,
    minimum_blockers,
};
use crate::rules::damage::{
    DamageResult, DamageTarget, calculate_damage_with_game, distribute_trample_damage,
};
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::triggers::{
    DamageEventTarget, TriggerEvent, TriggerQueue, TriggeredAbilityEntry, check_triggers,
    generate_step_trigger_events, verify_intervening_if,
};
use crate::turn::{PriorityResult, PriorityTracker, TurnError, pass_priority, reset_priority};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

mod combat_damage;
mod combat_decisions;
mod priority_apply;
mod priority_cast;
mod priority_core;
mod priority_mana;
mod priority_state;
mod saga;
mod sba_triggers;
mod stack_resolution;
mod targeting;
#[cfg(all(test, feature = "engine-integration-tests"))]
mod tests;
mod turn_execution;
mod types;

use self::combat_damage::*;
use self::combat_decisions::*;
use self::priority_apply::*;
use self::priority_cast::*;
use self::priority_core::*;
use self::priority_mana::*;
use self::priority_state::*;
use self::saga::*;
use self::sba_triggers::*;
use self::stack_resolution::*;
use self::targeting::*;
use self::turn_execution::*;
use self::types::*;

pub use self::combat_damage::*;
pub use self::combat_decisions::*;
pub use self::priority_apply::apply_priority_response_with_dm;
pub use self::priority_core::*;
pub use self::priority_mana::run_priority_loop_with;
pub use self::priority_state::*;
pub use self::saga::*;
pub use self::sba_triggers::*;
pub use self::stack_resolution::*;
pub use self::targeting::{
    ExtractedTarget, compute_legal_targets, compute_legal_targets_with_tagged_objects,
    extract_target_spec, player_matches_filter_with_combat, requires_target_selection,
    spell_has_legal_targets,
};
pub use self::turn_execution::*;
pub use self::types::*;

pub(crate) use self::priority_mana::{apply_decision_context_with_dm, mana_ability_is_undo_safe};
pub(crate) use self::targeting::{
    drain_pending_trigger_events, extract_target_requirements_for_effect_with_state,
    spell_has_legal_targets_with_modes, spell_has_legal_targets_with_modes_and_view,
};
