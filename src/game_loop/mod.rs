//! Game loop and integration for MTG.
//!
//! This module provides the main game loop integration including:
//! - Stack resolution
//! - Combat damage execution
//! - Priority loop with player decisions
//! - State-based action integration
//! - Full turn execution

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

include!("types.rs");
include!("targeting.rs");
include!("stack_resolution.rs");
include!("saga.rs");
include!("combat_damage.rs");
include!("priority_state.rs");
include!("priority_core.rs");
include!("priority_apply.rs");
include!("priority_cast.rs");
include!("priority_mana.rs");
include!("sba_triggers.rs");
include!("combat_decisions.rs");
include!("turn_execution.rs");

#[cfg(all(test, feature = "engine-integration-tests"))]
include!("tests.rs");
